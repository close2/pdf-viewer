//! A refused font program does not take the document's displacements with it.
//!
//! ISO 32000-2 §9.2.4 states a glyph's width twice over — "[t]he width information for each glyph
//! shall be stored both in the font dictionary and in the font program itself" — and its NOTE 2
//! says what the second copy is for:
//!
//! > Storing this information in the font dictionary, although redundant, enables a PDF processor
//! > to determine glyph positioning without having to look inside the font program.
//!
//! So a reader that cannot draw a font's glyphs can still place them, and §9.4.4 still requires
//! it to: "[a]fter the glyph is painted, the text matrix shall be updated according to the glyph
//! displacement and any spacing parameters that apply." A refusal that skipped the update did not
//! stay inside the font it was about — it moved every glyph after it on the same line, including
//! the glyphs of a font this tree loads perfectly well. ADR 1094 is the argument and
//! `pdf_font::LoadedFont::metrics_only` the construction.
//!
//! # The two arms, and why both are needed
//!
//! The **witness** is a corpus document, because that is what shows the defect was real:
//! `issue6127.pdf` page 1 states `/C2_14 1 Tf 5.737 0 Td <0003>Tj /C2_2 1 Tf
//! [<0003>187<000b>…]TJ`, where `/C2_14` is an `/Identity-H` over a `CIDFontType2` with no
//! program — a combination §9.7.5.2 forbids outright, so the refusal is right — and `/C2_2` is a
//! font this tree loads.
//!
//! The **fixture** is hand-built, because the simple-font half of the same rule has no corpus
//! witness to be had: a simple font with no program is *substituted* (§9.7.4.2's route by
//! character), so the only way to a refusal is a program that is present and will not parse. It
//! carries its own control, which is trap 13's rule rather than a nicety — an arm that states no
//! width at all must put the second font back where the `Td` left it, or the test is measuring
//! something other than Table 109's array.

#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that stops opening should fail loudly"
)]

use std::fmt::Write as _;
use std::path::Path;

use pdf_model::{Pages, interpret};
use pdf_syntax::Document;

/// Where each of a page's readback characters begins, paired with the character.
fn placed(bytes: Vec<u8>, page: usize) -> Vec<(String, f32)> {
    let document = Document::open(bytes).expect("the document opens");
    let page = Pages::new(&document)
        .get(page)
        .expect("the document has that page");
    let interpretation = interpret(&document, &page);
    interpretation
        .text_layer
        .iter()
        .map(|placed| {
            let text = interpretation
                .text
                .get(placed.span.clone())
                .unwrap_or_default()
                .to_owned();
            (text, placed.quad[0])
        })
        .collect()
}

/// The witness's `(réf. S3182).`, which the round that diagnosed this measured in both references.
///
/// `pdftotext` puts the opening parenthesis at xMin 165.117 and `mutool` at 165.11672; this tree
/// put it at 162.101 while the displacement `/C2_14`'s descendant states for CID 3 — `/W` 250,
/// through `t x = (0.25 + T c 0.0013) × 12.0008` — is 3.0158 pt. Agreement with two readers is
/// evidence about this reading and never its definition (`CLAUDE.md` principle 5); what fixes the
/// number here is the file's own `/W`, and the references are why it is worth asserting to a
/// thousandth.
#[test]
fn the_font_after_a_refused_one_starts_where_the_file_says() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join("issue6127.pdf");
    let Ok(bytes) = std::fs::read(path) else {
        return; // the pdf.js submodule is not checked out
    };
    // Found by the word rather than by a coordinate, which would be the answer smuggled into
    // the question: the page states four other parentheses.
    let placed = placed(bytes, 0);
    let found = placed.windows(4).find(|run| {
        run.iter()
            .map(|(text, _)| text.as_str())
            .eq(["(", "r", "é", "f"])
    });
    let x = found.expect("page one shows (réf.")[0].1;
    assert!(
        (x - 165.1167).abs() < 0.001,
        "the TJ after the refused Tj begins at the displacement /W states, not 3.0158 pt short: \
         {x}"
    );
}

