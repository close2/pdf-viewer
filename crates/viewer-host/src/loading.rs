//! The document opening on a thread of its own, and the bar a window shows while it does.
//!
//! # What this is for
//!
//! The project owner, on a valid one-page file whose content stream holds twenty-nine million
//! operators: "As long as it isn't a resource problem (only a time problem) we should allow it,
//! and possible display a loading bar." Every window here opens its document on a thread and has
//! page one interpreted there before the window asks for it (ADRs 0182, 1531) — and then *joined*
//! that thread, so the window waited for the interpretation however long it took: fifteen seconds
//! with no repaint, no key and no word about why.
//!
//! [`Loading`] is that thread, shared by the three windows, and it changes one thing: **the wait
//! is bounded by [`WARN`] and the rest is a bar.** A window asks [`Loading::wait`] with what is
//! left of `WARN` since the thread began; a document that is ready inside it — every document of
//! the launch gate and of `doc/pdf.js`'s first pages, by `WARN`'s own measurement — is joined
//! the moment it is ready, as before — a channel's wake in place of a join's. A document that is
//! not ready goes on opening on its thread while the window answers its toolkit, looks again
//! every [`TICK`], and shows [`Loading::showing`]'s figure: how much of page one's `/Contents`
//! has been read, from `pdf_model::progress`.
//!
//! # What it is not
//!
//! **Not a deadline.** Nothing here ends the opening: ADR 0657 refused the frame nobody will look
//! at, and a page that takes a minute takes a minute with a bar in front of it. The person's way
//! out is the window's own close, which no longer waits for the page, because the window is
//! answering its toolkit. ADR 1811.
//!
//! [`WARN`]: crate::drawing::WARN

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use pdf_model::progress::{Handed, Parts, Progress, Reading};
use pdf_render::{DisplayList, Raster, TargetSpec};
use render_cpu::{CpuRasterizer, Underway};
use viewer_core::{Anticipating, Event, Foreseen, Viewer};

use crate::drawing::{GLIMPSE_AFTER, GLIMPSE_EVERY, WARN};

/// How often a window that is showing the bar looks at the thread again.
///
/// **Two 60 Hz refreshes**, bounded from both sides. From above: the look is also when the
/// finished page is noticed, so this is added at most once to a wait that is already longer than
/// [`WARN`] — three hundredths of a second at the least. From below: a bar that moves more often
/// than a person can read it moves for nobody, and a look costs one `try_recv` and one title or
/// label written. Nothing is armed for it outside a wait that has outlasted `WARN`.
pub const TICK: Duration = Duration::from_nanos(1_000_000_000 / 30);

/// A document opening on a thread of its own, with the figure of how far its first page has got.
#[derive(Debug)]
pub struct Loading<T> {
    /// The thread, until a window joins it.
    thread: JoinHandle<T>,
    /// Told when the thread's work has ended, however it ended.
    ended: Receiver<()>,
    /// Written by the interpretation, read by the window.
    progress: Progress,
    /// When the thread was started, which [`WARN`] is counted from.
    began: Instant,
}

/// Tells the window the work has ended, on the way out of the thread however it is left.
struct Ended(Sender<()>);

impl Drop for Ended {
    fn drop(&mut self) {
        // A window that has gone has nobody to tell.
        let _ = self.0.send(());
    }
}

