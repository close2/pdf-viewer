//! §14.7's structure handed to AccessKit, and the window position AT-SPI needs with it.
//!
//! Separate from the frame it follows because the ordering is a rule rather than a convenience:
//! nothing here may run before the first page has been presented, and what it costs is measured
//! beside the frame rather than inside it. The three questions it asks are the three only a host
//! knows — what the window is called, which page is showing, and what the page could not draw.

use viewer_accessibility::Act;
use viewer_core::{Answer, Command, PointerAction, Query};
use winit::event::ElementState;

use crate::app::App;
use crate::trace::Topic;

impl App {
    /// Brings the accessibility bridge up, once, and keeps it in step with the page.
    ///
    /// **Called from the first present and from every one after it, and never before one.** That
    /// ordering is `CLAUDE.md`'s startup rule made concrete: `Bridge::new` spawns a thread that
    /// connects to the session bus, and page one may not wait behind a D-Bus round trip for a
    /// screen reader that is probably not there. What it costs after the first frame is one
    /// comparison per frame, because the structure of a page does not change when it is scrolled.
    ///
    /// **It cost 2.0 ms on average and 3.9 at worst on every page turn, and the three-hundred-
    /// and-ninety-first session found out where** (ADR 0228). Not §14.7's tree: `Query::
    /// AccessibilityTree` is 0.13 to 0.25 ms on this document and the whole of [`App::speak`]
    /// — both queries, the tree built and the bridge given it — is 0.17 to 0.33. It was
    /// [`App::place_window`], at **1.8 to 3.2 ms**, which is two synchronous X11 round trips for
    /// a window position that **does not change when a page turns**. So it is asked where it can
    /// change — when the bridge comes up, when the window moves, and when it is resized — and
    /// the page turn is left with the query that is actually about the page.
    pub(crate) fn attend(&mut self) {
        if self.accessibility.is_none() {
            // What the bridge wakes this loop with. `EventLoopProxy` is the only thing winit gives
            // a foreign thread, and a request that arrived without one would wait for whatever
            // else made the loop come round — which on a window resting in `ControlFlow::Wait` is
            // nothing at all.
            let waker = self.waker.clone();
            self.accessibility = Some(viewer_accessibility::Bridge::new(move || {
                if let Some(waker) = waker.as_ref() {
                    // The loop has exited: the program is closing and there is nobody to wake.
                    let _ = waker.send_event(());
                }
            }));
            // **One of these two sentences used to be false, and ADR 0227 settled which.**
            // The Windows trace printed `note: this build has no accessibility bridge` at line
            // 2 and `trace: accessibility bridge up` at line 46. The note was right:
            // `Bridge::new` builds an `accesskit_unix::Adapter` on Linux and, on every other
            // platform, a struct with no adapter in it at all — so what came "up" there was a
            // tree with nowhere to publish it. The line now asks `shortfall`, which is the same
            // function the note came from, so the two cannot disagree again.
            let said = match viewer_accessibility::Bridge::shortfall() {
                None => "accessibility bridge up",
                Some(_) => {
                    "accessibility: no bridge on this platform — §14.7's tree is still built \
                     and published to nothing, and the note above says so"
                }
            };
            self.trace.say(Topic::Access, format_args!("{said}"));
            // The one place a page turn used to pay for this: a bridge that has just come up has
            // never been told where the window is, and this is the frame that knows there is one.
            self.place_window();
        }
        let Some((width, height, _)) = self.window() else {
            return;
        };
        let Answer::Page { index: page, .. } = self.viewer.query(Query::CurrentPage) else {
            return;
        };
        if self.spoken == Some((page, width, height)) {
            return;
        }
        self.spoken = Some((page, width, height));
        self.speak();
    }

