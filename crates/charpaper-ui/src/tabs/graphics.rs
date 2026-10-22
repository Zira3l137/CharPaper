use bevy::prelude::*;
use charpaper_scene::ActiveLook;
use charpaper_scene::ActiveSuite;
use charpaper_scene::Focused;
use charpaper_scene::LookEdited;
use charpaper_scene::RestoreLook;
use charpaper_scene::focused_changed;
use charpaper_settings::AmbientOcclusionQuality;
use charpaper_settings::AmbientOcclusionSoftness;
use charpaper_settings::AntiAliasing;
use charpaper_settings::DepthOfFieldQuality;
use charpaper_settings::FogQuality;
use charpaper_settings::FogResolution;
use charpaper_settings::Graphics;
use charpaper_settings::RENDER_SCALES;
use charpaper_settings::RenderSettings;
use charpaper_settings::ShadowQuality;
use charpaper_settings::ShadowUpdates;
use charpaper_settings::StrokeStrength;
use charpaper_settings::StrokeWidth;
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
const LUT_STRENGTH: Scale = Scale { step: 0.05, min: 0.0, max: 1.0, per_unit: 100.0 };
// cd/m², spaced so each step looks about as big as the last.
const BRIGHTNESS_STOPS: [f32; 15] = [
    50.0, 100.0, 200.0, 300.0, 500.0, 700.0, 1000.0, 1500.0, 2000.0, 3000.0, 5000.0, 8000.0,
    12000.0, 20000.0, 30000.0,
];

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
// shares it.
pub(crate) fn page() -> impl Bundle {
    children![quality(), lighting(), fog(), strokes(), post_processing()]
}

