use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use charpaper_scene::Screen;
use charpaper_wallpaper::DesktopMonitor;

use crate::wallpaper::Backend;
use crate::wallpaper::WallpaperSettings;
use crate::wallpaper::attach::Attachment;
use crate::wallpaper::key;
use crate::wallpaper::monitors::Targets;

// Where a window belongs. Attaching and placing the window follow it.
#[derive(Component, Clone, PartialEq)]
pub(crate) struct OnMonitor(pub DesktopMonitor);

// A window made for a monitor beyond the first, closed again when that monitor is no longer
// wanted. The primary window is never closed: that would end the app.
#[derive(Component)]
pub(crate) struct ExtraWindow;

pub(crate) struct WindowsPlugin;

impl Plugin for WindowsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, adopt_primary_window)
            .add_systems(Update, assign_windows.run_if(resource_changed::<Targets>));
    }
}

fn adopt_primary_window(mut commands: Commands, window: Single<Entity, With<PrimaryWindow>>) {
    commands.entity(*window).insert(Attachment::default());
    commands.spawn((Name::new("Screen"), Screen { window: *window }, ChildOf(*window)));
}

// Every window keeps its monitor while that is still wanted, so a change elsewhere moves
// nothing. A window whose monitor is gone takes one nobody has, and the rest are closed or
// made.
pub(crate) fn assign_windows(
    mut commands: Commands,
    mut backend: ResMut<Backend>,
    config: Res<WallpaperSettings>,
    targets: Res<Targets>,
    primary: Single<(Entity, &Window, Option<&OnMonitor>), With<PrimaryWindow>>,
    extras: Query<(Entity, Option<&OnMonitor>), With<ExtraWindow>>,
) {
    let mut free = targets.0.clone();
    if free.is_empty() {
        return;
    }
    // Without attaching, the app is one ordinary window and never makes more.
    if !config.attaches() {
        free.truncate(1);
    }

    let (primary, template, primary_on) = primary.into_inner();
    let mut take = |on: Option<&OnMonitor>| {
        let index = free.iter().position(|m| on.is_some_and(|on| on.0.id == m.id))?;
        Some(free.remove(index))
    };
    let mut windows: Vec<(Entity, Option<&OnMonitor>, Option<DesktopMonitor>)> =
        std::iter::once((primary, primary_on))
            .chain(&extras)
            .map(|(window, on)| (window, on, take(on)))
            .collect();

    // The primary window can't close, so it must end up with a monitor: a free one, or else
    // the one another window has, which then closes.
    if windows[0].2.is_none() {
        windows[0].2 = match free.is_empty() {
            false => Some(free.remove(0)),
            true => windows.iter_mut().find_map(|(_, _, monitor)| monitor.take()),
        };
    }
    for (_, _, monitor) in windows.iter_mut().skip(1) {
        if monitor.is_none() && !free.is_empty() {
            *monitor = Some(free.remove(0));
        }
    }

    for (window, on, monitor) in windows {
        match monitor {
            Some(monitor) if on.is_none_or(|on| on.0 != monitor) => {
                commands.entity(window).insert(OnMonitor(monitor));
            }
            Some(_) => {}
            None => {
                backend.0.detach(key(window));
                commands.entity(window).despawn();
            }
        }
    }
    for monitor in free {
        spawn_window(&mut commands, template, monitor);
    }
}

// A copy of the primary window's settings, so every window looks and paces the same.
fn spawn_window(commands: &mut Commands, template: &Window, monitor: DesktopMonitor) {
    let name = Name::new(format!("Window on {}", monitor.name));
    // Hidden until attached, like the first one, and never taking the focus from the app in use.
    let window = Window { visible: false, focused: false, ..template.clone() };
    let window =
        commands.spawn((name, window, ExtraWindow, OnMonitor(monitor), Attachment::default())).id();
    commands.spawn((Name::new("Screen"), Screen { window }, ChildOf(window)));
}
