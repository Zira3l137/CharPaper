use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::time::Duration;

use rodio::Decoder;
use rodio::DeviceSinkBuilder;
use rodio::DeviceSinkError;
use rodio::MixerDeviceSink;
use rodio::Player;
use rodio::Source;
use rodio::cpal;
use rodio::cpal::DeviceId;
use rodio::cpal::StreamError;
use rodio::cpal::traits::DeviceTrait;
use rodio::cpal::traits::HostTrait;
use rodio::decoder::DecoderError;
use rodio::mixer::Mixer;
use rodio::source::Zero;

use crate::envelope::Envelope;

// An opened sound card. Every sound plays into `mix`, and `mix` plays through `master`: the
// one place for the master volume, the master fade, and pausing everything at once.
pub(crate) struct Device {
    pub id: Option<DeviceId>,
    pub name: String,
    pub master: Player,
    mix: Mixer,
    lost: Arc<AtomicBool>,
    _sink: MixerDeviceSink,
}

impl Device {
    pub fn open_default(master_fade: &Envelope) -> Result<Self, DeviceSinkError> {
        let device =
            cpal::default_host().default_output_device().ok_or(DeviceSinkError::NoDevice)?;
        let id = device.id().ok();
        let name = match device.description() {
            Ok(description) => description.name().to_string(),
            Err(_) => "an unnamed device".to_string(),
        };

        // Runs on the audio thread. An underrun is a glitch the stream gets over; any other
        // error means the device is gone or must be opened again.
        let lost = Arc::new(AtomicBool::new(false));
        let flag = lost.clone();
        let mut sink = DeviceSinkBuilder::from_device(device)?
            .with_error_callback(move |err| {
                if !matches!(err, StreamError::BufferUnderrun) {
                    flag.store(true, Relaxed);
                }
            })
            .open_sink_or_fallback()?;
        sink.log_on_drop(false);

        let (channels, rate) = (sink.config().channel_count(), sink.config().sample_rate());
        let (mix, mixed) = rodio::mixer::mixer(channels, rate);
        // A mix with nothing in it ends, and the master would drop it for good. Endless
        // silence in it keeps it going.
        mix.add(Zero::new(channels, rate));
        let master = Player::connect_new(sink.mixer());
        master.append(master_fade.apply(mixed));

        Ok(Self { id, name, master, mix, lost, _sink: sink })
    }

    pub fn is_lost(&self) -> bool {
        self.lost.load(Relaxed)
    }

    // Decoded here rather than by Bevy's audio, which panics on a broken file.
    pub fn play<B>(
        &self,
        bytes: B,
        looped: bool,
        from: Duration,
        envelope: &Envelope,
    ) -> Result<Player, DecoderError>
    where
        B: AsRef<[u8]> + Send + Sync + 'static,
    {
        let len = bytes.as_ref().len() as u64;
        let decoder = Decoder::builder().with_byte_len(len).with_data(Cursor::new(bytes));
        if looped {
            return Ok(self.connect(envelope.apply(decoder.build_looped()?)));
        }
        let mut decoder = decoder.build()?;
        if !from.is_zero() {
            // Coming up short only means starting from an earlier point.
            let _ = decoder.try_seek(from);
        }
        Ok(self.connect(envelope.apply(decoder)))
    }

    fn connect(&self, source: impl Source + Send + 'static) -> Player {
        let player = Player::connect_new(&self.mix);
        player.append(source);
        player
    }
}

pub(crate) fn default_device_id() -> Option<DeviceId> {
    cpal::default_host().default_output_device()?.id().ok()
}
