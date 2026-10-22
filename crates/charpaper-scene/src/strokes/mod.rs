// Lines along the edges of what the camera sees, for an inked, painterly look. Drawn over the
// finished scene from its depth and surface directions, which the prepasses keep: an outline
// wherever something stands in front of what's behind it, and a crease wherever the surface
// bends sharply. Each line darkens what's under it rather than covering it, so it reads as
// ink of the same color. Hand-drawn lines wander and swell a little, and redraw a few times
// a second, as hand-drawn animation does.

mod pass;

use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_resource::ShaderType;
use charpaper_settings::Graphics;

use crate::SceneSet;

// Hand-drawn lines change their wobble this many times a second.
const BOIL_RATE: f32 = 8.0;
// Lines are at full strength up to this many meters away and fade out beyond.
const FADE_DISTANCE: f32 = 15.0;

pub(crate) struct StrokesPlugin;

impl Plugin for StrokesPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(pass::StrokesPassPlugin)
            .add_systems(Update, follow_time_and_lens.in_set(SceneSet::Run));
    }
}

// On a scene camera while it draws strokes, as the shader reads it.
#[derive(Component, ExtractComponent, ShaderType, Clone, Copy, Default, PartialEq)]
pub(crate) struct SceneStrokes {
    darkening: f32,
    // In pixels.
    width: f32,
    // How sharply the surface must bend for a crease; above 1 for none.
    crease: f32,
    // How far lines wander, in pixels; 0 for straight.
    wobble: f32,
    seed: f32,
    // The depth FADE_DISTANCE away, which is the camera's near plane over that distance in
    // Bevy's reversed depth; 0 for no fading.
    fade_depth: f32,
}

pub(crate) fn scene_strokes(graphics: &Graphics) -> SceneStrokes {
    let width = graphics.stroke_width.pixels();
    SceneStrokes {
        darkening: graphics.stroke_strength.darkening(),
        width,
        crease: if graphics.stroke_creases { 0.25 } else { 2.0 },
        wobble: if graphics.stroke_hand_drawn { width } else { 0.0 },
        ..default()
    }
}

fn follow_time_and_lens(time: Res<Time>, mut cameras: Query<(&mut SceneStrokes, &Projection)>) {
    let seed = (time.elapsed_secs_wrapped() * BOIL_RATE).floor();
    for (mut strokes, projection) in &mut cameras {
        let fade_depth = match projection {
            Projection::Perspective(lens) => lens.near / FADE_DISTANCE,
            _ => 0.0,
        };
        let followed = SceneStrokes { seed, fade_depth, ..*strokes };
        strokes.set_if_neq(followed);
    }
}
