use std::collections::BTreeMap;

use bevy::camera::Exposure;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::ecs::system::SystemParam;
use bevy::light::EnvironmentMapLight;
use bevy::light::Skybox;
use bevy::post_process::bloom::Bloom;
use bevy::post_process::bloom::BloomCompositeMode;
use bevy::post_process::bloom::BloomPrefilter;
use bevy::post_process::effect_stack;
use bevy::post_process::effect_stack::ChromaticAberration;
use bevy::prelude::*;
use bevy::render::view::ColorGrading;
use bevy::render::view::ColorGradingGlobal;
use bevy::render::view::ColorGradingSection;
use charpaper_suite::GradingSection;
use charpaper_suite::Look;
use charpaper_suite::LookEdits;
use charpaper_suite::Suite;
use charpaper_suite::Tonemapping as SuiteTonemapping;

use crate::SceneSet;
use crate::camera::SceneCamera;
use crate::environment::EnvironmentReady;
use crate::environment::ShownEnvironment;
use crate::environment::light_listed;
use crate::environment::shown_environment;
use crate::environment::spawn_environment_scenes;
use crate::render::SceneImage;
use crate::render::ScreenParts;
use crate::stage::FocusedStage;
use crate::stage::StageOf;
use crate::suite::ActiveSuite;
use crate::suite::folder_of;

// Applies the look, the settings the viewer can change while the app runs, to the camera
// and the environment's lights. Recomputed whole on every change, so no value is kept twice.
//
// Each stage has a look of its own: the author's from suite.toml, with that stage's edits on
// top. Edits never touch suite.toml; the app saves them with the stage.
pub(crate) struct LookPlugin;

impl Plugin for LookPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<RestoreLook>()
            .add_systems(Update, (restore_look, note_edits).chain().in_set(SceneSet::Run))
            .add_systems(
                Update,
                (apply_look, apply_grain)
                    .run_if(look_needs_applying)
                    .in_set(SceneSet::Run)
                    .after(spawn_environment_scenes),
            );
    }
}

pub const DEFAULT_TONEMAPPING: SuiteTonemapping = SuiteTonemapping::TonyMcMapface;
pub const DEFAULT_EXPOSURE: f32 = 0.0;
pub const DEFAULT_BLOOM: f32 = 0.0;
pub const DEFAULT_BLOOM_THRESHOLD: f32 = 1.0;
// Light a little under the threshold glows a little, so the glow has no hard edge.
const BLOOM_SOFTNESS: f32 = 0.5;
pub const DEFAULT_CHROMATIC_ABERRATION: f32 = 0.0;
pub const DEFAULT_BRIGHTNESS: f32 = 1000.0;
pub const DEFAULT_SHADOWS: bool = true;

#[derive(Component, Deref, DerefMut, Clone, Debug, Default)]
pub struct ActiveLook(pub Look);

// Whether the look shown differs from the author's, which is when there is something to
// restore.
#[derive(Component, Default, Debug, PartialEq)]
pub struct LookEdited(pub bool);

// Takes the focused stage's look back to the suite's own.
#[derive(Message, Debug)]
pub struct RestoreLook;

// A stage's edits to each suite's look, by the suite's folder, for the suites it is not
// showing. The one shown is in ActiveLook.
#[derive(Component, Default, Debug)]
pub(crate) struct EditedLooks(pub BTreeMap<String, LookEdits>);

impl EditedLooks {
    pub fn look_for(&self, suite: &Suite) -> Look {
        let mut look = suite.look();
        if let Some(edits) = self.0.get(&folder_of(suite)) {
            edits.apply(&mut look);
        }
        look
    }

    pub fn keep(&mut self, suite: &Suite, look: &Look) {
        let edits = LookEdits::between(&suite.look(), look);
        match edits.is_empty() {
            true => self.0.remove(&folder_of(suite)),
            false => self.0.insert(folder_of(suite), edits),
        };
    }
}

fn note_edits(
    mut stages: Query<(&ActiveSuite, &ActiveLook, &mut LookEdited), Changed<ActiveLook>>,
) {
    for (suite, look, mut edited) in &mut stages {
        edited.set_if_neq(LookEdited(look.0 != suite.look()));
    }
}

