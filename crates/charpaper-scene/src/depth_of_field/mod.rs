// Depth of field, drawn by CharPaper rather than Bevy. Bevy's blurs each pixel by its own
// distance only, so a sharp background pixel beside a blurred object in front stays sharp, and
// the object keeps a sharp outline around its blurred inside. This one also spreads what is in
// front of the focus over what is behind it, as a real lens does.
//
// It runs where Bevy's would, after bloom and before tonemapping, at half resolution apart from
// the last step. pass.rs has the steps.

mod pass;

use bevy::camera::Projection;
use bevy::ecs::query::QueryItem;
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::extract_component::ExtractComponentPlugin;
use bevy::render::sync_component::SyncComponent;

pub(crate) struct DepthOfFieldPlugin;

impl Plugin for DepthOfFieldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<SceneDepthOfField>::default(),
            pass::DepthOfFieldPassPlugin,
        ));
    }
}

// On a scene camera while it blurs what is out of focus.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(crate) struct SceneDepthOfField {
    // In meters, along the view axis.
    pub focal_distance: f32,
    pub f_stop: f32,
    // In meters.
    pub sensor_height: f32,
    // The widest blur, as a diameter in pixels of the scene image.
    pub max_blur: f32,
    // Anything further, the sky included, blurs as if it were this far, in meters.
    pub max_depth: f32,
    // Round highlights from bright spots, rather than a plain blur.
    pub bokeh: bool,
    // Taken by each blurred pixel.
    pub samples: u32,
    // Whether what is in front of the focus blurs too.
    pub foreground: bool,
    // How far bright spots outweigh the rest in a bokeh blur.
    pub highlights: f32,
}

// What the passes need, worked out from the camera's lens.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(crate) struct ExtractedDepthOfField {
    focal_distance: f32,
    focal_length: f32,
    // focal_length² / (sensor_height · f_stop).
    coc_scale: f32,
    max_radius: f32,
    max_depth: f32,
    bokeh: bool,
    samples: u32,
    foreground: bool,
    highlights: f32,
}

impl SyncComponent for SceneDepthOfField {
    type Target = ExtractedDepthOfField;
}

impl ExtractComponent for SceneDepthOfField {
    type QueryData = (&'static SceneDepthOfField, &'static Projection);
    type QueryFilter = ();
    type Out = ExtractedDepthOfField;

    // A lens blur only means something through a perspective lens.
    fn extract_component(
        (settings, projection): QueryItem<'_, '_, Self::QueryData>,
    ) -> Option<Self::Out> {
        let Projection::Perspective(perspective) = projection else {
            return None;
        };
        // From the vertical field of view, as Bevy's own depth of field works it out.
        let focal_length = 0.5 * settings.sensor_height / (0.5 * perspective.fov).tan();
        Some(ExtractedDepthOfField {
            focal_distance: settings.focal_distance,
            focal_length,
            coc_scale: focal_length * focal_length / (settings.sensor_height * settings.f_stop),
            max_radius: 0.5 * settings.max_blur,
            max_depth: settings.max_depth,
            bokeh: settings.bokeh,
            samples: settings.samples.max(1),
            foreground: settings.foreground,
            highlights: settings.highlights.max(0.0),
        })
    }
}
