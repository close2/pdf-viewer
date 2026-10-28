//! The drawing thread: the pages and windows it has finished taken back and answered for, the
//! rows a person sees drawn first, and a draw that outlasts its warning said (ADRs 0668, 1868,
//! 1892; filed here by ADR 1908). And the graphics device beside it, whose pages are read back and
//! taken in on the same poll (ADR 2070).

use std::collections::VecDeque;

use viewer_core::{Answer, Query};
use viewer_device::renderer::ReadBack;
use viewer_host::native_window::NativeWindow;
use viewer_host::trace::Topic;

use super::Host;

/// The graphics device for a window of this program, brought up now on a thread of its own.
///
/// **No wake**: the device's pages are taken in on the drawing poll, whose interval includes the
/// device's while a page is out (`viewer_host::native_window::WindowState::drawing_interval`). A
/// wake would be a call from the device's thread into `QApplication::exec`, and there is no such
/// path that does not cost this crate a second hand-written `unsafe` token — `drawing_wait`'s own
/// reason (ADRs 0668, 2070).
#[must_use]
pub fn device_lane() -> ReadBack {
    let mut lane = ReadBack::new(None);
    lane.bring_up();
    lane
}

impl Host {
    /// Draws this window's pages on the graphics device and reads each back, in the lane its
    /// prediction gives it, from now on (ADR 2070).
    pub fn draw_on_the_device(&mut self, lane: ReadBack) {
        self.state.lane = Some(Box::new(lane));
    }

    /// The C++ side copied a frame and queued its widget update, with what the copy cost.
    /// Actual widget painting follows on Qt's event loop.
    pub(crate) fn painted(&mut self, bytes: usize, nanos: u64) {
        self.state.trace.say(
            Topic::Frames,
            format_args!(
                "{bytes} bytes into a QImage in {:?}",
                std::time::Duration::from_nanos(nanos)
            ),
        );
        // A page still being drawn, shown as it stands, is said as that and is not the first
        // frame: `first frame on the screen` is a finished page in every window (ADR 1825).
        if let Some(partial) = self.state.drawing.partial()
            && self
                .frames()
                .iter()
                .any(|placed| std::ptr::eq(placed.frame.raster, partial.raster()))
        {
            self.state.trace.say(
                Topic::Frames,
                format_args!(
                    "page {} shown as it stands, {:.0} % drawn",
                    partial.page().saturating_add(1),
                    partial.fraction() * 100.0
                ),
            );
        }
        if self.playing.is_none() {
            for placed in self
                .frames()
                .iter()
                .filter(|placed| placed.waits_for_a_frame())
            {
                self.state
                    .trace
                    .say(Topic::Frames, format_args!("{placed}"));
            }
        }
        // A window drawn of a page zoomed past it is the page's frame on the screen,
        // though the viewer holds no pixels of it (ADR 1930).
        let held = self.playing.is_some()
            || self.state.oversized.shows(self.state.documents.focused())
            || matches!(self.state.viewer.query(Query::Frame), Answer::Frame(frames) if !frames.is_empty());
        if !self.state.presented && held {
            self.state.presented = true;
            self.state.read_look = None;
            self.state.trace.say(
                Topic::Launch,
                format_args!(
                    "first frame on the screen at {:?}",
                    self.state.trace.since_start()
                ),
            );
            // The command line's later documents, and not one moment sooner: nothing page one
            // does not need happens before page one (`CLAUDE.md` section 2).
            self.later_open_the_next();
        }
        // After the first frame rather than before it, and after every one after that: §14.7's
        // tree on AT-SPI. This follows the copy and the request for a widget
        // update. The toolkit paints later; this work is part of the wait before that paint.
        self.attend();
    }

    /// How long the window should wait before asking the drawing thread again, in milliseconds.
    ///
    /// `-1` where nothing is being drawn, which is what stops the timer — so a window showing a
    /// drawn page wakes for this exactly never. [`Host::presentation_wait`]'s shape and the same
    /// argument: the interval is `viewer_host::Drawing`'s decision, shared with the other native
    /// host, and the timer is Qt's.
    ///
    /// **The same timer looks for §12.7.6.2's answers** while one is out, rather than a fifth timer
    /// and a bridge function for it: the look is one channel drain, and the shorter of the two
    /// intervals is the one that stands (ADR 1291).
    pub(crate) fn drawing_wait(&self) -> i32 {
        // A conversion running is looked for on the same timer as a server's answer, at the same
        // interval (ADR 1954).
        let looking = self
            .state
            .submitter
            .interval()
            .or(self.state.converter.interval());
        let interval = match (self.state.drawing_interval(), looking) {
            (Some(drawing), Some(submitting)) => Some(drawing.min(submitting)),
            (drawing, submitting) => drawing.or(submitting),
        };
        // And for the launch document's outline while it is read beside page one (ADR 1553), at
        // the interval chosen for a page's own latency.
        let interval = match (interval, self.preparing.is_some()) {
            (Some(interval), true) => Some(interval.min(viewer_host::drawing::POLL)),
            (None, true) => Some(viewer_host::drawing::POLL),
            (interval, false) => interval,
        };
        // And for a document still opening past `WARN`, at the bar's own tick (ADR 1811).
        let interval = match (interval, self.state.awaiting.is_some()) {
            (Some(interval), true) => Some(interval.min(viewer_host::loading::TICK)),
            (None, true) => Some(viewer_host::loading::TICK),
            (interval, false) => interval,
        };
        interval.map_or(-1, |interval| {
            i32::try_from(interval.as_millis()).unwrap_or(i32::MAX)
        })
    }

    /// One look at the drawing thread: answer the viewer for whatever landed, and draw it.
    ///
    /// **A look that finds nothing flags no update at all**, which is [`Host::presentation_tick`]'s
    /// own answer to the same question: a page being drawn is asked about a thousand times a
    /// second, and rebuilding this window's `QImage` at that rate would copy megabytes for a
    /// picture that has not changed.
    pub(crate) fn drawing_pump(&mut self) {
        if self.state.awaiting.is_some() {
            self.mind_the_opening();
        }
        self.take_the_answers();
        self.take_the_preparation();
        let mut queue = VecDeque::new();
        self.take_the_drawn(&mut queue);
        // The rows the window shows are on it, finished, before the page lands (ADR 1868).
        if let Some((page, after)) = self.state.drawing.seen_drawn() {
            self.state.trace.say(
                Topic::Frames,
                format_args!(
                    "page {} shown finished where the window shows it, {after:?} after it was asked",
                    page.saturating_add(1)
                ),
            );
        }
        if queue.is_empty() {
            // A page still being drawn has shown how it stands: a picture worth the copy, where a
            // look that found nothing is not (ADR 1825). The bar follows it.
            if self.state.drawing.glimpsed() {
                self.update.frame = true;
                self.update.status = true;
            }
            return;
        }
        self.pump(queue.into());
    }
}
