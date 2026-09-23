//! Settings for the scene, owned by the crate that uses them.

use bevy::prelude::Resource;

/// Derives `Resource` so the systems can read it with `Res<SceneConfig>`.
///
/// In Bevy 0.19 `Resource` is built on top of `Component`, so it has to be
/// *derived* -- a hand-written `impl Resource` will not compile.
#[derive(Resource, Clone, Debug)]
pub struct SceneConfig {
    /// Degrees per second the cube spins around its Y axis.
    pub spin_speed_deg: f32,

    /// Background colour, linear sRGB 0.0-1.0.
    pub clear_color: [f32; 3],

    pub cube_color: [f32; 3],

    pub cube_size: f32,

    /// Where the camera sits, in world units.
    pub camera_pos: [f32; 3],
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self {
            spin_speed_deg: 45.0,
            clear_color: [0.05, 0.06, 0.09],
            cube_color: [0.35, 0.65, 0.95],
            cube_size: 1.5,
            camera_pos: [0.0, 1.0, 1.0],
        }
    }
}
