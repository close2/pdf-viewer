//! §7.8.2's *other* content streams, damaged: drawn to where the damage is, and said out loud.
//!
//! ISO 32000-2 §7.8.2 defines what a content stream is —
//!
//! > A content stream is a PDF stream object whose data consists of a sequence of instructions
//! > describing the graphical elements to be painted on a page.
//!
//! — and then names the objects that are one without being a page's `/Contents`:
//!
//! > Content streams shall also be used to package sequences of instructions as self-contained
//! > graphical elements, such as forms (see 8.10, "Form XObjects"), patterns (8.7, "Patterns"),
//! > certain fonts (9.6.4, "Type 3 fonts"), and annotation appearances (12.5.5, "Appearance
//! > streams").
//!
//! ADR 0343 settled the rule for the first kind: a prefix of a sequence of instructions is a
//! shorter sequence of the same kind, made of bytes the producer's own encoder emitted, so it is
//! drawn — and §7.4.1's other half, that this is not "the original form", is the report. ADR 0359
//! carries that to the four the clause names beside it. `tests/contents_entry.rs` is the same rule
//! for Table 31's entry; this file is the rest of the sentence.
//!
//! **Each rule is pinned by a pair of fixtures differing only in the damage**, which is trap 8's
//! fourth shape: no document on this disk carries a damaged tiling pattern, Type 3 glyph
//! description or appearance stream, so a corpus cannot show the rule either way. What a corpus
//! *can* show is the form `XObject`, and the last test here opens `comments.pdf`.
//!
//! `RunLengthDecode` is the filter throughout for the reason `contents_entry.rs` gives: §7.4.5
//! makes a length byte of 128 its end-of-data, so a stream that ends without one is truncated in
//! the clause's own words and can be written into a text fixture a byte at a time.

#![expect(
    clippy::expect_used,
    reason = "test code: a malformed fixture should fail loudly"
)]

use std::fmt::Write as _;
use std::path::Path;

use pdf_model::{Pages, Unsupported, interpret};
use pdf_syntax::{Damage, Document};

