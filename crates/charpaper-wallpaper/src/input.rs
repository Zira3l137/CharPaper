#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PointerEvent {
    Entered,
    Left,
    // Physical pixels from the top-left of our window.
    Moved {
        x: f64,
        y: f64,
    },
    Button {
        button: PointerButton,
        pressed: bool,
    },
    // Lines. Positive scrolls right and down, as in winit.
    Wheel {
        x: f32,
        y: f32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

// Only Send: the natural implementation wraps a channel receiver, which isn't Sync.
pub trait PointerSource: Send + 'static {
    fn drain(&mut self, out: &mut Vec<PointerEvent>);
}

// Given to the backend to call from its own input thread when a press or wheel turn comes in.
// Between frames the app sleeps, up to a second while rendering is paused, and only wakes
// early for input the OS sends to its window, which forwarded input never is. Must return
// quickly: it runs inside the backend's input path.
pub type Wake = Box<dyn Fn() + Send>;
