use bevy::prelude::*;
use charpaper_scene::ActiveSuite;
use charpaper_scene::AvailableSuites;
use charpaper_scene::CharacterClips;
use charpaper_scene::CharacterState;
use charpaper_scene::Expressions;
use charpaper_settings::GazeSettings;
use charpaper_settings::PhysicsSettings;
use charpaper_settings::ViewSettings;
use i18n_embed::fluent::FluentLanguageLoader;
use i18n_embed_fl::fl;

use super::outfit;
use crate::held;
use crate::held::Loads;
use crate::row::percent;
use crate::row::*;
use crate::section::Section;
use crate::section::section;

const GAZE_STRENGTH: Scale =
    Scale { step: 0.25, min: GazeSettings::WEAKEST, max: GazeSettings::STRONGEST, per_unit: 100.0 };
const PARALLAX_STRENGTH: Scale =
    Scale { step: 0.25, min: ViewSettings::WEAKEST, max: ViewSettings::STRONGEST, per_unit: 100.0 };

// What the focused stage shows, and from where: the suite and what the character wears and
// does, the camera and the environment. Clicks write to its CharacterState.
pub(crate) fn page() -> impl Bundle {
    children![suite(), outfit::section(), pose(), place(), gaze(), physics()]
}

fn suite() -> impl Bundle {
    section(
        |l| fl!(l, "section-suite"),
        Section::Suite,
        children![
            Setting {
                label: |l| fl!(l, "label-suite"),
                help: |l| fl!(l, "help-suite"),
                show: |w, _| {
                    let options = suites(w);
                    let current = held::current(w, Loads::Suite)?;
                    Some(choice(&options, &current, name).when(options.len() > 1))
                },
                change: |w, step| {
                    let options = suites(w);
                    held::step(w, Loads::Suite, &options, step);
                },
            }
            .picker()
        ],
    )
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
