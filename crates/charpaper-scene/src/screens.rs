use bevy::prelude::*;
use charpaper_settings::StageId;

// Where the scene follows the screens: stages start and stop, and each screen gets the cameras
// of the stage it shows. In PreUpdate, with the app describing the screens before it and the
// panels following after, so that by Update whatever went away is gone. A system there can't
// pick up a screen, camera or panel that is despawned while its commands wait.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScreenSync;

// A window a stage is shown in. The app spawns one per window; the scene gives each a camera
// of its own, at that window's size, looking from the view of the stage it shows. Despawning
// the entity takes all of that away again.
#[derive(Component, Debug, Clone, Copy)]
pub struct Screen {
    pub window: Entity,
}

// The stage a Screen shows. The app sets it from the screen's monitor.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowsStage(pub StageId);

// Where a Screen's window sits on the desktop, in physical pixels. The app keeps it current;
// span mode cuts the one wide picture up by it.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ScreenPlace(pub IRect);

// Nobody can see this Screen now, so its camera rests while the others go on.
#[derive(Component, Debug)]
pub struct Unseen;

// The camera a Screen's UI draws through, added once the scene has made it.
#[derive(Component, Debug, Clone, Copy)]
pub struct ScreenUi(pub Entity);
