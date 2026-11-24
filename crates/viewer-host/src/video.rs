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
//! clock of RFC 0015 Part 1 ([`MediaClock`], ADR 3342). A clip's pictures carry their presentation
//! times on the movie's timeline — composition offsets and the video edit list applied
//! (`pdf_media_worker::MediaWorker::decode_mp4_video_from`, ADR 3554); the window asks
//! [`VideoPlayer::due`] at each tick, which hands on the newest picture whose time has come and
//! counts every earlier one it passes over as **dropped late** — a slow window shows fewer pictures
//! and never a delayed one, and nothing the window does waits for the decoder. The decoding thread
//! is held back by a queue of [`AHEAD`] pictures, so it runs no further ahead of the clock than
//! that.
//!
//! **Whose clock.** A clip with sound is timed by its sound (ADR 3555): the pictures follow the
//! sound player's [`Listening`], the device's count of the frames it had reached — what is heard,
//! not what was written — so the two cannot drift apart. They wait while the sound has not begun,
//! stand while it is paused, and run on the monotonic clock from the sound's last time once it is
//! over (a sound shorter than its pictures) or where the clip never sounds.
//!
//! **What a play parameter does to the pictures.** Table 291's `/A` false holds the clip at its
//! start until it is resumed; §13.2.4.3's section places each pass on the movie's timeline and cuts
//! its pictures at the section's end; a simple duration (`/D`) shorter than the pictures cuts them
//! and a longer one holds the last picture up until it has elapsed; `/RC` counts the passes, a
//! fraction of one cutting the last (`timeline`, ADR 3593) — the sound's own cuts, on its clock.
//!
//! **Stop, pause and resume** reach the pictures as they reach a sound (ADR 3433): a pause stops
//! the decoding and keeps the clock's place; a resume decodes from the last sync sample at or
//! before that place — not from the track's start — and shows nothing before it (ADR 3555).

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

use crate::sound::clock::MediaClock;
use crate::sound::{Listening, SoundTime, Wake};

pub mod controller;
pub mod screens;
mod timeline;
mod window;

pub use controller::{BAR_HEIGHT, Controller, Pressed};
pub use window::{Looked, Pictures};

/// How many decoded pictures may wait for their time: enough that a picture is always in hand
/// when its time comes, few enough that a 1080-line clip holds 9 MiB of them.
pub const AHEAD: usize = 3;

/// The clock's rate: microseconds, which is finer than any picture rate a track states.
const CLOCK_RATE: u32 = 1_000_000;

/// How soon a clip waiting for its sound to begin is looked at again.
const WAITING_LOOK: Duration = Duration::from_millis(5);

/// How soon a window with no wake looks again for a picture the decoding thread may have queued:
/// a quarter of the shortest interval a clip of 60 pictures a second has, so a picture is put up
/// at most this late (ADR 3562).
pub const DECODING_LOOK: Duration = Duration::from_millis(4);

impl InHand {
    /// Where the pass's pictures ended, in its own time: one interval after the last picture, or
    /// the cut.
    fn pictures_end(&self) -> Duration {
        self.last_at.saturating_add(self.interval)
    }

    /// When the pass in hand ends, in its own time, once its pictures are over: `None` for an
    /// infinite duration, whose last picture is held until the clip is stopped.
    fn ends(&self) -> Option<Duration> {
        self.timeline.ends(self.pictures_end())
    }

    /// The clip's time at `now`, read and kept: its sound's heard time while the sound is in hand,
    /// `None` while the sound has not begun, and the clip's own clock otherwise — taken up from
    /// the sound's last time at the moment the sound is found over.
    fn clip_time(&mut self, now: Instant) -> Option<Duration> {
        if let Some(sound) = &self.sound {
            match sound.time(now) {
                SoundTime::At(time) => {
                    self.time = time;
                    return Some(time);
                }
                SoundTime::Waiting => return None,
                SoundTime::Over => {
                    self.sound = None;
                    self.clock = MediaClock::new(CLOCK_RATE);
                    self.clock
                        .thawed(u64::try_from(self.time.as_micros()).unwrap_or(u64::MAX));
                    if self.running {
                        self.clock.freeze(now);
                    }
                }
            }
        }
        self.time = self.clock.time(now);
        Some(self.time)
    }

