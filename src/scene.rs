//! The thing we actually draw. Nothing here knows about wallpapers or Windows.

use bevy::prelude::*;

use crate::config::SceneConfig;

#[derive(Component)]
struct Spinner;

pub struct ScenePlugin {
    pub config: SceneConfig,
}

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        let [r, g, b] = self.config.clear_color;

        app.insert_resource(ClearColor(Color::srgb(r, g, b)))
            .insert_resource(self.config.clone())
            .add_systems(Startup, spawn_scene)
            .add_systems(Update, spin_cube);
    }
}

fn spawn_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<SceneConfig>,
) {
    let [cr, cg, cb] = config.cube_color;
    let [px, py, pz] = config.camera_pos;

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(config.cube_size, config.cube_size, config.cube_size))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(cr, cg, cb),
            perceptual_roughness: 0.35,
            metallic: 0.1,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Spinner,
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(12.0, 0.2, 12.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.12, 0.13, 0.16),
            perceptual_roughness: 0.9,
            ..default()
        })),
        Transform::from_xyz(0.0, -1.6, 0.0),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::OVERCAST_DAY,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(px, py, pz).looking_at(Vec3::ZERO, Vec3::Y),
        AmbientLight { color: Color::srgb(0.6, 0.7, 1.0), brightness: 200.0, ..default() },
    ));
}

fn spin_cube(
    time: Res<Time>,
    config: Res<SceneConfig>,
    mut spinners: Query<&mut Transform, With<Spinner>>,
) {
    let radians_per_second = config.spin_speed_deg.to_radians();

    for mut transform in &mut spinners {
        transform.rotate_y(radians_per_second * time.delta_secs());
    }
}
