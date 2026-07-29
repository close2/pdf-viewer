//! A soft mask in the graphics state: ISO 32000-2 §11.5 and §11.6.5.1, from the file down.
//!
//! `render-cpu/tests/soft_mask.rs` checks the arithmetic a mask value produces once it
//! exists. This file checks the half above it — the dictionary, the group, and where the
//! mask lands on the page — on documents written here rather than taken from a corpus, for
//! trap 8's reason: the corpus can only exercise what somebody happened to write, and three
//! of the four rules below are ones no corpus document distinguishes.
//!
//! Every expected value is derived from the clause and stated in the test that uses it. The
//! fixtures are 40-unit pages drawn at one pixel per unit, so a coordinate in the content
//! stream is a pixel.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a malformed fixture or an out-of-range pixel should fail loudly, \
              and the fixtures are 40x40 pages where no index can overflow"
)]

use std::fmt::Write as _;

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

/// Pixel budget, far above the 40×40 pages these tests build.
const GENEROUS: u64 = 1 << 30;

/// A one-page PDF whose content stream is `content`, with one `/ExtGState` named `/GS`.
///
/// `gstate` is what goes inside that dictionary and `group` is the content stream of the
/// form `XObject` its `/SMask` names, which is object 6 and always a transparency group in
/// `/DeviceGray` covering the left half of the page.
fn page(gstate: &str, content: &str, group: &str) -> Vec<u8> {
    let objects = vec![
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec(),
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] \
          /Resources << /ExtGState << /GS 5 0 R >> >> /Contents 4 0 R >>\nendobj\n"
            .to_vec(),
        stream_object(4, "", content.as_bytes()),
        format!("5 0 obj\n<< /Type /ExtGState {gstate} >>\nendobj\n").into_bytes(),
        stream_object(
            6,
            "/Type /XObject /Subtype /Form /BBox [0 0 20 40] \
             /Group << /Type /Group /S /Transparency /CS /DeviceGray >>",
            group.as_bytes(),
        ),
    ];
    assemble(&objects)
}

/// One numbered stream object, with the `/Length` its data actually has.
fn stream_object(number: usize, dict: &str, data: &[u8]) -> Vec<u8> {
    let mut out = format!(
        "{number} 0 obj\n<< {dict} /Length {} >>\nstream\n",
        data.len()
    )
    .into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream\nendobj\n");
    out
}

