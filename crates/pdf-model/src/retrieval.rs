//! Addressing a document by what it says about itself: its sections, and what an annotation
//! covers.
//!
//! Every other module here answers a question ISO 32000-2 asks — what a page draws, what an
//! outline item points at, what a structure element is. This one answers the question a
//! *consumer* asks, which the standard does not state and which every reader of a large document
//! has: **give me §9.6.5.4, and the notes attached to it.** The pieces were all present and
//! joined to nothing — §12.3.3's outline names destinations, §12.3.2's destinations name pages,
//! §12.5.6.10's `/QuadPoints` name text — and this module is the join.
//!
//! # What is derived and what is chosen
//!
//! Principle 5's line matters here, because half of this is the standard's and half is not.
//!
//! **Derived.** A section's *pages* come from §12.3.3: an outline is "a hierarchy of outline
//! items … which serve as a visual table of contents", and each item's destination is
//! §12.3.2.2's, so the page an item begins on is the document's own statement rather than this
//! reader's inference. [`text_under`] is §12.5.6.10's, whose `/QuadPoints` are stated to
//! "encompass a word or group of contiguous words in the text underlying the annotation" — the
//! clause says the entry names text, and this reads which text it names.
//!
//! **Chosen, and the standard states nothing about either.** That a section *number* is the
//! leading token of an outline item's title is a convention of technical documents and not a
//! rule of ISO 32000-2, which makes `/Title` "[t]he text that shall be displayed on the screen
//! for this item" and says nothing about its shape. And that a section ends where the next
//! outline item that is **not** one of its descendants begins is a choice among two defensible
//! ones — the other being that it ends at the next item of any kind, which would make asking
//! for §9.6 give the paragraph above §9.6.1 and nothing else. Asking for a section and being
//! given its subsections is what a reader means; both are documented as decisions in ADR 0257
//! rather than presented as readings.

use std::collections::BTreeSet;

use pdf_render::{Transform, geom::Point};
use pdf_syntax::{Dictionary, Document, ObjectId};

use crate::outline::{Item, Outline};
use crate::page::{Page, Pages};

/// One addressable section: an outline item, and the pages its text occupies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    /// The outline item's own object, which is how §12.3.3 says to *activate* it.
    pub id: ObjectId,
    /// The leading token of the title where it looks like a clause number, as `9.6.5.4`.
    ///
    /// `None` for `Foreword`, `Contents` and every other heading a document does not number.
    pub number: Option<String>,
    /// Table 151's `/Title`, whole and unmodified.
    pub title: String,
    /// How deep in the outline the item sits, the root's own items being 0.
    pub depth: usize,
    /// The zero-based index of the page the section begins on.
    pub first_page: usize,
    /// The zero-based index of the last page it can reach, **inclusive**.
    ///
    /// The page the next non-descendant item begins on, because a section ends where the next
    /// one starts and the two share that page whenever a heading falls mid-page. The document's
    /// last page where no later item resolves to one.
    pub last_page: usize,
    /// The title of the item that ends this section, where there is one.
    ///
    /// A consumer trimming the section's text needs to find where it stops, and this is the
    /// string to look for. `None` for the last addressable section of the document.
    pub ends_at: Option<String>,
}

