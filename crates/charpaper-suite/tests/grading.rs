use charpaper_suite::Grading;

fn balance(warmth: f32, tint: f32) -> [f32; 2] {
    Grading { warmth: Some(warmth), tint: Some(tint), ..Default::default() }.white_balance()
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

#[test]
fn no_warmth_and_no_tint_leave_white_alone() {
    let [temperature, tint] = Grading::default().white_balance();
    assert!(close(temperature, 0.0) && close(tint, 0.0), "{temperature} {tint}");
}

// Bevy's positive temperature is redder, negative tint greener: together, orange.
#[test]
fn warmth_follows_the_black_body_curve() {
    let [temperature, tint] = balance(1.0, 0.0);
    assert!(close(temperature, 0.0555) && close(tint, -0.0639), "{temperature} {tint}");
    let [temperature, tint] = balance(-1.0, 0.0);
    assert!(close(temperature, -0.0698) && close(tint, 0.0548), "{temperature} {tint}");
}

#[test]
fn tint_moves_only_y_and_both_are_kept_in_range() {
    assert_eq!(balance(0.0, 1.0)[0], balance(0.0, 0.0)[0]);
    assert!(close(balance(0.0, 1.0)[1], 0.05));
    assert_eq!(balance(5.0, -5.0), balance(1.0, -1.0));
}

#[test]
fn a_backwards_midtones_range_is_made_empty_not_inverted() {
    let grading = Grading { midtones_range: Some([0.8, 0.3]), ..Default::default() };
    assert_eq!(grading.midtones_range(), [0.8, 0.8]);
}
