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
         /Resources << /Font << /F1 5 0 R >> /Properties << /AFile 30 0 R >> >> \
         /Contents 4 0 R {page_extra} >>\nendobj\n\
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

/// A catalog `/Lang` that is not a BCP 47 tag is unknown, exactly as an absent one is.
///
/// Errata Collection 3 (Issue #105) inserts *or invalid (see 14.9.2, "Natural language
/// specification")* into Table 29's `/Lang` entry, so its last sentence reads: if this entry
/// is absent or invalid, the language shall be considered unknown. §14.9.2.2 is what "invalid"
/// means — not the empty string and not a BCP 47 `Language-Tag` — and the fixture's value is
/// the shape real producers write there: prose naming the language to a person, which no
/// locale matches and no screen reader should be handed as an identifier.
#[test]
fn an_invalid_catalog_language_is_unknown() {
    let drawn = interpret(
        "BT /F1 12 Tf 10 50 Td (was?) Tj ET",
        "/Lang (German, not a tag)",
        "",
        "",
    );
    assert_eq!(drawn.language, None);
    assert_eq!(runs(&drawn.speech()), vec![("was?", None)]);
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

/// §14.9.4's **third** location, which Errata Collection 3 added: an `Artifact` tag's property list.
///
/// The published bullet list names two places a replacement text may be:
///
/// > ( PDF 1.5 ) A marked-content sequence (see 14.6, "Marked content"), through an ActualText
/// > entry in a property list attached to the marked-content sequence with a Span tag.
///
/// Issue #483 (`/State` `Review`, `Accepted`) adds a third as the last item of that list, in the
/// same words but with an `Artifact` tag in place of the `Span` one, pointing at §14.8.2.2.2.
/// It is stated here in prose rather than as a blockquote because it is the erratum's sentence
/// and not the published one, and `doc/md/` carries the published one (ADR 0252).
///
/// The interpreter reads §14.9's four entries off *every* `BDC`'s property list rather than off a
/// `/Span`'s alone, so the erratum is met by construction — and nothing in this tree executed it
/// until this test, which is why it is here rather than as a sentence in the ledger. A folio
/// artifact drawn as roman numerals whose `/ActualText` is the arabic number is the shape the
/// erratum is about: what a person sees is `vii` and what the page reads back is `7`.
///
/// It also pins the *order* of the two records, which `Interpreter::run` takes deliberately: the
/// artifact's range is over the readback as the replacement left it, not over the glyphs the
/// replacement removed.
#[test]
fn an_artifacts_replacement_text_replaces_what_it_encloses() {
    let drawn = interpret(
        "BT /F1 12 Tf 10 80 Td (Real content.) Tj \
         0 -30 Td /Artifact << /Type /Pagination /Subtype /Footer /ActualText (7) >> BDC \
         (vii) Tj EMC ET",
        "",
        "",
        "",
    );
    assert!(
        drawn.text.contains("Real content."),
        "the unmarked content is untouched: {:?}",
        drawn.text
    );
    assert!(
        !drawn.text.contains("vii"),
        "the glyphs the artifact drew are replaced, not described: {:?}",
        drawn.text
    );
    let folio = drawn.artifacts.first().expect("the folio artifact");
    assert_eq!(
        folio.artifact.kind,
        Some(pdf_model::structure::ArtifactKind::Pagination)
    );
    assert_eq!(
        drawn.text.get(folio.range.clone()).map(str::trim),
        Some("7"),
        "the artifact's range covers the replacement rather than what it replaced: {:?}",
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

/// §14.8.2.5.3's own EXAMPLE, run as the clause writes it.
///
/// > /ReversedChars BMC ( olleH) Tj -200 0 Td ( .dlrow) Tj EMC
///
/// > represents the text
///
/// > Hello world.
///
/// Two show strings, each reversed on its own — "[i]f the sequence encompasses multiple show
/// strings, only the individual characters within each string shall be reversed" — so reversing
/// the pair together would give `world. Hello` and reversing the whole readback `.dlrow olleH`
/// backwards. The leading spaces are the file's own word breaks and end up between the words,
/// which is the clause's point: in such a block a break is stated rather than inferred.
#[test]
fn a_reversed_chars_sequence_is_read_back_forwards() {
    let drawn = interpret(
        "BT /F1 12 Tf 190 50 Td /ReversedChars BMC ( olleH) Tj -200 0 Td ( .dlrow) Tj EMC ET",
        "",
        "",
        "",
    );
    assert_eq!(drawn.text.trim(), "Hello world.");
}

/// A code §9.10.2 cannot name is counted the same inside the tag and outside it.
///
/// The pair is the point rather than either half. `Interpretation::codes_without_a_character`
/// asks what the *font* said about a code; the branch that decides it asked what the readback
/// *buffer* held until the four-hundred-and-seventy-sixth session — and inside §14.8.2.5.3's
/// reversal no code's text ever reaches that buffer while the string is being shown, because the
/// clause makes the whole string arrive backwards after it. So a buffer-reading rule answers
/// "nothing" for every code here, whether the font named it or not, and both of these would count
/// two.
///
/// `/F1` is the substituted Helvetica with no `/Encoding`, so Table 112's default base encoding
/// is `StandardEncoding` (§9.6.5.1) and code 1 is one the table leaves unencoded: no glyph name,
/// no `/ToUnicode`, no program name, and 0x01 outside the printable range §9.10.2's closing
/// permission is taken for. Code 0x41 is `A`. One of the two is nameable and one is not,
/// whichever order the clause puts them in.
#[test]
fn a_code_no_method_can_name_is_counted_through_a_reversal_too() {
    let plain = interpret("BT /F1 12 Tf 10 50 Td <0141> Tj ET", "", "", "");
    let reversed = interpret(
        "BT /F1 12 Tf 10 50 Td /ReversedChars BMC <0141> Tj EMC ET",
        "",
        "",
        "",
    );
    assert_eq!(
        (plain.text.trim(), plain.codes_without_a_character.total()),
        ("A", 1),
        "one of the two codes is named and one is not"
    );
    assert_eq!(
        (
            reversed.text.trim(),
            reversed.codes_without_a_character.total()
        ),
        ("A", 1),
        "and the reversal changes the order of the readback, not what the font said"
    );
}

/// Without the tag, the same stream reads back exactly as it was written.
///
/// The point of the pair: §14.8.2.5.3 is a *marked-content* rule, not a property of the glyphs
/// or of the writing direction, and a reader that reversed by looking at the geometry would
/// pass the test above and fail this one.
#[test]
fn the_same_strings_outside_the_tag_are_not_reversed() {
    let drawn = interpret(
        "BT /F1 12 Tf 190 50 Td ( olleH) Tj -200 0 Td ( .dlrow) Tj ET",
        "",
        "",
        "",
    );
    assert!(
        drawn.text.contains("olleH"),
        "read back as painted: {:?}",
        drawn.text
    );
}

/// §14.8.2.2's artifacts, in both of the forms §14.8.2.2.2 states.
///
/// > For artifacts defined using the marked-content sequence method, the form indicated in
/// > EXAMPLE 1 shall be used to identify a generic artifact; the form indicated in EXAMPLE 2
/// > shall be used for those artifacts that have an associated property list.
///
/// **That sentence is informative under Errata Collection 3** — Issue #484, `/State` `Review`
/// `Completed`, which splits the paragraph and makes this half a NOTE 2 with its two `shall`s
/// softened to "is". The blockquote is kept verbatim because it is what `doc/md/` carries and
/// what the conformance gate verifies against (ADR 0252); what changes is its force, and the
/// requirement this test actually rests on is the surviving normative half — Table 363 states
/// the property list's entries, and both forms remain the two the clause shows.
///
/// The first artifact here is a running head with Table 363's `/Type /Pagination`, a
/// `/Subtype /Header` and an `/Attached [/Top]`; the second is the generic `BMC` form. **Both
/// stay in the text**: the clause leaves what to do with an artifact to the consumer, so this
/// crate says which ranges are artifacts and removes none of them.
#[test]
fn an_artifact_is_recorded_over_its_own_range_and_left_in_the_text() {
    let drawn = interpret(
        "BT /F1 12 Tf 10 80 Td /Artifact << /Type /Pagination /Subtype /Header \
         /Attached [/Top] /BBox [0 90 200 100] >> BDC (Chapter One) Tj EMC \
         0 -30 Td (Real content.) Tj /Artifact BMC ( 7) Tj EMC ET",
        "",
        "",
        "",
    );
    assert_eq!(drawn.artifacts.len(), 2, "{:?}", drawn.artifacts);

    let head = drawn.artifacts.first().expect("the running head");
    assert_eq!(
        head.artifact.kind,
        Some(pdf_model::structure::ArtifactKind::Pagination)
    );
    assert_eq!(head.artifact.subtype.as_deref(), Some("Header"));
    assert_eq!(head.artifact.attached, [true, false, false, false]);
    assert_eq!(head.artifact.bbox, Some([0.0, 90.0, 200.0, 100.0]));
    assert_eq!(
        drawn.text.get(head.range.clone()).map(str::trim),
        Some("Chapter One"),
        "the range covers what the section drew: {:?}",
        drawn.text
    );

    let folio = drawn.artifacts.get(1).expect("the page number");
    assert_eq!(
        folio.artifact.kind, None,
        "the generic BMC form states no type"
    );
    assert_eq!(
        drawn.text.get(folio.range.clone()).map(str::trim),
        Some("7")
    );
    assert!(
        drawn.text.contains("Real content."),
        "nothing was removed: {:?}",
        drawn.text
    );
}

/// §14.13.5's `/AF` tag associates a file with the graphics objects a section encloses.
///
/// > One or more files may be associated with sections of content in a content stream by
/// > enclosing those sections between the marked-content operators BDC and EMC … with a
/// > marked-content tag of AF
///
/// The clause's own example is a `MathML` version of an equation associated with what draws it, so
/// the fixture is that shape: a run of text inside `/AF … BDC` whose property list names a file
/// specification with the relationship `Supplement`. The property list is a **named resource**
/// rather than an inline dictionary, which §14.6.2 requires of any list holding an indirect
/// reference — and an `/AF` array is nothing but indirect references. The range is over the readback, like an artifact's,
/// because what the file is associated with is *this* content and not the document.
#[test]
fn an_af_tagged_section_associates_a_file_with_what_it_draws() {
    let drawn = interpret(
        "BT /F1 12 Tf 10 50 Td (x squared) Tj ET \
         /AF /AFile BDC BT /F1 12 Tf 10 20 Td (equation) Tj ET EMC",
        "",
        "30 0 obj\n<< /AF [32 0 R] >>\nendobj\n\
         32 0 obj\n<< /Type /Filespec /F (equation.xml) /AFRelationship /Supplement \
         /EF << /F 31 0 R >> >>\nendobj\n\
         31 0 obj\n<< /Type /EmbeddedFile /Subtype /application#2Fmathml+xml /Length 7 >>\n\
         stream\n<math/>\nendstream\nendobj\n",
        "",
    );

    let [(range, file)] = drawn.associated_files.as_slice() else {
        panic!("one associated file, got {:?}", drawn.associated_files);
    };
    assert_eq!(file.name, "equation.xml");
    assert_eq!(
        file.relationship,
        pdf_model::attachment::Relationship::Supplement
    );
    assert_eq!(
        drawn.text.get(range.clone()).map(str::trim),
        Some("equation"),
        "the range covers the section's own content, not the page: {:?}",
        drawn.text
    );
}

/// The same section written the way Errata Collection 3 names the property list's key.
///
/// Issue #374 puts "a dictionary with an MCAF entry defining" in front of §14.13.5's sentence
/// about the property list, so a file conforming to the amended clause writes `/MCAF` where this
/// tree read `/AF` — and read nothing, silently, until the four-hundred-and-seventeenth session
/// (ADR 0253). The tag stays `AF`; only the key inside the named resource moves.
#[test]
fn an_af_tagged_section_reads_the_property_lists_mcaf_key() {
    let drawn = interpret(
        "BT /F1 12 Tf 10 50 Td (x squared) Tj ET \
         /AF /AFile BDC BT /F1 12 Tf 10 20 Td (equation) Tj ET EMC",
        "",
        "30 0 obj\n<< /MCAF [32 0 R] >>\nendobj\n\
         32 0 obj\n<< /Type /Filespec /F (equation.xml) /AFRelationship /Supplement \
         /EF << /F 31 0 R >> >>\nendobj\n\
         31 0 obj\n<< /Type /EmbeddedFile /Subtype /application#2Fmathml+xml /Length 7 >>\n\
         stream\n<math/>\nendstream\nendobj\n",
        "",
    );

    let [(_, file)] = drawn.associated_files.as_slice() else {
        panic!("one associated file, got {:?}", drawn.associated_files);
    };
    assert_eq!(file.name, "equation.xml");
}
