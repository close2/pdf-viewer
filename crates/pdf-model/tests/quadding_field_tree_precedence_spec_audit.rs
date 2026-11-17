//! Table 228 Q values are observable in rendered ink; field-tree entries precede `AcroForm` Q.

#![expect(
    clippy::expect_used,
    reason = "test fixture construction and public render assertions"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

fn pdf(parent_q: &str, terminal_q: &str, form_q: &str) -> Vec<u8> {
    let objects = [
        format!("<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /NeedAppearances true /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv 7 0 R >> >> {form_q} >> >>"),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> /Contents 8 0 R /Annots [6 0 R] >>".to_owned(),
        format!("<< /T (group) /Kids [5 0 R] {parent_q} >>"),
        format!("<< /Parent 4 0 R /Kids [6 0 R] /FT /Tx /Ff 0 /T (leaf) /V (Hi) /DA (/Helv 12 Tf 0 g) {terminal_q} >>"),
        "<< /Type /Annot /Subtype /Widget /Parent 5 0 R /P 3 0 R /Rect [20 40 180 70] /F 4 >>".to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_owned(),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{object}\nendobj", index.saturating_add(1));
    }
    let xref = out.len();
    let _ = writeln!(out, "xref\n0 9\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size 9 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

fn ink_span(bytes: Vec<u8>) -> (u32, u32) {
    let document = Document::open(bytes).expect("the PDF is structurally valid");
    assert!(
        !document.was_recovered(),
        "the exact xref resolves the field tree"
    );
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "{:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, 1 << 30).expect("valid target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported display list");
    let inked: Vec<u32> = (0..raster.width)
        .filter(|x| {
            (0..raster.height).any(|row| {
                let pixel = usize::try_from(row)
                    .expect("row fits usize")
                    .saturating_mul(usize::try_from(raster.width).expect("width fits usize"))
                    .saturating_add(usize::try_from(*x).expect("column fits usize"));
                let alpha = pixel.saturating_mul(4).saturating_add(3);
                raster.data.get(alpha).is_some_and(|value| *value > 0)
            })
        })
        .collect();
    assert!(!inked.is_empty(), "the literal Hi value is visibly drawn");
    (
        inked.first().copied().expect("nonempty ink"),
        inked.last().copied().expect("nonempty ink"),
    )
}

#[test]
fn inherited_q_and_acroform_default_move_a_separate_widgets_text() {
    // A field-tree value outranks AcroForm /Q and is literal left justification.
    let left = ink_span(pdf("/Q 2", "/Q 0", "/Q 1"));
    assert!((20..=24).contains(&left.0), "terminal Q=0 wins: {left:?}");

    // Parent Q is inherited and outranks the document-wide Q=0 fallback.
    let right = ink_span(pdf("/Q 2", "", "/Q 0"));
    assert!(
        (175..=179).contains(&right.1),
        "ancestor Q=2 wins: {right:?}"
    );

    // With no field-tree Q, Table 224's document-wide Q=1 is used.
    let centred = ink_span(pdf("", "", "/Q 1"));
    assert!(
        centred.0 > left.0 && centred.1 < right.1,
        "AcroForm Q=1 centres the same two-character value: {centred:?}"
    );

    // The widget is [20,180], so its midpoint is x=100; allow three pixels for
    // glyph sidebearings and raster quantization, independently of layout code.
    assert!(
        (194..=206).contains(&centred.0.saturating_add(centred.1)),
        "centred about x=100: {centred:?}"
    );
    assert!(
        left.1
            .saturating_sub(left.0)
            .abs_diff(right.1.saturating_sub(right.0))
            <= 1
    );

    // With no Q anywhere, Table 228's default is left.
    let default = ink_span(pdf("", "", ""));
    assert_eq!(default, left, "absent Q is the same left-justified text");
}