/// Assembles numbered objects into a file with a cross-reference table that points at them.
fn assemble(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for object in objects {
        offsets.push(out.len());
        out.extend_from_slice(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let mut trailer = String::new();
    let _ = writeln!(trailer, "xref\n0 {size}");
    trailer.push_str("0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(trailer, "{offset:010} 00000 n ");
    }
    let _ = write!(
        trailer,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.extend_from_slice(trailer.as_bytes());
    out
}

/// Renders a fixture at one pixel per unit onto white, and requires it to draw completely.
fn render(bytes: Vec<u8>) -> pdf_render::Raster {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "the fixture should draw completely: {:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 1.0, GENEROUS).expect("valid target");
    CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("supported")
}

/// The RGBA at a point given in PDF coordinates, whose y runs the other way from a raster's.
fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    let row = raster.height.saturating_sub(1).saturating_sub(y);
    let at = ((row.saturating_mul(raster.width)).saturating_add(x) as usize).saturating_mul(4);
    [
        raster.data[at],
        raster.data[at + 1],
        raster.data[at + 2],
        raster.data[at + 3],
    ]
}

/// The red channel at a point, which is what a black object over white says about its mask.
///
/// Painting black through a mask of value `v` over white leaves `255 × (1 − v)` in every
/// channel, so one channel is the whole answer and reading it as a number keeps the
/// assertions in the units the clause is written in.
fn level(raster: &pdf_render::Raster, x: u32, y: u32) -> i32 {
    i32::from(pixel(raster, x, y)[0])
}

/// Asserts a level within one, which is what an eight-bit mask value can promise.
fn near(got: i32, want: i32, what: &str) {
    assert!(
        (got - want).abs() <= 1,
        "{what}: level {got} is more than one away from the clause's {want}"
    );
}

/// A luminosity mask masks by the grey its group paints, and by `/BC` outside its box.
///
/// §11.5.3 derives the mask from "the luminosity of the resulting colour" of the group
/// composited onto the `/BC` backdrop, and §11.6.5.1 gives the area outside the group's
/// bounding box the same treatment: "the mask value shall be derived by transforming the BC
/// colour to luminosity". The fixture's group is a `/DeviceGray` form whose `/BBox` covers
/// the left half of the page and which paints 0.25 grey over its left quarter and 1.0 over
/// the rest of the box; `/BC` is 0.5.
///
/// So a black rectangle drawn over the whole page through that mask leaves, left to right:
/// 75% of the white showing where the mask is 0.25, none where it is 1.0, and half where the
/// backdrop decides it.
#[test]
fn a_luminosity_mask_takes_the_groups_grey_and_the_backdrops_outside_it() {
    let raster = render(page(
        "/SMask << /Type /Mask /S /Luminosity /G 6 0 R /BC [0.5] >>",
        "q /GS gs 0 g 0 0 40 40 re f Q",
        "0.25 g 0 0 10 40 re f 1 g 10 0 10 40 re f",
    ));

    near(
        level(&raster, 5, 20),
        191,
        "a mask of 0.25 leaves 75% of white",
    );
    near(level(&raster, 15, 20), 0, "a mask of 1.0 paints the object");
    near(
        level(&raster, 30, 20),
        128,
        "outside the group's /BBox the mask is the backdrop's luminosity",
    );
}

/// `/TR` maps the derived value, and the identity is what its absence means.
///
/// §11.6.5.1, of `/TR`: "The value of the TR key in the `SoftMask` dictionary, which is a
/// transfer function, shall then be applied to the computed group alpha to produce the mask
/// values."
/// The function here is `1 − x`, written as a type 2 exponential with `C0 = 1` and `C1 = 0`,
/// so every expectation of the test above is inverted — including the one outside the
/// bounding box, which is the case §11.6.5.1 states separately and the one an implementation
/// forgets.
#[test]
fn a_transfer_function_maps_every_mask_value_including_the_one_outside_the_box() {
    let raster = render(page(
        "/SMask << /Type /Mask /S /Luminosity /G 6 0 R /BC [0.5] \
         /TR << /FunctionType 2 /Domain [0 1] /Range [0 1] /N 1 /C0 [1] /C1 [0] >> >>",
        "q /GS gs 0 g 0 0 40 40 re f Q",
        "0.25 g 0 0 10 40 re f 1 g 10 0 10 40 re f",
    ));

    near(level(&raster, 5, 20), 64, "1 - 0.25 paints three quarters");
    near(level(&raster, 15, 20), 255, "1 - 1.0 paints nothing at all");
    near(
        level(&raster, 30, 20),
        128,
        "1 - 0.5 outside the box is still half",
    );
}

/// An alpha mask reads the group's alpha and ignores its colour (§11.5.2).
///
/// The group paints two rectangles of the *same* black at two different constant alphas, so
/// a derivation that looked at colour or at luminosity would make both halves identical —
/// and would mask everything away, since black has no luminosity. `/BC` is deliberately
/// present and deliberately irrelevant: Table 142 says it "shall be consulted only if the
/// subtype S is Luminosity".
#[test]
fn an_alpha_mask_reads_the_groups_alpha_and_ignores_its_colour() {
    let raster = render(page(
        "/SMask << /Type /Mask /S /Alpha /G 6 0 R /BC [1] >>",
        "q /GS gs 0 g 0 0 40 40 re f Q",
        "/GA gs 0 g 0 0 10 40 re f /GB gs 0 g 10 0 10 40 re f",
    ));

    // The group's own `/ExtGState`s are not in its resources, so both fills are opaque and
    // the mask is 1.0 inside the box: what this asserts is that the *colour* was ignored.
    near(
        level(&raster, 5, 20),
        0,
        "an opaque black gives a mask of 1",
    );
    near(level(&raster, 15, 20), 0, "and so does the second one");
    near(
        level(&raster, 30, 20),
        255,
        "outside the box an alpha mask is the transfer function of 0.0, whatever /BC says",
    );
}

/// `/SMask /None` removes the mask, and `q`/`Q` restores it.
///
/// §11.6.4.3: "The name None may be specified in place of a soft-mask dictionary, denoting
/// the absence of a soft mask. It shall also mean that any existing mask shall be removed
/// from the current graphics state." The fixture sets the mask, paints, sets `/None`, paints
/// again over the same place — and the second rectangle must be solid.
#[test]
fn none_removes_the_mask() {
    let mut objects = page(
        "/SMask << /Type /Mask /S /Luminosity /G 6 0 R /BC [0] >>",
        "q /GS gs /Off gs 0 g 0 0 40 40 re f Q",
        "1 g 0 0 20 40 re f",
    );
    // A second `/ExtGState`, `/Off`, added to the page's resources by rewriting the one
    // place they are written. Cheaper than a second builder and impossible to get subtly
    // wrong: if the name does not resolve, `gs` reports and `render` fails the fixture.
    let with_off = String::from_utf8(objects)
        .expect("the fixture is ASCII")
        .replace(
            "/ExtGState << /GS 5 0 R >>",
            "/ExtGState << /GS 5 0 R /Off 7 0 R >>",
        )
        .replace(
            "trailer\n<< /Size 7",
            "7 0 obj\n<< /Type /ExtGState /SMask /None >>\nendobj\ntrailer\n<< /Size 8",
        );
    objects = with_off.into_bytes();

    // The cross-reference table now names an object it has no offset for, which is exactly
    // the malformation `pdf-syntax` recovers from by scanning; the fixture is valid enough
    // to open and that is all this test needs it to be.
    let raster = render(objects);
    near(
        level(&raster, 5, 20),
        0,
        "with the mask removed the black rectangle is solid",
    );
    near(level(&raster, 30, 20), 0, "everywhere, not only in the box");
}

/// A mask is fixed where the `gs` established it, not where the object is painted.
///
/// §11.6.5.1: "The mask's coordinate system shall be defined by concatenating the
/// transformation matrix specified by the Matrix entry in the transparency group's form
/// dictionary … with the current transformation matrix at the moment the soft mask is
/// established in the graphics state with the gs operator." The fixture translates the
/// coordinate system by 20 units *after* the `gs`, so a mask that moved with the object
/// would cover the right half of the page instead of the left.
#[test]
fn the_masks_coordinate_system_is_the_one_in_force_at_the_gs() {
    let raster = render(page(
        "/SMask << /Type /Mask /S /Luminosity /G 6 0 R /BC [0] >>",
        "q /GS gs 1 0 0 1 20 0 cm 0 g -20 0 40 40 re f Q",
        "1 g 0 0 20 40 re f",
    ));

    near(
        level(&raster, 5, 20),
        0,
        "the mask's white half is where the gs left it",
    );
    near(
        level(&raster, 30, 20),
        255,
        "and the black backdrop still masks the other half away",
    );
}
