//! What Table 11's `/Rows` bounds, and what §8.9.5.1's `/Height` fills.
//!
//! ISO 32000-2 §7.4.6 Table 11 gives one entry power over another:
//!
//! > A flag indicating whether the filter shall expect the encoded data to be terminated by an
//! > end-of-block pattern, overriding the Rows parameter. If false , the filter shall stop when
//! > it has decoded the number of lines indicated by Rows or when its data has been exhausted,
//! > whichever occurs first.
//!
//! That is the `/EndOfBlock` row, whose default the same row states as true. So `/Rows` binds the
//! filter in exactly one case — `/EndOfBlock` false — and in that case it may legitimately
//! stop the decode short of the image, whose extent is the dictionary's `/Height`
//! (§8.9.5.1) and nothing in Table 11. Until the five-hundred-and-ninety-ninth session
//! [`pdf_sandbox`]'s pipe carried **one** number for both jobs, so the short raster came back
//! short and `pdf_model::image` refused the whole picture for being the size the clause asked
//! for.
//!
//! **The fixtures are hand-built and come in a pair differing in one entry's value**, which is
//! trap 8's construction: the corpus contains no `/EndOfBlock false` with a short `/Rows` — a fax
//! gateway is likelier to emit one than a page layout program — and a corpus cannot exercise a
//! rule no document happens to state. Everything else about the two files is identical, down to
//! the encoded bytes, so nothing but the flag can explain the difference in what is drawn.
//!
//! The encoded data is written here rather than taken from anywhere: four scan lines of eight
//! black pixels, Group 3 one-dimensional, which ITU-T T.4's terminating codes spell as a white
//! run of zero (`00110101`) followed by a black run of eight (`000101`), fourteen bits a line and
//! fifty-six for the image. That is the one thing §7.4.6 does *not* state — it defers the coding
//! entirely to T.4 and T.6 — so it is named here in full rather than cited.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture should fail loudly, and the fixtures are tiny \
              pages where no index can overflow"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer as _, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Pixel budget, far above the pages these tests build.
const GENEROUS: u64 = 1 << 30;

/// Four scan lines of eight black pixels, Group 3 one-dimensional, with no end-of-block pattern.
///
/// `00110101` is T.4's terminating code for a white run of zero and `000101` its code for a black
/// run of eight; a scan line is therefore fourteen bits and four of them fill seven bytes exactly.
const FOUR_BLACK_LINES: [u8; 7] = [0x35, 0x14, 0xD4, 0x53, 0x51, 0x4D, 0x45];

/// A one-page PDF drawing one 8×4 CCITT image over the whole 40×40 page.
///
/// `parms` is the `/DecodeParms` dictionary's body and is the only thing the fixtures vary.
fn page_with_ccitt_image(parms: &str, data: &[u8]) -> Vec<u8> {
    page_with_ccitt_image_of_width(8, parms, data)
}

