use charpaper_suite::EnvironmentEntry;
use charpaper_suite::Look;
use charpaper_suite::LookEdits;

fn look(sun_follows_sky: Option<bool>) -> Look {
    let mut look = Look::default();
    look.environments
        .insert("room".into(), EnvironmentEntry { sun_follows_sky, ..Default::default() });
    look
}

#[test]
fn sun_follows_sky_is_saved_as_an_edit_and_put_back() {
    let author = look(None);
    let edited = look(Some(false));
    let edits = LookEdits::between(&author, &edited);
    assert_eq!(edits.environments["room"].sun_follows_sky, Some(false));

    let saved = toml::to_string(&edits).unwrap();
    let loaded: LookEdits = toml::from_str(&saved).unwrap();
    let mut restored = author.clone();
    loaded.apply(&mut restored);
    assert_eq!(restored.environment("room").sun_follows_sky, Some(false));
}

#[test]
fn setting_it_back_to_the_authors_value_is_no_edit() {
    let author = look(Some(false));
    assert!(LookEdits::between(&author, &look(Some(false))).is_empty());
}
