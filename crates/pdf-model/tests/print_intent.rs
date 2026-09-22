//! ISO 32000-2 §12.5.3's Table 167 and §12.5.6.22, asked of a page that is going onto paper.
//!
//! Table 167 states two answers about the same annotation and the output decides which: bit 6 is
//! "do not render the annotation on the screen" and bit 3 is "print the annotation when the page
//! is printed", so a reader that never states which device it is drawing for has only ever
//! applied half the table. `ViewState::set_purpose` is where the operation says so, and this file
//! is the table's own truth values read back off the page.
//!
//! **Every expected value below comes from one of Table 167's cells, quoted in the test that
//! rests on it.** Nothing here is derived from what another renderer does with these flags.
//!
//! The fixtures are hand-built and trap 8 is why they say so: bit 3 is the most-stated flag in
//! both corpora — `pdf-model/examples/spec_annotation_census` counts it — but a corpus cannot
//! hold the *pair* of documents that differ in one bit and in nothing else, which is the only
//! shape that discriminates between reading the bit and ignoring it. Each assertion below has
//! its control beside it for that reason.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    reason = "test code: a malformed fixture should fail loudly, and the fixtures are small \
              enough that no index can overflow"
)]

use std::fmt::Write as _;

use pdf_model::optional_content::Purpose;
use pdf_model::view::ViewState;
use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Pixel budget, far above the 100×100 pages these tests build.
const GENEROUS: u64 = 1 << 30;

/// Wraps a body of objects in a header, a cross-reference table and a trailer.
fn assemble(body: &str) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// A one-page PDF carrying one square annotation with the stated `/F` and appearance entries.
///
/// `appearance` is the annotation's own `/AP` text, written out so that a fixture can state one
/// appearance stream, a subdictionary of them, or none at all — which is the condition bit 3's
/// third sentence turns on.
fn page_with(flags: &str, appearance: &str) -> Vec<u8> {
    let stream = "1 0 0 rg 0 0 10 10 re f";
    assemble(&format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Square /Rect [20 20 80 80] {flags} \
         /IC [0 0 1] /C [0 0 0] {appearance} >>\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] /Length {} >>\n\
         stream\n{stream}\nendstream\nendobj\n",
        stream.len().saturating_add(1)
    ))
}

/// A one-page watermark fixture: a media box, a `/Rect` and a fixed print dictionary.
fn watermark(media_box: &str, rect: &str, fixed_print: &str) -> Vec<u8> {
    let stream = "0 0 0 rg 0 0 10 10 re f";
    assemble(&format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox {media_box} \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Watermark /Rect {rect} /F 4 \
         /AP << /N 6 0 R >> {fixed_print} >>\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] /Length {} >>\n\
         stream\n{stream}\nendstream\nendobj\n",
        stream.len().saturating_add(1)
    ))
}

/// What one interpretation of a fixture came to: whether anything was drawn, and what was owed.
struct Drawn {
    marks: bool,
    unsupported: Vec<String>,
}

/// Interprets a fixture with the output's purpose and, for print, its sheet, stated by the caller.
///
/// Both go in through `ViewState`, which `CLAUDE.md`'s rule 1 makes the one channel by which
/// anything outside the file may decide a mark — so these tests take the path a print job takes
/// rather than a back door built for them.
fn drawn(bytes: Vec<u8>, purpose: Purpose, paper: Option<[f32; 4]>) -> Drawn {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let mut state = ViewState::of(&document);
    state.set_purpose(purpose);
    state.set_paper(paper);
    let interpretation = pdf_model::content::interpret_with(&document, &page, &state);
    Drawn {
        marks: !interpretation.display_list.commands().is_empty(),
        unsupported: interpretation
            .unsupported
            .iter()
            .map(|owed| format!("{owed:?}"))
            .collect(),
    }
}

