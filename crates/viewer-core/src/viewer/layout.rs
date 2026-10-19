//! Zoom, scroll and the one place a render is scheduled: [`Viewer::settle`].

use std::sync::Arc;

use pdf_render::{Point, Rect, TargetSpec};

use super::geometry::content_bounds;
use super::{DocumentId, MAX_PIXELS, RenderToken, Turn, Viewer};
#[cfg(doc)]
use crate::Rendered;
use crate::command::{PageTarget, Viewing, Zoom};
use crate::event::{Event, RenderRequest};
use crate::open::{Open, Pending};

impl Viewer {
    /// Moves the page under the viewport, clamped to the raster the host is holding.
    ///
    /// A method rather than an arm of [`Self::act`] because the clamp needs the viewport *and*
    /// the focused document at once, and the borrow that gets one has to end before the other.
    pub(super) fn scroll(&mut self, dx: f32, dy: f32, events: &mut Vec<Event>) {
        let viewport = self.viewport;
        let Some(open) = self.focused_mut() else {
            return;
        };
        open.scroll = (open.scroll.0 + dx, open.scroll.1 + dy);
        // Clamped — and, under a continuous arrangement, carried across the row boundary — in
        // `settle`, which is where the magnification the arrangement is measured in is known.
        events.push(damage(viewport));
    }

    /// Resolves a zoom command into the magnification it lands on.
    ///
    /// A step is resolved here rather than stored, because two steps have to compose and "one
    /// larger than fitted" is not a state a mode can hold. Resolving also holds one point of the
    /// viewport still — the pointer's, or its centre where the command names none — which is what
    /// makes zooming feel like magnification rather than like jumping to a corner. ADR 0166.
    pub(super) fn set_zoom(&mut self, zoom: Zoom, at: Option<(f32, f32)>, events: &mut Vec<Event>) {
        let (viewport, scale) = (self.viewport, self.scale);
        let Some(open) = self.focused_mut() else {
            return;
        };
        let before = open.magnification(viewport, scale);
        match zoom {
            Zoom::In | Zoom::Out => {
                let Some(before) = before else { return };
                open.zoom = Zoom::Scale(Open::stepped(before / scale, zoom));
            }
            Zoom::FitPage | Zoom::FitWidth | Zoom::FitHeight | Zoom::Scale(_) => open.zoom = zoom,
        }
        if let (Some(before), Some(after)) = (before, open.magnification(viewport, scale))
            && before > 0.0
        {
            open.hold(viewport, scale, before, after, at);
        }
        events.push(damage(viewport));
    }

    /// Puts the reader back at a view this crate answered with.
    ///
    /// **Three assignments in one command, and the order is the whole of the implementation.**
    /// The page goes first because a turn is what zeroes the scroll ([`Self::go_to`] says why),
    /// and going second would throw the restored offset away. The magnification goes next because
    /// the scroll is measured in device pixels and the magnification is what makes one — and
    /// through [`Self::set_zoom`] rather than by assignment, so that a `Viewing` built by hand
    /// with [`Zoom::In`] means the step it means everywhere else. The scroll goes last, and it is
    /// the only part written straight down: it is the one component this vocabulary could not
    /// otherwise state, and every other route to it passes through a clamp or an arithmetic.
    ///
    /// [`Self::settle`] clamps what lands here, exactly as it clamps a wheel's delta. A view this
    /// crate answered with was clamped when it was made, so the clamp is a no-op for it; a view a
    /// host composed for itself is held to the same bound as anything else, which is what keeps
    /// the public fields from being a way round the arrangement.
    pub(super) fn restore(&mut self, view: Viewing, events: &mut Vec<Event>) {
        self.go_to(PageTarget::Index(view.page), Turn::Requested, events);
        self.set_zoom(view.zoom, None, events);
        let viewport = self.viewport;
        let Some(open) = self.focused_mut() else {
            return;
        };
        open.scroll = view.scroll;
        events.push(damage(viewport));
    }

