//! Consumer regressions for §11.6.5.1 soft-mask coordinates and §11.5.3 group kinds.
//! Independent pixel expectations distinguish matrix order and mask-group knockout.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "fixture construction and known in-bounds raster samples"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const GENEROUS: u64 = 1 << 20;

#[test]
fn a_mask_applies_its_form_matrix_before_the_ctm_at_gs() {
    // The form's Matrix doubles x; the CTM already in force at /gs translates by 10.
    // Application order therefore maps the group's white x=[0,10] region to page x=[10,30].
    // Ignoring Matrix would put it at [10,20], and applying it after the CTM would put it at
    // [20,40]. Samples distinguish all three placements.
    let bytes = pdf(
        "/SMask << /Type /Mask /S /Luminosity /G 6 0 R /BC [0] >>",
        "q 1 0 0 1 10 0 cm /GS gs 0 g -10 0 40 40 re f Q",
        "/Matrix [2 0 0 1 0 0]",
        "/BBox [0 0 20 40] /Group << /Type /Group /S /Transparency /CS /DeviceGray >>",
        "1 g 0 0 10 40 re f",
    );
    let raster = render_complete(bytes);

    near_gray(&raster, 5, 20, 255);
    near_gray(&raster, 15, 20, 0);
    near_gray(&raster, 25, 20, 0);
    near_gray(&raster, 35, 20, 255);
}

#[test]
fn a_luminosity_soft_mask_respects_a_knockout_group_against_normal_control() {
    // Normal control: opaque 0.5 gray, followed by white at alpha .5, yields .75 in overlap.
    // In a knockout group the second rectangle's geometric shape removes the gray beneath it,
    // then white at .5 over the group's black backdrop yields .5. A black page mark over white
    // therefore leaves about 64 in the normal control and 128 in the knockout overlap.
    let normal = render_complete(group_kind_pdf(false));
    near_gray(&normal, 5, 20, 128);
    near_gray(&normal, 15, 20, 64);
    near_gray(&normal, 25, 20, 128);
    near_gray(&normal, 35, 20, 255);

    let (knockout, complete) = render(group_kind_pdf(true));
    // Check unaffected regions before the overlap assertion that exposes the defect.
    near_gray(&knockout, 5, 20, 128);
    near_gray(&knockout, 25, 20, 128);
    near_gray(&knockout, 35, 20, 255);
    near_gray(&knockout, 15, 20, 128);
    assert!(
        complete,
        "a supported valid knockout group should not be reported"
    );
}

fn group_kind_pdf(knockout: bool) -> Vec<u8> {
    let flag = if knockout { "true" } else { "false" };
    pdf(
        "/SMask << /Type /Mask /S /Luminosity /G 6 0 R /BC [0] >>",
        "q /GS gs 0 g 0 0 40 40 re f Q",
        "",
        &format!(
            "/BBox [0 0 30 40] /Resources << /ExtGState << /Half 7 0 R >> >> \
             /Group << /Type /Group /S /Transparency /CS /DeviceGray /I true /K {flag} >>"
        ),
        "0.5 g 0 0 20 40 re f /Half gs 1 g 10 0 20 40 re f",
    )
}

fn pdf(
    soft_mask: &str,
    page_content: &str,
    matrix: &str,
    form_dictionary: &str,
    group_content: &str,
) -> Vec<u8> {
    let group_resources = if form_dictionary.contains("/Resources") {
        ""
    } else {
        "/Resources << >>"
    };
    let form_dictionary = format!("{matrix} {form_dictionary} {group_resources}");
    assemble(&[
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
          /Resources << /ExtGState << /GS 5 0 R >> >> /Contents 4 0 R >>\nendobj\n"
            .to_vec(),
        stream(4, "", page_content.as_bytes()),
        format!("5 0 obj\n<< /Type /ExtGState {soft_mask} >>\nendobj\n").into_bytes(),
        stream(
            6,
            &format!("/Type /XObject /Subtype /Form {form_dictionary}"),
            group_content.as_bytes(),
        ),
        b"7 0 obj\n<< /Type /ExtGState /ca 0.5 >>\nendobj\n".to_vec(),
    ])
}

fn render_complete(bytes: Vec<u8>) -> pdf_render::Raster {
    let (raster, complete) = render(bytes);
    assert!(
        complete,
        "valid control fixture should draw without reports"
    );
    raster
}

fn render(bytes: Vec<u8>) -> (pdf_render::Raster, bool) {
    let document = Document::open(bytes).expect("complete PDF fixture opens");
    assert!(!document.was_recovered(), "fixture has a correct xref");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("one page exists");
    let interpretation = pdf_model::interpret(&document, &page);
    let complete = interpretation.is_complete();
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, GENEROUS)
        .expect("40-unit page at one pixel per unit is a valid target");
    let raster = CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .expect("supported complete fixture");
    (raster, complete)
}

fn stream(number: usize, dictionary: &str, data: &[u8]) -> Vec<u8> {
    let mut object = format!(
        "{number} 0 obj\n<< {dictionary} /Length {} >>\nstream\n",
        data.len()
    )
    .into_bytes();
    object.extend_from_slice(data);
    object.extend_from_slice(b"\nendstream\nendobj\n");
    object
}

fn assemble(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut output = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for object in objects {
        offsets.push(output.len());
        output.extend_from_slice(object);
    }
    let xref = output.len();
    let size = offsets.len() + 1;
    let mut trailer = String::new();
    let _ = writeln!(trailer, "xref\n0 {size}");
    trailer.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(trailer, "{offset:010} 00000 n ");
    }
    let _ = write!(
        trailer,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    output.extend_from_slice(trailer.as_bytes());
    output
}

fn near_gray(raster: &pdf_render::Raster, x: u32, y: u32, expected: u8) {
    assert!(x < raster.width && y < raster.height);
    let row = raster.height - 1 - y;
    let offset = usize::try_from(row * raster.width + x).expect("small fixture") * 4;
    for channel in &raster.data[offset..offset + 3] {
        assert!(
            channel.abs_diff(expected) <= 1,
            "({x}, {y}): {channel} versus {expected}"
        );
    }
    assert_eq!(raster.data[offset + 3], 255, "opaque white page backdrop");
}
