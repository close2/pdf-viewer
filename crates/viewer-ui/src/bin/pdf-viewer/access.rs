//! §14.7's structure handed to AccessKit, and the window position AT-SPI needs with it.
//!
//! Separate from the frame it follows because the ordering is a rule rather than a convenience:
//! nothing here may run before the first page has been presented, and what it costs is measured
//! beside the frame rather than inside it. The three questions it asks are the three only a host
//! knows — what the window is called, which page is showing, and what the page could not draw.

use viewer_core::{Answer, Query};

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
            self.accessibility = Some(viewer_accessibility::Bridge::new());
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

    /// Hands §14.7's structure for the page being shown to the platform's accessibility API.
    ///
    /// **The fifth of the five things `doc/ui-boundary.md` lists as blocked on the
    /// `viewer-core` boundary, and the last.** `Query::AccessibilityTree` has answered since the
    /// hundred-and-forty-ninth session and nothing asked; this asks. What crosses is
    /// `viewer-accessibility`'s business — §14.8.4's types onto AccessKit's roles, and AT-SPI
    /// underneath that — and what is this host's is the three things only a host knows: what the
    /// window is called, which page is showing, and what the page could not draw.
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
        let (page, label, pages) = match self.viewer.query(Query::CurrentPage) {
            Answer::Page {
                index, label, of, ..
            } => (index, label, of),
            _ => (0, None, 0),
        };
        let reports: Vec<String> = match self.viewer.query(Query::Reports) {
            Answer::Reports(notes) => notes.to_vec(),
            _ => Vec::new(),
        };
        let nodes = match self.viewer.query(Query::AccessibilityTree) {
            Answer::Accessibility(nodes) => nodes,
            _ => Vec::new(),
        };
        self.trace.say(
            Topic::Access,
            format_args!(
                "accessibility: {} element(s), {} report(s) on page {}",
                nodes.len(),
                reports.len(),
                page.saturating_add(1)
            ),
        );
        let view = viewer_accessibility::PageView {
            window: &window,
            document: &document,
            page,
            label: label.as_deref(),
            pages,
            viewport,
            nodes: &nodes,
            reports: &reports,
        };
        if let Some(bridge) = self.accessibility.as_mut() {
            bridge.publish(&view);
            // An action nobody carries out is said out loud rather than dropped. The tree this
            // program publishes declares no actions at all, so this list is expected to be empty
            // — and a line here would mean a client asked for something anyway, which is worth
            // knowing (trap 5).
            for asked in bridge.requested() {
                println!(
                    "note: an assistive technology asked for {:?} on node {:?}, which this host \
                     does not carry out",
                    asked.action, asked.node
                );
            }
        }
    }
}
