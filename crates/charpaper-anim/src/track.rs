use glam::Quat;
use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interpolation {
    Linear,
    Step,
}

pub trait Sample: Copy {
    fn mix(self, other: Self, t: f32) -> Self;
}

impl Sample for f32 {
    fn mix(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }
}

impl Sample for Vec3 {
    fn mix(self, other: Self, t: f32) -> Self {
        self.lerp(other, t)
    }
}

impl Sample for Quat {
    fn mix(self, other: Self, t: f32) -> Self {
        self.slerp(other, t)
    }
}

/// Keyed values over time, in seconds. Before the first key it holds the first value, after
/// the last key the last, as Blender does with constant extrapolation.
#[derive(Debug, Clone, PartialEq)]
pub struct Track<T> {
    times: Vec<f32>,
    values: Vec<T>,
    interpolation: Interpolation,
}

impl<T: Sample> Track<T> {
    /// None unless there is one value per time, at least one, with times rising.
    pub fn new(times: Vec<f32>, values: Vec<T>, interpolation: Interpolation) -> Option<Self> {
        let rising = times.windows(2).all(|pair| pair[0] < pair[1]);
        let finite = times.iter().all(|time| time.is_finite());
        (!times.is_empty() && times.len() == values.len() && rising && finite).then_some(Self {
            times,
            values,
            interpolation,
        })
    }

    /// One value per frame, the first at 0.
    pub fn frames(values: Vec<T>, fps: f32) -> Option<Self> {
        let times = (0..values.len()).map(|frame| frame as f32 / fps).collect();
        Self::new(times, values, Interpolation::Linear)
    }

    pub fn sample(&self, time: f32) -> T {
        let next = self.times.partition_point(|&key| key <= time);
        if next == 0 {
            return self.values[0];
        }
        let Some(&end) = self.times.get(next) else {
            return self.values[next - 1];
        };
        let (start, value) = (self.times[next - 1], self.values[next - 1]);
        match self.interpolation {
            Interpolation::Step => value,
            Interpolation::Linear => value.mix(self.values[next], (time - start) / (end - start)),
        }
    }

    pub fn first_time(&self) -> f32 {
        self.times[0]
    }

    pub(crate) fn shift(&mut self, by: f32) {
        self.times.iter_mut().for_each(|time| *time -= by);
    }
}