/// Every section a document's outline addresses, in the outline's own order.
///
/// An item with no resolvable destination is left out rather than given a page: §12.3.3 permits
/// an item whose `/A` is any of §12.6's actions, and an item that runs a JavaScript or opens a
/// file has not named a place in *this* document. Nothing is invented for it.
///
/// One walk of the page tree for the whole outline, through [`Pages::indices`], for the reason
/// [`Outline::section_at`] gives: resolving 988 destinations one at a time is 988 tree walks.
#[must_use]
pub fn sections(document: &Document, pages: &Pages<'_>, outline: &Outline) -> Vec<Section> {
    let indices = pages.indices();
    let mut flat: Vec<(usize, &Item)> = Vec::new();
    flatten(&outline.items, 0, &mut flat);
    let resolved: Vec<Option<usize>> = flat
        .iter()
        .map(|(_, item)| {
            item.destination
                .and_then(|destination| destination.page_index_with(document, &indices))
        })
        .collect();
    let last = pages.len().saturating_sub(1);
    let mut out = Vec::new();
    for (position, (depth, item)) in flat.iter().enumerate() {
        let Some(first_page) = resolved.get(position).copied().flatten() else {
            continue;
        };
        // The next item that is not a descendant *and* names a page. An item that names no page
        // ends nothing, because a consumer cannot look for a heading on a page nobody stated.
        let ended_by = flat
            .iter()
            .enumerate()
            .skip(position.saturating_add(1))
            .find(|(next, (next_depth, _))| {
                *next_depth <= *depth && resolved.get(*next).copied().flatten().is_some()
            });
        out.push(Section {
            id: item.id,
            number: number_of(&item.title),
            title: item.title.clone(),
            depth: *depth,
            first_page,
            last_page: ended_by
                .and_then(|(next, _)| resolved.get(next).copied().flatten())
                .map_or(last, |page| page.max(first_page)),
            ends_at: ended_by.map(|(_, (_, item))| item.title.clone()),
        });
    }
    out
}

/// The section an address names, by clause number or by the start of its title.
///
/// The number is tried first and exactly, so `9.6.5` is §9.6.5 and never §9.6.5.4; a leading
/// section sign is accepted because that is how this project writes one. Failing that, the
/// address is
/// matched against the start of a title with spaces and case removed, which is what makes
/// `composite fonts` find `9.7  Composite fonts` — two extractions of one heading do not agree
/// about the spaces between its words, and a caller typing a title has no way to know which
/// spacing the file used.
#[must_use]
pub fn section<'a>(sections: &'a [Section], address: &str) -> Option<&'a Section> {
    // The section sign this project writes in front of a clause number, dropped so that
    // `9.6.5.4` and the way a comment spells it address the same thing.
    let wanted = address.trim().trim_start_matches('\u{a7}').trim();
    if wanted.is_empty() {
        return None;
    }
    if let Some(found) = sections
        .iter()
        .find(|section| section.number.as_deref() == Some(wanted))
    {
        return Some(found);
    }
    let asked = squeezed(wanted);
    sections
        .iter()
        .find(|section| squeezed(&section.title).starts_with(&asked))
}

/// The page's own text under an annotation's `/QuadPoints`.
///
/// Table 182 gives the entry to §12.5.6.10's four text markup annotations and states what it
/// names:
///
/// > Each quadrilateral shall encompasses a word or group of contiguous words in the text
/// > underlying the annotation.
///
/// so the text under a highlight is a fact the file states and this reads it back.
/// `Interpretation::text_layer` is in the display list's own coordinates, so the quadrilaterals
/// go through [`crate::content::page_transform`] — the map that exists for exactly this
/// direction.
///
/// `None` where the annotation states no well-formed `/QuadPoints`, and where the quadrilaterals
/// cover no glyph at all — a strikeout drawn over a figure retires no words, which is the
/// distinction `tools/spec-errata` counts by.
#[must_use]
pub fn text_under(
    document: &Document,
    annotation: &Dictionary,
    page: &Page,
    interpretation: &crate::Interpretation,
) -> Option<String> {
    let mut out = String::new();
    for span in spans_under(document, annotation, page, interpretation) {
        if let Some(text) = interpretation.text.get(span) {
            out.push_str(text);
        }
    }
    (!out.trim().is_empty()).then_some(out)
}

