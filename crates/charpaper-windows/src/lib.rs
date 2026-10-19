// Compiles to nothing on other systems, so the workspace still builds there.
#![cfg(windows)]

mod activity;
mod backend;
mod desktop;
mod hook;
mod monitors;
mod options;
mod pointer;
mod startup;
mod sys;

pub use backend::WindowsBackend;
pub use backend::inspect_report;
pub use options::AttachStrategy;
pub use options::Options;
pub use startup::set_starts_at_login;
pub use startup::starts_at_login;