impl<T: Send + 'static> Loading<T> {
    /// Starts `work` on a thread of its own, with a figure installed for it to publish to.
    ///
    /// # Panics
    ///
    /// Where the operating system refuses a thread, as `std::thread::spawn` does — the one
    /// failure every window here already treats as the end of the launch.
    #[must_use]
    pub fn start(work: impl FnOnce() -> T + Send + 'static) -> Self {
        Self::start_reporting(|_| work())
    }

    /// [`Self::start`], for work whose page is interpreted somewhere this thread's figure cannot
    /// see — a confined worker — and which is therefore handed the figure to report into.
    ///
    /// # Panics
    ///
    /// As [`Self::start`].
    #[must_use]
    pub fn start_reporting(work: impl FnOnce(Progress) -> T + Send + 'static) -> Self {
        let progress = Progress::new();
        let (tell, ended) = channel();
        let watched = progress.clone();
        let thread = std::thread::spawn(move || {
            let _ended = Ended(tell);
            let reported = watched.clone();
            watched.watch(move || work(reported))
        });
        Self {
            thread,
            ended,
            progress,
            began: Instant::now(),
        }
    }

    /// Waits for the work for at most what is left of [`WARN`], and answers whether it ended.
    ///
    /// **The launch path's one wait, and it is the wait every window already made** — for any
    /// document that is ready before `WARN` has passed, which is every document a launch gate
    /// measures, this returns the moment the thread ends, exactly as the join it replaces did.
    /// What changes is a document that is not ready: the window stops waiting at `WARN` and shows
    /// the bar instead of a frozen frame.
    #[must_use]
    pub fn wait(&self) -> bool {
        let left = WARN.saturating_sub(self.began.elapsed());
        match self.ended.recv_timeout(left) {
            Ok(()) | Err(RecvTimeoutError::Disconnected) => true,
            Err(RecvTimeoutError::Timeout) => false,
        }
    }

    /// [`Self::wait`], bounded by `bound` since the thread began where that is sooner than
    /// [`WARN`] — for a window that has something to show before `WARN`: page one as it is drawn
    /// while it is read, from [`GLIMPSE_AFTER`] ([`Ahead`]).
    #[must_use]
    pub fn wait_within(&self, bound: Duration) -> bool {
        let left = WARN.min(bound).saturating_sub(self.began.elapsed());
        match self.ended.recv_timeout(left) {
            Ok(()) | Err(RecvTimeoutError::Disconnected) => true,
            Err(RecvTimeoutError::Timeout) => false,
        }
    }

    /// Whether the work has ended, without waiting at all — what a window asks on each [`TICK`].
    #[must_use]
    pub fn ended(&self) -> bool {
        self.thread.is_finished()
    }

    /// What the work produced.
    ///
    /// Blocks only where the work has not ended, which a window that asked [`Self::wait`] or
    /// [`Self::ended`] first never meets.
    ///
    /// # Errors
    ///
    /// The thread's panic, as `JoinHandle::join` answers it.
    pub fn join(self) -> std::thread::Result<T> {
        self.thread.join()
    }

    /// How far the first page has got, once [`WARN`] has passed — `None` before, so that a window
    /// never shows a bar for a document that is about to arrive anyway.
    #[must_use]
    pub fn showing(&self) -> Option<Reading> {
        (self.began.elapsed() >= WARN).then(|| self.progress.read())
    }
}

/// What a window says while it waits for the page it opens at: how much of it has been read, and
/// how many operators that was.
///
/// **The share is of the page's own stream and the count is a count** (`pdf_model::progress` says
/// why the two differ), so the sentence never pretends the operators are a fraction. Before the
/// page's stream has begun there is no share, and the sentence says only that it is reading.
#[must_use]
pub fn reading(reading: Reading) -> String {
    match reading.fraction() {
        Some(fraction) => format!(
            "reading the page: {} of its contents, {} operators so far",
            percent(fraction),
            operators(reading.operators),
        ),
        None => "reading the document".to_owned(),
    }
}

/// What a window says once the page it showed a bar for has been read, which takes the bar back.
///
/// Said only where [`reading`] was, so an ordinary launch is as silent as it was; the page is then
/// drawn like any other, and a draw that outlasts `WARN` in its turn says
/// [`crate::status::still_drawing`].
pub const READ: &str = "the page has been read, and is being drawn";

/// A bar of `cells` characters for a window that has no widget to draw one with — `quorra`'s
/// title — or `None` where there is no share to draw.
#[must_use]
pub fn bar(reading: Reading, cells: usize) -> Option<String> {
    let fraction = reading.fraction()?;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "a share in [0, 1] of a few dozen cells"
    )]
    let full = ((fraction * cells as f64).round() as usize).min(cells);
    Some(format!(
        "{}{}",
        "\u{2588}".repeat(full),
        "\u{2591}".repeat(cells.saturating_sub(full)),
    ))
}

