use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use bevy::ecs::system::SystemParam;
use bevy::light::FogVolume;
use bevy::prelude::*;
use bevy::render::renderer::RenderAdapterInfo;
use bevy::window::PrimaryWindow;
use charpaper_scene::ActiveLook;
use charpaper_scene::ActiveSuite;
use charpaper_scene::AntiAliasing;
use charpaper_scene::CharacterClips;
use charpaper_scene::CharacterState;
use charpaper_scene::DepthOfFieldQuality;
use charpaper_scene::Focused;
use charpaper_scene::FocusedMut;
use charpaper_scene::FogQuality;
use charpaper_scene::FogResolution;
use charpaper_scene::FpsLimit;
use charpaper_scene::Graphics;
use charpaper_scene::RENDER_SCALES;
use charpaper_scene::RenderSettings;
use charpaper_scene::Resolved;
use charpaper_scene::ShadowQuality;
use charpaper_suite::Look;

use crate::wallpaper::Paused;

pub const RESULTS_FILE: &str = "benchmark.txt";

// Long enough for the suite's files to load and their shaders to compile.
const WARM_UP: Duration = Duration::from_secs(8);
const LOAD_LIMIT: Duration = Duration::from_secs(60);
// A changed setting can need new shaders, which stall the first frames after it.
const SETTLE: Duration = Duration::from_secs(3);
const MEASURE: Duration = Duration::from_secs(5);

// Draws the shown scene with one render setting after another, as fast as it can, and reports
// how long a frame takes. Each row changes one thing from the saved settings in the first
// row, so its difference from that row is what the one thing costs.
pub struct BenchmarkPlugin {
    pub results: PathBuf,
}

impl Plugin for BenchmarkPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Benchmark { results: self.results.clone(), ..default() })
            .add_systems(Update, run);
    }
}

// Frames come as fast as the GPU allows, and the terminal the benchmark was started from
// can't pause it by covering the desktop.
pub fn unthrottle(settings: &mut RenderSettings) {
    settings.fps_limit = FpsLimit::Max;
    settings.pause_when_fullscreen = false;
    settings.pause_when_covered = false;
    settings.pause_on_battery = false;
}

#[derive(Resource, Default)]
struct Benchmark {
    results: PathBuf,
    phase: Phase,
    elapsed: Duration,
    waited: Duration,
    rows: Vec<Row>,
    next: usize,
    frames: Vec<f32>,
    look: Look,
    environment: Option<String>,
    header: String,
    measured: Vec<Measured>,
}

#[derive(Default, PartialEq)]
enum Phase {
    #[default]
    Loading,
    Settling,
    Measuring,
    Done,
}

struct Row {
    label: String,
    setup: Setup,
    shadows: Option<bool>,
}

// The settings every stage shares, and those of the stage measured.
#[derive(Clone, PartialEq)]
struct Setup {
    render: RenderSettings,
    graphics: Graphics,
}

// What a row changes, on the stage the benchmark measures: the focused one.
#[derive(SystemParam)]
struct Tuned<'w, 's> {
    render: ResMut<'w, RenderSettings>,
    graphics: FocusedMut<'w, 's, Graphics>,
    look: FocusedMut<'w, 's, ActiveLook>,
}

struct Measured {
    label: String,
    median: f32,
    slow: f32,
}

#[derive(SystemParam)]
struct Shown<'w, 's> {
    suite: Focused<'w, 's, ActiveSuite>,
    clips: Focused<'w, 's, CharacterClips>,
    state: Focused<'w, 's, CharacterState>,
    fog: Query<'w, 's, (), With<FogVolume>>,
    lights: Query<'w, 's, (), Or<(With<DirectionalLight>, With<PointLight>, With<SpotLight>)>>,
    window: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    adapter: Option<Res<'w, RenderAdapterInfo>>,
}

