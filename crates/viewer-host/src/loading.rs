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
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use pdf_model::progress::{Progress, Reading};

use crate::drawing::WARN;

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

    #[test]
    fn work_that_panics_still_ends_the_wait() {
        let loading = Loading::start(|| -> u8 { panic!("the work fell over") });
        assert!(loading.wait());
        assert!(loading.join().is_err());
    }
}
