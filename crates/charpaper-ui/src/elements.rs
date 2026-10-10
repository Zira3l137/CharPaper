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
    Sound,
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
            Section::Sound => "sound",
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

// A row's setting: a number stepped with `<` and `>`, or a choice from a list.
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
    MasterVolume,
    MusicVolume,
    AmbienceVolume,
    CharacterVolume,
    Language,
}

impl Cycler {
    // Numbers rather than a list of choices. The faster the wheel spins over these, the further
    // each notch goes; a list moves one choice per notch so none is skipped unseen.
    pub(crate) fn accelerates(self) -> bool {
        matches!(
            self,
            Cycler::Exposure
                | Cycler::Bloom
                | Cycler::ChromaticAberration
                | Cycler::Vignette
                | Cycler::VignetteSize
                | Cycler::Grain
                | Cycler::GrainSize
                | Cycler::Warmth
                | Cycler::Tint
                | Cycler::Saturation
                | Cycler::Contrast
                | Cycler::LutStrength
                | Cycler::MasterVolume
                | Cycler::MusicVolume
                | Cycler::AmbienceVolume
                | Cycler::CharacterVolume
        )
    }
}

// What a button does when clicked.
#[derive(Component, Debug, Clone, PartialEq)]
pub(crate) enum UiButton {
    HideMenu,
    RevealMenu,
    Exit,
    Tab(Tab),
    Fold(Section),
    // A choice picked by its place in the row's list.
    Choose(Cycler, usize),
    // Opens or closes the list of a row's choices.
    Open(Cycler),
    Skin(String),
    Object(String),
    Previous(Cycler),
    Next(Cycler),
    RestoreLook,
}

impl UiButton {
    // The step a press on `<` or `>` makes.
    pub(crate) fn step(&self) -> Option<Step> {
        let (cycler, by) = match *self {
            UiButton::Previous(cycler) => (cycler, -1),
            UiButton::Next(cycler) => (cycler, 1),
            _ => return None,
        };
        Some(Step { cycler, change: Change::By(by), continuous: false })
    }
}

// Steps through a row's values. Each tab watches for the rows it owns.
#[derive(Event, Debug, Clone, Copy)]
pub(crate) struct Step {
    pub cycler: Cycler,
    pub change: Change,
    // From the wheel or a held arrow rather than a single click.
    pub continuous: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Change {
    // Places to move, forward when positive.
    By(i32),
    // A choice picked straight from a list, by its place in it.
    To(usize),
}

impl Step {
    // A click goes round from the last value to the first; spinning the wheel stops at the end
    // instead of landing back where it started.
    pub(crate) fn wraps(&self) -> bool {
        !self.continuous
    }

    // How far a number moves. A number is never picked from a list, so a pick moves it nowhere.
    pub(crate) fn by(&self) -> i32 {
        match self.change {
            Change::By(by) => by,
            Change::To(_) => 0,
        }
    }
}

// A row's `<`, value and `>` together: the part the wheel changes rather than scrolls past.
#[derive(Component)]
pub(crate) struct Control(pub Cycler);

// The box around a cycler's value.
#[derive(Component)]
pub(crate) struct Well(pub Cycler);

// What a row shows. The tab that owns the row works it out, and the row's widgets draw it.
#[derive(Component, Debug, Clone, PartialEq, Default)]
pub(crate) enum Shown {
    #[default]
    Nothing,
    // A number as text, and where it sits in its range from 0 to 1. A centered range, one with
    // its neutral value in the middle, fills out from the middle.
    Number {
        text: String,
        fill: f32,
        centered: bool,
    },
    // A list of choices by their labels, and which one is current.
    Choice {
        labels: Vec<String>,
        current: Option<usize>,
    },
}

// The bar along the bottom of a number's box, as long as the number is far into its range.
#[derive(Component)]
pub(crate) struct FillBar(pub Cycler);

// Holds a row's segment buttons, one per choice.
#[derive(Component)]
pub(crate) struct SegmentBar(pub Cycler);

// The list that opens under a row, with a button per choice.
#[derive(Component)]
pub(crate) struct PickerList(pub Cycler);

// The text showing a row's current value: a number, or the name of the current choice.
#[derive(Component)]
pub(crate) struct CyclerValue(pub Cycler);

// The arrow beside a section's title, pointing down while the section is open.
#[derive(Component)]
pub(crate) struct Chevron(pub Section);

// The line at the bottom of the panel saying what the row under the pointer does.
#[derive(Component)]
pub(crate) struct HelpText;

// The line under the panel's title: the suite's name.
#[derive(Component)]
pub(crate) struct StatusText;
