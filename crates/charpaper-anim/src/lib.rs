//! Animation for every actor in a scene, without Bevy: clips read from the exporter's files,
//! stacked in slots and mixed into one pose per frame.
//!
//! A clip owns only what it keys. Slots are mixed from the bottom up: in each, the clips that
//! key a channel are averaged by their fade weights, then laid over what the slots below gave,
//! by as much as those weights add up to. So a crossfade inside a slot never dips, and a clip
//! fading in on top of the others covers them by its weight.

mod clip;
mod file;
mod mixer;
mod track;

pub use clip::Clip;
pub use clip::LightTracks;
pub use clip::NodeTracks;
pub use file::ClipError;
pub use file::ClipJson;
pub use file::FORMAT;
pub use file::JsonLight;
pub use mixer::Actor;
pub use mixer::ClipId;
pub use mixer::Event;
pub use mixer::Light;
pub use mixer::Mode;
pub use mixer::Pose;
pub use mixer::Rig;
pub use mixer::Transform;
pub use track::Interpolation;
pub use track::Sample;
pub use track::Track;
