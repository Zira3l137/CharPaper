use bevy::prelude::*;

// Where updating the app stands, for the panel. Not in state.toml: each run checks anew.
#[derive(Resource, Default, Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    // Nothing newer found, not yet checked, or not checking at all.
    #[default]
    Idle,
    Available {
        version: String,
    },
    // Bytes so far and in all. The total is 0 until the download has started.
    Downloading {
        version: String,
        done: u64,
        total: u64,
    },
    // The download or its checksum failed, and can be tried again.
    Failed {
        version: String,
        reason: String,
    },
    // Downloaded; the app is quitting to install it and start again.
    Installing {
        version: String,
    },
}

// What changed in each version since the running one, up to the one on offer, newest first.
#[derive(Resource, Default, Debug, Clone, PartialEq, Eq)]
pub struct Changelog(pub Vec<VersionNotes>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionNotes {
    pub version: String,
    // Each section's title, in English as the release notes have it, and its changes.
    pub sections: Vec<(String, Vec<String>)>,
}

// What the panel asks of the updater, about the version it offers.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateAction {
    // Download it, then restart into it. Also tries a failed download again.
    Install,
    // Stop offering it. A newer version is offered again.
    Skip,
    // Its page on GitHub, with what changed.
    OpenPage,
}
