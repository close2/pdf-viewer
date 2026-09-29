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
//! (§8.9.5.1) and nothing in Table 11. [`pdf_sandbox`]'s pipe carries the two numbers apart
//! (ADR 0434): with **one** number for both jobs, the short raster would come back short and
//! `pdf_model::image` would refuse the whole picture for being the size the clause asked for.
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

/// The same page over a mid-grey fill, so that a scan line the image left *unpainted* reads
/// as the grey beneath it and a scan line the image painted white reads as white — the one
/// difference the damaged-data test below is about.
fn page_with_ccitt_image_over_grey(parms: &str, data: &[u8]) -> Vec<u8> {
    page_with_ccitt_image_drawn_by(
        8,
        "0.5 g 0 0 40 40 re f 40 0 0 40 0 0 cm /Im Do",
        parms,
        data,
    )
}

/// The same page, with the image's own `/Width` chosen by the caller.
///
/// `/Width` is separate from Table 11's `/Columns` and the last test below is about exactly that
/// difference, so the fixture has to be able to state the two apart.
fn page_with_ccitt_image_of_width(width: u32, parms: &str, data: &[u8]) -> Vec<u8> {
    page_with_ccitt_image_drawn_by(width, "40 0 0 40 0 0 cm /Im Do", parms, data)
}

