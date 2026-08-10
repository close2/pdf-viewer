//! A search across a whole document, one page per step.
//!
//! [`crate::Query::Find`] answers for the page being shown, and that is all a highlight over what
//! is on the screen needs. **Annex O asks for the other question**, and asks it as a `shall` on a
//! processor opening a URI. ISO 32000-2 §O.2.2, Table Annex O.4's `search`:
//!
//! > Open the document and search for one or more words, selecting the first matching word in the
//! > document.
//!
//! "In the document" is the whole of the difference: answering it means reading pages that are not
//! on the screen, and reading a page means interpreting it. On ISO 32000-2's own 1023 pages a full
//! sweep is **5.84 s** and the readback of all of them is 2.66 MB (measured in the
//! four-hundred-and-fourteenth session, `--profile gates`), so this cannot be a [`crate::Query`]:
//! `Viewer::query` takes `&self`, is asked at pointer speed, and would pay that again on every
//! keystroke.
//!
//! # One page per step, and the host decides how many steps a frame gets
//!
//! `doc/ui-boundary.md`'s rule 4 is "no threads the core was not handed, and no blocking", and
//! rule 3 is that there is no clock here — so this crate cannot spend a *time* budget and cannot
//! spawn a worker to spend it elsewhere. What it can do is the smallest honest unit of work:
//! [`Searching::step`] reads **one page**, which the same measurement puts at 5.7 ms on average,
//! and says how many are left. A host pumps [`crate::Find::Continue`] until the count reaches
//! zero, at whatever rate its own frame budget allows, exactly as it answers
//! [`crate::Event::NeedsRender`] rather than being handed a finished page.
//!
//! # The readback is kept, under a bound, and this module still knows nothing about that
//!
//! ADR 0250 declined to keep it — "a readback cache with a byte budget and an invalidation rule
//! for every view-state change that alters what a page draws" — and the four-hundred-and-twentieth
//! session built exactly that, because the byte budget it was missing turned out to be a number:
//! the whole of ISO 32000-2 reads back as 2.66 MB. `crate::readback` is the cache,
//! `crate::open::Open::stale` is the invalidation rule, and a **repeated** full-document sweep
//! fell from 5.45 s to 7.27 ms while the first one did not move (ADR 0256).
//!
//! What did *not* change is anything here: [`Searching`] is handed a `&str` and has never known
//! where it came from, which is why the cache cost this module no lines at all.

use crate::command::FindDirection;

/// A document-wide search, and how far through the document it has got.
///
/// **Any of the needles matching is a match**, because Annex O's argument is a list: "[t]he
/// wordList argument defines the search words and shall be a string enclosed within quotation
/// marks comprised of individual words separated by space characters". A find bar starts one with
/// a single needle, which is the same shape with a list of one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Searching {
    /// What is being looked for. Empty is refused by [`Searching::new`].
    pub(crate) needles: Vec<String>,
    /// Which way through the document the pages are read.
    pub(crate) direction: FindDirection,
    /// The page the search began on, and how far into that page's readback.
    origin: (usize, usize),
    /// The page [`Searching::step`] reads next.
    next: usize,
    /// How many pages are still to be read, this one included.
    pub(crate) remaining: usize,
    /// Whether the scan has passed the document's end (or, going backwards, its beginning).
    pub(crate) wrapped: bool,
    /// Whether it is allowed to.
    wrap: bool,
    /// Whether the origin page has been read once already.
    ///
    /// Only a wrapping search reads it twice, and the two visits look at different halves of it:
    /// the first at the readback after the place the search started, the second at the readback
    /// before it. Without that split a search started on a match would find that match again.
    passed_origin: bool,
    /// How many pages the document has, kept so that a step can wrap without asking again.
    pages: usize,
}