    /// [`Self::clip_time`] without keeping anything, for a look that changes nothing.
    fn peek_time(&self, now: Instant) -> Option<Duration> {
        match self.sound.as_ref().map(|sound| sound.time(now)) {
            Some(SoundTime::At(time)) => Some(time),
            Some(SoundTime::Waiting) => None,
            Some(SoundTime::Over) => Some(self.time),
            None => Some(self.clock.time(now)),
        }
    }
}

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
    /// Table 291's `/RC`: zero for ever, otherwise the passes, a fraction of one cutting the last.
    pub repeat: f64,
    /// Table 291's `/D`, which each pass lasts (ADR 3593).
    pub duration: pdf_model::media::SimpleDuration,
    /// §13.2.4.3's section of the clip each pass plays (ADR 3593).
    pub section: pdf_model::media::Section,
    /// Table 307's `/ShowControls`: whether a controller bar is drawn over the pictures (ADR 3595).
    pub controls: bool,
    /// Table 291's `/F`: how a picture is fitted to the rectangle ([`placed`]).
    pub fit: pdf_model::media::Fit,
    /// Table 294's floating or full-screen window the pictures are shown in instead of a
    /// rectangle of the page — `/W` 0 or 1 — for a window that opens one (ADR 3587).
    pub window: Option<pdf_model::media::Window>,
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
            duration: playing.duration,
            section: playing.section,
            controls: playing.controls,
            fit: playing.fit,
            window: sound.window.clone(),
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

/// A picture placed in its rectangle: the part of it shown, in samples, and the quadrilateral in
/// window pixels that part covers, its corners clockwise from the part's top-left — the window
/// maps the part's top-left sample corner to `quad`'s first corner, its rows along the first edge
/// and its columns down the last.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    /// The first column of the picture shown.
    pub left: u32,
    /// The first row shown.
    pub top: u32,
    /// The first column past the part shown.
    pub right: u32,
    /// The first row past it.
    pub bottom: u32,
    /// Where the part goes, `[x0, y0, … x3, y3]`.
    pub quad: [f32; 8],
}

impl Placed {
    /// The affine from the picture's sample space — `(0, 0)` its top-left corner, one unit a
    /// sample — to window pixels: what puts [`Self::left`]..[`Self::right`] by
    /// [`Self::top`]..[`Self::bottom`] on [`Self::quad`], as `[a, b, c, d, e, f]` (§8.3.3's
    /// order: `x' = a·x + c·y + e`, `y' = b·x + d·y + f`).
    #[must_use]
    pub fn affine(&self) -> [f32; 6] {
        let [x0, y0, x1, y1, _, _, x3, y3] = self.quad;
        #[expect(
            clippy::cast_precision_loss,
            reason = "a picture's side is at most 1920 samples, far inside f32's exact integers"
        )]
        let (columns, rows) = (
            self.right.saturating_sub(self.left).max(1) as f32,
            self.bottom.saturating_sub(self.top).max(1) as f32,
        );
        let (a, b) = ((x1 - x0) / columns, (y1 - y0) / columns);
        let (c, d) = ((x3 - x0) / rows, (y3 - y0) / rows);
        #[expect(
            clippy::cast_precision_loss,
            reason = "as above: a sample index inside a picture's side"
        )]
        let (left, top) = (self.left as f32, self.top as f32);
        [a, b, c, d, x0 - a * left - c * top, y0 - b * left - d * top]
    }
    /// The samples of `raster` — the picture converted, RGBA as `render-cpu`'s conversion answers
    /// it — that this placement shows, for a window that draws the part as an image of its own.
    #[must_use]
    pub fn part_of(&self, raster: &pdf_render::Raster) -> pdf_render::Raster {
        let width = usize::try_from(raster.width).unwrap_or(usize::MAX);
        let to = |value: u32| usize::try_from(value).unwrap_or(usize::MAX);
        let (left, right) = (to(self.left).min(width), to(self.right).min(width));
        let (top, bottom) = (to(self.top), to(self.bottom).min(to(raster.height)));
        let stride = width.saturating_mul(4).max(1);
        let mut data = Vec::new();
        for row in raster
            .data
            .chunks_exact(stride)
            .skip(top)
            .take(bottom.saturating_sub(top))
        {
            data.extend_from_slice(
                row.get(left.saturating_mul(4)..right.saturating_mul(4))
                    .unwrap_or_default(),
            );
        }
        pdf_render::Raster {
            width: u32::try_from(right.saturating_sub(left)).unwrap_or(0),
            height: u32::try_from(bottom.saturating_sub(top)).unwrap_or(0),
            format: raster.format,
            data,
        }
    }

    /// Whether the part shown is the whole of `raster`: meet, fill and the default show all of
    /// it, and a window then hands the picture on without [`Self::part_of`]'s copy.
    #[must_use]
    pub const fn is_whole(&self, raster: &pdf_render::Raster) -> bool {
        self.left == 0
            && self.top == 0
            && self.right >= raster.width
            && self.bottom >= raster.height
    }

    /// The bounding box of [`Self::quad`] in window pixels: its top-left corner and its extent
    /// rounded to whole pixels. A page turned by a quarter keeps its rectangle's box, and a window
    /// that draws the part upright fills it — GTK's construction, which Qt's window and the C
    /// ABI's take (ADRs 3562, 3578).
    #[must_use]
    pub fn bounding_box(&self) -> ((f32, f32), (u32, u32)) {
        let quad = self.quad;
        let xs = [quad[0], quad[2], quad[4], quad[6]];
        let ys = [quad[1], quad[3], quad[5], quad[7]];
        let left = xs.iter().copied().fold(f32::INFINITY, f32::min);
        let right = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let top = ys.iter().copied().fold(f32::INFINITY, f32::min);
        let bottom = ys.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a viewport extent, non-negative after max(0) and far inside u32's range"
        )]
        let extent = (
            (right - left).round().max(0.0) as u32,
            (bottom - top).round().max(0.0) as u32,
        );
        ((left, top), extent)
    }
}

