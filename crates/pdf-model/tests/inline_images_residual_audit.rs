//! Focused residual §8.9.7 reader tests: mask exception,
//! page `DefaultGray` substitution, and the LZW abbreviation's EOD route.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "the self-contained PDF fixtures have fixed layouts and bounded page pixels"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::{Document, Limits, filter};
use render_cpu::CpuRasterizer;

const BUDGET: u64 = 1 << 30;

fn fixture(content: &[u8], resources: &str) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    body.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    body.extend_from_slice(
        format!("3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] /Resources << {resources} >> /Contents 4 0 R >>\nendobj\n").as_bytes(),
    );
    body.extend_from_slice(
        format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes(),
    );
    body.extend_from_slice(content);
    body.extend_from_slice(b"\nendstream\nendobj\n");

    // PDF 1.7 keeps this fixture independent of PDF 2.0's trailer-ID requirement.
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    let mut rest = body.as_slice();
    while let Some(end) = rest.windows(7).position(|window| window == b"endobj\n") {
        let stop = end + 7;
        offsets.push(bytes.len());
        bytes.extend_from_slice(&rest[..stop]);
        rest = &rest[stop..];
    }
    assert!(rest.is_empty(), "fixture body ends at object boundary");
    let xref_at = bytes.len();
    let size = offsets.len() + 1;
    let mut trailer = String::new();
    let _ = writeln!(trailer, "xref\n0 {size}");
    trailer.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(trailer, "{offset:010} 00000 n ");
    }
    let _ = write!(
        trailer,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    bytes.extend_from_slice(trailer.as_bytes());
    bytes
}

fn render(content: &[u8], resources: &str) -> pdf_render::Raster {
    let document = Document::open(fixture(content, resources)).expect("complete PDF fixture opens");
    assert!(!document.was_recovered(), "fixture has an exact xref");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.unsupported.is_empty(),
        "complete interpretation: {:?}",
        interpretation.unsupported
    );
    let target =
        TargetSpec::for_page(&interpretation.display_list, 1.0, BUDGET).expect("40-point page");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpretation.display_list, target)
        .expect("supported raster")
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

fn lzw_literals(bytes: &[u8]) -> Vec<u8> {
    // Clear, one literal code per sample, then EOD. This short fixture stays at 9 bits/code.
    let codes = std::iter::once(256u16)
        .chain(bytes.iter().copied().map(u16::from))
        .chain(std::iter::once(257u16));
    let mut output = Vec::new();
    let (mut held, mut bits) = (0u32, 0u32);
    for code in codes {
        held = (held << 9) | u32::from(code);
        bits += 9;
        while bits >= 8 {
            let byte = u8::try_from((held >> (bits - 8)) & 0xff).expect("one packed byte");
            output.push(byte);
            bits -= 8;
            held &= (1u32 << bits).saturating_sub(1);
        }
    }
    if bits > 0 {
        output.push(u8::try_from(held << (8 - bits)).expect("padded byte"));
    }
    output
}

#[test]
fn image_mask_true_may_omit_color_space_and_stencils_the_current_color() {
    // ImageMask true is the explicit ColorSpace exception in §8.9.7. With default Decode,
    // sample zero paints the current red and sample one leaves the transparent page untouched.
    let content = b"1 0 0 rg q 40 0 0 20 0 0 cm BI /W 2 /H 1 /BPC 1 /IM true ID \x40 EI Q";
    let raster = render(content, "");
    assert_eq!(
        pixel(&raster, 10, 10),
        [255, 0, 0, 255],
        "zero sample paints current color"
    );
    assert_eq!(
        pixel(&raster, 30, 10),
        [0, 0, 0, 0],
        "one sample leaves the page alone"
    );
}

