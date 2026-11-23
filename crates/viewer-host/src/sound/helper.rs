//! The [`Sink`] a window plays through: the sound helper program, `quorra-sound`, started at the
//! first sound and handed each converted sound over a pipe (ADR 3317).
//!
//! The window links no audio library. `cpal`, and the `libasound` its ALSA back end links at load
//! time, are in the helper alone, so a window maps nothing for sound when it starts and starts on a
//! machine that has no audio library — where the first sound is then said not to have been played,
//! and the window goes on. Every toolkit's window takes the same sink: it is [`open`], an
//! [`super::OpenSink`] as it stands.

use std::path::Path;

use quorra_sound::{Format, Helper};

use super::{DeviceFormat, MAX_PLAYED_SAMPLES, Sink};

// The helper reads at most its own bound per sound, and the window sends at most what it converts.
const _: () = assert!(quorra_sound::wire::MAX_SAMPLES >= MAX_PLAYED_SAMPLES);

/// The helper beside this executable, or the one `QUORRA_SOUND` names, started and its device
/// opened.
///
/// # Errors
///
/// The sentence to say where the helper is missing, could not be loaded, or could not open the
/// device.
pub fn open() -> Result<Box<dyn Sink>, String> {
    open_at(&located()?)
}

/// The sentence to say where this window has no sound helper to play through, or `None` where it
/// has one: the question a window asks once, at its launch, before it tells the core whether it
/// plays a sound here (ADR 3417).
///
/// A look for one file — the environment variable, then the places beside this executable — and
/// no process started, so the helper is still started only by the first sound.
#[must_use]
pub fn missing() -> Option<String> {
    located().err()
}

/// Where the helper is: the program `QUORRA_SOUND` names, or the one beside this executable.
fn located() -> Result<std::path::PathBuf, String> {
    confined_transport::program_beside_executable(
        quorra_sound::HELPER_PROGRAM,
        quorra_sound::HELPER_PATH_VARIABLE,
    )
    .map_err(|missing| {
        format!(
            "the sound helper `{}` was {missing}; build it with `cargo build -p quorra-sound`, or \
             name it in {}",
            quorra_sound::HELPER_PROGRAM,
            quorra_sound::HELPER_PATH_VARIABLE
        )
    })
}

/// The helper `program`, started and its device opened.
///
/// # Errors
///
/// As [`open`].
pub fn open_at(program: &Path) -> Result<Box<dyn Sink>, String> {
    Helper::spawn(program)
        .map(|helper| Box::new(HelperSink(helper)) as Box<dyn Sink>)
        .map_err(|error| error.to_string())
}

/// The running helper, as a sink.
struct HelperSink(Helper);

impl Sink for HelperSink {
    fn format(&self) -> DeviceFormat {
        let Format { rate, channels } = self.0.format();
        DeviceFormat { rate, channels }
    }

    fn load(&mut self) -> Result<(), String> {
        self.0.load().map_err(|error| error.to_string())
    }

    fn append(&mut self, samples: &[f32]) -> Result<(), String> {
        self.0.append(samples).map_err(|error| error.to_string())
    }

    fn complete(&mut self) -> Result<(), String> {
        self.0.complete().map_err(|error| error.to_string())
    }

    fn start(&mut self, from: u64, frames: u64, report: super::Report) -> Result<(), String> {
        self.0
            .play(from, frames, report)
            .map_err(|error| error.to_string())
    }

    // A helper that does not answer leaves the resumption where the reports put it, as a device
    // that cannot say does.
    fn pause(&mut self) -> Option<u64> {
        self.0.pause().ok().flatten()
    }

    fn stop(&mut self) {
        self.0.stop();
    }
}
