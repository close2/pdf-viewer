//! The pages that hold a fault of their content stream the target refuses, found so that
//! `as-if-printed` can photograph them (`doc/adr/2516`, owner answer A694 item 1).
//!
//! # Why a page, and which pages
//!
//! ADR 0816's fence is that the converter never edits a producer's content stream, so a rule a
//! content stream fails (an operator the base standard does not define, a hexadecimal string with a
//! byte that is no digit, an inline image's `/I`) has no repair on the page. What a company does
//! with such a letter is print it and scan the print: the page survives as the marks the window
//! draws, and the fault is not in them. The page is photographed by `fonts::photographed`
//! and the requirement stops being failed because the content stream that failed it is no longer
//! in the output.
//!
//! This module only says **which pages**. A requirement is covered when every place the validator
//! kept for it names a page and the list is whole; one that reports an object, or a list past the
//! validator's bound, is left to its refusal, because the page that reaches the object is not
//! known from the report. The output is held to the target afterwards (`super::hold_the_output_to_the_target`),
//! so a page missed here makes the file refused rather than wrong.

use std::collections::{BTreeMap, BTreeSet};

use pdf_archive::Report;
use pdf_archive::report::Outcome;
use pdf_archive::survey::Survey;
use pdf_model::Pages;
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Object, ObjectId};

/// The operator row, whose pages the survey names exactly.
pub(super) const OPERATORS: &str = "graphics/only-operators-the-base-standard-defines";

/// The rows a page's content stream fails and that nothing repairs on the page, besides
/// [`OPERATORS`] (ISO 19005-2 section 6.2.2, ISO 19005-4 section 6.2.2): an inline image's
/// interpolation, a hexadecimal string holding a byte that is no digit, a glyph procedure whose
/// width the procedure contradicts, an `/ActualText` holding a Private Use character, and a bound
/// name that is not UTF-8 (ISO 19005-2 section 6.1.8).
pub(super) const ROWS: [&str; 6] = [
    OPERATORS,
    "graphics/inline-image-interpolation-is-off",
    "file-structure/hexadecimal-string-holds-only-digits",
    "fonts/type3-glyph-procedures-state-their-width",
    "fonts/actual-text-states-no-private-use",
    "file-structure/bound-names-are-valid-utf8",
];

/// What the failed fenced rows hold on the document's pages.
#[derive(Debug, Default)]
pub(super) struct Fenced {
    /// The pages (zero-based) each failed row holds a fault on, keyed by page.
    pub(super) pages: BTreeMap<usize, BTreeSet<&'static str>>,
    /// The spellings of the operators no standard defines that the content streams used: the one
    /// kind of operator the interpreter reports and the photograph may still be taken over, since
    /// no reader draws anything for it.
    pub(super) operators: BTreeSet<String>,
}

/// The pages each failed fenced row holds a fault on.
///
/// A row whose pages are not all known is absent from every value, so the requirement keeps the
/// refusal it had. Where `drawn` is given, `super::as_drawn` wrote the number this reader draws
/// in place of every run it salvages in the streams it could read, so an operator that is such a
/// run, in such a stream, holds no page: its page converts as text (`doc/adr/2638`). A run in a
/// stream it could not read is still in the output, and its page is photographed (ADR 2803).
///
/// **An operator in a stream an annotation's appearance reaches holds no page either**, and its
/// row keeps the refusal (ADR 2803): a photograph replaces the page's `/Contents` and
/// `/Resources` and leaves its `/Annots`, so ISO 32000-2 §12.5.5's appearance stream, and every
/// form, pattern, glyph procedure and soft mask group it names, is still in the output with the
/// operator in it.
pub(super) fn pages(
    document: &Document,
    input: &Report,
    survey: &Survey,
    drawn: Option<&super::as_drawn::Drawn>,
) -> Fenced {
    let mut found = Fenced::default();
    let mut annotations_reach: Option<BTreeSet<ObjectId>> = None;
    for judgement in input.failures() {
        let Some(row) = ROWS.iter().copied().find(|row| *row == judgement.id) else {
            continue;
        };
        let pages: BTreeSet<usize> = if row == OPERATORS {
            let unwritten = || {
                survey.unlisted_operators().iter().filter(|unlisted| {
                    !drawn.is_some_and(|drawn| {
                        drawn.answers(&unlisted.spelling) && !drawn.left(document, unlisted)
                    })
                })
            };
            let reached =
                annotations_reach.get_or_insert_with(|| reached_from_annotations(document));
            if unwritten().any(|unlisted| {
                unlisted
                    .place
                    .object
                    .is_some_and(|object| reached.contains(&object))
            }) {
                continue;
            }
            for unlisted in unwritten() {
                found
                    .operators
                    .insert(String::from_utf8_lossy(&unlisted.spelling).into_owned());
            }
            unwritten().map(|unlisted| unlisted.page).collect()
        } else {
            let Outcome::Failed { places, total } = &judgement.outcome else {
                continue;
            };
            if row == TYPE3_WIDTHS {
                let reached =
                    annotations_reach.get_or_insert_with(|| reached_from_annotations(document));
                match type3_pages(document, judgement, reached) {
                    Some(pages) => pages,
                    None => continue,
                }
            } else {
                let named: Option<BTreeSet<usize>> =
                    places.iter().map(|finding| finding.place.page).collect();
                match named {
                    Some(named) if places.len() == *total => named,
                    _ => continue,
                }
            }
        };
        for page in pages {
            found.pages.entry(page).or_default().insert(row);
        }
    }
    found
}

