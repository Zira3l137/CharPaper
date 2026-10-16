// Each module owns some of the panel's rows: it builds their sections and keeps them up to
// date. The pages below lay those sections out into the tabs.

pub(crate) mod character;
pub(crate) mod graphics;
pub(crate) mod look;
pub(crate) mod sound;
pub(crate) mod system;

use bevy::prelude::*;

// What is shown and from where.
pub(crate) fn scene_page() -> impl Bundle {
    children![
        character::suite(),
        character::outfit(),
        character::animation(),
        character::expression(),
        character::gaze(),
        look::camera(),
        look::environment(),
    ]
}

// How finely it is drawn, and how the picture is finished.
pub(crate) fn graphics_page() -> impl Bundle {
    children![graphics::quality(), graphics::fog(), look::post_processing()]
}
