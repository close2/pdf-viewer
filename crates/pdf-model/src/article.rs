//! ISO 32000-2 §12.4.3's articles: a reading order laid over pages that are not consecutive.
//!
//! The clause's own example is a newsletter:
//!
//! > Some types of documents may contain sequences of content items that are logically
//! > connected but not physically sequential.
//!
//! A story starting on page one and continuing on page seven is one *article*, whose flow is
//! an **article thread** and whose pieces are **beads** on that thread. What a viewer does
//! with them is stated as a permission — "[i]nteractive PDF processors may provide navigation
//! facilities to allow the user to follow a thread from one bead to the next" — so this module
//! reads the structure and offers the step; following it is a window's.
//!
//! # A doubly-linked ring, entered once
//!
//! §12.4.3 makes the chain circular, which is unusual enough to be worth quoting:
//!
//! > The thread dictionary's F entry shall refer to the first bead in the thread; the beads
//! > shall be chained together sequentially in a doubly linked list through their N (next) and
//! > V (previous) entries.
//!
//! and Table 163 closes the ring at both ends: `/N` "[i]n the last bead … shall refer to the
//! first bead", `/V` "[i]n the first bead … shall refer to the last bead". So the walk cannot
//! stop at "no next" the way §12.3.3's outline does — it stops when it comes back to where it
//! started, and refuses to visit an object twice so that a producer's broken `/N` ends the
//! thread instead of the process. Only `/N` is followed, for [`crate::outline`]'s reason:
//! `/V` is redundant with it and could only disagree.
//!
//! # The document states the beads twice, and the two statements disagree about their order
//!
//! Table 31's `/B` on a *page* is the same set of beads reached the other way — and its NOTE 2
//! says so outright: "[t]he information in this entry can be created or recreated from the
//! information obtained from the Threads key in the catalog dictionary". That makes it
//! checkable against the thread walk, which is the habit an outline's `/Count` and an LZW
//! stream's length taught here: *what does this file say about itself twice, and does it
//! agree?*
//!
//! The two clauses do **not** agree about the order, and this is the standard contradicting
//! itself rather than a file doing it. Table 31: "[t]he beads shall be listed in the array in
//! natural reading order." §12.4.3: "the page object … shall contain a B entry whose value is
//! an array of indirect references to the beads on the page, in drawing order." Nothing here
//! has to choose, because nothing here draws from `/B`: [`Articles::beads_on_page`] answers
//! from the *threads*, whose order §12.4.3 states unambiguously, and [`Articles::page_array_agrees`]
//! reports whether a page's own array holds the same set. Comparing the set rather than the
//! sequence is a **documented choice**, and it is the only comparison both sentences license.
//!
//! # What is not here
//!
//! A bead's `/R` is "[a] rectangle specifying the location of this bead on the page in default
//! user space" — the region a viewer would zoom to. It is read, and nothing zooms: `viewer-ui`
//! fits a whole page to its surface, which is the same reason §12.3.2.1's view parameters are
//! carried and unapplied.
//!
//! # What the corpus says about this clause, and what it was said to say
//!
//! Measured, in `tests/articles.rs`: **no document of the 974 states an article**. Two catalogs
//! carry a `/Threads` entry and neither carries a thread — one an empty array, one a reference
//! resolving to null — and not one page carries a `/B`.
//!
//! **This section used to be headed "the corpus has nothing to say about this clause", and that
//! was a claim about pdf.js wearing the corpus's name.** The four submodules under
//! `doc/corpora/` hold four documents with real threads and 115 beads between them, and
//! `tests/articles.rs` walks the clearest of them: `PDFBOX-3110-poems-beads.pdf`, two poems as
//! two threads, whose titles decode as §7.9.2.2 text strings, whose every bead names a page, and
//! whose rings close. So this module is no longer written from the clause alone — which is worth
//! knowing precisely because it was for a long time, and nothing announced the change. ADR 0405.
//!
//! The habit that survives is the one the old heading got right: a corpus cannot rank a
//! requirement no file exercises, and "no corpus document does this" is a measurement rather than
//! an impression — **of a stated population**, which is the half that was missing.

use std::collections::BTreeSet;

use pdf_syntax::{Dictionary, Document, Object, ObjectId};

use crate::page::Pages;

/// Most threads read from one document.
///
/// An article is a story a person reads; a document listing more than this is one making a
/// reader work rather than describing a magazine.
const MAX_THREADS: usize = 1024;

