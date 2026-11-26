use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use charpaper_scene::ActiveSuite;
use charpaper_scene::AvailableSuites;
use charpaper_scene::CharacterClips;
use charpaper_scene::CharacterState;
use charpaper_scene::Expressions;
use charpaper_scene::Focused;
use charpaper_scene::Thumbnails;
use charpaper_scene::focused_changed;
use charpaper_settings::GazeSettings;
use charpaper_settings::PhysicsSettings;
use charpaper_settings::ViewSettings;
use i18n_embed::fluent::FluentLanguageLoader;
use i18n_embed_fl::fl;

use super::outfit;
use super::outfit::TilePlaceholder;
use super::outfit::picture_tile;
use super::outfit::show_picture;
use crate::held;
use crate::held::Loads;
use crate::icons::Icon;
use crate::panel::Hint;
use crate::panel::new_panel;
use crate::row::percent;
use crate::row::*;
use crate::section::Content;
use crate::section::Section;
use crate::section::section;
use crate::theme::*;

const GAZE_STRENGTH: Scale =
    Scale { step: 0.25, min: GazeSettings::WEAKEST, max: GazeSettings::STRONGEST, per_unit: 100.0 };
const PARALLAX_STRENGTH: Scale =
    Scale { step: 0.25, min: ViewSettings::WEAKEST, max: ViewSettings::STRONGEST, per_unit: 100.0 };

// What the focused stage shows, and from where: the suite and what the character wears and
// does, the camera and the environment. Clicks write to its CharacterState.
pub(crate) fn page() -> impl Bundle {
    children![suite(), outfit::section(), pose(), place(), gaze(), physics()]
}

// A tile per character, with a close-up of it, which the scene takes in the background. Made
// again whenever the characters folder changes.
fn suite() -> impl Bundle {
    section(
        |l| fl!(l, "section-suite"),
        Section::Suite,
        children![(
            Node {
                display: Display::None,
                grid_template_columns: vec![RepeatedGridTrack::flex(3, 1.0)],
                row_gap: Val::Px(8.0),
                column_gap: Val::Px(8.0),
                padding: UiRect::vertical(Val::Px(10.0)),
                ..default()
            },
            Pickable::IGNORE,
            CharacterGrid,
        )],
    )
}

pub(crate) struct CharacterPlugin;

impl Plugin for CharacterPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_click).add_systems(
            Update,
            (
                fill_characters.run_if(resource_changed::<AvailableSuites>.or_eager(new_panel)),
                style_characters.run_if(focused_changed::<CharacterState>.or_eager(new_panel)),
                show_close_ups.run_if(
                    resource_changed::<Thumbnails>
                        .or_eager(any_match_filter::<Added<CloseUpPicture>>),
                ),
            )
                .chain(),
        );
    }
}

#[derive(Component)]
#[require(Content)]
struct CharacterGrid;

// Shows the character in the folder of this name.
#[derive(Component)]
struct CharacterTile(String);

#[derive(Component)]
struct CloseUpPicture(String);

fn fill_characters(
    mut commands: Commands,
    available: Res<AvailableSuites>,
    grids: Query<(Entity, &mut Node), With<CharacterGrid>>,
) {
    for (grid, mut node) in grids {
        node.display = if available.0.is_empty() { Display::None } else { Display::Grid };
        commands.entity(grid).despawn_children().with_children(|grid| {
            for folder in &available.0 {
                let action = (
                    CharacterTile(folder.clone()),
                    Hint(|l| fl!(l, "help-suite")),
                    Hovered::default(),
                );
                let picture = CloseUpPicture(folder.clone());
                grid.spawn(picture_tile(folder, Icon::Character, action, picture));
            }
        });
    }
}

fn style_characters(
    state: Focused<CharacterState>,
    tiles: Query<(&CharacterTile, &mut Fill, &mut Stroke)>,
) {
    let Some(state) = state.get() else {
        return;
    };
    for (tile, mut fill, mut border) in tiles {
        let shown = state.suite.as_ref() == Some(&tile.0);
        fill.set_if_neq(Fill(if shown { SELECTED_BG } else { BUTTON_BG }));
        border.set_if_neq(Stroke(if shown { ACCENT } else { CLEAR }));
    }
}

