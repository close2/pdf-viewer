//! ISO 32000-2 §14.7's structure on AT-SPI, from a GTK window.
//!
//! # Why this is AccessKit and not ATK
//!
//! GTK4 has an accessibility layer of its own, and a screen reader on this desktop talks to
//! AT-SPI either way — so the question this module answers is not *whether* to reach AT-SPI but
//! *through what*. It goes through [`viewer_accessibility`], the same crate `viewer-ui` drives,
//! for a reason that is about the standard rather than about the toolkit: §14.7.3's role map is a
//! `shall` on this reader, §14.8.4's forty-one types are then mapped onto a platform vocabulary,
//! and doing that mapping once is what makes three windows say the same thing about one document.
//! Publishing through `GtkAccessible` would map §14.8.4 onto `GtkAccessibleRole` and let GTK map
//! *that* onto AT-SPI — a second vocabulary in the middle that can only lose — and it would be a
//! second implementation beside the one `tools/state.sh accessibility` ratchets. ADR 0623 has the
//! argument, the alternative that was tried, and the cost this choice carries.
//!
//! # What GTK cannot do, said rather than left silent
//!
//! **This window cannot say where it is on the screen.** AT-SPI reports a node's extents in screen
//! coordinates and this program's are the viewport's, so an adapter needs the window's own origin
//! to add — which winit gives `viewer-ui` and Qt gives `viewer-qt`. GTK4 exposes it nowhere:
//! neither `GtkWindow`, nor `GdkSurface`, nor `GdkToplevel` has a position, and `gtk4-sys` has no
//! symbol for one either, which is a deliberate consequence of Wayland having no such concept. So
//! [`viewer_accessibility::Bridge::placed`] is never called here and a client reading
//! `Component.GetExtents` is told where an element is *in this window*. It is reported by name at
//! the moment the bridge comes up, because a coordinate system that is silently the wrong one is
//! worse than one a person is told about (trap 5).
//!
//! # Why a client's request is drained by a timer
//!
//! `accesskit_unix` calls back from its own D-Bus thread, and nothing in `gtk4-rs` carries a
//! `Weak<RefCell<Host>>` across a thread — the `Send` half of the toolkit's main loop takes a
//! `Send` closure, and this host is not `Send` by construction (ADR 0244). So the wake sets a flag
//! and a `glib::timeout_add_local` drains it, armed **only once an assistive technology has
//! attached** ([`viewer_accessibility::Bridge::attended`]). A window nobody is listening to arms
//! no source at all, which is the same discipline §12.4.4.1's clock keeps one module over.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use gtk4::glib;
use viewer_accessibility::{Act, Bridge, Reading, Showing};
use viewer_core::{Command, PointerAction};
use viewer_host::trace::Topic;

use crate::host::{Host, with};

