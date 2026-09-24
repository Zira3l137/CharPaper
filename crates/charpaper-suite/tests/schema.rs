use std::fs;
use std::path::PathBuf;

const UPDATE_VAR: &str = "CHARPAPER_UPDATE_SCHEMA";

#[test]
fn committed_schema_matches_the_manifest_types() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/suite.schema.json");
    let generated = charpaper_suite::manifest_schema();

    if std::env::var_os(UPDATE_VAR).is_some() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, generated).unwrap();
        return;
    }

    let committed = fs::read_to_string(&path).unwrap_or_default();
    assert!(
        committed.replace("\r\n", "\n") == generated,
        "{} is stale; rerun the tests with {UPDATE_VAR}=1 and commit the result",
        path.display()
    );
}

#[test]
fn example_manifest_parses() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/suite.example.toml");
    let text = fs::read_to_string(path).unwrap();
    let manifest: charpaper_suite::Manifest = toml::from_str(&text).unwrap();
    assert_eq!(manifest.animations.len(), 2);
}
