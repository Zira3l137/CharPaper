use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

use crate::layout::Look;
use crate::manifest::Tonemapping;

// What a viewer changed in a suite's look, kept apart from the author's suite.toml. Laid over
// the author's look, it gives the look shown. Only what the panel edits is here, and a setting
// left out keeps the author's value, so an updated suite.toml still shows through.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(default)]
pub struct LookEdits {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tonemapping: Option<Tonemapping>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lut: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub post: BTreeMap<PostSetting, f32>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub environments: BTreeMap<String, EnvironmentEdits>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum PostSetting {
    Exposure,
    Bloom,
    ChromaticAberration,
    Vignette,
    VignetteSize,
    Grain,
    GrainSize,
    Warmth,
    Tint,
    Saturation,
    Contrast,
    LutStrength,
}

impl PostSetting {
    const ALL: [PostSetting; 12] = [
        PostSetting::Exposure,
        PostSetting::Bloom,
        PostSetting::ChromaticAberration,
        PostSetting::Vignette,
        PostSetting::VignetteSize,
        PostSetting::Grain,
        PostSetting::GrainSize,
        PostSetting::Warmth,
        PostSetting::Tint,
        PostSetting::Saturation,
        PostSetting::Contrast,
        PostSetting::LutStrength,
    ];

    fn of(self, look: &mut Look) -> &mut Option<f32> {
        let post = &mut look.post;
        match self {
            PostSetting::Exposure => &mut post.exposure,
            PostSetting::Bloom => &mut post.bloom,
            PostSetting::ChromaticAberration => &mut post.chromatic_aberration,
            PostSetting::Vignette => &mut post.vignette.intensity,
            PostSetting::VignetteSize => &mut post.vignette.size,
            PostSetting::Grain => &mut post.grain.intensity,
            PostSetting::GrainSize => &mut post.grain.size,
            PostSetting::Warmth => &mut post.grading.warmth,
            PostSetting::Tint => &mut post.grading.tint,
            PostSetting::Saturation => &mut post.grading.saturation,
            PostSetting::Contrast => &mut post.grading.contrast,
            PostSetting::LutStrength => &mut post.lut.strength,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(default)]
pub struct EnvironmentEdits {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brightness: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadows: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exposure: Option<f32>,
}

impl LookEdits {
    // What `look` changes from `author`. A setting the viewer took back to the author's value
    // is no edit.
    pub fn between(author: &Look, look: &Look) -> Self {
        let mut edits = LookEdits::default();
        if look.post.tonemapping != author.post.tonemapping {
            edits.tonemapping = look.post.tonemapping;
        }
        if look.post.lut.name != author.post.lut.name {
            edits.lut = look.post.lut.name.clone();
        }
        let (mut author, mut look) = (author.clone(), look.clone());
        for setting in PostSetting::ALL {
            let (from, to) = (*setting.of(&mut author), *setting.of(&mut look));
            if let Some(to) = to.filter(|_| from != to) {
                edits.post.insert(setting, to);
            }
        }
        for (name, entry) in &look.environments {
            let base = author.environment(name);
            let environment = EnvironmentEdits {
                brightness: changed(base.brightness, entry.brightness),
                shadows: changed(base.shadows, entry.shadows),
                exposure: changed(base.exposure, entry.exposure),
            };
            if environment != EnvironmentEdits::default() {
                edits.environments.insert(name.clone(), environment);
            }
        }
        edits
    }

    pub fn apply(&self, look: &mut Look) {
        if let Some(tonemapping) = self.tonemapping {
            look.post.tonemapping = Some(tonemapping);
        }
        if let Some(lut) = &self.lut {
            look.post.lut.name = Some(lut.clone());
        }
        for (&setting, &value) in &self.post {
            *setting.of(look) = Some(value);
        }
        for (name, edits) in &self.environments {
            let entry = look.environments.entry(name.clone()).or_default();
            entry.brightness = edits.brightness.or(entry.brightness);
            entry.shadows = edits.shadows.or(entry.shadows);
            entry.exposure = edits.exposure.or(entry.exposure);
        }
    }

    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

fn changed<T: PartialEq>(from: Option<T>, to: Option<T>) -> Option<T> {
    if from != to { to } else { None }
}