/// A picture of `size` samples fitted to the rectangle `quad` (corners clockwise from its
/// top-left, in window pixels) as ISO 32000-2 Table 291's `/F` says — "the manner in which the
/// player shall treat a visual media type that does not exactly fit the rectangle in which it
/// plays" (ADR 3553):
///
/// - **0, meet**, and **5**, this player's default: scaled by one factor, the largest at which the
///   whole picture is inside the rectangle;
/// - **1, slice**: scaled by one factor, the smallest at which the rectangle is covered, and the
///   part past the rectangle not shown;
/// - **2, fill**: each side scaled to the rectangle's;
/// - **4, hidden**, and **3, scroll**, whose interface no window provides (the rendition judged it):
///   not scaled — one sample a window pixel, the reading of "not scaled" this player takes, since
///   the table names no unit — and cut to the rectangle.
///
/// Where the picture does not fill the rectangle, or is cut by it, it stands at the rectangle's
/// top-left corner (`pdf_model::media::Fit`'s choice). The rectangle's sides are measured along
/// its own edges, so a turned or mirrored view fits the same picture. A picture is taken to have
/// square samples. `None` where the rectangle or the picture has no extent.
#[must_use]
pub fn placed(fit: pdf_model::media::Fit, size: (u32, u32), quad: [f32; 8]) -> Option<Placed> {
    use pdf_model::media::Fit;
    let [x0, y0, x1, y1, _, _, x3, y3] = quad;
    let (across, down) = (
        f64::from(x1 - x0).hypot(f64::from(y1 - y0)),
        f64::from(x3 - x0).hypot(f64::from(y3 - y0)),
    );
    let (width, height) = (f64::from(size.0), f64::from(size.1));
    if !(across > 0.0 && down > 0.0 && width > 0.0 && height > 0.0) {
        return None;
    }
    // Each side's scale from samples to the rectangle's pixels.
    let (sx, sy) = match fit {
        Fit::Meet | Fit::PlayersDefault => {
            let scale = (across / width).min(down / height);
            (scale, scale)
        }
        Fit::Slice => {
            let scale = (across / width).max(down / height);
            (scale, scale)
        }
        Fit::Fill => (across / width, down / height),
        Fit::Hidden | Fit::Scroll => (1.0, 1.0),
    };
    // The part of the picture inside the rectangle, from its top-left corner, in whole samples
    // (a part's edge to the nearest sample), and the share of each side of the rectangle it covers.
    let shown = |side: f64, extent: f64, scale: f64| (extent / scale).min(side).round().max(1.0);
    let (columns, rows) = (shown(width, across, sx), shown(height, down, sy));
    let (u, v) = (
        (columns * sx / across).min(1.0),
        (rows * sy / down).min(1.0),
    );
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a share of the rectangle, between zero and one, scales a pixel coordinate"
    )]
    let (u, v) = (u as f32, v as f32);
    let corner = |s: f32, t: f32| {
        (
            x0 + s * (x1 - x0) + t * (x3 - x0),
            y0 + s * (y1 - y0) + t * (y3 - y0),
        )
    };
    let (q0, q1, q2, q3) = (
        corner(0.0, 0.0),
        corner(u, 0.0),
        corner(u, v),
        corner(0.0, v),
    );
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a count of samples, at least one and at most the picture's side"
    )]
    let (right, bottom) = (columns as u32, rows as u32);
    Some(Placed {
        left: 0,
        top: 0,
        right,
        bottom,
        quad: [q0.0, q0.1, q1.0, q1.1, q2.0, q2.1, q3.0, q3.1],
    })
}

