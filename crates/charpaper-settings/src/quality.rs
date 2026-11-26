use serde::Deserialize;
use serde::Serialize;

use crate::render::AmbientOcclusionQuality;
use crate::render::AntiAliasing;
use crate::render::DepthOfFieldQuality;
use crate::render::FogQuality;
use crate::render::FogResolution;
use crate::render::Graphics;
use crate::render::RenderSettings;
use crate::render::ShadowQuality;
use crate::render::ShadowUpdates;

// A preset sets, all at once, everything on a stage that trades looks for speed. Shadows are
// the exception that reaches further: Bevy draws every light's shadow maps at one size, so
// their quality is shared, and a preset picked on one stage changes them on every stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    Fastest,
    Moderate,
    #[default]
    Fancy,
    Best,
    // The viewer's own mix, from changing by hand anything a preset sets.
    Custom,
}

impl Preset {
    pub const ALL: [Preset; 5] =
        [Preset::Fastest, Preset::Moderate, Preset::Fancy, Preset::Best, Preset::Custom];

    // None for Custom, which is whatever the viewer made it.
    pub fn tuning(self) -> Option<Tuning> {
        match self {
            Preset::Fastest => Some(FASTEST),
            Preset::Moderate => Some(MODERATE),
            Preset::Fancy => Some(FANCY),
            Preset::Best => Some(BEST),
            Preset::Custom => None,
        }
    }

    // A save from before presets keeps its settings as they were, which no preset need match.
    pub(crate) fn before_presets() -> Self {
        Preset::Custom
    }
}

// Effects a preset can keep from being drawn. A preset never turns one on: strokes are the
// viewer's own choice and the rest come with the look, in amounts the author set. Allowing one
// only lets it show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Allowed {
    pub strokes: bool,
    pub bloom: bool,
    pub chromatic_aberration: bool,
    pub vignette: bool,
    pub grain: bool,
}

impl Allowed {
    pub const ALL: Allowed = Allowed {
        strokes: true,
        bloom: true,
        chromatic_aberration: true,
        vignette: true,
        grain: true,
    };
    pub const NONE: Allowed = Allowed {
        strokes: false,
        bloom: false,
        chromatic_aberration: false,
        vignette: false,
        grain: false,
    };
}

impl Default for Allowed {
    fn default() -> Self {
        Allowed::ALL
    }
}

// Everything a preset sets. The viewer's Custom mix is kept as one too.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tuning {
    pub render_scale: u32,
    pub anti_aliasing: AntiAliasing,
    pub depth_of_field: bool,
    pub depth_of_field_quality: DepthOfFieldQuality,
    pub fog: bool,
    pub fog_quality: FogQuality,
    pub fog_dithering: bool,
    pub fog_resolution: FogResolution,
    pub ambient_occlusion: bool,
    pub ambient_occlusion_quality: AmbientOcclusionQuality,
    pub allowed: Allowed,
    // In RenderSettings, shared by every stage.
    pub shadow_quality: ShadowQuality,
    pub shadow_updates: ShadowUpdates,
}

impl Default for Tuning {
    fn default() -> Self {
        FANCY
    }
}

impl Tuning {
    pub fn of(graphics: &Graphics, render: &RenderSettings) -> Self {
        Self {
            render_scale: graphics.render_scale,
            anti_aliasing: graphics.anti_aliasing,
            depth_of_field: graphics.depth_of_field.enabled,
            depth_of_field_quality: graphics.depth_of_field.quality,
            fog: graphics.fog,
            fog_quality: graphics.fog_quality,
            fog_dithering: graphics.fog_dithering,
            fog_resolution: graphics.fog_resolution,
            ambient_occlusion: graphics.ambient_occlusion,
            ambient_occlusion_quality: graphics.ambient_occlusion_quality,
            allowed: graphics.allowed,
            shadow_quality: render.shadow_quality,
            shadow_updates: render.shadow_updates,
        }
    }

