use std::fs;
use std::path::PathBuf;

use charpaper_suite::EnvironmentEntry;
use charpaper_suite::Look;
use charpaper_suite::Tonemapping;
use charpaper_suite::has_backup;
use charpaper_suite::restore_look;
use charpaper_suite::save_look;

const AUTHORED: &str = r#"# Aki, by the author
schema = 1

[post]
bloom = 0.15   # a gentle glow
exposure = 0

[environments.room]
brightness = 1000
"#;

fn folder(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("charpaper-edit-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("suite.toml"), AUTHORED).unwrap();
    root
}

fn read(root: &PathBuf) -> String {
    fs::read_to_string(root.join("suite.toml")).unwrap()
}

#[test]
fn edits_keep_the_authors_file_and_back_it_up_once() {
    let root = folder("keep");
    let mut look = Look::default();
    look.post.bloom = Some(0.3);
    look.post.tonemapping = Some(Tonemapping::Agx);
    look.environments
        .insert("room".into(), EnvironmentEntry { shadows: Some(false), ..Default::default() });
    look.environments
        .insert("field".into(), EnvironmentEntry { exposure: Some(-2.5), ..Default::default() });

    assert!(save_look(&root, &look).unwrap());
    let text = read(&root);
    assert!(text.starts_with("# Aki, by the author\n"), "{text}");
    assert!(text.contains("bloom = 0.3   # a gentle glow"), "{text}");
    assert!(text.contains("tonemapping = \"agx\""), "{text}");
    assert!(text.contains("[environments.room]\nbrightness = 1000\nshadows = false"), "{text}");
    assert!(text.contains("[environments.field]\nexposure = -2.5"), "{text}");
    assert!(!text.contains("[environments]\n"), "{text}");
    assert_eq!(fs::read_to_string(root.join("suite.toml.bak")).unwrap(), AUTHORED);
    toml::from_str::<charpaper_suite::Manifest>(&text).unwrap();

    look.post.bloom = Some(0.5);
    assert!(save_look(&root, &look).unwrap());
    assert_eq!(fs::read_to_string(root.join("suite.toml.bak")).unwrap(), AUTHORED);

    assert!(restore_look(&root).unwrap());
    assert_eq!(read(&root), AUTHORED);
    assert!(!has_backup(&root));
    assert!(!restore_look(&root).unwrap());
}

#[test]
fn values_the_file_already_holds_change_nothing() {
    let root = folder("same");
    let mut look = Look::default();
    look.post.bloom = Some(0.15);
    look.post.exposure = Some(0.0);
    look.environments
        .insert("room".into(), EnvironmentEntry { brightness: Some(1000.0), ..Default::default() });

    assert!(!save_look(&root, &look).unwrap());
    assert_eq!(read(&root), AUTHORED);
    assert!(!has_backup(&root));
}
