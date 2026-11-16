//! Public render control for inherited button /V versus widget /AS, under §12.7.5.2.3.
//!
//! The input with unequal V and AS is intentionally a producer-mismatch case: §12.7.5.2.3 first
//! says the entries shall agree, then explicitly says the reader shall use AS when unequal. The
//! other cases are matching positive controls. /V lives only on the parent field (Table 226 says
//! it is inheritable); /AS and /AP/N's state subdictionary live on its widget. Each state stream
//! paints a different solid color. Exact xref offsets and no-recovery assertion keep parser
//! recovery from satisfying the test accidentally.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "source-controlled PDF and pixel fixture failures should be immediate"
)]

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

#[derive(Clone, Copy)]
enum AppearanceState {
    On,
    Off,
}

fn push_object(output: &mut Vec<u8>, offsets: &mut [usize], number: usize, body: &[u8]) {
    offsets[number] = output.len();
    output.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
    output.extend_from_slice(body);
    output.extend_from_slice(b"\nendobj\n");
}

fn stream(content: &[u8]) -> Vec<u8> {
    let mut body = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 40 40] /Resources << >> /Length {} >>\nstream\n",
        content.len()
    )
    .into_bytes();
    body.extend_from_slice(content);
    body.extend_from_slice(b"\nendstream");
    body
}

fn pdf(value: AppearanceState, appearance: AppearanceState) -> Vec<u8> {
    let value = match value {
        AppearanceState::On => "On",
        AppearanceState::Off => "Off",
    };
    let appearance = match appearance {
        AppearanceState::On => "On",
        AppearanceState::Off => "Off",
    };
    let mut output = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = vec![0usize; 9];
    push_object(
        &mut output,
        &mut offsets,
        1,
        b"<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] >> >>",
    );
    push_object(
        &mut output,
        &mut offsets,
        2,
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    );
    push_object(
        &mut output,
        &mut offsets,
        3,
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Contents 4 0 R /Annots [6 0 R] >>",
    );
    push_object(
        &mut output,
        &mut offsets,
        4,
        b"<< /Length 0 >>\nstream\n\nendstream",
    );
    // V is inherited from this field by the widget at object 6.
    let field = format!("<< /FT /Btn /T (box) /V /{value} /Kids [6 0 R] >>");
    push_object(&mut output, &mut offsets, 5, field.as_bytes());
    let widget = format!(
        "<< /Type /Annot /Subtype /Widget /Parent 5 0 R /Rect [10 10 50 50] /F 4 /AS /{appearance} /AP << /N << /On 7 0 R /Off 8 0 R >> >> >>"
    );
    push_object(&mut output, &mut offsets, 6, widget.as_bytes());
    push_object(
        &mut output,
        &mut offsets,
        7,
        &stream(b"1 0 0 rg 0 0 40 40 re f"),
    );
    push_object(
        &mut output,
        &mut offsets,
        8,
        &stream(b"0 0 1 rg 0 0 40 40 re f"),
    );

    let xref = output.len();
    output.extend_from_slice(format!("xref\n0 {}\n", offsets.len()).as_bytes());
    output.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        output.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    output.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            offsets.len()
        )
        .as_bytes(),
    );
    output
}

fn centre(value: AppearanceState, appearance: AppearanceState) -> [u8; 4] {
    let document = Document::open(pdf(value, appearance)).expect("valid exact-xref PDF");
    assert!(!document.was_recovered(), "fixture xref offsets are exact");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpreted = pdf_model::content::interpret(&document, &page);
    assert!(
        interpreted.is_complete(),
        "complete appearance: {:?}",
        interpreted.unsupported
    );
    let target =
        TargetSpec::for_page(&interpreted.display_list, 1.0, 1 << 20).expect("small raster");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpreted.display_list, target)
        .expect("solid form XObjects are supported");
    let row = raster.height.saturating_sub(1).saturating_sub(30);
    let start = usize::try_from((row * raster.width + 30) * 4).expect("bounded center pixel");
    raster
        .data
        .get(start..start.saturating_add(4))
        .expect("center pixel in bounds")
        .try_into()
        .expect("RGBA pixel")
}

#[test]
fn inherited_field_value_and_widget_appearance_state_match_and_mismatch_controls() {
    // Matched controls: each appearance-state stream has a literal, independent color.
    assert_eq!(
        centre(AppearanceState::On, AppearanceState::On),
        [255, 0, 0, 255]
    );
    assert_eq!(
        centre(AppearanceState::Off, AppearanceState::Off),
        [0, 0, 255, 255]
    );

    // §12.7.5.2.3 expressly resolves a mismatch in favor of the widget's AS, not inherited V.
    // These are deliberately unequal source entries; this is reader-precedence coverage, not a
    // claim that a producer is permitted to write a mismatch.
    assert_eq!(
        centre(AppearanceState::On, AppearanceState::Off),
        [0, 0, 255, 255]
    );
    assert_eq!(
        centre(AppearanceState::Off, AppearanceState::On),
        [255, 0, 0, 255]
    );
}
