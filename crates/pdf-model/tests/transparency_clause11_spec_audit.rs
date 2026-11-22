//! Specification audit of ISO 32000-2 clause 11 (transparency): one failing test per behaviour
//! finding of `doc/reviews/2171-specification-audit-11.md`, each beside a control that passes on
//! main.
//!
//! Every fixture is a 100 by 100 page drawn by `render-cpu`, the oracle, onto its white medium,
//! and read at its centre. A soft mask of one half is a `/Luminosity` mask whose `DeviceRGB`
//! group fills the page with `0.5 g`: §11.5.3's device formula gives 0.5, so a red fill under it
//! reads (255, 127, 127) and one without it (255, 0, 0).

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "the complete tiny fixture PDF and fixed raster extent are test preconditions"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;

fn assemble(body: &str) -> Vec<u8> {
    let mut output = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(output.len());
        output.push_str(object);
    }
    let xref_at = output.len();
    let size = offsets.len() + 1;
    let _ = writeln!(output, "xref\n0 {size}");
    output.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(output, "{offset:010} 00000 n ");
    }
    let _ = write!(
        output,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    output.into_bytes()
}

fn stream(number: u32, dict: &str, content: &str) -> String {
    format!(
        "{number} 0 obj\n<< {dict} /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
        content.len() + 1
    )
}

/// One page: `group` is the page's `/Group` entry, `resources` the inside of its resource
/// dictionary, `content` its stream, and `extra` the objects numbered from 5.
fn page(group: &str, resources: &str, content: &str, extra: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] {group} \
         /Resources << {resources} >> /Contents 4 0 R >>\nendobj\n{}{extra}",
        stream(4, "", content)
    );
    assemble(&body)
}

/// The centre pixel `render-cpu` draws, and what the interpretation reported.
fn centre(bytes: Vec<u8>) -> ([u8; 4], Vec<String>) {
    let document = Document::open(bytes).expect("complete fixture PDF");
    assert!(!document.was_recovered(), "exact source xref");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("one fixture page");
    let interpretation = pdf_model::interpret(&document, &page);
    let list = &interpretation.display_list;
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("100x100 page target");
    let mut renderer = render_cpu::CpuRasterizer::new();
    let raster = renderer
        .rasterize(list, target)
        .expect("fixture rasterizes");
    let offset = ((50_u32 * raster.width + 50) * 4) as usize;
    let pixel = [
        raster.data[offset],
        raster.data[offset + 1],
        raster.data[offset + 2],
        raster.data[offset + 3],
    ];
    let reports = interpretation
        .unsupported
        .iter()
        .map(|report| format!("{report:?}"))
        .collect();
    (pixel, reports)
}

fn assert_near(actual: [u8; 4], expected: [u8; 4], reason: &str) {
    for (got, want) in actual.iter().zip(expected) {
        assert!(
            got.abs_diff(want) <= 3,
            "{reason}: got {actual:?}, expected {expected:?} within raster rounding"
        );
    }
}

const RGB_PAGE: &str = "/Group << /Type /Group /S /Transparency /CS /DeviceRGB >>";

const HALF_MASK: &str =
    "/M << /Type /ExtGState /SMask << /Type /Mask /S /Luminosity /G 5 0 R >> >>";

fn half_mask_group(group: &str) -> String {
    stream(
        5,
        &format!("/Type /XObject /Subtype /Form /BBox [0 0 100 100] /Group << {group} >>"),
        "0.5 g 0 0 100 100 re f",
    )
}

#[test]
fn a_graphics_state_without_smask_keeps_the_soft_mask_in_force_spec_audit() {
    // §8.4.5: a parameter dictionary sets the parameters its entries name, and §11.6.4.3 gives
    // only the name /None the power to remove a mask — "It shall also mean that any existing
    // mask shall be removed from the current graphics state." /A states /ca alone, so the
    // half mask /M set before it is still the current soft mask when the red fill is painted.
    let resources = format!("/ExtGState << {HALF_MASK} /A << /Type /ExtGState /ca 1 >> >>");
    let mask = half_mask_group("/S /Transparency /CS /DeviceRGB");

    let (control, _) = centre(page(
        RGB_PAGE,
        &resources,
        "/M gs 1 0 0 rg 0 0 100 100 re f",
        &mask,
    ));
    assert_near(control, [255, 127, 127, 255], "the half mask alone");

    let (after_a, reports) = centre(page(
        RGB_PAGE,
        &resources,
        "/M gs /A gs 1 0 0 rg 0 0 100 100 re f",
        &mask,
    ));
    assert_near(
        after_a,
        [255, 127, 127, 255],
        &format!("a gs with no /SMask entry leaves the mask in force ({reports:?})"),
    );
}

