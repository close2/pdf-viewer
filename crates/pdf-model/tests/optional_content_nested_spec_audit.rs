//! ISO §8.11 visibility composition with matched rendered positive controls.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;
use std::fmt::Write as _;

const GENEROUS: u64 = 1 << 30;

fn assemble(body: &str) -> Vec<u8> {
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
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn render(body: &str) -> pdf_render::Raster {
    let document = Document::open(assemble(body)).expect("exact-xref fixture opens");
    assert!(!document.was_recovered(), "fixture uses exact xref offsets");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "no recovery or unsupported content: {:?}",
        interpretation.unsupported
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, GENEROUS).expect("target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpretation.display_list, target)
        .expect("supported raster")
}

fn alpha(r: &pdf_render::Raster, x: u32, y: u32) -> u8 {
    let row = r.height - 1 - y;
    r.data[((row * r.width + x) * 4 + 3) as usize]
}

/// The parent-group rule following Table 96 and §8.11.3.3 compose: an ON form's own /OC cannot
/// make it visible through an enclosing hidden marked-content sequence. A visible
/// square outside the sequence controls page drawing; the same form with the
/// outer group ON separately proves the form paints when unhidden.
#[test]
fn visible_form_oc_is_still_hidden_by_its_outer_marked_content_context() {
    let page_content = "/OC /outer BDC /Fx Do EMC 60 20 30 60 re f\n";
    let form_content = "10 20 30 60 re f\n";
    let body = |outer_off: bool| {
        format!(
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [5 0 R 6 0 R] /D << {} >> >> >>\nendobj\n\
2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Properties << /outer 5 0 R >> /XObject << /Fx 7 0 R >> >> /Contents 4 0 R >>\nendobj\n\
4 0 obj\n<< /Length {} >>\nstream\n{}endstream\nendobj\n\
5 0 obj\n<< /Type /OCG /Name (Outer off) >>\nendobj\n\
6 0 obj\n<< /Type /OCG /Name (Form on) >>\nendobj\n\
7 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /OC 6 0 R /Length {} >>\nstream\n{}endstream\nendobj\n",
            if outer_off { "/OFF [5 0 R]" } else { "" },
            page_content.len(),
            page_content,
            form_content.len(),
            form_content
        )
    };
    let unhidden = render(&body(false));
    assert!(
        alpha(&unhidden, 20, 50) > 0,
        "same form paints when outer group is ON"
    );
    let hidden = render(&body(true));
    assert_eq!(
        alpha(&hidden, 20, 50),
        0,
        "ON form is suppressed by hidden outer context"
    );
    assert!(
        alpha(&hidden, 70, 50) > 0,
        "ordinary square after EMC remains visible"
    );
}

/// Table 97 says null/deleted OCG references are ignored and, if all are invalid,
/// /P has no effect. The first fixture therefore draws; the real OFF-group control
/// must hide under `AnyOn`, proving this is not a permissive rendering path.
#[test]
fn null_membership_references_make_policy_have_no_effect() {
    let content = "/OC /oc BDC 20 20 60 60 re f EMC\n";
    let common = |ocmd: &str, group: &str| {
        format!(
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [5 0 R] /D << /OFF [5 0 R] >> >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Properties << /oc 6 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{}endstream\nendobj\n\
         {group}{ocmd}",
            content.len(),
            content
        )
    };
    let real = render(&common(
        "6 0 obj\n<< /Type /OCMD /OCGs [5 0 R] /P /AnyOn >>\nendobj\n",
        "5 0 obj\n<< /Type /OCG /Name (Off) >>\nendobj\n",
    ));
    assert_eq!(alpha(&real, 50, 50), 0, "valid OFF group makes AnyOn false");
    let null = render(&common(
        "6 0 obj\n<< /Type /OCMD /OCGs [null] /P /AnyOn >>\nendobj\n",
        "5 0 obj\n<< /Type /OCG /Name (Off) >>\nendobj\n",
    ));
    assert!(
        alpha(&null, 50, 50) > 0,
        "all-invalid OCGs leave /P without effect"
    );
}
