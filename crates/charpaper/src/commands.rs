use std::path::Path;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use charpaper_bake::BakeSettings;
use charpaper_suite::Suite;

// Commands that run instead of the app. They never start Bevy: no window, no GPU, and
// output goes straight to stdout rather than through the log.

#[cfg(feature = "diagnostics")]
pub fn inspect() {
    for line in crate::wallpaper::inspect_report() {
        println!("{line}");
    }
}

pub fn bake_environments(path: &Path, dry_run: bool) -> Result<()> {
    let suite =
        Suite::load(path).with_context(|| format!("cannot load suite at {}", path.display()))?;
    let mut any = false;
    for environment in &suite.environments {
        let Some(panorama) = &environment.panorama else {
            continue;
        };
        any = true;
        let folder = suite.absolute(&environment.folder());
        let missing = charpaper_bake::missing(&folder);
        let maps: Vec<&str> = missing.iter().map(|m| m.file_name()).collect();
        let name = &environment.name;
        if missing.is_empty() {
            println!("{name:<16} up to date");
        } else if dry_run {
            println!("{name:<16} would bake {} from {}", maps.join(", "), panorama.display());
        } else {
            println!("{name:<16} baking {} from {}", maps.join(", "), panorama.display());
            let started = Instant::now();
            charpaper_bake::bake(&suite.absolute(panorama), &folder, &BakeSettings::default())
                .with_context(|| format!("cannot bake environment {name:?}"))?;
            println!("{name:<16} done in {:.1}s", started.elapsed().as_secs_f32());
        }
    }
    if !any {
        println!("no environment in {} has a panorama to bake from", path.display());
    }
    Ok(())
}
