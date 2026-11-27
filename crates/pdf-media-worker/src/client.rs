//! The host's half: a sound's bytes split into requests, each answered by the worker under its own
//! processor-time budget, and the worker started only when the first sound is played.

use std::ops::ControlFlow;
use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

use confined_transport::ring::RingError;
use confined_transport::stream::StreamError;
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

mod video;
pub use video::{MAX_GROUP_PARTS, RING_SLOTS, VideoStreamed};

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

/// Why a sound or a video track was not decoded.
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
    /// An interlaced H.264 track, which no decoder this program admits reads: the caller hands it
    /// to this machine's own player and says so, in this sentence (ADR 3526).
    #[error("{0}")]
    Interlaced(String),
    /// A video run's stream failed or broke its contract, and the worker was stopped.
    #[error("{0}")]
    Stream(#[from] StreamError),
    /// The ring a video track's pictures cross in could not be attached.
    #[error("{0}")]
    Ring(#[from] RingError),
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

/// One run of a sound as the worker decoded it, handed on before the next is asked for, so that a
/// player can start the sound at its first run (ADR 3457).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Run<'s> {
    /// The sound's rate, the same in every run.
    pub rate: u32,
    /// Its channels, the same in every run.
    pub channels: u16,
    /// Its speakers, as [`Sound::speakers`], the same in every run.
    pub speakers: u32,
    /// The most frames the whole sound holds, where that is known before it is decoded: a sound
    /// object's and a WAVE file's exactly, an MP4 clip's where its edit list states the length it
    /// keeps. A player refuses a sound past its bound by it before anything sounds.
    pub at_most: Option<u64>,
    /// This run's samples, whole frames interleaved, each in the most significant bits of an `i32`.
    pub samples: &'s [i32],
    /// What the decode does not carry of the stream's data, as far as it is known at this run;
    /// [`Streamed::notes`] holds these and any found later.
    pub notes: &'s [String],
    /// Whether this run is the sound's last, so that a sound of one run is taken whole.
    pub last: bool,
}

/// A sound's run handed on: [`ControlFlow::Break`] to stop decoding — the sound was stopped while
/// it was still arriving.
pub type Deliver<'d> = dyn FnMut(Run<'_>) -> ControlFlow<()> + 'd;

/// What a streamed decode says once its last run has been handed on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Streamed {
    /// What the decode did not carry of the stream's data, each said, as [`Sound::notes`].
    pub notes: Vec<String>,
    /// Whether the receiver stopped it before its end.
    pub stopped: bool,
}

/// A whole sound gathered from its runs: what [`MediaWorker::decode`] and its kin answer.
#[derive(Default)]
struct Gathered {
    format: Option<(u32, u16, u32)>,
    samples: Vec<i32>,
}