    /// The pixel budget a render request is held to, which is the host's tier and not a
    /// property of the page.
    ///
    /// [`MAX_PIXELS`] bounds a raster this crate hands back, so it binds a host that takes one
    /// and says nothing to a host that draws its own frames at its own size. What remains for
    /// both is `TargetSpec::for_page`'s own refusal of a dimension `f32` cannot resolve.
    ///
    /// A host that answers [`Rendered::Listed`] for some pages is the first kind: the budget is
    /// per request and it has not stopped taking rasters.
    ///
    /// **A host that draws an oversized page from its list is the second kind for that page**
    /// ([`crate::Oversize::Listed`]): it allocates what its window shows and not the page, so the
    /// request goes out at the page's whole target and [`Rendered::Listed`] is its answer. Every
    /// page under the bound is asked for exactly as before; which of the two a request is, the
    /// host reads off the target (ADR 1892).
    pub(super) const fn raster_budget(&self) -> u64 {
        if self.holds_rasters && matches!(self.oversize, crate::Oversize::Refused) {
            MAX_PIXELS
        } else {
            u64::MAX
        }
    }

    /// Works out whether what is on the screen is what should be, and asks for a render if not.
    ///
    /// The one place a render is scheduled. Interpretation happens here too, which is what makes
    /// [`Event::NeedsRender`] self-contained — see the crate documentation on who interprets and
    /// who rasterises.
    ///
    /// **Table 29's `/PageLayout` made this a loop over the pages on the screen rather than a
    /// statement about one page.** The order is what it always was and is the whole of what the
    /// function is: the view state, then the magnification, then where the scroll now stands, then
    /// which pages that shows, then their interpretations, then §12.3.2.1's pending view, then one
    /// render request per page whose pixels are not the pixels it should have.
    pub(super) fn settle(&mut self, events: &mut Vec<Event>) {
        let (viewport, scale) = (self.viewport, self.scale);
        // A viewport with no extent is a window that has not been laid out yet, and rendering
        // into it would be a request for a zero-pixel raster.
        if viewport.0 == 0 || viewport.1 == 0 {
            return;
        }
        let Some(id) = self.focused else { return };
        let delegated = self.delegated;
        let Some(open) = self.documents.get_mut(&id) else {
            return;
        };

        // §12.5.3's `NoZoom` makes one annotation's placement a function of the magnification,
        // so the state interpretation reads has to carry it — and **logical** pixels per user
        // unit rather than device ones, because "the same fixed size on the screen" is a size a
        // person sees and a doubled display should draw it sharper rather than smaller.
        //
        // Re-interpreting only where the last interpretation said the page would notice is what
        // keeps the display list's promise, and **the question is asked of each page rather than
        // of the arrangement**: `Open::reinterpret` decides it per page now, because Table 29's
        // continuous arrangements put two pages on the screen and one annotation was costing the
        // interpretation of both. `tools/state.sh` counts the population; ADR 0775 has the price.
        //
        // `reinterpret` and not `stale`, because the ink did not move: a zoom changes where
        // §12.5.3 puts the annotation, not what the document's state is, and a host holding a
        // picture of the old magnification may go on standing in with it while the new
        // interpretation renders. `stale` here superseded the ink, so a host refused that
        // picture as "of another ink" and every zoom step of such a page froze for the length
        // of the real frame — the owner's "zooming is laggy", traced to 234 such refusals in
        // one gesture.
        //
        // **And §8.11.4.5 reads the same number**: "[w]henever there is a change to a factor that
        // the usage application dictionaries with event type View depend on (such as zoom level),
        // the corresponding dictionaries shall be reapplied". A reapplication that moves an
        // optional content group changes what the page *draws*, so the picture a host is holding
        // is of something else and `stale` is owed rather than `reinterpret` — which is why
        // `set_magnification` answers with `Magnified` rather than a `bool`. Nothing in the corpus
        // reaches it (`examples/oc_usage_census`), so the ordinary zoom keeps the paragraph above.
        let magnification = open
            .magnification(viewport, scale)
            .map(|device| device / scale);
        let magnified = open.view.set_magnification(magnification);
        open.view.set_window_view(crate::scripting::window_view(
            open.zoom,
            open.layout,
            magnification,
        ));
        if magnified.supersedes_ink() {
            open.stale();
        } else if magnified.needs_interpreting() {
            open.reinterpret();
        }

        // §6.3.2.2's "unless otherwise instructed", carried to whichever document is on screen.
        // Unconditional where it changed — unlike the magnification above, which asks whether
        // the page has anything that would notice: a page with no widget on it costs one field
        // walk that answers nothing, and a page with one would be wrong to keep.
        if open.view.set_widget_appearances(delegated) {
            open.stale();
        }

        let Some(magnification) = open.magnification(viewport, scale) else {
            return;
        };
        // Table 29's continuous arrangements move the current page when the scroll crosses a row,
        // and a page becoming current is a page turn however it happened — §12.6.3's `/O` and
        // `/C` are owed for it exactly as they are owed for an arrow key.
        let left = crate::layout::settle_scroll(open, viewport, scale, magnification);
        if let Some(left) = left {
            self.announce_page(events);
            self.page_events(id, Some(left), events);
        }

        self.arrange(id, magnification, events);

        // §12.3.2.1's location and magnification, applied here because this is the first place
        // both of the things they need exist: a viewport, and — for the three `/FitB` forms —
        // a display list to measure the page's contents from. ADR 0162.
        let Some(open) = self.documents.get_mut(&id) else {
            return;
        };
        if !open.pending_views.is_empty()
            && let Some(list) = open
                .interpreted()
                .map(|interpreted| Arc::clone(&interpreted.list))
        {
            let bounds = content_bounds(&list);
            let mut changed = false;
            // In order: §O.2 makes left-to-right normative for a fragment identifier, and a
            // document's own `/OpenAction` states at most one of these.
            for view in std::mem::take(&mut open.pending_views) {
                changed |= open.apply_view(view, viewport, scale, bounds);
            }
            if changed {
                for on_screen in &mut open.on_screen {
                    on_screen.shown = None;
                }
                let Some(magnification) = open.magnification(viewport, scale) else {
                    return;
                };
                self.arrange(id, magnification, events);
            }
        }

        self.schedule(id, events);
    }

