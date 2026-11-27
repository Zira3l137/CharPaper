// Pictures for the panel's tiles: one of every outfit of a suite on show, and a close-up of
// every character, its upper body in the outfit it starts in. The app takes them itself rather
// than the Blender add-on, which would make exports slower still: the outfit's file is put on a
// stage of its own, out of sight on a render layer no screen shows, in the rest pose its file
// holds, lit from three sides and seen from a three-quarter view. One picture at a time, so the
// wallpaper hardly feels it.
//
// The pictures are kept in the suite's thumbnails folder, under the outfit's name, or
// #character for the close-up, and a hash of the outfit file's size and time, so an outfit
// exported again is taken again and every other picture is read back on later runs.

use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

use bevy::app::Propagate;
use bevy::asset::RenderAssetUsages;
use bevy::camera::Exposure;
use bevy::camera::RenderTarget;
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::Render;
use bevy::render::RenderApp;
use bevy::render::RenderSystems;
use bevy::render::gpu_readback::Readback;
use bevy::render::gpu_readback::ReadbackComplete;
use bevy::render::render_resource::Extent3d;
use bevy::render::render_resource::PipelineCache;
use bevy::render::render_resource::TextureDimension;
use bevy::render::render_resource::TextureFormat;
use bevy::render::render_resource::TextureUsages;
use bevy::world_serialization::WorldInstanceReady;

use charpaper_suite::Suite;

use crate::assets::load_gltf_in;
use crate::config::SceneConfig;
use crate::suite::ActiveSuite;
use crate::suite::AvailableSuites;

// Pixels a side. A row of 768 bytes is a whole number of the 256 a GPU copies a texture's rows
// in, so the read-back picture has no padding to strip.
const SIZE: u32 = 192;
// Past the stages' layers, which count up from 1.
const STUDIO_LAYER: usize = 63;
// Bevy compiles a pipeline for each new mix of material, vertex layout and camera in the
// background, and leaves out of the picture every mesh whose pipeline isn't ready. A frame
// missing them comes out the same as the next, so two frames alike aren't enough: a picture is
// taken once nothing has been compiling for QUIET, two frames in a row come out the same, and
// the camera has been in place for SETTLE. Past half of GIVE_UP, whatever there is is taken.
const SETTLE: Duration = Duration::from_millis(500);
const QUIET: Duration = Duration::from_millis(250);
const GIVE_UP: Duration = Duration::from_secs(20);
const FOLDER: &str = "thumbnails";
// The close-up's name in the folder. No outfit is likely to be called that.
const CLOSE_UP: &str = "#character";
// How much of the character's height a close-up shows, down from the top of its head: the
// head, the shoulders and the chest.
const CLOSE_UP_SHARE: f32 = 0.3;
// Part of the hash, so pictures taken by an older way of taking them are taken again.
const VERSION: u8 = 2;

pub(crate) struct ThumbnailsPlugin;

impl Plugin for ThumbnailsPlugin {
    fn build(&self, app: &mut App) {
        let compiling = Compiling::default();
        app.init_resource::<Thumbnails>()
            .init_resource::<Studio>()
            .insert_resource(compiling.clone())
            .add_observer(mark_ready)
            .add_systems(
                Update,
                (find_close_ups.run_if(resource_changed::<AvailableSuites>), find_outfits, shoot)
                    .chain(),
            );
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .insert_resource(compiling)
                .add_systems(Render, note_compiling.in_set(RenderSystems::Cleanup));
        }
    }
}

// Whether the renderer had pipelines still compiling on its last frame. The render world
// writes it and the studio reads it, through the one flag both hold.
#[derive(Resource, Clone, Default)]
struct Compiling(Arc<AtomicBool>);

fn note_compiling(cache: Res<PipelineCache>, compiling: Res<Compiling>) {
    compiling.0.store(cache.waiting_pipelines().next().is_some(), Ordering::Relaxed);
}

// The pictures taken so far: each outfit's by the suite's folder and the outfit's name, and
// each character's close-up by the name of its folder in the characters folder.
#[derive(Resource, Default)]
pub struct Thumbnails {
    outfits: HashMap<(PathBuf, String), Handle<Image>>,
    close_ups: HashMap<String, Handle<Image>>,
}

impl Thumbnails {
    pub fn outfit(&self, suite: &Path, skin: &str) -> Option<&Handle<Image>> {
        self.outfits.get(&(suite.to_path_buf(), skin.to_string()))
    }

