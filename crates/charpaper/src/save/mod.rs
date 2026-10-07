mod look;
mod state;

use std::time::Duration;

pub use look::LookFilePlugin;
pub use state::STATE_FILE;
pub use state::StatePlugin;
pub use state::load;

// How long changes must stop before they are written, so stepping a setting with the wheel
// writes its file once rather than on every step.
const SETTLE: Duration = Duration::from_secs(1);

// A change not on disk yet, and when it was made.
struct Unsaved<T> {
    value: T,
    changed_at: Duration,
}

impl<T> Unsaved<T> {
    fn new(value: T, now: Duration) -> Self {
        Self { value, changed_at: now }
    }

    // Quitting writes at once: the app is gone before the settle time is up.
    fn is_due(&self, now: Duration, quitting: bool) -> bool {
        quitting || now.saturating_sub(self.changed_at) >= SETTLE
    }
}
