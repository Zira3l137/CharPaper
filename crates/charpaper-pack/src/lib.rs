//! A suite's files in one `.chr` file. See `docs/chr-format.md` for the layout byte by byte.

mod error;
mod format;
mod path;
mod reader;
mod writer;

pub use error::PackError;
pub use format::ContentHash;
pub use format::Entry;
pub use format::Method;
pub use format::PackId;
pub use format::VERSION;
pub use path::check_path;
pub use path::pack_path;
pub use reader::Pack;
pub use reader::PackFile;
pub use writer::DEFAULT_LEVEL;
pub use writer::PackWriter;
pub use writer::Progress;
pub use writer::Summary;

/// The pack's name in a suite folder, beside suite.toml.
pub const PACK_FILE: &str = "suite.chr";