#[test]
fn the_page_groups_knockout_flag_is_honoured_spec_audit() {
    // Table 147's /K, "if true, they shall be composited with the group's initial backdrop and
    // shall overwrite ("knock out") any earlier overlapping objects", has no page exception: the
    // altered page readings are /I's and /CS's alone, and §11.4.7 treats the page group as an
    // isolated group, whose initial backdrop is transparent. So at the centre the blue square at
    // ca 0.5 replaces the red one and composites over the white medium alone: (127, 127, 255).
    // Without /K it composites over red at ca 0.5 too: (127, 63, 191).
    let resources = "/ExtGState << /H << /Type /ExtGState /ca 0.5 >> >>";
    let content = "/H gs 1 0 0 rg 10 10 80 80 re f 0 0 1 rg 30 30 60 60 re f";

    let (non_knockout, _) = centre(page(RGB_PAGE, resources, content, ""));
    assert_near(
        non_knockout,
        [127, 63, 191, 255],
        "a non-knockout page group",
    );

    let (knockout, reports) = centre(page(
        "/Group << /Type /Group /S /Transparency /CS /DeviceRGB /K true >>",
        resources,
        content,
        "",
    ));
    assert_near(
        knockout,
        [127, 127, 255, 255],
        &format!("a knockout page group: the later square knocks out the earlier ({reports:?})"),
    );
}

#[test]
fn a_backdrop_colour_of_the_wrong_length_is_reported_spec_audit() {
    // Table 142's /BC: "The array shall consist of n numbers, where n is the number of components
    // in the colour space specified by the CS entry". [1] beside a DeviceRGB group is not that
    // colour; the tree draws the default black backdrop in its place, which may be the right
    // drawing, but it says nothing (trap 5). The well-formed [1 1 1] is the control: its white
    // backdrop leaves the unpainted centre of the group at luminosity 1.
    let group = stream(
        5,
        "/Type /XObject /Subtype /Form /BBox [0 0 100 100] \
         /Group << /S /Transparency /CS /DeviceRGB >>",
        "1 g 0 0 10 10 re f",
    );
    let mask = |bc: &str| {
        format!(
            "/ExtGState << /M << /Type /ExtGState \
             /SMask << /Type /Mask /S /Luminosity /G 5 0 R /BC {bc} >> >> >>"
        )
    };

    let (white, reports) = centre(page(
        RGB_PAGE,
        &mask("[1 1 1]"),
        "/M gs 1 0 0 rg 0 0 100 100 re f",
        &group,
    ));
    assert_near(white, [255, 0, 0, 255], "a white /BC lets the fill through");
    assert!(
        reports.is_empty(),
        "the well-formed mask is complete: {reports:?}"
    );

    let (_, reports) = centre(page(
        RGB_PAGE,
        &mask("[1]"),
        "/M gs 1 0 0 rg 0 0 100 100 re f",
        &group,
    ));
    assert!(
        !reports.is_empty(),
        "a one-number /BC beside a three-component /CS is reported, not replaced in silence"
    );
}

#[test]
fn a_luminosity_group_without_a_colour_space_is_reported_spec_audit() {
    // Table 142's /G: "If the subtype S is Luminosity , the group attributes dictionary shall
    // contain a CS entry defining the colour space in which the compositing computation is to be
    // performed", and Table 147's /CS repeats it for "one that is the value of the G entry in a
    // soft-mask dictionary of subtype Luminosity". The tree composites such a group on device
    // RGB and draws the mask, in silence.
    let resources = format!("/ExtGState << {HALF_MASK} >>");

    let (stated, reports) = centre(page(
        RGB_PAGE,
        &resources,
        "/M gs 1 0 0 rg 0 0 100 100 re f",
        &half_mask_group("/S /Transparency /CS /DeviceRGB"),
    ));
    assert_near(stated, [255, 127, 127, 255], "the group with its /CS");
    assert!(
        reports.is_empty(),
        "the well-formed mask is complete: {reports:?}"
    );

    let (_, reports) = centre(page(
        RGB_PAGE,
        &resources,
        "/M gs 1 0 0 rg 0 0 100 100 re f",
        &half_mask_group("/S /Transparency"),
    ));
    assert!(
        !reports.is_empty(),
        "a /Luminosity mask whose group states no /CS is reported"
    );
}

#[test]
fn an_unrecognised_blend_mode_name_is_reported_spec_audit() {
    // §11.6.3: the /BM value "shall be either a name object, designating one of the standard
    // blend modes listed in "Table 134 ..." and "Table 135 ..." ... or ... an array of such
    // names". /Foo is neither. Drawing it as Normal is the array form's own fallback ("or Normal
    // if it recognizes none of them"), but a name outside both tables is unsupported input and
    // is said. The control is the array whose second name is recognised: green multiplied over
    // red.
    let content = "1 0 0 rg 0 0 100 100 re f /B gs 0 1 0 rg 0 0 100 100 re f";

    let (array, reports) = centre(page(
        RGB_PAGE,
        "/ExtGState << /B << /Type /ExtGState /BM [/Foo /Multiply] >> >>",
        content,
        "",
    ));
    assert_near(array, [0, 0, 0, 255], "the first recognised name, Multiply");
    assert!(
        reports.is_empty(),
        "the array form is complete: {reports:?}"
    );

    let (_, reports) = centre(page(
        RGB_PAGE,
        "/ExtGState << /B << /Type /ExtGState /BM /Foo >> >>",
        content,
        "",
    ));
    assert!(
        !reports.is_empty(),
        "a /BM name outside Tables 134 and 135 is reported"
    );
}