/// A share as a whole percentage, rounded down so that a page is never "100 %" before it is.
fn percent(fraction: f64) -> String {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a share in [0, 1], times a hundred"
    )]
    let whole = (fraction * 100.0).floor() as u32;
    format!("{whole} %")
}

/// An operator count as a person reads one: millions past a million, the count below it.
fn operators(count: u64) -> String {
    if count >= 1_000_000 {
        #[expect(
            clippy::cast_precision_loss,
            reason = "one decimal of a count in millions"
        )]
        let millions = count as f64 / 1_000_000.0;
        format!("{millions:.1} million")
    } else {
        count.to_string()
    }
}

/// Page one drawn while it is read: its display list handed over in parts as it is read, and each
/// part painted on a thread of its own into the target the first frame will be asked at (ADR 1855).
///
/// # What this is for
///
/// A page that reads for seconds showed a bar for all of them, and then drew: on the project
/// owner's drawing the read is three quarters of the first frame (ADR 1845 §1). Here the opening
/// thread reads the page as before while a second thread draws it as it is read — pdf.js's
/// operator list flushed in chunks, taken as evidence about the approach — so a window shows the
/// page as it stands at [`GLIMPSE_AFTER`] instead of a bar, and the first frame's request finds the
/// page already drawn through all but its last part.
///
/// # When it draws, and when it does not
///
/// **Nothing is drawn ahead until the read has run [`GLIMPSE_AFTER`]**, ADR 1825's wait: no page
/// of `doc/pdf.js`'s first pages reads that long, so no launch any gate measures pays a thread's
/// drawing beside its read, and a page read past it is the page a person is waiting for. Nor is
/// anything drawn before the window has said how large it is ([`Ahead::window_is`]), because the
/// target is the window's. The parts that came in the meantime are waiting for the drawer and are
/// drawn first.
///
/// **The pixels are the page's at the drawing's own cuts**, by `render_cpu::Underway`'s
/// construction: as many even strips as the pool is wide, where the whole draw balances its own
/// (`Underway::in_strips`). Every page where that is not known — a part taken back by the reader,
/// a soft mask, a mark across a cut, a page finished with a plane of its own, a request at another
/// target than the one drawn — is drawn whole by the drawing thread exactly as before. What a
/// page drawn ahead costs that one drawn after does not is the reader sharing the pool with the
/// drawing of each part for the length of the read.
#[derive(Clone)]
pub struct Ahead {
    shared: Arc<AheadShared>,
}

/// What the opening thread and the window share.
struct AheadShared {
    /// How the window places the page in the target it draws, from where the core will ask for it.
    place: Place,
    /// The rasteriser the window's first frame is drawn with, whose settings the page drawn ahead
    /// must share — a window's ground, for one.
    rasterizer: CpuRasterizer,
    window: Mutex<Option<Window>>,
    look: Mutex<Option<Look>>,
    drawn: Mutex<Option<(Parts, Underway)>>,
}

/// How a window turns the core's forecast into the target it draws the page into: the page's own
/// target for a window that places page rasters ([`Ahead::page_sized`]), or the window's extent
/// with the page placed in it for one that composes the arrangement into one raster.
pub type Place = Box<dyn Fn(Foreseen, Window) -> TargetSpec + Send + Sync>;

/// What a window tells the opening thread about itself.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Window {
    /// Its extent, in device pixels.
    pub width: u32,
    /// Its extent, in device pixels.
    pub height: u32,
    /// How much of its width the core is not told about — a panel's — in device pixels.
    pub inset: u32,
    /// Its scale factor.
    pub scale: f32,
}

impl Window {
    /// The viewport the core is told about: the window less its inset.
    #[must_use]
    pub fn viewport(self) -> (u32, u32) {
        (
            self.width.saturating_sub(self.inset).max(1),
            self.height.max(1),
        )
    }
}