#[test]
fn inline_device_gray_uses_default_gray_and_matches_named_resource_control() {
    let cal_gray = "[/CalGray << /WhitePoint [0.9505 1 1.089] /Gamma 4 >>]";
    let content_for = |space: &str| {
        let mut bytes =
            format!("q 40 0 0 40 0 0 cm BI /W 1 /H 1 /BPC 8 /CS {space} ID ").into_bytes();
        bytes.push(128);
        bytes.extend_from_slice(b" EI Q");
        bytes
    };
    let resources = format!("/ColorSpace << /DefaultGray {cal_gray} /Cs0 {cal_gray} >>");

    // EC3 Issue #19 says the device name is direct, or reached through its Default space;
    // the explicitly named space is a positive control for the same CalGray conversion.
    let via_default = render(&content_for("/G"), &resources);
    let via_resource = render(&content_for("/Cs0"), &resources);
    let untouched_device = render(&content_for("/G"), "");
    let default_pixel = pixel(&via_default, 20, 20);
    assert_eq!(
        default_pixel,
        pixel(&via_resource, 20, 20),
        "default and named CalGray agree"
    );
    assert!(
        default_pixel[0].abs_diff(pixel(&untouched_device, 20, 20)[0]) > 20,
        "DefaultGray must alter the mid-gray sample; got {default_pixel:?} without override {:?}",
        pixel(&untouched_device, 20, 20)
    );
}

#[test]
fn lzw_abbreviation_expands_and_its_eod_marks_the_inline_extent() {
    let samples = [0x11, 0x22, 0x33, 0x44];
    let encoded = lzw_literals(&samples);
    for filter_spelling in ["/LZW", "/LZWDecode"] {
        let mut content =
            format!("BI /W 4 /H 1 /BPC 8 /CS /G /F {filter_spelling} ID ").into_bytes();
        content.extend_from_slice(&encoded);
        content.extend_from_slice(b" EI q Q");
        let document = Document::open(fixture(&content, "")).expect("complete PDF opens");
        assert!(!document.was_recovered(), "fixture has a complete xref");
        let page = pdf_model::Pages::new(&document).get(0).expect("page one");
        let scan = pdf_model::inline_image::scan(&document, &content, 2, &page.resources, true);
        let image = scan.image.expect("LZW inline image scans through EOD");
        assert_eq!(image.data.as_ref(), encoded.as_slice());
        assert_eq!(
            image
                .dict
                .get("Filter")
                .and_then(pdf_syntax::Object::as_name)
                .map(pdf_syntax::Name::as_bytes),
            Some(&b"LZWDecode"[..])
        );
        assert_eq!(
            filter::decode(b"LZWDecode", image.data.as_ref(), None, Limits::DEFAULT)
                .expect("LZW decodes")
                .as_ref(),
            samples
        );
        assert_eq!(content.get(scan.resume..), Some(&b" q Q"[..]));
    }
}

#[test]
fn ascii85_final_filter_can_start_at_id_with_a_delimiter() {
    // EC3 Issue #20 changes the normative separator exception to “final or only” filter.
    // This one-filter ASCII85 image starts with `%`, a PDF token delimiter: ID is still a
    // separate keyword, and the delimiter is passed unchanged as encoded data, not a comment.
    // ASCII85 `%0-A.` decodes to [0x0d, 0, 0, 0].
    for content in [
        &b"BI /W 4 /H 1 /BPC 8 /CS /G /F /A85 ID%0-A.~> EI q Q"[..],
        &b"BI /W 4 /H 1 /BPC 8 /CS /G /F /A85 ID %0-A.~> EI q Q"[..],
    ] {
        let document = Document::open(fixture(content, "")).expect("complete PDF fixture opens");
        assert!(!document.was_recovered(), "fixture has a complete xref");
        let page = pdf_model::Pages::new(&document).get(0).expect("page one");
        let scan = pdf_model::inline_image::scan(&document, content, 2, &page.resources, true);
        let image = scan.image.expect("valid final ASCII85 filter scans");
        assert_eq!(image.data.as_ref(), b"%0-A.~>");
        assert_eq!(
            image
                .dict
                .get("Filter")
                .and_then(pdf_syntax::Object::as_name)
                .map(pdf_syntax::Name::as_bytes),
            Some(&b"ASCII85Decode"[..])
        );
        assert_eq!(
            filter::decode(b"ASCII85Decode", image.data.as_ref(), None, Limits::DEFAULT)
                .expect("ASCII85 decodes")
                .as_ref(),
            &[0x0d, 0, 0, 0]
        );
        assert_eq!(content.get(scan.resume..), Some(&b" q Q"[..]));
    }
}
