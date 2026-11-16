use charpaper_suite::LightAnimation;
use charpaper_suite::LightSample;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

fn flicker() -> LightAnimation {
    LightAnimation {
        duration: 2.0,
        strength: Some(vec![1.0, 0.5, 0.0, 0.5]),
        color: Some(vec![[1.0, 0.5, 0.25], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0], [0.0, 0.0, 0.0]]),
    }
}

#[test]
fn samples_are_read_in_between() {
    let LightSample { strength, color } = flicker().at(0.25);
    assert!(close(strength, 0.75));
    assert!(close(color[0], 0.5) && close(color[1], 0.25) && close(color[2], 0.125));
}

#[test]
fn the_last_sample_leads_back_into_the_first_and_the_loop_repeats() {
    let animation = flicker();
    assert!(close(animation.at(1.75).strength, 0.75));
    assert!(close(animation.at(2.0).strength, 1.0));
    assert!(close(animation.at(4.25).strength, animation.at(0.25).strength));
    assert!(close(animation.at(-0.25).strength, animation.at(1.75).strength));
}

#[test]
fn missing_samples_leave_the_light_as_loaded() {
    let still = LightAnimation { duration: 1.0, strength: None, color: Some(Vec::new()) };
    assert_eq!(still.at(0.3), LightSample { strength: 1.0, color: [1.0; 3] });

    let one = LightAnimation { duration: 0.0, strength: Some(vec![0.4]), color: None };
    assert!(close(one.at(12.0).strength, 0.4));
}
