//! A sound played in the window that shows the document: ISO 32000-2 §13.3's samples decoded in the
//! confined media worker, converted to the device's format, and handed to a sink, all on a thread
//! of the player's own (RFC 0015 section 5, ADRs 3297, 3301).
//!
//! **Nothing here runs before the first sound.** [`Player::new`] starts no thread, no worker and no
//! device: the thread is spawned by the first [`Player::play`], and on it the sink is opened — the
//! audio device probed — before that sound is handed over, and the worker started before it is
//! decoded. A document that plays nothing costs the launch path nothing (`CLAUDE.md`'s startup
//! rules, RFC 0015 section 4.6).
//!
//! **Nothing here runs on the window's thread either.** Decoding waits on the worker, up to its
//! processor budget per request (`pdf_media_worker::PROCESSOR_BUDGET`), and opening a device can
//! wait on the sound server; the window sends an order and is woken when there is something to
//! say, which it takes with [`Player::heard`].
//!
//! The device is behind [`Sink`], the seam a window fills with [`helper::open`] — the sound helper
//! program, which alone links an audio library (ADR 3317) — and a test with a recording double.
//! What crosses it is samples this program decoded, never the document's bytes, which is the
//! standing ADR 3297 gave the C on the output side.

pub mod clock;
mod convert;
pub mod helper;
mod painting;
mod passes;
mod window;

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use clock::MediaClock;
use passes::{Pass, Passes, levelled};
pub use quorra_sound::{Progress, Report};

pub use convert::{ConversionError, DeviceFormat, MAX_PLAYED_SAMPLES, converted, speakers_said};
use pdf_media_worker::MediaWorker;
use pdf_media_worker::sound::{Encoding, SoundFormat};
use viewer_core::{DocumentId, Recording, SoundData};
pub use window::{HANDED_ON, HELD, Hearing, LOOK, PAUSED, Sounds};

/// Where the converted samples go: a device, or a test's record of what reached it.
///
/// Made on the player's thread by [`OpenSink`] and never moved off it, so that a toolkit whose
/// stream may not cross threads can be one.
pub trait Sink {
    /// The device's rate and channel count, which each sound is converted to before
    /// [`Self::start`].
    fn format(&self) -> DeviceFormat;

    /// Starts `samples`, interleaved at [`Self::format`] as fractions of full scale, in place of
    /// whatever was sounding; `report` is told, from whichever thread the device runs on, how many
    /// frames have been written as they are, and once that the last has been — the count the
    /// media clock is read from ([`clock`], ADR 3342).
    ///
    /// # Errors
    ///
    /// The sentence to say where the device would not start.
    fn start(&mut self, samples: Vec<f32>, report: Report) -> Result<(), String>;

    /// Stops what is sounding; nothing where nothing is.
    fn stop(&mut self);
}

/// What opens the sink, called on the player's thread at the first sound and at no other time
/// while it stays open.
pub type OpenSink = Box<dyn FnMut() -> Result<Box<dyn Sink>, String> + Send>;

/// What wakes the window's loop when the player has something to say.
pub type Wake = Box<dyn Fn() + Send>;

/// One sound to play, from [`viewer_core::Event::Media`]'s `sound`.
#[derive(Debug, Clone, PartialEq)]
pub struct Played {
    /// Which document asked for it, so that closing that document stops it.
    pub document: DocumentId,
    /// The document's label and the sound's name, for the sentences.
    pub label: String,
    /// The sound as the document holds it: §13.3's samples or a rendition's RIFF WAVE clip.
    pub sound: SoundData,
}

/// What the player has to say, taken by the window with [`Player::heard`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Heard {
    /// A sound is sounding; the sentence says which, and what it was converted to.
    Playing(String),
    /// A sound is sounding synchronously — Table 212's `/Synchronous` — and holds the reader:
    /// "allowing no further user interaction other than cancelling the sound, until the sound has
    /// been completely played" (ISO 32000-2 §12.6.4.9). The window refuses every key and click but
    /// the one that stops it (ADR 3343).
    Holding(String),
    /// A sound was decoded and is held at its start, not sounding: Table 291's `/A` false, "the
    /// media shall be initially paused when activated". [`Player::resume`] starts it.
    Paused(String),
    /// The sound came to its end.
    Ended(String),
    /// The sound was stopped: by [`Player::stop`], a second sound, or its document closing.
    Stopped(String),
    /// A sound was not played: the worker, the conversion or the device refused, and the sentence
    /// says which and why.
    Refused(String),
}

