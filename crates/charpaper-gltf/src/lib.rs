//! Writes the glTF files of a suite from what the Blender add-on reads out of Blender, in
//! place of Blender's own glTF exporter, which spends most of an export in Python.
//!
//! What it writes is what Blender's exporter would for the same scene, number for number:
//! [`blender`] holds the exporter's conversions, and the tests check them against files
//! Blender wrote.

pub mod blender;
mod build;
pub mod clip;

pub use clip::Clip;
pub use clip::ClipNode;
pub use clip::Trs;
