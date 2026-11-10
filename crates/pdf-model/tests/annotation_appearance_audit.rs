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
    appearance_pdf_over("", appearance_dictionary, streams)
}

fn appearance_pdf_over(
    page_content: &str,
    appearance_dictionary: &str,
    streams: &[(&str, &str)],
) -> Vec<u8> {
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> \
          /Contents 4 0 R /Annots [5 0 R] >>"
            .to_vec(),
        format!(
            "<< /Length {} >>\nstream\n{page_content}\nendstream",
            page_content.len()
        )
        .into_bytes(),
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

fn sample_rgb(
    bytes: Vec<u8>,
    purpose: Purpose,
    pointer: Option<Pointer>,
    x: u32,
    y: u32,
) -> (u8, u8, u8) {
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
    let row = raster.height.saturating_sub(1).saturating_sub(y);
    let offset = ((row * raster.width + x) as usize) * 4;
    (
        raster.data[offset],
        raster.data[offset + 1],
        raster.data[offset + 2],
    )
}

fn centre_rgb(bytes: Vec<u8>, purpose: Purpose, pointer: Option<Pointer>) -> (u8, u8, u8) {
    sample_rgb(bytes, purpose, pointer, 50, 50)
}

#[test]
fn an_appearance_without_group_equals_the_explicit_default_and_group_flags_change_pixels() {
    let page = "0 0 1 rg 0 0 100 100 re f";
    // Red multiplied with the non-isolated group's blue initial backdrop is black. With an
    // isolated group, the element first sees transparency and remains red.
    let content = "1 0 0 rg /G1 gs 10 10 80 80 re f";
    let resources = "/Resources << /ExtGState << /G1 << /BM /Multiply >> >> >>";
    let missing_group = appearance_pdf_over(page, "<< /N 6 0 R >>", &[(resources, content)]);
    let explicit_default_group = appearance_pdf_over(
        page,
        "<< /N 6 0 R >>",
        &[(
            &format!("/Group << /S /Transparency /I false /K false >> {resources}"),
            content,
        )],
    );
    let isolated_control = appearance_pdf_over(
        page,
        "<< /N 6 0 R >>",
        &[(
            &format!("/Group << /S /Transparency /I true /K false >> {resources}"),
            content,
        )],
    );

    let missing = centre_rgb(missing_group, Purpose::View, None);
    let explicit = centre_rgb(explicit_default_group, Purpose::View, None);
    let isolated = centre_rgb(isolated_control, Purpose::View, None);
    assert_eq!(
        missing, explicit,
        "a missing /Group uses its stated default"
    );
    assert_ne!(
        missing, isolated,
        "the /I true control changes the backdrop seen by Multiply"
    );
    assert_eq!(
        missing,
        (0, 0, 0),
        "non-isolated Multiply sees the blue page backdrop"
    );
    assert_eq!(
        isolated,
        (255, 0, 0),
        "isolated Multiply starts on transparency"
    );
}

#[test]
fn an_appearance_group_knockout_changes_only_the_overlapping_element_result() {
    let page = "0 0 1 rg 0 0 100 100 re f";
    let resources = "/Resources << /ExtGState << /G0 << /ca 0.5 >> >> >>";
    let content = "/G0 gs 1 0 0 rg 10 10 50 50 re f 0 1 0 rg 40 40 50 50 re f";
    let draw = |knockout: bool| {
        let group = format!("/Group << /S /Transparency /I true /K {knockout} >> {resources}");
        appearance_pdf_over(page, "<< /N 6 0 R >>", &[(group.as_str(), content)])
    };

    let knockout = draw(true);
    let stacked = draw(false);
    let missing = appearance_pdf_over(page, "<< /N 6 0 R >>", &[(resources, content)]);
    let default_group = format!("/Group << /S /Transparency /I false /K false >> {resources}");
    let explicit_default =
        appearance_pdf_over(page, "<< /N 6 0 R >>", &[(default_group.as_str(), content)]);
    assert_eq!(
        sample_rgb(missing.clone(), Purpose::View, None, 50, 50),
        sample_rgb(explicit_default, Purpose::View, None, 50, 50),
        "the absent Group also defaults K to false where elements overlap"
    );
    assert_ne!(
        sample_rgb(missing, Purpose::View, None, 50, 50),
        sample_rgb(knockout.clone(), Purpose::View, None, 50, 50),
        "the absent default is observably non-knockout"
    );
    let green_alone = sample_rgb(knockout.clone(), Purpose::View, None, 70, 70);
    assert_eq!(
        sample_rgb(knockout, Purpose::View, None, 50, 50),
        green_alone,
        "/K true composites the later element with the group's initial backdrop"
    );
    assert_ne!(
        sample_rgb(stacked.clone(), Purpose::View, None, 50, 50),
        green_alone,
        "the non-knockout control composites over the earlier red element"
    );
    assert_eq!(
        sample_rgb(stacked, Purpose::View, None, 70, 70),
        green_alone,
        "the control is identical outside the overlap"
    );
}

fn appearance_pdf_with_state(
    appearance_dictionary: &str,
    state: &str,
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
            "<< /Type /Annot /Subtype /Square /Rect [10 10 90 90] /F 4 /AS /{state} \
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

#[test]
fn rollover_and_down_state_subdictionaries_select_the_state_named_by_as() {
    let ap = "<< /N 6 0 R /R << /On 7 0 R /Off 8 0 R >> \
             /D << /On 9 0 R /Off 10 0 R >> >>";
    let streams = [
        ("", "1 0 0 rg 0 0 80 80 re f"),
        ("", "0 1 0 rg 0 0 80 80 re f"),
        ("", "1 1 0 rg 0 0 80 80 re f"),
        ("", "0 0 1 rg 0 0 80 80 re f"),
        ("", "1 0 1 rg 0 0 80 80 re f"),
    ];
    let on = appearance_pdf_with_state(ap, "On", &streams);
    let off = appearance_pdf_with_state(ap, "Off", &streams);

    assert_eq!(
        centre_rgb(on.clone(), Purpose::View, None),
        (255, 0, 0),
        "/N remains a stream"
    );
    assert_eq!(
        centre_rgb(on.clone(), Purpose::View, Some(Pointer::Over)),
        (0, 255, 0),
        "/AS /On selects /R /On"
    );
    assert_eq!(
        centre_rgb(on, Purpose::View, Some(Pointer::Down)),
        (0, 0, 255),
        "/AS /On selects /D /On"
    );
    assert_eq!(
        centre_rgb(off.clone(), Purpose::View, Some(Pointer::Over)),
        (255, 255, 0),
        "/AS /Off selects /R /Off"
    );
    assert_eq!(
        centre_rgb(off, Purpose::View, Some(Pointer::Down)),
        (255, 0, 255),
        "/AS /Off selects /D /Off"
    );
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
