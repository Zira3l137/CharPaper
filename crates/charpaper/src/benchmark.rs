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
use charpaper_scene::FogQuality;
use charpaper_scene::FpsLimit;
use charpaper_scene::RENDER_SCALES;
use charpaper_scene::RenderSettings;
use charpaper_scene::Resolved;
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
    settings: RenderSettings,
    shadows: Option<bool>,
}

struct Measured {
    label: String,
    median: f32,
    slow: f32,
}

#[derive(SystemParam)]
struct Shown<'w, 's> {
    suite: Option<Res<'w, ActiveSuite>>,
    clips: Option<Res<'w, CharacterClips>>,
    state: Res<'w, CharacterState>,
    fog: Query<'w, 's, (), With<FogVolume>>,
    lights: Query<'w, 's, (), Or<(With<DirectionalLight>, With<PointLight>, With<SpotLight>)>>,
    window: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    adapter: Option<Res<'w, RenderAdapterInfo>>,
}

fn run(
    time: Res<Time<Real>>,
    mut bench: ResMut<Benchmark>,
    mut settings: ResMut<RenderSettings>,
    mut look: Option<ResMut<ActiveLook>>,
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
            let ready = scene.suite.is_some() && scene.clips.is_some();
            if ready {
                bench.elapsed += delta;
            }
            if ready && bench.elapsed >= WARM_UP {
                let saved_look = look.as_deref().map(|l| l.0.clone()).unwrap_or_default();
                start(bench, &settings, saved_look, &scene);
                apply(bench, &mut settings, look.as_deref_mut());
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
                apply(bench, &mut settings, look.as_deref_mut());
            } else {
                finish(bench);
                exit.write(AppExit::Success);
                bench.phase = Phase::Done;
            }
        }
        Phase::Done => {}
    }
}

fn start(bench: &mut Benchmark, saved: &RenderSettings, look: Look, scene: &Shown) {
    let environment = scene.state.environment.clone();
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

fn apply(bench: &mut Benchmark, settings: &mut RenderSettings, look: Option<&mut ActiveLook>) {
    let row = &bench.rows[bench.next];
    if *settings != row.settings {
        *settings = row.settings.clone();
    }
    if let Some(look) = look {
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

fn plan(saved: &RenderSettings, fog: bool, shadows: Option<bool>) -> Vec<Row> {
    let mut rows = vec![changed(saved, "saved settings", |_| {})];

    if fog {
        if saved.fog {
            rows.push(changed(saved, "fog off", |s| s.fog = false));
        }
        for quality in FogQuality::ALL {
            if saved.fog && saved.fog_quality == quality {
                continue;
            }
            rows.push(changed(saved, format!("fog {}", fog_name(quality)), |s| {
                s.fog = true;
                s.fog_quality = quality;
            }));
        }
    }
    if let Some(on) = shadows {
        let label = if on { "shadows off" } else { "shadows on" };
        rows.push(Row { label: label.into(), settings: saved.clone(), shadows: Some(!on) });
    }
    for anti_aliasing in AntiAliasing::ALL {
        if anti_aliasing != saved.anti_aliasing {
            let label = format!("anti-aliasing {}", anti_aliasing_name(anti_aliasing));
            rows.push(changed(saved, label, |s| s.anti_aliasing = anti_aliasing));
        }
    }
    if saved.depth_of_field != DepthOfFieldQuality::Off {
        rows.push(changed(saved, "depth of field off", |s| {
            s.depth_of_field = DepthOfFieldQuality::Off;
        }));
    }
    for scale in RENDER_SCALES {
        if scale != saved.scale_percent() {
            rows.push(changed(saved, format!("render scale {scale}%"), |s| s.render_scale = scale));
        }
    }
    rows
}

fn changed(
    saved: &RenderSettings,
    label: impl Into<String>,
    change: impl FnOnce(&mut RenderSettings),
) -> Row {
    let mut settings = saved.clone();
    change(&mut settings);
    Row { label: label.into(), settings, shadows: None }
}

fn header(scene: &Shown) -> String {
    let mut text = String::from("charpaper benchmark\n");
    if let Some(suite) = &scene.suite {
        let _ = write!(text, "suite {:?}", suite.name);
    }
    let state = &scene.state;
    if let Some(environment) = &state.environment {
        let _ = write!(text, ", environment {environment:?}");
    }
    let _ = writeln!(text, ", camera {:?}", state.camera.as_deref().unwrap_or("orbit"));
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