    pub fn character(&self, folder: &str) -> Option<&Handle<Image>> {
        self.close_ups.get(folder)
    }

    fn insert(&mut self, job: &Job, image: Handle<Image>) {
        match &job.kind {
            Kind::Outfit => self.outfits.insert((job.suite.clone(), job.skin.clone()), image),
            Kind::CloseUp { folder, .. } => self.close_ups.insert(folder.clone(), image),
        };
    }
}

#[derive(Resource, Default)]
struct Studio {
    queue: VecDeque<Job>,
    shoot: Option<Shoot>,
    // Pictures there are, waiting to be taken, or that failed: each is looked at once a run.
    known: HashSet<(PathBuf, String)>,
}

struct Job {
    suite: PathBuf,
    skin: String,
    file: PathBuf,
    // Changes with the file. A packed file's is its content hash, so exporting it again
    // unchanged keeps its picture.
    stamp: String,
    // Where the character faces in its rest pose, in glTF's axes.
    front: Vec3,
    cache: PathBuf,
    kind: Kind,
}

#[derive(Clone)]
enum Kind {
    Outfit,
    // Of the character in the folder of this name, centered on its head bone when gaze names
    // one.
    CloseUp { folder: String, head: Option<String> },
}

impl Job {
    // How it's named in the log and in the thumbnails folder.
    fn name(&self) -> &str {
        match &self.kind {
            Kind::Outfit => &self.skin,
            Kind::CloseUp { .. } => CLOSE_UP,
        }
    }

    fn label(&self) -> String {
        match &self.kind {
            Kind::Outfit => format!("outfit {:?}", self.skin),
            Kind::CloseUp { folder, .. } => format!("character {folder:?}"),
        }
    }
}

struct Shoot {
    job: Job,
    gltf: Handle<Gltf>,
    set: Entity,
    target: Handle<Image>,
    started: Duration,
    // When the camera was placed.
    framed: Duration,
    // Since when no pipeline has been compiling, if none is now.
    quiet: Option<Duration>,
    step: Step,
    last: Option<Vec<u8>>,
}

#[derive(PartialEq)]
enum Step {
    Loading,
    Spawning,
    // Waiting a frame for the outfit's bounds, then placing the camera.
    Framing,
    Taking,
}

// The root of the outfit's own stage.
#[derive(Component)]
struct StudioSet;

#[derive(Component)]
struct StudioOutfit;

#[derive(Component)]
struct OutfitReady;

fn mark_ready(
    ready: On<WorldInstanceReady>,
    outfits: Query<(), With<StudioOutfit>>,
    mut commands: Commands,
) {
    if outfits.contains(ready.entity) {
        commands.entity(ready.entity).insert(OutfitReady);
    }
}

// Every character has its close-up read, or queued to be taken, in the outfit it starts in.
fn find_close_ups(
    config: Res<SceneConfig>,
    available: Res<AvailableSuites>,
    mut studio: ResMut<Studio>,
    mut thumbnails: ResMut<Thumbnails>,
    mut images: ResMut<Assets<Image>>,
) {
    for folder in &available.0 {
        let root = config.characters_dir.join(folder);
        if !studio.known.insert((root.clone(), CLOSE_UP.to_string())) {
            continue;
        }
        // Read here just for this; a suite that fails to load says why when it's shown.
        let Ok(suite) = Suite::load(&root) else {
            continue;
        };
        let worn = suite
            .default_skin
            .as_ref()
            .and_then(|name| suite.skins.iter().find(|s| &s.name == name));
        let Some(skin) = worn.or(suite.skins.first()) else {
            continue;
        };
        let head = suite.gaze.as_ref().and_then(|gaze| gaze.head.clone());
        let kind = Kind::CloseUp { folder: folder.clone(), head };
        look_up(&mut studio, &mut thumbnails, &mut images, &suite, &skin.name, &skin.file, kind);
    }
}

// A suite shown for the first time this run has its outfits' pictures read, or queued to be
// taken.
fn find_outfits(
    suites: Query<&ActiveSuite, Changed<ActiveSuite>>,
    mut studio: ResMut<Studio>,
    mut thumbnails: ResMut<Thumbnails>,
    mut images: ResMut<Assets<Image>>,
) {
    for suite in &suites {
        for skin in &suite.skins {
            if !studio.known.insert((suite.root.clone(), skin.name.clone())) {
                continue;
            }
            look_up(
                &mut studio,
                &mut thumbnails,
                &mut images,
                suite,
                &skin.name,
                &skin.file,
                Kind::Outfit,
            );
        }
    }
}

