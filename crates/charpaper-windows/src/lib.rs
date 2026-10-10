// Compiles to nothing on other systems, so the workspace still builds there.
#[cfg(windows)]
mod activity;
#[cfg(windows)]
mod backend;
#[cfg(windows)]
mod desktop;
#[cfg(windows)]
mod hook;
#[cfg(windows)]
mod monitors;
#[cfg(windows)]
mod pointer;
#[cfg(windows)]
pub mod sys;

#[cfg(windows)]
pub use backend::WindowsBackend;
#[cfg(windows)]
pub use backend::inspect_report;
