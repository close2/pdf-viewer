//! A document's text, addressed the way a program asks for it: a page, a section, and the
//! annotations attached to either.
//!
//! # Why this exists
//!
//! `viewer-core` was built for a *person* at a window, and `pdf-model` reads everything a
//! document says about itself. Between them the readers were complete and the joins were
//! missing: an outline item names a destination, a destination names a page and a point, and
//! nothing turned "§9.6.5.4" into a range of text. This crate is the join, and its consumer is a
//! program — `doc/todo/36`.
//!
//! # What it does not do, deliberately
//!
//! It does not rasterise, it does not interpret a document more than once for one question, and
//! **it never converts the answer into something more convenient than what the file says**. The
//! default readback is exactly `pdf_model::Interpretation::text` — the string
//! `pdf-model/tests/text_extraction.rs` compares against `pdftotext` over 974 documents — so
//! trusting what comes out of here is the same act as trusting that gate, rather than a second
//! claim on top of it.
//!
//! Every departure from that string is asked for and is reported in the answer: §14.8.2.2's
//! artifacts dropped, §14.8.2.5's logical order taken instead of the stream's, a section trimmed
//! at its headings. A consumer that asks for none of them gets the readback and can say so.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod json;

use std::path::{Path, PathBuf};

use pdf_model::outline::Outline;
use pdf_model::page_label::PageLabels;
use pdf_model::retrieval::{Section, annotations, sections, spans_under, text_under};
use pdf_model::structure::Tree;
use pdf_model::{Interpretation, Page, Pages};
use pdf_syntax::{Dictionary, Document, Object};

/// What went wrong.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The file could not be read at all.
    #[error("{0}: cannot be read ({1})")]
    Unreadable(PathBuf, std::io::Error),
    /// The bytes are not a PDF this tree opens.
    #[error("{0}: does not open as a PDF ({1})")]
    Unopenable(PathBuf, pdf_syntax::SyntaxError),
    /// A page index past the end of the document.
    #[error("page {0} is past the end: this document has {1}")]
    NoSuchPage(usize, usize),
    /// An address §12.3.3's outline does not carry.
    #[error("no section is addressed by {0:?}")]
    NoSuchSection(String),
}

/// A document opened once, with the readers a retrieval needs beside it.
///
/// Held together because each of them costs a walk of something — the page tree, the outline,
/// §12.4.2's label tree — and a caller asking two questions of one document should pay once.
#[derive(Debug)]
pub struct Retrieval {
    /// The document itself.
    document: Document,
    /// §12.3.3's outline, read once.
    outline: Outline,
    /// Every section the outline addresses, in its order.
    sections: Vec<Section>,
    /// §12.4.2's labels, for naming a page the way the document does.
    labels: PageLabels,
}

/// Which order a page's text came back in, and it is an answer rather than a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    /// §14.8.2.5.1's page content order: "the sequencing of graphics objects within a page's
    /// content stream", which is what the interpreter produces.
    Content,
    /// §14.8.2.5.1's logical content order, "a depth-first traversal of the document's logical
    /// structure hierarchy".
    Logical,
}

impl Order {
    /// The word this appears as in a report.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Content => "content",
            Self::Logical => "logical",
        }
    }
}

/// What a caller wants attached to, or taken out of, the text it asked for.
#[derive(Debug, Clone, Default)]
pub struct Wanted {
    /// Read the page's annotations and attach them.
    pub annotations: bool,
    /// Which `/Subtype`s to keep, or every one where it is empty.
    ///
    /// A filter rather than a fixed vocabulary, because what an annotation *is to a reader* is
    /// the file's business and not this tool's: the errata in ISO 32000-2 are `StrikeOut` and
    /// `Caret` marks, another document's notes are `Text`, and §12.5.6.5's links are navigation
    /// that a retrieval consumer usually wants out of the way. Table 171's own names, compared
    /// exactly.
    pub subtypes: Vec<String>,
    /// Drop §14.8.2.2's artifacts — running heads, folios, the licence stamp on every page of a
    /// sponsored ISO document.
    pub drop_artifacts: bool,
    /// Ask for §14.8.2.5's logical order rather than the stream's.
    ///
    /// A request rather than a promise: a page whose document states no structure tree, or whose
    /// tree does not reach the page, comes back in content order with [`PageText::order`] saying
    /// so.
    pub logical: bool,
}

