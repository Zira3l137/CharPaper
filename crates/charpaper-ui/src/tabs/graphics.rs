use bevy::prelude::*;
use charpaper_scene::ActiveLook;
use charpaper_scene::ActiveSuite;
use charpaper_scene::CharacterState;
use charpaper_scene::Focused;
use charpaper_scene::LookEdited;
use charpaper_scene::RestoreLook;
use charpaper_scene::focused_changed;
use charpaper_settings::Allowed;
use charpaper_settings::AmbientOcclusionQuality;
use charpaper_settings::AmbientOcclusionSoftness;
use charpaper_settings::AntiAliasing;
use charpaper_settings::DepthOfFieldQuality;
use charpaper_settings::DepthOfFieldShape;
use charpaper_settings::FogQuality;
use charpaper_settings::FogResolution;
use charpaper_settings::Graphics;
use charpaper_settings::Preset;
use charpaper_settings::RENDER_SCALES;
use charpaper_settings::RenderSettings;
use charpaper_settings::ShadowQuality;
use charpaper_settings::ShadowUpdates;
use charpaper_settings::StrokeStrength;
use charpaper_settings::StrokeWidth;
use charpaper_settings::Tuning;
use charpaper_suite::BackgroundBlur;
use charpaper_suite::NO_LUT;
use charpaper_suite::Tonemapping;
use i18n_embed::fluent::FluentLanguageLoader;
use i18n_embed_fl::fl;

use super::look::edit_look;
use super::look::resolved;
use crate::panel::new_panel;
use crate::row::percent;
use crate::row::*;
use crate::section::Section;
use crate::section::section;
use crate::section::subsection;
use crate::tabs::look::shown_environment;
use crate::widgets::*;

const EXPOSURE: Scale = Scale { step: 0.5, min: -10.0, max: 10.0, per_unit: 10.0 };
const BLOOM: Scale = Scale { step: 0.05, min: 0.0, max: 1.0, per_unit: 100.0 };
// Brightness before tonemapping, 1.0 being white.
const BLOOM_THRESHOLD: Scale = Scale { step: 0.1, min: 0.0, max: 5.0, per_unit: 10.0 };
const ABERRATION: Scale = Scale { step: 0.0025, min: 0.0, max: 0.05, per_unit: 10_000.0 };
const VIGNETTE: Scale = Scale { step: 0.05, min: 0.0, max: 1.0, per_unit: 100.0 };
const VIGNETTE_SIZE: Scale = Scale { step: 0.1, min: 0.3, max: 2.0, per_unit: 10.0 };
const GRAIN: Scale = Scale { step: 0.01, min: 0.0, max: 0.2, per_unit: 100.0 };
const GRAIN_SIZE: Scale = Scale { step: 0.25, min: 1.0, max: 4.0, per_unit: 100.0 };
// Warmth and tint.
const SHIFT: Scale = Scale { step: 0.05, min: -1.0, max: 1.0, per_unit: 100.0 };
// 100% leaves the picture as it is, which sits in the middle of both ranges.
const SATURATION: Scale = Scale { step: 0.05, min: 0.0, max: 2.0, per_unit: 100.0 };
const CONTRAST: Scale = Scale { step: 0.05, min: 0.5, max: 1.5, per_unit: 100.0 };
// Degrees, wrapping around at half a turn.
const SKY_TURN_STEP: f32 = 15.0;
// A share of the sky's brightness.
const BACKGROUND: Scale = Scale { step: 0.1, min: 0.0, max: 2.0, per_unit: 100.0 };
const BACKGROUND_BLUR: Scale = Scale { step: 0.05, min: 0.0, max: 1.0, per_unit: 100.0 };
const LUT_STRENGTH: Scale = Scale { step: 0.05, min: 0.0, max: 1.0, per_unit: 100.0 };
// Depth of field strength, a stop of aperture apart: each doubles or halves the blur's area.
const BLUR_STRENGTH_STOPS: [f32; 9] = [0.25, 0.35, 0.5, 0.7, 1.0, 1.4, 2.0, 2.8, 4.0];
const FOG_DENSITY_STOPS: [f32; 9] = BLUR_STRENGTH_STOPS;
// Diameters in pixels. Beyond 128 the near blur would need more tiles around each pixel.
const MAX_BLUR_STOPS: [f32; 7] = [16.0, 24.0, 32.0, 48.0, 64.0, 96.0, 128.0];
const HIGHLIGHTS: Scale = Scale { step: 0.25, min: 0.0, max: 3.0, per_unit: 100.0 };
// As Blender's Background Strength, spaced so each step looks about as big as the last.
const SKY_STRENGTH_STOPS: [f32; 15] =
    [0.05, 0.1, 0.2, 0.3, 0.5, 0.7, 1.0, 1.5, 2.0, 3.0, 5.0, 8.0, 12.0, 20.0, 30.0];

// The button under the post-processing rows that drops the viewer's look edits.
pub(crate) struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_click).add_systems(
            Update,
            show_restore.run_if(focused_changed::<LookEdited>.or_eager(new_panel)),
        );
    }
}

#[derive(Component)]
struct RestoreButton;

// How finely the focused stage is drawn, which the app saves with the stage in its Graphics,
// and how the picture is finished, in its look. Shadow quality is the exception: every stage
// shares it. A preset sets all that trades looks for speed at once; changing any of it by hand
// makes the mix the viewer's own, Custom.
pub(crate) fn page() -> impl Bundle {
    children![quality(), lighting(), fog(), depth_of_field(), strokes(), post_processing()]
}