    /// Carries out whatever an assistive technology has asked for since this was last called.
    ///
    /// **Called from the loop's user event and from nowhere else**, which is the fix for a drain
    /// that could not run: the requests used to be read inside [`App::speak`], and `speak` runs
    /// only when the page or the window changes — so a request arriving while a person read one
    /// page sat in the queue until they turned it, and on a window nobody touched, for ever.
    ///
    /// Everything a request means is a **place**, in the same device pixels a pointer works in
    /// (`viewer_accessibility::Act`), so this is three existing commands and no new message.
    /// A request this program cannot place is printed by name rather than dropped, which is what
    /// the whole of this function used to be (trap 5).
    pub(crate) fn act(&mut self) {
        let Some(bridge) = self.accessibility.as_mut() else {
            return;
        };
        let asked = bridge.requested();
        for one in asked {
            let Some(means) = one.means else {
                println!(
                    "note: an assistive technology asked for {:?} on node {:?}, which this host \
                     does not carry out",
                    one.action, one.node
                );
                continue;
            };
            match means {
                Act::Show { at } => self.show(at),
                // A press and a release at one point, through the same three steps a mouse takes:
                // see `App::click_page`.
                Act::Click { at } => {
                    self.click_page(at, ElementState::Pressed);
                    self.click_page(at, ElementState::Released);
                }
                // A press puts the anchor down and a drag carries it to the other end, which is
                // exactly what a person's drag sends. A caret is the degenerate case where the
                // two points are equal, and then the drag is not sent at all: `Command::Pointer`
                // with `Dragged` at the anchor would set a selection of nothing, which is what a
                // press has already done.
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
        self.redraw();
    }

    /// Brings a rectangle of the page into the viewport, which is AT-SPI's `Component.ScrollTo`.
    ///
    /// **The smallest scroll that makes it visible**, rather than one that centres it: a magnifier
    /// following a caret down a page should move the page as little as it can, and a rectangle
    /// already on the screen should not move it at all. A rectangle taller or wider than the
    /// viewport is aligned to its top-left corner, because the beginning of an element is what a
    /// person is being taken to.
    ///
    /// `Command::Scroll`'s `dy` is positive to move the *content* up, so bringing something below
    /// the viewport into view is a positive delta.
    fn show(&mut self, at: [f32; 4]) {
        let Some((width, height, _)) = self.window() else {
            return;
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "a viewport in device pixels; f32 is exact to 2^24 and no display is"
        )]
        let viewport = (
            width.saturating_sub(self.inset()).max(1) as f32,
            height.max(1) as f32,
        );
        let dx = delta(at[0], at[2], viewport.0);
        let dy = delta(at[1], at[3], viewport.1);
        if dx == 0.0 && dy == 0.0 {
            return;
        }
        self.dispatch(Command::Scroll { dx, dy });
    }

    /// Tells the bridge where the window is, which is what AT-SPI needs to place a node.
    ///
    /// A node's bounds cross in the window's own pixels and AT-SPI reports them in the screen's,
    /// so the adapter adds the window's position. Under Wayland an application cannot learn its
    /// own position and winit says so by refusing the call; there is nothing to be done about
    /// that here and nothing is claimed instead.
    ///
    /// **Called when the answer can have changed and not otherwise**: when the bridge comes up,
    /// on `WindowEvent::Moved` and on `WindowEvent::Resized`. It used to be called on every page
    /// turn, where the two winit calls below are two synchronous X11 round trips — 1.8 to 3.2 ms
    /// — for a number a page turn cannot move (ADR 0228).
    #[expect(
        clippy::cast_precision_loss,
        reason = "a window's position and size in device pixels; f32 is exact to 2^24"
    )]
    pub(crate) fn place_window(&mut self) {
        // Asked of the crate that has the adapter rather than decided here, because the answer
        // is about AT-SPI's screen coordinates rather than about having a bridge at all — see
        // `Bridge::wants_window_bounds`, which is where the two part company when `doc/todo/31`
        // wires the Windows and macOS adapters in.
        if !viewer_accessibility::Bridge::wants_window_bounds() || self.accessibility.is_none() {
            return;
        }
        let Some(state) = self.state.as_ref() else {
            return;
        };
        let outer = state.window.outer_size();
        let inner = state.window.inner_size();
        let (Ok(outer_at), Ok(inner_at)) =
            (state.window.outer_position(), state.window.inner_position())
        else {
            return;
        };
        let outer = (
            outer_at.x as f32,
            outer_at.y as f32,
            (outer_at.x.saturating_add_unsigned(outer.width)) as f32,
            (outer_at.y.saturating_add_unsigned(outer.height)) as f32,
        );
        let inner = (
            inner_at.x as f32,
            inner_at.y as f32,
            (inner_at.x.saturating_add_unsigned(inner.width)) as f32,
            (inner_at.y.saturating_add_unsigned(inner.height)) as f32,
        );
        if let Some(bridge) = self.accessibility.as_mut() {
            bridge.placed(outer, inner);
        }
    }

    /// Hands §14.7's structure for every page on the screen to the platform's accessibility API.
    ///
    /// **The fifth of the five things `doc/ui-boundary.md` lists as blocked on the
    /// `viewer-core` boundary, and the last.** `Query::AccessibilityTree` has answered since the
    /// hundred-and-forty-ninth session and nothing asked; this asks. What crosses is
    /// `viewer-accessibility`'s business — §14.8.4's types onto AccessKit's roles, and AT-SPI
    /// underneath that — and what is this host's is the three things only a host knows: what the
    /// window is called, which pages are showing, and what each of them could not draw.
    ///
    /// **One entry per page Table 29's arrangement is showing**, since the six-hundred-and-tenth
    /// session. Under a column this host used to publish the current page's tree while the window
    /// showed four, which told a screen reader the document was one page long — the sharpest form
    /// of trap 5, because the person it misleads is the one for whom the picture is no answer.
    ///
    /// **Everything is copied before the bridge is touched.** `Query::Reports` borrows the
    /// viewer, and the bridge is a field of the same struct; owning the answer first is what lets
    /// both be reached in one function without the borrow checker having to be argued with.
    ///
    /// Does nothing until [`App::accessibility`] exists, which is after the first present.
    fn speak(&mut self) {
        if self.accessibility.is_none() {
            return;
        }
        let document = self.named().to_owned();
        let window = format!("{document} — {}", self.caption);
        #[expect(
            clippy::cast_precision_loss,
            reason = "a viewport in device pixels; f32 is exact to 2^24 and no display is"
        )]
        let viewport = self.window().map_or((0.0, 0.0), |(width, height, _)| {
            (width as f32, height as f32)
        });
        let (current, pages) = match self.viewer.query(Query::CurrentPage) {
            Answer::Page { index, of, .. } => (index, of),
            _ => (0, 0),
        };
        // The structure is the population: it answers for every page the arrangement shows, so a
        // page with no reports and no readback shortfall still gets a node saying it is there.
        let structures = match self.viewer.query(Query::AccessibilityTree) {
            Answer::Accessibility(pages) => pages,
            _ => Vec::new(),
        };
        let reports: Vec<(usize, Vec<String>)> = match self.viewer.query(Query::Reports) {
            Answer::Reports(pages) => pages
                .iter()
                .map(|page| (page.page, page.notes.to_vec()))
                .collect(),
            _ => Vec::new(),
        };
        // The other half of what a person who cannot see the page is owed, and it is a *count*
        // rather than a note: §9.10.2's own "there is no way to determine what the character code
        // represents" is not a refusal of ours and may not join the list above (ADR 0422).
        let readbacks: Vec<(usize, pdf_model::content::Shortfall)> =
            match self.viewer.query(Query::Readback) {
                Answer::Readback(pages) => pages
                    .iter()
                    .map(|page| (page.page, page.shortfall))
                    .collect(),
                _ => Vec::new(),
            };
        let (labels, places) = self.placed_pages(&structures, viewport);
        let elements: usize = structures
            .iter()
            .map(|structure| structure.nodes.len())
            .sum();
        let said: usize = reports.iter().map(|(_, notes)| notes.len()).sum();
        let unreadable: usize = readbacks
            .iter()
            .map(|(_, shortfall)| shortfall.unnamed.total())
            .sum();
        self.trace.say(
            Topic::Access,
            format_args!(
                "accessibility: {} page(s) on screen, {elements} element(s), {said} report(s), \
                 {unreadable} unreadable code(s), current page {}",
                structures.len(),
                current.saturating_add(1)
            ),
        );
        let shown: Vec<viewer_accessibility::PageView<'_>> = structures
            .iter()
            .enumerate()
            .map(|(slot, structure)| viewer_accessibility::PageView {
                page: structure.page,
                label: labels.get(slot).and_then(Option::as_deref),
                bounds: places.get(slot).copied().unwrap_or([0.0, 0.0, 0.0, 0.0]),
                nodes: &structure.nodes,
                reports: reports
                    .iter()
                    .find(|(page, _)| *page == structure.page)
                    .map_or(&[][..], |(_, notes)| notes.as_slice()),
                readback: readbacks
                    .iter()
                    .find(|(page, _)| *page == structure.page)
                    .map_or_else(pdf_model::content::Shortfall::default, |(_, count)| *count),
            })
            .collect();
        let view = viewer_accessibility::DocumentView {
            window: &window,
            document: &document,
            pages,
            viewport,
            shown: &shown,
        };
        if let Some(bridge) = self.accessibility.as_mut() {
            bridge.publish(&view);
        }
    }
}

