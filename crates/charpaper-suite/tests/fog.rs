use charpaper_suite::Fog;
use charpaper_suite::FogNoise;
use charpaper_suite::MAX_NOISE_DETAIL;
use charpaper_suite::NOISE_CURVE_POINTS;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

#[test]
fn a_fresh_principled_volume_scatters_and_absorbs_half() {
    let medium = Fog::default().medium();
    assert_eq!(medium.color, [1.0; 3]);
    assert!(close(medium.scattering, 0.5) && close(medium.absorption, 0.5));
    assert_eq!((medium.density, medium.anisotropy), (1.0, 0.0));
}

#[test]
fn white_fog_only_scatters_and_a_white_absorption_color_absorbs_nothing() {
    let white = Fog { color: Some([1.0; 3]), ..Default::default() }.medium();
    assert!(close(white.scattering, 1.0) && close(white.absorption, 0.0));

    let black =
        Fog { color: Some([0.0; 3]), absorption_color: Some([1.0; 3]), ..Default::default() };
    let black = black.medium();
    assert_eq!(black.color, [0.0; 3]);
    assert!(close(black.scattering, 0.0) && close(black.absorption, 0.0));
}

#[test]
fn tinted_fog_keeps_its_hue_and_full_anisotropy_is_kept_finite() {
    let fog = Fog { color: Some([0.6, 0.3, 0.0]), anisotropy: Some(1.0), ..Default::default() };
    let medium = fog.medium();
    assert!(close(medium.scattering, 0.3));
    assert!(close(medium.color[0], 2.0) && close(medium.color[1], 1.0));
    assert!(medium.anisotropy < 1.0);
}

fn noise(density: Vec<f32>) -> FogNoise {
    FogNoise {
        transform: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]],
        drift: None,
        detail: None,
        roughness: None,
        lacunarity: None,
        normalize: None,
        density,
    }
}

#[test]
fn noise_that_gives_the_same_density_everywhere_is_plain_fog() {
    let fog = Fog { density: Some(2.0), noise: Some(noise(vec![0.5, 0.5])), ..Default::default() };
    let medium = fog.medium();
    assert!(close(medium.density, 1.0));
    assert!(medium.noise.unwrap().thickness.iter().all(|&t| close(t, 1.0)));
}

#[test]
fn a_straight_ramp_averages_to_its_middle_and_keeps_its_ends() {
    let medium = Fog { noise: Some(noise(vec![0.0, 2.0])), ..Default::default() }.medium();
    assert!(close(medium.density, 1.0));
    let thickness = medium.noise.unwrap().thickness;
    assert!(close(thickness[0], 0.0) && close(thickness[NOISE_CURVE_POINTS - 1], 2.0));
}

#[test]
fn fog_only_where_the_noise_rarely_reaches_is_thin_on_average() {
    let mut curve = vec![0.0; 11];
    curve[10] = 1.0;
    let medium = Fog { noise: Some(noise(curve)), ..Default::default() }.medium();
    assert!(medium.density < 0.01);
}

#[test]
fn detail_stops_at_three_and_a_curve_of_one_point_is_no_noise() {
    let deep = FogNoise { detail: Some(15.0), ..noise(vec![1.0, 1.0]) };
    let medium = Fog { noise: Some(deep), ..Default::default() }.medium();
    assert_eq!(medium.noise.unwrap().detail, MAX_NOISE_DETAIL);

    let medium = Fog { noise: Some(noise(vec![1.0])), ..Default::default() }.medium();
    assert!(medium.noise.is_none() && close(medium.density, 1.0));
}
