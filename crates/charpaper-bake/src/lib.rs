// Bakes an environment's skybox, specular and diffuse cubemaps from the equirectangular
// .hdr or .exr panorama its author lit the scene with in Blender. Bevy-free, like
// charpaper-suite, so it builds fast and runs from the CLI without the engine.

mod cube;
mod filter;
mod ktx2;
mod panorama;

use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;

use charpaper_suite::DIFFUSE_MAP;
use charpaper_suite::SKYBOX_MAP;
use charpaper_suite::SPECULAR_MAP;
use thiserror::Error;
use tracing::info;

use crate::panorama::Panorama;

// RGB9E5 takes 4 bytes a texel, so a skybox is 6 × size² × 4 bytes: 25 MB at 1024.
#[derive(Debug, Clone, PartialEq)]
pub struct BakeSettings {
    pub skybox_size: u32,
    pub specular_size: u32,
    pub specular_samples: u32,
    pub diffuse_size: u32,
}

impl Default for BakeSettings {
    fn default() -> Self {
        Self { skybox_size: 1024, specular_size: 256, specular_samples: 128, diffuse_size: 32 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Map {
    Skybox,
    Specular,
    Diffuse,
}

impl Map {
    pub const ALL: [Map; 3] = [Map::Skybox, Map::Specular, Map::Diffuse];

    pub fn file_name(self) -> &'static str {
        match self {
            Map::Skybox => SKYBOX_MAP,
            Map::Specular => SPECULAR_MAP,
            Map::Diffuse => DIFFUSE_MAP,
        }
    }
}

#[derive(Debug, Error)]
pub enum BakeError {
    #[error("cannot read the panorama {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: image::ImageError,
    },

    #[error(
        "the panorama {path} is {width}×{height}; an equirectangular one is twice as wide as tall"
    )]
    NotEquirectangular { path: PathBuf, width: u32, height: u32 },

    #[error("cannot write {path}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

// Existing maps are never touched. To bake one again, delete it.
pub fn missing(folder: &Path) -> Vec<Map> {
    Map::ALL.into_iter().filter(|map| !folder.join(map.file_name()).is_file()).collect()
}

pub fn bake(
    panorama: &Path,
    folder: &Path,
    settings: &BakeSettings,
) -> Result<Vec<PathBuf>, BakeError> {
    let maps = missing(folder);
    if maps.is_empty() {
        return Ok(Vec::new());
    }

    let started = Instant::now();
    let source = Panorama::load(panorama)?;
    let mut written = Vec::new();
    for map in maps {
        let path = folder.join(map.file_name());
        let levels = match map {
            Map::Skybox => vec![cube::render(settings.skybox_size, |dir| {
                source.sharp(dir, settings.skybox_size)
            })],
            Map::Specular => {
                filter::specular(&source, settings.specular_size, settings.specular_samples)
            }
            Map::Diffuse => filter::diffuse(&source, settings.diffuse_size),
        };
        ktx2::write(&path, &levels)
            .map_err(|source| BakeError::Write { path: path.clone(), source })?;
        written.push(path);
    }
    info!(
        "baked {} map(s) from {} in {:.1}s",
        written.len(),
        panorama.display(),
        started.elapsed().as_secs_f32()
    );
    Ok(written)
}
