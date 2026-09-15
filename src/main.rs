//! Milestone 1: a spinning cube on a black background, living in the desktop
//! wallpaper layer on Windows.
//!
//! Everything platform-specific is behind `platform::`. This file is pure Bevy
//! and should compile and run unchanged on any OS (as an ordinary window
//! elsewhere).
//!
//! ## Bevy in sixty seconds, for someone who knows Rust but not Bevy
//!
//! Bevy is an ECS — entity, component, system.
//!
//! * A **component** is a plain struct. `Transform` is a component. So is
//!   `Mesh3d`. So is our `Spinner` below.
//! * An **entity** is just an id with a set of components attached. There is no
//!   `GameObject` class; a cube "is" an entity that happens to have a mesh, a
//!   material and a transform.
//! * A **system** is a plain function. Its *parameter types* declare what it
//!   wants from the world, and Bevy supplies them by inspecting the signature —
//!   dependency injection driven by types. `Query<&mut Transform, With<Spinner>>`
//!   means "every entity that has a Transform and a Spinner, mutably".
//! * A **resource** is a singleton. `Res<Time>` reads it, `ResMut<T>` mutates it.
//! * A **schedule** is when a system runs. `Startup` runs once; `Update` runs
//!   every frame.
//!
//! Because systems declare their access up front, Bevy can run
//! non-conflicting ones in parallel across threads for free. The flip side is
//! the one gotcha worth knowing now: two systems that both want `&mut` the same
//! component cannot run at the same time, and if you write a query that
//! conflicts with itself you get a runtime panic rather than a compile error.

mod platform;

use std::f32::consts::TAU;

use bevy::prelude::*;
use bevy::winit::WinitSettings;
use platform::Coverage;
use platform::DesktopLayerConfig;
use platform::DesktopLayerPlugin;
use platform::RunMode;
use platform::ZOrder;

// ===========================================================================
// SETTINGS — everything tunable lives here
// ===========================================================================

#[derive(Resource, Clone, Debug)]
pub struct Settings {
    // --- Desktop integration ---
    /// `Wallpaper` reparents into the desktop layer. `Windowed` gives you a
    /// normal window with a title bar — use it while working on the scene.
    pub run_mode: RunMode,
    /// `PrimaryMonitor` or `VirtualDesktop`. See `platform::Coverage`.
    pub coverage: Coverage,
    /// Probe and log, change nothing. Try this first on a new machine.
    pub dry_run: bool,
    /// Log the window hierarchy that was discovered.
    pub verbose: bool,
    /// Dump the full desktop window tree once at startup. Use with `dry_run`
    /// when the strategy ladder picks an unexpected rung.
    pub inspect: bool,
    /// How often to check we are still attached, in seconds.
    pub reattach_check_seconds: f32,
    /// `BelowIcons` is the goal. Switch to `AboveIcons` to find out whether the
    /// scene renders at all — if it appears over your icons, rendering is fine
    /// and something above us is opaque.
    pub z_order: ZOrder,
    /// Resize by a pixel after reparenting, forcing wgpu to rebuild its
    /// presentation surface for the window's new parent.
    pub nudge_after_attach: bool,
    /// Log platform and Bevy window state every N seconds. 0 disables.
    pub heartbeat_seconds: f32,
    /// Ctrl+Alt+Shift+Q to quit. Keep this on; a wallpaper window has no close
    /// button and Task Manager is the only other way out.
    pub quit_hotkey: bool,

    // --- Scene ---
    pub background: Color,
    pub cube_color: Color,
    pub cube_size: f32,
    /// Full rotations per second. 0.15 is a slow, calm turn.
    pub cube_turns_per_second: f32,
    pub camera_distance: f32,
    pub camera_height: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            run_mode: RunMode::Wallpaper,
            coverage: Coverage::PrimaryMonitor,
            dry_run: false,
            verbose: true,
            inspect: true,
            reattach_check_seconds: 2.0,
            z_order: ZOrder::AboveIcons,
            nudge_after_attach: true,
            heartbeat_seconds: 5.0,
            quit_hotkey: true,

            background: Color::BLACK,
            cube_color: Color::srgb(0.35, 0.62, 0.92),
            cube_size: 1.5,
            cube_turns_per_second: 0.15,
            camera_distance: 5.0,
            camera_height: 1.2,
        }
    }
}

/// Window title. Load-bearing: the platform layer uses it to pick our render
/// window out of the several top-level windows winit creates. If you change it,
/// the `window_title` field in `DesktopLayerConfig` follows automatically.
const WINDOW_TITLE: &str = "wallpaper-cube";