// Reads the picture kept from an earlier run, or else queues it.
fn look_up(
    studio: &mut Studio,
    thumbnails: &mut Thumbnails,
    images: &mut Assets<Image>,
    suite: &Suite,
    skin: &str,
    file: &Path,
    kind: Kind,
) {
    let front = suite
        .gaze
        .as_ref()
        .filter(|gaze| gaze.front.is_some())
        .map_or(Vec3::Z, |gaze| Vec3::from_array(gaze.front_gltf()));
    let Some(stamp) = suite.files.stamp(file) else {
        warn!("thumbnail of {skin:?} in {}: cannot read its file", suite.root.display());
        return;
    };
    let job = Job {
        suite: suite.root.clone(),
        skin: skin.to_string(),
        file: file.to_path_buf(),
        stamp,
        front: front.with_y(0.0).try_normalize().unwrap_or(Vec3::Z),
        cache: PathBuf::new(),
        kind,
    };
    let cache = cache_path(&job);
    match fs::read(&cache) {
        Ok(pixels) if pixels.len() == (SIZE * SIZE * 4) as usize => {
            let image = images.add(picture(pixels));
            thumbnails.insert(&job, image);
        }
        _ => studio.queue.push_back(Job { cache, ..job }),
    }
}

fn shoot(
    mut commands: Commands,
    time: Res<Time<Real>>,
    assets: Res<AssetServer>,
    gltfs: Res<Assets<Gltf>>,
    mut images: ResMut<Assets<Image>>,
    mut studio: ResMut<Studio>,
    compiling: Res<Compiling>,
    outfits: Query<(Entity, Has<OutfitReady>), With<StudioOutfit>>,
    bounds: Query<(&Aabb, &GlobalTransform)>,
    children: Query<&Children>,
    bones: Query<(&Name, &GlobalTransform)>,
) {
    let now = time.elapsed();
    let Some(shoot) = studio.shoot.as_mut() else {
        if let Some(job) = studio.queue.pop_front() {
            let shoot = start(&mut commands, &assets, &mut images, job, now);
            studio.shoot = Some(shoot);
        }
        return;
    };
    if compiling.0.load(Ordering::Relaxed) {
        shoot.quiet = None;
    } else if shoot.quiet.is_none() {
        shoot.quiet = Some(now);
    }
    if now - shoot.started > GIVE_UP {
        give_up(&mut studio, &mut commands, "gave up, it took too long");
        return;
    }
    match shoot.step {
        Step::Loading => {
            if assets.load_state(shoot.gltf.id()).is_failed() {
                give_up(&mut studio, &mut commands, "its file failed to load");
                return;
            }
            if !assets.is_loaded_with_dependencies(shoot.gltf.id()) {
                return;
            }
            let Some(scene) = gltfs.get(&shoot.gltf).and_then(|gltf| {
                gltf.default_scene.clone().or_else(|| gltf.scenes.first().cloned())
            }) else {
                give_up(&mut studio, &mut commands, "its file holds no scene");
                return;
            };
            commands.spawn((
                Name::new("Studio outfit"),
                StudioOutfit,
                WorldAssetRoot(scene),
                Transform::default(),
                Visibility::default(),
                ChildOf(shoot.set),
            ));
            shoot.step = Step::Spawning;
        }
        Step::Spawning => {
            if outfits.iter().any(|(_, ready)| ready) {
                shoot.step = Step::Framing;
            }
        }
        Step::Framing => {
            let Some((outfit, _)) = outfits.iter().next() else {
                return;
            };
            let Some(mut bounds) = bounds_of(outfit, &children, &bounds) else {
                return;
            };
            if let Kind::CloseUp { head, .. } = &shoot.job.kind {
                let head = head.as_deref().and_then(|head| {
                    let named = |&e: &Entity| bones.get(e).is_ok_and(|(n, _)| n.as_str() == head);
                    let bone = children.iter_descendants(outfit).find(named)?;
                    bones.get(bone).ok().map(|(_, at)| at.translation())
                });
                bounds = close_up(bounds, head);
            }
            spawn_camera_and_lights(&mut commands, shoot, bounds);
            shoot.framed = now;
            shoot.step = Step::Taking;
        }
        Step::Taking => {}
    }
}