fn lighting() -> impl Bundle {
    section(
        |l| fl!(l, "section-lighting"),
        Section::Lighting,
        children![
            Setting {
                label: |l| fl!(l, "label-sky-strength"),
                help: |l| fl!(l, "help-sky-strength"),
                show: |w, _| {
                    let lit = shown_environment(w)
                        .is_some_and(|e| e.sky().is_some() || e.reflections().is_some());
                    let strength = resolved(w)?.sky_strength;
                    let [min, .., max] = SKY_STRENGTH_STOPS;
                    let text = format!("{strength:.2}");
                    Some(number(text, strength.max(min).ln(), min.ln(), max.ln()).when(lit))
                },
                change: |w, step| {
                    edit_look(w, |look, current, environment| {
                        if let Some(name) = environment {
                            let strength =
                                step_stops(&SKY_STRENGTH_STOPS, current.sky_strength, step.by());
                            look.environments.entry(name.into()).or_default().sky_strength =
                                Some(strength);
                        }
                    });
                },
            }
            .stepper(),
            Setting {
                label: |l| fl!(l, "label-sky-rotation"),
                help: |l| fl!(l, "help-sky-rotation"),
                show: |w, _| {
                    let lit = shown_environment(w)
                        .is_some_and(|e| e.sky().is_some() || e.reflections().is_some());
                    let turn = resolved(w)?.sky_rotation_deg;
                    Some(balance(format!("{turn:.0}°"), turn, -180.0, 180.0).when(lit))
                },
                change: |w, step| {
                    edit_look(w, |look, current, environment| {
                        if let Some(name) = environment {
                            let turned =
                                current.sky_rotation_deg + SKY_TURN_STEP * step.by() as f32;
                            // Into (-180, 180], so 180 and -180 aren't two stops.
                            let turn = 180.0 - (180.0 - turned).rem_euclid(360.0);
                            look.environments.entry(name.into()).or_default().sky_rotation_deg =
                                Some(turn);
                        }
                    });
                },
            }
            .stepper(),
            Setting {
                label: |l| fl!(l, "label-sun-follows-sky"),
                help: |l| fl!(l, "help-sun-follows-sky"),
                show: |w, l| {
                    let both = shown_environment(w)
                        .is_some_and(|e| e.scene.is_some() && e.sky().is_some());
                    Some(switch(l, resolved(w)?.sun_follows_sky).when(both))
                },
                change: |w, step| {
                    edit_look(w, |look, current, environment| {
                        if let Some(name) = environment {
                            let mut follows = current.sun_follows_sky;
                            flip(&mut follows, step);
                            look.environments.entry(name.into()).or_default().sun_follows_sky =
                                Some(follows);
                        }
                    });
                },
            }
            .switch(),
            Setting {
                label: |l| fl!(l, "label-background"),
                help: |l| fl!(l, "help-background"),
                show: |w, l| {
                    let sky = shown_environment(w).is_some_and(|e| e.sky().is_some());
                    let background = resolved(w)?.background;
                    let text =
                        if background <= 0.0 { on_off(l, false) } else { percent(background) };
                    Some(BACKGROUND.number(text, background).when(sky))
                },
                change: |w, step| {
                    edit_look(w, |look, current, environment| {
                        if let Some(name) = environment {
                            look.environments.entry(name.into()).or_default().background =
                                Some(BACKGROUND.nudge(current.background, step));
                        }
                    });
                },
            }
            .stepper(),
            Setting {
                label: |l| fl!(l, "label-background-blur"),
                help: |l| fl!(l, "help-background-blur"),
                show: |w, l| {
                    // Blurred from the reflection map.
                    let blurrable = shown_environment(w).is_some_and(|e| e.reflections().is_some());
                    let resolved = resolved(w)?;
                    let blur = resolved.background_blur;
                    let text = if blur <= 0.0 { on_off(l, false) } else { percent(blur) };
                    let shown = blurrable && resolved.background > 0.0;
                    Some(BACKGROUND_BLUR.number(text, blur).when(shown))
                },
                change: |w, step| {
                    edit_look(w, |look, current, environment| {
                        if let Some(name) = environment {
                            let blur = BACKGROUND_BLUR.nudge(current.background_blur, step);
                            look.environments.entry(name.into()).or_default().background_blur =
                                Some(BackgroundBlur(blur));
                        }
                    });
                },
            }
            .stepper(),
            Setting {
                label: |l| fl!(l, "label-shadows"),
                help: |l| fl!(l, "help-shadows"),
                show: |w, l| {
                    let scene = shown_environment(w).is_some_and(|e| e.scene.is_some());
                    Some(switch(l, resolved(w)?.shadows).when(scene))
                },
                change: |w, step| {
                    edit_look(w, |look, current, environment| {
                        if let Some(name) = environment {
                            let mut shadows = current.shadows;
                            flip(&mut shadows, step);
                            look.environments.entry(name.into()).or_default().shadows =
                                Some(shadows);
                        }
                    });
                },
            }
            .switch(),
            Setting {
                label: |l| fl!(l, "label-shadow-quality"),
                help: |l| fl!(l, "help-shadow-quality"),
                show: |w, l| {
                    let current = w.resource::<RenderSettings>().shadow_quality;
                    Some(choice(&ShadowQuality::ALL, &current, |quality| match quality {
                        ShadowQuality::Low => fl!(l, "value-low"),
                        ShadowQuality::Medium => fl!(l, "value-medium"),
                        ShadowQuality::High => fl!(l, "value-high"),
                    }))
                },
                change: |w, step| {
                    tune(w, |_, render| {
                        pick(&ShadowQuality::ALL, &mut render.shadow_quality, step)
                    });
                },
            }
            .segments(),
            Setting {
                label: |l| fl!(l, "label-shadow-updates"),
                help: |l| fl!(l, "help-shadow-updates"),
                show: |w, l| {
                    let current = w.resource::<RenderSettings>().shadow_updates;
                    Some(choice(&ShadowUpdates::ALL, &current, |updates| match updates {
                        ShadowUpdates::EveryFrame => fl!(l, "value-every-frame"),
                        ShadowUpdates::EveryOtherFrame => fl!(l, "value-every-other-frame"),
                    }))
                },
                change: |w, step| {
                    tune(w, |_, render| {
                        pick(&ShadowUpdates::ALL, &mut render.shadow_updates, step)
                    });
                },
            }
            .segments(),
            Setting {
                label: |l| fl!(l, "label-ambient-occlusion"),
                help: |l| fl!(l, "help-ambient-occlusion"),
                show: |w, l| {
                    let graphics = w.focused::<Graphics>()?;
                    Some(switch(l, graphics.ambient_occlusion).when(occludable(w, graphics)))
                },
                change: |w, step| tune(w, |g, _| flip(&mut g.ambient_occlusion, step)),
            }
            .switch(),
            Setting {
                label: |l| fl!(l, "label-ambient-occlusion-quality"),
                help: |l| fl!(l, "help-ambient-occlusion-quality"),
                show: |w, l| {
                    let graphics = w.focused::<Graphics>()?;
                    let current = graphics.ambient_occlusion_quality;
                    let shown = graphics.ambient_occlusion && occludable(w, graphics);
                    let label = |quality: &AmbientOcclusionQuality| match quality {
                        AmbientOcclusionQuality::Low => fl!(l, "value-low"),
                        AmbientOcclusionQuality::Medium => fl!(l, "value-medium"),
                        AmbientOcclusionQuality::High => fl!(l, "value-high"),
                        AmbientOcclusionQuality::Ultra => fl!(l, "value-ultra"),
                    };
                    Some(choice(&AmbientOcclusionQuality::ALL, &current, label).when(shown))
                },
                change: |w, step| {
                    tune(w, |g, _| {
                        pick(&AmbientOcclusionQuality::ALL, &mut g.ambient_occlusion_quality, step);
                    });
                },
            }
            .segments(),
            Setting {
                label: |l| fl!(l, "label-ambient-occlusion-softness"),
                help: |l| fl!(l, "help-ambient-occlusion-softness"),
                show: |w, l| {
                    let graphics = w.focused::<Graphics>()?;
                    let current = graphics.ambient_occlusion_softness;
                    let shown = graphics.ambient_occlusion && occludable(w, graphics);
                    let label = |softness: &AmbientOcclusionSoftness| match softness {
                        AmbientOcclusionSoftness::Sharp => fl!(l, "value-distance-short"),
                        AmbientOcclusionSoftness::Medium => fl!(l, "value-distance-medium"),
                        AmbientOcclusionSoftness::Soft => fl!(l, "value-distance-long"),
                    };
                    Some(choice(&AmbientOcclusionSoftness::ALL, &current, label).when(shown))
                },
                change: |w, step| {
                    w.edit(|g: &mut Graphics| {
                        let softness = &mut g.ambient_occlusion_softness;
                        pick(&AmbientOcclusionSoftness::ALL, softness, step);
                    });
                },
            }
            .segments(),
        ],
    )
}

