//! ISO 32000-2 §14.9's accessibility entries, driven through the interpreter.
//!
//! # Why these are here rather than beside `accessibility.rs`
//!
//! The unit tests in that module check the *rules* — which entry replaces what, where a word
//! break goes, which language wins — over spans handed to it directly. What they cannot check
//! is the half this file exists for: that a `BDC` operand's property list and a structure
//! element reached through §14.7.5.4's parent tree both arrive as those spans, with the right
//! ranges over the text the same pass read back. That is a statement about the interpreter, and
//! §14.9.2.3's own three examples are written as content streams, so they are run as content
//! streams.
//!
//! # The one machine dependency
//!
//! A glyph outline comes from a substituted standard-14 face, as in `text_render_modes.rs`. If
//! this machine had no fonts the readback would be empty and every assertion here would pass
//! vacuously, so the helper panics naming that rather than skipping.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that cannot exercise what the test is about is a failure, \
              and these offsets are within a fixture written in this file"
)]

use std::fmt::Write as _;

use pdf_model::accessibility::Spoken;
use pdf_syntax::Document;

/// A one-page fixture, optionally tagged.
///
/// `catalog` and `structure` are spliced in so that one builder serves both routes §14.9 states:
/// a page with no structure at all, and one whose `/StructParents` reaches a parent tree.
fn fixture(content: &str, catalog: &str, structure: &str, page_extra: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R {catalog} >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R {page_extra} >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n{structure}",
        content.len() + 1,
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    let mut cursor = out.len();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(cursor);
        cursor += object.len();
    }
    out.push_str(&body);
    let xref_at = out.len();
    let _ = write!(out, "xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1);
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
        offsets.len() + 1
    );
    out.into_bytes()
}

/// Interprets the fixture's one page.
fn interpret(
    content: &str,
    catalog: &str,
    structure: &str,
    page_extra: &str,
) -> pdf_model::Interpretation {
    let bytes = fixture(content, catalog, structure, page_extra);
    let document = Document::open(bytes).expect("the fixture is a valid file");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0).expect("the fixture has one page");
    let drawn = pdf_model::interpret(&document, &page);
    assert!(
        drawn.glyphs > 0,
        "no glyphs were drawn, so this machine has no substitute for Helvetica and every \
         assertion below would pass vacuously"
    );
    drawn
}

/// The whole spoken text of a page, with its runs' languages, for a compact assertion.
fn runs(spoken: &[Spoken]) -> Vec<(&str, Option<&str>)> {
    spoken
        .iter()
        .map(|run| (run.text.as_str(), run.language.as_deref()))
        .collect()
}

/// §14.9.2.3's EXAMPLE 1, run as the clause writes it: a language stated for the document as a
/// whole, overridden by one on a marked-content sequence, independent of any logical structure.
///
/// The clause's own stream, with the catalog `/Lang` of `en-US` it describes but does not show.
#[test]
fn a_span_overrides_the_documents_language() {
    let drawn = interpret(
        "BT /F1 12 Tf 10 50 Td (See you later, or as Arnold would say, ) Tj \
         /Span << /Lang (es-MX) >> BDC (Hasta la vista.) Tj EMC ET",
        "/Lang (en-US)",
        "",
        "",
    );
    assert_eq!(drawn.language.as_deref(), Some("en-US"));
    assert_eq!(
        runs(&drawn.speech()),
        vec![
            ("See you later, or as Arnold would say, ", Some("en-US")),
            ("Hasta la vista.", Some("es-MX")),
        ]
    );
}

/// §14.9.2.3's EXAMPLE 2: a structure element's language reaches its marked-content sequence.
///
/// > the Lang entry in the structure element dictionary (specifying English) applies to the
/// > marked-content sequence having an MCID (marked-content identifier) value of 0
///
/// The element is found the only way a content stream can find one — the page's
/// `/StructParents` into `/ParentTree`, then the `/MCID` as an index — so this also pins that
/// §14.9's entries take that route and not only the property list's.
#[test]
fn a_structure_element_supplies_the_language_of_its_sequence() {
    let drawn = interpret(
        "BT /F1 12 Tf 10 50 Td /P << /MCID 0 >> BDC \
         (See you later, or in Spanish you would say, ) Tj \
         /Span << /Lang (es-MX) >> BDC (Hasta la vista.) Tj EMC EMC ET",
        "/StructTreeRoot 6 0 R",
        "6 0 obj\n<< /Type /StructTreeRoot /ParentTree 7 0 R >>\nendobj\n\
         7 0 obj\n<< /Nums [3 [8 0 R]] >>\nendobj\n\
         8 0 obj\n<< /Type /StructElem /S /P /Lang (en-US) >>\nendobj\n",
        "/StructParents 3",
    );
    assert_eq!(
        runs(&drawn.speech()),
        vec![
            (
                "See you later, or in Spanish you would say, ",
                Some("en-US")
            ),
            ("Hasta la vista.", Some("es-MX")),
        ]
    );
}

