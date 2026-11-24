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
//! **A sound starts at its first run** (ADR 3457). The worker answers a sound a run at a time; each
//! run is converted, levelled, cut to the section and handed to the sink as it arrives, and the
//! first pass starts once the first frames are there, so that the wait before a sound is one run's
//! and not the whole sound's. The sink holds the one copy every pass reads, so a repeat sends
//! nothing again, and a pause asks the sink for the frame the device reached, where the
//! resumption starts (ADR 3458).
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
use pdf_model::media::SimpleDuration;
pub use quorra_sound::{Held, Progress, Report};

pub use convert::{ConversionError, DeviceFormat, MAX_PLAYED_SAMPLES, converted, speakers_said};
use convert::{Converter, samples_after};
use pdf_media_worker::sound::{Encoding, SoundFormat};
use pdf_media_worker::{MediaWorker, Run};
use pdf_syntax::ObjectId;
use std::ops::ControlFlow;
use viewer_core::{DocumentId, MediaOperation, Recording, Screen, SoundData};
pub use window::{HANDED_ON, HELD, Hearing, LOOK, PAUSED, Sounds};

/// Where the converted samples go: a device, or a test's record of what reached it.
///
/// Made on the player's thread by [`OpenSink`] and never moved off it, so that a toolkit whose
/// stream may not cross threads can be one.
pub trait Sink {
    /// The device's rate and channel count, which each sound is converted to before it is
    /// handed over.
    fn format(&self) -> DeviceFormat;

    /// Begins a new sound in place of the one held, stopping whatever sounds; its samples follow
    /// by [`Self::append`] and end with [`Self::complete`]. The sink holds one sound, which every
    /// pass of it reads (ADR 3457).
    ///
    /// # Errors
    ///
    /// The sentence to say where the device has gone.
    fn load(&mut self) -> Result<(), String>;

    /// Adds `samples`, interleaved at [`Self::format`] as fractions of full scale, at the end of
    /// the sound held.
    ///
    /// # Errors
    ///
    /// As [`Self::load`].
    fn append(&mut self, samples: &[f32]) -> Result<(), String>;

    /// Says that the sound held has had its last samples.
    ///
    /// # Errors
    ///
    /// As [`Self::load`].
    fn complete(&mut self) -> Result<(), String>;

    /// Starts a pass of the sound held, from frame `from` for at most `frames` frames, in place of
    /// whatever was sounding, whether or not every sample has arrived; `report` is told, from
    /// whichever thread the device runs on, how many frames of the pass have been written as they
    /// are, and once that the last has been — the count the media clock is read from ([`clock`],
    /// ADR 3342). A pass reaching samples still to come waits for them, counting nothing, and
    /// ends only where `frames` ends it or the sound is complete and its last frame written.
    ///
    /// # Errors
    ///
    /// The sentence to say where the device would not start.
    fn start(&mut self, from: u64, frames: u64, report: Report) -> Result<(), String>;

    /// Stops the pass that sounds and answers how many of its frames the device had reached, or
    /// `None` where it cannot say or nothing sounded (ADR 3458).
    fn pause(&mut self) -> Option<u64>;

    /// Stops what is sounding; nothing where nothing is.
    fn stop(&mut self);
}

/// What opens the sink, called on the player's thread at the first sound and at no other time
/// while it stays open.
pub type OpenSink = Box<dyn FnMut() -> Result<Box<dyn Sink>, String> + Send>;

/// What wakes the window's loop when the player has something to say.
pub type Wake = Box<dyn Fn() + Send>;

/// Which played sound the shared clock is of: the ticket of the one begun last, and the latest
/// ticket whose sound is over — ended, stopped, refused, or never begun.
#[derive(Debug, Default)]
struct Listened {
    ticket: u64,
    over: u64,
}

/// One played sound's time as a listener hears it, for a clip's pictures to follow (ADR 3555):
/// the device's count of the frames it had reached, never the processor's clock.
#[derive(Clone)]
pub struct Listening {
    clock: Arc<Mutex<Option<MediaClock>>>,
    listened: Arc<Mutex<Listened>>,
    ticket: u64,
}

impl std::fmt::Debug for Listening {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Listening")
            .field("ticket", &self.ticket)
            .finish_non_exhaustive()
    }
}

/// Where a [`Listening`] stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundTime {
    /// The sound has not begun: it is still being decoded, or the device is opening.
    Waiting,
    /// The sound is in hand, heard at this time ([`MediaClock::heard_time`]); a paused or held
    /// sound's stands still.
    At(Duration),
    /// The sound is over, or never sounded.
    Over,
}