// ===========================================================================
// App
// ===========================================================================

fn main() {
    let settings = Settings::default();
    let windowed = settings.run_mode == RunMode::Windowed;

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: WINDOW_TITLE.into(),
                // In wallpaper mode we want no chrome at all. The platform layer
                // resizes and repositions the window, so its initial size does
                // not matter. In windowed mode, keep decorations so you can
                // close it normally.
                decorations: windowed,
                resizable: windowed,
                ..default()
            }),
            ..default()
        }))
        // The clear colour is what shows through everywhere the scene does not
        // draw. For a wallpaper this is your background, so it is opaque black
        // rather than transparent — real window transparency on Windows is a
        // fight we do not need to pick. If you later want the user's actual
        // wallpaper visible behind the scene, read its path with
        // SystemParametersInfo(SPI_GETDESKWALLPAPER) and draw it as a texture.
        .insert_resource(ClearColor(settings.background))
        // CRITICAL for wallpaper mode.
        //
        // Bevy's default desktop behaviour throttles updates when the window is
        // not focused, to be polite about battery. Our window is a child of the
        // desktop and is NEVER focused, so with the default settings you get a
        // slideshow. `WinitSettings::game()` runs continuously regardless of
        // focus.
        //
        // The cost is that we now render flat out. Frame limiting and
        // pause-when-occluded are a later milestone; until then, do not leave
        // this running on a laptop on battery.
        .insert_resource(WinitSettings::game())
        .insert_resource(settings.clone())
        .add_plugins(DesktopLayerPlugin(DesktopLayerConfig {
            mode: settings.run_mode,
            coverage: settings.coverage,
            dry_run: settings.dry_run,
            verbose: settings.verbose,
            inspect: settings.inspect,
            reattach_check_seconds: settings.reattach_check_seconds,
            window_title: WINDOW_TITLE.to_string(),
            z_order: settings.z_order,
            nudge_after_attach: settings.nudge_after_attach,
            heartbeat_seconds: settings.heartbeat_seconds,
            quit_hotkey: settings.quit_hotkey,
        }))
        .add_systems(Startup, setup_scene)
        .add_systems(Update, spin_cube)
        .run();
}

// ===========================================================================
// Scene
// ===========================================================================

/// Marker component. Carries no data; its only job is to let a query say
/// "the things that spin". This is the idiomatic ECS way to tag entities.
#[derive(Component)]
struct Spinner;

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<Settings>,
) {
    // Camera. `Camera3d` is a component; Bevy's "required components" feature
    // (since 0.15) automatically inserts everything else a 3D camera needs, so
    // there is no bundle to assemble. Bevy is right-handed, Y-up, and the camera
    // looks down its own -Z.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, settings.camera_height, settings.camera_distance)
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Directional light — a sun. Only its rotation matters, not its position,
    // but `looking_at` is the readable way to express a direction.
    //
    // Shadows are deliberately left at their default (off). Note that the field
    // was renamed from `shadows_enabled` to `shadow_maps_enabled` in Bevy 0.19,
    // since lights now also support contact shadows.
    commands.spawn((
        DirectionalLight { illuminance: 8_000.0, ..default() },
        Transform::from_xyz(4.0, 8.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // The cube. `meshes.add(...)` uploads a mesh asset and returns a handle;
    // `Mesh3d` and `MeshMaterial3d` are the components that reference them.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(settings.cube_size, settings.cube_size, settings.cube_size))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: settings.cube_color,
            perceptual_roughness: 0.35,
            ..default()
        })),
        Transform::IDENTITY,
        Spinner,
    ));
}

/// Rotate everything tagged `Spinner`.
///
/// Always scale motion by `time.delta_secs()` rather than assuming a frame rate.
/// It matters more than usual here: this app will run at your monitor's refresh
/// rate today and at a capped 30fps once frame limiting goes in, and the cube
/// should turn at the same speed either way.
fn spin_cube(
    time: Res<Time>,
    settings: Res<Settings>,
    mut spinners: Query<&mut Transform, With<Spinner>>,
) {
    let radians = TAU * settings.cube_turns_per_second * time.delta_secs();
    for mut transform in &mut spinners {
        transform.rotate_y(radians);
        // A little tumble on X so it reads as a 3D object rather than a
        // rotating square.
        transform.rotate_local_x(radians * 0.35);
    }
}