impl App {
    /// §12.4.2's label and the place on the screen, for each page the arrangement is showing.
    ///
    /// Two answers gathered in one walk because they are asked of the same pages and both are
    /// per page: a label is what a person navigates by and a rectangle is where AT-SPI says the
    /// page is. **The rectangle is the page's own and not the window's** — under a column two
    /// page nodes claiming the viewport would be two nodes claiming one place.
    fn placed_pages(
        &self,
        structures: &[viewer_core::PageStructure],
        viewport: (f32, f32),
    ) -> (Vec<Option<String>>, Vec<[f32; 4]>) {
        let labels = structures
            .iter()
            .map(
                |structure| match self.viewer.query(Query::PageLabel(structure.page)) {
                    Answer::Label(label) => Some(label),
                    _ => None,
                },
            )
            .collect();
        let places = structures
            .iter()
            .map(
                |structure| match self.viewer.query(Query::PageGeometry(structure.page)) {
                    #[expect(
                        clippy::cast_precision_loss,
                        reason = "a raster's extent in device pixels; f32 is exact to 2^24 and \
                                  no display is"
                    )]
                    Answer::Geometry(geometry) => [
                        geometry.origin.0,
                        geometry.origin.1,
                        geometry.origin.0 + geometry.width as f32,
                        geometry.origin.1 + geometry.height as f32,
                    ],
                    _ => [0.0, 0.0, viewport.0, viewport.1],
                },
            )
            .collect();
        (labels, places)
    }
}