    /// Places Table 29's arrangement and interprets whatever of it is not interpreted yet.
    ///
    /// Split from [`Self::settle`] because §12.3.2.1's pending view may change the magnification
    /// after the first pass and the arrangement then has to be made again — the same work, and
    /// one description of it rather than two.
    fn arrange(&mut self, id: DocumentId, magnification: f32, events: &mut Vec<Event>) {
        let (viewport, scale, unread) = (self.viewport, self.scale, self.unread);
        let Some(open) = self.documents.get_mut(&id) else {
            return;
        };
        let placements = crate::layout::place(open, viewport, scale, magnification);
        // What has scrolled off the screen is forgotten, which is the eviction rule
        // `Open::on_screen` did not have to invent: the view says what is kept.
        open.on_screen.retain(|on_screen| {
            placements
                .iter()
                .any(|placed| placed.page == on_screen.page)
        });
        for placed in &placements {
            if let Some(on_screen) = open.on_mut(placed.page) {
                on_screen.origin = placed.origin;
                on_screen.raster = placed.raster;
                continue;
            }
            // One page-tree walk per page *arriving* on the screen, and never per frame: this is
            // the cache ADR 0124 put in, with the arrangement deciding its contents.
            let Some(object) = open.page(placed.page) else {
                continue;
            };
            open.on_screen.push(crate::open::OnScreen {
                page: placed.page,
                object,
                origin: placed.origin,
                raster: placed.raster,
                interpreted: None,
                replaceable: None,
                revision: 0,
                shown: None,
                frame: None,
                pending: None,
                refused: None,
            });
        }
        open.on_screen.sort_by_key(|on_screen| on_screen.page);
        // A selection is a range of each of its pages' readbacks, so it lives exactly as long as
        // **every** page it covers is on the screen — which under `SinglePage` is the page turn
        // that used to end it. Not "as long as one of them is": a selection whose middle this
        // crate no longer holds the readback for could answer with text that has a hole in it,
        // and a hole nothing announces is the plausible-looking picture principle 1 is about.
        if open.selection.is_some_and(|chosen| {
            let (first, last) = chosen.ordered();
            (first.page..=last.page).any(|page| open.on(page).is_none())
        }) {
            open.selection = None;
        }

        if !unread {
            Self::interpret_arranged(open, id, events);
        }
    }

    /// Interprets every page of the arrangement that is not interpreted yet.
    ///
    /// [`Self::arrange`]'s second half, and [`Self::anticipate`]'s whole: one description of what
    /// interpreting an arranged page does — its reports, its readback, its revision — so that a
    /// page interpreted ahead of the first viewport is the page the first arrangement would have
    /// interpreted (ADR 1531).
    pub(super) fn interpret_arranged(open: &mut Open, id: DocumentId, events: &mut Vec<Event>) {
        for index in 0..open.on_screen.len() {
            if open.on_screen[index].interpreted.is_some() {
                continue;
            }
            let page = open.on_screen[index].page;
            let Some(read) = crate::open::interpret(open, page) else {
                continue;
            };
            Self::install(open, id, index, read, events);
        }
        // …unless a search put one there for a page that has only now been interpreted, which is
        // the one range made *after* a turn rather than before it. See `Open::pending_selection`.
        if let Some(chosen) = open.pending_selection
            && open.interpretation(chosen.from.page).is_some()
        {
            open.pending_selection = None;
            open.selection = Some(chosen);
        }
    }

