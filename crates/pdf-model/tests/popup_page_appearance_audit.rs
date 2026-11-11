//! Reader control for §12.5.6.14: a Popup is host-window data, not a page appearance.
//! Parent `Square` has a valid stored appearance as a positive control; the linked Popup has no
//! `AP`, as required. This intentionally does not assert generic markup activation without /Popup.

#![expect(
    clippy::indexing_slicing,
    reason = "small complete fixture and exact interior pixels; malformed setup should fail loudly"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

fn fixture() -> Vec<u8> {
    let appearance = "1 0 0 rg 0 0 10 10 re f";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R 7 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Square /Rect [10 70 30 90] \
         /Contents (parent note) /Popup 7 0 R /AP << /N 6 0 R >> >>\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] /Resources << >> \
         /Length {} >>\nstream\n{}\nendstream\nendobj\n\
         7 0 obj\n<< /Type /Annot /Subtype /Popup /Rect [40 20 90 60] /Parent 5 0 R >>\nendobj\n",
        appearance.len().saturating_add(1),
        appearance
    );

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
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

#[test]
fn linked_popup_has_no_page_mark_while_its_stored_ap_parent_paints() {
    let document = Document::open(fixture()).expect("complete PDF parses");
    assert!(!document.was_recovered(), "fixture xref is authoritative");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "supported parent AP is complete: {:?}",
        interpretation.unsupported
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, 1 << 20)
        .expect("small page target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpretation.display_list, target)
        .expect("CPU rasterizer supports this stored form");

    // PDF coordinates map to top-down raster rows. These are deep interior points, not edges.
    let sample = |x: u32, y: u32| {
        let row = raster.height.saturating_sub(1).saturating_sub(y);
        let offset =
            ((row.saturating_mul(raster.width).saturating_add(x)) as usize).saturating_mul(4);
        (
            raster.data[offset],
            raster.data[offset + 1],
            raster.data[offset + 2],
            raster.data[offset + 3],
        )
    };
    assert_eq!(
        sample(20, 80),
        (255, 0, 0, 255),
        "positive stored-AP control"
    );
    assert_eq!(
        sample(60, 40).3,
        0,
        "Popup rectangle has no page appearance"
    );
}
