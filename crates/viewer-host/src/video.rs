//! A media rendition's H.264 pictures shown in the screen annotation's rectangle: the frame sink's
//! host half (RFC 0015 section 6, rounds 5 and 6; ADR 3538).
//!
//! **What is decoded where.** The clip's file goes to the confined media worker, which decodes its
//! first enabled H.264 track and writes each picture into the shared ring (ADRs 3513, 3526, 3527);
//! a thread of this player's own waits on it, so the window never does. The window takes each
//! picture when it is due and converts it where it draws: on the graphics device by
//! `render_raster::VideoSink`, kept there as a layer, or on the processor by
//! [`on_the_processor`], the oracle's own conversion (ADR 3529).
//!
//! **When a picture is due.** ISO 32000-2 §13.2.5's play parameters are timed against the media
//! clock of RFC 0015 Part 1 ([`MediaClock`], ADR 3342). A clip's pictures carry the times
//! `pdf_media_worker::MediaWorker::decode_mp4_video_timed` gives them, on the track's own clock;
//! the window asks [`VideoPlayer::due`] at each tick, which hands on the newest picture whose time
//! has come and counts every earlier one it passes over as **dropped late** — a slow window shows
//! fewer pictures and never a delayed one, and nothing the window does waits for the decoder. The
//! decoding thread is held back by a queue of [`AHEAD`] pictures, so it runs no further ahead of
//! the clock than that.
//!
//! **What is approximate until §6 round 9.** The pictures' clock runs on the monotonic clock from
//! the moment the clip started, as `MediaClock`'s freeze runs, while the clip's sound — played by
//! [`crate::sound::Player`] from the same file — runs on its device's count. The two start
//! together and are not locked to each other, so they drift by the two clocks' difference and by
//! the sound device's start-up latency. A track's composition offsets (`ctts`) and its edit list
//! are not read, so a picture is timed at its sample's decoding time.
//!
//! **What a play parameter does to the pictures.** Table 291's `/A` false holds the clip at its
//! start until it is resumed; `/RC` repeats it in whole passes (a fraction of a pass rounds up); a
//! simple duration (`/D`) is not applied to the pictures, nor a §13.2.4.3 section, and the
//! sentence for such a clip says so.
//!
//! **Stop, pause and resume** reach the pictures as they reach a sound (ADR 3433): a pause stops
//! the decoding and keeps the clock's place; a resume decodes the track again from its start and
//! shows nothing before that place, so a resumption costs the decoding of what was already shown.

use std::ops::ControlFlow;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, SyncSender, TryRecvError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use pdf_media_worker::{MediaError, MediaWorker};
use pdf_render::video::{Conversion, ConversionRefused, Planes};
use pdf_syntax::ObjectId;
use viewer_core::{DocumentId, MediaOperation};

use crate::sound::Wake;
use crate::sound::clock::MediaClock;

mod window;

pub use window::{Looked, Pictures};

/// How many decoded pictures may wait for their time: enough that a picture is always in hand
/// when its time comes, few enough that a 1080-line clip holds 9 MiB of them.
pub const AHEAD: usize = 3;

/// The clock's rate: microseconds, which is finer than any picture rate a track states.
const CLOCK_RATE: u32 = 1_000_000;

/// A clip to show, as the window was handed it in `viewer_core::Event::Media`'s sound.
#[derive(Debug, Clone)]
pub struct Shown {
    /// The document the clip belongs to.
    pub document: DocumentId,
    /// What the sentences call it.
    pub label: String,
    /// The MP4 file, read only by the media worker.
    pub file: Arc<[u8]>,
    /// The screen annotation whose rectangle the pictures are drawn over (Table 294's `/W` 3),
    /// `None` where the rendition plays in no rectangle of the page.
    pub screen: Option<ObjectId>,
    /// The annotation a rendition action associated the clip with, for Table 218's operations.
    pub associated: Option<ObjectId>,
    /// Table 291's `/A`: whether the clip starts at once.
    pub auto_play: bool,
    /// Table 291's `/RC`: zero for ever, otherwise the passes, a fraction rounded up.
    pub repeat: f64,
    /// Whether a simple duration or a section was stated, which the pictures do not follow yet.
    pub cut: bool,
}