    /// Asks for a render of every page whose pixels are not the pixels it should have.
    fn schedule(&mut self, id: DocumentId, events: &mut Vec<Event>) {
        let (viewport, scale) = (self.viewport, self.scale);
        // Read before the document is borrowed: it is a fact about the host, not the page.
        let budget = self.raster_budget();
        let mut token = self.next_token;
        let Some(open) = self.documents.get_mut(&id) else {
            return;
        };
        let Some(magnification) = open.magnification(viewport, scale) else {
            return;
        };
        let mut requests = Vec::new();
        let mut refusals = Vec::new();
        // Read once, outside the loop, because it is a fact about the document rather than about
        // one of its pages: every request this pass makes is of the same ink.
        let ink = open.ink;
        for on_screen in &mut open.on_screen {
            let Some(interpreted) = on_screen.interpreted.as_ref() else {
                continue;
            };
            let list = Arc::clone(&interpreted.list);
            let target = match TargetSpec::for_page(&list, magnification, budget) {
                Ok(target) => target,
                // Named rather than clamped, because a page silently drawn at a scale nobody chose
                // is a page a person has been told something false about. **Named once for the
                // view it is about**: every command settles, a pointer moving over the window
                // included, and a refusal said again at each of them is a sentence a person sees
                // a hundred times a second for nothing that changed. A new magnification or a new
                // interpretation is a new question, and is answered again (ADR 1892).
                Err(error) => {
                    let view = (magnification.to_bits(), on_screen.revision);
                    if on_screen.refused.replace(view) != Some(view) {
                        refusals.push((
                            on_screen.page,
                            format!("this page cannot be drawn at this size: {error}"),
                        ));
                    }
                    continue;
                }
            };
            on_screen.refused = None;
            let showing = on_screen.shown == Some((target, on_screen.revision));
            let asked = on_screen.pending.as_ref().is_some_and(|pending| {
                pending.target == target && pending.revision == on_screen.revision
            });
            if showing || asked {
                continue;
            }
            let issued = RenderToken(token);
            token = token.saturating_add(1);
            on_screen.pending = Some(Pending {
                token: issued,
                page: on_screen.page,
                target,
                revision: on_screen.revision,
            });
            requests.push(Event::NeedsRender(RenderRequest {
                token: issued,
                document: id,
                page: on_screen.page,
                list,
                target,
                ink,
            }));
        }
        self.next_token = token;
        for (page, note) in refusals {
            events.push(Event::Reported {
                document: id,
                page: Some(page),
                notes: vec![note],
            });
        }
        events.extend(requests);
    }
}

/// The whole viewport, which is what changes when a frame arrives or a page scrolls.
///
/// A tighter rectangle is available for a scroll — everything but the exposed strip is a
/// translation of what is already on screen — and it is not computed, because a host that blits
/// from a held raster pays for the whole viewport either way and one that scrolls by copying is
/// optimising a case nobody has measured.
pub(super) fn damage(viewport: (u32, u32)) -> Event {
    Event::Damage(Rect::from_corners(
        Point::new(0.0, 0.0),
        Point::new(px(viewport.0), px(viewport.1)),
    ))
}

/// A pixel count as a coordinate.
///
/// Exact for every extent a raster may have: [`pdf_render::MAX_EXTENT`] is 2²⁴, which is the
/// largest integer an `f32` represents exactly. A *viewport* larger than that would round, and
/// no display is.
pub(crate) fn px(value: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "exact below 2^24, which bounds every raster extent; see the doc comment"
    )]
    let value = value as f32;
    value
}

/// Turns a [`PageTarget`] into an index, clamped to the document.
pub(super) fn resolve(open: &Open, target: PageTarget) -> Option<usize> {
    let (current, count) = (open.page_index, open.page_count);
    let last = count.checked_sub(1)?;
    Some(match target {
        PageTarget::Index(index) => index.min(last),
        PageTarget::First => 0,
        PageTarget::Last => last,
        PageTarget::Next => current.saturating_add(1).min(last),
        PageTarget::Previous => current.saturating_sub(1),
        PageTarget::Relative(delta) => current.saturating_add_signed(delta).min(last),
    })
}
