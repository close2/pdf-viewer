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
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R {catalog} \
         /AcroForm << /Fields [22 0 R] /DR << /Font << /Helv 20 0 R >> >> >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R {page_extra} >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n\
         20 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n\
         21 0 obj\n<< /Type /Annot /Subtype /Square /Rect [120 20 180 60] /C [1 0 0] \
         /Contents (a red square) >>\nendobj\n\
         22 0 obj\n<< /Type /Annot /Subtype /Widget /FT /Tx /T (f) /V (typed) \
         /Rect [120 20 180 60] /DA (/Helv 9 Tf 0 g) /Contents (a text field) >>\nendobj\n\
         {structure}",
        content.len() + 1,
    );

    // Offsets are keyed by the number each object states rather than by its position, so a
    // test may add objects at any number without renumbering the ones this builder supplies.
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
    let mut cursor = out.len();
    for object in body.split_inclusive("endobj\n") {
        let number: usize = object
            .split_whitespace()
            .next()
            .and_then(|word| word.parse().ok())
            .expect("every object states its number");
        offsets.insert(number, cursor);
        cursor += object.len();
    }
    out.push_str(&body);
    let xref_at = out.len();
    let size = offsets.keys().copied().max().unwrap_or(0) + 1;
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for number in 1..size {
        match offsets.get(&number) {
            Some(offset) => {
                let _ = writeln!(out, "{offset:010} 00000 n ");
            }
            None => {
                let _ = writeln!(out, "0000000000 65535 f ");
            }
        }
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
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

/// §14.9.4: two consecutive replacement texts have no word break between them.
///
/// > If each of two (or more) consecutive structure or marked-content sequences has an
/// > ActualText entry, they shall be treated as if no word break is present between them.
///
/// The two glyphs are placed far enough apart that the readback's own space inference puts a
/// space between them — which is what makes this test discriminate. That space is neither
/// sequence's; it is what the *placement* pass concluded from the gap, and the clause says the
/// two replacements are one word.
#[test]
fn consecutive_replacement_texts_are_not_separated() {
    let apart = interpret(
        "BT /F1 12 Tf 10 50 Td (Dru) Tj 40 0 Td (k) Tj ET",
        "",
        "",
        "",
    );
    assert!(
        apart.text.contains(' '),
        "the fixture's gap infers a space without the entries: {:?}",
        apart.text
    );

    let drawn = interpret(
        "BT /F1 12 Tf 10 50 Td /Span << /ActualText (Dru) >> BDC (Dru) Tj EMC \
         40 0 Td /Span << /ActualText (c) >> BDC (k) Tj EMC ET",
        "",
        "",
        "",
    );
    assert_eq!(drawn.text.trim_end(), "Druc");
}

/// A sequence between two replacements ends their adjacency.
///
/// The rule is about *consecutive* sequences, so text drawn between two of them is a word break
/// the clause does not remove — otherwise every `/ActualText` on a page would run into the next.
#[test]
fn a_replacement_after_ordinary_text_keeps_its_break() {
    let drawn = interpret(
        "BT /F1 12 Tf 10 50 Td /Span << /ActualText (one) >> BDC (a) Tj EMC \
         40 0 Td (X) Tj 40 0 Td /Span << /ActualText (two) >> BDC (b) Tj EMC ET",
        "",
        "",
        "",
    );
    assert!(
        drawn.text.contains('X'),
        "the middle text is still there: {:?}",
        drawn.text
    );
    assert!(
        !drawn.text.contains("onetwo"),
        "the two replacements are not adjacent: {:?}",
        drawn.text
    );
}

/// §14.9.3's third location: an annotation with no text of its own is described by `/Contents`.
///
/// > Any type of annotation (see 12.5, "Annotations") that does not already have a text
/// > representation, through a Contents entry in the annotation dictionary
///
/// The fixture's `Square` annotation has no appearance stream, so this tree constructs one from
/// §12.5.6.8 — a shape and no text at all. What a screen reader is given for it is the
/// `/Contents` the producer wrote, and what a person copying the page gets is unchanged.
#[test]
fn an_annotation_with_no_text_is_described_by_its_contents() {
    let drawn = interpret(
        "BT /F1 12 Tf 10 50 Td (page text) Tj ET",
        "",
        "",
        "/Annots [21 0 R]",
    );
    assert_eq!(
        drawn.text.trim_end(),
        "page text",
        "extraction is untouched"
    );
    assert_eq!(
        runs(&drawn.speech()),
        vec![("page text a red square ", None)]
    );
}

/// An annotation that *does* read as text keeps its own words.
///
/// The clause's condition is "does not already have a text representation", and it is checked
/// rather than assumed: a widget whose field value this tree lays out (§12.7.4.3) reads back
/// what it drew, so its `/Contents` is not a substitute for it.
#[test]
fn an_annotation_that_reads_as_text_is_not_replaced() {
    let drawn = interpret(
        "BT /F1 12 Tf 10 80 Td (page text) Tj ET",
        "",
        "",
        "/Annots [22 0 R]",
    );
    assert!(
        drawn.text.contains("typed"),
        "the field's own value was drawn: {:?}",
        drawn.text
    );
    assert!(
        !format!("{:?}", drawn.speech()).contains("a text field"),
        "its /Contents did not stand in for it: {:?}",
        drawn.speech()
    );
}