impl Listening {
    /// Where the sound stands at `now`.
    #[must_use]
    pub fn time(&self, now: Instant) -> SoundTime {
        let listened = self.listened.lock().unwrap_or_else(PoisonError::into_inner);
        if listened.over >= self.ticket || listened.ticket > self.ticket {
            return SoundTime::Over;
        }
        if listened.ticket < self.ticket {
            return SoundTime::Waiting;
        }
        self.clock
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .map_or(SoundTime::Waiting, |clock| {
                SoundTime::At(clock.heard_time(now))
            })
    }
}

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
    /// An order changed nothing, and the sentence says why: §12.6.4.14's stop, pause or resume
    /// found no rendition associated with its annotation, or one not in the state it acts on —
    /// the clause's "there is no effect" — or Table 218's `/OP` 4 found its annotation's rendition
    /// already playing, where it does nothing (ADR 3433). Whatever sounds sounds on. Also what the
    /// worker found of a sound only once it had decoded the last of it, which sounds on too (ADR
    /// 3457).
    Unchanged(String),
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
            | Self::Refused(said)
            | Self::Unchanged(said) => said,
        }
    }
}

/// What the window asks of the player's thread.
enum Order {
    /// Play this, stopping whatever sounds, under its ticket.
    Play(Played, u64),
    /// Stop whatever sounds.
    Stop,
    /// This document closed; stop its sound if it is the one sounding.
    Closed(DocumentId),
    /// Start the sound held at its start, or paused.
    Resume,
    /// §12.6.4.14's stop, pause or resume of the rendition associated with an annotation.
    Operate {
        document: DocumentId,
        annotation: ObjectId,
        operation: MediaOperation,
    },
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
    /// Which played sound the clock is of, and which are over (ADR 3555).
    listened: Arc<Mutex<Listened>>,
    /// The ticket of the last sound given to [`Self::play`], counted from 1.
    tickets: u64,
    /// The screen annotation a sound plays in, followed from what the player says (ADR 3407).
    painting: painting::Painting,
}

/// What the player's thread tells the window: a sentence, or the screen annotation whose rectangle
/// is painted now (ADR 3407) — told by the thread, which alone knows when a sound starts, stops
/// and pauses.
enum Said {
    Heard(Heard),
    Painted(Option<(DocumentId, Screen)>),
}

/// The player's thread, and the two directions of talk with it.
struct Running {
    orders: Sender<Order>,
    heard: Receiver<Said>,
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
            listened: Arc::new(Mutex::new(Listened::default())),
            tickets: 0,
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
        self.tickets = self.tickets.wrapping_add(1);
        self.send(Order::Play(played, self.tickets));
    }

    /// The sound last given to [`Self::play`], to be listened to: what a clip's pictures are
    /// slaved to while it sounds (ADR 3555).
    #[must_use]
    pub fn listening(&self) -> Listening {
        Listening {
            clock: Arc::clone(&self.clock),
            listened: Arc::clone(&self.listened),
            ticket: self.tickets,
        }
    }

    /// Stops whatever sounds. Nothing before the first sound.
    pub fn stop(&mut self) {
        self.send(Order::Stop);
    }

    /// Stops the sound `document` asked for, if it is the one sounding.
    pub fn closed(&mut self, document: DocumentId) {
        self.send(Order::Closed(document));
    }

    /// Starts the sound [`Heard::Paused`] said is held at its start or paused, from where it was
    /// held; nothing where none is.
    pub fn resume(&mut self) {
        self.send(Order::Resume);
    }

    /// §12.6.4.14's `operation` on the rendition `document` associated with `annotation` — the
    /// sound a rendition action naming it as Table 218's `/AN` played — and [`Heard::Unchanged`]
    /// where no sound in hand is that one or it is not in a state the operation acts on (ADR 3433).
    ///
    /// **A pause keeps the device and gives up the stream.** The sink is told to stop, which in the
    /// sound helper closes the output stream and keeps the device it opened at the first sound, as
    /// it is kept between sounds; the samples stay in the helper and the place in them here, and the
    /// clock holds the frame the device said it had reached (ADR 3458). A resume starts the rest of the pass on a new stream of
    /// the same device, which costs no probe of the sound server, and a paused sound holds no
    /// stream another program's sound would share the device with.
    pub fn operate(
        &mut self,
        document: DocumentId,
        annotation: ObjectId,
        operation: MediaOperation,
    ) {
        if self.running.is_none() {
            self.early
                .push(Heard::Unchanged(unassociated(annotation, operation)));
            return;
        }
        self.send(Order::Operate {
            document,
            annotation,
            operation,
        });
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
            for said in running.heard.try_iter() {
                match said {
                    Said::Heard(said) => heard.push(said),
                    Said::Painted(now) => self.painting.paint(now),
                }
            }
        }
        heard
    }

    /// What the core is owed since the last call about the rectangle a sound plays in: a
    /// [`viewer_core::Command::PlayingIn`] when a sound with a [`viewer_core::SoundData::screen`]
    /// starts, and one with no screen when it ends, is stopped or another takes its place — taken
    /// in by [`Self::heard`], so a window asks after it (ISO 32000-2 Table 294's `/W` 3, ADR 3407).
    /// A pause leaves the rectangle painted: the rendition is still associated with it.
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
        let listened = Arc::clone(&self.listened);
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
                    listened,
                    ticket: 0,
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
    /// Paused by Table 218's `/OP` 2, the pass's place kept: [`Sounding::from`] and what is left of
    /// [`Sounding::pass`].
    Paused,
}

