//! The readback of pages a search has already read, bounded and evicted least-recently-used.
//!
//! A document-wide search reads one page per step and each step **interprets** that page, which
//! is the expensive half of this program: 5.4 ms a page over ISO 32000-2's 1023, of which 19% is
//! §7.7.3.2's page tree walked from the root and 81% is the content stream. ADR 0250 declined to
//! keep any of that, on the ground that a cache needs "a byte budget and an invalidation rule";
//! the budget it lacked was a number, and the project owner supplied one — below 10 MB is
//! definitely acceptable, 1 GB is not.
//!
//! # What is kept, and what a bound is worth
//!
//! Only [`pdf_model::Interpretation::text`] — the readback, and not the display list, the text
//! layer or the decoded images. That is what makes the number small: the whole of ISO 32000-2
//! reads back as **2 658 697 bytes**, 2.6 KB a page, for the largest document this project owns.
//! The display list for one page of it is larger than that.
//!
//! [`BUDGET`] is the ceiling and it is in one place. Above it the least recently used page's
//! readback is dropped, which is the same shape as `pdf_model`'s `MASK_BUDGET` and the confined
//! worker's address-space ceiling: a viewer may spend memory on a large document, and what makes
//! that acceptable is that the spending is bounded and can be read off rather than that it is
//! small.
//!
//! # Purity is about the answer, not about how fast it is reached
//!
//! `pdf_model::content::interpret_with` stays a pure function of a document, a page and a view
//! state — the oracle's 1794-page comparison rests on exactly that — so this cache lives *beside*
//! it, in the viewer's per-document state, and never inside it. What it holds is invalidated
//! whenever the view state that produced it moves; [`crate::open::Open::stale`] is the one place
//! that says so.

use std::collections::BTreeMap;
use std::sync::Arc;

/// How many bytes of readback one open document may hold.
///
/// **4 MiB, and the number is measured rather than chosen.** ISO 32000-2's 1023 pages — the
/// largest document this project owns, against a corpus whose largest is `freeculture.pdf`'s 352
/// — read back as 2 658 697 bytes in total, so the whole of it fits here with 1.5 MB to spare and
/// no eviction ever runs on any document this tree has seen. Four documents open at once is 16
/// MB, which is the same order as the "below 10 MB is definitely ok" the project owner gave, and
/// a document larger than every one of them degrades to eviction rather than to unbounded growth.
///
/// What it buys, measured in the four-hundred-and-twentieth session with
/// `viewer-core/examples/find_cost`, medians of seven runs: a **repeated** full-document search
/// of ISO 32000-2 falls from **5.45 s to 7.27 ms**, and the first one is 5.51 s against 5.61 s —
/// unchanged, inside a spread of 0.16 s and 0.32 s respectively.
pub(crate) const BUDGET: usize = 4 * 1024 * 1024;

/// One page's readback and when it was last wanted.
#[derive(Debug)]
struct Entry {
    /// The readback itself. `Arc` rather than `String` because a hit hands it to a caller that
    /// holds it across a `&mut` borrow of the same document, and a 2.6 KB copy per step would be
    /// the whole cost of a hit.
    text: Arc<str>,
    /// The value of [`Readbacks::clock`] when this entry was last read or written.
    used: u64,
}

/// The readbacks one open document is holding, under a byte budget.
#[derive(Debug)]
pub(crate) struct Readbacks {
    /// What is held, by zero-based page index.
    held: BTreeMap<usize, Entry>,
    /// How many bytes of readback that is, kept rather than summed on every insertion.
    bytes: usize,
    /// The ceiling those bytes are held under.
    budget: usize,
    /// A counter that only goes up, which is what "least recently used" is ordered by.
    clock: u64,
    /// How many lookups found what they wanted, since the document opened.
    hits: u64,
    /// How many did not, which is how many pages were interpreted.
    misses: u64,
    /// How many entries the budget has dropped.
    evicted: u64,
}

