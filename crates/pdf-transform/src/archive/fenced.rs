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

/// The operator row, whose pages the survey names exactly.
const OPERATORS: &str = "graphics/only-operators-the-base-standard-defines";

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
/// refusal it had.
pub(super) fn pages(input: &Report, survey: &Survey) -> Fenced {
    let mut found = Fenced::default();
    for judgement in input.failures() {
        let Some(row) = ROWS.iter().copied().find(|row| *row == judgement.id) else {
            continue;
        };
        let pages: BTreeSet<usize> = if row == OPERATORS {
            for unlisted in survey.unlisted_operators() {
                found
                    .operators
                    .insert(String::from_utf8_lossy(&unlisted.spelling).into_owned());
            }
            survey
                .unlisted_operators()
                .iter()
                .map(|unlisted| unlisted.page)
                .collect()
        } else {
            let Outcome::Failed { places, total } = &judgement.outcome else {
                continue;
            };
            let named: Option<BTreeSet<usize>> =
                places.iter().map(|finding| finding.place.page).collect();
            match named {
                Some(named) if places.len() == *total => named,
                _ => continue,
            }
        };
        for page in pages {
            found.pages.entry(page).or_default().insert(row);
        }
    }
    found
}