// Ambient occlusion only darkens the sky's light, and there is none without its maps. Bevy's
// can't run with MSAA.
fn occludable(world: &World, graphics: &Graphics) -> bool {
    let sky_lit =
        shown_environment(world).is_some_and(|e| e.sky().is_some() || e.reflections().is_some());
    sky_lit && graphics.anti_aliasing != AntiAliasing::Msaa4
}

// Fog only shows in an environment with fog boxes.
fn foggy(world: &World) -> bool {
    shown_environment(world).is_some_and(|e| !e.settings.fog.is_empty())
}

// The rows that tune the blur, while there is one.
fn blurs(world: &World) -> bool {
    has_lens(world) && world.focused::<Graphics>().is_some_and(|g| g.depth_of_field.enabled)
}

// Depth of field only for a camera whose lens asks for it, as the scene reads it.
fn has_lens(world: &World) -> bool {
    let (Some(state), Some(suite)) =
        (world.focused::<CharacterState>(), world.focused::<ActiveSuite>())
    else {
        return false;
    };
    match state.camera.as_deref() {
        None => suite.camera.f_stop.is_some(),
        Some(name) => suite.cameras.iter().any(|c| c.name == name && c.settings.f_stop.is_some()),
    }
}

fn quality() -> impl Bundle {
    section(
        |l| fl!(l, "section-quality"),
        Section::Quality,
        children![
            Setting {
                label: |l| fl!(l, "label-preset"),
                help: |l| fl!(l, "help-preset"),
                show: |w, l| {
                    let current = w.focused::<Graphics>()?.preset;
                    Some(choice(&Preset::ALL, &current, |preset| match preset {
                        Preset::Fastest => fl!(l, "preset-fastest"),
                        Preset::Moderate => fl!(l, "preset-moderate"),
                        Preset::Fancy => fl!(l, "preset-fancy"),
                        Preset::Best => fl!(l, "preset-best"),
                        Preset::Custom => fl!(l, "preset-custom"),
                    }))
                },
                change: |w, step| {
                    let Some(current) = w.focused::<Graphics>().map(|g| g.preset) else {
                        return;
                    };
                    if let Some(preset) = stepped(&Preset::ALL, &current, step) {
                        edit_graphics(w, |graphics, render| graphics.pick(preset, render));
                    }
                },
            }
            .picker(),
            Setting {
                label: |l| fl!(l, "label-render-scale"),
                help: |l| fl!(l, "help-render-scale"),
                show: |w, _| {
                    let scale = w.focused::<Graphics>()?.scale_percent();
                    Some(choice(&RENDER_SCALES, &scale, |scale| format!("{scale}%")))
                },
                change: |w, step| {
                    tune(w, |graphics, _| {
                        let mut scale = graphics.scale_percent();
                        pick(&RENDER_SCALES, &mut scale, step);
                        graphics.render_scale = scale;
                    });
                },
            }
            .segments(),
            Setting {
                label: |l| fl!(l, "label-anti-aliasing"),
                help: |l| fl!(l, "help-anti-aliasing"),
                show: |w, l| {
                    let current = w.focused::<Graphics>()?.anti_aliasing;
                    Some(choice(&AntiAliasing::ALL, &current, |aa| match aa {
                        AntiAliasing::Off => on_off(l, false),
                        AntiAliasing::Msaa4 => "4x".into(),
                        AntiAliasing::Taa => "TAA".into(),
                    }))
                },
                change: |w, step| {
                    tune(w, |g, _| pick(&AntiAliasing::ALL, &mut g.anti_aliasing, step));
                },
            }
            .segments(),
        ],
    )
}

