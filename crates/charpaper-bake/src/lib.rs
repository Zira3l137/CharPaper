//! Bakes an environment's reflection maps from the panorama its author
//! lit the scene with in Blender: an equirectangular `.hdr` or `.exr`.
//!
//! Three cubemaps come out, written next to the panorama as KTX2:
//! - `skybox.ktx2`: the panorama folded onto a cube, sharp.
//! - `specular.ktx2`: the same cube as a chain of blur levels, one per
//!   roughness, for shiny reflections.
//! - `diffuse.ktx2`: a tiny cube of the light arriving from each broad
//!   direction, for matte surfaces.
//!
//! Only missing maps are baked; existing ones are never touched. To bake a map
//! again, delete it.

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

/// How big and how carefully the maps are baked. The defaults balance a sharp
/// sky against file size: RGB9E5 takes 4 bytes a texel, so the skybox is
/// `6 × size² × 4` bytes, 25 MB at 1024.
#[derive(Debug, Clone, PartialEq)]
pub struct BakeSettings {
    /// Face size of the skybox, in pixels.
    pub skybox_size: u32,
    /// Face size of the specular map's sharpest level. Each level halves it,
    /// down to 1×1.
    pub specular_size: u32,
    /// Samples per texel for each blurred specular level. More is smoother and
    /// slower; each sample reads a pre-shrunk copy of the panorama sized to
    /// its spread, so even a few dozen show no speckle.
    pub specular_samples: u32,
    /// Face size of the diffuse map. It is nearly featureless by nature, so
    /// larger buys nothing.
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

/// The maps not yet in `folder`, in the order [`bake`] writes them.
pub fn missing(folder: &Path) -> Vec<Map> {
    Map::ALL.into_iter().filter(|map| !folder.join(map.file_name()).is_file()).collect()
}

/// Bakes whichever maps are missing from `folder` out of `panorama`, and
/// returns the files written: none if nothing was missing, in which case the
/// panorama is not even read.
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
