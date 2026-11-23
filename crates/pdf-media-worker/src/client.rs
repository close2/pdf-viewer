//! The host's half: a sound's bytes split into requests, each answered by the worker under its own
//! processor-time budget, and the worker started only when the first sound is played.

use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

use confined_transport::{Canceller, Host, TransportError};
use pdf_sandbox::{Allowance, Ended};

use crate::aac::MAX_FRAMES_PER_UNIT;
use crate::mp4::{TOP_HEADER_BYTES, rescaled};
use crate::sound::{Decoded, SoundFormat};
use crate::wave::Found;
use crate::wire::{
    FRAME_BOX, FRAME_BOXED, FRAME_DECODE, FRAME_LAYOUT, FRAME_MOVIE, FRAME_ONWARD, FRAME_PCM,
    FRAME_PLACED, FRAME_REFUSED, FRAME_SAMPLES, FRAME_TRACK, FRAME_UNITS, MAGIC, MAX_MOVIE_BYTES,
    MAX_REQUEST_BYTES, MAX_RUN_BYTES, MAX_SAMPLES_PER_REQUEST, MAX_UNITS_PER_RUN, Track,
    decode_boxed, decode_onward, decode_pcm, decode_placed, decode_samples, decode_track,
    encode_box_request, encode_layout_request, encode_movie_request, encode_request, encode_units,
    sentence,
};

/// The most boxes at the top of an MP4 file the host walks to find its movie.
///
/// A file this player reads is a handful — `ftyp`, `moov`, `mdat`, a `free` or two; a fragmented
/// file has one pair a fragment, and is declined anyway once its movie is read. Each step is one
/// request, so the bound keeps a hostile file's walk to a second or so.
pub const MAX_TOP_LEVEL_BOXES: usize = 1024;

