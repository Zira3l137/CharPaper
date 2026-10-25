//! Blender's drivers on shape keys, as the exporter writes them next to a skin: read, checked
//! and evaluated, without Bevy.
//!
//! A driver sets a shape key from the pose: it reads variables off the bones, combines them,
//! by a formula for a scripted driver, maps the result through its curve, and clamps it to the
//! shape key's range. Only what the exporter can carry over is here: Blender's simple
//! expressions, and the variables that read bones (Rotational Difference, Transform Channel,
//! Distance) or other shape keys (Single Property).

mod curve;
mod driver;
mod expression;
mod file;
mod rotation;
mod transform;

pub use curve::Curve;
pub use driver::Driver;
pub use expression::Expression;
pub use file::BoneAxes;
pub use file::DriverEntry;
pub use file::DriversFile;
pub use file::Kind;
pub use file::RestCheck;
pub use file::Source;
pub use file::StandIn;
pub use file::Variable;
pub use rotation::rotation_difference;
pub use transform::Axes;
pub use transform::Channel;
pub use transform::RotationMode;
pub use transform::Space;
pub use transform::channel;