fn restore_look(
    mut requests: MessageReader<RestoreLook>,
    focused: Res<FocusedStage>,
    mut stages: Query<(&ActiveSuite, &mut ActiveLook)>,
) {
    if requests.read().count() == 0 {
        return;
    }
    let Some((suite, mut look)) = focused.0.and_then(|stage| stages.get_mut(stage).ok()) else {
        return;
    };
    look.0 = suite.look();
    info!("the look of suite {:?} is the author's again", suite.name);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resolved {
    pub tonemapping: SuiteTonemapping,
    pub bloom: f32,
    pub bloom_threshold: f32,
    pub chromatic_aberration: f32,
    pub vignette: f32,
    pub vignette_size: f32,
    pub grain: f32,
    pub grain_size: f32,
    pub warmth: f32,
    pub tint: f32,
    pub saturation: f32,
    pub contrast: f32,
    pub lut_strength: f32,
    pub brightness: f32,
    pub shadows: bool,
    pub exposure: f32,
}

impl Resolved {
    pub fn new(look: &Look, environment: Option<&str>) -> Self {
        let entry = environment.map(|name| look.environment(name)).unwrap_or_default();
        Self {
            tonemapping: look.post.tonemapping.unwrap_or(DEFAULT_TONEMAPPING),
            bloom: look.post.bloom.unwrap_or(DEFAULT_BLOOM),
            bloom_threshold: look.post.bloom_threshold.unwrap_or(DEFAULT_BLOOM_THRESHOLD),
            chromatic_aberration: look
                .post
                .chromatic_aberration
                .unwrap_or(DEFAULT_CHROMATIC_ABERRATION),
            vignette: look.post.vignette.intensity(),
            vignette_size: look.post.vignette.size(),
            grain: look.post.grain.intensity(),
            grain_size: look.post.grain.size(),
            warmth: look.post.grading.warmth(),
            tint: look.post.grading.tint(),
            saturation: look.post.grading.saturation(),
            contrast: look.post.grading.contrast(),
            lut_strength: look.post.lut.strength(),
            brightness: entry.brightness.unwrap_or(DEFAULT_BRIGHTNESS),
            shadows: entry.shadows.unwrap_or(DEFAULT_SHADOWS),
            exposure: entry.exposure.or(look.post.exposure).unwrap_or(DEFAULT_EXPOSURE),
        }
    }
}

// As Blender's Glare node does it: only light past the threshold glows, and the glow is added
// on top of the image. Bevy's own default instead fades the whole image towards a blurred copy
// of itself, which at full strength is nothing but the blur.
fn bloom(resolved: &Resolved) -> Bloom {
    Bloom {
        intensity: resolved.bloom,
        prefilter: BloomPrefilter {
            threshold: resolved.bloom_threshold,
            threshold_softness: BLOOM_SOFTNESS,
        },
        composite_mode: BloomCompositeMode::Additive,
        ..Bloom::NATURAL
    }
}

type LookedCameras<'w, 's> = Query<
    'w,
    's,
    (Entity, Option<&'static mut Skybox>, Option<&'static mut EnvironmentMapLight>),
    With<SceneCamera>,
>;
type LookedLights<'w, 's> = Query<
    'w,
    's,
    AnyOf<(&'static mut DirectionalLight, &'static mut PointLight, &'static mut SpotLight)>,
>;

pub(crate) fn apply_look(
    mut commands: Commands,
    stages: Query<(Entity, Option<&ActiveLook>, &ShownEnvironment, Option<&ActiveSuite>)>,
    mut cameras: LookedCameras,
    stage_of: StageOf,
    tree: LightTree,
    mut lights: LookedLights,
) {
    for (stage, look, shown, suite) in &stages {
        let look = look.map(|l| l.0.clone()).unwrap_or_default();
        let mine: Vec<Entity> = cameras
            .iter()
            .map(|(camera, ..)| camera)
            .filter(|&c| stage_of.get(c) == Some(stage))
            .collect();
        let unshadowed = shown_environment(suite, shown)
            .map(|e| e.settings.no_shadows.as_slice())
            .unwrap_or_default();
        let lit = (unshadowed, &tree, &mut lights);
        apply_stage_look(&mut commands, &look, shown, &mine, &mut cameras, lit);
    }
}

#[derive(SystemParam)]
pub(crate) struct LightTree<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    parents: Query<'w, 's, &'static ChildOf>,
    names: Query<'w, 's, &'static Name>,
}

fn apply_stage_look(
    commands: &mut Commands,
    look: &Look,
    shown: &ShownEnvironment,
    mine: &[Entity],
    cameras: &mut LookedCameras,
    (unshadowed, tree, lights): (&[String], &LightTree, &mut LookedLights),
) {
    let resolved = Resolved::new(look, shown.name.as_deref());

    for &camera in mine {
        let Ok((entity, skybox, environment_light)) = cameras.get_mut(camera) else {
            continue;
        };
        // A higher EV100 is darker; a higher look exposure is brighter.
        let ev100 = Exposure::EV100_BLENDER - resolved.exposure;
        commands.entity(entity).insert((to_bevy(resolved.tonemapping), Exposure { ev100 }));
        if resolved.bloom > 0.0 {
            commands.entity(entity).insert(bloom(&resolved));
        } else {
            commands.entity(entity).remove::<Bloom>();
        }
        if resolved.chromatic_aberration > 0.0 {
            let intensity = resolved.chromatic_aberration;
            commands.entity(entity).insert(ChromaticAberration { intensity, ..default() });
        } else {
            commands.entity(entity).remove::<ChromaticAberration>();
        }
        let vignette = &look.post.vignette;
        if vignette.intensity() > 0.0 {
            let [r, g, b] = vignette.color();
            commands.entity(entity).insert(effect_stack::Vignette {
                intensity: vignette.intensity(),
                radius: vignette.size(),
                smoothness: vignette.falloff(),
                roundness: vignette.roundness(),
                color: Color::srgb(r, g, b),
                ..default()
            });
        } else {
            commands.entity(entity).remove::<effect_stack::Vignette>();
        }
        commands.entity(entity).insert(color_grading(look));
        if let Some(mut skybox) = skybox {
            skybox.brightness = resolved.brightness;
        }
        if let Some(mut environment_light) = environment_light {
            environment_light.intensity = resolved.brightness;
        }
    }

    // glTF can't say whether a light casts shadows. The look decides for all of them, and
    // suite.toml can leave some out, as Blender's per-light Shadow does.
    let Some(root) = shown.root else {
        return;
    };
    for entity in tree.children.iter_descendants(root) {
        let Ok((directional, point, spot)) = lights.get_mut(entity) else {
            continue;
        };
        let shadows =
            resolved.shadows && !light_listed(unshadowed, entity, &tree.names, &tree.parents);
        if let Some(mut light) = directional {
            light.shadow_maps_enabled = shadows;
            light.contact_shadows_enabled = shadows;
        }
        if let Some(mut light) = point {
            light.shadow_maps_enabled = shadows;
            light.contact_shadows_enabled = shadows;
        }
        if let Some(mut light) = spot {
            light.shadow_maps_enabled = shadows;
            light.contact_shadows_enabled = shadows;
        }
    }
}

// Always present, and neutral when the look has no grading.
fn color_grading(look: &Look) -> ColorGrading {
    let grading = &look.post.grading;
    let [temperature, tint] = grading.white_balance();
    let [start, end] = grading.midtones_range();
    let section = |section: &GradingSection| ColorGradingSection {
        saturation: section.saturation(),
        contrast: section.contrast() * grading.contrast(),
        gamma: section.gamma(),
        gain: section.gain(),
        lift: section.lift(),
    };
    ColorGrading {
        global: ColorGradingGlobal {
            temperature,
            tint,
            hue: grading.hue_deg().to_radians(),
            post_saturation: grading.saturation(),
            midtones_range: start..end,
            ..default()
        },
        shadows: section(&grading.shadows),
        midtones: section(&grading.midtones),
        highlights: section(&grading.highlights),
    }
}

pub(crate) fn apply_grain(
    screens: Query<&ScreenParts>,
    stage_of: StageOf,
    looks: Query<&ActiveLook>,
    mut materials: ResMut<Assets<SceneImage>>,
) {
    for parts in &screens {
        let look = stage_of.get(parts.camera).and_then(|stage| looks.get(stage).ok());
        let grain = look.map(|l| l.post.grain.clone()).unwrap_or_default();
        let colored = if grain.colored() { 1.0 } else { 0.0 };
        let wanted = Vec4::new(grain.intensity(), grain.size(), colored, 0.0);
        // Only on a real change: every change rebuilds the material on the GPU.
        if materials.get(&parts.material).is_some_and(|m| m.settings.grain != wanted)
            && let Some(mut material) = materials.get_mut(&parts.material)
        {
            material.settings.grain = wanted;
        }
    }
}

// A new camera, for a screen added while the app runs, needs the whole look too.
pub(crate) fn look_needs_applying(
    stages: Query<(), Or<(Changed<ActiveLook>, Changed<ShownEnvironment>)>>,
    ready: Query<(), Added<EnvironmentReady>>,
    new_cameras: Query<(), Added<SceneCamera>>,
) -> bool {
    !stages.is_empty() || !ready.is_empty() || !new_cameras.is_empty()
}

fn to_bevy(from: SuiteTonemapping) -> Tonemapping {
    match from {
        SuiteTonemapping::None => Tonemapping::None,
        SuiteTonemapping::Reinhard => Tonemapping::Reinhard,
        SuiteTonemapping::ReinhardLuminance => Tonemapping::ReinhardLuminance,
        SuiteTonemapping::AcesFitted => Tonemapping::AcesFitted,
        SuiteTonemapping::Agx => Tonemapping::AgX,
        SuiteTonemapping::SomewhatBoringDisplayTransform => {
            Tonemapping::SomewhatBoringDisplayTransform
        }
        SuiteTonemapping::TonyMcMapface => Tonemapping::TonyMcMapface,
        SuiteTonemapping::BlenderFilmic => Tonemapping::BlenderFilmic,
    }
}
