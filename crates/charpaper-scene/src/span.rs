use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::camera::CameraProjection;
use bevy::camera::SubCameraView;
use bevy::camera::primitives::Frustum;
use bevy::camera::visibility::VisibilitySystems;
use bevy::math::primitives::ViewFrustum;
use bevy::prelude::*;

use crate::SceneSet;
use crate::camera::SceneCamera;
use crate::render::ScreenParts;
use crate::screens::ScreenPlace;
use crate::screens::ScreenSettings;

// Span mode: the screens show one wide view between them, as if their monitors were a single
// screen. Every camera keeps the shared pose and lens, and renders only its own monitor's part
// of the whole picture, which Bevy calls a sub camera view.
pub(crate) struct SpanPlugin;

impl Plugin for SpanPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, cut_views.in_set(SceneSet::Run)).add_systems(
            PostUpdate,
            fit_frusta
                .after(VisibilitySystems::UpdateFrusta)
                .before(VisibilitySystems::CheckVisibility),
        );
    }
}

// The whole picture is the smallest rectangle around every monitor. Each camera's part sits
// where its monitor does, so the picture runs on unbroken from one monitor to the next, and
// nothing is drawn for the corners no monitor covers.
fn cut_views(
    settings: Option<Res<ScreenSettings>>,
    screens: Query<(&ScreenParts, Option<&ScreenPlace>)>,
    mut cameras: Query<(&mut Camera, Option<&mut TemporalAntiAliasing>), With<SceneCamera>>,
) {
    let places: Vec<IRect> = screens.iter().filter_map(|(_, place)| Some(place?.0)).collect();
    let spanning = settings.is_some_and(|s| s.span) && places.len() > 1;
    let whole = places.iter().copied().reduce(|a, b| a.union(b)).filter(|_| spanning);

    for (parts, place) in &screens {
        let Ok((mut camera, taa)) = cameras.get_mut(parts.camera) else {
            continue;
        };
        let wanted = whole.zip(place).map(|(whole, place)| SubCameraView {
            full_size: whole.size().as_uvec2(),
            offset: (place.0.min - whole.min).as_vec2(),
            size: place.0.size().as_uvec2(),
        });
        if camera.sub_camera_view == wanted {
            continue;
        }
        camera.sub_camera_view = wanted;
        // Its past frames show another part of the picture.
        if let Some(mut taa) = taa {
            taa.reset = true;
        }
    }
}

// Bevy builds a camera's frustum from its projection alone, as if the camera saw the middle of
// the whole picture. A camera showing a part off to the side would then leave out whatever
// stands beyond that middle, so its frustum is built again from its own part.
fn fit_frusta(
    mut cameras: Query<(&Camera, &Projection, &GlobalTransform, &mut Frustum), With<SceneCamera>>,
) {
    for (camera, projection, transform, mut frustum) in &mut cameras {
        let Some(part) = &camera.sub_camera_view else {
            continue;
        };
        let clip_from_world =
            projection.get_clip_from_view_for_sub(part) * transform.affine().inverse();
        *frustum = Frustum(ViewFrustum::from_clip_from_world_custom_far(
            &clip_from_world,
            &transform.translation(),
            &transform.back(),
            projection.far(),
        ));
    }
}