impl Gathered {
    fn deliver(&mut self) -> impl FnMut(Run<'_>) -> ControlFlow<()> + '_ {
        |run| {
            self.format = Some((run.rate, run.channels, run.speakers));
            self.samples.extend_from_slice(run.samples);
            ControlFlow::Continue(())
        }
    }

    fn sound(self, streamed: Streamed, fallback: (u32, u16)) -> Sound {
        let (rate, channels, speakers) = self.format.unwrap_or((fallback.0, fallback.1, 0));
        Sound {
            rate,
            channels,
            samples: self.samples,
            speakers,
            notes: streamed.notes,
        }
    }
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
    /// The bytes of decoded planes each part of a group played backward holds, where a caller
    /// lowered it below [`crate::video::MAX_GROUP_BYTES`] ([`Self::with_group_bytes`]).
    group_bytes: Option<usize>,
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

    /// The same handle letting each group of a movie played backward out in parts of at most
    /// `bytes` of decoded planes, where that is below [`crate::video::MAX_GROUP_BYTES`]: the bound
    /// is only ever lowered, which a test uses to split a small fixture's groups exactly as a
    /// 1080-line clip's are split (ADR 3843).
    #[must_use]
    pub fn with_group_bytes(mut self, bytes: usize) -> Self {
        self.group_bytes = Some(bytes.min(crate::video::MAX_GROUP_BYTES));
        self
    }

    /// How many workers this handle has started.
    #[must_use]
    pub fn starts(&self) -> usize {
        self.starts
    }

    /// The running worker's process, where one runs: what a measurement of its memory reads
    /// `/proc/<pid>/status` of (ADR 3528).
    #[must_use]
    pub fn process_id(&self) -> Option<u32> {
        self.host.as_ref()?.canceller().process_id()
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
        let mut gathered = Gathered::default();
        let streamed = self.decode_streamed(format, data, max_samples, &mut gathered.deliver())?;
        Ok(gathered.sound(streamed, (format.rate, format.channels)))
    }

    /// [`Self::decode`], each run handed to `deliver` as the worker answers it rather than
    /// gathered, so that the sound can start at its first (ADR 3457). Every bound is checked as
    /// [`Self::decode`] checks it, the sound's length before the first request.
    ///
    /// # Errors
    ///
    /// As [`Self::decode`]; a run already handed on stays handed on.
    pub fn decode_streamed(
        &mut self,
        format: &SoundFormat,
        data: &[u8],
        max_samples: usize,
        deliver: &mut Deliver<'_>,
    ) -> Result<Streamed, MediaError> {
        self.samples_streamed(*format, data, max_samples, 0, deliver)
    }

    /// What a sound object's or a WAVE file's decode does not carry: the worker's confinement where
    /// it fell short, and the samples after the last whole frame.
    fn samples_notes(&self, unframed: u64, channels: u16) -> Vec<String> {
        let mut notes = self.shortfall();
        if unframed != 0 {
            notes.push(format!(
                "the last {unframed} sample(s) of the sound object complete no frame of {channels} \
                 channels (§13.3's interleaved order) and were not played"
            ));
        }
        notes
    }

    /// The worker's confinement where it fell short, said.
    fn shortfall(&self) -> Vec<String> {
        self.host
            .as_ref()
            .and_then(|host| host.confinement().shortfall())
            .map(|shortfall| format!("the media worker ran {shortfall}"))
            .into_iter()
            .collect()
    }

    /// A sound object's samples, or a WAVE file's, decoded run by run with `speakers` beside them.
    fn samples_streamed(
        &mut self,
        format: SoundFormat,
        data: &[u8],
        max_samples: usize,
        speakers: u32,
        deliver: &mut Deliver<'_>,
    ) -> Result<Streamed, MediaError> {
        if let Some(refusal) = format.refusal() {
            return Err(MediaError::Format(refusal));
        }
        let (samples, unframed) = crate::sound::sample_counts(&format, data.len());
        if usize::try_from(samples).map_or(true, |samples| samples > max_samples) {
            return Err(MediaError::TooLarge {
                samples,
                limit: max_samples,
            });
        }
        let at_most = samples.checked_div(u64::from(format.channels));
        let unit = usize::try_from(format.whole_frames_unit()).unwrap_or(usize::MAX);
        // At most `MAX_RUN_BYTES`, cut on a frame: a unit is at most 65,535 channels of 32 bits,
        // 262,140 bytes, under the run's 262,144, so one unit always fits.
        let run_len = MAX_RUN_BYTES
            .checked_div(unit)
            .unwrap_or(1)
            .max(1)
            .saturating_mul(unit)
            .max(1);
        let mut stopped = false;
        let mut delivered = false;
        let mut notes: Option<Vec<String>> = None;
        let runs = data.len().div_ceil(run_len);
        for (index, run) in data.chunks(run_len).enumerate() {
            let part = self.request(format, run)?;
            let notes = notes.get_or_insert_with(|| self.samples_notes(unframed, format.channels));
            if part.samples.is_empty() {
                continue;
            }
            delivered = true;
            let handed = deliver(Run {
                rate: format.rate,
                channels: format.channels,
                speakers,
                at_most,
                samples: &part.samples,
                notes,
                last: index.saturating_add(1) == runs,
            });
            if handed.is_break() {
                stopped = true;
                break;
            }
        }
        let notes = notes.unwrap_or_else(|| self.samples_notes(unframed, format.channels));
        // A sound of no frames still says its format, which says it lasts nothing.
        if !delivered {
            stopped = deliver(Run {
                rate: format.rate,
                channels: format.channels,
                speakers,
                at_most,
                samples: &[],
                notes: &notes,
                last: true,
            })
            .is_break();
        }
        Ok(Streamed { notes, stopped })
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
        let mut gathered = Gathered::default();
        let streamed = self.decode_wave_streamed(file, max_samples, &mut gathered.deliver())?;
        Ok(gathered.sound(streamed, (0, 0)))
    }

    /// [`Self::decode_wave`], each run of samples handed to `deliver` as it is decoded (ADR 3457).
    ///
    /// # Errors
    ///
    /// As [`Self::decode_wave`].
    pub fn decode_wave_streamed(
        &mut self,
        file: &[u8],
        max_samples: usize,
        deliver: &mut Deliver<'_>,
    ) -> Result<Streamed, MediaError> {
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
        self.samples_streamed(
            format,
            samples,
            max_samples,
            layout.described.channel_mask,
            deliver,
        )
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
        let mut gathered = Gathered::default();
        let streamed = self.decode_mp4_streamed(file, max_samples, &mut gathered.deliver())?;
        Ok(gathered.sound(streamed, (0, 0)))
    }

    /// [`Self::decode_mp4`], each run of presented samples handed to `deliver` as it is decoded,
    /// the edit list applied on the way (ADR 3457).
    ///
    /// # Errors
    ///
    /// As [`Self::decode_mp4`]; a clip whose length its edit list does not state and which passes
    /// `max_samples` is refused at the run that passes it, after the runs before it were handed on.
    pub fn decode_mp4_streamed(
        &mut self,
        file: &[u8],
        max_samples: usize,
        deliver: &mut Deliver<'_>,
    ) -> Result<Streamed, MediaError> {
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
        self.decode_track(file, &track, max_samples, deliver)
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

    /// `track`'s access units decoded in runs by the worker's clip decoder, each run presented as
    /// its edit list says and handed on as it is decoded.
    fn decode_track(
        &mut self,
        file: &[u8],
        track: &Track,
        max_samples: usize,
        deliver: &mut Deliver<'_>,
    ) -> Result<Streamed, MediaError> {
        let mut format: Option<(u32, u16)> = None;
        let mut concealed = 0u64;
        // The speakers the first run that held samples stated, which every later run must keep.
        let mut speakers: Option<u32> = None;
        let mut presenting: Option<Presenting> = None;
        let mut notes: Option<Vec<String>> = None;
        let mut stopped = false;
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
            let Some((rate, channels)) = format else {
                continue;
            };
            let presenting = match &mut presenting {
                Some(presenting) => presenting,
                None => presenting.insert(Presenting::new(track, rate, channels, max_samples)?),
            };
            let (samples, last) = presenting.presented(&pcm.samples)?;
            let notes = notes.get_or_insert_with(|| {
                let mut notes = self.shortfall();
                notes.extend(track.presentation.note.iter().cloned());
                notes
            });
            let handed = deliver(Run {
                rate,
                channels,
                speakers: speakers.unwrap_or_default(),
                at_most: presenting.at_most,
                samples: &samples,
                notes,
                last: last || next >= track.units.len(),
            });
            if handed.is_break() {
                stopped = true;
                break;
            }
            if last {
                break;
            }
        }
        if format.is_none() {
            return Err(MediaError::Refused(format!(
                "none of the clip's {} access units decoded",
                track.units.len()
            )));
        }
        let mut notes = notes.unwrap_or_default();
        if concealed != 0 {
            notes.push(format!(
                "{concealed} of the clip's {} AAC access units could not be decoded and were \
                 played as silence",
                track.units.len()
            ));
        }
        Ok(Streamed { notes, stopped })
    }

    /// Starts a worker, the one this player's next request goes to.
    fn start(&mut self) -> Result<(), MediaError> {
        let program = self.program()?;
        let host = Host::start(&program, MAGIC, &Canceller::new())?;
        self.starts = self.starts.saturating_add(1);
        self.host = Some(host);
        Ok(())
    }

    /// The running worker dropped and killed: it answered what no request could.
    fn distrust(&mut self) {
        if let Some(host) = self.host.take() {
            host.canceller().cancel();
        }
    }

    /// One request and its reply, starting a worker where none runs, under the request's budget.
    fn exchange(&mut self, kind: u8, payload: &[u8]) -> Result<(u8, Vec<u8>), MediaError> {
        if self.host.is_none() {
            self.start()?;
        }
        let Some(host) = self.host.as_mut() else {
            return Err(MediaError::Start(
                "the worker ended as it started".to_owned(),
            ));
        };
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

/// A clip's edit list applied run by run (ISO/IEC 14496-12, 8.6.6; [`crate::mp4::presentation`]):
/// the delay as silence first, the skipped priming dropped, and no more than the kept length after
/// it — each figure moved from the media's clock to the output's rate.
#[derive(Debug)]
struct Presenting {
    /// Samples of silence still to put before the first decoded sample.
    delay: usize,
    /// Decoded samples still to drop.
    skip: usize,
    /// Decoded samples still to keep: the edit list's length, or what is left under the bound.
    keep: usize,
    /// Whether `keep` is the edit list's own length, so that reaching it is the clip's end and not
    /// a clip past the bound.
    stated: bool,
    /// The samples presented so far, which the bound is reported against.
    presented: usize,
    max_samples: usize,
    /// The most frames the clip presents, where its edit list states its length.
    at_most: Option<u64>,
}

impl Presenting {
    /// The edit list of `track` at `rate` and `channels`, under `max_samples`.
    ///
    /// # Errors
    ///
    /// [`MediaError::TooLarge`] where the delay alone passes the bound.
    fn new(
        track: &Track,
        rate: u32,
        channels: u16,
        max_samples: usize,
    ) -> Result<Self, MediaError> {
        let presentation = &track.presentation;
        let width = u64::from(channels);
        let to_samples = |units: u64| {
            let frames = rescaled(units, track.time_scale, rate);
            usize::try_from(frames.saturating_mul(width)).unwrap_or(usize::MAX)
        };
        let delay = to_samples(presentation.delay);
        let skip = to_samples(presentation.skip);
        // A length past the bound is held as no length: the clip is refused only if what it
        // decodes to passes the bound, which an edit list longer than its media may not.
        let stated = presentation
            .keep
            .map(to_samples)
            .filter(|kept| delay.saturating_add(*kept) <= max_samples);
        if delay > max_samples {
            return Err(MediaError::TooLarge {
                samples: u64::try_from(delay).unwrap_or(u64::MAX),
                limit: max_samples,
            });
        }
        let at_most = stated.map(|kept| {
            u64::try_from(delay.saturating_add(kept))
                .unwrap_or(u64::MAX)
                .checked_div(width)
                .unwrap_or(0)
        });
        Ok(Self {
            delay,
            skip,
            keep: stated.unwrap_or(max_samples.saturating_sub(delay)),
            stated: stated.is_some(),
            presented: 0,
            max_samples,
            at_most,
        })
    }

    /// `decoded`, the next run, presented — with the delay before the first — and whether the
    /// clip's stated length has been reached, so that what follows is the encoder's padding.
    ///
    /// # Errors
    ///
    /// [`MediaError::TooLarge`] where a clip whose list states no length passes the bound.
    fn presented(&mut self, decoded: &[i32]) -> Result<(Vec<i32>, bool), MediaError> {
        let mut samples = vec![0; std::mem::take(&mut self.delay)];
        let skipped = self.skip.min(decoded.len());
        self.skip = self.skip.saturating_sub(skipped);
        let after = decoded.get(skipped..).unwrap_or_default();
        if !self.stated && after.len() > self.keep {
            return Err(MediaError::TooLarge {
                samples: u64::try_from(
                    self.presented
                        .saturating_add(samples.len())
                        .saturating_add(after.len()),
                )
                .unwrap_or(u64::MAX),
                limit: self.max_samples,
            });
        }
        let taken = after.len().min(self.keep);
        samples.extend_from_slice(after.get(..taken).unwrap_or(after));
        self.keep = self.keep.saturating_sub(taken);
        self.presented = self.presented.saturating_add(samples.len());
        Ok((samples, self.stated && self.keep == 0))
    }
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