/// What the decoding thread sends the window.
enum Decoded {
    /// A picture of pass `generation`, at its time on the track's clock.
    Picture {
        generation: u64,
        picture: pdf_media_worker::video::Picture,
        at: Duration,
    },
    /// Pass `generation`'s decode is over, and how: its last picture's time and the sentence for
    /// an edit list not applied, or why it stopped.
    Over {
        generation: u64,
        result: Result<(Duration, Option<String>), MediaError>,
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
    /// The clip's own clock, on the monotonic clock: the one the pictures run on where the clip is
    /// silent or its sound is over.
    clock: MediaClock,
    /// The clip's sound, which the pictures follow while it is in hand (ADR 3555).
    sound: Option<Listening>,
    /// The clip's time as last read, on whichever clock it runs.
    time: Duration,
    /// Whether the clock runs: false while paused or held at the start.
    running: bool,
    /// The pass being decoded, and where in the clip's time that pass begins.
    generation: u64,
    pass_start: Duration,
    /// The passes begun, counting this one.
    passes: u32,
    /// Where each pass begins on the movie's timeline, where its pictures are cut and how long it
    /// lasts (ADR 3593).
    timeline: timeline::Timeline,
    /// The last picture's size in samples, which [`VideoPlayer::placed`] fits.
    size: Option<(u32, u32)>,
    /// The first picture not yet due, kept from the queue.
    next: Option<(Duration, pdf_media_worker::video::Picture)>,
    /// The newest picture's time in the pass, and the interval before it: the pass's pictures end
    /// one interval after the last, which is shown for that long.
    last_at: Duration,
    interval: Duration,
    /// How this pass's decode ended, kept until its last picture's interval has run.
    over: Option<Result<(Duration, Option<String>), MediaError>>,
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

    /// Shows `shown`, a silent clip or one whose sound no player plays, stopping the clip in hand
    /// first (a play stops the one in hand, ADR 3433); its pictures run on the monotonic clock.
    pub fn play(&mut self, shown: Shown) {
        self.play_beside(shown, None);
    }

    /// Shows `shown` beside its sound, `sound` — what the sound player was handed from the same
    /// event ([`crate::sound::Player::listening`]) — whose heard time the pictures follow while it
    /// is in hand: they wait for it to begin, stand when it is paused, and run on the monotonic
    /// clock from where it was once it is over or where it never sounds (ADR 3555).
    pub fn play_beside(&mut self, shown: Shown, sound: Option<Listening>) {
        self.stop();
        if shown.screen.is_none() && shown.window.is_none() {
            self.seen.push(Seen::Refused(format!(
                "{}: its pictures are not shown, because the rendition plays them in a hidden \
                 window (Table 294's /W 2) or names no screen annotation; its sound plays",
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
        let auto_play = shown.auto_play;
        let file = Arc::clone(&shown.file);
        let timeline = timeline::Timeline::new(&pdf_model::media::Playing {
            duration: shown.duration,
            section: shown.section,
            repeat: shown.repeat,
            ..pdf_model::media::Playing::default()
        });
        let begin = timeline.begin();
        let unknown_fraction = timeline.first_fraction_unknown();
        self.in_hand = Some(InHand {
            shown,
            clock,
            sound,
            time: Duration::ZERO,
            size: None,
            running: auto_play,
            generation: 0,
            pass_start: Duration::ZERO,
            passes: 1,
            timeline,
            next: None,
            last_at: Duration::ZERO,
            interval: Duration::ZERO,
            over: None,
            shown_count: 0,
            dropped: 0,
        });
        let generation = self.decode(file, begin);
        if let Some(hand) = self.in_hand.as_mut() {
            hand.generation = generation;
        }
        // A repeat count below one under the intrinsic duration cuts a pass whose length only its
        // own decode teaches, so its pictures are shown whole and that is said (trap 5).
        let caveat = if unknown_fraction {
            "; its repeat count is less than one pass, whose length is not known until its \
             pictures have been decoded, so they are shown whole"
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
        let now = Instant::now();
        let _ = hand.clip_time(now);
        hand.clock.pause(now);
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
        let from = hand
            .clip_time(now)
            .unwrap_or(hand.time)
            .saturating_sub(hand.pass_start)
            .saturating_add(hand.timeline.begin());
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

    /// The document, the label and the window of its own the pictures are shown in, while a clip
    /// that Table 294's `/W` 0 or 1 shows in one is in hand (ADR 3587).
    #[must_use]
    pub fn window(&self) -> Option<(DocumentId, &str, &pdf_model::media::Window)> {
        let hand = self.in_hand.as_ref()?;
        Some((
            hand.shown.document,
            hand.shown.label.as_str(),
            hand.shown.window.as_ref()?,
        ))
    }

    /// What a window of its own shows for the clip in hand, `inner` pixels of it: Table 294's
    /// `/B` over the whole — "used if the media object does not entirely cover the rectangle" —
    /// and `raster`, the picture held up, over it, fitted by Table 291's `/F` and smoothed as the page's
    /// layers are; the colour alone before the first picture. `None` where no clip shown in a
    /// window is in hand (ADR 3587).
    ///
    /// Drawn by the processor, the oracle's rasteriser, for any toolkit window: GTK holds it as a
    /// texture and `quorra` presents it through its software surface. A window's few pictures a
    /// second at its own size are the processor's to draw; the page's layer stays the device's.
    ///
    /// # Errors
    ///
    /// The rasteriser's refusal, as a sentence.
    #[must_use]
    pub fn window_picture(
        &self,
        raster: Option<&pdf_render::Raster>,
        inner: (u32, u32),
    ) -> Option<Result<pdf_render::Raster, String>> {
        use pdf_render::{
            BlendMode, Color, Command, DisplayList, Image, ImageSource, Medium, Rasterizer as _,
            SampleAlpha, Size, TargetSpec, Transform,
        };
        let (_, _, window) = self.window()?;
        let [r, g, b] = window.colour;
        #[expect(
            clippy::cast_precision_loss,
            reason = "a window's pixels are far below f32's exact integer range"
        )]
        let mut list = DisplayList::new(Size::new(inner.0 as f32, inner.1 as f32));
        if let Some((raster, placed)) = raster.zip(self.placed(screens::whole(inner))) {
            let part = placed.part_of(raster);
            let [x0, y0, x1, y1, _, _, x3, y3] = placed.quad;
            list.push(Command::Image {
                image: ImageSource::Decoded(Image {
                    width: part.width,
                    height: part.height,
                    data: Arc::from(part.data.as_slice()),
                    interpolate: true,
                    sample_alpha: SampleAlpha::Shape,
                }),
                // §8.9.5's image space puts an image's first row at the unit square's `y = 1`, so
                // that edge goes to the quadrilateral's first side.
                transform: Transform {
                    a: x1 - x0,
                    b: y1 - y0,
                    c: x0 - x3,
                    d: y0 - y3,
                    e: x3,
                    f: y3,
                },
                alpha: 1.0,
                clip: None,
                mask: None,
                blend: BlendMode::Normal,
            });
        }
        let target = TargetSpec {
            width: inner.0,
            height: inner.1,
            transform: Transform::IDENTITY,
        };
        Some(
            render_cpu::CpuRasterizer::new()
                .with_medium(Medium::uniform(Color { r, g, b, a: 1.0 }))
                .rasterize(&list, target)
                .map_err(|refused| refused.to_string()),
        )
    }

    /// Whether a clip is in hand, showing or paused: what Escape stops.
    #[must_use]
    pub fn in_hand(&self) -> bool {
        self.in_hand.is_some()
    }

    /// [`Self::next_due`] for a window the decoding thread cannot wake — Qt's, whose loop a wake
    /// would reach only through a second hand-written `unsafe` token (ADR 3562): while a pass is
    /// running and the decoding thread has queued nothing yet, a look at [`DECODING_LOOK`] after
    /// `now`, the queue being what the wake would have announced.
    #[must_use]
    pub fn next_due_unwoken(&self, now: Instant) -> Option<Instant> {
        self.next_due(now).or_else(|| {
            self.in_hand
                .as_ref()
                .filter(|hand| hand.running)
                .and_then(|_| now.checked_add(DECODING_LOOK))
        })
    }

    /// When the window should next ask [`Self::due`], for a window that sleeps until then: the
    /// next picture's time, or — once a pass's decode has ended — the moment its last picture has
    /// been up for its interval, when the pass ends. `None` where nothing is waiting for a time,
    /// and the decoding thread's wake is what comes next.
    #[must_use]
    pub fn next_due(&self, now: Instant) -> Option<Instant> {
        let hand = self.in_hand.as_ref().filter(|hand| hand.running)?;
        let at = match (&hand.next, &hand.over) {
            (Some((at, _)), _) => hand.timeline.in_pass(*at),
            // The pass's end has nothing after it to wake the window, so it is a time of its own;
            // an infinite duration's has none.
            (None, Some(_)) => hand.ends()?,
            (None, None) => return None,
        };
        // A clip waiting for its sound to begin is looked at again shortly: the sound's start
        // wakes nothing here.
        let Some(time) = hand.peek_time(now) else {
            return now.checked_add(WAITING_LOOK);
        };
        let wait = hand.pass_start.saturating_add(at).saturating_sub(time);
        now.checked_add(wait)
    }

    /// The newest picture whose time has come at `now`, every earlier one passed over and counted
    /// as dropped late; `None` where none has come since the last. Never waits.
    pub fn due(&mut self, now: Instant) -> Option<Frame> {
        let mut newest: Option<(Duration, pdf_media_worker::video::Picture)> = None;
        let mut over = None;
        let mut cut_decode = false;
        {
            let running = self.running.as_ref()?;
            let hand = self.in_hand.as_mut().filter(|hand| hand.running)?;
            // The pictures wait while the sound they follow has not begun.
            let clip_time = hand.clip_time(now)?.saturating_sub(hand.pass_start);
            let cut_at = hand.timeline.cut();
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
                let in_pass = hand.timeline.in_pass(at);
                // §13.2.4.3's section ends, or the pass's duration has run: this picture and every
                // later one of the pass are cut, its pictures end here, and the decode is stopped.
                if let Some(cut) = cut_at.filter(|cut| in_pass >= *cut) {
                    hand.interval = cut.saturating_sub(hand.last_at);
                    hand.over = Some(Ok((at, None)));
                    cut_decode = true;
                    break;
                }
                if in_pass > clip_time {
                    hand.next = Some((at, picture));
                    break;
                }
                hand.interval = in_pass.saturating_sub(hand.last_at);
                hand.last_at = in_pass;
                if newest.replace((at, picture)).is_some() {
                    hand.dropped = hand.dropped.saturating_add(1);
                }
            }
            // A refusal ends the clip at once; a decode that finished ends the pass once its last
            // picture has been up for its interval.
            let finished = match &hand.over {
                Some(Ok(_)) => hand.ends().is_some_and(|ends| clip_time >= ends),
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
        if cut_decode {
            self.retire();
        }
        if let Some(result) = over {
            self.pass_over(result);
        }
        let (at, picture) = newest?;
        let hand = self.in_hand.as_mut()?;
        hand.shown_count = hand.shown_count.saturating_add(1);
        let height = u32::try_from(picture.height).unwrap_or(u32::MAX);
        hand.size = Some((u32::try_from(picture.width).unwrap_or(u32::MAX), height));
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

    /// Where the clip's picture goes inside `quad`, the screen annotation in window pixels: the
    /// part of the picture shown and the quadrilateral it covers, by the clip's Table 291 `/F`
    /// ([`placed`]); `None` before the first picture or where nothing of it is shown.
    #[must_use]
    pub fn placed(&self, quad: [f32; 8]) -> Option<Placed> {
        let hand = self.in_hand.as_ref()?;
        placed(hand.shown.fit, hand.size?, quad)
    }

    /// What the controller bar shows at `now`, for a clip in hand that asks for one — Table 307's
    /// `/ShowControls` (ADR 3595) — and `None` otherwise: whether it runs, its place in its pass,
    /// and the pass's length where that is known.
    #[must_use]
    pub fn controller(&self, now: Instant) -> Option<Controller> {
        let hand = self.in_hand.as_ref().filter(|hand| hand.shown.controls)?;
        let time = if hand.running {
            hand.peek_time(now).unwrap_or(hand.time)
        } else {
            hand.time
        };
        let length = hand
            .timeline
            .length()
            .or_else(|| hand.over.as_ref().and_then(|_| hand.ends()));
        let position = time.saturating_sub(hand.pass_start);
        Some(Controller {
            playing: hand.running,
            position: length.map_or(position, |length| position.min(length)),
            length,
        })
    }

    /// The controller bar over `quad`, the clip's rectangle in window pixels, and the
    /// quadrilateral along its bottom the bar covers, where the clip in hand asks for one (ADR
    /// 3595).
    #[must_use]
    pub fn controller_bar(
        &self,
        quad: [f32; 8],
        now: Instant,
    ) -> Option<(pdf_render::Raster, [f32; 8])> {
        let controller = self.controller(now)?;
        let (bar, width) = controller::over(quad)?;
        Some((controller.bar(width)?, bar))
    }

    /// A press on the controller bar at `x` pixels from its left edge ([`Controller::pressed`]):
    /// the button pauses a running clip and resumes a paused or held one. Answers whether it acted.
    pub fn press(&mut self, x: u32) -> bool {
        if self.controller(Instant::now()).is_none() {
            return false;
        }
        match Controller::pressed(x) {
            Pressed::PlayPause => self.pause() || self.resume(),
            Pressed::Bar => false,
        }
    }

    /// Whether the window should draw the picture it holds: a clip is in hand. A clip that ended,
    /// was stopped or refused leaves the rectangle to the page.
    #[must_use]
    pub fn holds_a_picture(&self) -> bool {
        self.in_hand.is_some()
    }

    /// A pass's decode is over: the next pass, the clip's end, or the reason it stopped.
    fn pass_over(&mut self, result: Result<(Duration, Option<String>), MediaError>) {
        let Some(hand) = self.in_hand.as_mut() else {
            return;
        };
        let label = hand.shown.label.clone();
        match result {
            Ok((_, note)) => {
                if let Some(note) = note.filter(|_| hand.passes == 1) {
                    self.seen.push(Seen::Showing(format!("{label}: {note}")));
                }
                let Some(hand) = self.in_hand.as_mut() else {
                    return;
                };
                // §13.2.5's repeat count, a fraction of a pass cutting the last (ADR 3593).
                let pictures_end = hand.pictures_end();
                let ends = hand.ends();
                if hand.timeline.pass_over(pictures_end) {
                    hand.passes = hand.passes.saturating_add(1);
                    // The next pass begins where this one ended — its pictures' end, or its
                    // duration where the last picture was held up for the rest of it.
                    hand.pass_start = hand.pass_start.saturating_add(ends.unwrap_or(pictures_end));
                    hand.next = None;
                    hand.last_at = Duration::ZERO;
                    hand.interval = Duration::ZERO;
                    let file = Arc::clone(&hand.shown.file);
                    let begin = hand.timeline.begin();
                    let generation = self.decode(file, begin);
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
    // The decode begins at the sync sample before `from` (ADR 3555); the pictures between it and
    // `from` are decoded and not sent.
    let result = media.decode_mp4_video_from(&order.file, order.from, &mut |picture, at| {
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
            result: result.map(|streamed| (last, streamed.note)),
        });
        wake();
    }
}

#[cfg(test)]
mod tests;