impl Wanted {
    /// Whether the annotations are to be read at all.
    ///
    /// [`Self::subtypes`] implies [`Self::annotations`], because a caller that named which
    /// subtypes it wants and got none has been answered by a flag it did not know it had to
    /// set. The alternative — refusing the combination — would be a stricter API and a worse
    /// one: there is no reading of "keep only the strikeouts" that means "keep nothing".
    #[must_use]
    pub fn wants_annotations(&self) -> bool {
        self.annotations || !self.subtypes.is_empty()
    }
}

/// One annotation, as a retrieval consumer wants it.
///
/// Every field is the document's own: nothing here is constructed, and an entry the file does
/// not state is `None` rather than a default. §12.5.6 is where each lives — `/Contents` is
/// Table 166's "text to be displayed for the annotation", `/Subj` Table 172's "text representing
/// a short description of the subject", and [`Self::covers`] is Table 182's `/QuadPoints` read
/// back against the page.
#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    /// The zero-based page it is on.
    pub page: usize,
    /// Table 166's `/Subtype`, whose values Table 171 names.
    pub subtype: String,
    /// `/T`, the text label — "the title of the markup annotation's author" (§12.5.6.2).
    pub title: Option<String>,
    /// `/Subj`.
    pub subject: Option<String>,
    /// `/Contents`.
    pub contents: Option<String>,
    /// `/CreationDate`, as the file spells it.
    pub created: Option<String>,
    /// The page's own text under the annotation's `/QuadPoints`.
    pub covers: Option<String>,
    /// Where that text sits in the page's readback, ascending.
    pub spans: Vec<std::ops::Range<usize>>,
}

/// One page's text, and what was done to it.
#[derive(Debug, Clone, PartialEq)]
pub struct PageText {
    /// The zero-based index.
    pub index: usize,
    /// §12.4.2's label, where the document states one.
    pub label: Option<String>,
    /// The text.
    pub text: String,
    /// Which order it came back in.
    pub order: Order,
    /// Whether the interpreter drew everything the page asked for.
    ///
    /// A page this reader could not fully interpret may have lost text with it, so a consumer
    /// that cares whether it has the *whole* page has to be told. Trap 1's rule one level out:
    /// this says what the interpreter knows it skipped and cannot say a font produced rubbish.
    pub complete: bool,
    /// What it could not do, in its own words.
    pub unsupported: Vec<String>,
    /// The annotations, where they were asked for.
    pub annotations: Vec<Note>,
}

/// One section's text, and where its edges came from.
#[derive(Debug, Clone, PartialEq)]
pub struct SectionText {
    /// The section addressed.
    pub section: Section,
    /// The pages it was assembled from, in order.
    pub pages: Vec<usize>,
    /// The text, trimmed at whichever ends could be found.
    pub text: String,
    /// Which order the pages came back in.
    pub order: Order,
    /// Whether the section's own heading was found on its first page.
    ///
    /// `false` means the text begins at the top of that page and therefore carries the tail of
    /// whatever came before — reported rather than hidden, because a consumer quoting the result
    /// needs to know it may have caught a neighbour.
    pub trimmed_start: bool,
    /// Whether the *next* section's heading was found on the last page.
    pub trimmed_end: bool,
    /// Whether every page of it interpreted completely.
    pub complete: bool,
    /// What could not be interpreted, over all its pages.
    pub unsupported: Vec<String>,
    /// The annotations attached to it, where they were asked for.
    pub annotations: Vec<Note>,
}

impl Retrieval {
    /// Opens a document and reads what addressing it needs.
    ///
    /// # Errors
    ///
    /// [`Error::Unreadable`] where the file cannot be read, [`Error::Unopenable`] where it is
    /// not a PDF this tree opens.
    pub fn open(path: &Path) -> Result<Self, Error> {
        let bytes =
            std::fs::read(path).map_err(|error| Error::Unreadable(path.to_owned(), error))?;
        let document =
            Document::open(bytes).map_err(|error| Error::Unopenable(path.to_owned(), error))?;
        let (outline, sections, labels) = {
            let pages = Pages::new(&document);
            let outline = Outline::read(&document, &pages);
            let sections = sections(&document, &pages, &outline);
            (outline, sections, PageLabels::read(&document))
        };
        Ok(Self {
            document,
            outline,
            sections,
            labels,
        })
    }

    /// The document itself, for a caller that wants a reader this crate does not wrap.
    #[must_use]
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// §12.3.3's outline as it was read.
    #[must_use]
    pub fn outline(&self) -> &Outline {
        &self.outline
    }

    /// Every addressable section.
    #[must_use]
    pub fn sections(&self) -> &[Section] {
        &self.sections
    }