fn show_close_ups(
    mut commands: Commands,
    thumbnails: Res<Thumbnails>,
    pictures: Query<(Entity, &CloseUpPicture, &Children, Has<ImageNode>)>,
    mut placeholders: Query<&mut Node, With<TilePlaceholder>>,
) {
    for (entity, picture, children, shown) in &pictures {
        if let Some(image) = thumbnails.character(&picture.0) {
            show_picture(&mut commands, (entity, children, shown), image, &mut placeholders);
        }
    }
}

// Put in place at once, as a click on the old picker's list was.
fn on_click(event: On<Pointer<Click>>, mut commands: Commands, tiles: Query<&CharacterTile>) {
    let Ok(CharacterTile(folder)) = tiles.get(event.entity) else {
        return;
    };
    let folder = folder.clone();
    commands.queue(move |world: &mut World| {
        let options = suites(world);
        if let Some(index) = options.iter().position(|o| o.as_deref() == Some(folder.as_str())) {
            let step = Step { change: Change::To(index), continuous: false };
            held::step(world, Loads::Suite, &options, &step);
        }
    });
}

fn pose() -> impl Bundle {
    section(
        |l| fl!(l, "section-pose"),
        Section::Pose,
        children![
            Setting {
                label: |l| fl!(l, "label-animation"),
                help: |l| fl!(l, "help-animation"),
                show: |w, _| {
                    let state = w.focused::<CharacterState>()?;
                    let suite = w.focused::<ActiveSuite>()?;
                    let options = animations(w);
                    let label = |clip: &Option<String>| match clip {
                        Some(clip) => labelled(&suite.clips, clip, |c| (&c.name, &c.label)),
                        None => name(clip),
                    };
                    Some(choice(&options, &state.animation, label).when(!options.is_empty()))
                },
                change: |w, step| {
                    let options = animations(w);
                    w.edit(|state: &mut CharacterState| pick(&options, &mut state.animation, step));
                },
            }
            .picker(),
            Setting {
                label: |l| fl!(l, "label-expression"),
                help: |l| fl!(l, "help-expression"),
                show: show_expression,
                change: change_expression,
            }
            .picker(),
        ],
    )
}

fn place() -> impl Bundle {
    section(
        |l| fl!(l, "section-place"),
        Section::Place,
        children![
            Setting {
                label: |l| fl!(l, "label-environment"),
                help: |l| fl!(l, "help-environment"),
                show: |w, _| {
                    let options = environments(w)?;
                    let current = held::current(w, Loads::Environment)?;
                    Some(choice(&options, &current, name).when(options.len() > 1))
                },
                change: |w, step| {
                    if let Some(options) = environments(w) {
                        held::step(w, Loads::Environment, &options, step);
                    }
                },
            }
            .picker(),
            Setting {
                label: |l| fl!(l, "label-camera"),
                help: |l| fl!(l, "help-camera"),
                // The orbit camera alone is no choice.
                show: |w, l| {
                    let options = cameras(w)?;
                    let current = held::current(w, Loads::Camera)?;
                    let orbit = fl!(l, "camera-orbit");
                    let label =
                        |camera: &Option<String>| camera.clone().unwrap_or_else(|| orbit.clone());
                    Some(choice(&options, &current, label).when(options.len() > 1))
                },
                change: |w, step| {
                    if let Some(options) = cameras(w) {
                        held::step(w, Loads::Camera, &options, step);
                    }
                },
            }
            .picker(),
            // Only while the stage looks through the orbit camera.
            Setting {
                label: |l| fl!(l, "label-lock-orbit"),
                help: |l| fl!(l, "help-lock-orbit"),
                show: |w, l| {
                    let orbit = w.focused::<CharacterState>()?.camera.is_none();
                    Some(switch(l, w.focused::<ViewSettings>()?.locked).when(orbit))
                },
                change: |w, step| w.edit(|view: &mut ViewSettings| flip(&mut view.locked, step)),
            }
            .switch(),
            Setting {
                label: |l| fl!(l, "label-parallax"),
                help: |l| fl!(l, "help-parallax"),
                show: |w, l| Some(switch(l, w.focused::<ViewSettings>()?.parallax)),
                change: |w, step| w.edit(|view: &mut ViewSettings| flip(&mut view.parallax, step)),
            }
            .switch(),
            Setting {
                label: |l| fl!(l, "label-parallax-strength"),
                help: |l| fl!(l, "help-parallax-strength"),
                show: |w, _| {
                    let view = w.focused::<ViewSettings>()?;
                    let strength = view.parallax_strength();
                    Some(PARALLAX_STRENGTH.number(percent(strength), strength).when(view.parallax))
                },
                change: |w, step| {
                    w.edit(|view: &mut ViewSettings| {
                        view.parallax_strength =
                            PARALLAX_STRENGTH.nudge(view.parallax_strength(), step);
                    });
                },
            }
            .stepper(),
        ],
    )
}

