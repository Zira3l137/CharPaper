// Compiles to nothing on other systems, so the workspace still builds there.
#![cfg(windows)]

mod activity;
mod backend;
mod desktop;
mod hook;
mod monitors;
mod options;
mod pointer;
mod sys;

pub use backend::WindowsBackend;
pub use backend::inspect_report;
pub use options::AttachStrategy;
pub use options::Options;
