//! The audio device this window plays §13.3's sounds on: `cpal`'s default output, behind
//! `viewer_host::sound::Sink` (RFC 0015 section 5, ADR 3301).
//!
//! [`open`] is the player's `OpenSink`, called on its thread at the first sound and never before:
//! that is where the device is probed, so a document that plays nothing never touches it
//! (`CLAUDE.md`'s startup rules). Each sound is one output stream, built at [`Sink::start`] and
//! dropped at [`Sink::stop`] or at the next sound; the device's own configuration — its rate, its
//! channels and its sample format — is read once at the opening and the samples are converted to
//! it before they arrive here (`viewer_host::sound::converted`).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use viewer_host::sound::{DeviceFormat, Sink};

/// This machine's default audio output, opened.
///
/// # Errors
///
/// The sentence to say where this machine has no output device or will not describe it.
pub(crate) fn open() -> Result<Box<dyn Sink>, String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "this machine offers no audio output device".to_owned())?;
    let config = device
        .default_output_config()
        .map_err(|error| format!("the audio output device's configuration: {error}"))?;
    Ok(Box::new(Output {
        device,
        config,
        stream: None,
    }))
}

/// The device, its configuration, and the stream of the sound playing on it.
struct Output {
    device: cpal::Device,
    config: cpal::SupportedStreamConfig,
    stream: Option<cpal::Stream>,
}

impl Sink for Output {
    fn format(&self) -> DeviceFormat {
        DeviceFormat {
            rate: self.config.sample_rate(),
            channels: self.config.channels(),
        }
    }

    fn start(&mut self, samples: Vec<f32>, ended: Box<dyn FnOnce() + Send>) -> Result<(), String> {
        self.stop();
        let config = self.config.config();
        let stream = match self.config.sample_format() {
            cpal::SampleFormat::F32 => stream::<f32>(&self.device, config, samples, ended),
            cpal::SampleFormat::I16 => stream::<i16>(&self.device, config, samples, ended),
            cpal::SampleFormat::U16 => stream::<u16>(&self.device, config, samples, ended),
            cpal::SampleFormat::I32 => stream::<i32>(&self.device, config, samples, ended),
            cpal::SampleFormat::F64 => stream::<f64>(&self.device, config, samples, ended),
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
    ended: Box<dyn FnOnce() + Send>,
) -> Result<cpal::Stream, String>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    let mut at = 0_usize;
    let mut ended = Some(ended);
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
                if at >= samples.len()
                    && let Some(ended) = ended.take()
                {
                    ended();
                }
            },
            // The device's own failure, mid-sound, on its own thread: said once on the terminal,
            // where every other note of this window goes, rather than once per callback.
            move |error| {
                if !said.swap(true, Ordering::Relaxed) {
                    println!("note: the audio device failed while playing a sound: {error}");
                }
            },
            None,
        )
        .map_err(|error| format!("no output stream: {error}"))
}