// The part of the whole figure's box a close-up frames: from the top of the head down a share
// of the height, as wide as it is tall, around the head bone when there is one and else the
// middle. Arms out in a T-pose are left out at the sides.
fn close_up((min, max): (Vec3, Vec3), head: Option<Vec3>) -> (Vec3, Vec3) {
    let height = (max.y - min.y) * CLOSE_UP_SHARE;
    let middle = head.unwrap_or((min + max) / 2.0);
    let half = height / 2.0;
    (
        Vec3::new(middle.x - half, max.y - height, min.z.max(middle.z - half)),
        Vec3::new(middle.x + half, max.y, max.z.min(middle.z + half)),
    )
}

// Leaves the picture untaken for this run, with why.
fn give_up(studio: &mut Studio, commands: &mut Commands, why: &str) {
    if let Some(shoot) = studio.shoot.take() {
        warn!("thumbnail of {}: {why}", shoot.job.label());
        commands.entity(shoot.set).despawn();
    }
}

fn start(
    commands: &mut Commands,
    assets: &AssetServer,
    images: &mut Assets<Image>,
    job: Job,
    now: Duration,
) -> Shoot {
    let gltf = load_gltf_in(assets, &job.suite, &job.file);
    let mut target = Image::new_target_texture(SIZE, SIZE, TextureFormat::Rgba8UnormSrgb, None);
    target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let target = images.add(target);
    // Everything under the set takes its layer, the outfit, the camera and the lights alike.
    let set = commands
        .spawn((
            Name::new(format!("Studio for {}", job.label())),
            StudioSet,
            Transform::default(),
            Visibility::default(),
            Propagate(RenderLayers::layer(STUDIO_LAYER)),
        ))
        .id();
    info!("thumbnail of {}: taking it", job.label());
    Shoot {
        job,
        gltf,
        set,
        target,
        started: now,
        framed: now,
        quiet: None,
        step: Step::Loading,
        last: None,
    }
}

// A box around everything the outfit draws, from the bounds Bevy works out for its meshes.
fn bounds_of(
    outfit: Entity,
    children: &Query<&Children>,
    bounds: &Query<(&Aabb, &GlobalTransform)>,
) -> Option<(Vec3, Vec3)> {
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    for entity in children.iter_descendants(outfit) {
        let Ok((aabb, transform)) = bounds.get(entity) else {
            continue;
        };
        let (center, half) = (Vec3::from(aabb.center), Vec3::from(aabb.half_extents));
        for corner in [-1.0, 1.0].into_iter().flat_map(|x| {
            [-1.0, 1.0].into_iter().flat_map(move |y| [-1.0, 1.0].map(|z| Vec3::new(x, y, z)))
        }) {
            let point = transform.transform_point(center + half * corner);
            min = min.min(point);
            max = max.max(point);
        }
    }
    (min.x <= max.x).then_some((min, max))
}

