//! ISO 32000-2 §14.7's structure on AT-SPI, from a Qt window.
//!
//! # Why this is AccessKit and not `QAccessible`
//!
//! Qt has an accessibility layer of its own and a screen reader on this desktop talks to AT-SPI
//! either way, so what is being chosen here is the route rather than the destination. It goes
//! through [`viewer_accessibility`] — the crate `viewer-ui` and `viewer-gtk` also drive — for two
//! reasons, and the second is this crate's own.
//!
//! **The standard's reason.** §14.7.3's role map is a `shall` on this reader and §14.8.4's
//! forty-one types are then mapped onto a platform vocabulary; doing that mapping once is what
//! makes three windows say the same thing about one document. A `QAccessibleInterface` per element
//! would map §14.8.4 onto `QAccessible::Role` and let Qt map that onto AT-SPI, which is a second
//! vocabulary in the middle that can only lose.
//!
//! **And this crate's reason, which is stated in [`crate::bridge`] and is not negotiable here.**
//! C++ owns the [`crate::Host`] for the life of `QApplication::exec` and **Rust never calls a Qt
//! object**; the whole crate holds one hand-written `unsafe` token, asserted by
//! `tests/unsafe_position.rs`. `QAccessible` is a Qt object, and a `QAccessibleInterface` subclass
//! is C++ — so publishing through it would mean either a table of new `unsafe extern "C++"`
//! declarations or a second implementation of §14.8.4 in the one language in this tree that has no
//! test harness. AccessKit's Linux adapter takes three handlers and two rectangles and names no
//! toolkit at all, so it is reached from safe Rust with nothing crossing the bridge in that
//! direction. ADR 0623.
//!
//! # What crosses, and in which direction
//!
//! Everything here is C++ calling Rust, which is what `extern "Rust"` already is: the window tells
//! this side where it is (`window_placed`), asks how long to wait before draining
//! (`accessibility_wait`) and drains (`accessibility_pump`). **This host can place its nodes on
//! the screen and `viewer-gtk` cannot** — `QWidget::frameGeometry` is in screen coordinates and
//! GTK4 exposes no window position at all — which is a difference between two platforms rather
//! than between two hosts, and is recorded rather than hidden.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use viewer_accessibility::{Act, Bridge, Reading, Showing};
use viewer_core::{Command, PointerAction};
use viewer_host::trace::Topic;

use crate::bridge::ffi::QtPlace;
use crate::host::Host;

/// Where a window is on the screen: its frame, then its contents, each as two corners.
///
/// The shape [`Bridge::placed`] takes, kept in the host because a `moveEvent` arrives before the
/// first paint and the adapter does not exist until after it.
pub(crate) type WindowPlace = ((f32, f32, f32, f32), (f32, f32, f32, f32));

impl Host {
    /// Brings the bridge up after the first frame, and keeps it in step with the page.
    ///
    /// **Called from [`Host::painted`] and never before it**, which is `CLAUDE.md`'s startup rule
    /// made concrete: [`Bridge::new`] spawns a thread that connects to the session bus, and page
    /// one may not wait behind a D-Bus round trip for a screen reader that is probably not there.
    pub(crate) fn attend(&mut self) {
        if self.accessibility.is_none() {
            let pending = Arc::clone(&self.access_pending);
            self.accessibility = Some(Bridge::new(move || {
                pending.store(true, Ordering::Relaxed);
            }));
            let said = match Bridge::shortfall() {
                None => "accessibility bridge up",
                Some(_) => {
                    "accessibility: no bridge on this platform — §14.7's tree is still built \
                     and published to nothing"
                }
            };
            self.trace.say(Topic::Access, format_args!("{said}"));
            // Whatever the window last told this side about its own place, which it may have done
            // before there was an adapter to hand it to.
            self.tell_the_adapter_where_the_window_is();
        }
        let (width, height) = self.viewport;
        let Some(showing) = Showing::of(&self.viewer, width, height) else {
            return;
        };
        if self.spoken == Some(showing) {
            return;
        }
        self.spoken = Some(showing);
        self.speak();
    }

