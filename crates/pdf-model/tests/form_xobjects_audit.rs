//! Pixel oracles for §8.10.1 form invocation state and §8.10.2 /Matrix mapping.
//! Complete PDF 1.7 fixtures with exact stream lengths and xref offsets; including reference targets selected by label.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "small exact fixture and pixel indexing; failures should be loud"
)]

use std::fmt::Write as _;

use pdf_model::reference::Supply;
use pdf_render::{Rasterizer as _, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const PIXEL_BUDGET: u64 = 1 << 20;

fn stream(number: usize, dict: &str, body: &str) -> Vec<u8> {
    let data = body.as_bytes();
    let mut bytes = format!(
        "{number} 0 obj\n<< {dict} /Length {} >>\nstream\n",
        data.len()
    )
    .into_bytes();
    bytes.extend_from_slice(data);
    bytes.extend_from_slice(b"\nendstream\nendobj\n");
    bytes
}

fn fixture(form_dict: &str, form_body: &str, page_body: &str) -> Vec<u8> {
    let objects = vec![
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /XObject << /Fm 5 0 R >> >> /Contents 4 0 R >>\nendobj\n".to_vec(),
        stream(4, "", page_body),
        stream(
            5,
            &format!(
                "/Type /XObject /Subtype /Form /BBox [0 0 100 100] /Resources << >> {form_dict}"
            ),
            form_body,
        ),
    ];
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = vec![0usize];
    for object in objects {
        offsets.push(bytes.len());
        bytes.extend_from_slice(&object);
    }
    let xref = bytes.len();
    let mut trailer = String::new();
    let _ = writeln!(trailer, "xref\n0 {}", offsets.len());
    trailer.push_str("0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        let _ = writeln!(trailer, "{offset:010} 00000 n ");
    }
    let _ = write!(
        trailer,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        offsets.len()
    );
    bytes.extend_from_slice(trailer.as_bytes());
    bytes
}

fn render(bytes: Vec<u8>) -> pdf_render::Raster {
    let document = Document::open(bytes).expect("complete PDF opens");
    assert!(!document.was_recovered(), "fixture has exact xref offsets");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("one page in the page tree");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "valid form fixture is supported: {:?}",
        interpretation.unsupported
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, PIXEL_BUDGET)
        .expect("100x100 page fits the budget");
    CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .expect("supported display list rasterizes")
}

fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * raster.width + x) * 4) as usize;
    [
        raster.data[at],
        raster.data[at + 1],
        raster.data[at + 2],
        raster.data[at + 3],
    ]
}

#[test]
fn a_form_restores_the_callers_graphics_state_after_painting() {
    // §8.10.1 steps a/e save and restore graphics state. Form's blue fill must not leak into the
    // following caller path, whose red color is set before Do. Upper/lower Y are accounted for by
    // sampling the center of each painted rectangle.
    let raster = render(fixture(
        "",
        "0 0 1 rg 0 0 50 50 re f",
        "1 0 0 rg /Fm Do 50 0 50 50 re f",
    ));
    assert_eq!(
        pixel(&raster, 25, 75),
        [0, 0, 255, 255],
        "form's own blue mark"
    );
    assert_eq!(
        pixel(&raster, 75, 75),
        [255, 0, 0, 255],
        "caller state restored"
    );
}

#[test]
fn a_forms_matrix_maps_form_space_before_its_content_is_painted() {
    // Table 93's Matrix maps form space into user space; a 20-unit translation places the
    // form's red square at x=20..40. The blank point lies inside the untransformed location.
    let raster = render(fixture(
        "/Matrix [1 0 0 1 20 0]",
        "1 0 0 rg 0 0 20 20 re f",
        "/Fm Do",
    ));
    assert_eq!(
        pixel(&raster, 30, 90),
        [255, 0, 0, 255],
        "translated form mark"
    );
    assert_eq!(
        pixel(&raster, 10, 90),
        [255, 255, 255, 255],
        "old location stays blank"
    );
}

#[test]
fn repeated_form_paints_repeat_the_form_marks_at_each_invocation() {
    // §8.10.1 says a shared form produces the same results at each paint, subject to invocation
    // graphics state. Here only the caller CTM translates the second invocation, so corresponding
    // pixels must match; the outside sample controls against an accidentally unbounded mark.
    let raster = render(fixture(
        "",
        "1 0 0 rg 0 0 20 20 re f",
        "/Fm Do q 1 0 0 1 50 0 cm /Fm Do Q",
    ));
    assert_eq!(pixel(&raster, 10, 90), [255, 0, 0, 255]);
    assert_eq!(pixel(&raster, 60, 90), [255, 0, 0, 255]);
    assert_eq!(pixel(&raster, 35, 90), [255, 255, 255, 255]);
}

fn assemble_pdf(objects: &[Vec<u8>], trailer_id: &str) -> Vec<u8> {
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = vec![0usize];
    for object in objects {
        offsets.push(bytes.len());
        bytes.extend_from_slice(object);
    }
    let xref = bytes.len();
    let mut trailer = String::new();
    let _ = writeln!(trailer, "xref\n0 {}", offsets.len());
    trailer.push_str("0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        let _ = writeln!(trailer, "{offset:010} 00000 n ");
    }
    let _ = write!(
        trailer,
        "trailer\n<< /Size {} /Root 1 0 R /ID [{trailer_id}] >>\nstartxref\n{xref}\n%%EOF\n",
        offsets.len()
    );
    bytes.extend_from_slice(trailer.as_bytes());
    bytes
}

const LABEL_TARGET_ID: (&str, &str) = (
    "00112233445566778899aabbccddeeff",
    "00112233445566778899aabbccddeeff",
);
const LABEL_PROXY_ID: &str =
    "<11112222333344445555666677778888> <11112222333344445555666677778888>";