// A three-quarter view from a little above, turned from the character's front towards its
// left, the way most figures are drawn. Lit by a key light from the front left, a dimmer fill
// from the right and a rim light from behind, in Blender's units: a sun of 1 is 683 lux.
fn spawn_camera_and_lights(commands: &mut Commands, shoot: &Shoot, (min, max): (Vec3, Vec3)) {
    const FOV_DEG: f32 = 30.0;
    let front = shoot.job.front;
    let turn = |degrees: f32| Quat::from_rotation_y(degrees.to_radians()) * front;
    let towards = (turn(35.0) + Vec3::Y * 0.2).normalize();
    let center = (min + max) / 2.0;
    // Back far enough that every corner of the box is in view, with a little room around:
    // arms out in a T-pose turn partly away from a three-quarter view, which a sphere around
    // the box wouldn't know.
    let look = Transform::from_translation(center + towards).looking_at(center, Vec3::Y);
    let (right, up) = (look.right(), look.up());
    let slope = (FOV_DEG.to_radians() / 2.0).tan();
    let half = (max - min) / 2.0;
    let distance = [-1.0, 1.0]
        .into_iter()
        .flat_map(|x| {
            [-1.0, 1.0].into_iter().flat_map(move |y| [-1.0, 1.0].map(|z| Vec3::new(x, y, z)))
        })
        .map(|corner| {
            let at = half * corner;
            let reach = at.dot(*right).abs().max(at.dot(*up).abs());
            at.dot(towards) + reach / slope
        })
        .fold(0.0_f32, f32::max)
        * 1.06;
    let camera =
        Transform::from_translation(center + towards * distance).looking_at(center, Vec3::Y);
    let set = shoot.set;
    commands.spawn((
        Name::new("Studio camera"),
        Camera3d::default(),
        Camera { order: -10, clear_color: ClearColorConfig::Custom(Color::NONE), ..default() },
        RenderTarget::from(shoot.target.clone()),
        Projection::Perspective(PerspectiveProjection { fov: FOV_DEG.to_radians(), ..default() }),
        Exposure { ev100: Exposure::EV100_BLENDER },
        camera,
        ChildOf(set),
    ));
    let sun = |strength: f32, from: Vec3| {
        (
            DirectionalLight {
                illuminance: strength * 683.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(from).looking_at(Vec3::ZERO, Vec3::Y),
            ChildOf(set),
        )
    };
    commands.spawn(sun(3.0, turn(-40.0) + Vec3::Y * 0.8));
    commands.spawn(sun(1.0, turn(70.0) + Vec3::Y * 0.2));
    commands.spawn(sun(2.0, -front + Vec3::Y * 0.6));
    commands.spawn((Readback::texture(shoot.target.clone()), ChildOf(set))).observe(take);
}

// Every frame's picture comes back here until one is kept.
fn take(
    read: On<ReadbackComplete>,
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut studio: ResMut<Studio>,
    mut thumbnails: ResMut<Thumbnails>,
    mut images: ResMut<Assets<Image>>,
) {
    let now = time.elapsed();
    let Some(shoot) = studio.shoot.as_mut() else {
        return;
    };
    let pixels = read.data.clone();
    let drawn = pixels.as_chunks::<4>().0.iter().any(|pixel| pixel[3] > 0);
    let quiet = shoot.quiet.is_some_and(|since| now - since > QUIET);
    let settled = quiet && shoot.last.as_ref() == Some(&pixels) && now - shoot.framed > SETTLE;
    let late = now - shoot.started > GIVE_UP / 2;
    if !(drawn && (settled || late)) {
        shoot.last = Some(pixels);
        return;
    }
    let Some(shoot) = studio.shoot.take() else {
        return;
    };
    commands.entity(shoot.set).despawn();
    keep(&shoot.job, &pixels);
    thumbnails.insert(&shoot.job, images.add(picture(pixels)));
    info!("thumbnail of {}: taken", shoot.job.label());
}

// Written beside the suite, and any older picture of the same outfit removed. A failure only
// means it's taken again next time.
fn keep(job: &Job, pixels: &[u8]) {
    let Some(folder) = job.cache.parent() else {
        return;
    };
    let written = fs::create_dir_all(folder).and_then(|()| fs::write(&job.cache, pixels));
    if let Err(err) = written {
        warn!("thumbnail of {}: cannot keep it in {}: {err}", job.label(), folder.display());
        return;
    }
    let older = format!("{}.", job.name());
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let same_skin = name.strip_prefix(&older).is_some_and(|rest| {
            rest.strip_suffix(".rgba").is_some_and(|hash| hash.len() == 16 && !hash.contains('.'))
        });
        if same_skin && path != job.cache {
            let _ = fs::remove_file(path);
        }
    }
}

fn cache_path(job: &Job) -> PathBuf {
    let mut hash = Fnv::default();
    hash.write(job.stamp.as_bytes());
    hash.write(&[VERSION]);
    hash.write(&SIZE.to_le_bytes());
    // A close-up's hash differs from the outfit's own picture's, and changes with its head.
    if let Kind::CloseUp { head, .. } = &job.kind {
        hash.write(head.as_deref().unwrap_or_default().as_bytes());
    }
    job.suite.join(FOLDER).join(format!("{}.{:016x}.rgba", job.name(), hash.0))
}

// FNV-1a: tiny, and the same on every run and every Rust version, unlike the standard hasher.
struct Fnv(u64);

impl Default for Fnv {
    fn default() -> Self {
        Fnv(0xcbf2_9ce4_8422_2325)
    }
}

impl Fnv {
    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
        }
    }
}

fn picture(pixels: Vec<u8>) -> Image {
    Image::new(
        Extent3d { width: SIZE, height: SIZE, depth_or_array_layers: 1 },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}