/// The row whose places name a Type 3 font rather than a page (ISO 19005-4 section 6.2.10.5).
const TYPE3_WIDTHS: &str = "fonts/type3-glyph-procedures-state-their-width";

/// The pages a Type 3 width row's faults are on, or `None` where they are not all known.
///
/// ISO 19005-4 section 6.2.10.5's glyph procedure width is reported at its font, which no page
/// holds, so the pages are the ones whose resources reach the font; the validator keeps every
/// object it named past the places it lists (`pdf_archive::Named`), and a font written directly
/// in a resource dictionary is reported at its page. A font an annotation's appearance reaches
/// keeps the refusal, as an operator there does, because a photograph leaves `/Annots` (ADR
/// 2803, ADR 2847).
fn type3_pages(
    document: &Document,
    judgement: &pdf_archive::Judgement,
    annotations_reach: &BTreeSet<ObjectId>,
) -> Option<BTreeSet<usize>> {
    let Outcome::Failed { places, total } = &judgement.outcome else {
        return None;
    };
    let named = &judgement.named;
    if named.cut_short() || (named.elsewhere() && places.len() != *total) {
        return None;
    }
    let mut pages: BTreeSet<usize> = places
        .iter()
        .filter(|finding| finding.place.object.is_none())
        .map(|finding| finding.place.page)
        .collect::<Option<_>>()?;
    for &font in named.objects() {
        let reaching = pages_reaching(document, font);
        if annotations_reach.contains(&font) || reaching.is_empty() {
            return None;
        }
        pages.extend(reaching);
    }
    Some(pages)
}

/// The pages (zero-based) whose resources reach `object`, through every form `XObject`, pattern,
/// font and soft mask group below them: the pages a photograph has to take for the object to
/// leave the output with their resources.
fn pages_reaching(document: &Document, object: ObjectId) -> BTreeSet<usize> {
    Pages::new(document)
        .every()
        .into_iter()
        .enumerate()
        .filter_map(|(index, page)| {
            let page = page?;
            let mut queue = Vec::new();
            resource_entries(document, &page.resources, &mut queue);
            reached_from(document, queue)
                .contains(&object)
                .then_some(index)
        })
        .collect()
}

/// How many objects [`reached_from_annotations`] visits before it stops; past it the walk keeps
/// what it found, and an operator in a stream it did not reach is photographed over and still
/// held to the target afterwards, which refuses a file the photograph left failing.
const MOST_REACHED: usize = 1 << 16;

/// Every stream an annotation's appearance reaches: the appearance streams themselves (§12.5.5's
/// Table 168) and, through each one's `/Resources`, every form `XObject`, pattern, Type 3 glyph
/// procedure and soft mask group below them.
fn reached_from_annotations(document: &Document) -> BTreeSet<ObjectId> {
    let mut queue: Vec<Object> = Vec::new();
    for page in Pages::new(document).every().into_iter().flatten() {
        queue.extend(super::rewrite::appearance_streams(document, &page.dict));
    }
    reached_from(document, queue)
}

/// Every object `queue`'s references reach through `/Resources`, glyph procedures and soft mask
/// groups, bounded at [`MOST_REACHED`].
fn reached_from(document: &Document, mut queue: Vec<Object>) -> BTreeSet<ObjectId> {
    let mut reached = BTreeSet::new();
    while let Some(next) = queue.pop() {
        if reached.len() >= MOST_REACHED {
            break;
        }
        let Some(id) = next.as_reference() else {
            continue;
        };
        if !reached.insert(id) {
            continue;
        }
        let object = document.get(id);
        let dict = match &object {
            Object::Stream(stream) => &stream.dict,
            Object::Dictionary(dict) => dict,
            _ => continue,
        };
        // A Type 3 font's glyph procedures, where this object is one.
        if let Some(procedures) = document.get_key(dict, "CharProcs").as_dict() {
            queue.extend(procedures.iter().map(|(_, procedure)| procedure.clone()));
        }
        let resources = document.get_key(dict, "Resources");
        if let Some(resources) = resources.as_dict() {
            resource_entries(document, resources, &mut queue);
        }
    }
    reached
}

/// The entries of one resource dictionary the walk follows: its forms, patterns and fonts, and the
/// soft mask groups its graphics states name.
fn resource_entries(document: &Document, resources: &Dictionary, queue: &mut Vec<Object>) {
    for category in ["XObject", "Pattern", "Font"] {
        if let Some(entries) = document.get_key(resources, category).as_dict() {
            queue.extend(entries.iter().map(|(_, entry)| entry.clone()));
        }
    }
    if let Some(states) = document.get_key(resources, "ExtGState").as_dict() {
        for (_, state) in states.iter() {
            let state = document.resolve(state);
            let group = state
                .as_dict()
                .map(|state| document.get_key(state, "SMask"))
                .and_then(|mask| mask.as_dict().and_then(|mask| mask.get("G").cloned()));
            queue.extend(group);
        }
    }
}