impl Heard {
    /// The sentence, whichever it is.
    #[must_use]
    pub fn sentence(&self) -> &str {
        match self {
            Self::Playing(said)
            | Self::Holding(said)
            | Self::Paused(said)
            | Self::Ended(said)
            | Self::Stopped(said)
            | Self::Refused(said) => said,
        }
    }
}

/// What the window asks of the player's thread.
enum Order {
    /// Play this, stopping whatever sounds.
    Play(Played),
    /// Stop whatever sounds.
    Stop,
    /// This document closed; stop its sound if it is the one sounding.
    Closed(DocumentId),
    /// Start the sound held at its start.
    Resume,
    /// What the device said of the pass numbered so (sent by the sink's report).
    Progress(u64, Progress),
    /// The window is closing.
    Quit,
}

/// The window's handle on its sounds: one at a time, on a thread of its own from the first.
///
/// Each sound is played as §13.2.5's play parameters ask (ADR 3341): its level, its passes — the
/// simple duration, repeated as the repeat count says — and whether it starts at once; and its
/// time is the [`MediaClock`] the device's count drives (ADR 3342), which [`Self::position`] reads.
///
/// A second sound stops the first, which is Table 212's `/Mix` at its default `false`: "If
/// `false`, any previously playing sound shall be stopped before starting this sound". A `/Mix`
/// `true` is not mixed — the device takes one sound — and the sentence for the sound says so.
pub struct Player {
    /// The sink's maker and the waker, until the first sound takes them to the thread.
    unstarted: Option<(OpenSink, Wake)>,
    /// The worker program, where the window names it; otherwise found beside this executable.
    worker: Option<PathBuf>,
    /// The thread, from the first sound on.
    running: Option<Running>,
    /// What was said before a thread existed to say it — a thread that would not start.
    early: Vec<Heard>,
    /// The clock of the sound now sounding, which the thread moves and the window reads.
    clock: Arc<Mutex<Option<MediaClock>>>,
    /// The screen annotation a sound plays in, followed from what the player says (ADR 3407).
    painting: painting::Painting,
}

/// The player's thread, and the two directions of talk with it.
struct Running {
    orders: Sender<Order>,
    heard: Receiver<Heard>,
    thread: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for Player {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Player")
            .field("started", &self.running.is_some())
            .field("worker", &self.worker)
            .finish_non_exhaustive()
    }
}

impl Player {
    /// A player that will open its sink with `open` and wake the window with `wake`. Starts
    /// nothing now.
    #[must_use]
    pub fn new(open: OpenSink, wake: Wake) -> Self {
        Self {
            unstarted: Some((open, wake)),
            worker: None,
            running: None,
            early: Vec::new(),
            clock: Arc::new(Mutex::new(None)),
            painting: painting::Painting::default(),
        }
    }

    /// The same, decoding with the worker `program` rather than the one beside this executable.
    #[must_use]
    pub fn with_worker(mut self, program: PathBuf) -> Self {
        self.worker = Some(program);
        self
    }

    /// Whether the first sound has started the thread.
    #[must_use]
    pub fn started(&self) -> bool {
        self.running.is_some()
    }

    /// Plays `played` in place of whatever sounds, starting the thread if this is the first.
    pub fn play(&mut self, played: Played) {
        if self.running.is_none() {
            self.start();
        }
        self.painting.played(played.document, played.sound.screen);
        self.send(Order::Play(played));
    }

    /// Stops whatever sounds. Nothing before the first sound.
    pub fn stop(&mut self) {
        self.send(Order::Stop);
    }

    /// Stops the sound `document` asked for, if it is the one sounding.
    pub fn closed(&mut self, document: DocumentId) {
        self.send(Order::Closed(document));
    }