/// The same page, with the image's own `/Width` chosen by the caller.
///
/// `/Width` is separate from Table 11's `/Columns` and the last test below is about exactly that
/// difference, so the fixture has to be able to state the two apart.
fn page_with_ccitt_image_of_width(width: u32, parms: &str, data: &[u8]) -> Vec<u8> {
    let content = "40 0 0 40 0 0 cm /Im Do";
    let dict = format!(
        "/Type /XObject /Subtype /Image /Width {width} /Height 4 /ColorSpace /DeviceGray \
         /BitsPerComponent 1 /Filter /CCITTFaxDecode /DecodeParms << {parms} >>"
    );
    let objects = vec![
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
          /Resources << /XObject << /Im 5 0 R >> >> /Contents 4 0 R >>\nendobj\n"
            .to_vec(),
        stream_object(4, "", content.as_bytes()),
        stream_object(5, &dict, data),
    ];

    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for object in &objects {
        offsets.push(out.len());
        out.extend_from_slice(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let mut trailer = String::new();
    let _ = writeln!(trailer, "xref\n0 {size}");
    trailer.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(trailer, "{offset:010} 00000 n ");
    }
    let _ = write!(
        trailer,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.extend_from_slice(trailer.as_bytes());
    out
}

fn stream_object(number: u32, dict: &str, data: &[u8]) -> Vec<u8> {
    let mut out = format!(
        "{number} 0 obj\n<< {dict} /Length {} >>\nstream\n",
        data.len()
    )
    .into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream\nendobj\n");
    out
}

/// Interprets a fixture and hands back what it drew and what it said about it.
fn interpret(bytes: Vec<u8>) -> (pdf_render::Raster, Vec<String>) {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    let said = interpretation
        .unsupported
        .iter()
        .map(|report| format!("{report:?}"))
        .collect();
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, GENEROUS)
        .expect("a 40x40 page is a valid target");
    let raster = CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .expect("the display list holds nothing the CPU backend refuses");
    (raster, said)
}

/// The grey at the middle of scan line `row` of the four the image states.
fn scan_line(raster: &pdf_render::Raster, row: u32) -> u8 {
    let (x, y) = (20, row * 10 + 5);
    raster.data[((y * raster.width + x) * 4) as usize]
}

/// `/EndOfBlock` true overrides `/Rows`, so a `/Rows` below `/Height` decodes the whole image.
///
/// This is the fixture's control and the half of Table 11 ADR 0392 already implemented: with the
/// entry at its default the number 2 has no power at all, and all four lines the data carries are
/// drawn. It is stated here rather than assumed because the other test's whole meaning is the
/// difference between the two.
#[test]
fn end_of_block_true_ignores_a_short_rows_and_draws_the_whole_image() {
    let (raster, said) = interpret(page_with_ccitt_image(
        "/K 0 /Columns 8 /Rows 2 /EndOfBlock true",
        &FOUR_BLACK_LINES,
    ));
    for row in 0..4 {
        assert_eq!(
            scan_line(&raster, row),
            0,
            "scan line {row} is black, because the data carries all four and /Rows does not bind"
        );
    }
    assert!(
        said.is_empty(),
        "an image the filter reached the end of is not worth a report: {said:?}"
    );
}

/// `/EndOfBlock` false lets `/Rows` stop the filter, and the rest of the grid is blank and named.
///
/// The same seven bytes and the same `/Rows 2`: only the flag differs. Table 11's second sentence
/// now applies, the filter stops after two of the four lines, and the two the image still states
/// are white — which is a choice, since ISO 32000-2 says nothing about them, and therefore a
/// report. Before the fix this fixture drew *nothing*: the raster came back two lines tall and
/// the height check refused the picture.
#[test]
fn end_of_block_false_stops_at_rows_and_blanks_the_rest_out_loud() {
    let (raster, said) = interpret(page_with_ccitt_image(
        "/K 0 /Columns 8 /Rows 2 /EndOfBlock false",
        &FOUR_BLACK_LINES,
    ));
    assert_eq!(
        scan_line(&raster, 0),
        0,
        "the first line the filter reached"
    );
    assert_eq!(scan_line(&raster, 1), 0, "and the second, which is /Rows");
    assert_eq!(
        scan_line(&raster, 2),
        255,
        "the third line is past /Rows, so it is blank rather than absent"
    );
    assert_eq!(scan_line(&raster, 3), 255, "and so is the fourth");
    assert!(
        said.iter().any(|report| report.contains("7.4.6")
            && report.contains("/Rows 2")
            && report.contains("4 scan lines")),
        "the shortfall should name the clause and both numbers, and said {said:?}"
    );
}

/// A `/Rows` that reaches `/Height` is the ordinary file and says nothing.
///
/// The report's condition is Table 11's and not "this file mentions `/EndOfBlock`": where the
/// filter is bounded by exactly the lines the image states, nothing is substituted and there is
/// nothing to say. Trap 11 is what this test is for — a report that fires where the clause asks
/// for nothing is this project's commonest instrument defect.
#[test]
fn end_of_block_false_reaching_the_whole_height_says_nothing() {
    let (raster, said) = interpret(page_with_ccitt_image(
        "/K 0 /Columns 8 /Rows 4 /EndOfBlock false",
        &FOUR_BLACK_LINES,
    ));
    for row in 0..4 {
        assert_eq!(scan_line(&raster, row), 0, "scan line {row} is black");
    }
    assert!(
        said.is_empty(),
        "a filter bounded at the image's own height substitutes nothing: {said:?}"
    );
}

/// Four scan lines of a white run of four, a black run of eight and a white run of four.
///
/// Sixteen columns to the line, which is twelve rounded up to the next multiple of eight. T.4's
/// terminating codes spell a white run of four `1011` and a black run of eight `000101`, so a
/// line is fourteen bits and four of them fill seven bytes exactly — the same arithmetic
/// [`FOUR_BLACK_LINES`] rests on, over a wider line.
const FOUR_LINES_OF_SIXTEEN: [u8; 7] = [0xB1, 0x6E, 0xC5, 0xBB, 0x16, 0xEC, 0x5B];

/// `/Columns` is the image's width taken to the next multiple of eight, which is not a
/// disagreement.
///
/// §7.4.6 Table 11 defines the entry and then says what the filter does with it:
///
/// > The width of the image in pixels. If the value is not a multiple of 8, the filter shall
/// > adjust the width of the unencoded image to the next multiple of 8 so that each line starts
/// > on a byte boundary.
///
/// So a producer that writes the *adjusted* width has written the width the filter works on, and
/// the encoded runs are for that many columns: believing `/Columns` is what decodes the data at
/// all. §8.9.5.1's `/Width` then says how many of each line's samples are the image, and since
/// both numbers round to the same number of bytes per line, not one sample moves.
///
/// This tree refused the pair outright until the six-hundred-and-nineteenth session, on two
/// crawled scans whose `/Columns` were 872 against a `/Width` of 869 and 896 against 892 —
/// both of them exactly this arithmetic.
#[test]
fn columns_may_be_the_width_rounded_up_to_a_byte_boundary() {
    let (raster, said) = interpret(page_with_ccitt_image_of_width(
        12,
        "/K 0 /Columns 16 /Rows 4 /EndOfBlock false",
        &FOUR_LINES_OF_SIXTEEN,
    ));
    // Column 1 of twelve across a forty-unit page, and column 9: the first is inside the white
    // run of four and the second inside the black run of eight.
    let at = |column: u32, row: u32| -> u8 {
        let x = column * 40 / 12 + 1;
        let y = row * 10 + 5;
        raster.data[((y * raster.width + x) * 4) as usize]
    };
    for row in 0..4 {
        assert_eq!(at(1, row), 255, "line {row} opens with a white run of four");
        assert_eq!(at(9, row), 0, "and continues with a black run of eight");
    }
    assert!(
        said.is_empty(),
        "the adjusted width is what Table 11 asks the filter for, so there is nothing to \
         report: {said:?}"
    );
}

/// A `/Columns` that is not the width nor its byte-aligned form is still a refusal.
///
/// The relaxation above is Table 11's own sentence and nothing wider. Where the two numbers
/// disagree by more than the padding, the runs in the data are for a line this image is not, and
/// §7.3.8.2's "[a]ll of these constraints shall be consistent" has been broken in a way that
/// moves samples — so the picture is refused and named rather than drawn shifted.
#[test]
fn a_columns_that_is_not_the_padded_width_is_refused() {
    let (_, said) = interpret(page_with_ccitt_image_of_width(
        12,
        "/K 0 /Columns 24 /Rows 4 /EndOfBlock false",
        &FOUR_LINES_OF_SIXTEEN,
    ));
    assert!(
        said.iter()
            .any(|report| report.contains("/Columns 24") && report.contains("12")),
        "the refusal should name both numbers, and said {said:?}"
    );
}
