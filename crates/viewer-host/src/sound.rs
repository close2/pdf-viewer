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
//! The device is behind [`Sink`], the seam a window fills with its toolkit's output and a test with
//! a recording double. What crosses it is samples this program decoded, never the document's bytes,
//! which is the standing ADR 3297 gave the toolkits' C on the output side.

mod convert;

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;

pub use convert::{ConversionError, DeviceFormat, MAX_PLAYED_SAMPLES, converted};
use pdf_media_worker::MediaWorker;
use pdf_media_worker::sound::{Encoding, SoundFormat};
use viewer_core::{DocumentId, SoundData};

/// Where the converted samples go: a device, or a test's record of what reached it.
///
/// Made on the player's thread by [`OpenSink`] and never moved off it, so that a toolkit whose
/// stream may not cross threads can be one.
pub trait Sink {
    /// The device's rate and channel count, which each sound is converted to before
    /// [`Self::start`].
    fn format(&self) -> DeviceFormat;

    /// Starts `samples`, interleaved at [`Self::format`] as fractions of full scale, in place of
    /// whatever was sounding; `ended` is called once, from whichever thread the device runs on,
    /// when the last of them has been written.
    ///
    /// # Errors
    ///
    /// The sentence to say where the device would not start.
    fn start(&mut self, samples: Vec<f32>, ended: Box<dyn FnOnce() + Send>) -> Result<(), String>;

    /// Stops what is sounding; nothing where nothing is.
    fn stop(&mut self);
}

/// What opens the sink, called on the player's thread at the first sound and at no other time
/// while it stays open.
pub type OpenSink = Box<dyn FnMut() -> Result<Box<dyn Sink>, String> + Send>;

/// What wakes the window's loop when the player has something to say.
pub type Wake = Box<dyn Fn() + Send>;

/// One sound to play, from [`viewer_core::Event::Media`]'s `sound`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Played {
    /// Which document asked for it, so that closing that document stops it.
    pub document: DocumentId,
    /// The document's label and the sound's name, for the sentences.
    pub label: String,
    /// The sound object's samples as §13.3 stores them.
    pub sound: SoundData,
}

/// What the player has to say, taken by the window with [`Player::heard`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Heard {
    /// A sound is sounding; the sentence says which, and what it was converted to.
    Playing(String),
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
            Self::Playing(said) | Self::Ended(said) | Self::Stopped(said) | Self::Refused(said) => {
                said
            }
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
    /// The sound numbered so reached its end (sent by the sink's `ended`).
    Ended(u64),
    /// The window is closing.
    Quit,
}

/// The window's handle on its sounds: one at a time, on a thread of its own from the first.
///
/// A second sound stops the first, which is Table 210's `/Mix` at its default `false`: "If
/// `false`, stop any other sounds being played" is the behaviour this player has for every sound
/// until §13.2.5's play parameters are carried (RFC 0015 section 5, step 7).
pub struct Player {
    /// The sink's maker and the waker, until the first sound takes them to the thread.
    unstarted: Option<(OpenSink, Wake)>,
    /// The worker program, where the window names it; otherwise found beside this executable.
    worker: Option<PathBuf>,
    /// The thread, from the first sound on.
    running: Option<Running>,
    /// What was said before a thread existed to say it — a thread that would not start.
    early: Vec<Heard>,
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

    /// What the player has said since the last call, oldest first.
    pub fn heard(&mut self) -> Vec<Heard> {
        let mut heard = std::mem::take(&mut self.early);
        if let Some(running) = &self.running {
            heard.extend(running.heard.try_iter());
        }
        heard
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

/// What sounds now: its number, its document and its label.
struct Sounding {
    serial: u64,
    document: DocumentId,
    label: String,
}

/// The player's thread's state.
struct Thread {
    inbox: Receiver<Order>,
    /// A sender to its own inbox, which each sound's `ended` posts through.
    ended: Sender<Order>,
    say: Sender<Heard>,
    wake: Wake,
    open: OpenSink,
    media: MediaWorker,
    sink: Option<Box<dyn Sink>>,
    sounding: Option<Sounding>,
    serial: u64,
}

impl Thread {
    fn run(mut self) {
        let mut queued: VecDeque<Order> = VecDeque::new();
        loop {
            let order = match queued.pop_front() {
                Some(order) => order,
                None => match self.inbox.recv() {
                    Ok(order) => order,
                    Err(_) => return,
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
                Order::Ended(serial) => {
                    if self
                        .sounding
                        .as_ref()
                        .is_some_and(|sounding| sounding.serial == serial)
                        && let Some(sounding) = self.sounding.take()
                    {
                        self.tell(Heard::Ended(format!(
                            "{} came to its end (ISO 32000-2 §13.3)",
                            sounding.label
                        )));
                    }
                }
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
        let format = worker_format(played.sound.format);
        let decoded = self
            .media
            .decode(&format, &played.sound.data, MAX_PLAYED_SAMPLES)
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
            &decoded.samples,
            device,
            MAX_PLAYED_SAMPLES,
        )
        .map_err(|error| error.to_string())?;
        let mut said = format!(
            "{} is playing: {} Hz, {} channel(s), converted for the device's {} Hz, {} channel(s) \
             (ISO 32000-2 §13.3)",
            played.label, decoded.rate, decoded.channels, device.rate, device.channels
        );
        for note in &decoded.notes {
            said.push_str("; ");
            said.push_str(note);
        }
        Ok((samples, said))
    }

    fn sound(&mut self, played: Played, (samples, said): (Vec<f32>, String)) {
        self.serial = self.serial.wrapping_add(1);
        let serial = self.serial;
        let ended = self.ended.clone();
        let Some(sink) = self.sink.as_mut() else {
            return;
        };
        match sink.start(
            samples,
            Box::new(move || {
                let _ = ended.send(Order::Ended(serial));
            }),
        ) {
            Ok(()) => {
                self.sounding = Some(Sounding {
                    serial,
                    document: played.document,
                    label: played.label,
                });
                self.tell(Heard::Playing(said));
            }
            Err(why) => self.tell(Heard::Refused(format!(
                "{} was not played: the audio device: {why}",
                played.label
            ))),
        }
    }

    /// Stops what sounds, and says so where something did.
    fn silence(&mut self) {
        if let Some(sink) = self.sink.as_mut() {
            sink.stop();
        }
        if let Some(sounding) = self.sounding.take() {
            self.tell(Heard::Stopped(format!("{} was stopped", sounding.label)));
        }
    }

    fn tell(&self, heard: Heard) {
        if self.say.send(heard).is_ok() {
            (self.wake)();
        }
    }
}

/// Whether `order`, queued while a sound of `document` was being decoded, means that sound is not
/// to start.
fn supersedes(order: &Order, document: DocumentId) -> bool {
    match order {
        Order::Play(_) | Order::Stop | Order::Quit => true,
        Order::Closed(closed) => *closed == document,
        Order::Ended(_) => false,
    }
}

/// Table 305's entries as `pdf_model` read them, in the worker's vocabulary.
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
