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

// The monitors the wallpaper belongs on, the primary one first when it is among them. Empty
// only while no monitor is known yet.
#[derive(Resource, Default, PartialEq)]
pub(crate) struct Targets(pub Vec<DesktopMonitor>);

pub(crate) struct MonitorsPlugin {
    pub screen: ScreenSettings,
}

impl Plugin for MonitorsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.screen.clone())
            .init_resource::<Monitors>()
            .init_resource::<Layout>()
            .init_resource::<Targets>()
            .add_systems(
                PreUpdate,
                (
                    read_layout,
                    pick_targets.run_if(
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
    monitors.sort_by_key(|m| (m.area.x, m.area.y));
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

fn pick_targets(
    layout: Res<Layout>,
    screen: Res<ScreenSettings>,
    mut targets: ResMut<Targets>,
    mut listed: ResMut<Monitors>,
) {
    let chosen = |m: &&DesktopMonitor| screen.monitors.iter().any(|c| c.id == m.id);
    let mut next: Vec<DesktopMonitor> = layout.0.iter().filter(chosen).cloned().collect();
    if next.is_empty() {
        next.extend(layout.0.iter().find(|m| m.primary).or(layout.0.first()).cloned());
        if !screen.monitors.is_empty() && !next.is_empty() {
            info!("none of the chosen monitors is connected; the primary one stands in");
        }
    }
    // The primary window goes first, so it stays where it is for as long as it can.
    next.sort_by_key(|m| !m.primary);

    let ids = |list: &[DesktopMonitor]| list.iter().map(|m| m.id.clone()).collect::<Vec<_>>();
    if ids(&next) != ids(&targets.0) {
        let names: Vec<String> = next
            .iter()
            .map(|m| format!("{} ({}×{})", m.name, m.area.width, m.area.height))
            .collect();
        info!("the wallpaper shows on {}", names.join(", "));
    }
    let entries = layout
        .0
        .iter()
        .map(|m| MonitorEntry {
            id: m.id.clone(),
            name: m.name.clone(),
            width: m.area.width,
            height: m.area.height,
            shown: next.iter().any(|t| t.id == m.id),
        })
        .collect();
    listed.set_if_neq(Monitors(entries));
    targets.set_if_neq(Targets(next));
}
