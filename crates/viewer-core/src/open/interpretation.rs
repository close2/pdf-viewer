//! The interpretations an open document keeps of its pages, and when they go stale.
//!
//! What the view state moving discards, what a magnification change keeps, the page a point of the
//! screen falls on, and the selection's ranges over each page's readback.

use super::{Interpreted, OnScreen, Open};
use crate::viewer::px;

impl Open {
    /// Drops everything derived from what the page draws, because the view state moved.
    ///
    /// The one place that says what "the ink is stale" means, and it means two things now rather
    /// than one: the display list of the page showing, and the readback of every page a search
    /// has read. Both are functions of the document *and* [`Self::view`], and §8.11's layer
    /// switch, §12.7.5's field value, §12.5.5's appearance under the pointer and §6.3.2.2's
    /// delegated widgets each move the second of those two.
    ///
    /// **Every page's readback goes, not the page that changed.** A layer switched or a value
    /// typed is a change to the *state*, and the state is what every cached page was interpreted
    /// against; the precise alternative — forgetting only the page an annotation sits on — needs
    /// an invariant about which page that is, maintained at two call sites, and a search that
    /// returned different results after this change would be a defect rather than a speed-up.
    /// The cost is that a layer switch makes the next search cold again, which is written down
    /// in ADR 0256 as the deliberate half of the trade.
    /// **And it is where [`Self::ink`] moves**, for the same reason it is where the display lists
    /// go: a host holding pixels of a page cannot tell a re-interpretation of unchanged ink from
    /// an interpretation of changed ink, and this is the only place in this crate that knows.
    /// **And it is where the replacements go**, which is the difference between this and
    /// [`Self::reinterpret`] stated once more in the other direction: §12.5.3's content half was
    /// interpreted against the view state as it then was, so a layer switched or a value typed
    /// invalidates it exactly as it invalidates the display list beside it.
    pub(crate) fn stale(&mut self) {
        self.ink = self.ink.saturating_add(1);
        for on_screen in &mut self.on_screen {
            on_screen.interpreted = None;
            on_screen.replaceable = None;
        }
        self.readbacks.clear();
    }

    /// The display lists go and the ink stays: a re-interpretation the *view* asked for.
    ///
    /// **The one caller is §12.5.3's magnification path, and the distinction is what a host may
    /// show while the new interpretation renders.** A `NoZoom` annotation's placement is a
    /// function of the magnification, so a zoom re-interprets the page — but nothing about the
    /// document or [`Self::view`]'s ink moved: the same layers are on, the same values are in
    /// the fields, the same appearance is under the pointer. A picture of the old magnification
    /// is therefore still a picture of this page's ink, approximately placed — exactly what a
    /// stand-in is, and the opposite of the superseded-ink case [`Self::stale`] exists for,
    /// where a sharp picture of switched-off ink would assert something false. Bumping
    /// [`Self::ink`] here made every zoom of such a page freeze for the length of the real
    /// frame, because a host correctly refuses to show a picture of other ink.
    ///
    /// **Only the pages the clause is about.** `Interpretation::view_dependent` records whether
    /// a page carries an annotation whose placement §12.5.3 makes a function of the
    /// magnification, and a page that carries none has a display list the magnification cannot
    /// change — so re-interpreting it produces the list it already holds, at the price of the
    /// page. That price is per page and Table 29's continuous arrangements show more than one:
    /// a column scrolled across a boundary is two pages, of which one was being re-interpreted
    /// because the *other* one has an annotation on it. ADR 0775 has the measurement.
    ///
    /// The readbacks go with the lists — not because the text moved, but because the two are
    /// invalidated together everywhere else and a cache with two lifetimes needs an argument
    /// this one has not earned: re-interpretation repopulates it on the next pass. **Per page,
    /// for the same reason the lists are**: a page whose display list is kept cannot have a
    /// readback that moved, so a gesture no longer makes a document-wide search cold — which,
    /// on ISO 32000-2, was 1023 pages of readback dropped per wheel notch.
    ///
    /// **And what replaces the list is the annotation pass rather than the page**, which is the
    /// other half of ADR 0777: [`OnScreen::replaceable`] survives this, so the next `arrange`
    /// runs §12.5.3's pass again over the content half `pdf-model` kept instead of reading the
    /// content stream a second time. That is why this drops the list rather than rebuilding it
    /// here: the work belongs where every other interpretation of an arranged page happens, and
    /// a page about to leave the screen should not pay for a placement nothing will draw.
    pub(crate) fn reinterpret(&mut self) {
        for on_screen in &mut self.on_screen {
            let view_dependent = on_screen
                .interpreted
                .as_ref()
                .is_some_and(|interpreted| interpreted.view_dependent);
            if view_dependent {
                on_screen.interpreted = None;
                self.readbacks.forget(on_screen.page);
            }
        }
    }

