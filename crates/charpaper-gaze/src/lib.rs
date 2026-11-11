//! Where a character looks while she follows the cursor, and how her neck, head and eyes share
//! the turn. Plain maths on vectors and rotations, without Bevy: the app reads the pose and
//! writes the bones, and the Blender add-on's preview is checked against the same numbers.
//!
//! A look takes three steps. `look_direction` turns the cursor's place in the picture into a
//! direction. `Setup` says how far towards it the head and neck point, measured from her
//! chest, and what that leaves to the eyes. `Frame` turns those angles into rotations, which
//! the functions for bones fit onto a chain of them.

mod bones;
mod frame;
mod look;
mod share;
mod spring;

pub use bones::carried;
pub use bones::turn_from_rest;
pub use bones::turned_by;
pub use bones::turned_from_rest;
pub use frame::Frame;
pub use frame::front_from_eyes;
pub use look::View;
pub use look::look_direction;
pub use share::Eyes;
pub use share::Setup;
pub use spring::Spring;
pub use spring::follow;
