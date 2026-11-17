//! Table 228 marks DA and Q inheritable; DS is not. Compare an ancestor-only DS with the
//! no-DS control, while a terminal-field DS proves the style has a visible public effect.

#![expect(
    clippy::expect_used,
    reason = "test fixture construction and public render assertions"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

fn pdf(parent_ds: &str, terminal_ds: &str) -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /NeedAppearances true /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv 7 0 R >> >> >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> /Contents 8 0 R /Annots [6 0 R] >>".to_owned(),
        format!("<< /T (group) /Kids [5 0 R] {parent_ds} >>"),
        format!("<< /Parent 4 0 R /Kids [6 0 R] /FT /Tx /Ff 33554432 /T (leaf) /V (Hi) /RV (<body><p>Hi</p></body>) {terminal_ds} >>"),
        "<< /Type /Annot /Subtype /Widget /Parent 5 0 R /P 3 0 R /Rect [20 20 180 80] /F 4 >>".to_owned(),
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

fn inked_rows(bytes: Vec<u8>) -> Vec<u32> {
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
    (0..raster.height)
        .filter(|row| {
            (0..raster.width).any(|x| {
                let pixel = usize::try_from(*row)
                    .expect("row fits usize")
                    .saturating_mul(usize::try_from(raster.width).expect("width fits usize"))
                    .saturating_add(usize::try_from(x).expect("column fits usize"));
                let alpha = pixel.saturating_mul(4).saturating_add(3);
                raster.data.get(alpha).is_some_and(|value| *value > 0)
            })
        })
        .collect()
}

#[test]
fn nonterminal_default_style_does_not_leak_into_a_terminal_variable_text_field() {
    let plain = inked_rows(pdf("", ""));
    assert!(!plain.is_empty(), "the literal Hi value is visibly drawn");

    let terminal_style = inked_rows(pdf("", "/DS (font-size:36pt)"));
    assert!(
        terminal_style.len() > plain.len().saturating_mul(2),
        "local DS positive control"
    );

    let terminal_override = inked_rows(pdf("/DS (font-size:36pt)", "/DS (font-size:12pt)"));
    let same_terminal_value_without_ancestor = inked_rows(pdf("", "/DS (font-size:12pt)"));
    assert_eq!(
        terminal_override, same_terminal_value_without_ancestor,
        "a terminal field's own DS is used without inheriting the nonterminal ancestor's DS"
    );

    let ancestor_style = inked_rows(pdf("/DS (font-size:36pt)", ""));
    assert_eq!(
        ancestor_style, plain,
        "Table 228 does not designate /DS inheritable; only the terminal field's own entry applies"
    );
}