/// The sound now in hand: its document and label, where its passes are, and how long it is once
/// its last run has arrived. Its samples are the sink's, which holds the one copy (ADR 3457).
struct Sounding {
    /// The number of the pass now at the device, which its reports carry.
    serial: u64,
    document: DocumentId,
    label: String,
    passes: Passes,
    /// The pass now held, sounding or frozen — after a pause, what was left of it.
    pass: Pass,
    /// The frame of the sound the pass sounds from: zero, or where a pause stopped it.
    from: u64,
    /// The sound's frames once its last run has arrived (§13.2.5's intrinsic duration); `None`
    /// while it is still arriving.
    intrinsic: Option<u64>,
    /// The screen annotation a rendition action associated it with (Table 218's `/AN`).
    rendition: Option<viewer_core::Associated>,
    stage: Stage,
    /// Whether it holds the reader (Table 212's `/Synchronous`).
    synchronous: bool,
}

/// A sound arriving a run at a time: what its runs have done so far (ADR 3457).
#[derive(Default)]
struct Feed {
    /// The conversion for the device, made at the first run.
    converter: Option<Converter>,
    /// The device's channels.
    channels: usize,
    /// Samples still to drop before Table 291's section begins, and still to keep before it ends.
    skip: usize,
    keep: usize,
    /// Frames handed to the sink.
    sent: u64,
    /// The sentence for the window, made at the first run.
    said: String,
    /// How many of the worker's notes the sentence carries.
    noted: usize,
    /// Why the sound cannot go on.
    failed: Option<String>,
    /// Whether the sound is in hand: its first pass started, or held.
    begun: bool,
    /// Whether an order means it is not to go on.
    superseded: bool,
}

