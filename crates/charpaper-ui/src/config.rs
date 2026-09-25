use std::collections::HashMap;

use bevy::prelude::*;

#[derive(Clone, Resource, Default, Debug)]
pub struct UiLocale {
    strings: HashMap<String, String>,
}

impl UiLocale {
    pub fn new(strings: HashMap<String, String>) -> Self {
        Self { strings }
    }

    pub fn get<'a>(&'a self, key: &'a str) -> &'a str {
        self.strings.get(key).map(|s| s.as_str()).unwrap_or(key)
    }

    /// Like [`Self::get`], but falls back to `default` rather than the key, so
    /// a missing translation still reads as English instead of `tab.scene`.
    pub fn get_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.strings.get(key).map_or(default, String::as_str)
    }

    pub fn get_owned(&self, key: &str) -> String {
        self.strings.get(key).map(|s| s.to_owned()).unwrap_or(key.to_owned())
    }
}

#[derive(Clone, Resource, Default, Debug)]
pub struct UiConfig {
    pub locale: UiLocale,
}