    /// How many pages the document has.
    #[must_use]
    pub fn page_count(&self) -> usize {
        Pages::new(&self.document).len()
    }

    /// §12.4.2's label for a page, where the document states one.
    #[must_use]
    pub fn label(&self, index: usize) -> Option<String> {
        self.labels.label(index)
    }

    /// One page's text.
    ///
    /// # Errors
    ///
    /// [`Error::NoSuchPage`] for an index past the end.
    pub fn page(&self, index: usize, wanted: &Wanted) -> Result<PageText, Error> {
        let pages = Pages::new(&self.document);
        let page = pages
            .get(index)
            .ok_or_else(|| Error::NoSuchPage(index, pages.len()))?;
        Ok(self.read_page(index, &page, wanted).shown)
    }

    /// One section's text, addressed by clause number or by the start of its title.
    ///
    /// # Two coordinate systems, and why there are two
    ///
    /// An annotation's `/QuadPoints` are read back as ranges of
    /// `pdf_model::Interpretation::text` — the *raw* readback of a page and nothing else, because
    /// that is the string the text layer indexes. Dropping §14.8.2.2's artifacts or taking
    /// §14.8.2.5's order changes which characters are where, so an offset computed in one of
    /// those is meaningless in the other.
    ///
    /// So the assembly is done twice over the same pages: once raw, which is where the section's
    /// edges are found for the purpose of deciding **which annotations are inside it**, and once
    /// in whatever form was asked for, which is where they are found for the purpose of
    /// **cutting the text**. Each answer is computed in the only coordinates it is true in. The
    /// first version of this did it once and reported one annotation fewer with `--no-artifacts`
    /// than without, which is what a single coordinate system costs here.
    ///
    /// # Errors
    ///
    /// [`Error::NoSuchSection`] where §12.3.3's outline addresses nothing by that name.
    pub fn section(&self, address: &str, wanted: &Wanted) -> Result<SectionText, Error> {
        let section = pdf_model::retrieval::section(&self.sections, address)
            .ok_or_else(|| Error::NoSuchSection(address.to_owned()))?
            .clone();
        let pages = Pages::new(&self.document);
        let mut read = Vec::new();
        for index in section.first_page..=section.last_page {
            if let Some(page) = pages.get(index) {
                read.push(self.read_page(index, &page, wanted));
            }
        }
        let (raw, raw_starts) = assemble(read.iter().map(|page| page.raw.as_str()));
        let (shown, shown_starts) = assemble(read.iter().map(|page| page.shown.text.as_str()));
        let raw_edges = edges(&raw, &raw_starts, &section);
        let (start, end) = edges(&shown, &shown_starts, &section);
        let text = shown
            .get(start.unwrap_or(0)..end.unwrap_or(shown.len()))
            .unwrap_or(&shown)
            .to_owned();
        let attached = if wanted.wants_annotations() {
            Self::attach(
                &read,
                &raw_starts,
                raw_edges.0.unwrap_or(0),
                raw_edges.1.unwrap_or(raw.len()),
            )
        } else {
            Vec::new()
        };
        Ok(SectionText {
            pages: read.iter().map(|page| page.shown.index).collect(),
            // Content order unless *every* page came back logical: a section assembled from
            // one page of each has the weaker of the two properties and should say so.
            order: if !read.is_empty() && read.iter().all(|page| page.shown.order == Order::Logical)
            {
                Order::Logical
            } else {
                Order::Content
            },
            trimmed_start: start.is_some(),
            trimmed_end: end.is_some(),
            complete: read.iter().all(|page| page.shown.complete),
            unsupported: read
                .iter()
                .flat_map(|page| page.shown.unsupported.clone())
                .collect(),
            annotations: attached,
            section,
            text,
        })
    }

    /// Which of the pages' annotations belong to the section's own stretch of text.
    ///
    /// The rule, and it is a choice: an annotation that covers text is the section's where **any
    /// of the text it covers is**, and one that covers none — a `Text` note stuck to a point, a
    /// `Link` — is kept for its page. The first half is what makes asking for §9.6.5.4 give the
    /// errata on §9.6.5.4 rather than every mark on the three pages it touches; the second half
    /// is the honest limit, because a point on a shared page belongs to no clause the file
    /// states.
    ///
    /// `from` and `to` are offsets into the **raw** assembly, which is the only place a
    /// `/QuadPoints` span means anything.
    fn attach(read: &[Read], starts: &[usize], from: usize, to: usize) -> Vec<Note> {
        let mut out = Vec::new();
        for (position, page) in read.iter().enumerate() {
            let offset = starts.get(position).copied().unwrap_or(0);
            for note in &page.shown.annotations {
                let inside = note.spans.iter().any(|span| {
                    let at = offset.saturating_add(span.start);
                    at >= from && at < to
                });
                if note.spans.is_empty() || inside {
                    out.push(note.clone());
                }
            }
        }
        out
    }