/// Most beads read across all of a document's threads.
///
/// A bound on the whole family rather than on each thread, because the ring is what a
/// malformed file gets wrong: two threads whose `/N` chains cross would otherwise each pay a
/// full budget.
const MAX_BEADS: usize = 1 << 16;

/// One bead: a rectangle on one page, and its place in a thread.
#[derive(Debug, Clone, PartialEq)]
pub struct Bead {
    /// The bead dictionary's own object, so a `/B` array can be compared against it.
    pub id: ObjectId,
    /// Table 163's `/P`, "[t]he page object representing the page on which this bead appears".
    ///
    /// Kept as the reference the file wrote rather than as an index, because that is what
    /// identifies a page here — and resolved to an index by [`Bead::page_index`], which needs
    /// the page tree.
    pub page: Option<ObjectId>,
    /// Table 163's `/R`, "[a] rectangle specifying the location of this bead on the page in
    /// default user space", normalised so that `[x0, y0]` is the lower-left corner.
    ///
    /// §7.9.5 states that normalisation for rectangles generally — "the rectangle … shall be
    /// specified by two diagonally opposite corners", so neither corner is promised to be the
    /// lower one — and this crate applies it wherever it reads a rectangle.
    pub rect: Option<[f32; 4]>,
}

impl Bead {
    /// The zero-based index of the page this bead is on, or `None`.
    ///
    /// `None` is a bead whose `/P` is absent, or names an object the page tree does not hold —
    /// which Table 163 forbids ("[r]equired; shall be an indirect reference") and a file may
    /// still write.
    #[must_use]
    pub fn page_index(&self, pages: &Pages<'_>) -> Option<usize> {
        pages.index_of(self.page?)
    }
}

/// One article: a title, and the beads in the order the thread chains them.
#[derive(Debug, Clone, PartialEq)]
pub struct Thread {
    /// The thread dictionary's own object.
    pub id: ObjectId,
    /// `/Title` from Table 162's `/I` thread information dictionary, where there is one.
    ///
    /// The clause says of `/I` that "[t]he contents of this dictionary shall conform to the
    /// syntax for the document information dictionary (see 14.3.3)", so its `/Title` is a
    /// §7.9.2.2 text string and is decoded as one. §12.6.4.7 names a thread *by* this title,
    /// which is the one place it is load-bearing rather than decorative.
    pub title: Option<String>,
    /// The beads, starting at `/F` and following `/N` until the ring closes.
    pub beads: Vec<Bead>,
}

/// A document's articles, read once.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Articles {
    /// The threads, in the order the catalog's `/Threads` array holds them.
    ///
    /// The order is load-bearing: §12.6.4.7 lets a thread action name "[t]he index of the
    /// thread within the Threads array", and resolves a duplicated title by taking "the one
    /// appearing first in the document catalog's Threads array".
    pub threads: Vec<Thread>,
}

impl Articles {
    /// Reads the catalog's `/Threads`, which almost no document has.
    ///
    /// An absent entry is a document without articles rather than a defect, and produces an
    /// empty list.
    #[must_use]
    pub fn read(document: &Document) -> Self {
        let Ok(catalog) = document.catalog() else {
            return Self::default();
        };
        let threads = document.get_key(&catalog, "Threads");
        let Some(threads) = threads.as_array() else {
            return Self::default();
        };
        let mut visited = BTreeSet::new();
        let mut budget = MAX_BEADS;
        Self {
            threads: threads
                .iter()
                .take(MAX_THREADS)
                .filter_map(|entry| {
                    let id = entry.as_reference()?;
                    read_thread(document, id, &mut visited, &mut budget)
                })
                .collect(),
        }
    }

