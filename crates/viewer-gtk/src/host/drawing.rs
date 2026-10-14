//! The drawing thread: the pages and the windows drawn off GTK's thread taken back, and the long draw that offers a key to stop it.

use std::collections::VecDeque;

use gtk4::glib;
use gtk4::prelude::*;
use viewer_core::{Answer, Command, Query};

use viewer_host::native_window::NativeWindow;
use viewer_host::trace::Topic;

use super::{Host, with};

impl Host {
    /// Takes whatever the drawing thread has finished and answers the viewer for it.
    ///
    /// **A `Finished` whose outcome is `None` is answered to nobody**, which is trap 20 and is why
    /// the type carries an `Option` rather than a `Rendered`: `Rendered::Failed` would record the
    /// page as answered for and stop the scheduler ever asking again, which freezes a page this
    /// window merely chose not to finish drawing.
    ///
    /// **Before the first frame this waits, and after it this only asks** — ADR 0678. A poll
    /// cannot be dispatched while GTK's main loop is inside its own first frame, and that frame is
    /// the expensive one, so page one used to wait the whole of it out for an answer that was
    /// ready in three milliseconds. A window with nothing on the screen has no frame to spoil and
    /// no input to lose, and `viewer_host::drawing::SETTLE` bounds the whole of what it may spend.
    pub(super) fn take_the_drawn(&mut self, queue: &mut VecDeque<Command>) {
        let drawn = if self.presented {
            self.state.drawing.collect()
        } else {
            self.state.drawing.settle(viewer_host::drawing::SETTLE)
        };
        // A draw that ended takes the bar it was showing with it.
        if !drawn.is_empty() {
            self.show_the_drawing();
        }
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
            // itself — §12.8's signatures above all, whose answer digests the signed part of the
            // file. Once per opened document; `viewer_host::report::Due` is the rule. ADR 1044.
            if self.state.showing.report_due.after_a_frame() {
                queue.push_back(Command::Report);
                // And its open sequence, at the same moment and for the same reason: after the
                // first present, never in front of it (ADR 1602).
                queue.push_back(Command::Presented);
            }
            // Table 147's `/FitWindow` is about the first displayed page's size, which is known
            // now and not before (ADR 1429).
            if self.presenting.owes_placing() {
                self.fit_the_window();
            }
            // §12.4.4.1: the page a transition moves *to* is the one whose list has just
            // arrived, so this is where an armed one can begin. The face is kept whether or not
            // anything is presenting — it is an `Arc` and a target, and no rasterisation — because
            // §12.6.4.15's transition needs a page to move *from* and states no mode (ADR 1216).
            self.face_arrived(&finished.request);
        }
        self.take_the_windows(queue);
        self.mind_a_long_draw();
    }

    /// Takes the windows the drawing has finished, and answers the viewer for the first of each
    /// request — the host took its list (ADR 1892).
    fn take_the_windows(&mut self, queue: &mut VecDeque<Command>) {
        let windows = self.state.drawing.windows();
        // A window that lands after its request was answered is a scroll's, and nothing else
        // repaints for it: the look that found it does, as a glimpse is repainted for.
        let repaint = windows
            .iter()
            .any(|(finished, _)| finished.outcome.is_some());
        for (finished, window) in windows {
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
            if finished.outcome.is_none() {
                self.state.oversized.abandoned(&finished.request, window);
                continue;
            }
            if let Some((token, rendered)) = self.state.oversized.landed(finished, window) {
                queue.push_back(Command::RenderReady { token, rendered });
            }
        }
        if repaint {
            self.refresh();
        }
    }

    /// Obeys Table 147's `/FitWindow` at a frame of a document opened in front, while
    /// [`viewer_host::Presenting::place`] says it is owed: the window keeps its chrome and the
    /// page's viewport becomes the size of the page it shows ([`viewer_host::fitted`]), held to
    /// the monitor the window is on. A window that does not fit yet is asked to, and the next
    /// frame measures again — which it needs, because the first frame is drawn before the chrome
    /// this document hid has left the viewport.
    ///
    /// `/CenterWindow` is not obeyed here, and was said when the document opened
    /// ([`NO_POSITION_TO_SET`](super::window::NO_POSITION_TO_SET)). The size is `set_default_size`, which is GTK 4's one way to ask
    /// for a window's size, and a request the platform may refuse.
    pub(super) fn fit_the_window(&mut self) {
        let page = match self.state.viewer.query(Query::View) {
            Answer::View(viewing) => viewing.page,
            _ => 0,
        };
        let Some(placing) = self.presenting.place(page) else {
            return;
        };
        let geometry = match self.state.viewer.query(Query::PageGeometry(page)) {
            Answer::Geometry(geometry) if placing.fit => geometry,
            _ => {
                self.presenting.placed();
                return;
            }
        };
        let scale = f64::from(self.scale.max(1));
        let window = (
            f64::from(self.ui.window.width()),
            f64::from(self.ui.window.height()),
        );
        let viewport = (
            f64::from(self.state.viewport.0) / scale,
            f64::from(self.state.viewport.1) / scale,
        );
        let drawn = (
            f64::from(geometry.width) / scale,
            f64::from(geometry.height) / scale,
        );
        let screen = self.ui.window.surface().and_then(|surface| {
            let geometry = WidgetExt::display(&self.ui.window)
                .monitor_at_surface(&surface)?
                .geometry();
            Some((f64::from(geometry.width()), f64::from(geometry.height())))
        });
        let (width, height) = viewer_host::fitted(window, viewport, drawn, screen);
        // `set_default_size` measures the surface, which is the widget and the shadow a client
        // side decoration draws around it; `width` and `height` measure the widget alone.
        let shadow = self.ui.window.surface().map_or((0.0, 0.0), |surface| {
            (
                f64::from(surface.width()) - window.0,
                f64::from(surface.height()) - window.1,
            )
        });
        if (width - window.0).abs() <= 1.0 && (height - window.1).abs() <= 1.0 {
            self.presenting.placed();
            return;
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a window's extent in logical pixels, held to the screen above"
        )]
        self.ui.window.set_default_size(
            (width + shadow.0).round() as i32,
            (height + shadow.1).round() as i32,
        );
    }

    /// Takes down the pending drawing poll, if there still is one.
    ///
    /// [`Host::disarm`]'s reason one clause over: `SourceId::remove` panics on a source that has
    /// already fired, so the id is looked up before it is used.
    fn disarm_drawing(&mut self) {
        let Some(armed) = self.drawing_armed.take() else {
            return;
        };
        if let Some(source) = glib::MainContext::default().find_source_by_id(&armed) {
            source.destroy();
        }
    }

    /// Arms the next look at the drawing thread, on GTK's own main loop.
    ///
    /// A one-shot re-armed after every look, for [`Host::pump_presentation`]'s reason: the interval
    /// is `None` the moment nothing is being drawn, and a repeating source cannot stop.
    pub(super) fn pump_drawing(&mut self) {
        self.disarm_drawing();
        let Some(interval) = self.state.drawing.interval() else {
            return;
        };
        let me = self.me.clone();
        self.drawing_armed = Some(glib::timeout_add_local_once(interval, move || {
            with(&me, |host| {
                host.drawing_armed = None;
                host.look_at_the_drawing();
            });
        }));
    }

    /// One look at the drawing thread: answer the viewer for whatever landed, and draw it.
    ///
    /// **A look that finds nothing repaints nothing**, which is [`Host::turn_the_clock`]'s own
    /// answer to the same question: a page being drawn is asked about a thousand times a second,
    /// and rebuilding this window's textures at that rate would copy megabytes for a picture that
    /// has not changed.
    fn look_at_the_drawing(&mut self) {
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
            // A page still being drawn has shown how it stands: that is a picture worth the
            // copy, where a look that found nothing is not (ADR 1825).
            if self.state.drawing.glimpsed() {
                self.refresh();
                self.show_the_drawing();
            }
            self.pump_drawing();
            return;
        }
        self.pump(queue);
    }

    /// The bar a page still being read showed, handed over to the page being drawn (ADR 1825):
    /// while a draw is shown as it stands the bar carries how much of the page's drawing there is,
    /// and it goes when the page lands. The bar of a document still opening is that opening's.
    fn show_the_drawing(&mut self) {
        match self.state.drawing.partial() {
            Some(partial) => {
                self.ui.loading.set_fraction(partial.fraction());
                self.ui.loading.set_visible(true);
            }
            None if self.awaiting.is_none() && self.ui.loading.is_visible() => {
                self.ui.loading.set_visible(false);
            }
            None => {}
        }
    }
}
