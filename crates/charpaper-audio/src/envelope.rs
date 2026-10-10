use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::Relaxed;
use std::time::Duration;

use rodio::Source;

// Rodio's own player controls use the same period.
const TICK: Duration = Duration::from_millis(5);

// A level from silent (0) to full (1), shared by the app and the audio thread. The app sets
// where it should go and how fast; the audio thread moves it there a step every tick. Fades
// stay smooth that way even while the app wakes only once a second.
#[derive(Clone)]
pub(crate) struct Envelope(Arc<Levels>);

// f32 bits, since there is no atomic float.
struct Levels {
    now: AtomicU32,
    target: AtomicU32,
    per_second: AtomicU32,
}

impl Envelope {
    pub fn new(level: f32) -> Self {
        Self(Arc::new(Levels {
            now: AtomicU32::new(level.to_bits()),
            target: AtomicU32::new(level.to_bits()),
            per_second: AtomicU32::new(f32::INFINITY.to_bits()),
        }))
    }

    pub fn fading_in(over: Duration) -> Self {
        let envelope = Self::new(if over.is_zero() { 1.0 } else { 0.0 });
        envelope.fade_to(1.0, over);
        envelope
    }

    // `over` is how long a fade across the whole range takes.
    pub fn fade_to(&self, target: f32, over: Duration) {
        let per_second = if over.is_zero() { f32::INFINITY } else { over.as_secs_f32().recip() };
        self.0.per_second.store(per_second.to_bits(), Relaxed);
        self.0.target.store(target.to_bits(), Relaxed);
    }

    pub fn level(&self) -> f32 {
        f32::from_bits(self.0.now.load(Relaxed))
    }

    pub fn apply<S>(&self, source: S) -> impl Source + Send + 'static
    where
        S: Source + Send + 'static,
    {
        let envelope = self.clone();
        source
            .amplify(gain(self.level()))
            .periodic_access(TICK, move |amplified| amplified.set_factor(envelope.step()))
    }

    fn step(&self) -> f32 {
        let now = self.level();
        let target = f32::from_bits(self.0.target.load(Relaxed));
        let step = f32::from_bits(self.0.per_second.load(Relaxed)) * TICK.as_secs_f32();
        let next = if now < target { (now + step).min(target) } else { (now - step).max(target) };
        self.0.now.store(next.to_bits(), Relaxed);
        gain(next)
    }
}

impl Default for Envelope {
    fn default() -> Self {
        Self::new(0.0)
    }
}

// Squared, so a fade sounds even. Ears hear loudness on a curve: a straight line stays loud
// most of the way and then drops off all at once.
fn gain(level: f32) -> f32 {
    level * level
}
