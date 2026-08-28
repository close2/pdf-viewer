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
//! fourth shape: no document on this disk carries a damaged tiling pattern or appearance stream,
//! so a corpus cannot show those either way.
//!
//! **This sentence used to name the Type 3 glyph description among them and to say the form
//! `XObject` was the one kind a corpus could show, and both halves were wrong** (ADR 0744). The
//! form it meant is `comments.pdf`'s, which turned out to be a stream its producer *flushed* and
//! never finished — nothing missing, and the report was the defect. Asking the question again
//! found the opposite witness one page away: `poppler-90-0-fuzzed.pdf` names a glyph description
//! that really does stop short. The last two tests here are those two documents, and between
//! them they are what a corpus can say about this rule.
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
/// The damage of a stream held whole is read where `crate::annotation` resolves it, which is why
/// the report names the subtype; the two tests after this one pin where the *other* shapes of the
/// answer come from (ADR 0723).
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

/// Content large enough that the decoded-stream memo declines it, so the stream is pumped
/// through a window rather than held whole (`DECODED_BUDGET` is 4 MiB and [`run_length`]'s
/// literal chunks encode at about one byte per byte).
///
/// The padding is comments so that the fixture's cost is lexing rather than drawing: what makes
/// the stream windowed is its decoded length, and §7.2.4 makes a comment cost nothing else.
fn windowed_content() -> String {
    let mut content = String::from("1 0 0 rg 0 0 20 20 re f\n");
    // Ten bytes a line, so this is five mebibytes and a quarter over the four MiB budget.
    content.push_str(&"% eight b\n".repeat(512 * 1024));
    content.push_str("0 30 20 20 re f\n");
    content
}

/// [`page_with_appearance`], with a stream the memo declines: `content` is handed in so the
/// caller can state the expected `kept` from the same bytes.
fn page_with_windowed_appearance(content: &str, whole: bool) -> Vec<u8> {
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
            content,
            whole,
        ),
    ])
}

/// The windowed shape of the same rule: the run meets the damage and says it, once.
///
/// A stream the decoded-stream memo declines is pumped through a window each time it is read
/// (ADR 0427), so its damage is not known where the annotation is decided — and may not be asked
/// for there, because the answer costs the whole decode and the draw is about to produce it
/// anyway (ADR 0723; the pre-pass this replaces also said the same damage twice, in two
/// spellings). What pins this test is both halves: the report exists, and it exists **once**,
/// in the run's own words.
#[test]
fn a_windowed_appearance_streams_damage_is_reported_once_by_the_run() {
    let content = windowed_content();
    assert_eq!(
        damage_reports(page_with_windowed_appearance(&content, true)),
        Vec::new(),
        "an undamaged windowed appearance says nothing"
    );
    assert_eq!(
        damage_reports(page_with_windowed_appearance(&content, false)),
        vec![(
            "an annotation's appearance stream (§12.5.5)".to_owned(),
            Damage::Truncated,
            content.len()
        )],
        "the damage is met mid-run and reported there, exactly once"
    );
    assert!(
        command_count(page_with_windowed_appearance(&content, false)) > 0,
        "and the prefix's marks are on the page"
    );
}

/// A widget whose damaged, windowed appearance is *regenerated*, which is the one route that
/// still owes the full answer before the run.
fn page_with_regenerated_appearance(content: &str, whole: bool) -> Vec<u8> {
    document_of(&[
        dict_object(
            "<< /Type /Catalog /Pages 2 0 R \
             /AcroForm << /NeedAppearances true /Fields [5 0 R] >> >>",
        ),
        dict_object("<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        dict_object(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
             /Resources << >> /Annots [5 0 R] >>",
        ),
        plain("0 0 1 1 re f\n"),
        dict_object(
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (t1) /V (hello) \
             /Rect [10 10 90 40] /F 4 /DA (0 g /Helv 12 Tf) /AP << /N 6 0 R >> >>",
        ),
        run_length(
            "/Type /XObject /Subtype /Form /BBox [0 0 80 30]",
            content,
            whole,
        ),
    ])
}

