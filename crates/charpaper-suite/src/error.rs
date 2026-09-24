use std::path::PathBuf;

use thiserror::Error;

/// Anything that stops a folder from being a usable suite at all. Problems a
/// suite can survive are [`crate::Finding`]s instead.
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

    #[error("schema version {found} is not supported; this build understands 1 to {supported}")]
    Schema { found: u32, supported: u32 },

    #[error("{0:?} must be relative to the suite folder and must not contain `..`")]
    UnsafePath(PathBuf),

    #[error("{0:?} does not exist in the suite folder")]
    Missing(PathBuf),

    #[error("no model: put one .glb/.gltf next to suite.toml or set `character.model`")]
    NoModel,

    #[error("{place} holds several candidates ({found}); name one in suite.toml")]
    Ambiguous { place: &'static str, found: String },

    #[error("{kind} {name:?} exists as both a .glb and a .gltf")]
    DuplicateName { kind: &'static str, name: String },

    #[error("{kind} {name:?} uses a reserved name; rename its file")]
    ReservedName { kind: &'static str, name: String },

    #[error("default {kind} {name:?} is not one of the suite's {kind}s")]
    UnknownDefault { kind: &'static str, name: String },
}
