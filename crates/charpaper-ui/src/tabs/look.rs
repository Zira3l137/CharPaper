use bevy::prelude::*;
use charpaper_scene::ActiveLook;
use charpaper_scene::ActiveSuite;
use charpaper_scene::CharacterState;
use charpaper_scene::Resolved;
use charpaper_suite::Environment;
use charpaper_suite::Look;

use crate::row::FocusedWorld;

// The look as the focused stage shows it: the suite's, with the viewer's edits, for the
// environment on screen. A stage without a look shows the suite's own.
pub(super) fn resolved(world: &World) -> Option<Resolved> {
    let state = world.focused::<CharacterState>()?;
    let default = Look::default();
    let look = world.focused::<ActiveLook>().map_or(&default, |look| &look.0);
    Some(Resolved::new(look, state.environment.as_deref()))
}

// The environment on screen, from the suite. None when it shows none.
pub(super) fn shown_environment(world: &World) -> Option<&Environment> {
    let name = world.focused::<CharacterState>()?.environment.as_ref()?;
    world.focused::<ActiveSuite>()?.environments.iter().find(|e| &e.name == name)
}

// Edits the focused stage's look, which the app saves with the stage. `edit` gets the values as
// they are now, and the name of the environment on screen: environment settings are kept per
// environment, so they only change while one shows.
pub(super) fn edit_look(world: &mut World, edit: impl FnOnce(&mut Look, &Resolved, Option<&str>)) {
    let Some(environment) = world.focused::<CharacterState>().map(|s| s.environment.clone()) else {
        return;
    };
    let Some(mut look) = world.focused_mut::<ActiveLook>() else {
        return;
    };
    let resolved = Resolved::new(&look.0, environment.as_deref());
    edit(&mut look.0, &resolved, environment.as_deref());
}