/// A one-page fixture: a refused `TrueType`, then `/Helvetica`, on one line.
///
/// `/F1`'s `/FontFile2` is not a font program, so `pdf-font` refuses it and the page reports the
/// refusal; what `widths` says is the only thing that differs between the arms below. `(A)` is
/// shown through it and `(H)` through `/F2`, which loads, so the assertion is about where the
/// *second* font's glyph lands — the quantity a refusal must not be able to move.
///
/// No cross-reference table: `Document::open` rebuilds by scanning for object headers, which keeps
/// hand-written offsets out of a file whose object lengths change with every arm.
fn document(widths: &str, missing_width: &str) -> Vec<u8> {
    let content = "BT /F1 10 Tf 20 50 Td (A) Tj /F2 10 Tf (H) Tj ET\n";
    // Bytes that are not an sfnt, a bare CFF or a Type 1 program, so every reader `pdf-font`
    // offers declines them and the refusal is §9.9's rather than §9.8's.
    let program = "not a font";
    let mut out = String::from("%PDF-1.7\n");
    let _ = write!(
        out,
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Contents 4 0 R \
         /Resources << /Font << /F1 5 0 R /F2 8 0 R >> >> >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}endstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /TrueType /BaseFont /NotAFont \
         /FirstChar 65 /LastChar 65 {widths} /FontDescriptor 6 0 R >>\nendobj\n\
         6 0 obj\n<< /Type /FontDescriptor /FontName /NotAFont /Flags 4 \
         /ItalicAngle 0 /StemV 80 /FontBBox [0 0 1000 1000] {missing_width} \
         /FontFile2 7 0 R >>\nendobj\n\
         7 0 obj\n<< /Length {} >>\nstream\n{program}\nendstream\nendobj\n\
         8 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n\
         trailer\n<< /Root 1 0 R /Size 9 >>\n%%EOF\n",
        content.len(),
        program.len(),
    );
    out.into_bytes()
}

/// Where `/F2`'s `H` begins, for one arm of the fixture.
fn helvetica_starts_at(widths: &str, missing_width: &str) -> f32 {
    let placed = placed(document(widths, missing_width), 0);
    let found = placed.iter().find(|(text, _)| text == "H");
    let (_, x) = found.expect("the page reads back the H that /Helvetica draws");
    *x
}

/// Table 109's `/Widths` places the next font's glyph, though nothing drew the first one.
///
/// §9.6.2.1's table states the array and what indexes it:
///
/// > An array of ( LastChar - FirstChar + 1) numbers, each element being the glyph width for the
/// > character code that equals FirstChar plus the array index.
///
/// 500 thousandths of an em at `T fs` 10 is 5 points, so `H` begins at 25 — and §9.4.4's `t x` is
/// that whole answer here, since `T c`, `T w` and any `T j` are all absent and `T h` is 1.
#[test]
fn a_refused_program_still_advances_by_the_stated_width() {
    let x = helvetica_starts_at("/Widths [500]", "");
    assert!((x - 25.0).abs() < 0.001, "20 + 500/1000 × 10 = 25, not {x}");
}

/// Table 120's `/MissingWidth` answers for a code `/Widths` does not reach.
///
/// §9.8.1's table states it:
///
/// > The width to use for character codes whose widths are not specified in a font dictionary's
/// > Widths array.
///
/// The same arithmetic one entry further out: 800 thousandths at size 10 is 8 points.
#[test]
fn a_refused_program_falls_back_to_the_stated_missing_width() {
    let x = helvetica_starts_at("", "/MissingWidth 800");
    assert!((x - 28.0).abs() < 0.001, "20 + 800/1000 × 10 = 28, not {x}");
}

/// The control: a font stating no width moves nothing, which is what the defect looked like.
///
/// Trap 13 — an instrument that cannot fail says nothing when it passes. Table 120 gives
/// `/MissingWidth` a "[d]efault value: 0", so a dictionary that states neither entry displaces the
/// pen by nothing and `/Helvetica` starts at the `Td`'s own 20. That is exactly the position every
/// arm above had before the metrics were kept, so an implementation that ignored `/Widths` and one
/// that had no metrics at all would be told apart by these three numbers.
#[test]
fn a_refused_program_stating_no_width_advances_by_nothing() {
    let x = helvetica_starts_at("", "");
    assert!(
        (x - 20.0).abs() < 0.001,
        "Table 120's default width is 0, so the pen has not moved: {x}"
    );
}

/// And the page still says the font was refused, whatever it now places.
///
/// The metrics are not a recovery and must not read as one (trap 5): nothing of `/F1`'s was
/// drawn, so the page is incomplete and names the font.
#[test]
fn keeping_the_metrics_does_not_quiet_the_refusal() {
    let document = Document::open(document("/Widths [500]", "")).expect("the fixture opens");
    let page = Pages::new(&document)
        .get(0)
        .expect("the fixture has a page");
    let interpretation = interpret(&document, &page);
    let said = format!("{:?}", interpretation.unsupported);
    assert!(!interpretation.is_complete(), "the page is incomplete");
    assert!(
        said.contains("font /F1 could not be parsed"),
        "and the refusal names the font: {said}"
    );
}
