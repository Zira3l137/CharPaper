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

    pub fn get_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.strings.get(key).map_or(default, String::as_str)
    }
}

#[derive(Clone, Resource, Default, Debug)]
pub struct UiConfig {
    pub locale: UiLocale,
}