/// The page drawn ahead, as it stood at its last look.
#[derive(Debug, Clone)]
pub struct Look {
    /// Counts the looks, so a window puts up each one once.
    pub serial: u64,
    /// The pixels, at [`Self::target`].
    pub raster: Arc<Raster>,
    /// The target they are of.
    pub target: TargetSpec,
    /// How many of the page's commands they hold.
    pub commands: usize,
    /// Which page they are of, counted from zero.
    pub page: usize,
    /// Where the core will place the page's first frame in the viewport, in device pixels — the
    /// forecast the target was taken from ([`Foreseen::origin`]), so a window that places page
    /// rasters puts the look exactly where the frame that replaces it will be.
    pub origin: (f32, f32),
}

impl Look {
    /// The look as a window that places page rasters shows it: at its own size, where the core
    /// will place the first frame (ADR 1867).
    #[must_use]
    pub fn placed(&self) -> crate::stand_in::Placed<'_> {
        crate::stand_in::Placed {
            frame: viewer_core::FrameView {
                page: self.page,
                raster: &self.raster,
                origin: self.origin,
            },
            extent: (self.raster.width, self.raster.height),
            retained: false,
        }
    }
}

/// How often the opening thread looks for the window and the read while it waits to draw.
const AHEAD_POLL: Duration = Duration::from_millis(5);

impl Ahead {
    /// An arrangement for a window that draws `rasterizer`'s way and places the page with `place`.
    #[must_use]
    pub fn new(rasterizer: CpuRasterizer, place: Place) -> Self {
        Self {
            shared: Arc::new(AheadShared {
                place,
                rasterizer,
                window: Mutex::new(None),
                look: Mutex::new(None),
                drawn: Mutex::new(None),
            }),
        }
    }

    /// For a window whose pages are page-sized rasters it places itself — the toolkit windows.
    #[must_use]
    pub fn page_sized() -> Self {
        Self::new(
            CpuRasterizer::new(),
            Box::new(|foreseen: Foreseen, _| foreseen.target),
        )
    }

    /// Tells the opening thread how large the window is; asked again whenever it changes.
    pub fn window_is(&self, window: Window) {
        *lock(&self.shared.window) = Some(window);
    }

    /// The page as it stood at the last look, where one has been taken.
    #[must_use]
    pub fn look(&self) -> Option<Look> {
        lock(&self.shared.look).clone()
    }

    /// Whether the page drawn ahead is the picture of `list` at `target` — [`Self::take_for`]'s
    /// question, asked without taking anything, by a window deciding which lane a request goes to
    /// (ADR 2070): a page finished from its drawing ahead is the drawing thread's.
    #[must_use]
    pub fn holds(&self, list: &Arc<DisplayList>, target: TargetSpec) -> bool {
        lock(&self.shared.drawn)
            .as_ref()
            .is_some_and(|(parts, underway)| parts.is(list) && underway.target() == target)
    }

    /// The page drawn ahead, for the request that asks for `list` at `target` — and `None` for
    /// any other, after which nothing is held for the list's page: a request for it at another
    /// target is a frame the page drawn ahead is not.
    #[must_use]
    pub fn take_for(&self, list: &Arc<DisplayList>, target: TargetSpec) -> Option<Underway> {
        let mut drawn = lock(&self.shared.drawn);
        let (parts, underway) = drawn.as_ref()?;
        if !parts.is(list) {
            return None;
        }
        let matches = underway.target() == target;
        let (_, underway) = drawn.take()?;
        drop(lock(&self.shared.look).take());
        matches.then_some(underway)
    }