    /// Starts the sound [`Heard::Paused`] said is held at its start; nothing where none is.
    pub fn resume(&mut self) {
        self.send(Order::Resume);
    }

    /// Where the sound in hand is in its time, read from the device's count of frames (ADR 3342):
    /// every pass sounded and frozen so far and the device's count in this one. `None` where no
    /// sound is in hand.
    #[must_use]
    pub fn position(&self) -> Option<Duration> {
        self.clock
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .map(|clock| clock.time(Instant::now()))
    }

    /// What the player has said since the last call, oldest first.
    pub fn heard(&mut self) -> Vec<Heard> {
        let mut heard = std::mem::take(&mut self.early);
        if let Some(running) = &self.running {
            heard.extend(running.heard.try_iter());
        }
        for said in &heard {
            self.painting.heard(said);
        }
        heard
    }

    /// What the core is owed since the last call about the rectangle a sound plays in: a
    /// [`viewer_core::Command::PlayingIn`] when a sound with a [`viewer_core::SoundData::screen`]
    /// starts, and one with no screen when it ends, is stopped or another takes its place — read
    /// from what [`Self::heard`] took, so a window asks after it (ISO 32000-2 Table 294's `/W` 3,
    /// ADR 3407).
    pub fn repainted(&mut self) -> Vec<viewer_core::Command> {
        self.painting.owed()
    }

    fn send(&mut self, order: Order) {
        if let Some(running) = &self.running
            && running.orders.send(order).is_err()
        {
            self.early.push(Heard::Refused(
                "the sound player's thread has ended, so nothing more is played in this window"
                    .to_owned(),
            ));
        }
    }

    fn start(&mut self) {
        let Some((open, wake)) = self.unstarted.take() else {
            return;
        };
        let (orders, inbox) = channel();
        let (say, heard) = channel();
        let ended = orders.clone();
        let worker = self.worker.clone();
        let clock = Arc::clone(&self.clock);
        let spawned = std::thread::Builder::new()
            .name("media-player".to_owned())
            .spawn(move || {
                Thread {
                    inbox,
                    ended,
                    say,
                    wake,
                    open,
                    media: worker.map_or_else(MediaWorker::new, MediaWorker::with_program),
                    sink: None,
                    sounding: None,
                    serial: 0,
                    clock,
                }
                .run();
            });
        match spawned {
            Ok(thread) => {
                self.running = Some(Running {
                    orders,
                    heard,
                    thread: Some(thread),
                });
            }
            Err(error) => self.early.push(Heard::Refused(format!(
                "the sound player's thread could not be started: {error}"
            ))),
        }
    }
}

