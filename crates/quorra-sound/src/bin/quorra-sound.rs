//! The sound helper: this machine's default audio output, playing the samples a window sends.
//!
//! Not meant to be run by hand: a window's player starts it at the first sound and talks to it over
//! its standard input and output (`quorra_sound::wire`). It opens `cpal`'s default output first and
//! says the device's format, or why there is none, then plays each sound it is sent until its input
//! ends. Run without a parent it says the device's format and waits for a sound that never arrives.
//!
//! The one program in the tree that links an audio library, so that no window does (ADR 3317).
//! Each pass of the sound held is one output stream, built when it is ordered and dropped at a
//! pause, a stop, the next pass and the end; the device's configuration is read once at the
//! opening. A pass reads the one copy of the sound the helper holds, which may still be arriving
//! (ADR 3457).

#![forbid(unsafe_code)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use quorra_sound::{Device, Format, Pass, Progress, Report};

fn main() -> std::process::ExitCode {
    if let Err(error) = inherited_descriptors::keep_only_the_standard_descriptors() {
        eprintln!(
            "{}: cannot close the descriptors it inherited: {error}",
            quorra_sound::HELPER_PROGRAM
        );
        return std::process::ExitCode::FAILURE;
    }
    match quorra_sound::serve(std::io::stdin().lock(), std::io::stdout(), open) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}: {error}", quorra_sound::HELPER_PROGRAM);
            std::process::ExitCode::FAILURE
        }
    }
}

/// This machine's default audio output, opened.
///
/// # Errors
///
/// The sentence to say where this machine has no output device or will not describe it.
fn open() -> Result<Output, String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "this machine offers no audio output device".to_owned())?;
    let config = device
        .default_output_config()
        .map_err(|error| format!("the audio output device's configuration: {error}"))?;
    Ok(Output {
        device,
        config,
        stream: None,
    })
}

/// The device, its configuration, and the stream of the pass playing on it with its last
/// callback's timing.
struct Output {
    device: cpal::Device,
    config: cpal::SupportedStreamConfig,
    stream: Option<(cpal::Stream, Arc<Mutex<Timing>>)>,
}

/// What the device's last callback wrote and said, which a pause reads the frame the device had
/// reached from (ADR 3458).
#[derive(Clone, Copy, Debug, Default)]
struct Timing {
    /// The pass's frames written before the last callback.
    before: u64,
    /// The pass's frames that callback wrote.
    written: u64,
    /// How long after the callback the device said its first frame would be heard: `cpal`'s
    /// playback instant less its callback instant, the frames the device still held.
    delay: Duration,
    /// When the callback ran, on this process's monotonic clock; `None` before the first.
    at: Option<Instant>,
}

impl Timing {
    /// The frames of the pass the device had reached at `now`: the first frame of the last
    /// callback is heard `delay` after it ran, and the frames after it at the device's `rate` —
    /// held to what has been written, since a frame not yet written cannot have been heard.
    fn reached(&self, now: Instant, rate: u32) -> u64 {
        let Some(at) = self.at else {
            return 0;
        };
        let heard_at = at.checked_add(self.delay).unwrap_or(at);
        let written = self.before.saturating_add(self.written);
        if now >= heard_at {
            let since = frames_in(now.saturating_duration_since(heard_at), rate);
            self.before.saturating_add(since).min(written)
        } else {
            let until = frames_in(heard_at.saturating_duration_since(now), rate);
            self.before.saturating_sub(until)
        }
    }
}

/// The frames of a device of `rate` in `span`, rounded down.
fn frames_in(span: Duration, rate: u32) -> u64 {
    let frames = span.as_nanos().saturating_mul(u128::from(rate)) / 1_000_000_000;
    u64::try_from(frames).unwrap_or(u64::MAX)
}

impl Device for Output {
    fn format(&self) -> Format {
        Format {
            rate: self.config.sample_rate(),
            channels: self.config.channels(),
        }
    }

