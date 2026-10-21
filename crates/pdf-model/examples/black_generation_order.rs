//! Diagnose §11.7.5.3's dependence on the parameters at painting (ADR 2187).
//!
//! `cargo run -p pdf-model --example black_generation_order` prints the same RGB colour
//! selected before and after the black-generation pair, plus an image with that colour.
//! BG(k)=1−k and UCR(k)=k/2 give CMYK (0.6, 0.4, 0.2, 0.6) for RGB (0.2, 0.4, 0.6).
//! All rows owe the printed expected colour. A difference isolates stale converted state;
//! this instrument prints the defect rather than accepting it as a regression baseline.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "generated 100-unit fixture, bounded pixel indices and byte conversion; a malformed fixture must stop the instrument"
)]

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use std::fmt::Write as _;
/// The unused nested form keeps the fixture resource structure fixed.
const NESTED: &str = "";
/// Interprets the generated one-page witness.
fn interpret(bytes: Vec<u8>) -> pdf_model::Interpretation {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    pdf_model::interpret(&document, &page)
}
/// Reads the opaque left-half sample from the CPU result.
fn pixel(interpretation: &pdf_model::Interpretation, x: u32, y: u32) -> [u8; 4] {
    let list = &interpretation.display_list;
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("a 100x100 target");
    let raster = render_cpu::CpuRasterizer::new()
        .rasterize(list, target)
        .expect("the fixture rasterises");
    let at = ((y * raster.width + x) * 4) as usize;
    let bytes = &raster.data[at..at + 4];
    [bytes[0], bytes[1], bytes[2], bytes[3]]
}
/// The page and nested forms have explicit blending spaces; the right half forces compositing.
fn page_group_fixture(
    page_group: &str,
    form_group: &str,
    form_resources: &str,
    form: &str,
    page: &str,
) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] {page_group} \
         /Resources << /ExtGState << /GS << /ca 0.5 /CA 0.5 >> \
         /GK << /BG2 << /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >> >> \
         /GD << /BG2 /Default /BG << /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >> >> \
         /GB << /BG << /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [0] /N 1 >> \
         /UCR << /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [0.5] /N 1 >> >> >> \
         /XObject << /Fm 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{page}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] {form_group} \
         /Resources << /ExtGState << /GS << /ca 0.5 /CA 0.5 >> >> \
         {form_resources} /XObject << /In 6 0 R >> >> /Length {} >>\n\
         stream\n{form}\nendstream\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /S /Transparency /CS /DeviceCMYK >> \
         /Resources << /ExtGState << /GS << /ca 0.5 >> >> >> /Length {} >>\n\
         stream\n{NESTED}\nendstream\nendobj\n",
        page.len() + 1,
        form.len() + 1,
        NESTED.len() + 1
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
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
/// Prints the clause-derived colour beside the operator-order witnesses.
fn main() {
    for (name, mark) in [
        ("pair then solid", "/GB gs 0.2 0.4 0.6 rg 0 0 50 100 re f"),
        ("solid then pair", "0.2 0.4 0.6 rg /GB gs 0 0 50 100 re f"),
        (
            "image under pair",
            "/GB gs q 50 0 0 100 0 0 cm BI /W 1 /H 1 /BPC 8 /CS /RGB /F /AHx ID 336699> EI Q",
        ),
    ] {
        let content = format!("{mark} /GS gs 0 0 0 0 k 60 0 40 100 re f");
        let drawn = interpret(page_group_fixture(
            "/Group << /S /Transparency /CS /DeviceCMYK >>",
            "/Group << /S /Transparency >>",
            "",
            "",
            &content,
        ));
        println!(
            "{name}: {:?}, unsupported {:?}",
            pixel(&drawn, 25, 50),
            drawn.unsupported
        );
    }
    let expected = pdf_model::colour::device_cmyk_blending_space().convert(0.6, 0.4, 0.2, 0.6);
    println!("expected: {:?}", expected.map(|v| (v * 255.0 + 0.5) as u8));
}
