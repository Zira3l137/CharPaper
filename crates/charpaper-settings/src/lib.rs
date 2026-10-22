// What the viewer chooses that is kept between runs, in state.toml next to the executable,
// and the monitors the screen settings choose from. Plain data: whatever acts on a setting
// turns it into engine values itself.

mod audio;
mod file;
mod render;
mod screen;
mod stage;
mod system;
mod ui;

pub use audio::Volumes;
pub use file::Loaded;
pub use file::STATE_FILE;
pub use file::SavedState;
pub use file::load;
pub use file::write;
pub use render::AntiAliasing;
pub use render::DepthOfFieldQuality;
pub use render::FogQuality;
pub use render::FogResolution;
pub use render::FpsLimit;
pub use render::Graphics;
pub use render::RENDER_SCALES;
pub use render::RenderSettings;
pub use render::ShadowQuality;
pub use render::ShadowUpdates;
pub use screen::ChosenMonitor;
pub use screen::MonitorEntry;
pub use screen::Monitors;
pub use screen::ScreenSettings;
pub use screen::StageId;
pub use stage::GazeSettings;
pub use stage::MusicOrder;
pub use stage::MusicSettings;
pub use stage::OrbitPose;
pub use stage::Picks;
pub use stage::StageSeed;
pub use stage::StageSeeds;
pub use stage::ViewSettings;
pub use system::DisplayMode;
pub use system::StartAtLogin;
pub use system::SystemSettings;
pub use ui::Tab;
pub use ui::UiState;
