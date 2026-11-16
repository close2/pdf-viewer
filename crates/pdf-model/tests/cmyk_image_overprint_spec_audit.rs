//! §8.6.7 / §11.7.4.5: a sampled CMYK image does not use OPM 1's zero-tint retention.
//!
//! A CMYK image supplies all four process components. A zero sample in its K component is
//! therefore painted as zero; it is not the special OPM 1 case reserved for a direct
//! `DeviceCMYK` current-colour operation. Table 146 explicitly excludes a sampled image from its
//! first row. Compare it with the same page's direct fill under OPM 1 and with an ordinary fill
//! of the image's source colour.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "broken exact-xref fixture or out-of-bounds sample must fail loudly"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const LIMIT: u64 = 1 << 20;

fn page(content: &str, extra: &str) -> Vec<u8> {
    let image = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 \
         /ColorSpace /DeviceCMYK /BitsPerComponent 8 /Filter /ASCIIHexDecode /Length 9 >>\n\
         stream\nFF000000>\nendstream\nendobj\n{extra}"
    );
    let content_length = content.len() + 1;
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
         /Group << /S /Transparency /CS /DeviceCMYK >> \
         /Resources << /XObject << /Im 5 0 R >> \
         /ExtGState << /O << /OP true /op true /OPM 1 >> >> >> \
         /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {content_length} >>\nstream\n{content}\n\
         endstream\nendobj\n{image}"
    );
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(bytes.len());
        bytes.push_str(object);
    }
    let xref = bytes.len();
    let size = offsets.len() + 1;
    writeln!(bytes, "xref\n0 {size}\n0000000000 65535 f ").expect("writing to String cannot fail");
    for offset in offsets {
        writeln!(bytes, "{offset:010} 00000 n ").expect("writing to String cannot fail");
    }
    writeln!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF"
    )
    .expect("writing to String cannot fail");
    bytes.into_bytes()
}

fn centre(bytes: Vec<u8>) -> [u8; 4] {
    let document = Document::open(bytes).expect("valid fixture");
    assert!(!document.was_recovered(), "fixture has exact xref offsets");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "fixture is completely interpreted: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, LIMIT).expect("bounded 40x40 raster");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("CPU rasterizer supports the fixture");
    let index = ((20 * raster.width) + 20) as usize * 4;
    raster.data[index..index + 4]
        .try_into()
        .expect("four channels per pixel")
}

fn image(overprint: bool) -> [u8; 4] {
    let overprint_state = if overprint { "/O gs " } else { "" };
    centre(page(
        &format!(
            "0 0 0 1 k 0 0 40 40 re f {overprint_state} \
             q 40 0 0 40 0 0 cm /Im Do Q"
        ),
        "",
    ))
}

#[test]
fn device_cmyk_image_zero_samples_are_painted_even_when_opm_is_one() {
    let sampled_with_opm = image(true);
    let sampled_without_op = image(false);
    let ordinary_cyan = centre(page("1 0 0 0 k 0 0 40 40 re f", ""));
    assert_eq!(
        sampled_without_op, ordinary_cyan,
        "the sampled colour equals an ordinary direct cyan fill"
    );
    let direct_opm = centre(page(
        "0 0 0 1 k 0 0 40 40 re f /O gs 1 0 0 0 k 0 0 40 40 re f",
        "",
    ));

    assert_eq!(
        sampled_with_opm, sampled_without_op,
        "the image's zero K sample paints zero: image painting does not apply OPM 1"
    );
    assert_ne!(
        sampled_with_opm, direct_opm,
        "the direct DeviceCMYK fill is the distinct OPM 1 control and retains backdrop K"
    );
}
