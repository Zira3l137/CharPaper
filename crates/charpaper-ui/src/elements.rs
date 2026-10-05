use bevy::prelude::*;

use crate::Tab;

// Marks the parts of the panel that systems show, hide or fill in.
#[derive(Component, Debug, Clone, PartialEq)]
pub(crate) enum UiContainer {
    MainMenu,
    Page(Tab),
    Section(Section),
    OutfitGrid,
    ObjectList,
    // The row a cycler sits in, so a single row can be hidden.
    Row(Cycler),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Section {
    Suite,
    Outfit,
    Animation,
    Expression,
    Gaze,
    Camera,
    Environment,
    Image,
    Color,
    Film,
    FrameRate,
    Quality,
    Fog,
    Pause,
    Interface,
}

// The values stepped through with `<` and `>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cycler {
    Suite,
    Advanced,
    Animation,
    Expression,
    FollowCursor,
    Camera,
    Environment,
    Brightness,
    Shadows,
    Tonemapping,
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
    Lut,
    LutStrength,
    FpsLimit,
    RenderScale,
    AntiAliasing,
    DepthOfField,
    Fog,
    FogQuality,
    FogDithering,
    PauseFullscreen,
    PauseCovered,
    PauseBattery,
    Language,
}

// What a button does when clicked.
#[derive(Component, Debug, Clone, PartialEq)]
pub(crate) enum UiButton {
    HideMenu,
    RevealMenu,
    Exit,
    Tab(Tab),
    Skin(String),
    Object(String),
    Previous(Cycler),
    Next(Cycler),
    RestoreLook,
}

impl UiButton {
    // The cycler a `<` or `>` button steps, and whether it steps forward.
    pub(crate) fn step(&self) -> Option<(Cycler, bool)> {
        match *self {
            UiButton::Previous(cycler) => Some((cycler, false)),
            UiButton::Next(cycler) => Some((cycler, true)),
            _ => None,
        }
    }
}

// The text showing a cycler's current value.
#[derive(Component)]
pub(crate) struct CyclerValue(pub Cycler);

// The line under the panel's title: the suite's name.
#[derive(Component)]
pub(crate) struct StatusText;