/// Where in the page's readback [`text_under`] found that text, in ascending order.
///
/// The ranges rather than the string, for a caller that has to place the annotation as well as
/// read it — which section of a document a highlight belongs to is a question about *where* its
/// text is, and a string cannot answer it. Empty where the annotation states no well-formed
/// `/QuadPoints` or covers no glyph.
#[must_use]
pub fn spans_under(
    document: &Document,
    annotation: &Dictionary,
    page: &Page,
    interpretation: &crate::Interpretation,
) -> Vec<std::ops::Range<usize>> {
    let quads = document.get_key(annotation, "QuadPoints");
    let Some(array) = quads.as_array() else {
        return Vec::new();
    };
    let numbers: Vec<f32> = array
        .iter()
        .filter_map(|item| document.resolve(item).as_number())
        .map(|value| {
            // `as_number` answers an `f64` and the geometry is `f32`; a coordinate outside
            // `f32`'s range is not a page coordinate, and `as` saturates rather than wrapping.
            #[expect(
                clippy::cast_possible_truncation,
                reason = "the display list's own precision, and a page coordinate fits it"
            )]
            let narrowed = value as f32;
            narrowed
        })
        .filter(|value| value.is_finite())
        .collect();
    // "8 × n numbers": anything else has stated no quadrilateral.
    if numbers.is_empty() || !numbers.len().is_multiple_of(8) {
        return Vec::new();
    }
    let transform = crate::content::page_transform(page);
    let mut spans: Vec<std::ops::Range<usize>> = Vec::new();
    for placed in &interpretation.text_layer {
        if placed.span.is_empty() {
            continue;
        }
        let Some(centre) = centre(&placed.quad) else {
            continue;
        };
        if numbers
            .chunks_exact(8)
            .any(|quad| contains(quad, transform, centre))
        {
            spans.push(placed.span.clone());
        }
    }
    spans.sort_by_key(|span| span.start);
    spans.dedup();
    spans
}

/// The outline in reading order, each item with its depth.
fn flatten<'a>(items: &'a [Item], depth: usize, out: &mut Vec<(usize, &'a Item)>) {
    for item in items {
        out.push((depth, item));
        flatten(&item.children, depth.saturating_add(1), out);
    }
}

/// The leading token of a heading, where it looks like a clause number rather than a word.
///
/// Digits, full stops and capitals, with at least one digit among them: `9.6.5.4`, `0.1` and
/// annex `D.2` qualify, while `Foreword`, `Contents` and `ISO` do not. A trailing full stop is
/// dropped, because a document that writes `9.7.` and one that writes `9.7` have named the same
/// clause.
fn number_of(title: &str) -> Option<String> {
    let token = title.split_whitespace().next()?.trim_end_matches('.');
    if token.is_empty()
        || !token.chars().all(|character| {
            character.is_ascii_digit() || character == '.' || character.is_ascii_uppercase()
        })
    {
        return None;
    }
    token
        .chars()
        .any(|character| character.is_ascii_digit())
        .then(|| token.to_owned())
}