impl Searching {
    /// A search over `pages` pages, beginning at `from` and reading in `direction`.
    ///
    /// `from` is a page index and a byte offset into that page's readback: going forwards the
    /// scan takes matches beginning at or after it, going backwards those ending at or before it,
    /// so a host passing the current selection's far end gets *the next one* rather than the one
    /// it is already on.
    ///
    /// `wrap` is a find bar's convention and not the annex's: Annex O's `search` wants the first
    /// match "in the document" and so starts at the beginning and stops at the end.
    ///
    /// `None` for a document with no pages and for an empty word list, which are the two
    /// questions with no answer rather than the answer "nothing".
    pub(crate) fn new(
        needles: Vec<String>,
        direction: FindDirection,
        from: (usize, usize),
        pages: usize,
        wrap: bool,
    ) -> Option<Self> {
        if pages == 0 || needles.iter().all(String::is_empty) {
            return None;
        }
        let start = from.0.min(pages.saturating_sub(1));
        let remaining = if wrap {
            // Every page once, and the origin twice: the half after the starting point on the
            // way out and the half before it on the way round.
            pages.saturating_add(1)
        } else {
            match direction {
                FindDirection::Forward => pages.saturating_sub(start),
                FindDirection::Backward => start.saturating_add(1),
            }
        };
        Some(Self {
            needles,
            direction,
            origin: (start, from.1),
            next: start,
            remaining,
            wrapped: false,
            wrap,
            passed_origin: false,
            pages,
        })
    }

    /// The page the next call to [`Searching::step`] reads.
    pub(crate) fn page(&self) -> usize {
        self.next
    }

    /// Whether there is anything left to read.
    pub(crate) fn is_done(&self) -> bool {
        self.remaining == 0
    }

    /// Looks in one page's readback and moves on, answering the range of a match where there is
    /// one.
    ///
    /// The range indexes the text it was handed, which is what makes it usable as a selection:
    /// `Interpretation::text` for that page is the string every other offset in this crate is
    /// into. A step that finds something leaves the search finished — `remaining` is zero — so a
    /// host that keeps pumping is told nothing more.
    pub(crate) fn step(&mut self, text: &str) -> Option<(usize, usize)> {
        if self.remaining == 0 {
            return None;
        }
        let page = self.next;
        let found = self.matching(page, text);
        if found.is_some() {
            self.remaining = 0;
            return found;
        }
        self.advance(page);
        None
    }

    /// A step over a page this crate could not interpret at all.
    ///
    /// A page that will not draw is not a page with no text in it, and the difference is worth
    /// keeping: the search moves past it rather than stopping, which is what a reader expects of
    /// a damaged page in the middle of a document.
    pub(crate) fn skip(&mut self) {
        if self.remaining == 0 {
            return;
        }
        let page = self.next;
        self.advance(page);
    }

    /// The match this visit to `page` takes, out of every needle's occurrences.
    ///
    /// Forwards it is the earliest and backwards the latest, over all the needles at once, so a
    /// word list behaves as one search rather than as several run in the list's order.
    fn matching(&self, page: usize, text: &str) -> Option<(usize, usize)> {
        let first_visit = page == self.origin.0 && !self.passed_origin;
        let second_visit = page == self.origin.0 && self.passed_origin;
        let mut best: Option<(usize, usize)> = None;
        for needle in &self.needles {
            for (from, to) in crate::select::find(text, needle) {
                let allowed = match (first_visit, second_visit, self.direction) {
                    (true, _, FindDirection::Forward) => from >= self.origin.1,
                    (true, _, FindDirection::Backward) => to <= self.origin.1,
                    (_, true, FindDirection::Forward) => from < self.origin.1,
                    (_, true, FindDirection::Backward) => to > self.origin.1,
                    _ => true,
                };
                if !allowed {
                    continue;
                }
                let better = match (best, self.direction) {
                    (None, _) => true,
                    (Some((was, _)), FindDirection::Forward) => from < was,
                    (Some((was, _)), FindDirection::Backward) => from > was,
                };
                if better {
                    best = Some((from, to));
                }
            }
        }
        best
    }