/// §14.9.2.3: an element with no `/Lang` inherits one from an ancestor that has.
///
/// > If a structure element does not have a Lang entry, the element shall inherit its language
/// > from any parent element that has one.
///
/// Two levels, so that taking the element's own parent rather than walking is not enough.
#[test]
fn a_structure_element_inherits_a_language_from_its_ancestry() {
    let drawn = interpret(
        "BT /F1 12 Tf 10 50 Td /P << /MCID 0 >> BDC (mot juste) Tj EMC ET",
        "/StructTreeRoot 6 0 R",
        "6 0 obj\n<< /Type /StructTreeRoot /ParentTree 7 0 R /K 9 0 R >>\nendobj\n\
         7 0 obj\n<< /Nums [3 [8 0 R]] >>\nendobj\n\
         8 0 obj\n<< /Type /StructElem /S /Span /P 10 0 R >>\nendobj\n\
         9 0 obj\n<< /Type /StructElem /S /Document /K [10 0 R] /Lang (fr-FR) >>\nendobj\n\
         10 0 obj\n<< /Type /StructElem /S /P /P 9 0 R /K [8 0 R] >>\nendobj\n",
        "/StructParents 3",
    );
    assert_eq!(runs(&drawn.speech()), vec![("mot juste", Some("fr-FR"))]);
}

/// A cycle in `/P` terminates, answering "no language" rather than hanging.
///
/// `/P` is a reference the document controls and nothing in §14.7 forbids a file from writing
/// a loop; `structure::MAX_ANCESTRY` is what ends the walk. The catalog's language stands,
/// which is the answer an untagged document gives.
#[test]
fn a_cycle_in_the_structure_ancestry_terminates() {
    let drawn = interpret(
        "BT /F1 12 Tf 10 50 Td /P << /MCID 0 >> BDC (round and round) Tj EMC ET",
        "/StructTreeRoot 6 0 R /Lang (en)",
        "6 0 obj\n<< /Type /StructTreeRoot /ParentTree 7 0 R >>\nendobj\n\
         7 0 obj\n<< /Nums [3 [8 0 R]] >>\nendobj\n\
         8 0 obj\n<< /Type /StructElem /S /Span /P 9 0 R >>\nendobj\n\
         9 0 obj\n<< /Type /StructElem /S /P /P 8 0 R >>\nendobj\n",
        "/StructParents 3",
    );
    assert_eq!(runs(&drawn.speech()), vec![("round and round", Some("en"))]);
}

/// §14.9.3's own example: `/Alt` beside `/Lang` on one property list.
///
/// > /Span <</Lang (en-us) /Alt (six-point star)>> BDC (A) Tj EMC
///
/// The drawn glyph is an `A`; what a screen reader is given is the description, in the stated
/// language — and what a person *copying* the page gets is still the `A`, because §14.9.3 is a
/// description rather than §14.9.4's replacement. Both halves are asserted, since a
/// description that leaked into the extraction readback would be a defect no picture shows.
#[test]
fn an_alternate_description_is_spoken_but_not_extracted() {
    let drawn = interpret(
        "BT /F1 12 Tf 10 50 Td /Span << /Lang (en-us) /Alt (six-point star) >> BDC (A) Tj EMC ET",
        "",
        "",
        "",
    );
    assert_eq!(drawn.text.trim_end(), "A", "extraction is untouched");
    assert_eq!(
        runs(&drawn.speech()),
        vec![("six-point star ", Some("en-us"))]
    );
}

/// §14.9.5's own example, both abbreviations expanded.
///
/// > BT /Span <</E (Doctor)>> BDC (Dr.) Tj EMC (Healwell works at 123 Industrial ) Tj
/// > /Span <</E (Drive)>> BDC (Dr.) Tj EMC ET
#[test]
fn an_expansion_is_spoken_in_place_of_its_abbreviation() {
    let drawn = interpret(
        "BT /F1 12 Tf 10 50 Td /Span << /E (Doctor) >> BDC (Dr.) Tj EMC \
         (Healwell works at 123 Industrial ) Tj /Span << /E (Drive) >> BDC (Dr.) Tj EMC ET",
        "",
        "",
        "",
    );
    assert!(drawn.text.contains("Dr."), "extraction is untouched");
    assert_eq!(
        runs(&drawn.speech()),
        vec![("Doctor Healwell works at 123 Industrial Drive ", None)]
    );
}

/// A page that tags nothing carries no spans and speaks exactly what it extracts.
///
/// The cheap half of the claim that this costs untagged pages nothing: `described` is empty, so
/// there is no allocation per sequence and `speech` is one run.
#[test]
fn an_untagged_page_carries_nothing() {
    let drawn = interpret("BT /F1 12 Tf 10 50 Td (plain) Tj ET", "", "", "");
    assert!(drawn.described.is_empty());
    assert_eq!(drawn.language, None);
    assert_eq!(runs(&drawn.speech()), vec![("plain", None)]);
}

/// An empty `/Lang` states that the language is unknown, which is not a language.
///
/// §14.9.2.2: an identifier "shall either be the empty text string, to indicate that the
/// language is unknown, or a Language-Tag as defined in BCP 47". Reading the empty string as a
/// tag would hand a text-to-speech engine an identifier no locale matches.
#[test]
fn an_empty_language_identifier_states_nothing() {
    let drawn = interpret(
        "BT /F1 12 Tf 10 50 Td /Span << /Lang () >> BDC (quoi) Tj EMC ET",
        "/Lang (fr)",
        "",
        "",
    );
    assert!(drawn.described.is_empty(), "an empty tag records no span");
    assert_eq!(runs(&drawn.speech()), vec![("quoi", Some("fr"))]);
}