fn run(
    time: Res<Time<Real>>,
    mut bench: ResMut<Benchmark>,
    mut tuned: Tuned,
    paused: Res<Paused>,
    scene: Shown,
    mut exit: MessageWriter<AppExit>,
) {
    let delta = time.delta();
    let bench = &mut *bench;

    if paused.0.is_some() {
        if bench.phase == Phase::Measuring {
            bench.phase = Phase::Settling;
            bench.elapsed = Duration::ZERO;
        }
        return;
    }

    match bench.phase {
        Phase::Loading => {
            bench.waited += delta;
            // The clips are ready once the armature and every animation file have loaded.
            let ready = scene.suite.get().is_some() && scene.clips.get().is_some();
            if ready {
                bench.elapsed += delta;
            }
            if ready && bench.elapsed >= WARM_UP {
                let saved_look = tuned.look.get().map(|l| l.0.clone()).unwrap_or_default();
                let graphics = tuned.graphics.get().cloned().unwrap_or_default();
                let saved = Setup { render: tuned.render.clone(), graphics };
                start(bench, &saved, saved_look, &scene);
                apply(bench, &mut tuned);
            } else if bench.waited >= LOAD_LIMIT {
                error!("benchmark: the suite did not finish loading within a minute");
                exit.write(AppExit::error());
                bench.phase = Phase::Done;
            }
        }
        Phase::Settling => {
            bench.elapsed += delta;
            if bench.elapsed >= SETTLE {
                bench.phase = Phase::Measuring;
                bench.elapsed = Duration::ZERO;
                bench.frames.clear();
            }
        }
        Phase::Measuring => {
            bench.elapsed += delta;
            bench.frames.push(delta.as_secs_f32() * 1000.0);
            if bench.elapsed < MEASURE {
                return;
            }
            let row = &bench.rows[bench.next];
            let (median, slow) = spread(&mut bench.frames);
            info!("benchmark: {}: {median:.2} ms", row.label);
            bench.measured.push(Measured { label: row.label.clone(), median, slow });
            bench.next += 1;
            if bench.next < bench.rows.len() {
                apply(bench, &mut tuned);
            } else {
                finish(bench);
                exit.write(AppExit::Success);
                bench.phase = Phase::Done;
            }
        }
        Phase::Done => {}
    }
}

fn start(bench: &mut Benchmark, saved: &Setup, look: Look, scene: &Shown) {
    let environment = scene.state.get().and_then(|state| state.environment.clone());
    let shadows = Resolved::new(&look, environment.as_deref()).shadows;
    let lit = !scene.lights.is_empty();
    bench.rows = plan(saved, !scene.fog.is_empty(), lit.then_some(shadows));
    bench.look = look;
    bench.environment = environment;
    bench.header = header(scene);
    info!(
        "benchmark: measuring {} settings, about {} s",
        bench.rows.len(),
        bench.rows.len() as u64 * (SETTLE + MEASURE).as_secs()
    );
}

fn apply(bench: &mut Benchmark, tuned: &mut Tuned) {
    let row = &bench.rows[bench.next];
    if *tuned.render != row.setup.render {
        *tuned.render = row.setup.render.clone();
    }
    if let Some(mut graphics) = tuned.graphics.get_mut()
        && *graphics != row.setup.graphics
    {
        *graphics = row.setup.graphics.clone();
    }
    if let Some(mut look) = tuned.look.get_mut() {
        let mut wanted = bench.look.clone();
        if let (Some(shadows), Some(name)) = (row.shadows, &bench.environment) {
            wanted.environments.entry(name.clone()).or_default().shadows = Some(shadows);
        }
        if look.0 != wanted {
            look.0 = wanted;
        }
    }
    bench.phase = Phase::Settling;
    bench.elapsed = Duration::ZERO;
}

