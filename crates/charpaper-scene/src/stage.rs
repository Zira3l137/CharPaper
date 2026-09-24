//! Everything around the character: lights, the camera and its effects.
//!
//! Each setting starts from an engine default below and is replaced by
//! whatever `suite.toml` says. A later user-override layer slots in between
//! the suite and the value used.

use bevy::prelude::*;

use crate::OrbitCamera;
use crate::suite::ActiveSuite;

const SUN_ILLUMINANCE: f32 = light_consts::lux::OVERCAST_DAY;
const SUN_COLOR: [f32; 3] = [1.0, 1.0, 1.0];
const SUN_DIRECTION: [f32; 3] = [-4.0, -8.0, -4.0];
const SUN_SHADOWS: bool = true;
const AMBIENT_COLOR: [f32; 3] = [0.6, 0.7, 1.0];
const AMBIENT_BRIGHTNESS: f32 = 200.0;

pub(crate) fn spawn_stage(mut commands: Commands, suite: Option<Res<ActiveSuite>>) {
    let lighting = suite.as_ref().map(|s| s.lighting.clone()).unwrap_or_default();
    let sun = lighting.sun.unwrap_or_default();
    let ambient = lighting.ambient.unwrap_or_default();

    let shadows = sun.shadows.unwrap_or(SUN_SHADOWS);
    commands.spawn((
        Name::new("Sun"),
        DirectionalLight {
            illuminance: sun.illuminance.unwrap_or(SUN_ILLUMINANCE),
            color: srgb(sun.color.unwrap_or(SUN_COLOR)),
            shadow_maps_enabled: shadows,
            contact_shadows_enabled: shadows,
            ..default()
        },
        // A directional light shines along its forward axis; `looking_to`
        // copes with a direction parallel to Y or of zero length.
        Transform::default()
            .looking_to(Vec3::from_array(sun.direction.unwrap_or(SUN_DIRECTION)), Vec3::Y),
    ));

    commands.spawn((
        Camera3d::default(),
        OrbitCamera::default(),
        Transform::default(),
        AmbientLight {
            color: srgb(ambient.color.unwrap_or(AMBIENT_COLOR)),
            brightness: ambient.brightness.unwrap_or(AMBIENT_BRIGHTNESS),
            ..default()
        },
    ));
}

fn srgb([r, g, b]: [f32; 3]) -> Color {
    Color::srgb(r, g, b)
}