fn depth_of_field() -> impl Bundle {
    section(
        |l| fl!(l, "section-depth-of-field"),
        Section::DepthOfField,
        children![
            Setting {
                label: |l| fl!(l, "label-depth-of-field"),
                help: |l| fl!(l, "help-depth-of-field"),
                show: |w, l| {
                    let dof = w.focused::<Graphics>()?.depth_of_field;
                    Some(switch(l, dof.enabled).when(has_lens(w)))
                },
                change: |w, step| {
                    tune(w, |g, _| flip(&mut g.depth_of_field.enabled, step));
                },
            }
            .switch(),
            Setting {
                label: |l| fl!(l, "label-blur-shape"),
                help: |l| fl!(l, "help-blur-shape"),
                show: |w, l| {
                    let dof = w.focused::<Graphics>()?.depth_of_field;
                    let label = |shape: &DepthOfFieldShape| match shape {
                        DepthOfFieldShape::Blur => fl!(l, "value-blur"),
                        DepthOfFieldShape::Bokeh => fl!(l, "value-bokeh"),
                    };
                    Some(choice(&DepthOfFieldShape::ALL, &dof.shape, label).when(blurs(w)))
                },
                change: |w, step| {
                    w.edit(|g: &mut Graphics| {
                        pick(&DepthOfFieldShape::ALL, &mut g.depth_of_field.shape, step);
                    });
                },
            }
            .segments(),
            Setting {
                label: |l| fl!(l, "label-blur-strength"),
                help: |l| fl!(l, "help-blur-strength"),
                show: |w, _| {
                    let dof = w.focused::<Graphics>()?.depth_of_field;
                    let [min, .., max] = BLUR_STRENGTH_STOPS;
                    let shown =
                        number(percent(dof.strength), dof.strength.ln(), min.ln(), max.ln());
                    Some(shown.when(blurs(w)))
                },
                change: |w, step| {
                    w.edit(|g: &mut Graphics| {
                        let dof = &mut g.depth_of_field;
                        dof.strength = step_stops(&BLUR_STRENGTH_STOPS, dof.strength, step.by());
                    });
                },
            }
            .stepper(),
            Setting {
                label: |l| fl!(l, "label-max-blur"),
                help: |l| fl!(l, "help-max-blur"),
                show: |w, _| {
                    let dof = w.focused::<Graphics>()?.depth_of_field;
                    let [min, .., max] = MAX_BLUR_STOPS;
                    let text = format!("{:.0} px", dof.max_blur);
                    Some(number(text, dof.max_blur.ln(), min.ln(), max.ln()).when(blurs(w)))
                },
                change: |w, step| {
                    w.edit(|g: &mut Graphics| {
                        let dof = &mut g.depth_of_field;
                        dof.max_blur = step_stops(&MAX_BLUR_STOPS, dof.max_blur, step.by());
                    });
                },
            }
            .stepper(),
            Setting {
                label: |l| fl!(l, "label-blur-quality"),
                help: |l| fl!(l, "help-blur-quality"),
                show: |w, l| {
                    let dof = w.focused::<Graphics>()?.depth_of_field;
                    let label = |quality: &DepthOfFieldQuality| match quality {
                        DepthOfFieldQuality::Low => fl!(l, "value-low"),
                        DepthOfFieldQuality::Medium => fl!(l, "value-medium"),
                        DepthOfFieldQuality::High => fl!(l, "value-high"),
                        DepthOfFieldQuality::Ultra => fl!(l, "value-ultra"),
                    };
                    Some(choice(&DepthOfFieldQuality::ALL, &dof.quality, label).when(blurs(w)))
                },
                change: |w, step| {
                    tune(w, |g, _| {
                        pick(&DepthOfFieldQuality::ALL, &mut g.depth_of_field.quality, step);
                    });
                },
            }
            .segments(),
            Setting {
                label: |l| fl!(l, "label-blur-foreground"),
                help: |l| fl!(l, "help-blur-foreground"),
                show: |w, l| {
                    let dof = w.focused::<Graphics>()?.depth_of_field;
                    Some(switch(l, dof.foreground).when(blurs(w)))
                },
                change: |w, step| {
                    w.edit(|g: &mut Graphics| flip(&mut g.depth_of_field.foreground, step));
                },
            }
            .switch(),
            Setting {
                label: |l| fl!(l, "label-bokeh-highlights"),
                help: |l| fl!(l, "help-bokeh-highlights"),
                show: |w, l| {
                    let dof = w.focused::<Graphics>()?.depth_of_field;
                    let text = if dof.highlights <= 0.0 {
                        on_off(l, false)
                    } else {
                        percent(dof.highlights)
                    };
                    let bokeh = blurs(w) && dof.shape == DepthOfFieldShape::Bokeh;
                    Some(HIGHLIGHTS.number(text, dof.highlights).when(bokeh))
                },
                change: |w, step| {
                    w.edit(|g: &mut Graphics| {
                        let dof = &mut g.depth_of_field;
                        dof.highlights = HIGHLIGHTS.nudge(dof.highlights, step);
                    });
                },
            }
            .stepper(),
        ],
    )
}

