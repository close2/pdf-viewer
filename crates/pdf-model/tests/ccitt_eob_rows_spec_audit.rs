//! Public-reader control for ISO 32000-2 Table 11's EndOfBlock-over-Rows rule.
//!
//! The Group 4 stream is a hand-assembled two-line source from T.4/T.6 codewords and ends with
//! EOFB. Table 11 says `EndOfBlock` true overrides Rows. With image Height 2 and Rows 1, the PDF
//! consumer must still receive both complete rows; this exercises `DecodeParms` parsing and the
//! image reader, beyond the existing low-level pdf-ccitt Summary assertion.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "the small hand-built fixture has fixed bounds and should fail loudly"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const GENEROUS: u64 = 1 << 20;
const EOFB: &str = "000000000001 000000000001";
const H: &str = "001";

fn bits(digits: &str) -> Vec<u8> {
    let digits: Vec<bool> = digits
        .chars()
        .filter(|character| !character.is_whitespace())
        .map(|character| character == '1')
        .collect();
    digits
        .chunks(8)
        .map(|chunk| {
            chunk.iter().enumerate().fold(0_u8, |byte, (index, one)| {
                byte | (u8::from(*one) << 7_usize.saturating_sub(index))
            })
        })
        .collect()
}

fn stream_object(number: u32, dictionary: &str, data: &[u8]) -> Vec<u8> {
    let mut object = format!(
        "{number} 0 obj\n<< {dictionary} /Length {} >>\nstream\n",
        data.len()
    )
    .into_bytes();
    object.extend_from_slice(data);
    object.extend_from_slice(b"\nendstream\nendobj\n");
    object
}

fn pdf(rows: u32) -> Vec<u8> {
    // The first line uses T.4's white-zero and black-16 terminating codes after T.6 horizontal
    // mode. The second line uses horizontal white-16 and black-zero codes; EOFB follows both.
    let encoded = bits(&format!(
        "{H}00110101 0000010111 {H}101010 0000110111 {EOFB}"
    ));
    let objects = [
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] /Resources << /XObject << /Im 5 0 R >> >> /Contents 4 0 R >>\nendobj\n".to_vec(),
        stream_object(4, "", b"40 0 0 40 0 0 cm /Im Do"),
        stream_object(
            5,
            &format!("/Type /XObject /Subtype /Image /Width 16 /Height 2 /ColorSpace /DeviceGray /BitsPerComponent 1 /Filter /CCITTFaxDecode /DecodeParms << /K -1 /Columns 16 /Rows {rows} /EndOfBlock true >>"),
            &encoded,
        ),
    ];

    let mut out = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::new();
    for object in objects {
        offsets.push(out.len());
        out.extend_from_slice(&object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    let mut xref = String::new();
    let _ = writeln!(xref, "xref\n0 {size}");
    xref.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(xref, "{offset:010} 00000 n ");
    }
    let _ = write!(
        xref,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.extend_from_slice(xref.as_bytes());
    out
}

fn render(rows: u32) -> pdf_render::Raster {
    let document = Document::open(pdf(rows)).expect("complete PDF 1.7 fixture");
    assert!(!document.was_recovered(), "fixture has exact xref offsets");
    assert!(
        !document.cross_reference_chain_was_cut(),
        "the fixture is not accepted through xref-chain truncation"
    );
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "{:?}",
        interpretation.unsupported
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, GENEROUS)
        .expect("40x40 page fits target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpretation.display_list, target)
        .expect("valid Group 4 image renders")
}

fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    let row = raster.height.saturating_sub(1).saturating_sub(y);
    let at = ((row * raster.width + x) as usize) * 4;
    [
        raster.data[at],
        raster.data[at + 1],
        raster.data[at + 2],
        raster.data[at + 3],
    ]
}

#[test]
fn table11_eob_true_overrides_rows_through_the_image_reader() {
    let raster = render(1);
    let rows_control = render(2);

    // The upper source row is black, the lower row white. A duplicated first row, a missing
    // second row, or a Rows-limited decode cannot satisfy these opaque literal samples.
    assert_eq!(pixel(&raster, 20, 10), [255, 255, 255, 255]);
    assert_eq!(pixel(&raster, 20, 30), [0, 0, 0, 255]);
    assert_eq!(pixel(&raster, 20, 10), pixel(&rows_control, 20, 10));
    assert_eq!(pixel(&raster, 20, 30), pixel(&rows_control, 20, 30));
}
