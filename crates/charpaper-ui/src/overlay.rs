use bevy::prelude::*;
use bevy::text::Justify;
use bevy::text::LineBreak;
use charpaper_monitor::AdapterKind;
use charpaper_monitor::Amount;
use charpaper_monitor::Gpu;
use charpaper_monitor::Memory;
use charpaper_monitor::Monitoring;
use charpaper_monitor::Readings;
use charpaper_scene::ScreenSync;
use charpaper_scene::ScreenUi;
use charpaper_settings::UiState;
use i18n_embed::fluent::FluentLanguageLoader;
use i18n_embed_fl::fl;

use crate::locale::Locale;
use crate::theme::*;
use crate::widgets::*;

// The performance monitor's readings at the top of every screen, switched on from the Render
// tab. Measuring starts and stops with it.
pub(crate) struct OverlayPlugin;

impl Plugin for OverlayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, spawn_overlays.after(ScreenSync))
            .add_observer(despawn_overlay)
            .add_systems(
                Update,
                (
                    switch_monitoring.run_if(resource_changed::<UiState>),
                    show_overlays.run_if(resource_changed::<UiState>.or_eager(new_overlay)),
                    fill_overlays.run_if(
                        resource_changed::<Readings>
                            .or_eager(resource_changed::<Locale>)
                            .or_eager(new_overlay),
                    ),
                ),
            );
    }
}

#[derive(Component)]
struct Overlay {
    screen: Entity,
}

// The readings sit in three columns of lines: what, how much, and more about it.
#[derive(Component, Clone, Copy)]
enum Column {
    Labels,
    Values,
    Notes,
}

fn new_overlay(overlays: Query<(), Added<Overlay>>) -> bool {
    !overlays.is_empty()
}

fn spawn_overlays(
    mut commands: Commands,
    screens: Query<(Entity, &ScreenUi)>,
    overlays: Query<&Overlay>,
) {
    for (screen, camera) in &screens {
        if !overlays.iter().any(|overlay| overlay.screen == screen) {
            commands.spawn(overlay(screen, camera.0));
        }
    }
}

fn despawn_overlay(
    removed: On<Remove, ScreenUi>,
    overlays: Query<(Entity, &Overlay)>,
    mut commands: Commands,
) {
    for (entity, overlay) in &overlays {
        if overlay.screen == removed.entity {
            commands.entity(entity).despawn();
        }
    }
}