impl Host {
    /// Brings the bridge up after the first frame, and keeps it in step with the page.
    ///
    /// **Called from the end of [`Host::refresh`] and never before one**, which is
    /// `CLAUDE.md`'s startup rule made concrete: [`Bridge::new`] spawns a thread that connects to
    /// the session bus, and page one may not wait behind a D-Bus round trip for a screen reader
    /// that is probably not there. After that it costs one [`Showing`] comparison per frame,
    /// because the structure of a page does not change when it is scrolled.
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
            // The half of AT-SPI's geometry this toolkit has no answer for. Said once, at the
            // moment the claim starts being made, rather than per page.
            if Bridge::wants_window_bounds() {
                self.trace.say(
                    Topic::Access,
                    format_args!(
                        "accessibility: this window cannot report its own position — GTK4 \
                         exposes none — so a node's extents are this window's pixels rather \
                         than the screen's"
                    ),
                );
            }
        }
        let (width, height) = self.viewport;
        let Some(showing) = Showing::of(&self.viewer, width, height) else {
            return;
        };
        if self.spoken == Some(showing) {
            self.pump_accessibility();
            return;
        }
        self.spoken = Some(showing);
        self.speak();
        self.pump_accessibility();
    }

    /// Hands §14.7's structure for every page on the screen to AT-SPI.
    ///
    /// The three things only a host knows — what the window is called, how large the viewport is,
    /// and what the document is called — and nothing else: what to ask the viewer, and in what
    /// order, is [`Reading`]'s, shared with the other two windows so that a screen reader is told
    /// the same thing whichever of them a person opened the file in.
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

    /// Keeps the drain armed at whatever interval the bridge asks for.
    ///
    /// **`Host::pump_presentation`'s shape one clause family over, and Qt's `pumpAccessibility` is
    /// the same three lines** — which is the point: the interval is a decision and it is
    /// [`Bridge::wait_millis`]'s, so the two native hosts wake at the same rate for the same
    /// reason and only the toolkit call differs. Re-armed rather than repeating, because the
    /// interval changes exactly once in a window's life: when a client attaches.
    fn pump_accessibility(&mut self) {
        let wait = match self.accessibility.as_ref() {
            Some(bridge) => bridge.wait_millis(),
            None => return,
        };
        if self.access_interval == Some(wait) && self.access_draining.is_some() {
            return;
        }
        self.disarm_the_drain();
        self.trace.say(
            Topic::Access,
            format_args!("accessibility: draining a client's requests every {wait} ms"),
        );
        self.access_interval = Some(wait);
        let me = self.me();
        let every = std::time::Duration::from_millis(wait.unsigned_abs().into());
        self.access_draining = Some(glib::timeout_add_local(every, move || {
            with(&me, |host| {
                host.act();
                host.pump_accessibility();
            });
            glib::ControlFlow::Continue
        }));
    }

    /// Takes the drain's source down, if there still is one.
    ///
    /// `SourceId::remove` panics on a source that is already gone — it unwraps a `Result` — so the
    /// identifier is looked up first, which is what `Host::disarm` does for §12.4.4.1's clock.
    fn disarm_the_drain(&mut self) {
        let Some(armed) = self.access_draining.take() else {
            return;
        };
        if let Some(source) = glib::MainContext::default().find_source_by_id(&armed) {
            source.destroy();
        }
    }

    /// Carries out whatever an assistive technology has asked for since this was last called.
    ///
    /// Everything a request means is a **place**, in the same device pixels a pointer works in
    /// ([`Act`]), so this is three commands this host already sends and no new message. A request
    /// this program cannot place is printed by name rather than dropped (trap 5).
    ///
    /// The match over [`Act`] is exhaustive in all three windows, which is what makes a fourth
    /// action a compile error in three places rather than a silent no-op in two.
    pub(crate) fn act(&mut self) {
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
                // here — **with §12.7.5.2's toggling in front of it, which is what a click on a
                // check box or a radio button *is*.** This host delegates the widget's appearance
                // and a real `GtkCheckButton` is what a person's click lands on, so a synthetic
                // press at a page coordinate goes past it to the page underneath; the value is
                // therefore given to the field directly, which is the same `Edit::SetField` the
                // control's own callback sends and is decided by the same function (ADR 0630).
                Act::Click { at } => {
                    self.click_page(at);
                    self.dispatch(Command::Pointer {
                        at,
                        action: PointerAction::Pressed,
                    });
                    self.dispatch(Command::Pointer {
                        at,
                        action: PointerAction::Released,
                    });
                }
                // A press puts the anchor down and a drag carries it to the other end, which is
                // what a person's drag sends. A caret is the degenerate case where the two points
                // are equal, and the drag is then not sent at all: it would set a selection of
                // nothing over what the press has already done.
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

    /// §12.7.5.2's half of a click, at a point of the page's own viewport.
    ///
    /// **The other half of the delegated click, and it needed no message** (ADR 0630). A person's
    /// click on a check box lands on a real [`gtk4::CheckButton`] this host placed; an assistive
    /// technology's `Action.DoAction` names a §14.8.4.7.2 `Form` element, which
    /// [`viewer_accessibility::Act::Click`] resolves to a *point on the page* — and a point on the
    /// page is under the control rather than on it. So the value goes to the field directly, which
    /// is the same [`viewer_core::Edit::SetField`] the control's own callback sends, decided by the
    /// same [`viewer_host::clicked`] so that the two cannot become different clicks. The control
    /// then follows the field, because `Host::place_fields` writes a toggle's state back on the
    /// frame the edit causes.
    ///
    /// **The match is exhaustive in all three windows**, which is what makes a case added to
    /// [`viewer_host::Clicked`] a compile error in three places rather than a silent no-op in two.
    fn click_page(&mut self, at: (f32, f32)) {
        let clicked = viewer_host::clicked(&self.viewer, at);
        // `true`: this host places a real control over every widget, so a synthetic press at a
        // page coordinate is exactly what cannot reach one. Trap 5 — the refusal is by name.
        if let Some(said) = clicked.note(true) {
            self.trace
                .say(Topic::Access, format_args!("refused: {said}"));
            eprintln!("note: {said}");
        }
        match clicked {
            viewer_host::Clicked::Toggles { name, value } => {
                self.trace.say(
                    Topic::Access,
                    format_args!("setting the field {} to {value}", name.shown()),
                );
                self.dispatch(Command::Edit(viewer_core::Edit::SetField {
                    field: name.qualified,
                    value: viewer_core::Entered::Text(value),
                }));
            }
            // Said above, or the pointer's: §12.6.3's triggers and §12.5.5's appearance are what
            // a click on a push button, a signature or the page itself comes to, and the press
            // and release below carry those in every host.
            viewer_host::Clicked::ReadOnly { .. }
            | viewer_host::Clicked::Stays { .. }
            | viewer_host::Clicked::Unnamed { .. }
            | viewer_host::Clicked::Pointed { .. }
            | viewer_host::Clicked::Aimed { .. }
            | viewer_host::Clicked::Page => {}
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

/// How far the content must move along one axis to bring `low..high` inside `0..extent`.
///
/// Zero where it is already inside, which is what makes a client's repeated `ScrollTo` on the
/// element it is reading cost nothing. The same arithmetic all three windows use; it is four lines
/// and lives beside each host's own viewport rather than in a shared crate, because what differs
/// between the three is precisely what "the viewport" means — this host's page area is a widget
/// and `viewer-ui`'s is the window minus its own drawn sidebar.
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