fn strokes() -> impl Bundle {
    section(
        |l| fl!(l, "section-strokes"),
        Section::Strokes,
        children![
            Setting {
                label: |l| fl!(l, "label-strokes"),
                help: |l| fl!(l, "help-strokes"),
                show: |w, l| Some(switch(l, w.focused::<Graphics>()?.draws_strokes())),
                // Turning them on while the preset keeps them off lets them show.
                change: |w, step| {
                    let Some(mut on) = w.focused::<Graphics>().map(Graphics::draws_strokes) else {
                        return;
                    };
                    flip(&mut on, step);
                    tune(w, |g, _| {
                        g.strokes = on;
                        g.allowed.strokes |= on;
                    });
                },
            }
            .switch(),
            Setting {
                label: |l| fl!(l, "label-stroke-width"),
                help: |l| fl!(l, "help-stroke-width"),
                show: |w, l| {
                    let graphics = w.focused::<Graphics>()?;
                    let label = |width: &StrokeWidth| match width {
                        StrokeWidth::Thin => fl!(l, "value-width-thin"),
                        StrokeWidth::Medium => fl!(l, "value-width-medium"),
                        StrokeWidth::Thick => fl!(l, "value-width-thick"),
                    };
                    let current = graphics.stroke_width;
                    Some(choice(&StrokeWidth::ALL, &current, label).when(graphics.draws_strokes()))
                },
                change: |w, step| {
                    w.edit(|g: &mut Graphics| pick(&StrokeWidth::ALL, &mut g.stroke_width, step));
                },
            }
            .segments(),
            Setting {
                label: |l| fl!(l, "label-stroke-strength"),
                help: |l| fl!(l, "help-stroke-strength"),
                show: |w, l| {
                    let graphics = w.focused::<Graphics>()?;
                    let label = |strength: &StrokeStrength| match strength {
                        StrokeStrength::Light => fl!(l, "value-strength-light"),
                        StrokeStrength::Medium => fl!(l, "value-strength-medium"),
                        StrokeStrength::Strong => fl!(l, "value-strength-strong"),
                    };
                    let current = graphics.stroke_strength;
                    Some(
                        choice(&StrokeStrength::ALL, &current, label)
                            .when(graphics.draws_strokes()),
                    )
                },
                change: |w, step| {
                    w.edit(|g: &mut Graphics| {
                        pick(&StrokeStrength::ALL, &mut g.stroke_strength, step);
                    });
                },
            }
            .segments(),
            Setting {
                label: |l| fl!(l, "label-stroke-creases"),
                help: |l| fl!(l, "help-stroke-creases"),
                show: |w, l| {
                    let graphics = w.focused::<Graphics>()?;
                    Some(switch(l, graphics.stroke_creases).when(graphics.draws_strokes()))
                },
                change: |w, step| w.edit(|g: &mut Graphics| flip(&mut g.stroke_creases, step)),
            }
            .switch(),
            Setting {
                label: |l| fl!(l, "label-stroke-hand-drawn"),
                help: |l| fl!(l, "help-stroke-hand-drawn"),
                show: |w, l| {
                    let graphics = w.focused::<Graphics>()?;
                    Some(switch(l, graphics.stroke_hand_drawn).when(graphics.draws_strokes()))
                },
                change: |w, step| w.edit(|g: &mut Graphics| flip(&mut g.stroke_hand_drawn, step)),
            }
            .switch(),
        ],
    )
}

