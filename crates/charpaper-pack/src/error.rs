use std::path::PathBuf;

use thiserror::Error;

use crate::format::VERSION;

#[derive(Debug, Error)]
pub enum PackError {
    #[error("cannot read {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("cannot write {path}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("{0} is not a CharPaper suite pack")]
    NotAPack(PathBuf),

    #[error("{path} is pack format {found}, and this CharPaper reads format {VERSION}")]
    Version { path: PathBuf, found: u16 },

    #[error("{path} is damaged: {problem}; export the suite again")]
    Damaged { path: PathBuf, problem: String },

    #[error("{0} was replaced while it was open, so it has to be opened again")]
    Replaced(PathBuf),

    #[error("{pack} holds no file {file:?}")]
    Missing { pack: PathBuf, file: String },

    #[error("{0:?} can't be a path in a pack: {1}")]
    BadPath(String, &'static str),

    #[error("{0:?} and {1:?} differ only in capitals, which Windows treats as one name")]
    CaseClash(String, String),

    #[error("{0:?} is added to the pack twice")]
    Duplicate(String),

    #[error("packing was stopped")]
    Stopped,
}