impl Drop for Player {
    /// Ends the thread and waits for it, so that the device is closed and the worker killed before
    /// the window is gone. The wait is at most one request's decoding, which the worker's budget
    /// bounds.
    fn drop(&mut self) {
        if let Some(running) = &mut self.running {
            let _ = running.orders.send(Order::Quit);
            if let Some(thread) = running.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

/// Where the sound now in hand is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    /// Decoded and held at its start: Table 291's `/A` false.
    Held,
    /// A pass's frames are at the device.
    Sounding,
    /// A pass's frames have sounded and its freeze lasts.
    Frozen,
}

/// The sound now in hand: its document and label, the whole of it converted and levelled, and
/// where its passes are.
struct Sounding {
    /// The number of the pass now at the device, which its reports carry.
    serial: u64,
    document: DocumentId,
    label: String,
    /// Every sample of the sound, converted for the device and scaled to its level; each pass
    /// sounds the first of them.
    samples: Vec<f32>,
    /// The device's channels, so that a pass's frames are counted in samples.
    channels: usize,
    passes: Passes,
    /// The pass now held, sounding or frozen.
    pass: Pass,
    stage: Stage,
    /// Whether it holds the reader (Table 212's `/Synchronous`).
    synchronous: bool,
}

/// The player's thread's state.
struct Thread {
    inbox: Receiver<Order>,
    /// A sender to its own inbox, which each pass's report posts through.
    ended: Sender<Order>,
    say: Sender<Heard>,
    wake: Wake,
    open: OpenSink,
    media: MediaWorker,
    sink: Option<Box<dyn Sink>>,
    sounding: Option<Sounding>,
    serial: u64,
    /// The clock of the sound in hand, shared with [`Player::position`].
    clock: Arc<Mutex<Option<MediaClock>>>,
}

impl Thread {
    fn run(mut self) {
        let mut queued: VecDeque<Order> = VecDeque::new();
        loop {
            let order = match queued.pop_front() {
                Some(order) => order,
                None => match self.thaws_at() {
                    None => match self.inbox.recv() {
                        Ok(order) => order,
                        Err(_) => return,
                    },
                    Some(at) => match self
                        .inbox
                        .recv_timeout(at.saturating_duration_since(Instant::now()))
                    {
                        Ok(order) => order,
                        Err(RecvTimeoutError::Timeout) => {
                            self.thawed();
                            continue;
                        }
                        Err(RecvTimeoutError::Disconnected) => return,
                    },
                },
            };
            match order {
                Order::Quit => return,
                Order::Stop => self.silence(),
                Order::Closed(document) => {
                    if self
                        .sounding
                        .as_ref()
                        .is_some_and(|sounding| sounding.document == document)
                    {
                        self.silence();
                    }
                }
                Order::Resume => self.resumed(),
                Order::Progress(serial, progress) => self.progressed(serial, progress),
                Order::Play(played) => {
                    self.silence();
                    let samples = match self.prepared(&played) {
                        Ok(samples) => samples,
                        Err(why) => {
                            self.tell(Heard::Refused(format!(
                                "{} was not played: {why}",
                                played.label
                            )));
                            continue;
                        }
                    };
                    // Orders that arrived while the sound was decoded are read before it sounds:
                    // a stop, a newer sound or its document closing means it never starts.
                    queued.extend(self.inbox.try_iter());
                    if queued
                        .iter()
                        .any(|order| supersedes(order, played.document))
                    {
                        continue;
                    }
                    self.sound(played, samples);
                }
            }
        }
    }

    /// The sound decoded in the worker and converted for the sink, which is opened here at the
    /// first sound; the sentence for the window beside the samples.
    fn prepared(&mut self, played: &Played) -> Result<(Vec<f32>, String), String> {
        let decoded = match &played.sound.recording {
            Recording::Samples { format, data } => {
                self.media
                    .decode(&worker_format(*format), data, MAX_PLAYED_SAMPLES)
            }
            // A rendition's clip: the worker reads the file's header too (ADR 3349).
            Recording::Wave(file) => self.media.decode_wave(file, MAX_PLAYED_SAMPLES),
            // An MP4 clip's AAC: the worker walks the file's boxes and decodes its stream, and this
            // side cuts the file where it is told (ADR 3381).
            Recording::Mp4(file) => self.media.decode_mp4(file, MAX_PLAYED_SAMPLES),
        }
        .map_err(|error| error.to_string())?;
        if self.sink.is_none() {
            self.sink = Some((self.open)().map_err(|why| format!("the audio device: {why}"))?);
        }
        let Some(sink) = self.sink.as_ref() else {
            return Err("the audio device was not opened".to_owned());
        };
        let device = sink.format();
        let samples = converted(
            decoded.rate,
            decoded.channels,
            decoded.speakers,
            &decoded.samples,
            device,
            MAX_PLAYED_SAMPLES,
        )
        .map_err(|error| error.to_string())?;
        // §13.3 for a sound object's samples, §13.2.4.2 for a rendition's clip.
        let clause = match played.sound.recording {
            Recording::Samples { .. } => "§13.3",
            Recording::Wave(_) => "§13.2.4.2, a RIFF WAVE clip",
            Recording::Mp4(_) => "§13.2.4.2, an MP4 clip's AAC",
        };
        let mut said = format!(
            "{} is playing: {} Hz, {} channel(s), converted for the device's {} Hz, {} channel(s) \
             (ISO 32000-2 {clause})",
            played.label, decoded.rate, decoded.channels, device.rate, device.channels
        );
        let speakers = speakers_said(decoded.speakers, decoded.channels, device);
        for note in decoded.notes.iter().chain(&speakers) {
            said.push_str("; ");
            said.push_str(note);
        }
        Ok((samples, said))
    }

    /// The sound in hand from now: levelled, cut into its passes, its clock at zero, and its first
    /// pass started — or held, where it does not play at once.
    fn sound(&mut self, played: Played, (mut samples, mut said): (Vec<f32>, String)) {
        let playing = played.sound.playing;
        let Some(format) = self.sink.as_ref().map(|sink| sink.format()) else {
            return;
        };
        levelled(&mut samples, playing.level);
        for note in described(&playing) {
            said.push_str("; ");
            said.push_str(&note);
        }
        let channels = usize::from(format.channels.max(1));
        sectioned(&mut samples, playing.section, format.rate, channels);
        let intrinsic = u64::try_from(samples.len().checked_div(channels).unwrap_or_default())
            .unwrap_or(u64::MAX);
        let mut passes = Passes::new(&playing, intrinsic, format.rate);
        let Some(pass) = passes.next_pass() else {
            self.tell(Heard::Ended(format!(
                "{} has a duration of nothing (Table 292, Table 291's /RC), so nothing sounded",
                played.label
            )));
            return;
        };
        *self.clock.lock().unwrap_or_else(PoisonError::into_inner) =
            Some(MediaClock::new(format.rate));
        self.sounding = Some(Sounding {
            serial: 0,
            document: played.document,
            label: played.label,
            samples,
            channels,
            passes,
            pass,
            stage: Stage::Held,
            synchronous: playing.synchronous,
        });
        if !playing.auto_play {
            said.push_str(
                "; it is held at its start, because Table 291's /A is false: \"the media shall be \
                 initially paused when activated\"",
            );
            self.tell(Heard::Paused(said));
            return;
        }
        match self.start_pass() {
            Ok(()) => self.tell(if playing.synchronous {
                Heard::Holding(format!(
                    "{said}; it plays synchronously (Table 212's /Synchronous), so until it ends \
                     this window takes no key or click but Escape, which cancels it"
                ))
            } else {
                Heard::Playing(said)
            }),
            Err(why) => self.refused(&why),
        }
    }

    /// The pass in hand to the device, or straight to its freeze where it sounds nothing.
    fn start_pass(&mut self) -> Result<(), String> {
        self.serial = self.serial.wrapping_add(1);
        let serial = self.serial;
        let report = self.ended.clone();
        let (Some(sounding), Some(sink)) = (self.sounding.as_mut(), self.sink.as_mut()) else {
            return Ok(());
        };
        sounding.serial = serial;
        let frames = usize::try_from(sounding.pass.sounding).unwrap_or(usize::MAX);
        let end = frames
            .saturating_mul(sounding.channels)
            .min(sounding.samples.len());
        if end == 0 {
            sounding.stage = Stage::Frozen;
            self.with_clock(|clock| clock.freeze(Instant::now()));
            return Ok(());
        }
        sounding.stage = Stage::Sounding;
        let samples = sounding.samples.get(..end).unwrap_or_default().to_vec();
        sink.start(
            samples,
            Box::new(move |progress| {
                let _ = report.send(Order::Progress(serial, progress));
            }),
        )
    }

    /// The device's word on the pass numbered `serial`; a word on an earlier pass, or on a sound
    /// since stopped, moves nothing.
    fn progressed(&mut self, serial: u64, progress: Progress) {
        let Some(sounding) = self.sounding.as_ref() else {
            return;
        };
        if sounding.serial != serial || sounding.stage != Stage::Sounding {
            return;
        }
        let pass = sounding.pass;
        match progress {
            Progress::Played(frames) => self.with_clock(|clock| clock.played(frames)),
            Progress::Ended => {
                self.with_clock(|clock| clock.sounded(pass.sounding));
                if pass.frozen == Some(0) {
                    self.next_pass();
                } else {
                    if let Some(sounding) = self.sounding.as_mut() {
                        sounding.stage = Stage::Frozen;
                    }
                    self.with_clock(|clock| clock.freeze(Instant::now()));
                }
            }
        }
    }

    /// When the freeze in hand ends, by the clock; `None` where nothing is frozen or the freeze is
    /// for ever (Table 292's `/S /F`), which only an order ends.
    fn thaws_at(&self) -> Option<Instant> {
        let sounding = self.sounding.as_ref()?;
        if sounding.stage != Stage::Frozen {
            return None;
        }
        let frames = sounding.pass.frozen?;
        self.clock
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()?
            .thaws_at(frames)
    }

    /// The freeze in hand has lasted its length: the clock moves by exactly it, and the next pass
    /// starts.
    fn thawed(&mut self) {
        let Some(frames) = self
            .sounding
            .as_ref()
            .and_then(|sounding| sounding.pass.frozen)
        else {
            return;
        };
        self.with_clock(|clock| clock.thawed(frames));
        self.next_pass();
    }

    /// The next pass started, or the sound's end said where the repeat count is spent.
    fn next_pass(&mut self) {
        let Some(sounding) = self.sounding.as_mut() else {
            return;
        };
        match sounding.passes.next_pass() {
            Some(pass) => {
                sounding.pass = pass;
                if let Err(why) = self.start_pass() {
                    self.refused(&why);
                }
            }
            None => {
                if let Some(sounding) = self.sounding.take() {
                    *self.clock.lock().unwrap_or_else(PoisonError::into_inner) = None;
                    self.tell(Heard::Ended(format!(
                        "{} came to its end (ISO 32000-2 §13.3, §13.2.5)",
                        sounding.label
                    )));
                }
            }
        }
    }

    /// The sound held at its start, started.
    fn resumed(&mut self) {
        let Some(sounding) = self.sounding.as_ref() else {
            return;
        };
        if sounding.stage != Stage::Held {
            return;
        }
        let said = format!("{} is playing, resumed from its start", sounding.label);
        let synchronous = sounding.synchronous;
        match self.start_pass() {
            Ok(()) if synchronous => self.tell(Heard::Holding(said)),
            Ok(()) => self.tell(Heard::Playing(said)),
            Err(why) => self.refused(&why),
        }
    }

    /// The device would not start a pass: the sound is dropped and that is said.
    fn refused(&mut self, why: &str) {
        if let Some(sounding) = self.sounding.take() {
            *self.clock.lock().unwrap_or_else(PoisonError::into_inner) = None;
            self.tell(Heard::Refused(format!(
                "{} was not played: the audio device: {why}",
                sounding.label
            )));
        }
    }

    /// Stops what sounds, and says so where something did.
    fn silence(&mut self) {
        if let Some(sink) = self.sink.as_mut() {
            sink.stop();
        }
        if let Some(sounding) = self.sounding.take() {
            *self.clock.lock().unwrap_or_else(PoisonError::into_inner) = None;
            self.tell(Heard::Stopped(format!("{} was stopped", sounding.label)));
        }
    }

    fn with_clock(&self, move_it: impl FnOnce(&mut MediaClock)) {
        if let Some(clock) = self
            .clock
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_mut()
        {
            move_it(clock);
        }
    }

    fn tell(&self, heard: Heard) {
        if self.say.send(heard).is_ok() {
            (self.wake)();
        }
    }
}

/// Cuts `samples` — interleaved frames of `channels` at `rate` — to §13.2.4.3's `section`: from
/// its beginning to its end, or to the clip's own end where it states none or one past it. A
/// beginning at or past the end leaves nothing, which the clause calls "an empty range" and "valid"
/// (ADR 3398).
fn sectioned(
    samples: &mut Vec<f32>,
    section: pdf_model::media::Section,
    rate: u32,
    channels: usize,
) {
    let frames = samples.len().checked_div(channels).unwrap_or_default();
    let frame_at = |seconds: f64| -> usize {
        let at = (seconds * f64::from(rate)).round();
        // A finite, non-negative position past `usize` is past every clip, which `min` holds.
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a non-negative float to integer `as` saturates, and the result is held to the clip's frames"
        )]
        let at = at as usize;
        at.min(frames)
    };
    let end = section.end.map_or(frames, frame_at);
    let begin = frame_at(section.begin).min(end);
    samples.truncate(end.saturating_mul(channels));
    samples.drain(..begin.saturating_mul(channels));
}