/// How far the content must move along one axis to bring `low..high` inside `0..extent`.
///
/// Zero where it is already inside, which is what makes a client's repeated `ScrollTo` on the
/// element it is reading cost nothing.
fn delta(low: f32, high: f32, extent: f32) -> f32 {
    if low < 0.0 || high - low > extent {
        // Above the viewport, or too large to fit: take the near edge to the near edge.
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
    ///
    /// The zero case is the one worth a test rather than a comment: a client that reads a page
    /// asks `Component.ScrollTo` on every element it speaks, so an implementation that centred
    /// each one would move the page under a person on every word. Verified on a real bus — a link
    /// already on the screen answered `ScrollTo` with its extents unchanged.
    #[expect(
        clippy::float_cmp,
        reason = "every one of these is a difference of two of the inputs, so each is exact in \
                  binary and a tolerance would only hide an arithmetic that stopped being one"
    )]
    #[test]
    fn a_span_already_in_the_viewport_is_not_scrolled_to() {
        assert_eq!(delta(10.0, 40.0, 100.0), 0.0);
        assert_eq!(delta(0.0, 100.0, 100.0), 0.0);
        // Below: the far edge comes to the far edge.
        assert_eq!(delta(120.0, 140.0, 100.0), 40.0);
        // Above: the near edge comes to the near edge, and the delta is negative.
        assert_eq!(delta(-30.0, -10.0, 100.0), -30.0);
        // Larger than the viewport: the beginning of it is what a person is being taken to.
        assert_eq!(delta(20.0, 400.0, 100.0), 20.0);
    }
}
