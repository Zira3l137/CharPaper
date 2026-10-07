use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use bevy::ecs::system::SystemParam;
use bevy::input::ButtonState;
use bevy::input::mouse::MouseButtonInput;
use bevy::input::mouse::MouseScrollUnit;
use bevy::input::mouse::MouseWheel;
use bevy::input::touch::TouchPhase;
use bevy::picking::PickingSystems;
use bevy::platform::cell::SyncCell;
use bevy::prelude::*;
use bevy::window::CursorEntered;
use bevy::window::CursorLeft;
use bevy::window::CursorMoved;
use bevy::window::PrimaryWindow;
use bevy::window::WindowEvent;
use bevy::winit::EventLoopProxyWrapper;
use bevy::winit::WinitUserEvent;
use charpaper_wallpaper::PointerButton;
use charpaper_wallpaper::PointerEvent;
use charpaper_wallpaper::PointerSource;
use charpaper_wallpaper::Wake;

pub(crate) struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        // winit writes its input before First, so picking still sees ours this frame.
        app.init_resource::<PointerOnDesktop>().add_systems(
            First,
            replay_forwarded_pointer
                .run_if(resource_exists::<PointerSourceResource>)
                .before(PickingSystems::Input),
        );
    }
}

// SyncCell adds the Sync a Resource needs without a lock; PointerSource is only Send.
#[derive(Resource)]
pub struct PointerSourceResource(SyncCell<Box<dyn PointerSource>>);

impl PointerSourceResource {
    pub fn new(source: Box<dyn PointerSource>) -> Self {
        Self(SyncCell::new(source))
    }
}

// Whether the forwarded pointer was last seen over the desktop. The backend's input thread
// checks it through the waker, so a click in some other app doesn't wake this one.
#[derive(Resource, Default)]
pub(crate) struct PointerOnDesktop(Arc<AtomicBool>);

impl PointerOnDesktop {
    // Does for forwarded input what winit does for input sent to the window itself: pokes the
    // event loop so the app runs a frame now.
    pub(crate) fn waker(&self, proxy: Option<&EventLoopProxyWrapper>) -> Wake {
        let on_desktop = self.0.clone();
        let Some(proxy) = proxy.map(|proxy| (**proxy).clone()) else {
            return Box::new(|| {});
        };
        Box::new(move || {
            if on_desktop.load(Ordering::Relaxed) {
                let _ = proxy.send_event(WinitUserEvent::WakeUp);
            }
        })
    }
}

// Picking reads WindowEvent, most other code reads the separate messages. winit writes
// both, so we do too.
#[derive(SystemParam)]
struct WindowInputWriters<'w> {
    all: MessageWriter<'w, WindowEvent>,
    entered: MessageWriter<'w, CursorEntered>,
    left: MessageWriter<'w, CursorLeft>,
    moved: MessageWriter<'w, CursorMoved>,
    buttons: MessageWriter<'w, MouseButtonInput>,
    wheel: MessageWriter<'w, MouseWheel>,
}

// Replays forwarded input as if winit had delivered it.
//
// Window::cursor_position is never touched: bevy_winit takes a change to it as a request
// to move the real cursor. So it stays None here; use picking events instead.
fn replay_forwarded_pointer(
    mut source: ResMut<PointerSourceResource>,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    mut writers: WindowInputWriters,
    on_desktop: Res<PointerOnDesktop>,
    mut pending: Local<Vec<PointerEvent>>,
    mut last_position: Local<Option<Vec2>>,
) {
    // Drained even without a window, so the backend's queue can't back up.
    source.0.get().drain(&mut pending);
    let Ok((window, settings)) = windows.single() else {
        pending.clear();
        return;
    };
    let scale = settings.scale_factor() as f64;

    for event in pending.drain(..) {
        let w = &mut writers;
        match event {
            PointerEvent::Entered => {
                on_desktop.0.store(true, Ordering::Relaxed);
                write(&mut w.entered, &mut w.all, CursorEntered { window });
            }
            PointerEvent::Left => {
                on_desktop.0.store(false, Ordering::Relaxed);
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
