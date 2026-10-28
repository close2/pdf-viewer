//! ISO 32000-2 Annex F's linearized file whose first-page trailer names a main section that is
//! not there, measured on the document that does it.
//!
//! `doc/corpora/format-corpus/govdocs1-error-pdfs/error_set_2/027613.pdf` is linearized and
//! states `/N 429`. Its first-page section's `/Prev` is 1569328, the main table's `xref` keyword
//! stands at 1569316, and the file is twelve bytes shorter than its own `/L` (§F.3.3 Table F.1).
//! A reader that stops at the `/Prev` has the first page's 260 objects and draws one page. The
//! construction is ADR 2526; `pdf-syntax/tests/cross_references.rs` holds the same shape built by
//! hand, and this file is the producer's own bytes.

use std::path::Path;

use pdf_model::Pages;
use pdf_syntax::Document;

/// The pages the linearization parameter dictionary's `/N` states, which §F.3.3 Table F.1 defines
/// as "[t]he number of pages in the document".
const PAGES_STATED_BY_N: usize = 429;

/// The pages of `027613` are all reached although its first-page `/Prev` names no section.
#[test]
fn a_linearized_file_whose_previous_names_no_section_draws_all_the_pages_it_states() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/corpora/format-corpus/govdocs1-error-pdfs/error_set_2/027613.pdf");
    let Ok(bytes) = std::fs::read(path) else {
        println!("skipped: doc/corpora/format-corpus is not checked out");
        return;
    };
    let document =
        Document::open(bytes).unwrap_or_else(|e| panic!("027613.pdf does not open: {e}"));
    assert!(
        document.cross_reference_chain_was_cut(),
        "the document's /Prev names bytes that are no section"
    );
    assert!(
        !document.was_recovered(),
        "the first-page section was read, so the table was not rebuilt by a scan"
    );
    assert_eq!(Pages::new(&document).len(), PAGES_STATED_BY_N);
}
