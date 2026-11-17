//! PDF-reader integration for ISO 32000-2:2020(E) §7.4.7's own example.
//!
//! The encoded streams are transcribed from printed page 46 (physical page 61). This test
//! goes through exact-xref PDF parsing, `ASCIIHexDecode` -> `JBIG2Decode`, /`DecodeParms` array
//! alignment, indirect /`JBIG2Globals` lookup, bilevel unpacking and rasterization. The printed
//! example gives no complete expected bitmap, so assertions use independently stated structure:
//! its text region has two instances of one symbol and the decoder output must place two equal,
//! non-square, separated marks (as in the current worker-only probe). Do not treat that as a
//! pixel-perfect JBIG2 corpus oracle.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    reason = "small exact-xref image fixture and bounded raster indexing"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer as _, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const IMAGE_HEX: &str = concat!(
    "000000013000010000001300000034000000420000000000",
    "00000040000000000002062000010000001e000000340000",
    "004200000000000000000200100000000231db51ce51ffac>"
);
const GLOBALS_HEX: &str = concat!(
    "0000000000010000000032000003fffdff02fefefe000000",
    "01000000012ae225aea9a5a538b4d9999c5c8e56ef0f872",
    "7f2b53d4e37ef795cc5506dffac>"
);

fn stream_object(number: u32, dictionary: &str, data: &str) -> Vec<u8> {
    format!(
        "{number} 0 obj\n<< {dictionary} /Length {} >>\nstream\n{data}\nendstream\nendobj\n",
        data.len()
    )
    .into_bytes()
}

fn document(include_globals_parameter: bool) -> Vec<u8> {
    let parms = if include_globals_parameter {
        "/DecodeParms [null << /JBIG2Globals 6 0 R >>]"
    } else {
        "/DecodeParms [null null]"
    };
    let objects = vec![
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 52 66] /Resources << /XObject << /Im 5 0 R >> >> /Contents 4 0 R >>\nendobj\n".to_vec(),
        stream_object(4, "", "q 52 0 0 66 0 0 cm /Im Do Q"),
        stream_object(
            5,
            &format!(
                "/Type /XObject /Subtype /Image /Width 52 /Height 66 /ColorSpace /DeviceGray /BitsPerComponent 1 /Filter [/ASCIIHexDecode /JBIG2Decode] {parms}"
            ),
            IMAGE_HEX,
        ),
        stream_object(6, "/Filter /ASCIIHexDecode", GLOBALS_HEX),
    ];
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for object in objects {
        offsets.push(pdf.len());
        pdf.extend_from_slice(&object);
    }
    let xref = pdf.len();
    let mut trailer = format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1);
    for offset in offsets {
        writeln!(trailer, "{offset:010} 00000 n ").expect("String write");
    }
    write!(
        trailer,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects_len()
    )
    .expect("String write");
    pdf.extend_from_slice(trailer.as_bytes());
    pdf
}

fn objects_len() -> usize {
    7
}

fn render(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    let document = Document::open(bytes).map_err(|error| error.to_string())?;
    assert!(!document.was_recovered(), "exact PDF xref");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .ok_or("missing page")?;
    let interpretation = pdf_model::interpret(&document, &page);
    if !interpretation.unsupported.is_empty() {
        return Err(format!("{:?}", interpretation.unsupported));
    }
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, 1 << 20)
        .map_err(|error| error.to_string())?;
    let raster = CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .map_err(|error| error.to_string())?;
    assert_eq!((raster.width, raster.height), (52, 66));
    assert!(
        raster.data.chunks_exact(4).all(|pixel| pixel[0] == pixel[1]
            && pixel[1] == pixel[2]
            && matches!(pixel[0], 0 | 255)
            && pixel[3] == 255),
        "bilevel opaque gray output"
    );
    Ok(raster.data)
}

#[test]
fn iso_jbig2_example_reaches_the_image_reader_with_globals_and_filter_chain() {
    let rgba = render(document(true)).expect("the ISO example PDF is a valid image");
    let pixel = |x: usize, y: usize| rgba[(y * 52 + x) * 4];
    let occupied_rows: Vec<usize> = (0..66)
        .filter(|&y| (0..52).any(|x| pixel(x, y) < 128))
        .collect();
    let bands: Vec<(usize, usize)> = occupied_rows.iter().fold(Vec::new(), |mut bands, &y| {
        match bands.last_mut() {
            Some((_, previous)) if y == *previous + 1 => *previous = y,
            _ => bands.push((y, y)),
        }
        bands
    });
    assert_eq!(
        bands.len(),
        2,
        "the ISO text region has two symbol instances"
    );
    assert!(bands[0].1 < bands[1].0, "the two instances are separated");
    let bounds = |(top, bottom): (usize, usize)| {
        let points: Vec<(usize, usize)> = (top..=bottom)
            .flat_map(|y| {
                (0..52)
                    .filter(move |&x| pixel(x, y) < 128)
                    .map(move |x| (x, y))
            })
            .collect();
        let min_x = points.iter().map(|p| p.0).min().expect("ink");
        let max_x = points.iter().map(|p| p.0).max().expect("ink");
        (max_x - min_x + 1, bottom - top + 1)
    };
    assert_eq!(
        bounds(bands[0]),
        bounds(bands[1]),
        "one symbol is instantiated twice"
    );
    let cropped = |(top, bottom): (usize, usize)| {
        let left = (0..52)
            .find(|&x| (top..=bottom).any(|y| pixel(x, y) < 128))
            .expect("left ink");
        let right = (0..52)
            .rfind(|&x| (top..=bottom).any(|y| pixel(x, y) < 128))
            .expect("right ink");
        (top..=bottom)
            .flat_map(|y| (left..=right).map(move |x| pixel(x, y)))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        cropped(bands[0]),
        cropped(bands[1]),
        "both instances draw the same symbol samples"
    );
    let (width, height) = bounds(bands[0]);
    assert_ne!(width, height, "axis-sensitive structural control");
}

/// Project reporting policy for an incomplete example; no universal ISO rejection rule is inferred.
#[test]
fn missing_required_jbig2_globals_is_reported_by_the_reader() {
    assert!(
        render(document(false)).is_err(),
        "the symbol dictionary is in the globals stream"
    );
}
