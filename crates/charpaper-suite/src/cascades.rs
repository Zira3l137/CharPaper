// Bevy splits a sun's shadows into cascades: bands of distance from the camera, each drawn into
// its own shadow map so near shadows get more detail. By default there are four, the first
// ending 10 m out and the last 150 m, each reaching about 2.5 times as far as the one before.
// A scene only needs as many as it takes to reach its far side.

/// Where Bevy's first sun shadow cascade ends, in meters.
pub const FIRST_CASCADE: f32 = 10.0;
/// How far Bevy's sun shadows reach by default, in meters.
pub const MAX_SHADOW_DISTANCE: f32 = 150.0;
pub const MAX_CASCADES: usize = 4;

/// How many cascades a sun needs for its shadows to reach `depth` meters in front of the
/// camera, spaced the way Bevy spaces its four.
pub fn sun_cascades(depth: f32) -> usize {
    if depth.is_nan() || depth <= FIRST_CASCADE {
        return 1;
    }
    let growth = (MAX_SHADOW_DISTANCE / FIRST_CASCADE).powf(1.0 / (MAX_CASCADES - 1) as f32);
    let further = (depth.min(MAX_SHADOW_DISTANCE) / FIRST_CASCADE).ln() / growth.ln();
    // Float noise must not add a cascade at exactly a boundary.
    (1 + (further - 1e-4).ceil() as usize).min(MAX_CASCADES)
}