impl Default for Readbacks {
    fn default() -> Self {
        Self::with_budget(BUDGET)
    }
}

impl Readbacks {
    /// A cache holding at most `budget` bytes of readback.
    ///
    /// The budget is a parameter so that the eviction rule can be exercised by a test on a
    /// budget of a few bytes; every document opens with [`BUDGET`].
    pub(crate) fn with_budget(budget: usize) -> Self {
        Self {
            held: BTreeMap::new(),
            bytes: 0,
            budget,
            clock: 0,
            hits: 0,
            misses: 0,
            evicted: 0,
        }
    }

    /// A page's readback, where it is held, marking it as the most recently used.
    pub(crate) fn get(&mut self, page: usize) -> Option<Arc<str>> {
        self.clock = self.clock.saturating_add(1);
        let clock = self.clock;
        let Some(entry) = self.held.get_mut(&page) else {
            self.misses = self.misses.saturating_add(1);
            return None;
        };
        entry.used = clock;
        self.hits = self.hits.saturating_add(1);
        Some(Arc::clone(&entry.text))
    }

    /// Keeps a page's readback, dropping least-recently-used entries until it fits.
    ///
    /// A readback larger than the whole budget is not kept at all, rather than emptying the cache
    /// to hold one page that will be evicted by the next insertion.
    pub(crate) fn put(&mut self, page: usize, text: &Arc<str>) {
        let size = text.len();
        if size > self.budget {
            return;
        }
        self.clock = self.clock.saturating_add(1);
        if let Some(previous) = self.held.remove(&page) {
            self.bytes = self.bytes.saturating_sub(previous.text.len());
        }
        while self.bytes.saturating_add(size) > self.budget {
            if !self.evict() {
                return;
            }
        }
        self.bytes = self.bytes.saturating_add(size);
        self.held.insert(
            page,
            Entry {
                text: Arc::clone(text),
                used: self.clock,
            },
        );
    }

    /// Drops the least recently used entry, answering whether there was one.
    ///
    /// A scan rather than a second index ordered by use. Two maps to keep in step would be more
    /// lines and one more thing to get wrong, and this loop runs only for a document whose
    /// readback exceeds [`BUDGET`] — which is no document this project owns. Even then it is
    /// a walk of at most a few thousand `u64`s against the 5 ms of interpretation that filled
    /// the entry it is dropping.
    fn evict(&mut self) -> bool {
        let Some(oldest) = self
            .held
            .iter()
            .min_by_key(|(_, entry)| entry.used)
            .map(|(page, _)| *page)
        else {
            return false;
        };
        if let Some(entry) = self.held.remove(&oldest) {
            self.bytes = self.bytes.saturating_sub(entry.text.len());
            self.evicted = self.evicted.saturating_add(1);
        }
        true
    }

    /// Forgets everything, because the view state that produced it moved.
    ///
    /// The counters are deliberately *not* reset: they are what a person reads to find out
    /// whether the cache is working, and a rule that zeroed them on every layer switch would
    /// hide the switch rather than report it.
    pub(crate) fn clear(&mut self) {
        self.held.clear();
        self.bytes = 0;
    }

    /// What this cache is holding, for a host that wants to say so.
    pub(crate) fn report(&self) -> ReadbackCache {
        ReadbackCache {
            pages: self.held.len(),
            bytes: self.bytes,
            budget: self.budget,
            hits: self.hits,
            misses: self.misses,
            evicted: self.evicted,
        }
    }
}

