// What the app asks of any backend. Each backend takes options of its own when it is made.
#[derive(Clone, Debug)]
pub struct WallpaperConfig {
    pub enabled: bool,
    pub dry_run: bool,
    pub forward_input: bool,
}

impl WallpaperConfig {
    // Whether windows go behind the desktop icons at all, rather than staying ordinary ones.
    pub fn attaches(&self) -> bool {
        self.enabled && !self.dry_run
    }
}

impl Default for WallpaperConfig {
    fn default() -> Self {
        Self { enabled: true, dry_run: false, forward_input: true }
    }
}
