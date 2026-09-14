//! The survey's reading of ISO 32000-2 §7.8.3's fallbacks, on the fixtures the corpus cannot
//! witness.
//!
//! `tests/cross_check.rs` holds the survey to the interpreter; this file says what the survey
//! alone reads, so that the two agreeing is not the only thing known about it. Each fixture is
//! built so that the page's dictionary and the invoking stream's disagree, which the cross-check
//! over 2 908 corpus documents found nowhere (ADR 1055 section 5, ADR 1059).

#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that cannot be opened should fail loudly"
)]

use pdf_archive::survey::Survey;
use pdf_syntax::Document;

/// One fixture, opened from disk.
fn fixture(name: &str) -> Document {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/resource-fallbacks")
        .join(name);
    let bytes = std::fs::read(&path).expect("the fixture is in the tree");
    Document::open(bytes).expect("the fixture is a valid PDF")
}

/// Every name the survey found undefined, as `category /name`.
fn missing(name: &str) -> Vec<String> {
    Survey::of(&fixture(name))
        .missing_resources()
        .iter()
        .map(|miss| format!("{} /{}", miss.category, miss.name))
        .collect()
}

/// §7.8.3's NOTE 3 and Table 93's `/Resources` cell both name the page's dictionary, and
/// neither names the invoker's: `/Sq` is the page's and is found, `/Inv` is the invoking form's
/// alone and is not.
#[test]
fn a_form_nested_in_a_form_is_read_against_the_pages_dictionary() {
    assert_eq!(
        missing("form-in-form-without-resources.pdf"),
        vec!["XObject /Inv".to_owned()]
    );
}

/// §7.8.3's search for a glyph description's resources ends at the page as well (Errata
/// Collection 3, Issue #128), and the text-showing operator standing inside a form with
/// `/Resources` of its own does not move it.
#[test]
fn a_type3_font_shown_inside_a_form_is_read_against_the_pages_dictionary() {
    assert_eq!(
        missing("type3-in-form-without-resources.pdf"),
        vec!["XObject /Inv".to_owned()]
    );
}

/// Table 74 makes a tiling pattern's `/Resources` "( Required )" and no sentence gives a
/// pattern the fallback a form has, so the page's `/Sq` is not the cell's to find.
#[test]
fn a_tiling_pattern_stating_no_resources_is_read_against_nothing() {
    assert_eq!(
        missing("tiling-pattern-without-resources.pdf"),
        vec!["XObject /Sq".to_owned()]
    );
}