impl Shown {
    /// The clip `sound` carries, where it is an MP4 file with pictures
    /// ([`viewer_core::Recording::Mp4Video`]) — what a window that sent
    /// `viewer_core::Command::ShowsVideoHere` is handed for a rendition it shows itself — with
    /// §13.2.5's play parameters and Table 294's rectangle as the event states them; `None` for
    /// every sound without pictures, which is the sound player's alone.
    #[must_use]
    pub fn of(document: DocumentId, label: String, sound: &viewer_core::SoundData) -> Option<Self> {
        let viewer_core::Recording::Mp4Video(file) = &sound.recording else {
            return None;
        };
        let playing = &sound.playing;
        Some(Self {
            document,
            label,
            file: Arc::from(file.as_slice()),
            screen: sound.screen.as_ref().map(|screen| screen.annotation),
            associated: sound
                .rendition
                .as_ref()
                .map(|rendition| rendition.annotation),
            auto_play: playing.auto_play,
            repeat: playing.repeat,
            cut: !matches!(
                playing.duration,
                pdf_model::media::SimpleDuration::Intrinsic
            ) || playing.section != pdf_model::media::Section::WHOLE,
        })
    }
}

/// What the player says, each a sentence for the terminal and the last for the title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seen {
    /// The clip's pictures are showing.
    Showing(String),
    /// The clip is paused, or held at its start by Table 291's `/A` false.
    Paused(String),
    /// The clip ended, with how many pictures it showed and how many it dropped late.
    Ended(String),
    /// The clip was stopped — Escape, a later clip, Table 218's `/OP` 1 or its document closed.
    Stopped(String),
    /// The clip's pictures cannot be shown here; the sentence says why. Its sound plays on.
    Refused(String),
    /// The clip's coding is one the media worker does not decode, and is the machine's player's.
    HandOn(String),
}

impl Seen {
    /// The sentence.
    #[must_use]
    pub fn sentence(&self) -> &str {
        match self {
            Self::Showing(said)
            | Self::Paused(said)
            | Self::Ended(said)
            | Self::Stopped(said)
            | Self::Refused(said)
            | Self::HandOn(said) => said,
        }
    }
}

/// One picture whose time has come, for the window to convert and draw.
#[derive(Debug)]
pub struct Frame {
    /// The picture's three planes.
    pub picture: pdf_media_worker::video::Picture,
    /// The conversion its stream's VUI states (ADR 3529).
    pub conversion: Conversion,
    /// Its time on the clip's clock.
    pub at: Duration,
}

impl Frame {
    /// The picture's planes, as both conversions take them.
    ///
    /// # Errors
    ///
    /// A picture whose size does not fit the planes' `u32`s, which the worker's checked size
    /// bound makes impossible and which is refused rather than wrapped.
    pub fn planes(&self) -> Result<Planes<'_>, ConversionRefused> {
        let refused = || ConversionRefused::Planes {
            width: u32::MAX,
            height: u32::MAX,
        };
        Ok(Planes {
            width: u32::try_from(self.picture.width).map_err(|_| refused())?,
            height: u32::try_from(self.picture.height).map_err(|_| refused())?,
            y: &self.picture.y,
            cb: &self.picture.u,
            cr: &self.picture.v,
        })
    }
}

/// The frame converted on the processor, `render-cpu`'s oracle conversion: what a window that
/// draws on no graphics device shows (ADR 3529).
///
/// # Errors
///
/// The planes refused by the conversion.
pub fn on_the_processor(frame: &Frame) -> Result<pdf_render::Raster, ConversionRefused> {
    render_cpu::video::convert_picture(&frame.planes()?, &frame.conversion)
}

/// The worker's statement of a stream's signal as the renderer's (both are the VUI's codes).
fn signal_of(signal: pdf_media_worker::video::Signal) -> pdf_render::video::Signal {
    pdf_render::video::Signal {
        full_range: signal.full_range,
        colour_primaries: signal.colour_primaries,
        transfer_characteristics: signal.transfer_characteristics,
        matrix_coefficients: signal.matrix_coefficients,
        chroma_sample_loc_type: signal.chroma_sample_loc_type,
    }
}