/// What one open document's readback cache is holding, and how it has been used.
///
/// The bound this project asks of a memory budget is that it be *legible*, not that it be small,
/// so the number is readable rather than merely enforced. `pdf-viewer --trace=search` prints this
/// after every search step and `viewer-core/examples/find_cost` prints it after a sweep.
///
/// Not a [`crate::Query`], deliberately. This is an instrument rather than something a host draws
/// with: making it a query would put a variant into a vocabulary six consumers match exhaustively
/// and onto `viewer-confined`'s wire, for a number no interface displays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadbackCache {
    /// How many pages' readback is held.
    pub pages: usize,
    /// How many bytes of readback that is.
    pub bytes: usize,
    /// The ceiling those bytes are held under, which is [`BUDGET`] for every open document.
    pub budget: usize,
    /// How many lookups since the document opened were answered without interpreting a page.
    pub hits: u64,
    /// How many were not.
    pub misses: u64,
    /// How many entries the budget has dropped, which is zero unless a document's readback is
    /// larger than the budget.
    pub evicted: u64,
}

#[cfg(test)]
mod tests {
    use super::{BUDGET, ReadbackCache, Readbacks};
    use std::sync::Arc;

    /// A readback of `size` bytes, which is what the budget counts.
    fn text(size: usize) -> Arc<str> {
        Arc::from("x".repeat(size).as_str())
    }

    /// A hit costs nothing and a miss says so, and both are counted.
    #[test]
    fn a_page_read_once_is_answered_without_being_read_again() {
        let mut cache = Readbacks::default();
        assert_eq!(cache.get(7), None, "nothing has been put there");
        cache.put(7, &text(10));
        assert_eq!(cache.get(7).as_deref(), Some("xxxxxxxxxx"));
        assert_eq!(
            cache.report(),
            ReadbackCache {
                pages: 1,
                bytes: 10,
                budget: BUDGET,
                hits: 1,
                misses: 1,
                evicted: 0,
            }
        );
    }

    /// The budget is a ceiling on bytes, and what goes is what was wanted longest ago.
    #[test]
    fn the_budget_drops_the_least_recently_used_page_rather_than_growing() {
        // Room for two of these three and not the third.
        let mut cache = Readbacks::with_budget(25);
        cache.put(0, &text(10));
        cache.put(1, &text(10));
        // Page 0 is now the more recently used of the two, so page 1 is what the third insertion
        // costs.
        assert!(cache.get(0).is_some());
        cache.put(2, &text(10));
        assert!(cache.get(1).is_none(), "the least recently used one went");
        assert!(cache.get(0).is_some(), "and the one asked for stayed");
        assert!(cache.get(2).is_some());
        let held = cache.report();
        assert_eq!((held.pages, held.bytes), (2, 20));
        assert_eq!(held.evicted, 1);
        assert!(held.bytes <= held.budget, "the ceiling is a ceiling");
    }

    /// A readback larger than the whole budget is declined rather than emptying the cache.
    #[test]
    fn a_page_that_cannot_fit_does_not_empty_the_cache_on_its_way_out() {
        let mut cache = Readbacks::with_budget(25);
        cache.put(0, &text(20));
        cache.put(1, &text(30));
        assert!(cache.get(1).is_none(), "it never went in");
        assert!(cache.get(0).is_some(), "and it did not take page 0 with it");
        assert_eq!(cache.report().evicted, 0);
    }

    /// Putting the same page twice counts its bytes once.
    #[test]
    fn a_page_read_again_replaces_itself_rather_than_being_counted_twice() {
        let mut cache = Readbacks::default();
        cache.put(3, &text(100));
        cache.put(3, &text(40));
        let held = cache.report();
        assert_eq!((held.pages, held.bytes), (1, 40));
    }

    /// A view-state change empties the cache and leaves the counters alone.
    #[test]
    fn clearing_forgets_the_pages_and_keeps_the_tally() {
        let mut cache = Readbacks::default();
        cache.put(0, &text(10));
        assert!(cache.get(0).is_some());
        cache.clear();
        assert!(cache.get(0).is_none());
        let held = cache.report();
        assert_eq!((held.pages, held.bytes), (0, 0));
        assert_eq!((held.hits, held.misses), (1, 1), "the tally is the record");
    }
}
