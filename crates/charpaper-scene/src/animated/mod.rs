// A GIF in an environment's `animated/` plays on the scene's material it is named after.
//
// The material is copied with an image of the GIF as its base color texture, and the image's
// pixels are replaced whenever the frame changes. Bevy then copies the new pixels into the
// texture already on the GPU, as long as the size and format stay the same (prepare_asset in
// bevy_render's gpu_image.rs), so the material itself never has to be redone.

mod decode;

use std::time::Duration;

use bevy::asset::RenderAssetUsages;
use bevy::gltf::GltfMaterialName;
use bevy::prelude::*;
use bevy::render::render_resource::Extent3d;
use bevy::render::render_resource::TextureDimension;
use bevy::render::render_resource::TextureFormat;
use bevy::world_serialization::WorldInstanceReady;
use crossbeam_channel::TryRecvError;

use crate::SceneSet;
use crate::animated::decode::Frames;
use crate::animated::decode::Gif;
use crate::animated::decode::Opening;
use crate::environment::ShownEnvironment;
use crate::environment::shown_environment;
use crate::suite::ActiveSuite;

pub(crate) struct AnimatedPlugin;

impl Plugin for AnimatedPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(find_gif_materials).add_systems(Update, play_gifs.in_set(SceneSet::Run));
    }
}

// A child of the environment's root, so it, its decoding thread and its image go with it.
#[derive(Component)]
struct GifPlayer {
    material: Handle<StandardMaterial>,
    meshes: Vec<Entity>,
    // Made when the first frame is ready. Until then the meshes keep the scene's material,
    // with the still picture the scene file carries.
    image: Option<Handle<Image>>,
    playback: Playback,
}

enum Playback {
    Opening(Opening),
    Playing(Playing),
    Stopped,
}

struct Playing {
    width: u32,
    height: u32,
    frames: Frames,
    at: usize,
    // For a streamed GIF: how long the frame on screen is meant to stay.
    shown_for: Duration,
    elapsed: Duration,
}

enum Step {
    Show(Vec<u8>),
    Hold,
    Ended,
}

fn find_gif_materials(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    stages: Query<(&ShownEnvironment, Option<&ActiveSuite>)>,
    children: Query<&Children>,
    meshes: Query<(&GltfMaterialName, &MeshMaterial3d<StandardMaterial>)>,
) {
    let Some((shown, suite)) = stages.iter().find(|(shown, _)| shown.root == Some(ready.entity))
    else {
        return;
    };
    let (Some(suite), Some(environment)) = (suite, shown_environment(suite, shown)) else {
        return;
    };

    for (material, path) in &environment.animated {
        let wearing: Vec<(Entity, &MeshMaterial3d<StandardMaterial>)> = children
            .iter_descendants(ready.entity)
            .filter_map(|e| meshes.get(e).ok().map(|(name, worn)| (e, name, worn)))
            .filter(|(_, name, _)| name.0 == *material)
            .map(|(e, _, worn)| (e, worn))
            .collect();
        let Some((_, worn)) = wearing.first() else {
            warn!(
                "environment {:?}: no mesh wears the material {material:?} for {}",
                environment.name,
                path.display()
            );
            continue;
        };
        commands.spawn((
            Name::new(format!("GIF on {material}")),
            GifPlayer {
                material: worn.0.clone(),
                meshes: wearing.iter().map(|&(e, _)| e).collect(),
                image: None,
                playback: Playback::Opening(decode::open(suite.absolute(path))),
            },
            ChildOf(ready.entity),
        ));
    }
}

fn play_gifs(
    time: Res<Time>,
    mut commands: Commands,
    mut players: Query<&mut GifPlayer>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for mut player in &mut players {
        let player = &mut *player;
        if let Playback::Opening(opening) = &player.playback {
            player.playback = match opening.try_recv() {
                Ok(Ok(gif)) => Playback::Playing(Playing::new(gif)),
                Ok(Err(err)) => {
                    warn!("a GIF failed to open: {err}");
                    Playback::Stopped
                }
                Err(TryRecvError::Empty) => continue,
                Err(TryRecvError::Disconnected) => Playback::Stopped,
            };
        }
        let Playback::Playing(playing) = &mut player.playback else {
            continue;
        };

        let pixels = match playing.advance(time.delta(), player.image.is_none()) {
            Step::Show(pixels) => pixels,
            Step::Hold => continue,
            // The last frame shown stays.
            Step::Ended => {
                player.playback = Playback::Stopped;
                continue;
            }
        };
        if let Some(image) = &player.image {
            if let Some(mut image) = images.get_mut(image) {
                image.data = Some(pixels);
            }
            continue;
        }

        // The scene file's material is shared by every stage showing this environment, so
        // each player dresses its own meshes in a copy.
        let Some(mut copy) = materials.get(&player.material).cloned() else {
            player.playback = Playback::Stopped;
            continue;
        };
        let mut image = gif_image(playing.width, playing.height, pixels);
        // The still the scene file carries came with the Image Texture node's Extension and
        // Interpolation, as wrap and filter modes. Without them a scaled GIF wouldn't repeat.
        // Bevy keeps an uploaded image's settings in RAM, only its pixels are dropped.
        if let Some(still) = copy.base_color_texture.as_ref().and_then(|h| images.get(h)) {
            image.sampler = still.sampler.clone();
        }
        let image = images.add(image);
        copy.base_color_texture = Some(image.clone());
        let copy = materials.add(copy);
        for &mesh in &player.meshes {
            commands.entity(mesh).try_insert(MeshMaterial3d(copy.clone()));
        }
        player.image = Some(image);
    }
}

// Kept in RAM as well as on the GPU: an image that only the GPU holds can't be changed here.
fn gif_image(width: u32, height: u32, pixels: Vec<u8>) -> Image {
    Image::new(
        Extent3d { width, height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

impl Playing {
    fn new(gif: Gif) -> Self {
        let Gif { width, height, frames } = gif;
        Self { width, height, frames, at: 0, shown_for: Duration::ZERO, elapsed: Duration::ZERO }
    }

    // `first` asks for a frame now, whatever the time: nothing has been shown yet.
    fn advance(&mut self, delta: Duration, first: bool) -> Step {
        match &self.frames {
            Frames::Kept(frames) => {
                if first {
                    return Step::Show(frames[self.at].pixels.clone());
                }
                if frames.len() < 2 {
                    return Step::Hold;
                }
                self.elapsed += delta;
                let before = self.at;
                while self.elapsed >= frames[self.at].delay {
                    self.elapsed -= frames[self.at].delay;
                    self.at = (self.at + 1) % frames.len();
                }
                if self.at == before {
                    return Step::Hold;
                }
                Step::Show(frames[self.at].pixels.clone())
            }
            Frames::Streamed(next) => {
                if !first {
                    self.elapsed += delta;
                    if self.elapsed < self.shown_for {
                        return Step::Hold;
                    }
                }
                match next.try_recv() {
                    Ok(frame) => {
                        // Never more than one frame late: a stream can't skip frames, so
                        // after a pause it carries on from where it stopped.
                        self.elapsed = self.elapsed.saturating_sub(self.shown_for).min(frame.delay);
                        self.shown_for = frame.delay;
                        Step::Show(frame.pixels)
                    }
                    // The decoder fell behind; the frame on screen stays a little longer.
                    Err(TryRecvError::Empty) => {
                        self.elapsed = self.shown_for;
                        Step::Hold
                    }
                    Err(TryRecvError::Disconnected) => Step::Ended,
                }
            }
        }
    }
}
