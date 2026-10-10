use std::time::Duration;

use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;

use crate::Hush;
use crate::Volumes;
use crate::device::Device;
use crate::device::default_device_id;
use crate::envelope::Envelope;

// How often to ask which device is the default, to follow a switch made in the system's
// settings. A device that disappears is noticed sooner, through its stream's error.
const DEVICE_CHECK: Duration = Duration::from_secs(2);
const WAKE_FADE: Duration = Duration::from_secs(2);
const HUSH_FADE: Duration = Duration::from_secs(1);

#[derive(Resource, Default)]
pub(crate) struct Output {
    device: Option<Device>,
    // Counts the devices opened so far. A sound remembers the count it was started under; one
    // that is behind still plays into a device that is gone.
    pub generation: u32,
    // Up from silence on every new device, down while hushed.
    master_fade: Envelope,
    checked: Option<Duration>,
    failing: bool,
}

impl Output {
    pub fn device(&self) -> Option<&Device> {
        self.device.as_ref()
    }

    fn reopen(&mut self, hushed: bool, volume: f32) {
        let master_fade = Envelope::new(0.0);
        if !hushed {
            master_fade.fade_to(1.0, WAKE_FADE);
        }
        match Device::open_default(&master_fade) {
            Ok(device) => {
                info!("sound plays on {}", device.name);
                device.master.set_volume(volume);
                if hushed {
                    device.master.pause();
                }
                self.device = Some(device);
                self.master_fade = master_fade;
                self.generation += 1;
                self.failing = false;
            }
            Err(err) => {
                if self.device.as_ref().is_some_and(Device::is_lost) {
                    self.device = None;
                }
                if !self.failing {
                    warn!("no sound: {err}");
                    self.failing = true;
                }
            }
        }
    }
}

// On the main thread, where Bevy's own audio opens the device too: on Windows, the system's
// audio objects belong to the thread that made them.
pub(crate) fn watch_device(
    _main_thread: NonSendMarker,
    time: Res<Time<Real>>,
    hush: Res<Hush>,
    volumes: Res<Volumes>,
    mut output: ResMut<Output>,
) {
    let now = time.elapsed();
    let lost = output.device.as_ref().is_some_and(Device::is_lost);
    let due = output.checked.is_none_or(|at| now.saturating_sub(at) >= DEVICE_CHECK);
    if !lost && !due {
        return;
    }
    output.checked = Some(now);

    match &output.device {
        Some(_) if lost => warn!("the sound device stopped working"),
        Some(device) => {
            let default = default_device_id();
            if default.is_none() || default == device.id {
                return;
            }
            info!("the default sound device changed");
        }
        None => {}
    }
    output.reopen(hush.0, volumes.master());
}

pub(crate) fn follow_hush(hush: Res<Hush>, output: Res<Output>) {
    let Some(device) = output.device() else {
        return;
    };
    if hush.is_changed() {
        if hush.0 {
            output.master_fade.fade_to(0.0, HUSH_FADE);
        } else {
            device.master.play();
            output.master_fade.fade_to(1.0, WAKE_FADE);
        }
    }
    // Paused only once silent, so the fade is heard. While paused nothing moves on, and a
    // song resumes where it stopped.
    if hush.0 && output.master_fade.level() == 0.0 && !device.master.is_paused() {
        device.master.pause();
    }
}