fn gaze() -> impl Bundle {
    section(
        |l| fl!(l, "section-gaze"),
        Section::Gaze,
        children![
            Setting {
                label: |l| fl!(l, "label-follow-cursor"),
                help: |l| fl!(l, "help-follow-cursor"),
                // Only for a suite whose suite.toml says which bones follow the cursor.
                show: |w, l| {
                    let gaze = w.focused::<GazeSettings>()?;
                    let has_gaze = w.focused::<ActiveSuite>().is_some_and(|s| s.gaze.is_some());
                    Some(switch(l, gaze.follow_cursor).when(has_gaze))
                },
                change: |w, step| {
                    w.edit(|gaze: &mut GazeSettings| flip(&mut gaze.follow_cursor, step));
                },
            }
            .switch(),
            // Does nothing while the character doesn't follow.
            Setting {
                label: |l| fl!(l, "label-gaze-strength"),
                help: |l| fl!(l, "help-gaze-strength"),
                show: |w, _| {
                    let gaze = w.focused::<GazeSettings>()?;
                    let has_gaze = w.focused::<ActiveSuite>().is_some_and(|s| s.gaze.is_some());
                    let strength = gaze.strength();
                    let shown = GAZE_STRENGTH.number(percent(strength), strength);
                    Some(shown.when(has_gaze && gaze.follow_cursor))
                },
                change: |w, step| {
                    w.edit(|gaze: &mut GazeSettings| {
                        gaze.strength = GAZE_STRENGTH.nudge(gaze.strength(), step);
                    });
                },
            }
            .stepper(),
            Setting {
                label: |l| fl!(l, "label-reactions"),
                help: |l| fl!(l, "help-reactions"),
                // Only for a suite with hotspots.
                show: |w, l| {
                    let gaze = w.focused::<GazeSettings>()?;
                    let has_hotspots =
                        w.focused::<ActiveSuite>().is_some_and(|s| !s.hotspots.is_empty());
                    Some(switch(l, gaze.reactions).when(has_hotspots))
                },
                change: |w, step| {
                    w.edit(|gaze: &mut GazeSettings| flip(&mut gaze.reactions, step));
                },
            }
            .switch()
        ],
    )
}

