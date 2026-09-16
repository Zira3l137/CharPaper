//! Replays forwarded pointer input as if winit had delivered it.
//!
//! Writing the same messages `bevy_winit` writes means everything downstream
//! (`ButtonInput<MouseButton>`, picking, UI observers) works unchanged.
//!
//! One thing it deliberately does not do is touch `Window::cursor_position`.
//! `bevy_winit` treats a change to that field as a request to *move the OS
//! cursor*, so updating it from here would drag the real mouse around. As a
//! result `Window::cursor_position()` stays `None` and the legacy `Interaction`
//! component, which reads it, never changes. Use picking events, or read the
//! `PointerLocation` of the mouse pointer entity, instead.

use bevy::ecs::system::SystemParam;
use bevy::input::ButtonState;
use bevy::input::mouse::MouseButtonInput;
use bevy::input::mouse::MouseScrollUnit;
use bevy::input::mouse::MouseWheel;
use bevy::input::touch::TouchPhase;
use bevy::platform::cell::SyncCell;
use bevy::prelude::*;
use bevy::window::CursorEntered;
use bevy::window::CursorLeft;
use bevy::window::CursorMoved;
use bevy::window::PrimaryWindow;
use bevy::window::WindowEvent;
use charpaper_wallpaper::PointerButton;
use charpaper_wallpaper::PointerEvent;
use charpaper_wallpaper::PointerSource;

/// `SyncCell` supplies the `Sync` a `Resource` needs, which `PointerSource`
/// does not promise; it only ever hands out `&mut`, so no lock is involved.
#[derive(Resource)]
pub struct PointerSourceResource(SyncCell<Box<dyn PointerSource>>);

impl PointerSourceResource {
    pub fn new(source: Box<dyn PointerSource>) -> Self {
        Self(SyncCell::new(source))
    }
}

/// Picking reads the combined `WindowEvent` stream, while `ButtonInput` and
/// most user code read the individual messages. winit writes both, so we do.
#[derive(SystemParam)]
pub struct WindowInputWriters<'w> {
    all: MessageWriter<'w, WindowEvent>,
    entered: MessageWriter<'w, CursorEntered>,
    left: MessageWriter<'w, CursorLeft>,
    moved: MessageWriter<'w, CursorMoved>,
    buttons: MessageWriter<'w, MouseButtonInput>,
    wheel: MessageWriter<'w, MouseWheel>,
}

pub fn replay_forwarded_pointer(
    mut source: ResMut<PointerSourceResource>,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    mut writers: WindowInputWriters,
    mut pending: Local<Vec<PointerEvent>>,
    mut last_position: Local<Option<Vec2>>,
) {
    // Drained even without a window, so the backend's queue cannot back up.
    source.0.get().drain(&mut pending);
    let Ok((window, settings)) = windows.single() else {
        pending.clear();
        return;
    };
    let scale = settings.scale_factor() as f64;

    for event in pending.drain(..) {
        let w = &mut writers;
        match event {
            PointerEvent::Entered => write(&mut w.entered, &mut w.all, CursorEntered { window }),
            PointerEvent::Left => {
                *last_position = None;
                write(&mut w.left, &mut w.all, CursorLeft { window });
            }
            PointerEvent::Moved { x, y } => {
                let position = Vec2::new((x / scale) as f32, (y / scale) as f32);
                let delta = last_position.map(|last| position - last);
                *last_position = Some(position);
                write(&mut w.moved, &mut w.all, CursorMoved { window, position, delta });
            }
            PointerEvent::Button { button, pressed } => {
                let state = if pressed { ButtonState::Pressed } else { ButtonState::Released };
                let input = MouseButtonInput { button: mouse_button(button), state, window };
                write(&mut w.buttons, &mut w.all, input);
            }
            PointerEvent::Wheel { x, y } => {
                let unit = MouseScrollUnit::Line;
                let wheel = MouseWheel { unit, x, y, window, phase: TouchPhase::Moved };
                write(&mut w.wheel, &mut w.all, wheel);
            }
        }
    }
}

fn write<M: Message + Clone + Into<WindowEvent>>(
    specific: &mut MessageWriter<M>,
    all: &mut MessageWriter<WindowEvent>,
    message: M,
) {
    all.write(message.clone().into());
    specific.write(message);
}

fn mouse_button(button: PointerButton) -> MouseButton {
    match button {
        PointerButton::Left => MouseButton::Left,
        PointerButton::Right => MouseButton::Right,
        PointerButton::Middle => MouseButton::Middle,
        PointerButton::Back => MouseButton::Back,
        PointerButton::Forward => MouseButton::Forward,
    }
}
