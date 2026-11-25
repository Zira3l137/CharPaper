use std::path::PathBuf;

use thiserror::Error;

use crate::manifest::Timeline;

#[derive(Debug, Error)]
pub enum SuiteError {
    #[error("cannot read {path}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("{path} is not a valid suite manifest")]
    Manifest {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error(
        "schema version {found} is not supported; this build reads version {supported} only, so \
         export the suite again with the Blender add-on"
    )]
    Schema { found: u32, supported: u32 },

    #[error(
        "timeline from frame {} to {} at {} fps can't play: the end must not come before the \
         start, and fps must be above 0", .0.start, .0.end, .0.fps
    )]
    Timeline(Timeline),

    #[error("{0:?} must be relative to the suite folder and must not contain `..`")]
    UnsafePath(PathBuf),

    #[error("{0:?} does not exist in the suite folder")]
    Missing(PathBuf),

    #[error(
        "no model and no skin: put one .glb/.gltf next to suite.toml, set `character.model`, \
         or add a skin to take the armature from"
    )]
    NoModel,

    #[error("{place} holds several candidates ({found}); name one in suite.toml")]
    Ambiguous { place: &'static str, found: String },

    #[error("{kind} {name:?} exists as both a .glb and a .gltf")]
    DuplicateName { kind: &'static str, name: String },

    #[error("camera {name:?} is named twice ({files}); camera names must be unique")]
    DuplicateCamera { name: String, files: String },

    #[error("{kind} {name:?} uses a reserved name; rename its file")]
    ReservedName { kind: &'static str, name: String },

    #[error("default {kind} {name:?} is not one of the suite's {kind}s")]
    UnknownDefault { kind: &'static str, name: String },

    #[error("{folder:?} holds several panoramas ({found}); keep one")]
    SeveralPanoramas { folder: PathBuf, found: String },

    #[error("settings are given for {kind} {name:?}, which the suite does not have")]
    UnknownEntry { kind: &'static str, name: String },

    #[error("hotspot {hotspot:?}: {problem}")]
    Hotspot { hotspot: String, problem: String },

    #[error("environment {environment:?}: its wind {problem}")]
    Wind { environment: String, problem: String },
    #[error("physics {name:?}: {problem}")]
    Physics { name: String, problem: String },
}
