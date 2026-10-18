#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod app;
mod benchmark;
mod cli;
mod commands;
mod config;
mod logging;
mod pacing;
mod save;
mod wallpaper;

use anyhow::Result;
#[cfg(feature = "diagnostics")]
use charpaper_monitor::CountingAllocator;
use clap::Parser;
use mimalloc::MiMalloc;

use crate::cli::Cli;
use crate::config::AppConfig;

// Counted for the performance monitor's heap row.
#[cfg(feature = "diagnostics")]
#[global_allocator]
static ALLOCATOR: CountingAllocator<MiMalloc> = CountingAllocator(MiMalloc);

#[cfg(not(feature = "diagnostics"))]
#[global_allocator]
static ALLOCATOR: MiMalloc = MiMalloc;

fn main() -> Result<()> {
    let args = Cli::parse();
    if let Some(app) = args.keep_desktop {
        wallpaper::keep_desktop(app, args.orphaned);
        return Ok(());
    }
    #[cfg(feature = "diagnostics")]
    if args.inspect {
        commands::inspect();
        return Ok(());
    }
    if let Some(path) = &args.bake_environments {
        return commands::bake_environments(path, args.dry_run);
    }
    app::run(AppConfig::from_cli(&args))
}