/// What the decoding thread sends the window.
enum Decoded {
    /// A picture of pass `generation`, at its time on the track's clock.
    Picture {
        generation: u64,
        picture: pdf_media_worker::video::Picture,
        at: Duration,
    },
    /// Pass `generation`'s decode is over, and how.
    Over {
        generation: u64,
        result: Result<Duration, MediaError>,
    },
}

/// What the window asks of the decoding thread: decode `file` as pass `generation`, sending no
/// picture before `from`.
struct Decode {
    generation: u64,
    file: Arc<[u8]>,
    from: Duration,
}

/// The decoding thread and the two directions of talk with it.
struct Running {
    orders: Sender<Decode>,
    decoded: Receiver<Decoded>,
    /// The pass the window wants; the thread stops any other at its next picture.
    wanted: Arc<AtomicU64>,
    thread: Option<JoinHandle<()>>,
}

/// The clip in hand, the window's side of it.
struct InHand {
    shown: Shown,
    clock: MediaClock,
    /// Whether the clock runs: false while paused or held at the start.
    running: bool,
    /// The pass being decoded, and where in the clip's time that pass begins.
    generation: u64,
    pass_start: Duration,
    /// The passes begun, counting this one.
    passes: u32,
    /// The first picture not yet due, kept from the queue.
    next: Option<(Duration, pdf_media_worker::video::Picture)>,
    /// The newest picture's time in the pass, and the interval before it: the pass ends one
    /// interval after its last picture, which is shown for that long.
    last_at: Duration,
    interval: Duration,
    /// How this pass's decode ended, kept until its last picture's interval has run.
    over: Option<Result<Duration, MediaError>>,
    /// The pictures handed on and the ones passed over late.
    shown_count: u64,
    dropped: u64,
}

/// The window's video player: one clip at a time, its pictures decoded on a thread of its own
/// that is started at the first clip and never at open (CLAUDE.md principle 2).
pub struct VideoPlayer {
    /// The waker, until the first clip takes it to the thread.
    unstarted: Option<Wake>,
    worker: Option<std::path::PathBuf>,
    running: Option<Running>,
    in_hand: Option<InHand>,
    generation: u64,
    seen: Vec<Seen>,
}

impl std::fmt::Debug for VideoPlayer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("VideoPlayer")
            .field("started", &self.running.is_some())
            .field(
                "showing",
                &self.in_hand.as_ref().map(|hand| &hand.shown.label),
            )
            .finish_non_exhaustive()
    }
}

impl VideoPlayer {
    /// A player that wakes the window with `wake` when a picture or a sentence arrives. Starts
    /// nothing now.
    #[must_use]
    pub fn new(wake: Wake) -> Self {
        Self {
            unstarted: Some(wake),
            worker: None,
            running: None,
            in_hand: None,
            generation: 0,
            seen: Vec::new(),
        }
    }

    /// The same, decoding with the worker `program` rather than the one beside this executable.
    #[must_use]
    pub fn with_worker(mut self, program: std::path::PathBuf) -> Self {
        self.worker = Some(program);
        self
    }

    /// Shows `shown`, stopping the clip in hand first (a play stops the one in hand, ADR 3433).
    pub fn play(&mut self, shown: Shown) {
        self.stop();
        if shown.screen.is_none() {
            self.seen.push(Seen::Refused(format!(
                "{}: its pictures are not shown, because the rendition plays in no screen \
                 annotation's rectangle and a floating window is not built yet; its sound plays",
                shown.label
            )));
            return;
        }
        let mut clock = MediaClock::new(CLOCK_RATE);
        let now = Instant::now();
        if shown.auto_play {
            clock.freeze(now);
        }
        let label = shown.label.clone();
        let cut = shown.cut;
        let auto_play = shown.auto_play;
        let file = Arc::clone(&shown.file);
        self.in_hand = Some(InHand {
            shown,
            clock,
            running: auto_play,
            generation: 0,
            pass_start: Duration::ZERO,
            passes: 1,
            next: None,
            last_at: Duration::ZERO,
            interval: Duration::ZERO,
            over: None,
            shown_count: 0,
            dropped: 0,
        });
        let generation = self.decode(file, Duration::ZERO);
        if let Some(hand) = self.in_hand.as_mut() {
            hand.generation = generation;
        }
        let caveat = if cut {
            "; its simple duration or section governs its sound, and its pictures play whole"
        } else {
            ""
        };
        self.seen.push(if auto_play {
            Seen::Showing(format!("{label}: showing its pictures{caveat}"))
        } else {
            Seen::Paused(format!(
                "{label}: held at its start (Table 291's /A false){caveat}"
            ))
        });
    }