    /// One page, interpreted once, in whichever order was asked for.
    fn read_page(&self, index: usize, page: &Page, wanted: &Wanted) -> Read {
        let interpretation = pdf_model::interpret(&self.document, page);
        let (text, order) = self.ordered(page, &interpretation, wanted);
        let shown = PageText {
            index,
            label: self.labels.label(index),
            text: if wanted.drop_artifacts {
                without_artifacts(&text, &interpretation)
            } else {
                text
            },
            order,
            complete: interpretation.is_complete(),
            unsupported: interpretation
                .unsupported
                .iter()
                .map(|report| format!("{report:?}"))
                .collect(),
            annotations: if wanted.wants_annotations() {
                self.notes(index, page, &interpretation, &wanted.subtypes)
            } else {
                Vec::new()
            },
        };
        Read {
            shown,
            raw: interpretation.text,
        }
    }

    /// The readback, in §14.8.2.5's logical order where it was asked for and could be had.
    ///
    /// The fall-back is announced rather than silent, which is the whole reason [`Order`] is in
    /// the answer: a document with no structure tree, a page the tree does not reach and a tree
    /// larger than `pdf_model::structure`'s bound all give content order, and a consumer that
    /// asked for reading order has to be able to tell.
    fn ordered(
        &self,
        page: &Page,
        interpretation: &Interpretation,
        wanted: &Wanted,
    ) -> (String, Order) {
        if !wanted.logical {
            return (interpretation.text.clone(), Order::Content);
        }
        let logical = page
            .id
            .and_then(|id| {
                Tree::of(&self.document)?.logical_text(&self.document, id, interpretation)
            })
            .filter(|text| !text.is_empty());
        match logical {
            Some(text) => (text, Order::Logical),
            None => (interpretation.text.clone(), Order::Content),
        }
    }

    /// Every annotation on a page, read.
    fn notes(
        &self,
        index: usize,
        page: &Page,
        interpretation: &Interpretation,
        subtypes: &[String],
    ) -> Vec<Note> {
        annotations(&self.document, page)
            .iter()
            .map(|annotation| Note {
                page: index,
                subtype: self
                    .document
                    .get_key(annotation, "Subtype")
                    .as_name()
                    .map_or_else(String::new, |name| {
                        String::from_utf8_lossy(name.as_bytes()).into_owned()
                    }),
                title: self.string(annotation, "T"),
                subject: self.string(annotation, "Subj"),
                contents: self.string(annotation, "Contents"),
                created: self.string(annotation, "CreationDate"),
                covers: text_under(&self.document, annotation, page, interpretation),
                spans: spans_under(&self.document, annotation, page, interpretation),
            })
            .filter(|note| subtypes.is_empty() || subtypes.contains(&note.subtype))
            .collect()
    }

    /// One text-string entry, decoded by §7.9.2.2's rules.
    fn string(&self, dict: &Dictionary, key: &str) -> Option<String> {
        match self.document.get_key(dict, key) {
            Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
            _ => None,
        }
    }
}

/// One page read once, in both the coordinate systems [`Retrieval::section`] needs.
struct Read {
    /// What a caller asked for.
    shown: PageText,
    /// `pdf_model::Interpretation::text`, which is what [`Note::spans`] index.
    raw: String,
}

/// The pages' texts joined with a newline, and the offset each one starts at.
fn assemble<'a>(pages: impl Iterator<Item = &'a str>) -> (String, Vec<usize>) {
    let mut out = String::new();
    let mut starts = Vec::new();
    for page in pages {
        if !out.is_empty() {
            out.push('\n');
        }
        starts.push(out.len());
        out.push_str(page);
    }
    (out, starts)
}

