//! Screen and print flag interaction, plus the explicit export policy from ADR 1179.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

use std::fmt::Write as _;

use pdf_model::optional_content::Purpose;
use pdf_model::view::{Pointer, ViewState};
use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::{Document, ObjectId};
use render_cpu::CpuRasterizer;

const LIMIT: u64 = 1 << 20;
const ANNOTATION: ObjectId = ObjectId {
    number: 5,
    generation: 0,
};

fn pdf(flags: i64, subtype: &str) -> Vec<u8> {
    let appearance = "1 0 0 rg 0 0 80 80 re f";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /{subtype} /Rect [10 10 90 90] /F {flags} \
         /AP << /N 6 0 R >> >>\nendobj\n\
         6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 80 80] /Length {} >>\n\
         stream\n{appearance}\nendstream\nendobj\n",
        appearance.len() + 1
    );
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(bytes.len());
        bytes.push_str(object);
    }
    let xref = bytes.len();
    let size = offsets.len() + 1;
    let _ = writeln!(bytes, "xref\n0 {size}");
    bytes.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    bytes.into_bytes()
}

fn painted(flags: i64, subtype: &str, purpose: Purpose, pointer: Option<Pointer>) -> bool {
    let document = Document::open(pdf(flags, subtype)).expect("valid exact-xref PDF");
    assert!(!document.was_recovered(), "xref offsets are exact");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let mut view = ViewState::of(&document);
    view.set_purpose(purpose);
    view.set_pointer(pointer.map(|pointer| (ANNOTATION, pointer)));
    let interpretation = pdf_model::content::interpret_with(&document, &page, &view);
    assert!(
        interpretation.is_complete(),
        "supported standard appearance: {:?}",
        interpretation.unsupported
    );
    let target =
        TargetSpec::for_page(&interpretation.display_list, 1.0, LIMIT).expect("bounded target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpretation.display_list, target)
        .expect("supported raster");
    assert_eq!((raster.width, raster.height), (100, 100));
    let pixel = &raster.data[20_200..20_204];
    assert!(
        pixel == [255, 0, 0, 255] || pixel == [0, 0, 0, 0],
        "{pixel:?}"
    );
    assert_eq!(&raster.data[..4], [0, 0, 0, 0], "outside annotation");
    pixel == [255, 0, 0, 255]
}

#[test]
fn print_ignores_pointer_toggling_while_screen_and_export_use_no_view() {
    // Positive control: an ordinary annotation is visible in all three output purposes.
    for purpose in [Purpose::View, Purpose::Print, Purpose::Export] {
        assert!(painted(4, "Square", purpose, None));
    }

    // With NoView+ToggleNoView (F=292), the screen hides at rest and reveals under the pointer.
    // The paper ignores the pointer reading and uses Print. Export follows the explicit
    // ADR 1179 policy of the screen column; Table 167 itself specifies screen and print only.
    assert!(!painted(292, "Square", Purpose::View, None));
    assert!(painted(292, "Square", Purpose::View, Some(Pointer::Over)));
    assert!(painted(292, "Square", Purpose::Print, None));
    assert!(painted(292, "Square", Purpose::Print, Some(Pointer::Over)));
    assert!(!painted(292, "Square", Purpose::Export, None));

    // ToggleNoView alone (F=260) gives the inverse screen XOR, while Print still does not
    // consult pointer state. Export again follows screen semantics.
    assert!(painted(260, "Square", Purpose::View, None));
    assert!(!painted(260, "Square", Purpose::View, Some(Pointer::Over)));
    assert!(painted(260, "Square", Purpose::Print, None));
    assert!(painted(260, "Square", Purpose::Print, Some(Pointer::Over)));
    assert!(painted(260, "Square", Purpose::Export, None));
}
