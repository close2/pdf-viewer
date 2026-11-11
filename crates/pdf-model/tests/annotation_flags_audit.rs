//! Table 167 visibility and interaction controls through public model consumers.
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

fn appearance_pdf(
    subtype: &str,
    flags: u32,
    appearance_dictionary: &str,
    streams: &[(&str, &str)],
) -> Vec<u8> {
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> \
          /Contents 4 0 R /Annots [5 0 R] >>"
            .to_vec(),
        b"<< /Length 0 >>\nstream\n\nendstream".to_vec(),
        format!(
            "<< /Type /Annot /Subtype /{subtype} /Rect [10 10 90 90] /F {flags} /IC [1 0 0] /AS /Off \
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

fn fixture(subtype: &str, flags: u32) -> Vec<u8> {
    appearance_pdf(
        subtype,
        flags,
        "<< /N 6 0 R >>",
        &[("", "1 0 0 rg 0 0 80 80 re f")],
    )
}

#[test]
fn annotation_hit_testing_respects_readonly_and_toggle_no_view() {
    let id = ObjectId {
        number: 5,
        generation: 0,
    };
    for (subtype, flags, expected) in [
        ("Square", 4, true),
        ("Square", 68, false),
        ("Square", 36, false),
        ("Square", 292, true),
        ("Square", 6, false),
        ("Widget", 68, true),
    ] {
        let document = Document::open(fixture(subtype, flags)).expect("complete fixture");
        assert!(!document.was_recovered());
        let page = pdf_model::Pages::new(&document).get(0).expect("one page");
        let view = ViewState::of(&document);
        let hit = pdf_model::view::annotation_at(&document, &page, &view, 50.0, 50.0);
        assert_eq!(hit, expected.then_some(id), "{subtype} flags {flags}");
        assert_eq!(
            pdf_model::view::annotation_at(&document, &page, &view, 5.0, 5.0),
            None,
            "outside control"
        );
    }
    // ReadOnly does not itself suppress the visible annotation.
    assert_eq!(
        centre_rgb(fixture("Square", 68), Purpose::View, None),
        (255, 0, 0)
    );
    assert_eq!(
        centre_rgb(fixture("Square", 68), Purpose::Print, None),
        (255, 0, 0)
    );
}

#[test]
fn invisible_suppresses_unknown_subtype_even_when_print_is_set() {
    for purpose in [Purpose::View, Purpose::Print] {
        assert_eq!(
            centre_rgb(fixture("NotInTable171", 4), purpose, None),
            (255, 0, 0),
            "clear bit permits unknown normal AP"
        );
        assert_eq!(
            centre_rgb(fixture("NotInTable171", 5), purpose, None),
            (0, 0, 0),
            "Invisible suppresses unknown AP"
        );
        assert_eq!(
            centre_rgb(fixture("Square", 5), purpose, None),
            (255, 0, 0),
            "Invisible does not apply to standard Square"
        );
    }
}

#[test]
fn an_unselected_rollover_state_stream_still_makes_the_print_flag_apply() {
    let streams = [("", "0 1 0 rg 0 0 80 80 re f")];
    // Invalid N allows this reader's red constructed Square fallback. The control proves the
    // fallback paints, so absence of paint with R present cannot be explained by undefined AS.
    let no_stream = appearance_pdf("Square", 0, "<< /N 42 >>", &streams);
    assert_eq!(centre_rgb(no_stream, Purpose::Print, None), (255, 0, 0));
    let contained = "<< /N 42 /R << /On 6 0 R >> >>";
    let screen = appearance_pdf("Square", 0, contained, &streams);
    assert_eq!(centre_rgb(screen, Purpose::View, None), (255, 0, 0));
    let paper = appearance_pdf("Square", 0, contained, &streams);
    assert_eq!(
        centre_rgb(paper, Purpose::Print, None),
        (0, 0, 0),
        "R contains a stream even though no pointer selects it and AS names Off"
    );
    let permitted = appearance_pdf("Square", 4, contained, &streams);
    assert_eq!(centre_rgb(permitted, Purpose::Print, None), (255, 0, 0));
}