fn fog() -> impl Bundle {
    section(
        |l| fl!(l, "section-fog"),
        Section::Fog,
        children![
            Setting {
                label: |l| fl!(l, "label-fog"),
                help: |l| fl!(l, "help-fog"),
                show: |w, l| Some(switch(l, w.focused::<Graphics>()?.fog).when(foggy(w))),
                change: |w, step| tune(w, |g, _| flip(&mut g.fog, step)),
            }
            .switch(),
            Setting {
                label: |l| fl!(l, "label-fog-density"),
                help: |l| fl!(l, "help-fog-density"),
                show: |w, _| {
                    let graphics = w.focused::<Graphics>()?;
                    let density = graphics.fog_density;
                    let [min, .., max] = FOG_DENSITY_STOPS;
                    let shown = number(percent(density), density.ln(), min.ln(), max.ln());
                    Some(shown.when(graphics.fog && foggy(w)))
                },
                change: |w, step| {
                    w.edit(|g: &mut Graphics| {
                        g.fog_density = step_stops(&FOG_DENSITY_STOPS, g.fog_density, step.by());
                    });
                },
            }
            .stepper(),
            Setting {
                label: |l| fl!(l, "label-fog-quality"),
                help: |l| fl!(l, "help-fog-quality"),
                show: |w, l| {
                    let graphics = w.focused::<Graphics>()?;
                    let label = |quality: &FogQuality| match quality {
                        FogQuality::VeryLow => fl!(l, "value-very-low"),
                        FogQuality::Low => fl!(l, "value-low"),
                        FogQuality::Medium => fl!(l, "value-medium"),
                        FogQuality::High => fl!(l, "value-high"),
                        FogQuality::Ultra => fl!(l, "value-ultra"),
                    };
                    let shown = graphics.fog && foggy(w);
                    Some(choice(&FogQuality::ALL, &graphics.fog_quality, label).when(shown))
                },
                change: |w, step| {
                    tune(w, |g, _| pick(&FogQuality::ALL, &mut g.fog_quality, step));
                },
            }
            .picker(),
            Setting {
                label: |l| fl!(l, "label-fog-dithering"),
                help: |l| fl!(l, "help-fog-dithering"),
                show: |w, l| {
                    let graphics = w.focused::<Graphics>()?;
                    Some(switch(l, graphics.fog_dithering).when(graphics.fog && foggy(w)))
                },
                change: |w, step| tune(w, |g, _| flip(&mut g.fog_dithering, step)),
            }
            .switch(),
            Setting {
                label: |l| fl!(l, "label-fog-resolution"),
                help: |l| fl!(l, "help-fog-resolution"),
                show: |w, l| {
                    let graphics = w.focused::<Graphics>()?;
                    let label = |resolution: &FogResolution| match resolution {
                        FogResolution::Full => fl!(l, "value-full"),
                        FogResolution::Half => fl!(l, "value-half"),
                        FogResolution::Quarter => fl!(l, "value-quarter"),
                    };
                    let current = graphics.fog_resolution;
                    let shown = graphics.fog && foggy(w);
                    Some(choice(&FogResolution::ALL, &current, label).when(shown))
                },
                change: |w, step| {
                    tune(w, |g, _| pick(&FogResolution::ALL, &mut g.fog_resolution, step));
                },
            }
            .segments(),
        ],
    )
}

// Image, color and film, each folding on its own inside.
fn post_processing() -> impl Bundle {
    section(
        |l| fl!(l, "section-post-processing"),
        Section::PostProcessing,
        children![(
            Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(12.0), ..default() },
            Pickable::IGNORE,
            children![
                image(),
                color(),
                film(),
                translated_button(|l| fl!(l, "restore-look"))
                    .node(|n| n.display = Display::None)
                    .build(RestoreButton),
            ],
        )],
    )
}

fn image() -> impl Bundle {
    subsection(
        |l| fl!(l, "section-image"),
        Section::Image,
        children![
            Setting {
                label: |l| fl!(l, "label-tonemapping"),
                help: |l| fl!(l, "help-tonemapping"),
                show: |w, _| {
                    let current = resolved(w)?.tonemapping;
                    Some(choice(&Tonemapping::ALL, &current, |t| tonemapping_name(*t).into()))
                },
                change: |w, step| {
                    edit_look(w, |look, current, _| {
                        look.post.tonemapping =
                            stepped(&Tonemapping::ALL, &current.tonemapping, step);
                    });
                },
            }
            .picker(),
            // Kept for the environment on screen while one shows, and for the suite otherwise.
            Setting {
                label: |l| fl!(l, "label-exposure"),
                help: |l| fl!(l, "help-exposure"),
                show: |w, _| {
                    let exposure = resolved(w)?.exposure;
                    Some(EXPOSURE.balance(format!("{exposure:+.1} EV"), exposure))
                },
                change: |w, step| {
                    edit_look(w, |look, current, environment| {
                        let exposure = Some(EXPOSURE.nudge(current.exposure, step));
                        match environment {
                            Some(name) => {
                                look.environments.entry(name.into()).or_default().exposure =
                                    exposure;
                            }
                            None => look.post.exposure = exposure,
                        }
                    });
                },
            }
            .stepper(),
            Setting {
                label: |l| fl!(l, "label-bloom"),
                help: |l| fl!(l, "help-bloom"),
                show: |w, l| Some(strength(l, showing(w, resolved(w)?.bloom, |a| a.bloom), BLOOM)),
                change: |w, step| {
                    if gated(w, |a| &mut a.bloom, step) {
                        return;
                    }
                    edit_look(w, |look, current, _| {
                        look.post.bloom = Some(BLOOM.nudge(current.bloom, step));
                    });
                },
            }
            .stepper(),
            // Does nothing with bloom off.
            Setting {
                label: |l| fl!(l, "label-bloom-threshold"),
                help: |l| fl!(l, "help-bloom-threshold"),
                show: |w, _| {
                    let current = resolved(w)?;
                    let threshold = current.bloom_threshold;
                    let shown = BLOOM_THRESHOLD.number(format!("{threshold:.1}"), threshold);
                    Some(shown.when(showing(w, current.bloom, |a| a.bloom) > 0.0))
                },
                change: |w, step| {
                    edit_look(w, |look, current, _| {
                        let threshold = BLOOM_THRESHOLD.nudge(current.bloom_threshold, step);
                        look.post.bloom_threshold = Some(threshold);
                    });
                },
            }
            .stepper(),
            Setting {
                label: |l| fl!(l, "label-chromatic-aberration"),
                help: |l| fl!(l, "help-chromatic-aberration"),
                show: |w, l| {
                    let amount = resolved(w)?.chromatic_aberration;
                    Some(strength(l, showing(w, amount, |a| a.chromatic_aberration), ABERRATION))
                },
                change: |w, step| {
                    if gated(w, |a| &mut a.chromatic_aberration, step) {
                        return;
                    }
                    edit_look(w, |look, current, _| {
                        let aberration = ABERRATION.nudge(current.chromatic_aberration, step);
                        look.post.chromatic_aberration = Some(aberration);
                    });
                },
            }
            .stepper(),
        ],
    )
}