    /// Whether the document states any article at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.threads.is_empty()
    }

    /// The bead following a given one in its thread, wrapping at the end.
    ///
    /// The clause's ring is the reason this always answers: `/N` "[i]n the last bead … shall
    /// refer to the first bead", so following a thread past its end returns to its start. The
    /// caller decides whether that is a wrap it wants to make; a viewer showing "next in this
    /// article" usually does.
    ///
    /// `None` means the bead is not one this document states.
    #[must_use]
    pub fn next(&self, bead: ObjectId) -> Option<&Bead> {
        let (thread, at) = self.locate(bead)?;
        let beads = &self.threads.get(thread)?.beads;
        let following = at
            .checked_add(1)
            .filter(|following| *following < beads.len())
            .unwrap_or(0);
        beads.get(following)
    }

    /// Where a bead sits: its thread's index in `/Threads`, and its own index in that thread.
    ///
    /// §12.6.4.7's `/B` names a bead by exactly this second number — "[t]he index of the bead
    /// within its thread. The first bead in a thread has index 0" — so the pair is the
    /// coordinate this family is addressed by.
    #[must_use]
    pub fn locate(&self, bead: ObjectId) -> Option<(usize, usize)> {
        self.threads.iter().enumerate().find_map(|(t, thread)| {
            thread
                .beads
                .iter()
                .position(|candidate| candidate.id == bead)
                .map(|b| (t, b))
        })
    }

    /// Every bead lying on one page, in thread order, across all threads.
    ///
    /// Answered from the threads rather than from the page's own `/B`, because §12.4.3 states
    /// the thread's order in one sentence and the two clauses that describe `/B` disagree about
    /// its own — see the module comment.
    #[must_use]
    pub fn beads_on_page(&self, page: ObjectId) -> Vec<&Bead> {
        self.threads
            .iter()
            .flat_map(|thread| thread.beads.iter())
            .filter(|bead| bead.page == Some(page))
            .collect()
    }

    /// Whether a page's `/B` array names the same beads this reader found on that page.
    ///
    /// Table 31 NOTE 2 says `/B` "can be created or recreated from the information obtained
    /// from the `Threads` key", which makes it a second statement of a fact the threads
    /// already carry — and a document stating a fact twice can be checked against itself. The
    /// comparison is on the *set*, because Table 31 asks for "natural reading order" and
    /// §12.4.3 for "drawing order" and neither can be checked while the other stands.
    ///
    /// `None` where the page states no `/B`, which is not a disagreement: the entry is
    /// "recommended if the page contains article beads", never required.
    ///
    /// The page's own object identity has to be passed in, because a page dictionary does not
    /// carry it and a bead names its page by reference.
    #[must_use]
    pub fn page_array_agrees(
        &self,
        document: &Document,
        page: &Dictionary,
        id: ObjectId,
    ) -> Option<bool> {
        let stated = document.get_key(page, "B");
        let stated = stated.as_array()?;
        let stated: BTreeSet<ObjectId> = stated.iter().filter_map(Object::as_reference).collect();
        let found: BTreeSet<ObjectId> = self
            .beads_on_page(id)
            .into_iter()
            .map(|bead| bead.id)
            .collect();
        Some(stated == found)
    }
}

/// Reads one thread: Table 162's entries, then the ring its `/F` starts.
fn read_thread(
    document: &Document,
    id: ObjectId,
    visited: &mut BTreeSet<ObjectId>,
    budget: &mut usize,
) -> Option<Thread> {
    let thread = document.get(id);
    let thread = thread.as_dict()?;
    let information = document.get_key(thread, "I");
    let title = information
        .as_dict()
        .map(|information| document.get_key(information, "Title"))
        .and_then(|title| match title {
            Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
            _ => None,
        });
    let mut beads = Vec::new();
    let mut next = thread.get("F").and_then(Object::as_reference);
    while let Some(bead) = next {
        if *budget == 0 || !visited.insert(bead) {
            // Either the ring has closed — the expected way out, since the last bead's `/N`
            // names the first — or the file has chained a bead into a thread that already
            // holds it. Both end this thread with what was read, which is the document's own
            // list up to the point it contradicted itself.
            break;
        }
        *budget = budget.saturating_sub(1);
        let dict = document.get(bead);
        let Some(dict) = dict.as_dict() else {
            break;
        };
        beads.push(Bead {
            id: bead,
            page: dict.get("P").and_then(Object::as_reference),
            rect: rectangle(document, dict),
        });
        next = dict.get("N").and_then(Object::as_reference);
    }
    Some(Thread { id, title, beads })
}

/// Table 163's `/R`, normalised to `[x0, y0, x1, y1]` with the lower-left corner first.
fn rectangle(document: &Document, bead: &Dictionary) -> Option<[f32; 4]> {
    let rect = document.get_key(bead, "R");
    let rect = rect.as_array()?;
    let mut values = [0.0f32; 4];
    if rect.len() < 4 {
        return None;
    }
    for (slot, value) in values.iter_mut().zip(rect.iter()) {
        // `as_number` rather than `as_integer`: Table 163 says "rectangle", and §7.9.5 makes a
        // rectangle "an array of four numbers", of which an integer is one.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a page coordinate in f32, as every rectangle in this crate is"
        )]
        {
            *slot = document.resolve(value).as_number()? as f32;
        }
    }
    Some([
        values[0].min(values[2]),
        values[1].min(values[3]),
        values[0].max(values[2]),
        values[1].max(values[3]),
    ])
}

