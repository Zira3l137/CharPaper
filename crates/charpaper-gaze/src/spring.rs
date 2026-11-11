use glam::Vec2;

// A spring that doesn't swing past its goal is within a twentieth of it after this many of
// its own time units.
const SETTLED: f32 = 4.74;

/// Eases a value to a goal that keeps moving: it sets off gently, gathers speed and slows
/// into place, without swinging past. Exact for any step, so a low frame rate changes nothing
/// about the motion.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Spring {
    pub value: Vec2,
    velocity: Vec2,
}

impl Spring {
    /// `secs` is about how long it takes to get there.
    pub fn step(&mut self, goal: Vec2, secs: f32, dt: f32) {
        if secs <= 0.0 {
            *self = Self { value: goal, velocity: Vec2::ZERO };
            return;
        }
        let rate = SETTLED / secs;
        let away = self.value - goal;
        let drift = self.velocity + away * rate;
        let fade = (-rate * dt).exp();
        self.value = goal + (away + drift * dt) * fade;
        self.velocity = (drift - (away + drift * dt) * rate) * fade;
    }
}

/// Closes a steady share of the gap to the goal: at once at first, then ever slower. For what
/// should answer without delay, like the eyes.
pub fn follow(current: Vec2, goal: Vec2, secs: f32, dt: f32) -> Vec2 {
    if secs <= 0.0 {
        return goal;
    }
    current + (goal - current) * (1.0 - (-dt / secs).exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(steps: usize, dt: f32) -> Spring {
        let mut spring = Spring::default();
        for _ in 0..steps {
            spring.step(Vec2::ONE, 0.5, dt);
        }
        spring
    }

    #[test]
    fn it_gets_there_in_about_the_time_given() {
        assert!((run(30, 1.0 / 60.0).value.x - 0.95).abs() < 0.01);
        assert!((run(240, 1.0 / 60.0).value.x - 1.0).abs() < 1e-4);
    }

    #[test]
    fn it_never_swings_past() {
        let mut spring = Spring::default();
        for _ in 0..600 {
            spring.step(Vec2::ONE, 0.5, 1.0 / 60.0);
            assert!(spring.value.x <= 1.0 + 1e-5);
        }
    }

    #[test]
    fn the_frame_rate_changes_nothing() {
        let (fast, slow) = (run(120, 1.0 / 120.0), run(15, 1.0 / 15.0));
        assert!((fast.value - slow.value).length() < 1e-4);
    }

    #[test]
    fn it_sets_off_gently() {
        let spring = run(1, 1.0 / 60.0);
        assert!(spring.value.x < 0.02);
        assert!(follow(Vec2::ZERO, Vec2::ONE, 0.06, 1.0 / 60.0).x > 0.2);
    }
}