    pub fn apply(&self, graphics: &mut Graphics, render: &mut RenderSettings) {
        self.apply_to_stage(graphics);
        render.shadow_quality = self.shadow_quality;
        render.shadow_updates = self.shadow_updates;
    }

    pub(crate) fn apply_to_stage(&self, graphics: &mut Graphics) {
        graphics.render_scale = self.render_scale;
        graphics.anti_aliasing = self.anti_aliasing;
        graphics.depth_of_field.enabled = self.depth_of_field;
        graphics.depth_of_field.quality = self.depth_of_field_quality;
        graphics.fog = self.fog;
        graphics.fog_quality = self.fog_quality;
        graphics.fog_dithering = self.fog_dithering;
        graphics.fog_resolution = self.fog_resolution;
        graphics.ambient_occlusion = self.ambient_occlusion;
        graphics.ambient_occlusion_quality = self.ambient_occlusion_quality;
        graphics.allowed = self.allowed;
    }
}

// Nothing costly, everything else at its cheapest. Very low fog is meant for jitter, so it
// keeps the jitter for whoever turns fog back on.
const FASTEST: Tuning = Tuning {
    render_scale: 50,
    anti_aliasing: AntiAliasing::Off,
    depth_of_field: false,
    depth_of_field_quality: DepthOfFieldQuality::Low,
    fog: false,
    fog_quality: FogQuality::VeryLow,
    fog_dithering: true,
    fog_resolution: FogResolution::Quarter,
    ambient_occlusion: false,
    ambient_occlusion_quality: AmbientOcclusionQuality::Low,
    allowed: Allowed::NONE,
    shadow_quality: ShadowQuality::Low,
    shadow_updates: ShadowUpdates::EveryOtherFrame,
};

// Still nothing costly, but what is cheap to make nicer is: fog at a sixteenth of the pixels
// with jitter costs little, and the film effects next to nothing.
const MODERATE: Tuning = Tuning {
    render_scale: 75,
    anti_aliasing: AntiAliasing::Msaa4,
    depth_of_field: false,
    depth_of_field_quality: DepthOfFieldQuality::Low,
    fog: true,
    fog_quality: FogQuality::VeryLow,
    fog_dithering: true,
    fog_resolution: FogResolution::Quarter,
    ambient_occlusion: false,
    ambient_occlusion_quality: AmbientOcclusionQuality::Low,
    allowed: Allowed { strokes: false, bloom: false, ..Allowed::ALL },
    shadow_quality: ShadowQuality::Medium,
    shadow_updates: ShadowUpdates::EveryOtherFrame,
};

// The look first, holding back only where the cost outgrows what it shows. 4x keeps edges
// crisp in motion at low frame rates, where TAA trails, though ambient occlusion can't run
// with it.
const FANCY: Tuning = Tuning {
    render_scale: 100,
    anti_aliasing: AntiAliasing::Msaa4,
    depth_of_field: true,
    depth_of_field_quality: DepthOfFieldQuality::Medium,
    fog: true,
    fog_quality: FogQuality::Medium,
    fog_dithering: false,
    fog_resolution: FogResolution::Half,
    ambient_occlusion: false,
    ambient_occlusion_quality: AmbientOcclusionQuality::High,
    allowed: Allowed::ALL,
    shadow_quality: ShadowQuality::Medium,
    shadow_updates: ShadowUpdates::EveryFrame,
};

// Everything at its most. TAA, the smoothest, is also what lets ambient occlusion run.
const BEST: Tuning = Tuning {
    render_scale: 100,
    anti_aliasing: AntiAliasing::Taa,
    depth_of_field: true,
    depth_of_field_quality: DepthOfFieldQuality::Ultra,
    fog: true,
    fog_quality: FogQuality::Ultra,
    fog_dithering: false,
    fog_resolution: FogResolution::Full,
    ambient_occlusion: true,
    ambient_occlusion_quality: AmbientOcclusionQuality::Ultra,
    allowed: Allowed::ALL,
    shadow_quality: ShadowQuality::High,
    shadow_updates: ShadowUpdates::EveryFrame,
};