/// The player's thread's state.
struct Thread {
    inbox: Receiver<Order>,
    /// A sender to its own inbox, which each pass's report posts through.
    ended: Sender<Order>,
    say: Sender<Said>,
    wake: Wake,
    open: OpenSink,
    media: MediaWorker,
    sink: Option<Box<dyn Sink>>,
    sounding: Option<Sounding>,
    serial: u64,
    /// The clock of the sound in hand, shared with [`Player::position`].
    clock: Arc<Mutex<Option<MediaClock>>>,
    /// Which sound the clock is of, shared with each [`Listening`].
    listened: Arc<Mutex<Listened>>,
    /// The ticket of the sound the last play order asked for.
    ticket: u64,
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
                Order::Operate {
                    document,
                    annotation,
                    operation,
                } => self.operated(document, annotation, operation),
                Order::Progress(serial, progress) => self.progressed(serial, progress),
                Order::Play(played, ticket) => {
                    self.ticket = ticket;
                    if self.resumes_instead(&played) {
                        // The sound resumed is the one this ticket listens to.
                        self.listen(|listened| {
                            listened.ticket = ticket;
                        });
                        if self.sounding.is_none() {
                            self.listen(|listened| listened.over = ticket);
                        }
                        continue;
                    }
                    self.silence();
                    self.streamed(&played, &mut queued);
                    if self.sounding.is_none() {
                        // Refused, or over before this order was done with: nothing to wait for.
                        self.listen(|listened| listened.over = listened.over.max(ticket));
                    }
                }
            }
        }
    }

    /// An order read between two runs of a sound still arriving, which is not one that ends it.
    fn obeyed(&mut self, order: &Order) {
        match order {
            Order::Resume => self.resumed(),
            Order::Operate {
                document,
                annotation,
                operation,
            } => self.operated(*document, *annotation, *operation),
            Order::Progress(serial, progress) => self.progressed(*serial, *progress),
            Order::Stop => self.silence(),
            Order::Closed(document) => {
                if self
                    .sounding
                    .as_ref()
                    .is_some_and(|sounding| sounding.document == *document)
                {
                    self.silence();
                }
            }
            // Both end the sound arriving, and are queued rather than read here.
            Order::Play(..) | Order::Quit => {}
        }
    }

    /// `played` decoded in the worker a run at a time, each run converted and handed to the sink as
    /// it arrives, and its first pass started at the first frames (ADR 3457). Orders that arrive
    /// meanwhile are read between runs: a stop, a newer sound or its document closing ends the
    /// sound there and is queued for the loop; any other is obeyed at once.
    fn streamed(&mut self, played: &Played, queued: &mut VecDeque<Order>) {
        let mut feed = Feed::default();
        // The worker is lent to the decode so that each run can reach the rest of the thread.
        let mut media = std::mem::take(&mut self.media);
        let mut deliver = |run: Run<'_>| self.fed(played, &mut feed, &run, queued);
        let decoded = match &played.sound.recording {
            Recording::Samples { format, data } => media.decode_streamed(
                &worker_format(*format),
                data,
                MAX_PLAYED_SAMPLES,
                &mut deliver,
            ),
            // A rendition's clip: the worker reads the file's header too (ADR 3349).
            Recording::Wave(file) => {
                media.decode_wave_streamed(file, MAX_PLAYED_SAMPLES, &mut deliver)
            }
            // An MP4 clip's AAC: the worker walks the file's boxes and decodes its stream, and this
            // side cuts the file where it is told (ADR 3381).
            // A clip with pictures sounds its AAC here too; the pictures are `crate::video`'s
            // (ADR 3538).
            Recording::Mp4(file) | Recording::Mp4Video(file) => {
                media.decode_mp4_streamed(file, MAX_PLAYED_SAMPLES, &mut deliver)
            }
        };
        self.media = media;
        if feed.superseded {
            return;
        }
        let why = match decoded {
            Err(error) => error.to_string(),
            Ok(_) if feed.failed.is_some() => feed.failed.take().unwrap_or_default(),
            Ok(streamed) => match self.completed(played, &mut feed) {
                Ok(()) => {
                    let later: Vec<&str> = streamed
                        .notes
                        .iter()
                        .skip(feed.noted)
                        .map(String::as_str)
                        .collect();
                    if !later.is_empty() && self.sounding.is_some() {
                        self.tell(Heard::Unchanged(format!(
                            "{}: {}",
                            played.label,
                            later.join("; ")
                        )));
                    }
                    return;
                }
                Err(why) => why,
            },
        };
        if feed.begun {
            self.silence_saying(&format!(", because {why}"));
        } else {
            if let Some(sink) = self.sink.as_mut() {
                sink.stop();
            }
            self.tell(Heard::Refused(format!(
                "{} was not played: {why}",
                played.label
            )));
        }
    }

    /// One run of `played` handed on: converted, levelled, cut to the section and given to the
    /// sink; the sound begun at its first frames; then the orders that arrived meanwhile read.
    fn fed(
        &mut self,
        played: &Played,
        feed: &mut Feed,
        run: &Run<'_>,
        queued: &mut VecDeque<Order>,
    ) -> ControlFlow<()> {
        if feed.failed.is_some() || feed.superseded {
            return ControlFlow::Break(());
        }
        let mut converted = Vec::new();
        let ready = match feed.converter.as_mut() {
            Some(conversion) => conversion
                .push(run.samples, &mut converted)
                .map_err(|error| error.to_string()),
            None => self.first_run(played, feed, run).and_then(|()| {
                feed.converter.as_mut().map_or(Ok(()), |converter| {
                    converter
                        .push(run.samples, &mut converted)
                        .map_err(|error| error.to_string())
                })
            }),
        };
        if let Err(why) = ready.and_then(|()| self.handed(played, feed, converted)) {
            feed.failed = Some(why);
            return ControlFlow::Break(());
        }
        // A first pass whose length is the sound's own and only part of it — a repeat count below
        // one of the intrinsic duration — cannot be cut before the sound's length is known.
        let playing = played.sound.playing;
        let waits = matches!(playing.duration, SimpleDuration::Intrinsic)
            && playing.repeat > 0.0
            && playing.repeat < 1.0;
        // A sound of one run is taken whole, as it arrived whole.
        if !feed.begun && feed.sent > 0 && !waits && !run.last {
            self.begin(played, feed, None);
        }
        let arrived: Vec<Order> = self.inbox.try_iter().collect();
        for order in arrived {
            if feed.superseded || supersedes(&order, played.document) {
                feed.superseded = true;
                queued.push_back(order);
            } else if feed.begun {
                self.obeyed(&order);
            } else {
                queued.push_back(order);
            }
        }
        // An operation that stopped it (§12.6.4.14's `/OP` 1) ends it as a stop does.
        if feed.begun && self.sounding.is_none() {
            feed.superseded = true;
        }
        if feed.superseded {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }

    /// The sink opened at the first sound, the sound's length checked against the bound where it
    /// is known, the conversion made, the sink told a new sound comes, the section placed, and the
    /// sentence for the window made.
    fn first_run(&mut self, played: &Played, feed: &mut Feed, run: &Run<'_>) -> Result<(), String> {
        if self.sink.is_none() {
            self.sink = Some((self.open)().map_err(|why| format!("the audio device: {why}"))?);
        }
        let Some(sink) = self.sink.as_mut() else {
            return Err("the audio device was not opened".to_owned());
        };
        let device = sink.format();
        // A sound whose length is known is refused before any of it sounds, as a whole one was.
        if let Some(frames) = run.at_most {
            let samples = samples_after(frames, run.rate, device);
            if usize::try_from(samples).map_or(true, |samples| samples > MAX_PLAYED_SAMPLES) {
                return Err(ConversionError::TooLarge {
                    samples,
                    limit: MAX_PLAYED_SAMPLES,
                }
                .to_string());
            }
        }
        feed.converter = Some(
            Converter::new(
                run.rate,
                run.channels,
                run.speakers,
                device,
                MAX_PLAYED_SAMPLES,
            )
            .map_err(|error| error.to_string())?,
        );
        sink.load()
            .map_err(|why| format!("the audio device: {why}"))?;
        let channels = usize::from(device.channels.max(1));
        let (begin, end) = sectioned(played.sound.playing.section, device.rate);
        feed.channels = channels;
        feed.skip = begin.saturating_mul(channels);
        feed.keep = end.map_or(usize::MAX, |end| {
            end.saturating_sub(begin).saturating_mul(channels)
        });
        // §13.3 for a sound object's samples, §13.2.4.2 for a rendition's clip.
        let clause = match played.sound.recording {
            Recording::Samples { .. } => "§13.3",
            Recording::Wave(_) => "§13.2.4.2, a RIFF WAVE clip",
            Recording::Mp4(_) => "§13.2.4.2, an MP4 clip's AAC",
            Recording::Mp4Video(_) => "§13.2.4.2, the AAC beside an MP4 clip's pictures",
        };
        let mut said = format!(
            "{} is playing: {} Hz, {} channel(s), converted for the device's {} Hz, {} channel(s) \
             (ISO 32000-2 {clause})",
            played.label, run.rate, run.channels, device.rate, device.channels
        );
        let speakers = speakers_said(run.speakers, run.channels, device);
        for note in run.notes.iter().chain(&speakers) {
            said.push_str("; ");
            said.push_str(note);
        }
        feed.noted = run.notes.len();
        for note in described(&played.sound.playing) {
            said.push_str("; ");
            said.push_str(&note);
        }
        feed.said = said;
        Ok(())
    }

    /// Converted samples levelled, cut to the section, and handed to the sink.
    fn handed(
        &mut self,
        played: &Played,
        feed: &mut Feed,
        mut samples: Vec<f32>,
    ) -> Result<(), String> {
        let dropped = feed.skip.min(samples.len());
        feed.skip = feed.skip.saturating_sub(dropped);
        samples.drain(..dropped);
        samples.truncate(feed.keep);
        feed.keep = feed.keep.saturating_sub(samples.len());
        if samples.is_empty() {
            return Ok(());
        }
        levelled(&mut samples, played.sound.playing.level);
        let Some(sink) = self.sink.as_mut() else {
            return Err("the audio device was not opened".to_owned());
        };
        sink.append(&samples)
            .map_err(|why| format!("the audio device: {why}"))?;
        let frames = samples.len().checked_div(feed.channels).unwrap_or_default();
        feed.sent = feed
            .sent
            .saturating_add(u64::try_from(frames).unwrap_or(u64::MAX));
        Ok(())
    }

    /// The sound's last runs handed on: the conversion's last frames sent, the sink told the
    /// sound is complete, and its length — now known — given to its passes, or the sound begun
    /// where it had not been.
    fn completed(&mut self, played: &Played, feed: &mut Feed) -> Result<(), String> {
        let mut last = Vec::new();
        if let Some(converter) = feed.converter.as_mut() {
            converter
                .finish(&mut last)
                .map_err(|error| error.to_string())?;
        }
        self.handed(played, feed, last)?;
        if let Some(sink) = self.sink.as_mut() {
            sink.complete()
                .map_err(|why| format!("the audio device: {why}"))?;
        }
        let intrinsic = feed.sent;
        if !feed.begun {
            self.begin(played, feed, Some(intrinsic));
            return Ok(());
        }
        let Some(sounding) = self.sounding.as_mut() else {
            return Ok(());
        };
        sounding.intrinsic = Some(intrinsic);
        sounding.passes.learned(intrinsic);
        // The pass at the device was started with its provisional length, which the device ends
        // at the sound's end; restated, it is the length its end and the clock count.
        sounding.pass = sounding.passes.restated(sounding.pass, sounding.from);
        Ok(())
    }

    /// The sound in hand from now: cut into its passes — provisionally where its length is not yet
    /// known, which [`Passes::learned`] corrects — its clock at zero, and its first pass started,
    /// or held where it does not play at once.
    fn begin(&mut self, played: &Played, feed: &mut Feed, intrinsic: Option<u64>) {
        feed.begun = true;
        let playing = played.sound.playing;
        let Some(format) = self.sink.as_ref().map(|sink| sink.format()) else {
            return;
        };
        let mut passes = Passes::new(&playing, intrinsic.unwrap_or(u64::MAX), format.rate);
        let Some(pass) = passes.next_pass() else {
            self.tell(Heard::Ended(format!(
                "{} has a duration of nothing (Table 292, Table 291's /RC), so nothing sounded",
                played.label
            )));
            return;
        };
        *self.clock.lock().unwrap_or_else(PoisonError::into_inner) =
            Some(MediaClock::new(format.rate));
        let ticket = self.ticket;
        self.listen(|listened| listened.ticket = ticket);
        self.sounding = Some(Sounding {
            serial: 0,
            document: played.document,
            label: played.label.clone(),
            passes,
            pass,
            from: 0,
            intrinsic,
            rendition: played.sound.rendition,
            stage: Stage::Held,
            synchronous: playing.synchronous,
        });
        self.paint(played.sound.screen.map(|screen| (played.document, screen)));
        let mut said = std::mem::take(&mut feed.said);
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
        let past_the_end = sounding
            .intrinsic
            .is_some_and(|intrinsic| sounding.from >= intrinsic);
        if sounding.pass.sounding == 0 || past_the_end {
            sounding.stage = Stage::Frozen;
            self.with_clock(|clock| clock.freeze(Instant::now()));
            return Ok(());
        }
        sounding.stage = Stage::Sounding;
        sink.start(
            sounding.from,
            sounding.pass.sounding,
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
            Progress::Heard(frames) => self.with_clock(|clock| clock.heard(frames, Instant::now())),
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
                sounding.from = 0;
                if let Err(why) = self.start_pass() {
                    self.refused(&why);
                }
            }
            None => {
                if let Some(sounding) = self.sounding.take() {
                    self.clock_over();
                    self.paint(None);
                    self.tell(Heard::Ended(format!(
                        "{} came to its end (ISO 32000-2 §13.3, §13.2.5)",
                        sounding.label
                    )));
                }
            }
        }
    }

    /// The sound held at its start, started; or the sound paused, started again where it stopped.
    /// Nothing where neither is.
    fn resumed(&mut self) {
        let Some(sounding) = self.sounding.as_ref() else {
            return;
        };
        let said = match sounding.stage {
            Stage::Held => format!("{} is playing, resumed from its start", sounding.label),
            Stage::Paused => {
                let at = self.position_said();
                format!(
                    "{} is playing, resumed where it was paused, at {at} (ISO 32000-2 \
                     §12.6.4.14, Table 218's /OP 3)",
                    sounding.label
                )
            }
            Stage::Sounding | Stage::Frozen => return,
        };
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
            self.clock_over();
            self.paint(None);
            self.tell(Heard::Refused(format!(
                "{} was not played: the audio device: {why}",
                sounding.label
            )));
        }
    }

    /// Stops what sounds, and says so where something did.
    fn silence(&mut self) {
        self.silence_saying("");
    }

    /// [`Self::silence`], with `why` after the sentence.
    fn silence_saying(&mut self, why: &str) {
        if let Some(sink) = self.sink.as_mut() {
            sink.stop();
        }
        if let Some(sounding) = self.sounding.take() {
            self.clock_over();
            self.paint(None);
            self.tell(Heard::Stopped(format!(
                "{} was stopped{why}",
                sounding.label
            )));
        }
    }

    /// Table 218's `/OP` 4 where its annotation's rendition is the sound in hand: "If a rendition is
    /// already associated with the annotation, resume the rendition if it is paused; otherwise, do
    /// nothing." Answers whether it was, in which case `played` is not played.
    fn resumes_instead(&mut self, played: &Played) -> bool {
        let Some(associated) = played
            .sound
            .rendition
            .filter(|associated| associated.or_resume)
        else {
            return false;
        };
        let Some(sounding) = self.associated(played.document, associated.annotation) else {
            return false;
        };
        if matches!(sounding.stage, Stage::Held | Stage::Paused) {
            self.resumed();
        } else {
            let label = sounding.label.clone();
            self.tell(Heard::Unchanged(format!(
                "{} was not played: {label} is already associated with the screen annotation {} \
                 {} R and is not paused, and Table 218's /OP 4 then does nothing (ISO 32000-2 \
                 §12.6.4.14)",
                played.label, associated.annotation.number, associated.annotation.generation
            )));
        }
        true
    }

    /// The sound in hand, where `document` played it for a rendition action naming `annotation`.
    fn associated(&self, document: DocumentId, annotation: ObjectId) -> Option<&Sounding> {
        self.sounding.as_ref().filter(|sounding| {
            sounding.document == document
                && sounding
                    .rendition
                    .is_some_and(|associated| associated.annotation == annotation)
        })
    }

    /// §12.6.4.14's stop, pause or resume, performed on the sound associated with `annotation`, or
    /// said to have no effect.
    fn operated(&mut self, document: DocumentId, annotation: ObjectId, operation: MediaOperation) {
        let Some(stage) = self
            .associated(document, annotation)
            .map(|sounding| sounding.stage)
        else {
            return self.tell(Heard::Unchanged(unassociated(annotation, operation)));
        };
        match (operation, stage) {
            (MediaOperation::Stop, _) => self.silence_saying(
                ", and its association with the screen annotation removed (ISO 32000-2 \
                 §12.6.4.14, Table 218's /OP 1)",
            ),
            (MediaOperation::Pause, Stage::Sounding | Stage::Frozen) => self.paused(),
            (MediaOperation::Resume, Stage::Held | Stage::Paused) => self.resumed(),
            (MediaOperation::Pause, Stage::Held | Stage::Paused) => {
                self.tell(Heard::Unchanged(not_in_state(
                    annotation,
                    operation,
                    "already paused",
                )));
            }
            (MediaOperation::Resume, Stage::Sounding | Stage::Frozen) => {
                self.tell(Heard::Unchanged(not_in_state(
                    annotation,
                    operation,
                    "not paused",
                )));
            }
        }
    }

    /// The sound in hand paused where it is: the device's stream stopped, the frames it had
    /// sounded of the pass — or the part of the freeze that had lasted — kept, and the rest held
    /// for [`Self::resumed`] (Table 218's `/OP` 2).
    fn paused(&mut self) {
        let now = Instant::now();
        // The device's own word on where it reached, which a report may have run ahead of by the
        // frames it held unplayed (ADR 3458).
        if self
            .sounding
            .as_ref()
            .is_some_and(|sounding| sounding.stage == Stage::Sounding)
            && let Some(reached) = self.sink.as_mut().and_then(|sink| sink.pause())
        {
            self.with_clock(|clock| clock.reached(reached));
        }
        let mut folded = (0, 0);
        self.with_clock(|clock| folded = clock.pause(now));
        let (sounded, frozen) = folded;
        let Some(sounding) = self.sounding.as_mut() else {
            return;
        };
        match sounding.stage {
            Stage::Sounding => {
                if let Some(sink) = self.sink.as_mut() {
                    sink.stop();
                }
                let sounded = sounded.min(sounding.pass.sounding);
                sounding.from = sounding.from.saturating_add(sounded);
                sounding.pass.sounding = sounding.pass.sounding.saturating_sub(sounded);
            }
            Stage::Frozen => {
                // The pass sounded whole; what is left is the freeze's remainder, or a freeze for
                // ever where it was one.
                sounding.pass.sounding = 0;
                sounding.pass.frozen = sounding
                    .pass
                    .frozen
                    .map(|frames| frames.saturating_sub(frozen));
            }
            Stage::Held | Stage::Paused => return,
        }
        sounding.stage = Stage::Paused;
        let label = sounding.label.clone();
        let at = self.position_said();
        self.tell(Heard::Paused(format!(
            "{label} is paused at {at} (ISO 32000-2 §12.6.4.14, Table 218's /OP 2); its screen \
             annotation keeps it, and the audio device is kept with no stream open"
        )));
    }

    /// Where the sound in hand is in its time, for a sentence.
    fn position_said(&self) -> String {
        let at = self
            .clock
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .map_or_default(|clock| clock.time(Instant::now()));
        format!("{:.1} s", at.as_secs_f64())
    }

    fn paint(&self, now: Option<(DocumentId, Screen)>) {
        if self.say.send(Said::Painted(now)).is_ok() {
            (self.wake)();
        }
    }

    /// The sound in hand is over: its clock goes, and a [`Listening`] to it hears that it is.
    fn clock_over(&self) {
        *self.clock.lock().unwrap_or_else(PoisonError::into_inner) = None;
        self.listen(|listened| listened.over = listened.over.max(listened.ticket));
    }

    fn listen(&self, change: impl FnOnce(&mut Listened)) {
        change(&mut self.listened.lock().unwrap_or_else(PoisonError::into_inner));
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
        if self.say.send(Said::Heard(heard)).is_ok() {
            (self.wake)();
        }
    }
}

