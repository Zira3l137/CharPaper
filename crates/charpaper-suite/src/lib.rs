//! Character suites: a folder holding one character, its animations, its
//! skins and the scene around it.
//!
//! ```text
//! Aki/
//! ├── suite.toml
//! ├── aki.glb       the only .glb/.gltf here: the armature, nothing else
//! ├── animations/   every .glb/.gltf here is picked up
//! ├── skins/        one .glb/.gltf per skin, named after the file
//! └── environment/  optional scene and skybox
//! ```
//!
//! A skin is the whole visible character in one file, like a Blender
//! collection of meshes bound to the one armature. Switching skins hides every
//! mesh of the old one and shows every mesh of the new one, so what a skin
//! replaces is decided by what the artist put in it, never by configuration.
//!
//! [`manifest_schema`] describes `suite.toml` as a JSON Schema, which TOML
//! editors use for completion, hover docs and validation. The repository keeps
//! a copy in `schemas/suite.schema.json`; a test fails when it goes stale.
//!
//! [`Suite::load`] turns a folder into a resolved [`Suite`] without reading any
//! glTF. [`inspect`] then opens the glTF files and checks that the pieces fit
//! together.

mod error;
mod inspect;
mod layout;
mod manifest;

pub use error::SuiteError;
pub use inspect::Finding;
pub use inspect::Report;
pub use inspect::Severity;
pub use inspect::inspect;
pub use layout::AnimationFile;
pub use layout::ClipBinding;
pub use layout::ClipSet;
pub use layout::Environment;
pub use layout::Skin;
pub use layout::Suite;
pub use layout::discover;
pub use manifest::Ambient;
pub use manifest::Camera;
pub use manifest::Lighting;
pub use manifest::MANIFEST_FILE;
pub use manifest::Manifest;
pub use manifest::PlayMode;
pub use manifest::Post;
pub use manifest::SCHEMA_VERSION;
pub use manifest::Sun;
pub use manifest::Tonemapping;

/// `suite.toml` as a pretty-printed JSON Schema.
pub fn manifest_schema() -> String {
    let schema = schemars::schema_for!(Manifest);
    serde_json::to_string_pretty(&schema).expect("a schema always serializes") + "\n"
}