/// Where a section begins and ends in one assembly of its pages.
///
/// The heading is on the *first* page and the next one on the *last*, by construction — that is
/// what `Section`'s two page numbers mean — so each is looked for in that page's own stretch and
/// nowhere else. Searching the whole would let a cross-reference in the body ("see 9.7,
/// "Composite fonts"") end the section early, and ISO 32000-2 is full of them.
fn edges(assembled: &str, starts: &[usize], section: &Section) -> (Option<usize>, Option<usize>) {
    let first = starts.first().copied().unwrap_or(0);
    let first_end = starts.get(1).copied().unwrap_or(assembled.len());
    let start = assembled
        .get(first..first_end)
        .and_then(|page| find_squeezed(page, &section.title))
        .map(|at| first.saturating_add(at));
    let last = starts.last().copied().unwrap_or(0);
    let from = last.max(start.map_or(0, |at| at.saturating_add(1)));
    let end = section.ends_at.as_deref().and_then(|next| {
        assembled
            .get(from..)
            .and_then(|tail| find_squeezed(tail, next))
            .map(|at| from.saturating_add(at))
    });
    (start, end)
}

/// The readback with §14.8.2.2's artifacts taken out.
///
/// §14.8.2.2.1 divides a document's content in two and puts everything but the author's in the
/// second class:
///
/// > All other content is considered to be artifacts , whether generated by the PDF writer in
/// > the course of pagination, layout, or other mechanical processes or introduced by the
/// > document author for decoration or other purposes that are not relevant for understanding
/// > the content of the document.
///
/// A running head, a folio and the licence stamp on every page of a sponsored ISO document are
/// all artifacts, and a program retrieving a clause's text does not want any of them. It is
/// **not** the default, because the default is the readback the text gate measures and an
/// answer that quietly differed from it would put this crate between a caller and that
/// measurement.
fn without_artifacts(text: &str, interpretation: &Interpretation) -> String {
    if interpretation.artifacts.is_empty() {
        return text.to_owned();
    }
    text.char_indices()
        .filter(|(at, _)| {
            !interpretation
                .artifacts
                .iter()
                .any(|span| span.range.contains(at))
        })
        .map(|(_, character)| character)
        .collect()
}

/// Where a heading occurs in a page's text, ignoring the spaces and the case.
///
/// Two extractions of one heading do not agree about the spaces between its words — PDF
/// positions glyphs and not words, so `9.6.5.4 Encodings for TrueType fonts` in the outline is
/// `9.6.5.4  Encodings for TrueType fonts` in the page, and `tools/spec-errata` found the same
/// thing 72 times over (ADR 0253). So both sides are squeezed before the search and the answer is
/// mapped back to a byte offset in the original.
///
/// `None` where the heading is not on the page at all, which is a real answer: an outline item
/// may point at a page whose heading is drawn as an image, or point one page out.
fn find_squeezed(haystack: &str, needle: &str) -> Option<usize> {
    let wanted: String = needle
        .chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect();
    if wanted.is_empty() {
        return None;
    }
    // One entry per *byte* of the squeezed string, holding the offset in `haystack` of the
    // character that produced it, so that `find`'s byte answer maps straight back.
    let mut squeezed = String::new();
    let mut offsets: Vec<usize> = Vec::new();
    for (at, character) in haystack.char_indices() {
        if character.is_whitespace() {
            continue;
        }
        for lowered in character.to_lowercase() {
            let mut buffer = [0_u8; 4];
            for _ in 0..lowered.encode_utf8(&mut buffer).len() {
                offsets.push(at);
            }
            squeezed.push(lowered);
        }
    }
    let found = squeezed.find(&wanted)?;
    offsets.get(found).copied()
}

#[cfg(test)]
mod tests {
    use super::find_squeezed;

    /// The spacing a heading is found across, and the case.
    #[test]
    fn a_heading_is_found_whatever_its_spacing() {
        let page = "ISO 32000-2:2020(E)\n9.7  Composite fonts \nA composite font";
        assert_eq!(find_squeezed(page, "9.7 Composite fonts"), Some(20));
        assert_eq!(find_squeezed(page, "9.7 COMPOSITE FONTS"), Some(20));
        assert_eq!(find_squeezed(page, "9.8 Embedded fonts"), None);
        assert_eq!(find_squeezed(page, "   "), None);
    }

    /// A multi-byte character before the match does not move the answer.
    ///
    /// The offsets are per byte of the squeezed string and the haystack's are per byte of the
    /// original; a table indexed by *character* would put every heading after an em dash one or
    /// two bytes early, which is a class of defect that shows up as text starting mid-word.
    #[test]
    fn a_wide_character_before_the_match_does_not_shift_it() {
        let page = "© ISO 2020 — all rights reserved\n9.7 Composite fonts";
        let at = find_squeezed(page, "9.7 Composite fonts").expect("the heading is there");
        assert!(
            page.get(at..).is_some_and(|tail| tail.starts_with("9.7")),
            "the offset lands on the heading itself: {:?}",
            page.get(at..)
        );
    }
}