/// The smallest box containing every painted pixel, in PDF coordinates.
fn extent(bytes: Vec<u8>, purpose: Purpose, paper: Option<[f32; 4]>) -> (u32, u32, u32, u32) {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let mut state = ViewState::of(&document);
    state.set_purpose(purpose);
    state.set_paper(paper);
    let interpretation = pdf_model::content::interpret_with(&document, &page, &state);
    assert!(
        interpretation.unsupported.is_empty(),
        "the fixture should draw completely: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported");
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (u32::MAX, u32::MAX, 0, 0);
    for y in 0..raster.height {
        for x in 0..raster.width {
            let row = raster.height.saturating_sub(1).saturating_sub(y);
            let index = ((row * raster.width + x) as usize) * 4;
            if raster.data[index + 3] > 0 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    (min_x, min_y, max_x, max_y)
}

/// Table 167 bits 2, 3 and 6, on an annotation that **has** an appearance stream.
///
/// Bit 3's row, in §12.5.3, is the whole of the printed page's rule:
///
/// > If set, print the annotation when the page is printed unless the Hidden flag is also set.
/// > If clear, never print the annotation, regardless of whether it is rendered on the screen.
///
/// Bit 6's row states its own consequence for that rule: "[t]he annotation may be printed
/// (depending on the setting of the Print flag) but should be considered hidden for purposes of
/// on-screen display and user interaction". And bit 2's is unconditional — "do not render the
/// annotation or allow it to interact with the user, regardless of its annotation type or whether
/// an annotation handler is available".
///
/// The screen column is asserted beside the paper column on every row, because that pair is what
/// discriminates: a reader that had simply stopped consulting the flags would pass the paper
/// column alone, and one that had left the screen's reading in place would pass every row but
/// `/F 36`.
#[test]
fn table_167s_print_bit_decides_the_printed_page_and_the_view_bits_decide_the_screen() {
    // `/F`, then what the screen shows, then what the paper takes.
    let rows: [(&str, bool, bool); 7] = [
        // No `/F` at all: every flag is 0, so nothing hides it on screen and bit 3's second
        // sentence — "[i]f clear, never print the annotation" — keeps it off the paper.
        ("", true, false),
        // Print alone.
        ("/F 4", true, true),
        // Hidden alone.
        ("/F 2", false, false),
        // Hidden and Print: "unless the Hidden flag is also set".
        ("/F 6", false, false),
        // NoView alone: off the screen by its own row, off the paper by bit 3's second sentence.
        ("/F 32", false, false),
        // NoView and Print — the cell the whole switch is for. Off the screen, onto the paper.
        ("/F 36", false, true),
        // NoView and Hidden.
        ("/F 34", false, false),
    ];
    for (flags, on_screen, on_paper) in rows {
        let screen = drawn(page_with(flags, "/AP << /N 6 0 R >>"), Purpose::View, None);
        assert_eq!(
            screen.marks, on_screen,
            "`{flags}` on the screen: {:?}",
            screen.unsupported
        );
        let paper = drawn(page_with(flags, "/AP << /N 6 0 R >>"), Purpose::Print, None);
        assert_eq!(
            paper.marks, on_paper,
            "`{flags}` on paper: {:?}",
            paper.unsupported
        );
        // Table 167 is what the document says, not what this reader could not manage: an
        // annotation a flag keeps off one device is never a shortfall on either.
        assert!(
            screen.unsupported.is_empty() && paper.unsupported.is_empty(),
            "`{flags}` is an instruction, not a gap"
        );
    }
}

/// Bit 3's third sentence: with no appearance streams, the flag decides nothing.
///
/// §12.5.3, Table 167:
///
/// > If the annotation does not contain any appearance streams this flag shall be ignored.
///
/// This is the sentence that decides the population rather than an edge case. An annotation
/// stating no `/F` has bit 3 clear, and the second sentence alone would take every constructed
/// appearance off every printed page — so the three fixtures below are the three ways an
/// annotation can fail to contain one: no `/AP`, an `/AP` that holds nothing, and an `/AP`
/// whose `/N` is neither a stream nor a subdictionary of them.
///
/// Its control is the row above with `/AP << /N 6 0 R >>` and no `/F`, which is not printed: the
/// same flags, the same subtype, and an appearance stream is the one difference between them.
#[test]
fn a_print_flag_that_is_clear_is_ignored_where_the_annotation_has_no_appearance_stream() {
    for appearance in ["", "/AP << >>", "/AP << /N 42 >>"] {
        let paper = drawn(page_with("", appearance), Purpose::Print, None);
        assert!(
            paper.marks,
            "with `{appearance}` the flag is ignored and the constructed appearance prints: {:?}",
            paper.unsupported
        );
    }
    // And an `/AP` whose `/N` is a *state* subdictionary does contain appearance streams,
    // whichever one `/AS` selects — so the sentence does not apply and bit 3 clear keeps it off.
    let states = drawn(
        page_with("", "/AS /Off /AP << /N << /On 6 0 R >> >>"),
        Purpose::Print,
        None,
    );
    assert!(
        !states.marks,
        "a subdictionary of streams is streams the annotation contains"
    );
}

/// §12.5.6.22's target media is the sheet when one is known, and the media box when it is not.
///
/// Table 193 states both branches:
///
/// > If the dimensions of the target media are not known at the time of drawing, drawing shall be
/// > done relative to the dimensions specified by the page's MediaBox entry
///
/// and Table 194 makes `/H` and `/V` percentages "of the width of the target media (or if
/// unknown, the width of the page's MediaBox )". So every number here is one multiplication,
/// checkable by hand on a `/Rect [20 30 60 70]` translated to the origin under the identity
/// matrix — a 40 × 40 box at (0, 0):
///
/// ```text
/// /H 0.25 /V 0.25 of the 100 × 100 media box    x 25..65   y 25..65
/// /H 0.25 /V 0.25 of a 200 × 200 sheet          x 50..90   y 50..90
/// ```
///
/// **The three arms are the discriminating set.** A reader that always used the sheet would fail
/// the screen arm, which §12.5.6.22 requires by name — "[w]hen displaying a watermark annotation
/// on-screen, interactive PDF processors shall use the dimensions of the media box ... so that the
/// scroll and zoom behaviour is the same as for other annotations" — and one that never used it
/// would fail the paper arm. The third arm is printing onto a sheet nobody named, which is
/// Table 193's *not known* and lands back on the media box.
#[test]
#[expect(
    clippy::doc_markdown,
    reason = "verbatim quotations: §12.5.6.22, Table 193 and Table 194 spell FixedPrint and \
              MediaBox without backticks"
)]
fn a_fixed_print_watermark_goes_against_the_sheet_when_the_sheet_is_known() {
    let fixture = || {
        watermark(
            "[0 0 100 100]",
            "[20 30 60 70]",
            "/FixedPrint << /Type /FixedPrint /H 0.25 /V 0.25 >>",
        )
    };
    let sheet = Some([0.0, 0.0, 200.0, 200.0]);

    assert_eq!(
        extent(fixture(), Purpose::View, sheet),
        (25, 25, 64, 64),
        "on a screen the media box is the media, whatever sheet a host has in mind"
    );
    assert_eq!(
        extent(fixture(), Purpose::Print, sheet),
        (50, 50, 89, 89),
        "on paper the sheet is the media"
    );
    assert_eq!(
        extent(fixture(), Purpose::Print, None),
        (25, 25, 64, 64),
        "and a sheet nobody stated is Table 193's dimensions that are not known"
    );
}

/// An export is neither of Table 167's two devices, so it takes the screen's reading.
///
/// The table names a screen and a printed page and nothing else, so there is no third row to
/// apply to a raster written to a file. Reading bit 3 for one would let a document decide what an
/// exported image contains on the strength of a sentence about paper; this is a decision rather
/// than a fallthrough, and ADR 1179 is where it is argued.
///
/// The pair is what makes it a claim: `/F 36` is off the screen and onto the paper, and an export
/// answers with the screen.
#[test]
fn an_export_reads_table_167_as_a_screen_does() {
    for flags in ["", "/F 4", "/F 32", "/F 36"] {
        let screen = drawn(page_with(flags, "/AP << /N 6 0 R >>"), Purpose::View, None);
        let export = drawn(
            page_with(flags, "/AP << /N 6 0 R >>"),
            Purpose::Export,
            None,
        );
        assert_eq!(
            export.marks, screen.marks,
            "`{flags}`: an export is not a printed page"
        );
    }
}