    /// Moves to the page after (or before) the one just read.
    fn advance(&mut self, page: usize) {
        self.remaining = self.remaining.saturating_sub(1);
        if page == self.origin.0 {
            self.passed_origin = true;
        }
        if self.remaining == 0 {
            return;
        }
        match self.direction {
            FindDirection::Forward => {
                if page.saturating_add(1) >= self.pages {
                    self.next = 0;
                    self.wrapped = self.wrap;
                } else {
                    self.next = page.saturating_add(1);
                }
            }
            FindDirection::Backward => {
                if page == 0 {
                    self.next = self.pages.saturating_sub(1);
                    self.wrapped = self.wrap;
                } else {
                    self.next = page.saturating_sub(1);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Searching;
    use crate::command::FindDirection;

    /// Three pages of readback, the middle one holding two occurrences.
    fn pages() -> [&'static str; 3] {
        ["a needle here", "nothing", "needle and needle"]
    }

    /// Drives a search to its answer, the way a host pumps it, and says which page and range.
    fn run(mut search: Searching) -> (Option<(usize, usize, usize)>, usize) {
        let pages = pages();
        let mut steps = 0_usize;
        while !search.is_done() {
            let page = search.page();
            steps = steps.saturating_add(1);
            let text = pages.get(page).copied().unwrap_or_default();
            if let Some((from, to)) = search.step(text) {
                return (Some((page, from, to)), steps);
            }
        }
        (None, steps)
    }

    /// The plan is every page once, the origin twice when it wraps, and the halves do not overlap.
    #[test]
    fn a_wrapping_search_reads_the_origin_page_twice_and_finds_each_occurrence_once() {
        let forward = |from: (usize, usize)| {
            Searching::new(
                vec!["needle".to_owned()],
                FindDirection::Forward,
                from,
                3,
                true,
            )
            .expect("three pages and one needle")
        };
        assert_eq!(run(forward((0, 0))).0, Some((0, 2, 8)), "the first one");
        assert_eq!(
            run(forward((0, 8))).0,
            Some((2, 0, 6)),
            "past it, so the next is two pages on"
        );
        assert_eq!(
            run(forward((2, 6))).0,
            Some((2, 11, 17)),
            "the second on the same page"
        );
        // Started past the last occurrence: the scan runs off the end, comes round and lands on
        // page 0's — which is the half of the origin page the first visit did not look at.
        let (found, steps) = run(forward((2, 17)));
        assert_eq!(found, Some((0, 2, 8)), "wrapped to the beginning");
        assert_eq!(steps, 2, "page 2, then page 0");
    }

    /// Backwards takes the latest match before where it started, and it also wraps.
    #[test]
    fn a_backward_search_takes_the_occurrence_before_the_one_it_started_on() {
        let backward = |from: (usize, usize)| {
            Searching::new(
                vec!["needle".to_owned()],
                FindDirection::Backward,
                from,
                3,
                true,
            )
            .expect("three pages and one needle")
        };
        assert_eq!(
            run(backward((2, 17))).0,
            Some((2, 11, 17)),
            "the last one on the page, ending at the offset"
        );
        assert_eq!(
            run(backward((2, 11))).0,
            Some((2, 0, 6)),
            "the one before it"
        );
        assert_eq!(
            run(backward((2, 6))).0,
            Some((2, 0, 6)),
            "an occurrence ending exactly where the search began is before it"
        );
        assert_eq!(
            run(backward((2, 0))).0,
            Some((0, 2, 8)),
            "two pages back, over an empty one"
        );
    }

    /// A search that has no answer reads every page and says so rather than stopping early.
    ///
    /// The count is what a host shows as progress and what Annex O's `search` costs: one step per
    /// page, and the annex's own search does not wrap, so it is one step per page and no more.
    #[test]
    fn a_search_with_no_match_reads_the_whole_plan_and_answers_nothing() {
        let wrapping = Searching::new(
            vec!["absent".to_owned()],
            FindDirection::Forward,
            (1, 0),
            3,
            true,
        )
        .expect("three pages");
        assert_eq!(run(wrapping), (None, 4), "three pages, the origin twice");

        let annex = Searching::new(
            vec!["absent".to_owned(), "missing".to_owned()],
            FindDirection::Forward,
            (0, 0),
            3,
            false,
        )
        .expect("three pages");
        assert_eq!(run(annex), (None, 3), "no wrap: each page once");

        assert_eq!(
            Searching::new(
                vec!["a".to_owned()],
                FindDirection::Forward,
                (0, 0),
                0,
                true
            ),
            None,
            "a document with no pages has no answer rather than the answer nothing"
        );
        assert_eq!(
            Searching::new(vec![String::new()], FindDirection::Forward, (0, 0), 3, true),
            None,
            "and neither has an empty word list"
        );
    }
}