/// The parameters that differ from their defaults, each a sentence, so that a person told a sound
/// plays is told how (trap 5).
fn described(playing: &pdf_model::media::Playing) -> Vec<String> {
    use pdf_model::media::SimpleDuration;
    let mut said = Vec::new();
    let section = playing.section;
    if section != pdf_model::media::Section::WHOLE {
        said.push(match section.end {
            Some(end) => format!(
                "from {} s to {end} s of its clip (§13.2.4.3's media clip section)",
                section.begin
            ),
            None => format!(
                "from {} s of its clip to its end (§13.2.4.3's media clip section)",
                section.begin
            ),
        });
    }
    if (playing.level - 1.0).abs() >= f64::EPSILON {
        said.push(format!(
            "at {:.0} % of its recorded level",
            playing.level * 100.0
        ));
    }
    match playing.duration {
        SimpleDuration::Intrinsic => {}
        SimpleDuration::Infinite => {
            said.push("for an infinite duration, frozen silent after its end".to_owned());
        }
        SimpleDuration::Seconds(seconds) => {
            said.push(format!("for a duration of {seconds} s (Table 292)"));
        }
    }
    if playing.repeat == 0.0 {
        said.push("repeated until it is stopped".to_owned());
    } else if (playing.repeat - 1.0).abs() >= f64::EPSILON {
        said.push(format!("{} times (Table 291's /RC)", playing.repeat));
    }
    if playing.mix {
        said.push(
            "Table 212's /Mix asks for it to be mixed with any sound already playing, and this \
             player plays one sound at a time, so any other was stopped"
                .to_owned(),
        );
    }
    said
}