/// The sentence for §12.6.4.14's `operation` where no sound this window has in hand is associated
/// with `annotation`: the clause's "there is no effect". A clip handed to this machine's player is
/// named, because that player may be playing it and this window cannot reach it (ADR 3417).
#[must_use]
pub fn unassociated(annotation: ObjectId, operation: MediaOperation) -> String {
    format!(
        "a rendition action's {} (Table 218's /OP {}) had no effect: no rendition this window \
         plays is associated with the screen annotation {} {} R, and a clip handed to this \
         machine's player is not within its reach (ISO 32000-2 §12.6.4.14)",
        operation.verb(),
        operation.op(),
        annotation.number,
        annotation.generation
    )
}

/// The sentence for §12.6.4.14's `operation` on a rendition that is `state`: "there is no effect".
fn not_in_state(annotation: ObjectId, operation: MediaOperation, state: &str) -> String {
    format!(
        "a rendition action's {} (Table 218's /OP {}) had no effect: the rendition associated \
         with the screen annotation {} {} R is {state} (ISO 32000-2 §12.6.4.14)",
        operation.verb(),
        operation.op(),
        annotation.number,
        annotation.generation
    )
}

/// Cuts `samples` — interleaved frames of `channels` at `rate` — to §13.2.4.3's `section`: from
/// its beginning to its end, or to the clip's own end where it states none or one past it. A
/// beginning at or past the end leaves nothing, which the clause calls "an empty range" and "valid"
/// (ADR 3398).
/// Table 291's section as frames of a device of `rate`: where it begins and, where it states one,
/// where it ends — each held to the other, and to the sound's own length by the runs that do not
/// reach it.
fn sectioned(section: pdf_model::media::Section, rate: u32) -> (usize, Option<usize>) {
    let frame_at = |seconds: f64| -> usize {
        let at = (seconds * f64::from(rate)).round();
        // A finite, non-negative position past `usize` is past every clip, which the runs hold.
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a non-negative float to integer `as` saturates, and the result is held to the clip's frames"
        )]
        let at = at as usize;
        at
    };
    let end = section.end.map(frame_at);
    let begin = end.map_or(frame_at(section.begin), |end| {
        frame_at(section.begin).min(end)
    });
    (begin, end)
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
        Order::Play(..) | Order::Stop | Order::Quit => true,
        Order::Closed(closed) => *closed == document,
        Order::Resume | Order::Operate { .. } | Order::Progress(..) => false,
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

    /// §13.2.4.3's section is placed on whole frames of the device: its beginning and its end,
    /// none where it states none, and a beginning past its end held to the end, which leaves
    /// nothing; the runs hold both to the clip's own length.
    #[test]
    fn a_section_is_placed_on_whole_frames_and_an_empty_range_leaves_none() {
        let placed = |begin: f64, end: Option<f64>| sectioned(Section { begin, end }, 4);
        assert_eq!(placed(0.0, None), (0, None));
        assert_eq!(placed(0.5, Some(1.0)), (2, Some(4)));
        assert_eq!(placed(2.0, None), (8, None));
        assert_eq!(placed(1.0, Some(99.0)), (4, Some(396)));
        assert_eq!(placed(2.0, Some(1.0)), (4, Some(4)));
    }
}