    /// Starts a pass of `file` on the thread, started here at the first, and answers its number.
    fn decode(&mut self, file: Arc<[u8]>, from: Duration) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        if self.running.is_none() {
            let wake = self.unstarted.take().unwrap_or_else(|| Box::new(|| {}));
            self.running = Some(start(self.worker.clone(), wake));
        }
        let Some(running) = self.running.as_ref() else {
            return generation;
        };
        running.wanted.store(generation, Ordering::SeqCst);
        if running
            .orders
            .send(Decode {
                generation,
                file,
                from,
            })
            .is_err()
        {
            self.seen.push(Seen::Refused(
                "the video decoding thread has ended, so no picture is shown".to_owned(),
            ));
        }
        generation
    }

    /// Stops the clip in hand, if any, and says so.
    pub fn stop(&mut self) {
        if let Some(hand) = self.in_hand.take() {
            self.retire();
            self.seen.push(Seen::Stopped(format!(
                "{}: stopped after {} pictures, {} dropped late",
                hand.shown.label, hand.shown_count, hand.dropped
            )));
        }
    }

    /// The clip in hand's document closed: it stops.
    pub fn closed(&mut self, document: DocumentId) {
        if self
            .in_hand
            .as_ref()
            .is_some_and(|hand| hand.shown.document == document)
        {
            self.stop();
        }
    }

    /// Makes every pass the thread holds a stale one, and empties the queue so that a thread
    /// blocked on a full one sees it at its next picture.
    fn retire(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        if let Some(running) = &self.running {
            running.wanted.store(self.generation, Ordering::SeqCst);
            while running.decoded.try_recv().is_ok() {}
        }
    }

    /// Pauses the clip in hand; answers whether there was one sounding to pause.
    pub fn pause(&mut self) -> bool {
        let Some(hand) = self.in_hand.as_mut() else {
            return false;
        };
        if !hand.running {
            return false;
        }
        hand.clock.pause(Instant::now());
        hand.running = false;
        hand.next = None;
        hand.over = None;
        let said = format!("{}: paused", hand.shown.label);
        self.retire();
        self.seen.push(Seen::Paused(said));
        true
    }

    /// Resumes the clip in hand where it was paused or held; answers whether there was one.
    pub fn resume(&mut self) -> bool {
        let Some(hand) = self.in_hand.as_mut() else {
            return false;
        };
        if hand.running {
            return false;
        }
        let now = Instant::now();
        let from = hand.clock.time(now).saturating_sub(hand.pass_start);
        hand.clock.freeze(now);
        hand.running = true;
        hand.over = None;
        hand.next = None;
        let file = Arc::clone(&hand.shown.file);
        let said = format!("{}: showing its pictures again", hand.shown.label);
        let generation = self.decode(file, from);
        if let Some(hand) = self.in_hand.as_mut() {
            hand.generation = generation;
        }
        self.seen.push(Seen::Showing(said));
        true
    }

    /// Table 218's `/OP` 1, 2 or 3 on the clip associated with `annotation` in `document`; answers
    /// whether it acted, so that the window says the clause's "no effect" only where neither its
    /// sound nor its pictures did (ADR 3433).
    pub fn operate(
        &mut self,
        document: DocumentId,
        annotation: ObjectId,
        operation: MediaOperation,
    ) -> bool {
        let associated = self.in_hand.as_ref().is_some_and(|hand| {
            hand.shown.document == document && hand.shown.associated == Some(annotation)
        });
        if !associated {
            return false;
        }
        match operation {
            MediaOperation::Stop => {
                self.stop();
                true
            }
            MediaOperation::Pause => self.pause(),
            MediaOperation::Resume => self.resume(),
        }
    }

    /// The document and screen annotation the pictures are drawn over, while a clip is in hand.
    #[must_use]
    pub fn screen(&self) -> Option<(DocumentId, ObjectId)> {
        let hand = self.in_hand.as_ref()?;
        Some((hand.shown.document, hand.shown.screen?))
    }

    /// Whether a clip is in hand, showing or paused: what Escape stops.
    #[must_use]
    pub fn in_hand(&self) -> bool {
        self.in_hand.is_some()
    }

    /// When the window should next ask [`Self::due`], for a window that sleeps until then: the
    /// next picture's time, or — once a pass's decode has ended — the moment its last picture has
    /// been up for its interval, when the pass ends. `None` where nothing is waiting for a time,
    /// and the decoding thread's wake is what comes next.
    #[must_use]
    pub fn next_due(&self, now: Instant) -> Option<Instant> {
        let hand = self.in_hand.as_ref().filter(|hand| hand.running)?;
        let at = match (&hand.next, &hand.over) {
            (Some((at, _)), _) => *at,
            // The pass's end has nothing after it to wake the window, so it is a time of its own.
            (None, Some(_)) => hand.last_at.saturating_add(hand.interval),
            (None, None) => return None,
        };
        let wait = hand
            .pass_start
            .saturating_add(at)
            .saturating_sub(hand.clock.time(now));
        now.checked_add(wait)
    }

    /// The newest picture whose time has come at `now`, every earlier one passed over and counted
    /// as dropped late; `None` where none has come since the last. Never waits.
    pub fn due(&mut self, now: Instant) -> Option<Frame> {
        let mut newest: Option<(Duration, pdf_media_worker::video::Picture)> = None;
        let mut over = None;
        {
            let running = self.running.as_ref()?;
            let hand = self.in_hand.as_mut().filter(|hand| hand.running)?;
            let clip_time = hand.clock.time(now).saturating_sub(hand.pass_start);
            while hand.over.is_none() {
                let (at, picture) = match hand.next.take() {
                    Some(next) => next,
                    None => match running.decoded.try_recv() {
                        Ok(Decoded::Picture {
                            generation,
                            picture,
                            at,
                        }) if generation == hand.generation => (at, picture),
                        // The pass's end arrives after its last picture.
                        Ok(Decoded::Over { generation, result })
                            if generation == hand.generation =>
                        {
                            hand.over = Some(result);
                            break;
                        }
                        // A picture or an end of a pass the window has left.
                        Ok(Decoded::Picture { .. } | Decoded::Over { .. }) => continue,
                        Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
                    },
                };
                if at > clip_time {
                    hand.next = Some((at, picture));
                    break;
                }
                hand.interval = at.saturating_sub(hand.last_at);
                hand.last_at = at;
                if newest.replace((at, picture)).is_some() {
                    hand.dropped = hand.dropped.saturating_add(1);
                }
            }
            // A refusal ends the clip at once; a decode that finished ends the pass once its last
            // picture has been up for its interval.
            let finished = match &hand.over {
                Some(Ok(_)) => clip_time >= hand.last_at.saturating_add(hand.interval),
                Some(Err(_)) => true,
                None => false,
            };
            if finished {
                over = hand.over.take();
                // A picture taken in this look whose pass has already ended was up for none of
                // its interval: it is dropped late, and the end's counts include it.
                if newest.take().is_some() {
                    hand.dropped = hand.dropped.saturating_add(1);
                }
            }
        }
        if let Some(result) = over {
            self.pass_over(now, result);
        }
        let (at, picture) = newest?;
        let hand = self.in_hand.as_mut()?;
        hand.shown_count = hand.shown_count.saturating_add(1);
        let height = u32::try_from(picture.height).unwrap_or(u32::MAX);
        match Conversion::for_signal(signal_of(picture.signal), height) {
            Ok(conversion) => Some(Frame {
                picture,
                conversion,
                at,
            }),
            Err(refused) => {
                let label = hand.shown.label.clone();
                self.stop_saying(Seen::Refused(format!(
                    "{label}: its pictures are not shown, because {refused}"
                )));
                None
            }
        }
    }

    /// Whether the window should draw the picture it holds: a clip is in hand. A clip that ended,
    /// was stopped or refused leaves the rectangle to the page.
    #[must_use]
    pub fn holds_a_picture(&self) -> bool {
        self.in_hand.is_some()
    }

    /// A pass's decode is over: the next pass, the clip's end, or the reason it stopped.
    fn pass_over(&mut self, now: Instant, result: Result<Duration, MediaError>) {
        let Some(hand) = self.in_hand.as_mut() else {
            return;
        };
        let label = hand.shown.label.clone();
        match result {
            Ok(_) => {
                // §13.2.5's repeat count, in whole passes: zero repeats for ever.
                let repeat = hand.shown.repeat;
                let more = repeat <= 0.0 || f64::from(hand.passes) < repeat;
                if more {
                    hand.passes = hand.passes.saturating_add(1);
                    // The next pass begins where the clock stands as this one's last picture has
                    // been up for its interval.
                    hand.pass_start = hand.clock.time(now).max(hand.pass_start);
                    hand.next = None;
                    hand.last_at = Duration::ZERO;
                    hand.interval = Duration::ZERO;
                    let file = Arc::clone(&hand.shown.file);
                    let generation = self.decode(file, Duration::ZERO);
                    if let Some(hand) = self.in_hand.as_mut() {
                        hand.generation = generation;
                    }
                } else {
                    let said = format!(
                        "{label}: ended after {} pictures, {} dropped late",
                        hand.shown_count, hand.dropped
                    );
                    self.in_hand = None;
                    self.seen.push(Seen::Ended(said));
                }
            }
            Err(MediaError::Interlaced(why)) => self.stop_saying(Seen::HandOn(format!(
                "{label}: its pictures are field-coded ({why}), which no decoder this program \
                 admits decodes, so the clip is this machine's own player's"
            ))),
            Err(error) => self.stop_saying(Seen::Refused(format!(
                "{label}: its pictures are not shown, because {error}"
            ))),
        }
    }

    /// Ends the clip in hand with `seen` in place of the stop's own sentence.
    fn stop_saying(&mut self, seen: Seen) {
        self.in_hand = None;
        self.retire();
        self.seen.push(seen);
    }

    /// What the player has said since the last call.
    pub fn seen(&mut self) -> Vec<Seen> {
        std::mem::take(&mut self.seen)
    }
}