fn plan(saved: &Setup, fog: bool, shadows: Option<bool>) -> Vec<Row> {
    let mut rows = vec![changed(saved, "saved settings", |_| {})];

    if fog {
        if saved.graphics.fog {
            rows.push(changed(saved, "fog off", |s| s.graphics.fog = false));
        }
        for quality in FogQuality::ALL {
            if saved.graphics.fog && saved.graphics.fog_quality == quality {
                continue;
            }
            rows.push(changed(saved, format!("fog {}", fog_name(quality)), |s| {
                s.graphics.fog = true;
                s.graphics.fog_quality = quality;
            }));
        }
        for resolution in FogResolution::ALL {
            if saved.graphics.fog && saved.graphics.fog_resolution == resolution {
                continue;
            }
            let label = match resolution {
                FogResolution::Full => "fog full resolution",
                FogResolution::Half => "fog half resolution",
            };
            rows.push(changed(saved, label, |s| {
                s.graphics.fog = true;
                s.graphics.fog_resolution = resolution;
            }));
        }
    }
    if let Some(on) = shadows {
        let label = if on { "shadows off" } else { "shadows on" };
        rows.push(Row { label: label.into(), setup: saved.clone(), shadows: Some(!on) });
    }
    if shadows == Some(true) {
        for quality in ShadowQuality::ALL {
            if quality == saved.render.shadow_quality {
                continue;
            }
            let label = match quality {
                ShadowQuality::Low => "shadow quality low",
                ShadowQuality::Medium => "shadow quality medium",
                ShadowQuality::High => "shadow quality high",
            };
            rows.push(changed(saved, label, |s| s.render.shadow_quality = quality));
        }
    }
    for anti_aliasing in AntiAliasing::ALL {
        if anti_aliasing != saved.graphics.anti_aliasing {
            let label = format!("anti-aliasing {}", anti_aliasing_name(anti_aliasing));
            rows.push(changed(saved, label, |s| s.graphics.anti_aliasing = anti_aliasing));
        }
    }
    if saved.graphics.depth_of_field != DepthOfFieldQuality::Off {
        rows.push(changed(saved, "depth of field off", |s| {
            s.graphics.depth_of_field = DepthOfFieldQuality::Off;
        }));
    }
    for scale in RENDER_SCALES {
        if scale != saved.graphics.scale_percent() {
            rows.push(changed(saved, format!("render scale {scale}%"), |s| {
                s.graphics.render_scale = scale
            }));
        }
    }
    rows
}

fn changed(saved: &Setup, label: impl Into<String>, change: impl FnOnce(&mut Setup)) -> Row {
    let mut setup = saved.clone();
    change(&mut setup);
    Row { label: label.into(), setup, shadows: None }
}

fn header(scene: &Shown) -> String {
    let mut text = String::from("charpaper benchmark\n");
    if let Some(suite) = scene.suite.get() {
        let _ = write!(text, "suite {:?}", suite.name);
    }
    if let Some(state) = scene.state.get() {
        if let Some(environment) = &state.environment {
            let _ = write!(text, ", environment {environment:?}");
        }
        let _ = write!(text, ", camera {:?}", state.camera.as_deref().unwrap_or("orbit"));
    }
    text.push('\n');
    if let Some(adapter) = &scene.adapter {
        let _ = writeln!(text, "GPU {} ({:?})", adapter.name, adapter.backend);
    }
    if let Ok(window) = scene.window.single() {
        let _ = writeln!(text, "window {}×{}", window.physical_width(), window.physical_height());
    }
    text
}

fn finish(bench: &Benchmark) {
    let mut text = bench.header.clone();
    let _ = writeln!(
        text,
        "\nEach row changes one thing from the saved settings in the first row. \"frame\" is the\n\
         typical frame, \"slow\" the slowest 5%.\n"
    );
    let _ = writeln!(
        text,
        "{:<26}{:>10}{:>10}{:>8}{:>12}",
        "", "frame ms", "slow ms", "fps", "vs saved"
    );
    let baseline = bench.measured.first().map_or(0.0, |m| m.median);
    for (i, row) in bench.measured.iter().enumerate() {
        let fps = 1000.0 / row.median.max(0.001);
        let _ = write!(text, "{:<26}{:>10.2}{:>10.2}{:>8.0}", row.label, row.median, row.slow, fps);
        if i > 0 {
            let _ = write!(text, "{:>+12.2}", row.median - baseline);
        }
        text.push('\n');
    }

    println!("{text}");
    match fs::write(&bench.results, &text) {
        Ok(()) => info!("benchmark: results written to {}", bench.results.display()),
        Err(err) => warn!("benchmark: cannot write {}: {err}", bench.results.display()),
    }
}

// The median frame, and the frame only 5% are slower than.
fn spread(frames: &mut [f32]) -> (f32, f32) {
    if frames.is_empty() {
        return (0.0, 0.0);
    }
    frames.sort_by(f32::total_cmp);
    let at = |share: usize| frames[(frames.len() * share / 100).min(frames.len() - 1)];
    (at(50), at(95))
}

fn fog_name(quality: FogQuality) -> &'static str {
    match quality {
        FogQuality::VeryLow => "very low",
        FogQuality::Low => "low",
        FogQuality::Medium => "medium",
        FogQuality::High => "high",
        FogQuality::Ultra => "ultra",
    }
}

fn anti_aliasing_name(anti_aliasing: AntiAliasing) -> &'static str {
    match anti_aliasing {
        AntiAliasing::Off => "off",
        AntiAliasing::Msaa4 => "4x MSAA",
        AntiAliasing::Taa => "TAA",
    }
}
