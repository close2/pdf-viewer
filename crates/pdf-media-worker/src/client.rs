//! The host's half: a sound's bytes split into requests, each answered by the worker under its own
//! processor-time budget, and the worker started only when the first sound is played.

use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

use confined_transport::{Canceller, Host, TransportError};
use pdf_sandbox::{Allowance, Ended};

use crate::sound::{Decoded, SoundFormat};
use crate::wire::{
    FRAME_DECODE, FRAME_REFUSED, FRAME_SAMPLES, MAGIC, MAX_RUN_BYTES, decode_samples,
    encode_request, sentence,
};

/// Processor time one request may spend in the worker before it is killed (ADR 3133's budget,
/// read from outside the worker).
///
/// A request is at most [`MAX_RUN_BYTES`]; its worst case, a run of one-bit samples decoded to two
/// million words and written back, was measured at 30 to 40 milliseconds of processor in a debug
/// build (ADR 3297). One second is twenty-five times that, and short enough that a sound stopped
/// for it is said before a person wonders; the wall clock behind it is ten times as long, the
/// machine's ceiling.
pub const PROCESSOR_BUDGET: Duration = Duration::from_secs(1);

/// Why a sound was not decoded.
#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    /// Table 305's entries describe no samples this program decodes; the sentence says which.
    #[error("{0}")]
    Format(String),
    /// The decoded samples would pass the bound the caller gave.
    #[error("the sound decodes to {samples} samples, past the {limit} this player holds")]
    TooLarge {
        /// The samples the sound's bytes hold.
        samples: u64,
        /// The caller's bound.
        limit: usize,
    },
    /// The worker program could not be found or started.
    #[error("the media worker could not be started: {0}")]
    Start(String),
    /// The worker spent its processor budget on one request and was killed.
    #[error("the media worker spent its processor budget on one request and was stopped")]
    Spent,
    /// The worker neither answered nor spent its budget before the wall-clock ceiling — a bound
    /// the machine's load decides as much as the sound does (ADR 3133).
    #[error("the media worker did not answer within {after:?}, and was stopped")]
    Stalled {
        /// The wall-clock length of the wait.
        after: Duration,
    },
    /// The worker refused a request, or answered one with what its request could not hold.
    #[error("the media worker refused the sound: {0}")]
    Refused(String),
    /// The wire or the worker failed.
    #[error("{0}")]
    Transport(#[from] TransportError),
}

/// A sound decoded: §13.3's samples in one format, at the rate and channels the object states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sound {
    /// `/R`, in samples per second per channel.
    pub rate: u32,
    /// `/C`.
    pub channels: u16,
    /// The samples of every whole frame, interleaved, each in the most significant bits of an
    /// `i32` (`crate::sound`).
    pub samples: Vec<i32>,
    /// What the decode did not carry of the stream's data, each said.
    pub notes: Vec<String>,
}

/// The host's handle on a media worker, which it starts at the first sound it is asked to decode.
///
/// Constructing one starts nothing (RFC 0015 section 4.6): a document whose sounds are never
/// played, or a reader whose level does not play them here, never starts the worker. A worker that
/// dies or is killed is dropped, and the next sound starts another.
#[derive(Debug, Default)]
pub struct MediaWorker {
    /// The worker program, where the caller names it; otherwise found beside this executable or
    /// in [`crate::WORKER_PATH_VARIABLE`].
    program: Option<PathBuf>,
    /// The running worker, from the first sound on.
    host: Option<Host>,
    /// How many workers this handle has started.
    starts: usize,
}

impl MediaWorker {
    /// A handle that will start the worker found beside this executable, or the one
    /// [`crate::WORKER_PATH_VARIABLE`] names. Starts nothing now.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A handle that will start `program`. Starts nothing now.
    #[must_use]
    pub fn with_program(program: PathBuf) -> Self {
        Self {
            program: Some(program),
            ..Self::default()
        }
    }

    /// How many workers this handle has started.
    #[must_use]
    pub fn starts(&self) -> usize {
        self.starts
    }