    /// [`Viewer::anticipate`], with the page drawn on a thread of its own as it is read on this
    /// one — the call an opening thread makes in its place.
    ///
    /// # Panics
    ///
    /// Where the interpretation panicked, as [`Viewer::anticipate`] would have; or where the
    /// drawing thread did, with its panic.
    pub fn anticipate(&self, viewer: &mut Viewer) -> Vec<Event> {
        let Some(anticipating) = viewer.anticipating() else {
            return viewer.anticipate().collect();
        };
        let parts = Parts::new();
        let reading = anticipating.reader();
        let (tell, ended) = channel();
        // **The read stays on this thread and the drawing goes to a new one**, and the order is
        // a measurement: the read is the first frame's critical path, the opening thread is the
        // one a launch has already placed (`Viewer::anticipate`'s read runs here), and a new
        // thread may land on one of this machine's slower cores — the read lost a second to the
        // other order on the owner's display, where the drawer, which keeps up from either kind
        // of core, loses nothing that is waited for (ADR 1855 §4). The window's bar is fed as
        // before, by the figure installed on this thread.
        let (read, drawn) = std::thread::scope(|scope| {
            let (looking, handing) = (&anticipating, &parts);
            let drawer = scope.spawn(move || self.draw_beside(looking, handing, &ended));
            let read = {
                let _ended = Ended(tell);
                reading.read(&parts)
            };
            (read, drawer.join())
        });
        let drawn = match drawn {
            Ok(drawn) => drawn,
            Err(panic) => std::panic::resume_unwind(panic),
        };
        let events = viewer.anticipated(read, Some(&parts)).collect();
        *lock(&self.shared.drawn) = drawn.map(|underway| (parts, underway));
        events
    }

    /// The drawer's side: waits for the read to have run [`GLIMPSE_AFTER`] and for the window's
    /// size, then draws every part as it arrives and takes a look every [`GLIMPSE_EVERY`] —
    /// `None` where nothing was drawn ahead, or what was drawn is not the page.
    fn draw_beside(
        &self,
        anticipating: &Anticipating<'_>,
        parts: &Parts,
        read_ended: &Receiver<()>,
    ) -> Option<Underway> {
        let began = Instant::now();
        // Woken by the read's end the moment it comes, so a page read inside the wait — every
        // page a launch gate measures — is taken in as soon as it is read, as `anticipate` takes
        // it, and nothing about it waits for a poll.
        let window = loop {
            // The rest of the wait, then a look for the window's size every poll.
            let wait = Some(GLIMPSE_AFTER.saturating_sub(began.elapsed()))
                .filter(|left| !left.is_zero())
                .unwrap_or(AHEAD_POLL);
            match read_ended.recv_timeout(wait) {
                Ok(()) | Err(RecvTimeoutError::Disconnected) => return None,
                Err(RecvTimeoutError::Timeout) => {}
            }
            if began.elapsed() >= GLIMPSE_AFTER
                && let Some(window) = *lock(&self.shared.window)
            {
                break window;
            }
        };
        let foreseen = anticipating.foresee(window.viewport(), window.scale)?;
        let target = (self.shared.place)(foreseen, window);
        // In strips on the pool, so that the rest of the page is drawn there once the read ends:
        // on the owner's drawing one thread finished the frame later than the pool draws the
        // page whole after the read (ADR 1896).
        let mut underway = Underway::in_strips(&self.shared.rasterizer, target)?;
        let mut looked: Option<Instant> = None;
        let mut serial = 0_u64;
        loop {
            // Every part that is waiting is drawn in one pass over the strips, which reaches more
            // of the page than one part's marks do (`Underway::take_all`).
            let mut waiting = Vec::new();
            let mut ended = None;
            let mut wait = AHEAD_POLL;
            while let Some(handed) = parts.take(wait) {
                match handed {
                    Handed::Part(part) => waiting.push(part),
                    Handed::Ended(result) => {
                        ended = Some(result.is_ok());
                        break;
                    }
                }
                wait = Duration::ZERO;
            }
            if !waiting.is_empty() && !underway.take_all(waiting) {
                return None;
            }
            match ended {
                Some(true) => return Some(underway),
                Some(false) => return None,
                None => {}
            }
            if looked.is_none_or(|looked| looked.elapsed() >= GLIMPSE_EVERY)
                && underway.drawn().commands > 0
                && let Some(raster) = underway.look()
            {
                serial = serial.wrapping_add(1);
                *lock(&self.shared.look) = Some(Look {
                    serial,
                    raster: Arc::new(raster),
                    target,
                    commands: underway.drawn().commands,
                    page: foreseen.page,
                    origin: foreseen.origin,
                });
                looked = Some(Instant::now());
            }
        }
    }
}

