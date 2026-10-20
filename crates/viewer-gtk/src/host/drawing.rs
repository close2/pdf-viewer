//! The drawing thread: the pages and the windows drawn off GTK's thread taken back, and the long draw that offers a key to stop it.
//!
//! And the graphics device beside it, whose pages are read back and taken in the same way (ADR
//! 2070).

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Weak;

use gtk4::glib;
use gtk4::prelude::*;
use viewer_core::{Answer, Query};
use viewer_device::renderer::ReadBack;

use viewer_host::native_window::NativeWindow;
use viewer_host::trace::Topic;

use super::{Host, with};

thread_local! {
    /// The window the device's wake looks at, on GTK's own thread: the wake is called on the
    /// device's thread, where no `Rc` may go, so what crosses is a call to [`landed`] and this is
    /// where it finds its window.
    static WOKEN: RefCell<Weak<RefCell<Host>>> = const { RefCell::new(Weak::new()) };
}

/// The graphics device for a window of this program, brought up now on a thread of its own.
///
/// Its wake is GTK's main context: a page read back is taken in on GTK's next turn rather than at
/// the drawing poll's (ADR 2070). The poll stays the guarantee, because a wake that finds the
/// window inside a turn of its own is that turn's to answer.
#[must_use]
pub fn device_lane() -> ReadBack {
    let mut lane = ReadBack::new(Some(viewer_device::Wake::new(|| {
        glib::MainContext::default().invoke(landed);
    })));
    lane.bring_up();
    lane
}

/// What the device's wake runs on GTK's thread: one look at what has landed.
fn landed() {
    let Some(host) = WOKEN.with(|me| me.borrow().upgrade()) else {
        return;
    };
    // A window already borrowed is inside a turn that looks at its pages on the way out
    // (`react`'s `pump_drawing`), so the look is that turn's rather than lost.
    let Ok(mut host) = host.try_borrow_mut() else {
        return;
    };
    host.look_at_the_drawing();
}

impl Host {
    /// Draws this window's pages on the graphics device and reads each back, in the lane its
    /// prediction gives it, from now on (ADR 2070).
    pub fn draw_on_the_device(&mut self, lane: ReadBack) {
        WOKEN.with(|me| *me.borrow_mut() = self.me.clone());
        self.state.lane = Some(Box::new(lane));
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
        let Some(placing) = self.state.presenting.place(page) else {
            return;
        };
        let geometry = match self.state.viewer.query(Query::PageGeometry(page)) {
            Answer::Geometry(geometry) if placing.fit => geometry,
            _ => {
                self.state.presenting.placed();
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
            self.state.presenting.placed();
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
        let Some(interval) = self.state.drawing_interval() else {
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
    pub(super) fn show_the_drawing(&mut self) {
        match self.state.drawing.partial() {
            Some(partial) => {
                self.ui.loading.set_fraction(partial.fraction());
                self.ui.loading.set_visible(true);
            }
            None if self.state.awaiting.is_none() && self.ui.loading.is_visible() => {
                self.ui.loading.set_visible(false);
            }
            None => {}
        }
    }
}
