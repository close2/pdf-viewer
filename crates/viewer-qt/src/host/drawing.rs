//! The drawing thread: the pages and windows it has finished taken back and answered for, the
//! rows a person sees drawn first, and a draw that outlasts its warning said (ADRs 0668, 1868,
//! 1892; filed here by ADR 1908).

use std::collections::VecDeque;

use viewer_core::{Answer, Command, Query};
use viewer_host::native_window::NativeWindow;
use viewer_host::trace::Topic;

use super::Host;

impl Host {
    /// The C++ side finished putting a frame on the screen, with what the copy cost.
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
            for placed in self.frames().iter().filter(|placed| placed.stands_in()) {
                self.state
                    .trace
                    .say(Topic::Frames, format_args!("{placed}"));
            }
        }
        let held = self.playing.is_some()
            || matches!(self.state.viewer.query(Query::Frame), Answer::Frame(frames) if !frames.is_empty());
        if !self.presented && held {
            self.presented = true;
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
        // tree on AT-SPI. This is the one moment in this host that is on the far side of a paint,
        // which is exactly what `CLAUDE.md`'s startup rule asks of it (ADR 0623).
        self.attend();
    }

    /// Takes whatever the drawing thread has finished and answers the viewer for it.
    ///
    /// **A `Finished` whose outcome is `None` is answered to nobody**, which is trap 20 and is why
    /// the type carries an `Option` rather than a `Rendered`: `Rendered::Failed` would record the
    /// page as answered for and stop the scheduler ever asking again, which freezes a page this
    /// window merely chose not to finish drawing.
    ///
    /// **Before the first frame this waits, and after it this only asks** — ADR 0678, and the same
    /// three lines as `viewer-gtk`'s because the rule is `viewer_host::Drawing`'s rather than a
    /// toolkit's. A window with nothing on the screen has no frame to spoil and no input to lose,
    /// and `viewer_host::drawing::SETTLE` bounds the whole of what it may spend. The measurement
    /// that put it there is GTK's — Qt's own launch was not the one that lost 44 ms — and it is
    /// here for levelness and because a `QTimer` is no more dispatchable than a `glib` one while a
    /// toolkit is inside its own first frame.
    pub(super) fn take_the_drawn(&mut self, queue: &mut VecDeque<Command>) {
        let drawn = if self.presented {
            self.state.drawing.collect()
        } else {
            self.state.drawing.settle(viewer_host::drawing::SETTLE)
        };
        for finished in drawn {
            self.state.trace.say(
                Topic::Frames,
                format_args!(
                    "page {} {} {}x{} in {:?}, waited {:?}",
                    finished.request.page.saturating_add(1),
                    if finished.outcome.is_some() {
                        "rasterised"
                    } else {
                        "abandoned after"
                    },
                    finished.request.target.width,
                    finished.request.target.height,
                    finished.cost,
                    finished.waited
                ),
            );
            let Some(rendered) = finished.outcome else {
                continue;
            };
            if let viewer_core::Rendered::Raster(raster) = &rendered {
                let began = std::time::Instant::now();
                if self.state.retained.landed(&finished.request, raster) {
                    self.state.trace.say(
                        Topic::Frames,
                        format_args!(
                            "page {} retained, {} held, in {:?}",
                            finished.request.page.saturating_add(1),
                            self.state.retained.len(),
                            began.elapsed()
                        ),
                    );
                }
            }
            queue.push_back(Command::RenderReady {
                token: finished.request.token,
                rendered,
            });
            // The reader has their page, so the document may now be asked what it says about
            // itself — the same moment `viewer-gtk` asks, and `viewer_host::report::Due` is the
            // rule both follow. ADR 1044.
            if self.state.showing.report_due.after_a_frame() {
                queue.push_back(Command::Report);
                // And its open sequence, after the first present and never in front of it (ADR
                // 1602).
                queue.push_back(Command::Presented);
            }
            // Table 147's `/FitWindow` and `/CenterWindow` are about the first displayed page's
            // size, which is known now and not before; the window asks `place_window` with its
            // own extents (ADR 1429).
            if self.presenting.owes_placing() {
                self.update.placement = true;
            }
            // §12.4.4.1: the page a transition moves *to* is the one whose list has just arrived,
            // so this is where an armed one can begin. Only while a presentation is running,
            // because taking the face costs a whole-viewport rasterisation.
            self.face_arrived(&finished.request);
        }
        self.take_the_windows(queue);
        self.mind_a_long_draw();
    }

    /// Takes the windows the drawing has finished, and answers the viewer for the first of each
    /// request — the host took its list (ADR 1892).
    fn take_the_windows(&mut self, queue: &mut VecDeque<Command>) {
        for (finished, window) in self.state.drawing.windows() {
            self.state.trace.say(
                Topic::Frames,
                format_args!(
                    "page {} window {}x{} at {},{} {} in {:?}, waited {:?}",
                    finished.request.page.saturating_add(1),
                    window.width,
                    window.height,
                    window.x,
                    window.y,
                    if finished.outcome.is_some() {
                        "rasterised"
                    } else {
                        "abandoned"
                    },
                    finished.cost,
                    finished.waited
                ),
            );
            self.update.frame = true;
            if finished.outcome.is_none() {
                self.state.oversized.abandoned(&finished.request, window);
                continue;
            }
            if let Some((token, rendered)) = self.state.oversized.landed(finished, window) {
                queue.push_back(Command::RenderReady { token, rendered });
            }
        }
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
        let interval = match (
            self.state.drawing.interval(),
            self.state.submitter.interval(),
        ) {
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
        let interval = match (interval, self.awaiting.is_some()) {
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
        if self.awaiting.is_some() {
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