fn color() -> impl Bundle {
    subsection(
        |l| fl!(l, "section-color"),
        Section::Color,
        children![
            Setting {
                label: |l| fl!(l, "label-warmth"),
                help: |l| fl!(l, "help-warmth"),
                show: |w, _| {
                    let warmth = resolved(w)?.warmth;
                    Some(SHIFT.balance(signed_percent(warmth), warmth))
                },
                change: |w, step| {
                    edit_look(w, |look, current, _| {
                        look.post.grading.warmth = Some(SHIFT.nudge(current.warmth, step));
                    });
                },
            }
            .stepper(),
            Setting {
                label: |l| fl!(l, "label-tint"),
                help: |l| fl!(l, "help-tint"),
                show: |w, _| {
                    let tint = resolved(w)?.tint;
                    Some(SHIFT.balance(signed_percent(tint), tint))
                },
                change: |w, step| {
                    edit_look(w, |look, current, _| {
                        look.post.grading.tint = Some(SHIFT.nudge(current.tint, step));
                    });
                },
            }
            .stepper(),
            Setting {
                label: |l| fl!(l, "label-saturation"),
                help: |l| fl!(l, "help-saturation"),
                show: |w, _| {
                    let saturation = resolved(w)?.saturation;
                    Some(SATURATION.balance(percent(saturation), saturation))
                },
                change: |w, step| {
                    edit_look(w, |look, current, _| {
                        look.post.grading.saturation =
                            Some(SATURATION.nudge(current.saturation, step));
                    });
                },
            }
            .stepper(),
            Setting {
                label: |l| fl!(l, "label-contrast"),
                help: |l| fl!(l, "help-contrast"),
                show: |w, _| {
                    let contrast = resolved(w)?.contrast;
                    Some(CONTRAST.balance(percent(contrast), contrast))
                },
                change: |w, step| {
                    edit_look(w, |look, current, _| {
                        look.post.grading.contrast = Some(CONTRAST.nudge(current.contrast, step));
                    });
                },
            }
            .stepper(),
            // The LUT rows only for a suite with LUTs.
            Setting {
                label: |l| fl!(l, "label-lut"),
                help: |l| fl!(l, "help-lut"),
                show: |w, l| {
                    let options = luts(w)?;
                    let current = w.focused::<ActiveLook>().and_then(|look| look.post.lut.name());
                    let current = current.map(str::to_string);
                    let label =
                        |lut: &Option<String>| lut.clone().unwrap_or_else(|| on_off(l, false));
                    Some(choice(&options, &current, label).when(options.len() > 1))
                },
                change: |w, step| {
                    let Some(options) = luts(w) else {
                        return;
                    };
                    w.edit(|look: &mut ActiveLook| {
                        let current = look.post.lut.name().map(str::to_string);
                        if let Some(next) = stepped(&options, &current, step) {
                            look.post.lut.name = Some(next.unwrap_or_else(|| NO_LUT.to_string()));
                        }
                    });
                },
            }
            .picker(),
            Setting {
                label: |l| fl!(l, "label-lut-strength"),
                help: |l| fl!(l, "help-lut-strength"),
                show: |w, _| {
                    let has_luts = luts(w).is_some_and(|options| options.len() > 1);
                    let picked =
                        w.focused::<ActiveLook>().is_some_and(|l| l.post.lut.name().is_some());
                    let strength = resolved(w)?.lut_strength;
                    let shown = LUT_STRENGTH.number(percent(strength), strength);
                    Some(shown.when(has_luts && picked))
                },
                change: |w, step| {
                    edit_look(w, |look, current, _| {
                        look.post.lut.strength =
                            Some(LUT_STRENGTH.nudge(current.lut_strength, step));
                    });
                },
            }
            .stepper(),
        ],
    )
}

