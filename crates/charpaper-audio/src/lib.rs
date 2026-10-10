mod device;
mod envelope;
mod output;
mod voice;

use std::time::Duration;

use bevy::prelude::*;
pub use bevy_audio::AudioSource;
use serde::Deserialize;
use serde::Serialize;

// Sound flows from each sound's player into one mix, from the mix through the master player,
// and from there to the sound card, which is reopened whenever the device changes. Bevy's
// audio plugin is left out because it opens the sound card once and never again; only its
// file loader is used.
pub struct SoundPlugin {
    pub volumes: Volumes,
}

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<AudioSource>()
            .init_asset_loader::<bevy_audio::AudioLoader>()
            .insert_resource(self.volumes.clone())
            .init_resource::<Hush>()
            .init_resource::<output::Output>()
            .add_systems(
                PostUpdate,
                (
                    output::watch_device,
                    output::follow_hush,
                    voice::move_to_new_device,
                    voice::start_sounds,
                    voice::apply_volumes.run_if(resource_changed::<Volumes>),
                    voice::fade_out,
                    voice::despawn_finished,
                )
                    .chain(),
            );
    }
}

// Spawning an entity with this plays a sound, as soon as its file has loaded. A sound that
// doesn't loop despawns its entity when it ends. Despawning the entity stops the sound at
// once; inserting FadeOut fades it out first.
#[derive(Component, Clone, Debug)]
pub struct Sound {
    pub source: Handle<AudioSource>,
    pub layer: Layer,
    pub looped: bool,
    pub fade_in: Duration,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct FadeOut(pub Duration);

// Sounds are grouped by what they are, and each group has its own volume.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    Music,
    Ambience,
}

// From 0 (silent) to 1 (as loud as the file).
#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Volumes {
    pub master: f32,
    pub music: f32,
    pub ambience: f32,
}

// Quiet at first: a wallpaper that suddenly plays loud sound is unwelcome.
impl Default for Volumes {
    fn default() -> Self {
        Self { master: 0.25, music: 1.0, ambience: 1.0 }
    }
}

impl Volumes {
    fn master(&self) -> f32 {
        self.master.clamp(0.0, 1.0)
    }

    fn of(&self, layer: Layer) -> f32 {
        let volume = match layer {
            Layer::Music => self.music,
            Layer::Ambience => self.ambience,
        };
        volume.clamp(0.0, 1.0)
    }
}

// While true, all sound fades out and pauses. Once false again, it resumes where it stopped
// and fades back in.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hush(pub bool);

// Triggered on a sound's entity when its file won't load or play, right before the entity
// is despawned.
#[derive(EntityEvent, Debug, Clone, Copy)]
pub struct SoundFailed {
    pub entity: Entity,
}