fn lighting() -> impl Bundle {
    section(
        |l| fl!(l, "section-lighting"),
        Section::Lighting,
        children![
            Setting {
                label: |l| fl!(l, "label-brightness"),
                help: |l| fl!(l, "help-brightness"),
                show: |w, _| {
                    let lit = shown_environment(w)
                        .is_some_and(|e| e.sky().is_some() || e.reflections().is_some());
                    let brightness = resolved(w)?.brightness;
                    let [min, .., max] = BRIGHTNESS_STOPS;
                    let text = format!("{brightness:.0}");
                    Some(number(text, brightness.ln(), min.ln(), max.ln()).when(lit))
                },
                change: |w, step| {
                    edit_look(w, |look, current, environment| {
                        if let Some(name) = environment {
                            let brightness =
                                step_stops(&BRIGHTNESS_STOPS, current.brightness, step.by());
                            look.environments.entry(name.into()).or_default().brightness =
                                Some(brightness);
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
            .segments(),
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
                    let mut settings = w.resource_mut::<RenderSettings>();
                    pick(&ShadowQuality::ALL, &mut settings.shadow_quality, step);
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
                    let mut settings = w.resource_mut::<RenderSettings>();
                    pick(&ShadowUpdates::ALL, &mut settings.shadow_updates, step);
                },
            }
            .segments(),
            Setting {
                label: |l| fl!(l, "label-ambient-occlusion"),
                help: |l| fl!(l, "help-ambient-occlusion"),
                show: |w, l| {
                    Some(switch(l, w.focused::<Graphics>()?.ambient_occlusion).when(sky_lit(w)))
                },
                change: |w, step| w.edit(|g: &mut Graphics| flip(&mut g.ambient_occlusion, step)),
            }
            .segments(),
            Setting {
                label: |l| fl!(l, "label-ambient-occlusion-quality"),
                help: |l| fl!(l, "help-ambient-occlusion-quality"),
                show: |w, l| {
                    let graphics = w.focused::<Graphics>()?;
                    let current = graphics.ambient_occlusion_quality;
                    let shown = graphics.ambient_occlusion && sky_lit(w);
                    let label = |quality: &AmbientOcclusionQuality| match quality {
                        AmbientOcclusionQuality::Low => fl!(l, "value-low"),
                        AmbientOcclusionQuality::Medium => fl!(l, "value-medium"),
                        AmbientOcclusionQuality::High => fl!(l, "value-high"),
                        AmbientOcclusionQuality::Ultra => fl!(l, "value-ultra"),
                    };
                    Some(choice(&AmbientOcclusionQuality::ALL, &current, label).when(shown))
                },
                change: |w, step| {
                    w.edit(|g: &mut Graphics| {
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
                    let shown = graphics.ambient_occlusion && sky_lit(w);
                    let label = |softness: &AmbientOcclusionSoftness| match softness {
                        AmbientOcclusionSoftness::Sharp => fl!(l, "value-sharp"),
                        AmbientOcclusionSoftness::Medium => fl!(l, "value-medium"),
                        AmbientOcclusionSoftness::Soft => fl!(l, "value-soft"),
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

// Ambient occlusion only darkens the sky's light, and there is none without its maps.
fn sky_lit(world: &World) -> bool {
    shown_environment(world).is_some_and(|e| e.sky().is_some() || e.reflections().is_some())
}

fn quality() -> impl Bundle {
    section(
        |l| fl!(l, "section-quality"),
        Section::Quality,
        children![
            Setting {
                label: |l| fl!(l, "label-render-scale"),
                help: |l| fl!(l, "help-render-scale"),
                show: |w, _| {
                    let scale = w.focused::<Graphics>()?.scale_percent();
                    Some(choice(&RENDER_SCALES, &scale, |scale| format!("{scale}%")))
                },
                change: |w, step| {
                    w.edit(|graphics: &mut Graphics| {
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
                    w.edit(|g: &mut Graphics| pick(&AntiAliasing::ALL, &mut g.anti_aliasing, step));
                },
            }
            .segments(),
            Setting {
                label: |l| fl!(l, "label-depth-of-field"),
                help: |l| fl!(l, "help-depth-of-field"),
                show: |w, l| {
                    let current = w.focused::<Graphics>()?.depth_of_field;
                    Some(choice(&DepthOfFieldQuality::ALL, &current, |quality| match quality {
                        DepthOfFieldQuality::Off => on_off(l, false),
                        DepthOfFieldQuality::Blur => fl!(l, "value-blur"),
                        DepthOfFieldQuality::Bokeh => fl!(l, "value-bokeh"),
                    }))
                },
                change: |w, step| {
                    w.edit(|g: &mut Graphics| {
                        pick(&DepthOfFieldQuality::ALL, &mut g.depth_of_field, step);
                    });
                },
            }
            .segments(),
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
                show: |w, l| Some(switch(l, w.focused::<Graphics>()?.strokes)),
                change: |w, step| w.edit(|g: &mut Graphics| flip(&mut g.strokes, step)),
            }
            .segments(),
            Setting {
                label: |l| fl!(l, "label-stroke-width"),
                help: |l| fl!(l, "help-stroke-width"),
                show: |w, l| {
                    let graphics = w.focused::<Graphics>()?;
                    let label = |width: &StrokeWidth| match width {
                        StrokeWidth::Thin => fl!(l, "value-thin"),
                        StrokeWidth::Medium => fl!(l, "value-medium"),
                        StrokeWidth::Thick => fl!(l, "value-thick"),
                    };
                    let current = graphics.stroke_width;
                    Some(choice(&StrokeWidth::ALL, &current, label).when(graphics.strokes))
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
                        StrokeStrength::Light => fl!(l, "value-light"),
                        StrokeStrength::Medium => fl!(l, "value-medium"),
                        StrokeStrength::Strong => fl!(l, "value-strong"),
                    };
                    let current = graphics.stroke_strength;
                    Some(choice(&StrokeStrength::ALL, &current, label).when(graphics.strokes))
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
                    Some(switch(l, graphics.stroke_creases).when(graphics.strokes))
                },
                change: |w, step| w.edit(|g: &mut Graphics| flip(&mut g.stroke_creases, step)),
            }
            .segments(),
            Setting {
                label: |l| fl!(l, "label-stroke-hand-drawn"),
                help: |l| fl!(l, "help-stroke-hand-drawn"),
                show: |w, l| {
                    let graphics = w.focused::<Graphics>()?;
                    Some(switch(l, graphics.stroke_hand_drawn).when(graphics.strokes))
                },
                change: |w, step| w.edit(|g: &mut Graphics| flip(&mut g.stroke_hand_drawn, step)),
            }
            .segments(),
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
                show: |w, l| Some(switch(l, w.focused::<Graphics>()?.fog)),
                change: |w, step| w.edit(|g: &mut Graphics| flip(&mut g.fog, step)),
            }
            .segments(),
            Setting {
                label: |l| fl!(l, "label-fog-quality"),
                help: |l| fl!(l, "help-fog-quality"),
                show: |w, l| {
                    let current = w.focused::<Graphics>()?.fog_quality;
                    Some(choice(&FogQuality::ALL, &current, |quality| match quality {
                        FogQuality::VeryLow => fl!(l, "value-very-low"),
                        FogQuality::Low => fl!(l, "value-low"),
                        FogQuality::Medium => fl!(l, "value-medium"),
                        FogQuality::High => fl!(l, "value-high"),
                        FogQuality::Ultra => fl!(l, "value-ultra"),
                    }))
                },
                change: |w, step| {
                    w.edit(|g: &mut Graphics| pick(&FogQuality::ALL, &mut g.fog_quality, step));
                },
            }
            .picker(),
            Setting {
                label: |l| fl!(l, "label-fog-dithering"),
                help: |l| fl!(l, "help-fog-dithering"),
                show: |w, l| Some(switch(l, w.focused::<Graphics>()?.fog_dithering)),
                change: |w, step| w.edit(|g: &mut Graphics| flip(&mut g.fog_dithering, step)),
            }
            .segments(),
            Setting {
                label: |l| fl!(l, "label-fog-resolution"),
                help: |l| fl!(l, "help-fog-resolution"),
                show: |w, l| {
                    let current = w.focused::<Graphics>()?.fog_resolution;
                    Some(choice(&FogResolution::ALL, &current, |resolution| match resolution {
                        FogResolution::Full => fl!(l, "value-full"),
                        FogResolution::Half => fl!(l, "value-half"),
                        FogResolution::Quarter => fl!(l, "value-quarter"),
                    }))
                },
                change: |w, step| {
                    w.edit(|g: &mut Graphics| {
                        pick(&FogResolution::ALL, &mut g.fog_resolution, step);
                    });
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
    section(
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
                show: |w, l| Some(strength(l, resolved(w)?.bloom, BLOOM)),
                change: |w, step| {
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
                    Some(shown.when(current.bloom > 0.0))
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
                show: |w, l| Some(strength(l, resolved(w)?.chromatic_aberration, ABERRATION)),
                change: |w, step| {
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
    section(
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
                    let strength = resolved(w)?.lut_strength;
                    Some(LUT_STRENGTH.number(percent(strength), strength).when(has_luts))
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
    section(
        |l| fl!(l, "section-film"),
        Section::Film,
        children![
            Setting {
                label: |l| fl!(l, "label-vignette"),
                help: |l| fl!(l, "help-vignette"),
                show: |w, l| Some(strength(l, resolved(w)?.vignette, VIGNETTE)),
                change: |w, step| {
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
                    let size = resolved(w)?.vignette_size;
                    Some(VIGNETTE_SIZE.number(format!("{size:.1}"), size))
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
                show: |w, l| Some(strength(l, resolved(w)?.grain, GRAIN)),
                change: |w, step| {
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
                    let size = resolved(w)?.grain_size;
                    Some(GRAIN_SIZE.number(format!("{size:.2} px"), size))
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
