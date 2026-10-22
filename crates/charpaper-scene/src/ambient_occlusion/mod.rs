// Bevy's SSAO darkens within a fixed 0.73 m of each pixel, which leaves no way to make it
// softer or sharper. This swaps in a copy of its shader that takes the radius from the one
// value Bevy passes it per camera, the object thickness, so ScreenSpaceAmbientOcclusion's
// constant_object_thickness holds the radius in meters instead. Bevy loads its shader by this
// embedded path when its pipelines are made, after every plugin is built, so the copy is what
// it finds. It relies on Bevy 0.19.1's shader and its bindings; compare them again on every
// Bevy upgrade.

use std::path::Path;
use std::path::PathBuf;

use bevy::asset::io::embedded::EmbeddedAssetRegistry;
use bevy::pbr::ScreenSpaceAmbientOcclusion;
use bevy::pbr::ScreenSpaceAmbientOcclusionQualityLevel;
use bevy::prelude::*;
use charpaper_settings::AmbientOcclusionQuality;
use charpaper_settings::Graphics;

const BEVY_SHADER: &str = "bevy_pbr/ssao/ssao.wgsl";

pub(crate) struct AmbientOcclusionPlugin;

impl Plugin for AmbientOcclusionPlugin {
    fn build(&self, app: &mut App) {
        let source = PathBuf::from(file!()).with_file_name("ssao.wgsl");
        app.world_mut().resource_mut::<EmbeddedAssetRegistry>().insert_asset(
            source,
            Path::new(BEVY_SHADER),
            include_bytes!("ssao.wgsl"),
        );
    }
}

pub(crate) fn ambient_occlusion(graphics: &Graphics) -> ScreenSpaceAmbientOcclusion {
    ScreenSpaceAmbientOcclusion {
        quality_level: match graphics.ambient_occlusion_quality {
            AmbientOcclusionQuality::Low => ScreenSpaceAmbientOcclusionQualityLevel::Low,
            AmbientOcclusionQuality::Medium => ScreenSpaceAmbientOcclusionQualityLevel::Medium,
            AmbientOcclusionQuality::High => ScreenSpaceAmbientOcclusionQualityLevel::High,
            AmbientOcclusionQuality::Ultra => ScreenSpaceAmbientOcclusionQualityLevel::Ultra,
        },
        constant_object_thickness: graphics.ambient_occlusion_softness.radius(),
    }
}