/// The page, with its content stream chosen by the caller.
fn page_with_ccitt_image_drawn_by(width: u32, content: &str, parms: &str, data: &[u8]) -> Vec<u8> {
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

/// Two of the four lines, then bytes that are no T.4 code at all.
///
/// The first twenty-eight bits are [`FOUR_BLACK_LINES`]'s first two scan lines exactly; the
/// four bits that finish the fourth byte and the three bytes after it are zero, and no run of
/// eight zero bits begins a T.4 run-length code — the longest white code opens with seven, and
/// twelve zeros are an EOL, which `/EndOfLine false` says this data does not carry. So the
/// filter reads two lines whole and stops inside the third, which is §7.4.6's "an error occurs".
const TWO_BLACK_LINES_THEN_DAMAGE: [u8; 7] = [0x35, 0x14, 0xD4, 0x50, 0x00, 0x00, 0x00];

/// Data the filter stops on before it has delivered one whole scan line, drawn as nothing.
///
/// Every T.4 code has a one in its first eight bits, so an image whose data is four zero bytes
/// has no first line, and there is nothing to draw beside a report: the picture is refused, as
/// it always was. The test is the boundary of the change — what makes a damaged stream worth
/// drawing is a scan line the file carried, and this one carries none.
const DAMAGE_BEFORE_A_LINE: [u8; 4] = [0x00, 0x00, 0x00, 0x00];

/// Damaged data draws the scan lines before the damage, blanks the rest, and says so.
///
/// ISO 32000-2 §7.4.6: "The filter shall not perform any error correction or resynchronization"
/// beyond what `/DamagedRowsBeforeError` asks for, and Table 11 gives that parameter as "[t]he
/// number of damaged rows of data that shall be tolerated before an error occurs" with a
/// default of zero. So at the third line an error occurs and the filter's output is the two
/// lines before it. What the two lines it never reached show is stated nowhere, and they are
/// left *unpainted* — the grey the page has under them shows through, where a white the filter
/// invented would not — which is ADR 0356's choice for §7.3.8.2's short image and the same
/// clause's error; the report is what separates this page from one whose lower half the
/// producer left empty. Before ADR 0794 this fixture drew *nothing*: the whole picture was
/// refused for the rows after the damage.
#[test]
fn damaged_data_draws_the_lines_before_the_damage_and_says_so() {
    let (raster, said) = interpret(page_with_ccitt_image_over_grey(
        "/K 0 /Columns 8",
        &TWO_BLACK_LINES_THEN_DAMAGE,
    ));
    assert_eq!(
        scan_line(&raster, 0),
        0,
        "the first line the filter delivered"
    );
    assert_eq!(scan_line(&raster, 1), 0, "and the second");
    let grey = 100..=160;
    assert!(
        grey.contains(&scan_line(&raster, 2)),
        "the third line is where the error occurred, so it is left unpainted and the page's \
         grey shows through; read {}",
        scan_line(&raster, 2)
    );
    assert!(
        grey.contains(&scan_line(&raster, 3)),
        "and so is the fourth; read {}",
        scan_line(&raster, 3)
    );
    assert!(
        said.iter().any(|report| report.contains("CCITTFaxDecode")
            && report.contains("delivered 2 of the 4 scan lines")
            && report.contains("7.4.6")),
        "the shortfall should name the filter, both numbers and the clause, and said {said:?}"
    );
}

/// Damage before the first whole scan line is still a refusal, with the decoder's sentence.
#[test]
fn damage_before_the_first_line_refuses_the_picture_out_loud() {
    let (raster, said) = interpret(page_with_ccitt_image(
        "/K 0 /Columns 8",
        &DAMAGE_BEFORE_A_LINE,
    ));
    for row in 0..4 {
        assert_eq!(
            scan_line(&raster, row),
            255,
            "nothing was delivered to draw"
        );
    }
    assert!(
        said.iter()
            .any(|report| report.contains("CCITTFaxDecode") && !report.contains("delivered")),
        "the refusal should carry the filter's own sentence and no delivery, and said {said:?}"
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
/// The witnesses are two crawled scans whose `/Columns` are 872 against a `/Width` of 869 and
/// 896 against 892 — both of them exactly this arithmetic (ADR 0454).
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

/// `/DamagedRowsBeforeError` above zero is inert unless the entry applies, and applies only for
/// `/EndOfLine true` and `/K` non-negative.
///
/// ISO 32000-2 §7.4.6 Table 11 gives the entry a precondition in its own row:
///
/// > This entry shall apply only if `EndOfLine` is true and K is non-negative.
///
/// The concealment it asks for resynchronises by "searching for an `EndOfLine` pattern", so it
/// applies only where the encoding carries those patterns (`/EndOfLine true`) and is Group 3
/// (`/K` non-negative). Here the same `/DamagedRowsBeforeError 2` is inert, and the image decodes
/// exactly as it does with no such entry; the tests after it are the entry applied. The corpus
/// states no such value at all (a census over 1450 documents of five corpora found 1048 CCITT
/// images and not one with the entry above zero), so this is trap 8's construction: a rule no
/// document happens to exercise, pinned by hand-built pages.
#[test]
fn damaged_rows_is_inert_where_end_of_line_is_false() {
    let (raster, said) = interpret(page_with_ccitt_image(
        "/K 0 /Columns 8 /DamagedRowsBeforeError 2",
        &FOUR_BLACK_LINES,
    ));
    for row in 0..4 {
        assert_eq!(
            scan_line(&raster, row),
            0,
            "scan line {row} is black: the entry does not apply, so the image decodes whole"
        );
    }
    assert!(
        said.is_empty(),
        "an entry the standard says does not apply is dropped, not reported: {said:?}"
    );
}

/// Four scan lines of eight pels, Group 3 one-dimensional with an end-of-line code before each,
/// the third damaged: black, white, eight zero bits (which begin no T.4 codeword and are not an
/// end-of-line code, which needs eleven), black.
///
/// `000000000001` is T.4 section 4.1.2's end-of-line code; `00110101 000101` a white run of zero
/// and a black run of eight, `10011` a white run of eight (T.4 Table 2).
const BLACK_WHITE_DAMAGED_BLACK: &str = "000000000001 00110101000101 000000000001 10011 \
     000000000001 000000001 000000000001 00110101000101";

/// Packs binary digits, spaces ignored, into bytes, the last padded with zeros.
fn pack(digits: &str) -> Vec<u8> {
    let digits: Vec<u8> = digits
        .bytes()
        .filter(|digit| matches!(digit, b'0' | b'1'))
        .map(|digit| digit - b'0')
        .collect();
    digits
        .chunks(8)
        .map(|chunk| {
            chunk
                .iter()
                .enumerate()
                .fold(0, |byte, (at, bit)| byte | (bit << (7 - at)))
        })
        .collect()
}

/// With `/EndOfLine true` the entry applies, and the damaged third line is concealed as Table 11
/// says: "locating its end in the encoded data by searching for an `EndOfLine` pattern and then
/// substituting decoded data from the previous row if the previous row was not damaged" — so it
/// is the white line above it, the fourth line decodes after it, and the page says a line was
/// concealed. Over the grey page, a concealed line is the white it was given, not the grey an
/// unpainted one shows.
#[test]
fn a_damaged_row_is_concealed_as_the_row_above_where_the_entry_applies() {
    let (raster, said) = interpret(page_with_ccitt_image_over_grey(
        "/K 0 /Columns 8 /EndOfLine true /DamagedRowsBeforeError 2",
        &pack(BLACK_WHITE_DAMAGED_BLACK),
    ));
    let lines: Vec<u8> = (0..4).map(|row| scan_line(&raster, row)).collect();
    assert_eq!(
        lines,
        [0, 255, 255, 0],
        "black, white, the white line again in place of the damaged one, black"
    );
    assert!(
        said.iter()
            .any(|report| report.contains("concealed 1")
                && report.contains("DamagedRowsBeforeError 2")),
        "the concealment should be said beside the drawing, and said {said:?}"
    );
}

/// The same four lines at Table 11's default of zero: the damaged line is where "an error
/// occurs", so the two lines before it are drawn and the two from it are left unpainted.
#[test]
fn at_zero_the_damaged_row_ends_the_decode_where_the_entry_applies() {
    let (raster, said) = interpret(page_with_ccitt_image_over_grey(
        "/K 0 /Columns 8 /EndOfLine true",
        &pack(BLACK_WHITE_DAMAGED_BLACK),
    ));
    assert_eq!((scan_line(&raster, 0), scan_line(&raster, 1)), (0, 255));
    let grey = 100..=160;
    for row in 2..4 {
        assert!(
            grey.contains(&scan_line(&raster, row)),
            "scan line {row} is at or past the damage, so it is left unpainted; read {}",
            scan_line(&raster, row)
        );
    }
    assert!(
        said.iter()
            .any(|report| report.contains("delivered 2 of the 4 scan lines")),
        "the shortfall should be said, and said {said:?}"
    );
}
