//! Windows backend for the desktop wallpaper.
//!
//! Depends on `charpaper-wallpaper` and nothing else -- no Bevy, and no `windows`
//! crate either. This is the most stable code in the project, and isolating it
//! means editing the scene or upgrading the engine never rebuilds it.
//!
//! Everything is gated on `cfg(windows)` so `cargo check --workspace` on
//! another OS compiles this crate away to nothing instead of failing.

#[cfg(windows)]
mod activity;
#[cfg(windows)]
mod backend;
#[cfg(windows)]
mod desktop;
#[cfg(windows)]
mod hook;
#[cfg(windows)]
mod pointer;
#[cfg(windows)]
pub mod sys;

#[cfg(windows)]
pub use backend::WindowsBackend;
#[cfg(windows)]
pub use backend::inspect_report;
