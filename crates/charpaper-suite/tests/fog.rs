use charpaper_suite::Fog;

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