impl std::fmt::Debug for Ahead {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Ahead")
            .field("window", &*lock(&self.shared.window))
            .field(
                "drawn",
                &lock(&self.shared.drawn)
                    .as_ref()
                    .map(|(_, underway)| underway.drawn()),
            )
            .finish_non_exhaustive()
    }
}

/// A lock whose holder cannot have left it half-written: every write above is one assignment.
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(operators: u64, taken: u64, held: u64) -> Reading {
        Reading {
            operators,
            taken,
            held,
        }
    }

    #[test]
    fn the_sentence_states_a_share_of_the_stream_and_a_count_of_operators() {
        assert_eq!(
            reading(at(14_496_125, 19_766_752, 39_533_504)),
            "reading the page: 50 % of its contents, 14.5 million operators so far"
        );
        assert_eq!(reading(at(0, 0, 0)), "reading the document");
        assert_eq!(
            reading(at(999, 1, 1_000)),
            "reading the page: 0 % of its contents, 999 operators so far"
        );
    }

    #[test]
    fn the_bar_fills_by_the_share_and_is_absent_without_one() {
        assert_eq!(bar(at(1, 1, 4), 8).as_deref(), Some("██░░░░░░"));
        assert_eq!(bar(at(1, 4, 4), 4).as_deref(), Some("████"));
        assert_eq!(bar(at(0, 0, 0), 4), None);
    }

    #[test]
    fn a_page_is_not_a_hundred_percent_before_it_is() {
        assert_eq!(percent(0.999), "99 %");
        assert_eq!(percent(1.0), "100 %");
    }

    #[test]
    fn work_that_ends_inside_the_warning_is_joined_as_before() {
        let loading = Loading::start(|| 41 + 1);
        assert!(loading.wait());
        assert!(loading.showing().is_none());
        assert_eq!(loading.join().ok(), Some(42));
    }

    /// A document committed beside this crate, opened in a viewer with no viewport — the opening
    /// thread's state before page one is read.
    fn opened() -> Viewer {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf");
        let bytes = std::fs::read(path).expect("the document is committed beside this crate");
        let mut viewer = Viewer::new(0, 0, 1.0);
        let _ = viewer
            .handle(viewer_core::Command::Open {
                id: viewer_core::DocumentId(1),
                bytes: bytes.into(),
                password: None,
                fragment: None,
            })
            .count();
        viewer
    }

    /// The first render request a viewer makes once it is told a viewport.
    fn first_request(viewer: &mut Viewer) -> viewer_core::RenderRequest {
        viewer
            .handle(viewer_core::Command::Resize {
                width: 600,
                height: 800,
                scale: 1.0,
            })
            .find_map(|event| match event {
                Event::NeedsRender(request) => Some(request),
                _ => None,
            })
            .expect("a viewport asks for page one")
    }

    /// A page read inside [`GLIMPSE_AFTER`] — every page a launch gate measures — is read on the
    /// opening thread's behalf and taken in exactly as `Viewer::anticipate` takes it, and nothing
    /// is drawn ahead of it (ADR 1855).
    #[test]
    fn a_page_read_inside_the_wait_is_taken_in_as_anticipate_takes_it_and_drawn_after() {
        let mut before = opened();
        let _ = before.anticipate().count();
        let asked_before = first_request(&mut before);

        let ahead = Ahead::page_sized();
        ahead.window_is(Window {
            width: 600,
            height: 800,
            inset: 0,
            scale: 1.0,
        });
        let mut after = opened();
        drop(ahead.anticipate(&mut after));
        let asked_after = first_request(&mut after);

        assert_eq!(asked_after.target, asked_before.target);
        assert!(
            asked_after.list.as_ref() == asked_before.list.as_ref(),
            "the page read on its own thread is the page `anticipate` reads"
        );
        assert!(ahead.look().is_none(), "nothing was looked at");
        assert!(
            ahead
                .take_for(&asked_after.list, asked_after.target)
                .is_none(),
            "nothing was drawn ahead of a page read inside the wait"
        );
    }

    #[test]
    fn work_that_panics_still_ends_the_wait() {
        let loading = Loading::start(|| -> u8 { panic!("the work fell over") });
        assert!(loading.wait());
        assert!(loading.join().is_err());
    }
}
