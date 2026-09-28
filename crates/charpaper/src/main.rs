mod app;
mod cli;
mod commands;
mod config;
mod logging;
mod save;
mod wallpaper;

use anyhow::Result;

use crate::config::AppConfig;

fn main() -> Result<()> {
    let args = cli::parse();
    if args.inspect {
        commands::inspect();
        return Ok(());
    }
    if let Some(path) = &args.check_suite {
        return commands::check_suite(path);
    }

    let config = AppConfig::from_cli(&args);
    if let Some(path) = &args.bake_environments {
        return commands::bake_environments(path, &config.scene.bake, args.dry_run);
    }
    app::run(config)
}
