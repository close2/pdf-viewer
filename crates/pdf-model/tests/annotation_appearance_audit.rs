//! Complete-PDF consumer tests for §12.5.5 Table 170 appearance selection.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "the controlled fixtures should fail loudly if malformed or indexed incorrectly"
)]

use pdf_model::optional_content::Purpose;
use pdf_model::view::{Pointer, ViewState};
use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::{Document, ObjectId};
use render_cpu::CpuRasterizer;

fn assemble(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut output = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (number, object) in (1_usize..).zip(objects) {
        offsets.push(output.len());
        output.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
        output.extend_from_slice(object);
        output.extend_from_slice(b"\nendobj\n");
    }
    let xref = output.len();
    let size = offsets.len() + 1;
    output.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets {
        output.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    output.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    output
}

fn stream(dictionary: &str, content: &str) -> Vec<u8> {
    format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 80 80] {dictionary} /Length {} >>\nstream\n{content}\nendstream",
        content.len()
    )
    .into_bytes()
}

fn appearance_pdf(appearance_dictionary: &str, streams: &[(&str, &str)]) -> Vec<u8> {
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> \
          /Contents 4 0 R /Annots [5 0 R] >>"
            .to_vec(),
        b"<< /Length 0 >>\nstream\n\nendstream".to_vec(),
        format!(
            "<< /Type /Annot /Subtype /Square /Rect [10 10 90 90] /F 4 \
             /AP {appearance_dictionary} >>"
        )
        .into_bytes(),
    ];
    objects.extend(
        streams
            .iter()
            .map(|(dictionary, content)| stream(dictionary, content)),
    );
    assemble(&objects)
}

fn centre_rgb(bytes: Vec<u8>, purpose: Purpose, pointer: Option<Pointer>) -> (u8, u8, u8) {
    let document = Document::open(bytes).expect("complete PDF fixture parses");
    assert!(!document.was_recovered(), "fixture uses exact xref offsets");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let annotation = ObjectId {
        number: 5,
        generation: 0,
    };
    let mut view = ViewState::of(&document);
    view.set_purpose(purpose);
    if let Some(pointer) = pointer {
        view.set_pointer(Some((annotation, pointer)));
    }
    let interpretation = pdf_model::content::interpret_with(&document, &page, &view);
    assert!(
        interpretation.unsupported.is_empty(),
        "valid standard annotation appearance is supported: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, 1 << 20).expect("bounded target");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported raster");
    let offset = ((50 * raster.width + 50) as usize) * 4;
    (
        raster.data[offset],
        raster.data[offset + 1],
        raster.data[offset + 2],
    )
}

#[test]
fn explicit_rollover_is_selected_and_missing_down_defaults_to_normal() {
    let bytes = appearance_pdf(
        "<< /N 6 0 R /R 7 0 R >>",
        &[
            ("", "1 0 0 rg 0 0 80 80 re f"),
            ("", "0 1 0 rg 0 0 80 80 re f"),
        ],
    );
    assert_eq!(
        centre_rgb(bytes.clone(), Purpose::View, None),
        (255, 0, 0),
        "idle annotation uses /N"
    );
    assert_eq!(
        centre_rgb(bytes.clone(), Purpose::View, Some(Pointer::Over)),
        (0, 255, 0),
        "active-area hover uses the explicit /R stream"
    );
    assert_eq!(
        centre_rgb(bytes, Purpose::View, Some(Pointer::Down)),
        (255, 0, 0),
        "Table 170 defaults absent /D to /N, not to nothing or /R"
    );
}

#[test]
fn printing_uses_normal_even_if_the_view_state_carries_a_pressed_pointer() {
    let bytes = appearance_pdf(
        "<< /N 6 0 R /R 7 0 R /D 8 0 R >>",
        &[
            ("", "1 0 0 rg 0 0 80 80 re f"),
            ("", "0 1 0 rg 0 0 80 80 re f"),
            ("", "0 0 1 rg 0 0 80 80 re f"),
        ],
    );
    assert_eq!(
        centre_rgb(bytes, Purpose::Print, Some(Pointer::Down)),
        (255, 0, 0),
        "§12.5.5 expressly assigns /N to printing, independent of interactive pointer state"
    );
}