fn film() -> impl Bundle {
    subsection(
        |l| fl!(l, "section-film"),
        Section::Film,
        children![
            Setting {
                label: |l| fl!(l, "label-vignette"),
                help: |l| fl!(l, "help-vignette"),
                show: |w, l| {
                    Some(strength(l, showing(w, resolved(w)?.vignette, |a| a.vignette), VIGNETTE))
                },
                change: |w, step| {
                    if gated(w, |a| &mut a.vignette, step) {
                        return;
                    }
                    edit_look(w, |look, current, _| {
                        look.post.vignette.intensity = Some(VIGNETTE.nudge(current.vignette, step));
                    });
                },
            }
            .stepper(),
            Setting {
                label: |l| fl!(l, "label-vignette-size"),
                help: |l| fl!(l, "help-vignette-size"),
                show: |w, _| {
                    let resolved = resolved(w)?;
                    let size = resolved.vignette_size;
                    Some(
                        VIGNETTE_SIZE
                            .number(format!("{size:.1}"), size)
                            .when(showing(w, resolved.vignette, |a| a.vignette) > 0.0),
                    )
                },
                change: |w, step| {
                    edit_look(w, |look, current, _| {
                        look.post.vignette.size =
                            Some(VIGNETTE_SIZE.nudge(current.vignette_size, step));
                    });
                },
            }
            .stepper(),
            Setting {
                label: |l| fl!(l, "label-grain"),
                help: |l| fl!(l, "help-grain"),
                show: |w, l| Some(strength(l, showing(w, resolved(w)?.grain, |a| a.grain), GRAIN)),
                change: |w, step| {
                    if gated(w, |a| &mut a.grain, step) {
                        return;
                    }
                    edit_look(w, |look, current, _| {
                        look.post.grain.intensity = Some(GRAIN.nudge(current.grain, step));
                    });
                },
            }
            .stepper(),
            Setting {
                label: |l| fl!(l, "label-grain-size"),
                help: |l| fl!(l, "help-grain-size"),
                show: |w, _| {
                    let resolved = resolved(w)?;
                    let size = resolved.grain_size;
                    Some(
                        GRAIN_SIZE
                            .number(format!("{size:.2} px"), size)
                            .when(showing(w, resolved.grain, |a| a.grain) > 0.0),
                    )
                },
                change: |w, step| {
                    edit_look(w, |look, current, _| {
                        look.post.grain.size = Some(GRAIN_SIZE.nudge(current.grain_size, step));
                    });
                },
            }
            .stepper(),
        ],
    )
}

// An intensity reads as a share of the strongest the row allows, Off at none.
// The focused stage's graphics, with the settings every stage shares, which presets set too.
fn edit_graphics(world: &mut World, edit: impl FnOnce(&mut Graphics, &mut RenderSettings)) {
    let mut render = world.resource::<RenderSettings>().clone();
    world.edit(|graphics: &mut Graphics| edit(graphics, &mut render));
    if *world.resource::<RenderSettings>() != render {
        *world.resource_mut::<RenderSettings>() = render;
    }
}

// Changes by hand something a preset sets. When anything did change, what is there now is the
// viewer's own mix.
fn tune(world: &mut World, change: impl FnOnce(&mut Graphics, &mut RenderSettings)) {
    edit_graphics(world, |graphics, render| {
        let before = Tuning::of(graphics, render);
        change(graphics, render);
        if Tuning::of(graphics, render) != before {
            graphics.tuned_by_hand();
        }
    });
}

// How much of a look's effect shows: none while the preset keeps it from showing.
fn showing(world: &World, amount: f32, allowed: fn(&Allowed) -> bool) -> f32 {
    let shows = world.focused::<Graphics>().is_none_or(|g| allowed(&g.allowed));
    if shows { amount } else { 0.0 }
}

// While the preset keeps a look's effect from showing, its row shows Off and turning it up
// lets it show, at the look's own amount, rather than changing that amount. True when it took
// the step.
fn gated(world: &mut World, allowed: fn(&mut Allowed) -> &mut bool, step: &Step) -> bool {
    let Some(mut now) = world.focused::<Graphics>().map(|g| g.allowed) else {
        return false;
    };
    if *allowed(&mut now) {
        return false;
    }
    if step.by() > 0 {
        tune(world, |graphics, _| *allowed(&mut graphics.allowed) = true);
    }
    true
}

fn strength(l: &FluentLanguageLoader, value: f32, scale: Scale) -> Shown {
    let text = if value <= 0.0 { on_off(l, false) } else { percent(value / scale.max) };
    scale.number(text, value)
}

// None is no LUT.
fn luts(world: &World) -> Option<Vec<Option<String>>> {
    let suite = world.focused::<ActiveSuite>()?;
    Some(std::iter::once(None).chain(suite.luts.iter().map(|l| Some(l.name.clone()))).collect())
}

fn tonemapping_name(tonemapping: Tonemapping) -> &'static str {
    match tonemapping {
        Tonemapping::None => "None",
        Tonemapping::Reinhard => "Reinhard",
        Tonemapping::ReinhardLuminance => "Reinhard lum.",
        Tonemapping::AcesFitted => "ACES",
        Tonemapping::Agx => "AgX",
        Tonemapping::SomewhatBoringDisplayTransform => "Boring",
        Tonemapping::TonyMcMapface => "TonyMcMapface",
        Tonemapping::BlenderFilmic => "Filmic",
    }
}

fn show_restore(edited: Focused<LookEdited>, mut buttons: Query<&mut Node, With<RestoreButton>>) {
    let Some(edited) = edited.get() else {
        return;
    };
    for mut node in &mut buttons {
        node.display = display(edited.0);
    }
}

fn on_click(
    event: On<Pointer<Click>>,
    buttons: Query<(), With<RestoreButton>>,
    mut restore: MessageWriter<RestoreLook>,
) {
    if buttons.contains(event.entity) {
        restore.write(RestoreLook);
    }
}

// A value between stops, like one typed into suite.toml, moves to the next stop. Stops at the
// ends rather than going round.
fn step_stops(stops: &[f32], current: f32, by: i32) -> f32 {
    (0..by.unsigned_abs()).fold(current, |value, _| {
        let next = if by > 0 {
            stops.iter().copied().find(|&s| s > value * 1.001)
        } else {
            stops.iter().rev().copied().find(|&s| s < value * 0.999)
        };
        next.unwrap_or(value)
    })
}
