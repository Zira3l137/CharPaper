// Clips as Bevy assets, the timeline every stage plays them on, and when their sound cues are
// due. What mixes them is charpaper-anim; this is how the app feeds it.

use std::sync::Arc;

use bevy::app::AnimationSystems;
use bevy::asset::AssetLoader;
use bevy::asset::AssetPath;
use bevy::asset::LoadContext;
use bevy::asset::io::Reader;
use bevy::prelude::*;
use charpaper_anim::Clip;
use charpaper_anim::ClipJson;
use charpaper_anim::Event;
use charpaper_suite::ClipCue;

use crate::SceneSet;
use crate::suite::ActiveSuite;

pub(crate) struct ClipsPlugin;

impl Plugin for ClipsPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ClipAsset>()
            .register_asset_loader(ClipLoader)
            .configure_sets(
                PostUpdate,
                (Mixing::Clock, Mixing::Pose).chain().before(AnimationSystems),
            )
            .add_systems(Update, start_timeline.in_set(SceneSet::Fill))
            .add_systems(PostUpdate, advance_timelines.in_set(Mixing::Clock));
    }
}

// Each frame the timelines move on, then every actor's mixer writes its pose: before Bevy's
// own animation and gaze, which aims the head from that pose.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Mixing {
    Clock,
    Pose,
}

#[derive(Asset, TypePath)]
pub(crate) struct ClipAsset(pub Arc<Clip>);

#[derive(TypePath)]
struct ClipLoader;

#[derive(Debug, thiserror::Error)]
enum ClipLoadError {
    #[error("cannot read it: {0}")]
    Io(#[from] std::io::Error),
    #[error("cannot read the .glb beside it: {0}")]
    Glb(#[from] bevy::asset::ReadAssetBytesError),
    #[error(transparent)]
    Clip(#[from] charpaper_anim::ClipError),
}

impl AssetLoader for ClipLoader {
    type Asset = ClipAsset;
    type Settings = ();
    type Error = ClipLoadError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<ClipAsset, ClipLoadError> {
        let mut json = Vec::new();
        reader.read_to_end(&mut json).await?;
        // Only a clip that keys bones or objects has a .glb, named as its .json.
        let glb = match ClipJson::read(&json)?.nodes.is_empty() {
            true => None,
            false => {
                let path = load_context.path();
                let glb = AssetPath::from_path_buf(path.path().with_extension("glb"))
                    .with_source(path.source().clone_owned());
                Some(load_context.read_asset_bytes(glb).await?)
            }
        };
        Ok(ClipAsset(Arc::new(Clip::load(&json, glb.as_deref())?)))
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}

// Where a stage's timeline is, in seconds into its loop. Every clip in timeline mode shows
// this moment, so clips started at different times still play in step.
#[derive(Component, Default)]
pub(crate) struct Timeline {
    pub seconds: f64,
    length: f64,
}

fn start_timeline(mut stages: Query<(&ActiveSuite, &mut Timeline), Added<ActiveSuite>>) {
    for (suite, mut timeline) in &mut stages {
        *timeline = Timeline { seconds: 0.0, length: suite.timeline.seconds() };
    }
}

fn advance_timelines(time: Res<Time>, mut timelines: Query<&mut Timeline>) {
    for mut timeline in &mut timelines {
        let seconds = timeline.seconds + time.delta_secs_f64();
        timeline.seconds = if timeline.length > 0.0 { seconds % timeline.length } else { seconds };
    }
}

// The cues a clip passed on its way, as the mixer reports it.
pub(crate) fn due<'a>(cues: &'a [ClipCue], event: &Event) -> impl Iterator<Item = &'a ClipCue> {
    let event = *event;
    cues.iter().filter(move |cue| event.passes(cue.at))
}