/// The most windows of a RIFF WAVE file the host shows the worker to find its samples (ADR 3397).
///
/// A file's `fmt ` and `data` chunks follow a handful of others, and one more window is spent only
/// on a chunk that ends past the last; the bound keeps a hostile file's walk, one request a
/// window, to a second or so, as [`MAX_TOP_LEVEL_BOXES`] does an MP4 file's.
pub const MAX_WAVE_WINDOWS: usize = 1024;

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
    /// The speaker each channel sounds on, as a `WAVE_FORMAT_EXTENSIBLE` channel mask: the
    /// channels in frame order take the mask's set bits in ascending order, and `0` names no
    /// speaker, which is every sound object's (Table 305's `/C` is a count). A WAVE file's
    /// `dwChannelMask`, or the order the AAC decoder states for its channel configuration (ADR
    /// 3405); the host's conversion places the channels by it.
    pub speakers: u32,
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
            speakers: 0,
            notes,
        })
    }

    /// Decodes a RIFF WAVE file — a rendition's media clip (ADR 3349) — into at most
    /// `max_samples` samples.
    ///
    /// The worker reads the file's chunks a window of [`MAX_RUN_BYTES`] at a time and answers where
    /// the samples are and their format, or where the next window begins (ADR 3397); this half
    /// checks every answer against `file` and sends the samples as [`Self::decode`] sends a sound
    /// object's, so that no byte of the header is read here.
    ///
    /// # Errors
    ///
    /// [`Self::decode`]'s, the worker's refusal of the header, and a walk past
    /// [`MAX_WAVE_WINDOWS`].
    pub fn decode_wave(&mut self, file: &[u8], max_samples: usize) -> Result<Sound, MediaError> {
        let total = u64::try_from(file.len()).unwrap_or(u64::MAX);
        let mut at = 0u64;
        let mut described = None;
        let mut layout = None;
        for _ in 0..MAX_WAVE_WINDOWS {
            let start = usize::try_from(at).unwrap_or(usize::MAX);
            let window = file.get(start..).unwrap_or_default();
            let window = window.get(..MAX_RUN_BYTES).unwrap_or(window);
            let payload = encode_layout_request(total, at, described.as_ref(), window);
            let found = match self.exchange(FRAME_LAYOUT, &payload)? {
                (FRAME_PLACED, bytes) => decode_placed(&bytes, total).map(Found::Placed),
                (FRAME_ONWARD, bytes) => decode_onward(&bytes, at, total),
                (FRAME_REFUSED, bytes) => return Err(MediaError::Refused(sentence(&bytes))),
                (kind, _) => Err(format!(
                    "a reply of kind {kind}, which no layout request is answered with"
                )),
            }
            .map_err(|wrong| {
                // A worker that places the samples outside the file, or walks backwards, is not
                // believed again.
                self.distrust();
                MediaError::Refused(wrong)
            })?;
            match found {
                Found::Placed(placed) => {
                    layout = Some(placed);
                    break;
                }
                Found::Onward {
                    at: next,
                    described: read,
                } => {
                    at = next;
                    described = read;
                }
            }
        }
        let Some(layout) = layout else {
            return Err(MediaError::Refused(format!(
                "the file's data chunk is not found within {MAX_WAVE_WINDOWS} windows of its \
                 chunks"
            )));
        };
        // `decode_placed` held the offset and the length inside the file.
        let start = usize::try_from(layout.offset).unwrap_or(usize::MAX);
        let end =
            usize::try_from(layout.offset.saturating_add(layout.length)).unwrap_or(usize::MAX);
        let samples = file.get(start..end).unwrap_or_default();
        let format = layout.described.format;
        let mut sound = self.decode(&format, samples, max_samples)?;
        sound.speakers = layout.described.channel_mask;
        Ok(sound)
    }

    /// Decodes an MP4 file's AAC sound track — a rendition's media clip (ADR 3381) — into at most
    /// `max_samples` samples, with the track's edit list applied.
    ///
    /// The file is longer than a request, so no request carries it: the worker reads each box at
    /// the top of the file from its first bytes and says where it ends, until the movie box is
    /// found; the movie crosses whole and the worker answers where every access unit is and what
    /// the edit list presents; then the units cross in runs, in order, to the decoder the worker
    /// holds for the clip. This half reads no byte of the file — it cuts it where the answers say,
    /// each answer checked against the file's length and against the request it answers.
    ///
    /// # Errors
    ///
    /// [`Self::decode`]'s; the worker's refusal of the file, its track or its stream; a file whose
    /// movie is absent, doubled or past [`MAX_MOVIE_BYTES`]; and a clip that decodes to nothing.
    pub fn decode_mp4(&mut self, file: &[u8], max_samples: usize) -> Result<Sound, MediaError> {
        let total = u64::try_from(file.len()).unwrap_or(u64::MAX);
        let movie = self.movie(file, total)?;
        let track = match self.exchange(FRAME_MOVIE, &encode_movie_request(total, movie))? {
            (FRAME_TRACK, bytes) => decode_track(&bytes, total).map_err(|wrong| {
                self.distrust();
                MediaError::Refused(wrong)
            })?,
            (FRAME_REFUSED, bytes) => return Err(MediaError::Refused(sentence(&bytes))),
            (kind, _) => {
                self.distrust();
                return Err(MediaError::Refused(format!(
                    "a reply of kind {kind}, which no movie request is answered with"
                )));
            }
        };
        self.decode_track(file, &track, max_samples)
    }

    /// The movie box's payload in `file`, found by the worker's answers about each box at the top.
    fn movie<'f>(&mut self, file: &'f [u8], total: u64) -> Result<&'f [u8], MediaError> {
        let mut at = 0u64;
        let mut movie = None;
        let mut steps = 0usize;
        while at < total {
            steps = steps.saturating_add(1);
            if steps > MAX_TOP_LEVEL_BOXES {
                return Err(MediaError::Refused(format!(
                    "the file holds more than the {MAX_TOP_LEVEL_BOXES} boxes at its top this \
                     player walks"
                )));
            }
            let start = usize::try_from(at).unwrap_or(usize::MAX);
            let header = file
                .get(start..start.saturating_add(TOP_HEADER_BYTES).min(file.len()))
                .unwrap_or_default();
            let found = match self.exchange(FRAME_BOX, &encode_box_request(total, at, header))? {
                (FRAME_BOXED, bytes) => decode_boxed(&bytes, at, total).map_err(|wrong| {
                    self.distrust();
                    MediaError::Refused(wrong)
                })?,
                (FRAME_REFUSED, bytes) => return Err(MediaError::Refused(sentence(&bytes))),
                (kind, _) => {
                    self.distrust();
                    return Err(MediaError::Refused(format!(
                        "a reply of kind {kind}, which no box request is answered with"
                    )));
                }
            };
            if &found.kind == b"moov" {
                if movie.is_some() {
                    return Err(MediaError::Refused(
                        "the file holds two movie boxes (moov), and has one".to_owned(),
                    ));
                }
                movie = Some((found.payload, found.end));
            }
            at = found.end;
        }
        let (start, end) = movie
            .ok_or_else(|| MediaError::Refused("the file holds no movie box (moov)".to_owned()))?;
        // `decode_boxed` held both inside the file.
        let start = usize::try_from(start).unwrap_or(usize::MAX);
        let end = usize::try_from(end).unwrap_or(usize::MAX);
        let movie = file.get(start..end).unwrap_or_default();
        if movie.len() > MAX_MOVIE_BYTES {
            return Err(MediaError::Refused(format!(
                "the file's movie box is {} bytes, past the {MAX_MOVIE_BYTES} this player sends \
                 its media worker",
                movie.len()
            )));
        }
        Ok(movie)
    }

    /// `track`'s access units decoded in runs by the worker's clip decoder, then presented as its
    /// edit list says.
    fn decode_track(
        &mut self,
        file: &[u8],
        track: &Track,
        max_samples: usize,
    ) -> Result<Sound, MediaError> {
        let mut decoded: Vec<i32> = Vec::new();
        let mut format: Option<(u32, u16)> = None;
        let mut concealed = 0u64;
        // The speakers the first run that held samples stated, which every later run must keep.
        let mut speakers: Option<u32> = None;
        let mut next = 0usize;
        while next < track.units.len() {
            // The first run is one unit, which tells the channels; after it a run is as many as
            // its reply holds, under the request's bytes.
            let channels = format.map_or(0, |(_, channels)| usize::from(channels));
            let by_samples = MAX_SAMPLES_PER_REQUEST
                .checked_div(MAX_FRAMES_PER_UNIT.saturating_mul(channels))
                .unwrap_or(1)
                .clamp(1, MAX_UNITS_PER_RUN);
            let mut run: Vec<&[u8]> = Vec::new();
            let mut bytes = 5usize;
            for &(offset, size) in track.units.iter().skip(next).take(by_samples) {
                let size = usize::try_from(size).unwrap_or(usize::MAX);
                let cost = size.saturating_add(4);
                if !run.is_empty() && bytes.saturating_add(cost) > MAX_REQUEST_BYTES {
                    break;
                }
                bytes = bytes.saturating_add(cost);
                // `decode_track` held every unit inside the file.
                let start = usize::try_from(offset).unwrap_or(usize::MAX);
                run.push(
                    file.get(start..start.saturating_add(size))
                        .unwrap_or_default(),
                );
            }
            next = next.saturating_add(run.len());
            let pcm = match self.exchange(FRAME_UNITS, &encode_units(&run))? {
                (FRAME_PCM, bytes) => decode_pcm(&bytes, run.len()).map_err(|wrong| {
                    self.distrust();
                    MediaError::Refused(wrong)
                })?,
                (FRAME_REFUSED, bytes) => return Err(MediaError::Refused(sentence(&bytes))),
                (kind, _) => {
                    self.distrust();
                    return Err(MediaError::Refused(format!(
                        "a reply of kind {kind}, which no run of access units is answered with"
                    )));
                }
            };
            concealed = concealed.saturating_add(u64::from(pcm.concealed));
            if !pcm.samples.is_empty()
                && let Err(changed) = kept(&mut format, &mut speakers, &pcm)
            {
                self.distrust();
                return Err(MediaError::Refused(changed));
            }
            if let Some((rate, channels)) = format {
                let (ahead, kept_all) = presented_limit(track, rate, channels, max_samples);
                if u64::try_from(decoded.len().saturating_add(pcm.samples.len()))
                    .unwrap_or(u64::MAX)
                    > ahead
                {
                    // Past what the edit list keeps, the rest is the encoder's padding; past the
                    // bound, the clip is too long whatever the list keeps.
                    if !kept_all {
                        return Err(MediaError::TooLarge {
                            samples: ahead.saturating_add(1),
                            limit: max_samples,
                        });
                    }
                    let kept = usize::try_from(ahead).unwrap_or(usize::MAX);
                    let room = kept.saturating_sub(decoded.len());
                    decoded.extend_from_slice(pcm.samples.get(..room).unwrap_or(&pcm.samples));
                    break;
                }
            }
            decoded.extend_from_slice(&pcm.samples);
        }
        let Some((rate, channels)) = format else {
            return Err(MediaError::Refused(format!(
                "none of the clip's {} access units decoded",
                track.units.len()
            )));
        };
        let samples = presented(&decoded, track, rate, channels, max_samples)?;
        let mut notes = Vec::new();
        if let Some(shortfall) = self
            .host
            .as_ref()
            .and_then(|host| host.confinement().shortfall())
        {
            notes.push(format!("the media worker ran {shortfall}"));
        }
        if concealed != 0 {
            notes.push(format!(
                "{concealed} of the clip's {} AAC access units could not be decoded and were \
                 played as silence",
                track.units.len()
            ));
        }
        notes.extend(track.presentation.note.iter().cloned());
        Ok(Sound {
            rate,
            channels,
            samples,
            speakers: speakers.unwrap_or_default(),
            notes,
        })
    }

    /// The running worker dropped and killed: it answered what no request could.
    fn distrust(&mut self) {
        if let Some(host) = self.host.take() {
            host.canceller().cancel();
        }
    }

    /// One request and its reply, starting a worker where none runs, under the request's budget.
    fn exchange(&mut self, kind: u8, payload: &[u8]) -> Result<(u8, Vec<u8>), MediaError> {
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
        let (exchanged, ended) = match allowance {
            Some(allowance) => within(&allowance, &canceller, || {
                host.exchange(kind, payload, None)
            })?,
            None => (host.exchange(kind, payload, None), None),
        };
        if let Some(ended) = ended {
            self.host = None;
            canceller.reap();
            return Err(match ended {
                Ended::Spent => MediaError::Spent,
                Ended::Ceiling { after } => MediaError::Stalled { after },
            });
        }
        exchanged.map_err(|error| {
            self.host = None;
            error.into()
        })
    }

    /// One request and its checked reply, starting a worker where none runs.
    fn request(&mut self, format: SoundFormat, run: &[u8]) -> Result<Decoded, MediaError> {
        let payload = encode_request(&format, run);
        match self.exchange(FRAME_DECODE, &payload)? {
            (FRAME_SAMPLES, bytes) => decode_samples(&bytes, &format, run.len()).map_err(|wrong| {
                // A worker that answers what its request cannot hold is not believed again.
                self.distrust();
                MediaError::Refused(wrong)
            }),
            (FRAME_REFUSED, bytes) => Err(MediaError::Refused(sentence(&bytes))),
            (kind, _) => {
                self.distrust();
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

/// The most samples worth decoding of `track` at `rate` and `channels` — the skipped priming and
/// what the edit list keeps, or the skipped priming and `max_samples` where the list keeps more or
/// everything — and whether that is the whole of what the list keeps, so that decoding stops there
/// rather than refusing the clip as too long.
fn presented_limit(track: &Track, rate: u32, channels: u16, max_samples: usize) -> (u64, bool) {
    let presentation = &track.presentation;
    let skip = rescaled(presentation.skip, track.time_scale, rate);
    let max = u64::try_from(max_samples)
        .unwrap_or(u64::MAX)
        .checked_div(u64::from(channels))
        .unwrap_or(0);
    let kept = presentation
        .keep
        .map(|keep| rescaled(keep, track.time_scale, rate))
        .filter(|kept| *kept <= max);
    let frames = skip.saturating_add(kept.unwrap_or(max));
    (frames.saturating_mul(u64::from(channels)), kept.is_some())
}

/// `decoded` presented as `track`'s edit list says (ISO/IEC 14496-12, 8.6.6; [`crate::mp4::presentation`]):
/// the delay as silence first, the skipped priming dropped, and no more than the kept length after
/// it — each figure moved from the media's clock to the output's `rate`.
///
/// # Errors
///
/// [`MediaError::TooLarge`] where the presented clip passes `max_samples`.
fn presented(
    decoded: &[i32],
    track: &Track,
    rate: u32,
    channels: u16,
    max_samples: usize,
) -> Result<Vec<i32>, MediaError> {
    let presentation = &track.presentation;
    let width = u64::from(channels);
    let to_samples = |units: u64| {
        let frames = rescaled(units, track.time_scale, rate);
        usize::try_from(frames.saturating_mul(width)).unwrap_or(usize::MAX)
    };
    let delay = to_samples(presentation.delay);
    let skip = to_samples(presentation.skip).min(decoded.len());
    let after = decoded.get(skip..).unwrap_or_default();
    let kept = presentation
        .keep
        .map_or(after.len(), |keep| to_samples(keep).min(after.len()));
    let length = delay.saturating_add(kept);
    if length > max_samples {
        return Err(MediaError::TooLarge {
            samples: u64::try_from(length).unwrap_or(u64::MAX),
            limit: max_samples,
        });
    }
    let mut samples = vec![0; delay];
    samples.extend_from_slice(after.get(..kept).unwrap_or(after));
    Ok(samples)
}

/// Holds a run's rate, channels and speakers to the first run's that held samples, which one clip
/// keeps throughout; the sentence for a run that changes any of them.
fn kept(
    format: &mut Option<(u32, u16)>,
    speakers: &mut Option<u32>,
    pcm: &crate::aac::Pcm,
) -> Result<(), String> {
    if format.is_some_and(|first| first != (pcm.rate, pcm.channels)) {
        return Err(format!(
            "the stream's output changed to {} Hz and {} channel(s) after it began",
            pcm.rate, pcm.channels
        ));
    }
    if speakers.is_some_and(|first| first != pcm.speakers) {
        return Err(format!(
            "the stream's speakers changed to channel mask {:#010x} after it began",
            pcm.speakers
        ));
    }
    *format = Some((pcm.rate, pcm.channels));
    *speakers = Some(pcm.speakers);
    Ok(())
}