    /// Decodes a sound object's data — its bytes after Table 5's filters — under `format`, into at
    /// most `max_samples` samples.
    ///
    /// The data is cut into requests of whole frames ([`SoundFormat::whole_frames_unit`]), so that
    /// no sample is split between two, and each is answered under [`PROCESSOR_BUDGET`].
    ///
    /// # Errors
    ///
    /// [`MediaError`]: the format refused, the bound passed, the worker not started, killed for its
    /// budget, stalled, refusing, or answering what its request could not hold.
    pub fn decode(
        &mut self,
        format: &SoundFormat,
        data: &[u8],
        max_samples: usize,
    ) -> Result<Sound, MediaError> {
        if let Some(refusal) = format.refusal() {
            return Err(MediaError::Format(refusal));
        }
        let (samples, unframed) = crate::sound::sample_counts(format, data.len());
        if usize::try_from(samples).map_or(true, |samples| samples > max_samples) {
            return Err(MediaError::TooLarge {
                samples,
                limit: max_samples,
            });
        }
        let unit = usize::try_from(format.whole_frames_unit()).unwrap_or(usize::MAX);
        // At most `MAX_RUN_BYTES`, cut on a frame: a unit is at most 65,535 channels of 32 bits,
        // 262,140 bytes, under the run's 262,144, so one unit always fits.
        let run_len = MAX_RUN_BYTES
            .checked_div(unit)
            .unwrap_or(1)
            .max(1)
            .saturating_mul(unit)
            .max(1);
        let mut decoded: Vec<i32> = Vec::new();
        for run in data.chunks(run_len) {
            let part = self.request(*format, run)?;
            decoded.extend_from_slice(&part.samples);
        }
        let mut notes = Vec::new();
        if let Some(shortfall) = self
            .host
            .as_ref()
            .and_then(|host| host.confinement().shortfall())
        {
            notes.push(format!("the media worker ran {shortfall}"));
        }
        if unframed != 0 {
            notes.push(format!(
                "the last {unframed} sample(s) of the sound object complete no frame of {} \
                 channels (§13.3's interleaved order) and were not played",
                format.channels
            ));
        }
        Ok(Sound {
            rate: format.rate,
            channels: format.channels,
            samples: decoded,
            notes,
        })
    }

    /// One request and its checked reply, starting a worker where none runs.
    fn request(&mut self, format: SoundFormat, run: &[u8]) -> Result<Decoded, MediaError> {
        let program = self.program()?;
        let host = if let Some(host) = self.host.take() {
            host
        } else {
            let host = Host::start(&program, MAGIC, &Canceller::new())?;
            self.starts = self.starts.saturating_add(1);
            host
        };
        let host = self.host.insert(host);
        let canceller = host.canceller();
        let allowance = canceller
            .process_id()
            .map(|pid| Allowance::start(pid, PROCESSOR_BUDGET));
        let payload = encode_request(&format, run);
        let (exchanged, ended) = match allowance {
            Some(allowance) => within(&allowance, &canceller, || {
                host.exchange(FRAME_DECODE, &payload, None)
            })?,
            None => (host.exchange(FRAME_DECODE, &payload, None), None),
        };
        if let Some(ended) = ended {
            self.host = None;
            canceller.reap();
            return Err(match ended {
                Ended::Spent => MediaError::Spent,
                Ended::Ceiling { after } => MediaError::Stalled { after },
            });
        }
        let reply = match exchanged {
            Ok(reply) => reply,
            Err(error) => {
                self.host = None;
                return Err(error.into());
            }
        };
        match reply {
            (FRAME_SAMPLES, bytes) => {
                decode_samples(&bytes, &format, run.len()).map_err(|wrong| {
                    // A worker that answers what its request cannot hold is not believed again.
                    self.host = None;
                    canceller.cancel();
                    MediaError::Refused(wrong)
                })
            }
            (FRAME_REFUSED, bytes) => Err(MediaError::Refused(sentence(&bytes))),
            (kind, _) => {
                self.host = None;
                canceller.cancel();
                Err(MediaError::Refused(format!(
                    "a reply of kind {kind}, which no request is answered with"
                )))
            }
        }
    }

    /// The worker program.
    fn program(&self) -> Result<PathBuf, MediaError> {
        if let Some(program) = &self.program {
            return Ok(program.clone());
        }
        confined_transport::program_beside_executable(
            crate::WORKER_PROGRAM,
            crate::WORKER_PATH_VARIABLE,
        )
        .map_err(|missing| {
            MediaError::Start(format!(
                "`{}` was {missing}; build it with `cargo build -p pdf-media-worker --bins`, or \
                 name it in {}",
                crate::WORKER_PROGRAM,
                crate::WORKER_PATH_VARIABLE
            ))
        })
    }
}

/// Runs `work` while a watchdog reads the worker's processor time against `allowance`, cancelling
/// the worker when a bound ends the request; answers what `work` returned and the bound that ended
/// it, if one did.
///
/// # Errors
///
/// [`MediaError::Start`] where no watchdog could be armed, in which case `work` is not run: a
/// request without its budget is a request nothing bounds.
fn within<T>(
    allowance: &Allowance,
    canceller: &Canceller,
    work: impl FnOnce() -> T,
) -> Result<(T, Option<Ended>), MediaError> {
    let (finished, waiting) = mpsc::channel::<()>();
    std::thread::scope(|scope| {
        let watchdog = std::thread::Builder::new()
            .name("media budget".to_owned())
            .spawn_scoped(scope, move || {
                loop {
                    let wait = match allowance.next_wait() {
                        Ok(wait) => wait,
                        Err(ended) => {
                            canceller.cancel();
                            return Some(ended);
                        }
                    };
                    match waiting.recv_timeout(wait) {
                        Err(RecvTimeoutError::Timeout) => {}
                        Ok(()) | Err(RecvTimeoutError::Disconnected) => return None,
                    }
                }
            })
            .map_err(|error| MediaError::Start(format!("no budget could be armed: {error}")))?;
        let out = work();
        drop(finished);
        let ended = watchdog.join().unwrap_or(None);
        Ok((out, ended))
    })
}