/// Whether `order`, queued while a sound of `document` was being decoded, means that sound is not
/// to start.
fn supersedes(order: &Order, document: DocumentId) -> bool {
    match order {
        Order::Play(_) | Order::Stop | Order::Quit => true,
        Order::Closed(closed) => *closed == document,
        Order::Resume | Order::Progress(..) => false,
    }
}

/// Table 305's entries as `pdf_model` read them, in the worker's vocabulary. `Little` is the
/// worker's reading of a WAVE file's own header and no Table 305 entry names it.
fn worker_format(format: pdf_model::media::SampleFormat) -> SoundFormat {
    SoundFormat {
        rate: format.rate,
        channels: format.channels,
        bits: format.bits,
        encoding: match format.encoding {
            pdf_model::media::Encoding::Raw => Encoding::Raw,
            pdf_model::media::Encoding::Signed => Encoding::Signed,
            pdf_model::media::Encoding::MuLaw => Encoding::MuLaw,
            pdf_model::media::Encoding::ALaw => Encoding::ALaw,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::sectioned;
    use pdf_model::media::Section;

    /// §13.2.4.3's section cuts whole frames: from its beginning to its end, to the clip's end
    /// where it states none or one past it, and to nothing where it begins at or past its end.
    #[test]
    fn a_section_cuts_whole_frames_and_an_empty_range_leaves_none() {
        let frames: Vec<f32> = (0..20u8).map(f32::from).collect();
        let cut = |begin: f64, end: Option<f64>| {
            let mut samples = frames.clone();
            sectioned(&mut samples, Section { begin, end }, 4, 2);
            samples
        };
        assert_eq!(cut(0.0, None), frames);
        assert_eq!(cut(0.5, Some(1.0)), vec![4.0, 5.0, 6.0, 7.0]);
        assert_eq!(cut(2.0, None), vec![16.0, 17.0, 18.0, 19.0]);
        assert_eq!(cut(1.0, Some(99.0)).len(), 12);
        assert_eq!(cut(2.0, Some(1.0)), Vec::<f32>::new());
        assert_eq!(cut(9.0, None), Vec::<f32>::new());
    }
}