fn label_reference_target() -> Vec<u8> {
    let first = "0 1 0 rg 0 0 10 10 re f";
    let second = "0 0 1 rg 0 0 10 10 re f";
    let objects = vec![
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /PageLabels 5 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R 6 0 R] /Count 2 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << >> /Contents 4 0 R >>\nendobj\n".to_vec(),
        stream(4, "", first),
        b"5 0 obj\n<< /Nums [0 << /S /D >> 1 << /P (target) >>] >>\nendobj\n".to_vec(),
        b"6 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << >> /Contents 7 0 R >>\nendobj\n".to_vec(),
        stream(7, "", second),
    ];
    assemble_pdf(
        &objects,
        &format!("<{}> <{}>", LABEL_TARGET_ID.0, LABEL_TARGET_ID.1),
    )
}

fn label_reference_proxy(page: &str) -> Vec<u8> {
    let proxy = "1 0 0 rg 0 0 10 10 re f";
    let reference = format!(
        "/Ref << /F << /Type /Filespec /F (target.pdf) /UF (target.pdf) >> \
         /Page {page} /ID [<{}> <{}>] >>",
        LABEL_TARGET_ID.0, LABEL_TARGET_ID.1
    );
    let objects = vec![
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << /XObject << /Fm 5 0 R >> >> /Contents 4 0 R >>\nendobj\n".to_vec(),
        stream(4, "", "/Fm Do"),
        stream(
            5,
            &format!(
                "/Type /XObject /Subtype /Form /BBox [0 0 10 10] /Resources << >> {reference}"
            ),
            proxy,
        ),
    ];
    assemble_pdf(&objects, LABEL_PROXY_ID)
}

fn render_label_reference(page: &str, supply: &Supply) -> pdf_model::Interpretation {
    let document =
        Document::open(label_reference_proxy(page)).expect("complete containing PDF opens");
    assert!(!document.was_recovered(), "fixture has exact xref offsets");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("one containing page");
    pdf_model::content::interpret_importing(
        &document,
        &page,
        &pdf_model::view::ViewState::of(&document),
        &pdf_model::FontCache::new(),
        supply,
    )
}

#[test]
fn a_reference_page_label_selects_the_target_page() {
    // Table 95 permits a text page label, not just the zero-based index exercised by the
    // existing reference-XObject suite. The target's PageLabels number tree gives page index 1 the
    // literal label "target"; index 0 is green and index 1 is blue.
    let target = label_reference_target();
    let (supply, refused) = Supply::read([("target.pdf", target)], "draft page-label fixture");
    assert!(
        refused.is_empty(),
        "target fixture is a readable PDF: {refused:?}"
    );
    assert_eq!(supply.len(), 1);
    let interpretation = render_label_reference("(target)", &supply);
    assert!(
        interpretation.is_complete(),
        "target label resolves: {:?}",
        interpretation.unsupported
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, PIXEL_BUDGET)
        .expect("10x10 page fits the budget");
    let raster = CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .expect("supported display list rasterizes");
    assert_eq!(
        pixel(&raster, 5, 5),
        [0, 0, 255, 255],
        "page label exists only on page index 1: blue second page replaces red proxy, not green page 0"
    );
}

#[test]
fn a_reference_numeric_page_zero_selects_the_first_target_page() {
    let (supply, refused) = Supply::read(
        [("target.pdf", label_reference_target())],
        "numeric page control",
    );
    assert!(
        refused.is_empty(),
        "target fixture is readable: {refused:?}"
    );
    let interpretation = render_label_reference("0", &supply);
    assert!(
        interpretation.is_complete(),
        "numeric page resolves: {:?}",
        interpretation.unsupported
    );
    assert_eq!(
        pixel(&rasterize(&interpretation), 5, 5),
        [0, 255, 0, 255],
        "index 0 selects the green first page, not the blue labeled page"
    );
}

#[test]
fn a_reference_without_a_supplied_target_uses_its_red_proxy() {
    let interpretation = render_label_reference("(target)", &Supply::NONE);
    assert!(
        interpretation.is_complete(),
        "unavailable target is a specified proxy case: {:?}",
        interpretation.unsupported
    );
    assert_eq!(
        pixel(&rasterize(&interpretation), 5, 5),
        [255, 0, 0, 255],
        "the proxy is red; neither target page was supplied"
    );
}

fn rasterize(interpretation: &pdf_model::Interpretation) -> pdf_render::Raster {
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, PIXEL_BUDGET)
        .expect("small page fits budget");
    CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .expect("supported fixture rasterizes")
}

#[test]
fn form_matrix_is_concatenated_before_the_callers_scaled_ctm() {
    // §8.10.1: a form-space translation of 20 is transformed by the caller's x scale 2.
    // Reversing the composition paints x=20..40 instead of x=40..60.
    let raster = render(fixture(
        "/Matrix [1 0 0 1 20 0]",
        "1 0 0 rg 0 0 10 10 re f",
        "2 0 0 1 0 0 cm /Fm Do",
    ));
    assert_eq!(pixel(&raster, 50, 95), [255, 0, 0, 255]);
    assert_eq!(pixel(&raster, 30, 95), [255, 255, 255, 255]);
}

#[test]
fn a_form_inherits_the_colour_present_at_each_invocation() {
    // §8.10.1: initial state comes from the caller except the stated Matrix/BBox changes.
    let raster = render(fixture(
        "",
        "0 0 20 20 re f",
        "1 0 0 rg /Fm Do 0 0 1 rg 1 0 0 1 50 0 cm /Fm Do",
    ));
    assert_eq!(pixel(&raster, 10, 90), [255, 0, 0, 255]);
    assert_eq!(pixel(&raster, 60, 90), [0, 0, 255, 255]);
}