fn physics() -> impl Bundle {
    section(
        |l| fl!(l, "section-physics"),
        Section::Physics,
        children![
            // Only for a suite with chains.
            Setting {
                label: |l| fl!(l, "label-physics"),
                help: |l| fl!(l, "help-physics"),
                show: |w, l| {
                    let physics = w.focused::<PhysicsSettings>()?;
                    let swings =
                        w.focused::<ActiveSuite>().is_some_and(|s| !s.physics.chains.is_empty());
                    Some(switch(l, physics.enabled).when(swings))
                },
                change: |w, step| {
                    w.edit(|physics: &mut PhysicsSettings| flip(&mut physics.enabled, step));
                },
            }
            .switch(),
            // Only with colliders to make finer, and while the chains swing.
            Setting {
                label: |l| fl!(l, "label-physics-quality"),
                help: |l| fl!(l, "help-physics-quality"),
                show: |w, l| {
                    let physics = w.focused::<PhysicsSettings>()?;
                    let suite = w.focused::<ActiveSuite>();
                    let kept_off = suite.is_some_and(|s| {
                        !s.physics.chains.is_empty() && !s.physics.colliders.is_empty()
                    });
                    Some(switch(l, physics.high_quality).when(kept_off && physics.enabled))
                },
                change: |w, step| {
                    w.edit(|physics: &mut PhysicsSettings| flip(&mut physics.high_quality, step));
                },
            }
            .switch(),
            // Only where the environment shown has wind.
            Setting {
                label: |l| fl!(l, "label-wind"),
                help: |l| fl!(l, "help-wind"),
                show: |w, l| {
                    let physics = w.focused::<PhysicsSettings>()?;
                    let shown = w.focused::<CharacterState>()?.environment.clone();
                    let windy = w.focused::<ActiveSuite>().is_some_and(|s| {
                        !s.physics.chains.is_empty()
                            && s.environments.iter().any(|e| {
                                Some(&e.name) == shown.as_ref() && e.settings.wind.is_some()
                            })
                    });
                    Some(switch(l, physics.wind).when(windy && physics.enabled))
                },
                change: |w, step| {
                    w.edit(|physics: &mut PhysicsSettings| flip(&mut physics.wind, step));
                },
            }
            .switch(),
            Setting {
                label: |l| fl!(l, "label-physics-cursor"),
                help: |l| fl!(l, "help-physics-cursor"),
                show: |w, l| {
                    let physics = w.focused::<PhysicsSettings>()?;
                    let swings =
                        w.focused::<ActiveSuite>().is_some_and(|s| !s.physics.chains.is_empty());
                    Some(switch(l, physics.cursor).when(swings && physics.enabled))
                },
                change: |w, step| {
                    w.edit(|physics: &mut PhysicsSettings| flip(&mut physics.cursor, step));
                },
            }
            .switch()
        ],
    )
}

fn suites(world: &World) -> Vec<Option<String>> {
    world.resource::<AvailableSuites>().0.iter().cloned().map(Some).collect()
}

// Listed as soon as the suite is picked; a pick plays once its clip's file has loaded.
fn animations(world: &World) -> Vec<Option<String>> {
    let clips = world.focused::<CharacterClips>();
    clips
        .map(|clips| clips.offered().map(|name| Some(name.to_string())).collect())
        .unwrap_or_default()
}

// None is the orbit camera, which every suite has.
fn cameras(world: &World) -> Option<Vec<Option<String>>> {
    let suite = world.focused::<ActiveSuite>()?;
    Some(std::iter::once(None).chain(suite.cameras.iter().map(|c| Some(c.name.clone()))).collect())
}

fn environments(world: &World) -> Option<Vec<Option<String>>> {
    let suite = world.focused::<ActiveSuite>()?;
    Some(suite.environments.iter().map(|e| Some(e.name.clone())).collect())
}

// The name suite.toml gives a clip or an expression to show, else its file's.
fn labelled<T>(all: &[T], name: &str, parts: impl Fn(&T) -> (&String, &Option<String>)) -> String {
    let label = all.iter().map(&parts).find(|(own, _)| own.as_str() == name);
    label.and_then(|(_, label)| label.clone()).unwrap_or_else(|| name.to_string())
}

// Neutral is an empty name; see CharacterState::expressions. Only those keying the worn skin's
// shape keys are offered, so they follow every skin change.
fn expressions(world: &World) -> Option<Vec<String>> {
    let expressions = world.focused::<Expressions>()?;
    Some(std::iter::once(String::new()).chain(expressions.0.iter().cloned()).collect())
}

fn show_expression(world: &World, l: &FluentLanguageLoader) -> Option<Shown> {
    let state = world.focused::<CharacterState>()?;
    let options = expressions(world)?;
    let suite = world.focused::<ActiveSuite>()?;
    let current = state.expression().unwrap_or_default().to_string();
    let neutral = fl!(l, "expression-neutral");
    let label = |name: &String| match name.is_empty() {
        true => neutral.clone(),
        false => labelled(&suite.expressions, name, |e| (&e.name, &e.label)),
    };
    Some(choice(&options, &current, label).when(options.len() > 1))
}

// Kept per skin.
fn change_expression(world: &mut World, step: &Step) {
    let Some(options) = expressions(world) else {
        return;
    };
    let Some(mut state) = world.focused_mut::<CharacterState>() else {
        return;
    };
    let Some(skin) = state.skin.clone() else {
        return;
    };
    let current = state.expression().unwrap_or_default().to_string();
    if let Some(next) = stepped(&options, &current, step) {
        state.expressions.insert(skin, next);
    }
}