    /// Hands §14.7's structure for every page on the screen to AT-SPI.
    ///
    /// The three things only a host knows — what the window is called, how large the viewport is,
    /// and what the document is called — and nothing else: what to ask the viewer, and in what
    /// order, is [`Reading`]'s, shared with the other two windows.
    fn speak(&mut self) {
        let document = self.named();
        let window = format!("{document} — {}", self.caption());
        #[expect(
            clippy::cast_precision_loss,
            reason = "a viewport in device pixels; f32 is exact to 2^24 and no display is"
        )]
        let viewport = (self.viewport.0 as f32, self.viewport.1 as f32);
        let reading = Reading::of(&self.viewer, &window, &document, viewport);
        self.trace.say(
            Topic::Access,
            format_args!(
                "accessibility: {} page(s) on screen, {} element(s), {} report(s), \
                 {} unreadable code(s)",
                reading.pages_shown(),
                reading.elements(),
                reading.reports(),
                reading.unreadable()
            ),
        );
        if let Some(bridge) = self.accessibility.as_mut() {
            bridge.speak(&reading);
        }
    }

    /// Where this window is on the screen, as Qt has just reported it.
    ///
    /// Kept whether or not there is an adapter yet: a `moveEvent` arrives before the first paint
    /// and the bridge does not exist until after it, so the value is stored and handed over when
    /// there is somewhere to hand it.
    pub(crate) fn window_placed(&mut self, outer: QtPlace, inner: QtPlace) {
        self.window_at = Some((corners(outer), corners(inner)));
        self.tell_the_adapter_where_the_window_is();
    }

    /// Passes that on, if there is an adapter and this platform wants it.
    fn tell_the_adapter_where_the_window_is(&mut self) {
        if !Bridge::wants_window_bounds() {
            return;
        }
        let Some((outer, inner)) = self.window_at else {
            return;
        };
        if let Some(bridge) = self.accessibility.as_mut() {
            bridge.placed(outer, inner);
        }
    }

    /// How long the window should wait before draining, or `-1` while there is no bridge.
    ///
    /// `presentation_wait`'s shape one clause family over: the interval is a pull, so C++ owns the
    /// timer and this side owns the decision — which is [`Bridge::wait_millis`], shared with
    /// `viewer-gtk` so that the two native hosts wake at the same rate for the same reason.
    pub(crate) fn accessibility_wait(&self) -> i32 {
        self.accessibility.as_ref().map_or(-1, Bridge::wait_millis)
    }

    /// Carries out whatever an assistive technology has asked for since this was last called.
    ///
    /// Everything a request means is a **place**, in the same device pixels a pointer works in
    /// ([`Act`]), so this is three commands this host already sends and no new message. A request
    /// this program cannot place is printed by name rather than dropped (trap 5).
    ///
    /// The match over [`Act`] is exhaustive in all three windows, which is what makes a fourth
    /// action a compile error in three places rather than a silent no-op in two.
    pub(crate) fn accessibility_pump(&mut self) {
        if !self.access_pending.swap(false, Ordering::Relaxed) {
            return;
        }
        let Some(bridge) = self.accessibility.as_mut() else {
            return;
        };
        for one in bridge.requested() {
            let Some(means) = one.means else {
                eprintln!(
                    "note: an assistive technology asked for {:?} on node {:?}, which this host \
                     does not carry out",
                    one.action, one.node
                );
                continue;
            };
            match means {
                Act::Show { at } => self.show(at),
                // A press and a release at one point, which is what a mouse click already is
                // here — **except over §12.7's own widgets, which this host delegates.** A real
                // `QCheckBox` is what a person's click lands on and a synthetic press at a page
                // coordinate goes past it to the page underneath, so the click is refused by name
                // rather than appearing to work (ADR 0623).
                Act::Click { at } => {
                    if let viewer_core::Answer::Field { name, .. } =
                        self.viewer.query(viewer_core::Query::FieldAt(at))
                    {
                        let said = viewer_host::delegated_click(name.shown());
                        self.trace
                            .say(Topic::Access, format_args!("refused: {said}"));
                        eprintln!("note: {said}");
                        continue;
                    }
                    self.dispatch(Command::Pointer {
                        at,
                        action: PointerAction::Pressed,
                    });
                    self.dispatch(Command::Pointer {
                        at,
                        action: PointerAction::Released,
                    });
                }
                // A press puts the anchor down and a drag carries it to the other end. A caret is
                // the degenerate case where the two points are equal, and the drag is then not
                // sent: it would set a selection of nothing over what the press already did.
                Act::Caret { from, to } => {
                    self.dispatch(Command::Pointer {
                        at: from,
                        action: PointerAction::Pressed,
                    });
                    if to != from {
                        self.dispatch(Command::Pointer {
                            at: to,
                            action: PointerAction::Dragged,
                        });
                    }
                    self.dispatch(Command::Pointer {
                        at: to,
                        action: PointerAction::Released,
                    });
                }
            }
            self.trace
                .say(Topic::Access, format_args!("carried out {:?}", one.action));
        }
    }

    /// Brings a rectangle of the page into the viewport, which is AT-SPI's `Component.ScrollTo`.
    ///
    /// **The smallest scroll that makes it visible**, rather than one that centres it: a client
    /// asks this of every element it speaks, so an implementation that centred each one would move
    /// the page under a person on every word.
    fn show(&mut self, at: [f32; 4]) {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a viewport in device pixels; f32 is exact to 2^24 and no display is"
        )]
        let viewport = (self.viewport.0.max(1) as f32, self.viewport.1.max(1) as f32);
        let dx = delta(at[0], at[2], viewport.0);
        let dy = delta(at[1], at[3], viewport.1);
        if dx == 0.0 && dy == 0.0 {
            return;
        }
        self.dispatch(Command::Scroll { dx, dy });
    }
}

/// A Qt rectangle as the two corners [`Bridge::placed`] takes.
fn corners(place: QtPlace) -> (f32, f32, f32, f32) {
    (
        place.x,
        place.y,
        place.x + place.width,
        place.y + place.height,
    )
}

/// How far the content must move along one axis to bring `low..high` inside `0..extent`.
///
/// Zero where it is already inside, which is what makes a client's repeated `ScrollTo` on the
/// element it is reading cost nothing.
fn delta(low: f32, high: f32, extent: f32) -> f32 {
    if low < 0.0 || high - low > extent {
        // Above the viewport, or too large to fit: the near edge comes to the near edge.
        low
    } else if high > extent {
        high - extent
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::delta;

    /// The smallest move that brings a span inside the viewport, and none where it already is.
    #[expect(
        clippy::float_cmp,
        reason = "every one of these is a difference of two of the inputs, so each is exact in \
                  binary and a tolerance would only hide an arithmetic that stopped being one"
    )]
    #[test]
    fn a_span_already_in_the_viewport_is_not_scrolled_to() {
        assert_eq!(delta(10.0, 40.0, 100.0), 0.0);
        assert_eq!(delta(120.0, 140.0, 100.0), 40.0);
        assert_eq!(delta(-30.0, -10.0, 100.0), -30.0);
        assert_eq!(delta(20.0, 400.0, 100.0), 20.0);
    }
}
