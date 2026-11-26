// The camera leaning towards the cursor, swinging a few degrees around what it looks at, so
// what's in front slides one way and what's behind the other.
//
// The lean is worked out here and applied where the view gets its final transform, for the
// orbit camera and for the suite's own cameras alike. It leaves the orbit's pose alone, so
// what's remembered of it doesn't move.

use bevy::prelude::*;
use charpaper_settings::ViewSettings;

use crate::gaze::CursorAt;
use crate::gaze::CursorPosition;
use crate::gaze::seen_from;
use crate::render::ScreenParts;
use crate::screens::Screen;
use crate::screens::ScreenPlace;
use crate::stage::StageOf;

// How far it swings at the picture's edge, at a strength of 1.
const MOST_DEGREES: Vec2 = Vec2::new(3.0, 2.0);
// About how long it takes to follow the cursor, and to drift back once the mouse keeps still.
const FOLLOW_SECS: f32 = 0.35;
const RETURN_SECS: f32 = 2.0;
const STILL_SECS: f32 = 3.0;

#[derive(Component, Default)]
pub(crate) struct Sway {
    // Where it leans, as the cursor's place in the picture, -1 to 1 each way, eased.
    lean: Vec2,
    cursor: Option<CursorAt>,
    still: f32,
}

impl Sway {
    pub(crate) fn leaning(&self) -> bool {
        self.lean.length_squared() >= 1e-8
    }

    // Turns the transform around `pivot`, both in the stage's space.
    pub(crate) fn apply(&self, transform: &mut Transform, pivot: Vec3) {
        if !self.leaning() {
            return;
        }
        let angles = (self.lean * MOST_DEGREES).map(f32::to_radians);
        let turn = Quat::from_euler(EulerRot::YXZ, angles.x, -angles.y, 0.0);
        let around = transform.rotation * turn * transform.rotation.inverse();
        transform.translation = pivot + around * (transform.translation - pivot);
        transform.rotation = around * transform.rotation;
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn ease(
    time: Res<Time>,
    cursor: Res<CursorPosition>,
    buttons: Res<ButtonInput<MouseButton>>,
    settings: Query<&ViewSettings>,
    mut views: Query<(&mut Sway, &ChildOf)>,
    screens: Query<(&Screen, &ScreenParts, Option<&ScreenPlace>)>,
    windows: Query<&Window>,
    stage_of: StageOf,
) {
    let dt = time.delta_secs();
    for (mut sway, stage) in &mut views {
        let stage = stage.parent();
        let Ok(settings) = settings.get(stage) else {
            continue;
        };
        if sway.cursor == cursor.0 {
            sway.still += dt;
        } else {
            sway.cursor = cursor.0;
            sway.still = 0.0;
        }
        let toward =
            cursor.0.filter(|_| settings.parallax && sway.still < STILL_SECS).and_then(|at| {
                let (position, _, window) = seen_from(stage, at, &screens, &stage_of)?;
                let window = windows.get(window).ok()?;
                let size =
                    Vec2::new(window.physical_width() as f32, window.physical_height() as f32);
                let place = position / size.max(Vec2::ONE);
                let centered = Vec2::new(place.x * 2.0 - 1.0, 1.0 - place.y * 2.0);
                Some(centered.clamp(Vec2::NEG_ONE, Vec2::ONE) * settings.parallax_strength())
            });
        // A drag turns the orbit camera, which leaning along would fight.
        if buttons.get_pressed().next().is_some() && settings.parallax {
            continue;
        }
        let (target, secs) = match toward {
            Some(target) => (target, FOLLOW_SECS),
            None => (Vec2::ZERO, RETURN_SECS),
        };
        let lean = sway.lean;
        sway.lean = lean + (target - lean) * (1.0 - (-dt / secs).exp());
    }
}
