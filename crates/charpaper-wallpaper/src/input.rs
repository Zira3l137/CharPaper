//! Pointer input that the desktop keeps from our window.
//!
//! Once the window sits behind the icons, the OS delivers mouse input to the
//! icon layer instead of to us. A backend that causes this also provides a
//! [`PointerSource`] that recovers it, already translated into our window's
//! coordinate space, so the engine side can replay it without knowing how it
//! was captured.

/// One pointer event, as our window would have received it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PointerEvent {
    Entered,
    Left,
    /// Physical pixels, relative to the top-left corner of our window's client
    /// area.
    Moved {
        x: f64,
        y: f64,
    },
    Button {
        button: PointerButton,
        pressed: bool,
    },
    /// Lines, with winit's sign convention: positive values move the content
    /// right and down, which is what rolling a wheel away from you does.
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

/// A live stream of [`PointerEvent`]s.
///
/// Only `Send`: the natural implementations sit on a channel receiver, which
/// is not `Sync`. Every call takes `&mut self`, so the engine side can wrap it
/// in something that restores `Sync` without a lock.
pub trait PointerSource: Send + 'static {
    /// Append everything that arrived since the previous call, oldest first.
    fn drain(&mut self, out: &mut Vec<PointerEvent>);
}