// Centered at the top, clear of the panel on the right and of desktop icons, which Windows
// lines up from the top left.
fn overlay(screen: Entity, camera: Entity) -> impl Bundle {
    let column = |column, justify, color| {
        (label("", SMALL_SIZE, color), TextLayout::new(justify, LineBreak::NoWrap), column)
    };
    (
        Node {
            position_type: PositionType::Absolute,
            top: PANEL_MARGIN,
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            display: Display::None,
            ..default()
        },
        Pickable::IGNORE,
        UiTargetCamera(camera),
        Overlay { screen },
        children![(
            Node {
                column_gap: Val::Px(14.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                border: UiRect::all(LINE),
                border_radius: BorderRadius::all(PANEL_RADIUS),
                ..default()
            },
            Fill(PANEL_BG),
            Stroke(BORDER),
            Pickable::IGNORE,
            children![
                column(Column::Labels, Justify::Left, TEXT_LABEL),
                column(Column::Values, Justify::Right, TEXT),
                column(Column::Notes, Justify::Left, TEXT_DIM),
            ],
        )],
    )
}

fn switch_monitoring(ui: Res<UiState>, mut monitoring: ResMut<Monitoring>) {
    monitoring.set_if_neq(Monitoring(ui.performance_monitor));
}

fn show_overlays(ui: Res<UiState>, mut overlays: Query<&mut Node, With<Overlay>>) {
    for mut node in &mut overlays {
        node.display = display(ui.performance_monitor);
    }
}

fn fill_overlays(
    readings: Res<Readings>,
    locale: Res<Locale>,
    mut columns: Query<(&Column, &mut Text)>,
) {
    let rows = rows(&readings, &locale);
    let lines = |pick: fn(&Row) -> &str| rows.iter().map(pick).collect::<Vec<_>>().join("\n");
    let (labels, values, notes) =
        (lines(|row| &row.label), lines(|row| &row.value), lines(|row| &row.note));
    for (column, mut text) in &mut columns {
        let next = match column {
            Column::Labels => &labels,
            Column::Values => &values,
            Column::Notes => &notes,
        };
        if text.0 != *next {
            text.0.clone_from(next);
        }
    }
}

struct Row {
    label: String,
    value: String,
    note: String,
}

fn row(label: String, value: String, note: String) -> Row {
    Row { label, value, note }
}

// Nested parts of a reading are indented under it.
fn part(label: String) -> String {
    format!("    {label}")
}

fn subpart(label: String) -> String {
    format!("        {label}")
}

fn memory_rows(m: &Memory, l: &FluentLanguageLoader) -> Vec<Row> {
    let mb = |mb: f32| fl!(l, "monitor-mb", value = format!("{mb:.0}"));
    let value = |amount: Option<Amount>| amount.map(|a| mb(a.mb)).unwrap_or_else(|| "-".into());
    // How much it grew over the last minute, once there is enough of a minute to tell.
    let growth = |amount: Option<Amount>| {
        amount
            .and_then(|a| a.per_minute)
            .map(|rate| fl!(l, "monitor-growth", mb = format!("{rate:+.1}")))
            .unwrap_or_default()
    };
    let notes = |parts: &[String]| {
        parts.iter().filter(|p| !p.is_empty()).cloned().collect::<Vec<_>>().join(", ")
    };

    let mut rows = vec![
        row(fl!(l, "monitor-memory"), value(m.working_set), growth(m.working_set)),
        row(part(fl!(l, "monitor-committed")), value(m.committed), growth(m.committed)),
    ];
    // Counted only in diagnostics builds.
    if m.heap.is_some() {
        let blocks = fl!(l, "monitor-heap-blocks", count = m.heap_blocks.to_string());
        rows.push(row(
            part(fl!(l, "monitor-heap")),
            value(m.heap),
            notes(&[blocks, growth(m.heap)]),
        ));
    }
    if let Some(gpu) = &m.gpu {
        let allocated = Some(gpu.allocated);
        rows.push(row(
            part(fl!(l, "monitor-gpu-memory")),
            value(allocated),
            notes(&[
                fl!(l, "monitor-gpu-reserved", mb = format!("{:.0}", gpu.reserved_mb)),
                growth(allocated),
            ]),
        ));
        for (name, size) in &gpu.largest {
            rows.push(row(subpart(name.clone()), mb(*size), String::new()));
        }
    }
    if let Some(o) = m.gpu_objects {
        let total = o.buffers
            + o.textures
            + o.texture_views
            + o.bind_groups
            + o.samplers
            + o.pipelines
            + o.shader_modules;
        rows.push(row(
            part(fl!(l, "monitor-gpu-objects")),
            total.to_string(),
            fl!(
                l,
                "monitor-gpu-object-details",
                buffers = o.buffers.to_string(),
                textures = o.textures.to_string(),
                views = o.texture_views.to_string(),
                groups = o.bind_groups.to_string(),
                samplers = o.samplers.to_string(),
                pipelines = o.pipelines.to_string(),
                shaders = o.shader_modules.to_string()
            ),
        ));
    }
    rows.push(row(
        part(fl!(l, "monitor-entities")),
        m.entities.to_string(),
        m.render_entities
            .map(|count| fl!(l, "monitor-render-entities", count = count.to_string()))
            .unwrap_or_default(),
    ));
    let a = m.assets;
    rows.push(row(
        part(fl!(l, "monitor-assets")),
        (a.images + a.meshes + a.materials + a.clips + a.gltfs).to_string(),
        fl!(
            l,
            "monitor-asset-details",
            images = a.images.to_string(),
            meshes = a.meshes.to_string(),
            materials = a.materials.to_string(),
            clips = a.clips.to_string(),
            gltfs = a.gltfs.to_string()
        ),
    ));
    rows
}

fn rows(r: &Readings, l: &FluentLanguageLoader) -> Vec<Row> {
    let ms = |value: f32| fl!(l, "monitor-ms", value = format!("{value:.1}"));
    let maybe_ms = |value: Option<f32>| value.map(ms).unwrap_or_else(|| "-".into());
    let none = String::new;

    let mut rows = vec![
        row(fl!(l, "monitor-title"), none(), none()),
        row(
            fl!(l, "monitor-frame-rate"),
            fl!(l, "monitor-fps", value = format!("{:.0}", r.fps)),
            fl!(l, "monitor-frame-time", time = ms(r.frame_ms)),
        ),
        row(
            fl!(l, "monitor-cpu"),
            r.cpu_percent.map(|p| format!("{p:.0}%")).unwrap_or_else(|| "-".into()),
            r.cpu_percent
                .map(|p| {
                    let busy = p / 100.0 * r.cores as f32;
                    fl!(
                        l,
                        "monitor-cores",
                        busy = format!("{busy:.1}"),
                        total = r.cores.to_string()
                    )
                })
                .unwrap_or_default(),
        ),
        row(part(fl!(l, "monitor-main-thread")), maybe_ms(r.main_thread_ms), none()),
        row(part(fl!(l, "monitor-render-thread")), maybe_ms(r.render_thread_ms), none()),
    ];
    if let Some(stages) = r.render_stages {
        let recording = r
            .recording_ms
            .map(|time| fl!(l, "monitor-recording", time = ms(time)))
            .unwrap_or_default();
        rows.extend([
            row(subpart(fl!(l, "monitor-stage-assets")), ms(stages.assets), none()),
            row(subpart(fl!(l, "monitor-stage-specialize")), ms(stages.specialize), none()),
            row(
                subpart(fl!(l, "monitor-stage-views")),
                ms(stages.views),
                fl!(l, "monitor-stage-views-note"),
            ),
            row(subpart(fl!(l, "monitor-stage-queue")), ms(stages.queue), none()),
            row(subpart(fl!(l, "monitor-stage-prepare")), ms(stages.prepare), none()),
            row(subpart(fl!(l, "monitor-stage-render")), ms(stages.render), recording),
            row(subpart(fl!(l, "monitor-stage-cleanup")), ms(stages.cleanup), none()),
        ]);
    }
    rows.extend(memory_rows(&r.memory, l));

    if let Some(adapter) = &r.adapter {
        let kind = match adapter.kind {
            AdapterKind::Discrete => fl!(l, "monitor-discrete"),
            AdapterKind::Integrated => fl!(l, "monitor-integrated"),
            AdapterKind::Software => fl!(l, "monitor-software"),
            AdapterKind::Other => fl!(l, "monitor-other-gpu"),
        };
        rows.push(row(
            fl!(l, "monitor-graphics"),
            adapter.name.clone(),
            format!("{kind}, {}", adapter.backend),
        ));
    }

    match r.gpu {
        Gpu::Waiting => {
            rows.push(row(fl!(l, "monitor-gpu"), "-".into(), fl!(l, "monitor-waiting")))
        }
        Gpu::Unsupported => {
            rows.push(row(fl!(l, "monitor-gpu"), "-".into(), fl!(l, "monitor-unsupported")))
        }
        Gpu::Measured(gpu) => {
            // How much of the time the GPU spends on CharPaper, which is what Task Manager's
            // GPU column shows.
            let busy = gpu.total / r.frame_ms.max(0.001) * 100.0;
            let shadow_maps = r
                .shadow_maps
                .zip(r.shadow_maps_drawn)
                .map(|(count, drawn)| {
                    let (count, drawn) = (format!("{count:.0}"), format!("{drawn:.0}"));
                    fl!(l, "monitor-shadow-maps", count = count, drawn = drawn)
                })
                .unwrap_or_default();
            rows.extend([
                row(
                    fl!(l, "monitor-gpu"),
                    ms(gpu.total),
                    fl!(l, "monitor-busy", percent = format!("{busy:.0}%")),
                ),
                row(part(fl!(l, "monitor-light-shadows")), ms(gpu.light_shadows), shadow_maps),
                row(part(fl!(l, "monitor-prepass")), ms(gpu.prepass), none()),
                row(part(fl!(l, "monitor-main-pass")), ms(gpu.main), none()),
                row(part(fl!(l, "monitor-fog")), ms(gpu.fog), none()),
                row(part(fl!(l, "monitor-effects")), ms(gpu.effects), none()),
                row(part(fl!(l, "monitor-display")), ms(gpu.display), none()),
            ]);
        }
    }

    rows.push(row(
        fl!(l, "monitor-scene"),
        fl!(l, "monitor-lights", count = r.lights.total.to_string()),
        fl!(
            l,
            "monitor-light-details",
            shadowed = r.lights.shadowed.to_string(),
            fog = r.lights.in_fog.to_string(),
            boxes = r.fog_boxes.to_string()
        ),
    ));
    rows.push(row(
        part(fl!(l, "monitor-meshes")),
        r.meshes.total.to_string(),
        fl!(
            l,
            "monitor-mesh-details",
            drawn = r.meshes.drawn.to_string(),
            shadows = r.meshes.in_shadow_maps.to_string()
        ),
    ));
    rows
}
