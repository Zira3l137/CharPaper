use std::borrow::Cow;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use bevy::prelude::*;
use i18n_embed::AssetsMultiplexor;
use i18n_embed::DesktopLanguageRequester;
use i18n_embed::I18nAssets;
use i18n_embed::LanguageLoader;
use i18n_embed::fluent::FluentLanguageLoader;
use i18n_embed::unic_langid::LanguageIdentifier;

// English is compiled in and always complete. Other languages are read at startup from
// `locales/<language>/charpaper_ui.ftl` next to the executable, so a translation needs no
// rebuild, and a string it leaves out shows in English.

const ENGLISH: &str = "en-US";
const DOMAIN: &str = "charpaper_ui";
const FILE: &str = "charpaper_ui.ftl";
const ENGLISH_TEXT: &[u8] = include_bytes!("../i18n/en-US/charpaper_ui.ftl");

#[derive(Clone, Debug, Default)]
pub struct UiConfig {
    pub locales_dir: PathBuf,
    // A tag such as `de-DE`. None follows the system's languages.
    pub language: Option<String>,
}

#[derive(Resource, Deref)]
pub struct Locale(FluentLanguageLoader);

// How a fixed label gets its text. A function, not a closure capturing anything, so `fl!`
// inside it still checks the message name against English while compiling, and the text can
// be produced again when the language changes.
pub(crate) type Tr = fn(&FluentLanguageLoader) -> String;

// None for text that is not translated, like a skin's name, which stays as spawned.
#[derive(Component)]
pub(crate) struct Localized(pub Option<Tr>);

impl Locale {
    pub fn load(config: &UiConfig) -> Self {
        let loader = FluentLanguageLoader::new(DOMAIN, ENGLISH.parse().expect("a valid tag"));
        let assets = AssetsMultiplexor::new([
            Box::new(Folder(config.locales_dir.clone())) as Box<dyn I18nAssets + Send + Sync>,
            Box::new(Embedded),
        ]);

        let requested = match config.language.as_deref().map(str::parse::<LanguageIdentifier>) {
            Some(Ok(language)) => vec![language],
            Some(Err(_)) => {
                warn!("{:?} is not a language tag like en-US; using the system's", config.language);
                DesktopLanguageRequester::requested_languages()
            }
            None => DesktopLanguageRequester::requested_languages(),
        };
        if let Err(err) = i18n_embed::select(&loader, &assets, &requested) {
            warn!("cannot load translations: {err}; showing English");
            let _ = loader.load_fallback_language(&Embedded);
        }
        // Otherwise Fluent wraps every inserted value in invisible direction marks, which Bevy's
        // text shows as boxes. It must be set again after every load.
        loader.set_use_isolating(false);

        let locale = Self(loader);
        locale.report_gaps();
        locale
    }

    // For translators: which of English's strings their file lacks.
    fn report_gaps(&self) {
        let current = self.0.current_language();
        let english: LanguageIdentifier = ENGLISH.parse().expect("a valid tag");
        if current == english {
            return;
        }
        let names = |language: &LanguageIdentifier| -> BTreeSet<String> {
            self.0.with_message_iter(language, |messages| {
                messages.map(|m| m.id.name.to_string()).collect()
            })
        };
        let translated = names(&current);
        let missing: Vec<String> = names(&english).difference(&translated).cloned().collect();
        if missing.is_empty() {
            info!("showing the {current} translation");
        } else {
            info!(
                "showing the {current} translation; {} string(s) it lacks show in English: {}",
                missing.len(),
                missing.join(", ")
            );
        }
    }
}

pub(crate) fn relabel(locale: Res<Locale>, labels: Query<(Ref<Localized>, &mut Text)>) {
    for (localized, mut text) in labels {
        if let Some(tr) = localized.0
            && (locale.is_changed() || localized.is_added())
        {
            text.0 = tr(&locale);
        }
    }
}

struct Embedded;

impl I18nAssets for Embedded {
    fn get_files(&self, file_path: &str) -> Vec<Cow<'_, [u8]>> {
        match file_path == format!("{ENGLISH}/{FILE}") {
            true => vec![Cow::Borrowed(ENGLISH_TEXT)],
            false => Vec::new(),
        }
    }

    fn filenames_iter(&self) -> Box<dyn Iterator<Item = String> + '_> {
        Box::new(std::iter::once(format!("{ENGLISH}/{FILE}")))
    }
}

// The crate's own FileSystemAssets lists bare file names without their language folder, so
// it never finds a language; this lists `<language>/<file>`, which is what it looks for.
struct Folder(PathBuf);

impl I18nAssets for Folder {
    fn get_files(&self, file_path: &str) -> Vec<Cow<'_, [u8]>> {
        fs::read(self.0.join(file_path)).map(Cow::Owned).into_iter().collect()
    }

    fn filenames_iter(&self) -> Box<dyn Iterator<Item = String> + '_> {
        let languages = fs::read_dir(&self.0).into_iter().flatten().flatten();
        Box::new(languages.filter_map(|entry| {
            let language = entry.file_name().to_string_lossy().into_owned();
            self.0.join(&language).join(FILE).is_file().then(|| format!("{language}/{FILE}"))
        }))
    }
}
