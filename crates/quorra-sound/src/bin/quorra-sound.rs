//! The sound helper: this machine's default audio output, playing the samples a window sends.
//!
//! Not meant to be run by hand: a window's player starts it at the first sound and talks to it over
//! its standard input and output (`quorra_sound::wire`). It opens `cpal`'s default output first and
//! says the device's format, or why there is none, then plays each sound it is sent until its input
//! ends. Run without a parent it says the device's format and waits for a sound that never arrives.
//!
//! The one program in the tree that links an audio library, so that no window does (ADR 3317).
//! Each sound is one output stream, built when it is sent and dropped at a stop, at the next sound
//! and at the end; the device's configuration is read once at the opening.

#![deny(unsafe_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use quorra_sound::{Device, Format, Progress, Report};

fn main() -> std::process::ExitCode {
    if let Err(error) = keep_only_the_standard_descriptors() {
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

/// The device, its configuration, and the stream of the sound playing on it.
struct Output {
    device: cpal::Device,
    config: cpal::SupportedStreamConfig,
    stream: Option<cpal::Stream>,
}

impl Device for Output {
    fn format(&self) -> Format {
        Format {
            rate: self.config.sample_rate(),
            channels: self.config.channels(),
        }
    }

    fn start(&mut self, samples: Vec<f32>, report: Report) -> Result<(), String> {
        self.stop();
        let config = self.config.config();
        let stream = match self.config.sample_format() {
            cpal::SampleFormat::F32 => stream::<f32>(&self.device, config, samples, report),
            cpal::SampleFormat::I16 => stream::<i16>(&self.device, config, samples, report),
            cpal::SampleFormat::U16 => stream::<u16>(&self.device, config, samples, report),
            cpal::SampleFormat::I32 => stream::<i32>(&self.device, config, samples, report),
            cpal::SampleFormat::F64 => stream::<f64>(&self.device, config, samples, report),
            other => Err(format!(
                "the device takes samples as {other}, which this player does not write"
            )),
        }?;
        stream
            .play()
            .map_err(|error| format!("the stream would not start: {error}"))?;
        self.stream = Some(stream);
        Ok(())
    }

    fn stop(&mut self) {
        // Dropping the stream is what stops it: `cpal` closes the device's stream with it.
        self.stream = None;
    }
}

/// One sound's output stream, writing `samples` and then silence, calling `ended` once when the
/// last sample has been written.
fn stream<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    samples: Vec<f32>,
    mut report: Report,
) -> Result<cpal::Stream, String>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    let mut at = 0_usize;
    let mut ended = false;
    // The media clock's master is this count (ADR 3342): reported each time it has moved a
    // twentieth of a second's frames, which bounds the replies to twenty a second whatever the
    // device's period, and at the end.
    let channels = usize::from(config.channels.max(1));
    let step = u64::from(config.sample_rate / 20).max(1);
    let mut reported = 0_u64;
    let said = Arc::new(AtomicBool::new(false));
    device
        .build_output_stream::<T, _, _>(
            config,
            move |out: &mut [T], _: &cpal::OutputCallbackInfo| {
                for slot in out.iter_mut() {
                    let sample = samples.get(at).copied().unwrap_or(0.0);
                    *slot = T::from_sample_(sample);
                    at = at.saturating_add(1);
                }
                if ended {
                    return;
                }
                let finished = at >= samples.len();
                let frames = u64::try_from(
                    at.min(samples.len())
                        .checked_div(channels)
                        .unwrap_or_default(),
                )
                .unwrap_or(u64::MAX);
                if finished || frames.saturating_sub(reported) >= step {
                    reported = frames;
                    report(Progress::Played(frames));
                }
                if finished {
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

/// Closes every descriptor this process inherited past standard input, output and error (ADR 1945),
/// for `pdf-media-worker`'s reason: a descriptor leaked to this child by a library in the window is
/// an end of file somebody waits for while the helper lives.
#[cfg(target_os = "linux")]
#[expect(
    unsafe_code,
    reason = "closes by number the descriptors no object in this process owns (ADR 1945)"
)]
fn keep_only_the_standard_descriptors() -> std::io::Result<()> {
    // SAFETY: `main`'s first statement, before anything in this process has opened a descriptor
    // or made an object that owns one past the standard three, so no owner is left to use or close
    // what this closes; and `close_range` reads and writes none of this process's memory.
    let closed = unsafe { libc::close_range(3, u32::MAX, 0) };
    if closed == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

/// Nothing to close where `close_range` is not the system's call.
#[cfg(not(target_os = "linux"))]
fn keep_only_the_standard_descriptors() -> std::io::Result<()> {
    Ok(())
}