/// A heading with its spaces and its case taken out, which is how two spellings of one are
/// compared.
fn squeezed(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

/// The centre of a glyph's quadrilateral, in the display list's coordinates.
fn centre(quad: &[f32; 8]) -> Option<Point> {
    let mut x = 0.0f32;
    let mut y = 0.0f32;
    for pair in quad.chunks_exact(2) {
        x += pair.first()?;
        y += pair.get(1)?;
    }
    Some(Point {
        x: x / 4.0,
        y: y / 4.0,
    })
}

/// Whether a point of the display list lies in one `/QuadPoints` quadrilateral.
///
/// The quadrilateral's corners are mapped rather than the point unmapped, because
/// [`crate::content::page_transform`] is the direction this crate has and inverting it would
/// answer `None` for a degenerate page rather than for a degenerate quadrilateral.
///
/// A bounding box rather than the crossing-number rule the clause's counterclockwise order would
/// want: §12.5.6.10's own NOTE says producers have written these corners "in a different order",
/// and every quadrilateral in the documents this has been run over is axis-aligned, so the box is
/// the quadrilateral.
fn contains(quad: &[f32], transform: Transform, point: Point) -> bool {
    let mut low = Point {
        x: f32::MAX,
        y: f32::MAX,
    };
    let mut high = Point {
        x: f32::MIN,
        y: f32::MIN,
    };
    for pair in quad.chunks_exact(2) {
        let (Some(x), Some(y)) = (pair.first(), pair.get(1)) else {
            return false;
        };
        let mapped = transform.apply(Point { x: *x, y: *y });
        low.x = low.x.min(mapped.x);
        low.y = low.y.min(mapped.y);
        high.x = high.x.max(mapped.x);
        high.y = high.y.max(mapped.y);
    }
    point.x >= low.x && point.x <= high.x && point.y >= low.y && point.y <= high.y
}

/// Every annotation on a page, as the dictionary it is.
///
/// The array itself rather than a reading of it, because what a retrieval consumer wants out of
/// an annotation depends on why it is asking — the `/Contents` of a note, the `/Subj` of an
/// erratum, the text under a highlight — and this crate already reads each of those elsewhere.
/// Bounded by the set of references seen, because §7.7.3.3's `/Annots` is a document's array and
/// a file may repeat an entry.
#[must_use]
pub fn annotations(document: &Document, page: &Page) -> Vec<Dictionary> {
    let list = document.get_key(&page.dict, "Annots");
    let Some(items) = list.as_array() else {
        return Vec::new();
    };
    let mut seen: BTreeSet<ObjectId> = BTreeSet::new();
    let mut out = Vec::new();
    for item in items {
        if let Some(id) = item.as_reference()
            && !seen.insert(id)
        {
            continue;
        }
        if let Some(dict) = document.resolve(item).as_dict() {
            out.push(dict.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use pdf_syntax::ObjectId;

    use super::{Section, number_of, section};

    /// A section with the fields the address lookup reads and nothing else.
    fn addressable(number: Option<&str>, title: &str) -> Section {
        Section {
            id: ObjectId::new(1, 0),
            number: number.map(str::to_owned),
            title: title.to_owned(),
            depth: 0,
            first_page: 0,
            last_page: 0,
            ends_at: None,
        }
    }

    /// Which leading tokens are clause numbers and which are words.
    ///
    /// The rule has to separate `9.6.5.4` and annex `D.2` from `Foreword`, `Contents` and the
    /// `ISO` that begins three of ISO 32000-2's own front-matter items — 947 of that document's
    /// 988 outline items are numbered and the other 41 must not acquire a number here, because
    /// an invented one is an address that answers the wrong clause.
    #[test]
    fn a_clause_number_is_told_from_a_word() {
        assert_eq!(
            number_of("9.6.5.4 Encodings for TrueType fonts").as_deref(),
            Some("9.6.5.4")
        );
        assert_eq!(number_of("0.1 PDF").as_deref(), Some("0.1"));
        assert_eq!(
            number_of("D.2 Latin-text encodings").as_deref(),
            Some("D.2")
        );
        assert_eq!(number_of("9.7.  Composite fonts").as_deref(), Some("9.7"));
        assert_eq!(number_of("Foreword"), None);
        assert_eq!(number_of("Contents"), None);
        assert_eq!(number_of("ISO 32000-2:2020 front page"), None);
        assert_eq!(number_of(""), None);
    }

    /// A number is matched exactly and a title loosely, in that order.
    ///
    /// The exactness is what stops `9.6.5` answering `9.6.5.4`, which a prefix match would do
    /// and which is the difference between a clause and its parent. The looseness on titles is
    /// the opposite requirement: a caller typing a heading cannot know how many spaces the file
    /// puts between its number and its words.
    #[test]
    fn an_address_is_a_number_first_and_a_title_second() {
        let sections = vec![
            addressable(Some("9.6.5"), "9.6.5 Character encoding"),
            addressable(Some("9.6.5.4"), "9.6.5.4 Encodings for TrueType fonts"),
            addressable(None, "Foreword"),
        ];
        assert_eq!(
            section(&sections, "9.6.5").map(|found| found.title.as_str()),
            Some("9.6.5 Character encoding")
        );
        assert_eq!(
            section(&sections, "§9.6.5.4").map(|found| found.title.as_str()),
            Some("9.6.5.4 Encodings for TrueType fonts")
        );
        assert_eq!(
            section(&sections, "  foreword ").map(|found| found.title.as_str()),
            Some("Foreword")
        );
        assert_eq!(
            section(&sections, "9.6.5.4  Encodings").map(|found| found.title.as_str()),
            Some("9.6.5.4 Encodings for TrueType fonts"),
            "a title is matched with its spaces taken out"
        );
        assert!(section(&sections, "11.4.7").is_none());
        assert!(section(&sections, "  ").is_none());
    }
}