impl Drop for VideoPlayer {
    fn drop(&mut self) {
        self.retire();
        if let Some(mut running) = self.running.take() {
            let thread = running.thread.take();
            drop(running);
            if let Some(thread) = thread {
                // The thread ends at its next picture or when its orders close; a join that
                // failed is a panic already reported on the thread's own stderr.
                let _ = thread.join();
            }
        }
    }
}

/// Starts the decoding thread with a media worker of its own, which starts no process until its
/// first clip.
fn start(worker: Option<std::path::PathBuf>, wake: Wake) -> Running {
    let (orders, inbox) = std::sync::mpsc::channel::<Decode>();
    let (outbox, decoded) = std::sync::mpsc::sync_channel::<Decoded>(AHEAD);
    let wanted = Arc::new(AtomicU64::new(0));
    let watched = Arc::clone(&wanted);
    let thread = std::thread::Builder::new()
        .name("video".to_owned())
        .spawn(move || {
            let mut media = worker.map_or_else(MediaWorker::new, MediaWorker::with_program);
            for order in inbox {
                decode_pass(&mut media, &order, &watched, &outbox, &*wake);
            }
        })
        .ok();
    Running {
        orders,
        decoded,
        wanted,
        thread,
    }
}

/// One pass on the thread: each picture at or after `order.from` sent as it comes, the send
/// blocking while [`AHEAD`] wait, and the pass stopped at the first picture after the window
/// wants another.
fn decode_pass(
    media: &mut MediaWorker,
    order: &Decode,
    wanted: &AtomicU64,
    outbox: &SyncSender<Decoded>,
    wake: &(dyn Fn() + Send),
) {
    let generation = order.generation;
    let mut last = Duration::ZERO;
    let result = media.decode_mp4_video_timed(&order.file, &mut |picture, at| {
        if wanted.load(Ordering::SeqCst) != generation {
            return ControlFlow::Break(());
        }
        last = at;
        if at < order.from {
            return ControlFlow::Continue(());
        }
        if outbox
            .send(Decoded::Picture {
                generation,
                picture,
                at,
            })
            .is_err()
        {
            return ControlFlow::Break(());
        }
        wake();
        ControlFlow::Continue(())
    });
    if wanted.load(Ordering::SeqCst) == generation {
        let _ = outbox.send(Decoded::Over {
            generation,
            result: result.map(|_| last),
        });
        wake();
    }
}

#[cfg(test)]
mod tests;
