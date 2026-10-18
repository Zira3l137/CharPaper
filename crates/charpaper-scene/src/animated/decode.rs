// Each GIF decodes on a thread of its own. One whose frames fit in KEEP_LIMIT is decoded once
// and kept; a bigger one is decoded again on every loop, a couple of frames ahead of the one
// on screen. The thread ends once its receiver is dropped, which happens when the
// environment playing the GIF goes away.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use bevy::log::warn;
use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;
use crossbeam_channel::bounded;
use image::AnimationDecoder;
use image::ImageDecoder;
use image::codecs::gif::GifDecoder;

const KEEP_LIMIT: usize = 200 * 1024 * 1024;
const STREAM_AHEAD: usize = 2;

pub(crate) struct Frame {
    pub pixels: Vec<u8>,
    pub delay: Duration,
}

pub(crate) enum Frames {
    Kept(Vec<Frame>),
    Streamed(Receiver<Frame>),
}

pub(crate) struct Gif {
    pub width: u32,
    pub height: u32,
    pub frames: Frames,
}

pub(crate) type Opening = Receiver<Result<Gif, String>>;

pub(crate) fn open(path: PathBuf) -> Opening {
    let (opened, opening) = bounded(1);
    let failed = opened.clone();
    let spawned = thread::Builder::new()
        .name(format!("gif {}", path.file_name().unwrap_or_default().display()))
        .spawn(move || decode(&path, &opened));
    if let Err(err) = spawned {
        let _ = failed.send(Err(format!("no thread to decode it on: {err}")));
    }
    opening
}

fn decode(path: &Path, opened: &Sender<Result<Gif, String>>) {
    let read = read_frames(path).and_then(|(size, frames)| Ok((size, keep_all(frames)?)));
    let ((width, height), kept) = match read {
        Ok(read) => read,
        Err(err) => {
            let _ = opened.send(Err(format!("{}: {err}", path.display())));
            return;
        }
    };

    if let Some(kept) = kept {
        let frames = Frames::Kept(kept);
        let _ = opened.send(Ok(Gif { width, height, frames }));
        return;
    }
    // Too big to keep. What was decoded is thrown away and decoding starts over from the
    // first frame, so memory drops back at once.
    let (next, frames) = bounded(STREAM_AHEAD);
    if opened.send(Ok(Gif { width, height, frames: Frames::Streamed(frames) })).is_ok() {
        stream(path, &next);
    }
}

fn keep_all(
    frames: impl Iterator<Item = Result<Frame, String>>,
) -> Result<Option<Vec<Frame>>, String> {
    let mut kept = Vec::new();
    let mut bytes = 0;
    for frame in frames {
        let frame = frame?;
        bytes += frame.pixels.len();
        if bytes > KEEP_LIMIT {
            return Ok(None);
        }
        kept.push(frame);
    }
    if kept.is_empty() {
        return Err("has no frames".to_string());
    }
    Ok(Some(kept))
}

fn stream(path: &Path, next: &Sender<Frame>) {
    loop {
        let frames = match read_frames(path) {
            Ok((_, frames)) => frames,
            Err(err) => {
                warn!("{} stopped playing: {err}", path.display());
                return;
            }
        };
        let mut sent = 0;
        for frame in frames {
            let frame = match frame {
                Ok(frame) => frame,
                Err(err) => {
                    warn!("{} stopped playing: {err}", path.display());
                    return;
                }
            };
            if next.send(frame).is_err() {
                return;
            }
            sent += 1;
        }
        // Only if the file changed since it opened; looping over nothing would spin forever.
        if sent == 0 {
            return;
        }
    }
}

// Frames come out whole: the decoder draws each onto what the previous ones left, as GIF asks.
fn read_frames(
    path: &Path,
) -> Result<((u32, u32), impl Iterator<Item = Result<Frame, String>>), String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let decoder = GifDecoder::new(BufReader::new(file)).map_err(|e| e.to_string())?;
    let size = decoder.dimensions();
    let frames = decoder.into_frames().map(|frame| {
        let frame = frame.map_err(|e| e.to_string())?;
        let delay = shown_for(Duration::from(frame.delay()));
        Ok(Frame { pixels: frame.into_buffer().into_raw(), delay })
    });
    Ok((size, frames))
}

// As browsers do: a GIF asking for 10 ms or less, most often 0, means the default speed.
fn shown_for(delay: Duration) -> Duration {
    if delay <= Duration::from_millis(10) { Duration::from_millis(100) } else { delay }
}
