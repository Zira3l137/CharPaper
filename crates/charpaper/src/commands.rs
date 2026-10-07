use std::path::Path;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use charpaper_bake::BakeSettings;
use charpaper_suite::ORBIT_CAMERA;
use charpaper_suite::Suite;

use crate::wallpaper;

// Commands that run instead of the app. They never start Bevy: no window, no GPU, and
// output goes straight to stdout rather than through the log.

pub fn inspect() {
    for line in wallpaper::inspect_report() {
        println!("{line}");
    }
}

pub fn check_suite(path: &Path) -> Result<()> {
    let suite =
        Suite::load(path).with_context(|| format!("cannot load suite at {}", path.display()))?;

    let skins: Vec<&str> = suite.skins.iter().map(|s| s.name.as_str()).collect();
    println!("suite    {:?}", suite.name);
    println!("model    {}", suite.model.display());
    println!("clips    {} animation file(s)", suite.animations.len());
    println!("skins    [{}], default {:?}", skins.join(", "), suite.default_skin);
    let cameras: Vec<&str> = std::iter::once(ORBIT_CAMERA)
        .chain(suite.cameras.iter().map(|c| c.name.as_str()))
        .collect();
    let default_camera = suite.default_camera.as_deref().unwrap_or(ORBIT_CAMERA);
    println!("cameras  [{}], default {default_camera:?}", cameras.join(", "));
    let environments: Vec<&str> = suite.environments.iter().map(|e| e.name.as_str()).collect();
    println!("envs     [{}], default {:?}", environments.join(", "), suite.default_environment);
    for (i, cost) in charpaper_suite::costs(&suite).iter().enumerate() {
        for (j, line) in cost.to_string().lines().enumerate() {
            let label = if i + j == 0 { "cost" } else { "" };
            let indent = if j == 0 { "" } else { "  " };
            println!("{label:<9}{indent}{line}");
        }
    }

    let report = charpaper_suite::inspect(&suite);
    println!("{report}");
    if report.errors() > 0 {
        bail!("suite {:?} has {} error(s)", suite.name, report.errors());
    }
    Ok(())
}

pub fn bake_environments(path: &Path, settings: &BakeSettings, dry_run: bool) -> Result<()> {
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
            charpaper_bake::bake(&suite.absolute(panorama), &folder, settings)
                .with_context(|| format!("cannot bake environment {name:?}"))?;
            println!("{name:<16} done in {:.1}s", started.elapsed().as_secs_f32());
        }
    }
    if !any {
        println!("no environment in {} has a panorama to bake from", path.display());
    }
    Ok(())
}
