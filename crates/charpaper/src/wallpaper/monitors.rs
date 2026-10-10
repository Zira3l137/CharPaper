use std::time::Duration;

use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::window::Monitor;
use bevy::window::PrimaryMonitor;
use charpaper_scene::MonitorEntry;
use charpaper_scene::Monitors;
use charpaper_scene::ScreenSettings;
use charpaper_wallpaper::DesktopMonitor;
use charpaper_wallpaper::ScreenArea;

use crate::wallpaper::Backend;

// Neither the OS nor winit says when monitors change, so they are read again this often.
const POLL: Duration = Duration::from_secs(1);

// Every monitor and where it is on screen, as last read.
#[derive(Resource, Default, PartialEq)]
pub(crate) struct Layout(pub Vec<DesktopMonitor>);

// The monitor the wallpaper belongs on. None only while no monitor is known yet.
#[derive(Resource, Default, PartialEq)]
pub(crate) struct Target(pub Option<DesktopMonitor>);

pub(crate) struct MonitorsPlugin {
    pub screen: ScreenSettings,
}

impl Plugin for MonitorsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.screen.clone())
            .init_resource::<Monitors>()
            .init_resource::<Layout>()
            .init_resource::<Target>()
            .add_systems(
                PreUpdate,
                (
                    read_layout,
                    pick_target.run_if(
                        resource_changed::<Layout>.or_eager(resource_changed::<ScreenSettings>),
                    ),
                )
                    .chain(),
            );
    }
}

fn read_layout(
    _main_thread: NonSendMarker,
    time: Res<Time<Real>>,
    mut last_read: Local<Option<Duration>>,
    mut backend: ResMut<Backend>,
    bevy_monitors: Query<(&Monitor, Has<PrimaryMonitor>)>,
    mut layout: ResMut<Layout>,
    mut listed: ResMut<Monitors>,
) {
    let now = time.elapsed();
    // Every frame until something is found: attaching waits on it.
    if !layout.0.is_empty() && last_read.is_some_and(|last| now - last < POLL) {
        return;
    }
    *last_read = Some(now);

    let mut monitors = backend.0.monitors();
    if monitors.is_empty() {
        monitors =
            bevy_monitors.iter().map(|(monitor, primary)| from_bevy(monitor, primary)).collect();
    }
    let entries = monitors
        .iter()
        .map(|m| MonitorEntry {
            id: m.id.clone(),
            name: m.name.clone(),
            width: m.area.width,
            height: m.area.height,
        })
        .collect();
    listed.set_if_neq(Monitors(entries));
    layout.set_if_neq(Layout(monitors));
}

// Bevy's monitors have no lasting id, so the name stands in, or the position without one.
fn from_bevy(monitor: &Monitor, primary: bool) -> DesktopMonitor {
    let position = monitor.physical_position;
    let name = monitor.name.clone().unwrap_or_else(|| format!("{}, {}", position.x, position.y));
    DesktopMonitor {
        id: name.clone(),
        name,
        area: ScreenArea {
            x: position.x,
            y: position.y,
            width: monitor.physical_width,
            height: monitor.physical_height,
        },
        scale: monitor.scale_factor,
        primary,
    }
}

fn pick_target(layout: Res<Layout>, screen: Res<ScreenSettings>, mut target: ResMut<Target>) {
    let chosen = screen.monitor.as_deref().and_then(|id| layout.0.iter().find(|m| m.id == id));
    let primary = || layout.0.iter().find(|m| m.primary).or(layout.0.first());
    let next = chosen.or_else(primary).cloned();

    let id = |monitor: &Option<DesktopMonitor>| monitor.as_ref().map(|m| m.id.clone());
    if let Some(monitor) = next.as_ref().filter(|_| id(&next) != id(&target.0)) {
        let area = monitor.area;
        info!("the wallpaper goes on {} ({}×{})", monitor.name, area.width, area.height);
        if screen.monitor.is_some() && chosen.is_none() {
            info!("the chosen monitor isn't connected; the primary one stands in until it is");
        }
    }
    target.set_if_neq(Target(next));
}