#[cfg(test)]
mod tests {
    use super::{Articles, Bead};
    use crate::page::Pages;
    use pdf_syntax::{Document, ObjectId};

    /// Builds a document from object bodies numbered from 1.
    fn document(objects: &[&str]) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let _ = write!(
            out,
            "xref\n0 {}\n0000000000 65535 f \n",
            objects.len().saturating_add(1)
        );
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len().saturating_add(1)
        );
        Document::open(out.into_bytes()).expect("a valid file")
    }

    fn id(number: u32) -> ObjectId {
        ObjectId {
            number,
            generation: 0,
        }
    }

    /// §12.4.3's own EXAMPLE 2, renumbered onto two pages.
    ///
    /// > 22 0 obj <</F 23 0 R /I <</Title (Man Bites Dog)>> >> endobj
    /// > 23 0 obj <</T 22 0 R /N 24 0 R /V 25 0 R /P 8 0 R /R [158 247 318 905] >> endobj
    /// > 24 0 obj <</T 22 0 R /N 25 0 R /V 23 0 R /P 8 0 R /R [322 246 486 904] >> endobj
    /// > 25 0 obj <</T 22 0 R /N 23 0 R /V 24 0 R /P 10 0 R /R [157 254 319 903] >> endobj
    ///
    /// Two of its beads are on one page and the third on another, which is the whole point of
    /// an article — and its `/N` chain closes back onto the first bead, which is what a walk
    /// that stopped at "no next" would never notice.
    fn example() -> Document {
        document(&[
            "<< /Type /Catalog /Pages 2 0 R /Threads [3 0 R] >>",
            "<< /Type /Pages /Kids [4 0 R 5 0 R] /Count 2 /MediaBox [0 0 612 792] >>",
            "<< /F 6 0 R /I << /Title (Man Bites Dog) >> >>",
            "<< /Type /Page /Parent 2 0 R /B [6 0 R 7 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /B [8 0 R] >>",
            "<< /T 3 0 R /N 7 0 R /V 8 0 R /P 4 0 R /R [158 247 318 905] >>",
            "<< /T 3 0 R /N 8 0 R /V 6 0 R /P 4 0 R /R [322 246 486 904] >>",
            "<< /T 3 0 R /N 6 0 R /V 7 0 R /P 5 0 R /R [157 254 319 903] >>",
        ])
    }

    /// The clause's example reads as one titled thread of three beads, in `/N` order.
    #[test]
    fn the_clauses_own_example_is_three_beads_on_two_pages() {
        let doc = example();
        let articles = Articles::read(&doc);
        let [thread] = articles.threads.as_slice() else {
            panic!("one thread, got {articles:?}");
        };
        assert_eq!(thread.title.as_deref(), Some("Man Bites Dog"));
        assert_eq!(
            thread.beads,
            vec![
                Bead {
                    id: id(6),
                    page: Some(id(4)),
                    rect: Some([158.0, 247.0, 318.0, 905.0]),
                },
                Bead {
                    id: id(7),
                    page: Some(id(4)),
                    rect: Some([322.0, 246.0, 486.0, 904.0]),
                },
                Bead {
                    id: id(8),
                    page: Some(id(5)),
                    rect: Some([157.0, 254.0, 319.0, 903.0]),
                },
            ]
        );
        let pages = Pages::new(&doc);
        assert_eq!(
            thread
                .beads
                .iter()
                .map(|bead| bead.page_index(&pages))
                .collect::<Vec<_>>(),
            vec![Some(0), Some(0), Some(1)],
            "the third bead continues the story on the second page"
        );
    }

    /// The chain is a ring: the last bead's `/N` is the first, and the walk ends there.
    ///
    /// Table 163 states both ends of it — `/N` "[i]n the last bead … shall refer to the first
    /// bead" — so a reader that stopped only at a missing `/N` would walk this document
    /// forever. Following past the end wraps, which is what a viewer offering "next in this
    /// article" wants.
    #[test]
    fn the_ring_closes_and_following_it_wraps() {
        let doc = example();
        let articles = Articles::read(&doc);
        assert_eq!(articles.next(id(6)).map(|bead| bead.id), Some(id(7)));
        assert_eq!(articles.next(id(7)).map(|bead| bead.id), Some(id(8)));
        assert_eq!(
            articles.next(id(8)).map(|bead| bead.id),
            Some(id(6)),
            "the last bead's next is the thread's first"
        );
        assert_eq!(articles.locate(id(8)), Some((0, 2)));
        assert_eq!(articles.next(id(99)), None, "not a bead of this document");
    }

    /// A page's `/B` array and the threads state the same set of beads, and can disagree.
    ///
    /// Table 31 NOTE 2: "[t]he information in this entry can be created or recreated from the
    /// information obtained from the `Threads` key in the catalog dictionary." So the document
    /// says the same thing twice, and the second saying is checkable — which is the only use
    /// this reader has for `/B` at all.
    #[test]
    fn a_pages_bead_array_agrees_with_the_threads() {
        let doc = example();
        let articles = Articles::read(&doc);
        let pages = Pages::new(&doc);
        for (index, page) in [(0usize, id(4)), (1, id(5))] {
            let dict = pages.get(index).expect("a page").dict;
            assert_eq!(
                articles.page_array_agrees(&doc, &dict, page),
                Some(true),
                "page {index} lists exactly the beads the threads put on it"
            );
        }

        let wrong = document(&[
            "<< /Type /Catalog /Pages 2 0 R /Threads [3 0 R] >>",
            "<< /Type /Pages /Kids [4 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
            "<< /F 5 0 R >>",
            "<< /Type /Page /Parent 2 0 R /B [5 0 R 6 0 R] >>",
            "<< /N 5 0 R /V 5 0 R /P 4 0 R /R [0 0 10 10] >>",
            "<< /P 4 0 R /R [0 0 10 10] >>",
        ]);
        let articles = Articles::read(&wrong);
        let pages = Pages::new(&wrong);
        let dict = pages.get(0).expect("a page").dict;
        assert_eq!(
            articles.page_array_agrees(&wrong, &dict, id(4)),
            Some(false),
            "the page names a bead that hangs on no thread"
        );

        let none = document(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
            "<< /Type /Page /Parent 2 0 R >>",
        ]);
        let articles = Articles::read(&none);
        let pages = Pages::new(&none);
        let dict = pages.get(0).expect("a page").dict;
        assert!(articles.is_empty());
        assert_eq!(
            articles.page_array_agrees(&none, &dict, id(3)),
            None,
            "a page with no /B has not disagreed with anything"
        );
    }

    /// A `/N` that points back into the middle of a thread ends it rather than looping.
    ///
    /// Every one of these references is one a producer can get wrong, and the ring means the
    /// obvious termination test is unavailable. The visited set is what makes this terminate,
    /// and the thread keeps what it read before the file contradicted itself.
    #[test]
    fn a_chain_that_re_enters_itself_ends_the_thread() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /Threads [3 0 R] >>",
            "<< /Type /Pages /Kids [4 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
            "<< /F 5 0 R >>",
            "<< /Type /Page /Parent 2 0 R >>",
            "<< /N 6 0 R /P 4 0 R >>",
            "<< /N 7 0 R /P 4 0 R >>",
            "<< /N 6 0 R /P 4 0 R >>",
        ]);
        let articles = Articles::read(&doc);
        let [thread] = articles.threads.as_slice() else {
            panic!("one thread");
        };
        assert_eq!(
            thread.beads.iter().map(|bead| bead.id).collect::<Vec<_>>(),
            vec![id(5), id(6), id(7)],
            "each bead once, and the second visit to 6 ends the walk"
        );
    }

    /// Two threads sharing a bead do not each pay the whole budget.
    ///
    /// The visited set spans the document rather than the thread, which is what keeps a file
    /// that crosses its chains from costing `threads × beads` work.
    #[test]
    fn a_bead_belongs_to_one_thread() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /Threads [3 0 R 4 0 R] >>",
            "<< /Type /Pages /Kids [5 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
            "<< /F 6 0 R >>",
            "<< /F 6 0 R >>",
            "<< /Type /Page /Parent 2 0 R >>",
            "<< /N 6 0 R /P 5 0 R >>",
        ]);
        let articles = Articles::read(&doc);
        assert_eq!(articles.threads.len(), 2);
        assert_eq!(articles.threads[0].beads.len(), 1);
        assert!(
            articles.threads[1].beads.is_empty(),
            "the second thread's first bead is already on the first"
        );
    }
}