    fn start(&mut self, pass: Pass, report: Report) -> Result<(), String> {
        self.stop();
        let config = self.config.config();
        let timing = Arc::new(Mutex::new(Timing::default()));
        let shared = Arc::clone(&timing);
        let stream = match self.config.sample_format() {
            cpal::SampleFormat::F32 => stream::<f32>(&self.device, config, &pass, report, shared),
            cpal::SampleFormat::I16 => stream::<i16>(&self.device, config, &pass, report, shared),
            cpal::SampleFormat::U16 => stream::<u16>(&self.device, config, &pass, report, shared),
            cpal::SampleFormat::I32 => stream::<i32>(&self.device, config, &pass, report, shared),
            cpal::SampleFormat::F64 => stream::<f64>(&self.device, config, &pass, report, shared),
            other => Err(format!(
                "the device takes samples as {other}, which this player does not write"
            )),
        }?;
        stream
            .play()
            .map_err(|error| format!("the stream would not start: {error}"))?;
        self.stream = Some((stream, timing));
        Ok(())
    }

    fn pause(&mut self) -> Option<u64> {
        let (stream, timing) = self.stream.take()?;
        let now = Instant::now();
        // Read before the stream is dropped, and after the instant: the callback that runs between
        // the two wrote frames the device cannot yet have reached by `now`.
        let timing = *timing.lock().unwrap_or_else(PoisonError::into_inner);
        drop(stream);
        Some(timing.reached(now, self.config.sample_rate()))
    }

    fn stop(&mut self) {
        // Dropping the stream is what stops it: `cpal` closes the device's stream with it.
        self.stream = None;
    }
}

/// One pass's output stream, writing the pass's samples as they arrive — silence, counting
/// nothing, where the device has reached samples still to come — then silence once it is over,
/// calling `report` once when its last sample has been written.
fn stream<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    pass: &Pass,
    mut report: Report,
    timing: Arc<Mutex<Timing>>,
) -> Result<cpal::Stream, String>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    let channels = usize::from(config.channels.max(1));
    let from = usize::try_from(pass.from)
        .unwrap_or(usize::MAX)
        .saturating_mul(channels);
    let end = usize::try_from(pass.frames)
        .unwrap_or(usize::MAX)
        .saturating_mul(channels)
        .saturating_add(from);
    let mut reader = pass.held.reader(from, end);
    let mut buffer: Vec<f32> = Vec::new();
    let mut ended = false;
    // The media clock's master is this count (ADR 3342): reported each time it has moved a
    // twentieth of a second's frames, which bounds the replies to twenty a second whatever the
    // device's period, and at the end.
    let step = u64::from(config.sample_rate / 20).max(1);
    let mut reported = 0_u64;
    let said = Arc::new(AtomicBool::new(false));
    device
        .build_output_stream::<T, _, _>(
            config,
            move |out: &mut [T], info: &cpal::OutputCallbackInfo| {
                buffer.clear();
                buffer.resize(out.len(), 0.0);
                let read = if ended {
                    quorra_sound::held::Read {
                        copied: 0,
                        over: true,
                    }
                } else {
                    reader.read(&mut buffer)
                };
                for (slot, sample) in out.iter_mut().zip(&buffer) {
                    *slot = T::from_sample_(*sample);
                }
                let stamp = info.timestamp();
                let frames = u64::try_from(
                    reader
                        .position()
                        .saturating_sub(from)
                        .checked_div(channels)
                        .unwrap_or_default(),
                )
                .unwrap_or(u64::MAX);
                if let Ok(mut timing) = timing.lock() {
                    *timing = Timing {
                        before: timing.before.saturating_add(timing.written),
                        written: u64::try_from(read.copied.checked_div(channels).unwrap_or(0))
                            .unwrap_or(u64::MAX),
                        delay: stamp.playback.duration_since(stamp.callback),
                        at: Some(Instant::now()),
                    };
                }
                if ended {
                    return;
                }
                if read.over || frames.saturating_sub(reported) >= step {
                    reported = frames;
                    report(Progress::Played(frames));
                }
                if read.over {
                    ended = true;
                    report(Progress::Ended);
                }
            },
            // The device's own failure, mid-sound, on its own thread: said once, on the standard
            // error the helper shares with the window's terminal, rather than once per callback.
            move |error| {
                if !said.swap(true, Ordering::Relaxed) {
                    eprintln!(
                        "note: {}: the audio device failed while playing a sound: {error}",
                        quorra_sound::HELPER_PROGRAM
                    );
                }
            },
            None,
        )
        .map_err(|error| format!("no output stream: {error}"))
}