impl Graphics {
    // Puts a preset's settings in place. Leaving Custom keeps the viewer's mix to come back to;
    // picking Custom brings it back, or with none kept, only names what is there now as theirs.
    pub fn pick(&mut self, preset: Preset, render: &mut RenderSettings) {
        let tuning = match preset.tuning() {
            Some(tuning) => {
                if self.preset == Preset::Custom {
                    self.custom = Some(Tuning::of(self, render));
                }
                Some(tuning)
            }
            None => self.custom.take(),
        };
        if let Some(tuning) = tuning {
            tuning.apply(self, render);
        }
        self.preset = preset;
    }

    // For after the viewer changed, by hand, something a preset sets: what is there now is
    // their own mix, in place of one kept before.
    pub fn tuned_by_hand(&mut self) {
        self.preset = Preset::Custom;
        self.custom = None;
    }

    pub fn draws_strokes(&self) -> bool {
        self.strokes && self.allowed.strokes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_settings_are_fancy() {
        let (graphics, render) = (Graphics::default(), RenderSettings::default());
        assert_eq!(graphics.preset, Preset::Fancy);
        assert_eq!(Tuning::of(&graphics, &render), FANCY);
    }

    #[test]
    fn a_save_from_before_presets_is_custom() {
        let graphics: Graphics = toml::from_str("render_scale = 75").unwrap();
        assert_eq!(graphics.preset, Preset::Custom);
        assert_eq!(graphics.render_scale, 75);
    }

    #[test]
    fn leaving_custom_keeps_the_mix() {
        let (mut graphics, mut render) = (Graphics::default(), RenderSettings::default());
        graphics.render_scale = 75;
        graphics.fog = false;
        graphics.tuned_by_hand();
        let mine = Tuning::of(&graphics, &render);

        graphics.pick(Preset::Best, &mut render);
        assert_eq!(Tuning::of(&graphics, &render), BEST);
        assert_eq!(render.shadow_quality, ShadowQuality::High);

        graphics.pick(Preset::Custom, &mut render);
        assert_eq!(graphics.preset, Preset::Custom);
        assert_eq!(Tuning::of(&graphics, &render), mine);
        assert_eq!(graphics.custom, None);
    }

    #[test]
    fn a_change_by_hand_replaces_the_kept_mix() {
        let (mut graphics, mut render) = (Graphics::default(), RenderSettings::default());
        graphics.pick(Preset::Custom, &mut render);
        graphics.pick(Preset::Fastest, &mut render);
        assert!(graphics.custom.is_some());

        graphics.render_scale = 100;
        graphics.tuned_by_hand();
        assert_eq!(graphics.custom, None);
        graphics.pick(Preset::Custom, &mut render);
        assert_eq!(graphics.render_scale, 100);
        assert_eq!(graphics.anti_aliasing, AntiAliasing::Off);
    }

    #[test]
    fn custom_without_a_kept_mix_changes_nothing() {
        let (mut graphics, mut render) = (Graphics::default(), RenderSettings::default());
        graphics.pick(Preset::Moderate, &mut render);
        let before = Tuning::of(&graphics, &render);
        graphics.pick(Preset::Custom, &mut render);
        assert_eq!(Tuning::of(&graphics, &render), before);
        assert_eq!(graphics.preset, Preset::Custom);
    }

    #[test]
    fn gated_strokes_are_not_drawn() {
        let (mut graphics, mut render) = (Graphics::default(), RenderSettings::default());
        graphics.strokes = true;
        graphics.pick(Preset::Fastest, &mut render);
        assert!(graphics.strokes);
        assert!(!graphics.draws_strokes());
    }

    #[test]
    fn the_custom_mix_survives_a_save() {
        let (mut graphics, mut render) = (Graphics::default(), RenderSettings::default());
        graphics.render_scale = 50;
        graphics.tuned_by_hand();
        graphics.pick(Preset::Best, &mut render);
        let text = toml::to_string(&graphics).unwrap();
        let loaded: Graphics = toml::from_str(&text).unwrap();
        assert_eq!(loaded, graphics);
    }
}
