use bevy::prelude::*;

use crate::Tab;

// Marks the parts of the panel that systems show, hide or fill in.
#[derive(Component, Debug, Clone, PartialEq)]
pub(crate) enum UiContainer {
    MainMenu,
    Page(Tab),
    Section(Section),
    // The part of a section that folds away under its title.
    Body(Section),
    OutfitGrid,
    ObjectList,
    // The row a cycler sits in, so a single row can be hidden.
    Row(Cycler),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Section {
    Suite,
    Outfit,
    // The worn skin's objects, switched on and off one by one. Inside Outfit.
    Parts,
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

impl Section {
    // Its name in state.toml.
    pub(crate) fn key(self) -> &'static str {
        match self {
            Section::Suite => "suite",
            Section::Outfit => "outfit",
            Section::Parts => "parts",
            Section::Animation => "animation",
            Section::Expression => "expression",
            Section::Gaze => "gaze",
            Section::Camera => "camera",
            Section::Environment => "environment",
            Section::Image => "image",
            Section::Color => "color",
            Section::Film => "film",
            Section::FrameRate => "frame_rate",
            Section::Quality => "quality",
            Section::Fog => "fog",
            Section::Pause => "pause",
            Section::Interface => "interface",
        }
    }

    // Fine-tuning starts folded, so a first look at the panel shows the main choices.
    pub(crate) fn folded_by_default(self) -> bool {
        matches!(
            self,
            Section::Parts
                | Section::Image
                | Section::Color
                | Section::Film
                | Section::Fog
                | Section::Pause
        )
    }
}

// The values stepped through with `<` and `>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cycler {
    Suite,
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
    Fold(Section),
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

// The arrow beside a section's title, pointing down while the section is open.
#[derive(Component)]
pub(crate) struct Chevron(pub Section);

// The line under the panel's title: the suite's name.
#[derive(Component)]
pub(crate) struct StatusText;
