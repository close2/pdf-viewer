//! Find: the needle, the page being searched and the matches a host draws.

use std::sync::Arc;

use super::layout::damage;
use super::{DocumentId, Viewer};
use crate::command::{Command, Find, FindDirection, PageTarget};
use crate::event::{Event, Found};
use crate::open::{Chosen, Open};

impl Viewer {
    /// Annex O's `search`, and a find bar's *next*: one page of a document-wide search.
    ///
    /// The three verbs of [`Find`] fall into two: `Stop` throws the plan away, and the other two
    /// end in exactly one step being taken — `Start` after making a new plan, `Continue` against
    /// the one already there. A `Continue` with nothing in progress does nothing and says
    /// nothing, which is the right answer for a host that pumped once too often.
    pub(super) fn find(&mut self, find: Find, events: &mut Vec<Event>) {
        let Some(id) = self.focused else {
            return;
        };
        match find {
            Find::Stop => {
                if let Some(open) = self.focused_mut() {
                    open.searching = None;
                }
                return;
            }
            Find::Start { needle, direction } => {
                let Some(open) = self.focused_mut() else {
                    return;
                };
                // Where the *next* one is after. The far end of what is selected in the
                // direction of travel, so a search started while sitting on an occurrence moves
                // off it rather than finding it again — and with nothing selected, the whole
                // page in that direction. `usize::MAX` going backwards is the end of any
                // readback there could be, which is what "before nothing in particular" means.
                // Only where the selection is on the page a search starts from, which is the
                // current one: an offset into another page's readback would name a place in this
                // one that nobody selected.
                let here = open
                    .selection
                    .map(Chosen::ordered)
                    .and_then(|(first, last)| {
                        (first.page <= open.page_index && open.page_index <= last.page)
                            .then_some((first, last))
                    });
                let from = match direction {
                    // The end of what is selected *on this page*: a selection that runs on past
                    // the page the search starts from has its far end in another readback, and
                    // the whole of this one is what lies ahead here.
                    FindDirection::Forward => here.map_or(0, |(_, last)| {
                        if last.page == open.page_index {
                            last.offset
                        } else {
                            usize::MAX
                        }
                    }),
                    FindDirection::Backward => here.map_or(usize::MAX, |(first, _)| {
                        if first.page == open.page_index {
                            first.offset
                        } else {
                            0
                        }
                    }),
                };
                let pages = open.page_count;
                open.searching = crate::search::Searching::new(
                    vec![needle],
                    direction,
                    (open.page_index, from),
                    pages,
                    true,
                );
                if open.searching.is_none() {
                    // An empty string, or a document with no pages: there is no answer to give,
                    // and a host waiting for one has to be told so rather than left pumping.
                    events.push(Event::Searched {
                        document: id,
                        found: None,
                        remaining: 0,
                        wrapped: false,
                    });
                    return;
                }
            }
            Find::Continue => {}
        }
        self.find_step(id, events);
    }

    /// A page's readback, out of the cache where it is there and by interpreting it where it
    /// is not.
    ///
    /// **This is the whole cost of a search step.** `interpret` is the expensive half of this
    /// program — 5.4 ms a page over ISO 32000-2's 1023, of which §7.7.3.2's page tree is 19% and
    /// the content stream 81% — and without this every step would pay it, including for a page
    /// a search had read a moment before. `crate::readback` is the bound that makes keeping it
    /// acceptable (ADR 0256); a **repeated** full-document sweep of ISO 32000-2
    /// is 5.45 s without this and 7.27 ms with it, medians of seven runs of
    /// `viewer-core/examples/find_cost`, and the *first* sweep is unchanged.
    ///
    /// `None` is a page this program could not interpret at all, which is not the same as a page
    /// with no text on it and is not cached: a failure costs nothing to reproduce and caching it
    /// would put a second meaning into a map whose entries are otherwise readbacks.
    fn readback(
        &mut self,
        id: DocumentId,
        page: usize,
    ) -> Option<(Arc<str>, Arc<crate::select::Order>)> {
        if let Some(open) = self.documents.get_mut(&id)
            && let Some(held) = open.readbacks.get(page)
        {
            return Some(held);
        }
        let open = self.documents.get(&id)?;
        let read = crate::open::interpret(open, page)?;
        let order = Arc::new(crate::select::Order::of(
            &read.interpretation.text,
            &read.interpretation.text_layer,
        ));
        let text: Arc<str> = Arc::from(read.interpretation.text);
        if let Some(open) = self.documents.get_mut(&id) {
            open.readbacks.put(page, &text, &order);
        }
        Some((text, order))
    }

    /// Reads the one page the search is on, and says what it found there.
    ///
    /// Split from [`Self::find`] because the borrow is: interpreting a page needs `&Open` and
    /// stepping the plan needs `&mut`, and doing both in one function would mean holding the
    /// first across the second.
    fn find_step(&mut self, id: DocumentId, events: &mut Vec<Event>) {
        let Some(page) = self
            .documents
            .get(&id)
            .and_then(|open| open.searching.as_ref())
            .filter(|searching| !searching.is_done())
            .map(crate::search::Searching::page)
        else {
            return;
        };
        let text = self.readback(id, page);
        let Some(open) = self.documents.get_mut(&id) else {
            return;
        };
        let Some(searching) = open.searching.as_mut() else {
            return;
        };
        // A page this program cannot interpret is stepped over rather than treated as a page with
        // no text on it: the reports for it are the render path's business, and a search that
        // stopped at the first damaged page would be a worse answer than one that did not.
        let found = if let Some((text, order)) = text.as_ref() {
            searching.step(text, order)
        } else {
            searching.skip();
            None
        };
        let (remaining, wrapped) = (searching.remaining, searching.wrapped);
        let Some(range) = found else {
            if remaining == 0 {
                open.searching = None;
            }
            events.push(Event::Searched {
                document: id,
                found: None,
                remaining,
                wrapped,
            });
            return;
        };
        open.searching = None;
        // "[S]electing the first matching word in the document" — the annex's own verb, and this
        // crate has exactly one thing that means: the range a host draws its selection over. It
        // waits for the page because the turn below ends the selection that was on the old one.
        open.pending_selection = Some(Chosen::in_page(page, range));
        let viewport = self.viewport;
        self.act(Command::GoTo(PageTarget::Index(page)), events);
        events.push(Event::Searched {
            document: id,
            found: Some(Found { page, range }),
            remaining: 0,
            wrapped,
        });
        events.push(damage(viewport));
    }

    /// Every occurrence of `needle` on the page being shown, as shapes in device pixels.
    pub(super) fn found(&self, open: &Open, needle: &str) -> Vec<Vec<[f32; 8]>> {
        let Some(interpreted) = open.interpreted() else {
            return Vec::new();
        };
        let order = crate::select::Order::of(&interpreted.text, &interpreted.placed);
        crate::select::find(&interpreted.text, needle, &order)
            .into_iter()
            .map(|range| self.device_quads(open, open.page_index, range))
            .collect()
    }
}