    /// The arrangement's entry for a page, where the arrangement shows it.
    pub(crate) fn on(&self, page: usize) -> Option<&OnScreen> {
        self.on_screen
            .iter()
            .find(|on_screen| on_screen.page == page)
    }

    /// The same, mutably.
    pub(crate) fn on_mut(&mut self, page: usize) -> Option<&mut OnScreen> {
        self.on_screen
            .iter_mut()
            .find(|on_screen| on_screen.page == page)
    }

    /// The current page's interpretation, where it has one.
    ///
    /// What was one field until Table 29's arrangements arrived. Every question this crate
    /// answers about "the page being shown" comes through here, so that *which* page that is
    /// stays one decision rather than one per caller.
    pub(crate) fn interpreted(&self) -> Option<&Interpreted> {
        self.on(self.page_index)?.interpreted.as_ref()
    }

    /// The interpretation of any page the arrangement shows.
    pub(crate) fn interpretation(&self, page: usize) -> Option<&Interpreted> {
        self.on(page)?.interpreted.as_ref()
    }

    /// What is selected, as a range of each page's own readback, in page order.
    ///
    /// **This is where a selection stops being two points and becomes text**, and it is one
    /// function so that the text, the shapes, §14.8.2.5's logical order and §12.5.6.10's markup
    /// cannot disagree about what was chosen. The first page runs from the earlier end to *its*
    /// end, the last from *its* beginning to the later end, and a page between the two is
    /// selected whole — which is what dragging across a boundary means and is the only reading a
    /// column offers.
    ///
    /// Empty where nothing is selected. A page whose interpretation this crate does not hold
    /// contributes nothing rather than an empty range: it can only be a page the arrangement is
    /// not showing, and a selection whose pages have left the screen is dropped by `settle`
    /// before it can be asked about.
    pub(crate) fn spans(&self) -> Vec<(usize, (usize, usize))> {
        let Some(chosen) = self.selection else {
            return Vec::new();
        };
        let (from, to) = chosen.ordered();
        (from.page..=to.page)
            .filter_map(|page| {
                let readback = self.interpretation(page)?.text.len();
                let start = if page == from.page {
                    from.offset.min(readback)
                } else {
                    0
                };
                let end = if page == to.page {
                    to.offset.min(readback)
                } else {
                    readback
                };
                (start <= end).then_some((page, (start, end)))
            })
            .collect()
    }

    /// Which page of the arrangement a viewport point is inside, where it is inside one.
    ///
    /// `None` for a point in the gap between two pages, in the margin beside them, or over
    /// nothing at all — a caller that must answer anyway falls back to the current page and says
    /// so, because a drag that has left the page is still a drag inside its text.
    pub(crate) fn placed_at(&self, at: (f32, f32)) -> Option<&OnScreen> {
        self.on_screen.iter().find(|on_screen| {
            at.0 >= on_screen.origin.0
                && at.0 < on_screen.origin.0 + px(on_screen.raster.0)
                && at.1 >= on_screen.origin.1
                && at.1 < on_screen.origin.1 + px(on_screen.raster.1)
        })
    }
}
