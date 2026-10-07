// Bevy-free: suites are plain data, so this builds fast, is easy to test, and powers
// `--check-suite` without starting the engine.

mod cascades;
mod cube;
mod document;
mod edit;
mod error;
mod expressions;
mod fog;
mod grading;
mod inspect;
mod layout;
mod manifest;
mod shape_keys;

pub use cascades::FIRST_CASCADE;
pub use cascades::MAX_SHADOW_DISTANCE;
pub use cascades::sun_cascades;
pub use cube::CubeLut;
pub use cube::MAX_LUT_SIZE;
pub use cube::parse_cube;
pub use edit::BACKUP_FILE;
pub use edit::has_backup;
pub use edit::restore_look;
pub use edit::save_look;
pub use error::SuiteError;
pub use expressions::MAX_SHAPE_KEYS;
pub use expressions::SkinClips;
pub use expressions::skin_clips;
pub use fog::Medium;
pub use inspect::Cost;
pub use inspect::Finding;
pub use inspect::Report;
pub use inspect::Severity;
pub use inspect::costs;
pub use inspect::inspect;
pub use layout::AnimationFile;
pub use layout::ClipBinding;
pub use layout::ClipSet;
pub use layout::DIFFUSE_MAP;
pub use layout::Environment;
pub use layout::ExportedCamera;
pub use layout::Look;
pub use layout::LutFile;
pub use layout::ORBIT_CAMERA;
pub use layout::SKYBOX_MAP;
pub use layout::SPECULAR_MAP;
pub use layout::Skin;
pub use layout::Suite;
pub use layout::discover;
pub use manifest::Camera;
pub use manifest::CameraEntry;
pub use manifest::EnvironmentEntry;
pub use manifest::Fog;
pub use manifest::Gaze;
pub use manifest::GazeShapeKeys;
pub use manifest::Grading;
pub use manifest::GradingSection;
pub use manifest::Grain;
pub use manifest::Lut;
pub use manifest::MANIFEST_FILE;
pub use manifest::Manifest;
pub use manifest::NO_LUT;
pub use manifest::PlayMode;
pub use manifest::Post;
pub use manifest::SCHEMA_VERSION;
pub use manifest::Tonemapping;
pub use manifest::Vignette;
pub use shape_keys::MeshShapeKeys;
pub use shape_keys::mesh_shape_keys;

// The repository keeps a copy in schemas/suite.schema.json; a test fails when it goes stale.
pub fn manifest_schema() -> String {
    let schema = schemars::schema_for!(Manifest);
    serde_json::to_string_pretty(&schema).expect("a schema always serializes") + "\n"
}