/// §12.7.4.3's regeneration replaces the stored stream's content with a spliced copy, so the
/// run never reads the stored stream and the damage has to be known where the annotation is
/// decided — the reason ADR 0359 put the answer there, kept for exactly this route (ADR 0723).
///
/// The stream is windowed on purpose: for one held whole the stated answer and the read answer
/// are the same thing, and only the windowed shape can tell "asked before the run" from "met
/// during it".
#[test]
fn a_regenerated_widgets_stored_stream_still_reports_its_damage() {
    let content = windowed_content();
    assert_eq!(
        damage_reports(page_with_regenerated_appearance(&content, true)),
        Vec::new(),
        "an undamaged stored stream says nothing"
    );
    assert_eq!(
        damage_reports(page_with_regenerated_appearance(&content, false)),
        vec![(
            "a Widget annotation's appearance stream (§12.5.5)".to_owned(),
            Damage::Truncated,
            content.len()
        )],
        "the stored stream the splice replaced is still named, from the decision"
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

/// The document this test was written about, and what it turned out to be.
///
/// **`comments.pdf` was the corpus witness for the rule above and is now the witness against it.**
/// Its page one draws an ink annotation whose appearance invokes a form `XObject` whose flate
/// stream ends after 851 bytes with no RFC 1951 final block, and this test asserted the report —
/// while its own comment said the green loop was "short of where the producer's stylus went",
/// which was a diagnosis read off the report rather than off the bytes. The stream ends on a
/// *completed* block: its producer flushed and never called `deflateEnd`, so every byte it was
/// given is here and what is absent is the declaration that there is no more. Decidably so —
/// `pdf_syntax`'s inflate hands the decoder RFC 1951's final empty stored block and requires
/// `StreamEnd` with no further output (ADR 0744).
///
/// So the page says nothing, which is what a page missing no marks should say. Trap 11 from the
/// other side: the report exists to keep a page cut short from looking sparse, and this page was
/// never cut short.
#[test]
fn the_corpus_witness_turns_out_to_be_a_flush_and_says_nothing() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs/comments.pdf");
    let Ok(bytes) = std::fs::read(&path) else {
        println!("the pdf.js submodule is not checked out; skipping");
        return;
    };
    assert_eq!(
        damage_reports(bytes),
        Vec::new(),
        "a flush is not a truncation and the form is whole"
    );
}

/// The corpus witness the rule *does* have, and it is not the kind this file said it was.
///
/// **This file's own header said a corpus could not show a damaged Type 3 glyph description**,
/// and one has been on this disk the whole time: `poppler-90-0-fuzzed.pdf` page 10 reaches a
/// `/a14` whose `CharProcs` stream stops before RFC 1951's final block, and the page names it.
/// The claim was written when the only corpus damage anybody had looked at was
/// `comments.pdf`'s form, and nothing asked the question again when that turned out to be a
/// flush (ADR 0744). §9.6.4 makes a glyph description one of §7.8.2's content streams by name,
/// so the rule ADR 0359 states for the five kinds has a real document behind one more of them.
///
/// **The assertion names the stream rather than pinning the whole list.** The document is a
/// fuzzer's output carrying dozens of damaged streams; what is owed here is that the object the
/// clause calls a content stream is named when it stops short, not which bytes a fuzzer produced.
#[test]
fn a_corpus_document_that_really_cuts_a_glyph_description_short_names_it() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs/poppler-90-0-fuzzed.pdf");
    let Ok(bytes) = std::fs::read(&path) else {
        println!("the pdf.js submodule is not checked out; skipping");
        return;
    };
    let document = Document::open(bytes).expect("the corpus document opens");
    let page = Pages::new(&document)
        .get(9)
        .expect("the document has a tenth page");
    let reports: Vec<(String, Damage, usize)> = interpret(&document, &page)
        .unsupported
        .into_iter()
        .filter_map(|item| match item {
            Unsupported::DamagedContentStream { stream } => {
                Some((stream.detail, stream.damage, stream.kept))
            }
            _ => None,
        })
        .collect();
    assert!(
        reports.iter().any(
            |(detail, damage, _)| detail.starts_with("a Type 3 glyph description")
                && *damage == Damage::Truncated
        ),
        "no report names the glyph description this document cuts short: {reports:?}"
    );
}