/// Assembles a one-page document out of the object bodies given, with a correct cross-reference
/// table.
///
/// Objects are numbered from 1 in the order handed in, which every fixture below relies on: 1 is
/// the catalog, 2 the page tree, 3 the page, 4 the page's `/Contents`.
///
/// **Bytes rather than text throughout**, because [`run_length`]'s end-of-data marker is 128 and
/// a `String` would carry it as two bytes of UTF-8 — which makes `/Length` a lie and turns the
/// marker into a repeat count. That is a fixture bug that looks exactly like the rule failing.
fn document_of(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::from(*b"%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index.saturating_add(1)).as_bytes());
        out.extend_from_slice(object);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let mut tail = String::new();
    let _ = writeln!(tail, "xref\n0 {size}");
    tail.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(tail, "{offset:010} 00000 n ");
    }
    let _ = write!(
        tail,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.extend_from_slice(tail.as_bytes());
    out
}

/// A dictionary object, written as it stands.
fn dict_object(dict: &str) -> Vec<u8> {
    dict.as_bytes().to_vec()
}

/// An uncompressed content stream, with the `/Length` §7.3.8.2 requires to be right.
fn plain(content: &str) -> Vec<u8> {
    format!(
        "<< /Length {} >>\nstream\n{content}endstream",
        content.len()
    )
    .into_bytes()
}

/// A stream object holding `content` under `RunLengthDecode`, whole or cut off before its EOD.
///
/// A whole one ends with the length byte 128, which §7.4.5 makes the end-of-data — "[a] length
/// value of 128 shall denote EOD" — and a damaged one simply stops. The two differ in one byte,
/// which is what makes each pair below a statement about the damage and nothing else.
fn run_length(dict: &str, content: &str, whole: bool) -> Vec<u8> {
    let mut data: Vec<u8> = Vec::new();
    for chunk in content.as_bytes().chunks(128) {
        data.push(u8::try_from(chunk.len().saturating_sub(1)).unwrap_or(127));
        data.extend_from_slice(chunk);
    }
    if whole {
        data.push(128);
    }
    let mut out = format!(
        "<< {dict} /Filter /RunLengthDecode /Length {} >>\nstream\n",
        data.len()
    )
    .into_bytes();
    out.extend_from_slice(&data);
    out.extend_from_slice(b"endstream");
    out
}

/// Every damage report a page makes, in the order the interpreter deduplicated them into.
fn damage_reports(bytes: Vec<u8>) -> Vec<(String, Damage, usize)> {
    let document = Document::open(bytes).expect("the fixture opens");
    let page = Pages::new(&document)
        .get(0)
        .expect("the fixture has a page");
    interpret(&document, &page)
        .unsupported
        .into_iter()
        .filter_map(|item| match item {
            Unsupported::DamagedContentStream { stream } => {
                Some((stream.detail, stream.damage, stream.kept))
            }
            _ => None,
        })
        .collect()
}

/// How many commands a fixture's page draws, which is what says the prefix reached the page.
fn command_count(bytes: Vec<u8>) -> usize {
    let document = Document::open(bytes).expect("the fixture opens");
    let page = Pages::new(&document)
        .get(0)
        .expect("the fixture has a page");
    interpret(&document, &page).display_list.commands().len()
}

/// A page whose `Do` names a form `XObject` whose stream is `whole` or truncated.
fn page_with_form(whole: bool) -> Vec<u8> {
    document_of(&[
        dict_object("<< /Type /Catalog /Pages 2 0 R >>"),
        dict_object("<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        dict_object(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
             /Resources << /XObject << /Fm0 5 0 R >> >> >>",
        ),
        plain("/Fm0 Do\n"),
        run_length(
            "/Type /XObject /Subtype /Form /BBox [0 0 100 100]",
            "0 0 10 10 re f\n0 20 10 10 re f\n",
            whole,
        ),
    ])
}

/// §8.10's form: two rectangles arrive, and the report says the stream stopped short.
///
/// §8.10.1 makes a form "a self-contained description of any sequence of graphics objects", which
/// is §7.8.2's content stream under another name — so the marks the prefix carries are in the
/// producer's own places and the ones the damage took are simply absent. Nothing stands in place
/// of anything, which is trap 5's additive test and the reason this is drawn rather than refused.
#[test]
fn a_damaged_form_xobject_draws_its_prefix_and_reports_the_shortfall() {
    let whole = damage_reports(page_with_form(true));
    assert_eq!(whole, Vec::new(), "an undamaged form says nothing");

    let reports = damage_reports(page_with_form(false));
    assert_eq!(
        reports,
        vec![(
            "a form XObject /Fm0 (§8.10)".to_owned(),
            Damage::Truncated,
            31
        )],
        "the form is named, with why it stopped and what survived"
    );
    assert_eq!(
        command_count(page_with_form(false)),
        command_count(page_with_form(true)),
        "and both rectangles are on the page: this prefix ends at a complete operator"
    );
}

/// A page filling a rectangle with a tiling pattern whose cell stream is `whole` or truncated.
fn page_with_tiling_pattern(whole: bool) -> Vec<u8> {
    document_of(&[
        dict_object("<< /Type /Catalog /Pages 2 0 R >>"),
        dict_object("<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        dict_object(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
             /Resources << /Pattern << /P0 5 0 R >> >> >>",
        ),
        plain("/Pattern cs /P0 scn 0 0 100 100 re f\n"),
        run_length(
            "/PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 10 10] /XStep 10 /YStep 10 \
             /Resources << >>",
            "0 0 0 rg 0 0 4 4 re f\n",
            whole,
        ),
    ])
}

/// §8.7.3.1's pattern cell, which is a content stream by that clause's own sentence.
///
/// > The appearance of the pattern cell shall be defined by a content stream containing the
/// > painting operators needed to paint one instance of the cell.
///
/// A prefix of it is a cell with fewer marks in the same places, replicated at the file's own
/// `/XStep` and `/YStep` — so the damage is amplified across the filled area without ever
/// becoming substitutive, which is why the answer is the same as the form's.
#[test]
fn a_damaged_tiling_pattern_draws_its_prefix_and_reports_the_shortfall() {
    assert_eq!(
        damage_reports(page_with_tiling_pattern(true)),
        Vec::new(),
        "an undamaged cell says nothing"
    );
    assert_eq!(
        damage_reports(page_with_tiling_pattern(false)),
        vec![(
            "a tiling pattern /P0 (§8.7.3.1)".to_owned(),
            Damage::Truncated,
            22
        )],
        "the pattern is named by the name `scn` used"
    );
}

/// A page showing one code of a Type 3 font whose glyph description is `whole` or truncated.
fn page_with_type3_glyph(whole: bool) -> Vec<u8> {
    document_of(&[
        dict_object("<< /Type /Catalog /Pages 2 0 R >>"),
        dict_object("<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        dict_object(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
             /Resources << /Font << /T3 5 0 R >> >> >>",
        ),
        plain("BT /T3 12 Tf 10 10 Td (A) Tj ET\n"),
        dict_object(
            "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1000 1000] \
             /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs << /square 6 0 R >> \
             /Encoding << /Type /Encoding /Differences [65 /square] >> \
             /FirstChar 65 /LastChar 65 /Widths [1000] >>",
        ),
        run_length(
            "",
            "1000 0 0 0 1000 1000 d1\n0 0 500 500 re f\n0 600 500 400 re f\n",
            whole,
        ),
    ])
}

/// §9.6.4's glyph description, where Table 110 makes the prefix rule *stronger* than elsewhere.
///
/// > The stream shall include as its first operator either d0 or d1 , followed by operators
/// > describing one or more graphics objects.
///
/// So any prefix that carries a mark carries the glyph's own declaration ahead of it, and Table
/// 110's `/Widths` — not the description — supplies the advance. What the damage costs is marks
/// inside this glyph and never the position of the next one, which is what separates it from
/// ADR 0343's refusal of a damaged *font program*: there the prefix produced other glyphs
/// entirely.
#[test]
fn a_damaged_type3_glyph_description_draws_its_prefix_and_reports_the_shortfall() {
    assert_eq!(
        damage_reports(page_with_type3_glyph(true)),
        Vec::new(),
        "an undamaged description says nothing"
    );
    assert_eq!(
        damage_reports(page_with_type3_glyph(false)),
        vec![(
            "a Type 3 glyph description /square (§9.6.4)".to_owned(),
            Damage::Truncated,
            60
        )],
        "named by the glyph name §9.6.4 step a) produced, since /CharProcs is keyed by it"
    );
    assert!(
        command_count(page_with_type3_glyph(false)) > 0,
        "and the description's marks are on the page"
    );
}

/// A page with one square annotation whose stored appearance stream is `whole` or truncated.
fn page_with_appearance(whole: bool) -> Vec<u8> {
    document_of(&[
        dict_object("<< /Type /Catalog /Pages 2 0 R >>"),
        dict_object("<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        dict_object(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
             /Resources << >> /Annots [5 0 R] >>",
        ),
        plain("0 0 1 1 re f\n"),
        dict_object(
            "<< /Type /Annot /Subtype /Square /Rect [10 10 60 60] /F 4 /AP << /N 6 0 R >> >>",
        ),
        run_length(
            "/Type /XObject /Subtype /Form /BBox [0 0 50 50]",
            "1 0 0 rg 0 0 20 20 re f\n0 30 20 20 re f\n",
            whole,
        ),
    ])
}

/// §12.5.5's appearance stream, which the clause makes a form `XObject` outright.
///
/// > Each appearance stream is a form XObject (see 8.10, "Form XObjects"): a self-contained
/// > content stream that shall be rendered inside the annotation rectangle.
///
/// The damage is read where `crate::annotation` resolves the stream rather than where the
/// appearance is finally run, because §12.7.4.3's regeneration replaces a widget's content with a
/// *spliced copy* of these bytes — so a report taken at the draw would go quiet for exactly the
/// fields whose text a reader has changed.
#[test]
fn a_damaged_appearance_stream_draws_its_prefix_and_reports_the_shortfall() {
    assert_eq!(
        damage_reports(page_with_appearance(true)),
        Vec::new(),
        "an undamaged appearance says nothing"
    );
    assert_eq!(
        damage_reports(page_with_appearance(false)),
        vec![(
            "a Square annotation's appearance stream (§12.5.5)".to_owned(),
            Damage::Truncated,
            40
        )],
        "named by the subtype, which is what a reader has to go on"
    );
    assert!(
        command_count(page_with_appearance(false)) > 0,
        "and what the stream did carry is drawn over the page"
    );
}

/// A page painting under a luminosity soft mask whose group stream is `whole` or truncated.
fn page_with_soft_mask(whole: bool) -> Vec<u8> {
    document_of(&[
        dict_object("<< /Type /Catalog /Pages 2 0 R >>"),
        dict_object("<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        dict_object(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
             /Resources << /ExtGState << /GS0 5 0 R >> >> >>",
        ),
        plain("/GS0 gs 0 0 100 100 re f\n"),
        dict_object("<< /Type /ExtGState /SMask << /Type /Mask /S /Luminosity /G 6 0 R >> >>"),
        run_length(
            "/Type /XObject /Subtype /Form /BBox [0 0 100 100] \
             /Group << /S /Transparency /CS /DeviceGray >>",
            "1 1 1 rg 0 0 50 50 re f\n0 60 50 40 re f\n",
            whole,
        ),
    ])
}

/// §11.6.5.1's `/G`, and the one of the five where the answer needed checking rather than
/// carrying over.
///
/// §11.6.5.1 makes the group a transparency group `XObject` designated by `/G`, which §11.6.6
/// makes a form — but its marks become *mask values* over other objects, which is the shape ADR
/// 0356 refused for a sampled function. What decides it is that this clause states the mask's
/// value where the group painted nothing: the transfer function of 0.0 for `Alpha`, `/BC`'s
/// luminosity for `Luminosity`. A place the damage took is a place the group did not paint, and
/// the clause already answers for one of those — so these are places, not values, and the prefix
/// is drawn.
#[test]
fn a_damaged_soft_mask_group_draws_its_prefix_and_reports_the_shortfall() {
    assert_eq!(
        damage_reports(page_with_soft_mask(true)),
        Vec::new(),
        "an undamaged group says nothing"
    );
    assert_eq!(
        damage_reports(page_with_soft_mask(false)),
        vec![(
            "a soft mask's transparency group /G (§11.6.5.1)".to_owned(),
            Damage::Truncated,
            40
        )],
        "the group is named by the entry §11.6.5.1 puts it under"
    );
}

/// The corpus witness, which is the one of the five kinds a real document carries.
///
/// `comments.pdf`'s page one draws an ink annotation whose appearance invokes a form `XObject`
/// whose flate stream ends after 851 bytes — mid-way through the ink path, after a completed `S`.
/// The green loop it draws is short of where the producer's stylus went, and until ADR 0359 the
/// page said nothing about it: exactly trap 5's "a page cut short looks like a page meant to be
/// sparse". 7 of the corpus's 57 damaged streams are form `XObject`s and 46 of the crawl's 2260
/// are (`examples/damaged_stream_census`).
#[test]
fn the_corpus_witness_names_its_truncated_form() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs/comments.pdf");
    let Ok(bytes) = std::fs::read(&path) else {
        println!("the pdf.js submodule is not checked out; skipping");
        return;
    };
    assert_eq!(
        damage_reports(bytes),
        vec![(
            "a form XObject /Form (§8.10)".to_owned(),
            Damage::Truncated,
            851
        )],
        "the ink annotation's truncated form is named"
    );
}
