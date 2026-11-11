//! PDF-consumer coverage for Table 142 /TR identity and output-range behavior.
//!
//! These assertions exercise the interpreted soft mask through the CPU rasterizer. The clamp
//! test establishes output pixels, not a source-level need for an explicit clamp: float-to-byte
//! conversion can itself saturate, so deleting only a redundant clamp may be observationally
//! equivalent.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test fixture errors and checked pixel positions should fail loudly"
)]

use std::fmt::Write as _;

use pdf_model::{Pages, interpret};
use pdf_render::{Raster, Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const PIXEL_BUDGET: u64 = 1 << 20;

/// Build a valid one-page PDF with a soft-mask group and the requested transfer dictionary text.
fn pdf(subtype: &str, transfer: &str, backdrop: &str, group_content: &str) -> Vec<u8> {
    let objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] /Contents 4 0 R \
          /Resources << /ExtGState << /GS 5 0 R >> >> >>"
            .to_vec(),
        plain_stream("q /GS gs 0 g 0 0 40 40 re f Q\n"),
        format!(
            "<< /Type /ExtGState /SMask << /Type /Mask /S /{subtype} /G 6 0 R \
             /BC {backdrop} {transfer} >> >>"
        )
        .into_bytes(),
        form_stream(group_content),
        b"<< /Type /ExtGState /ca 0.25 >>".to_vec(),
    ];
    assemble(&objects)
}

fn plain_stream(content: &str) -> Vec<u8> {
    format!(
        "<< /Length {} >>\nstream\n{content}endstream",
        content.len()
    )
    .into_bytes()
}

fn form_stream(content: &str) -> Vec<u8> {
    let mut stream = format!(
        "<< /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 20 40] \
         /Resources << /ExtGState << /Fade 7 0 R >> >> \
         /Group << /Type /Group /S /Transparency /CS /DeviceGray /I true >> \
         /Length {} >>\nstream\n",
        content.len()
    )
    .into_bytes();
    stream.extend_from_slice(content.as_bytes());
    stream.extend_from_slice(b"endstream");
    stream
}

fn assemble(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index.saturating_add(1)).as_bytes());
        out.extend_from_slice(object);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let mut tail = String::new();
    let _ = writeln!(tail, "xref\n0 {size}");
    tail.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(tail, "{offset:010} 00000 n ");
    }
    let _ = write!(
        tail,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.extend_from_slice(tail.as_bytes());
    out
}

fn render(bytes: Vec<u8>) -> Raster {
    let document = Document::open(bytes).expect("valid complete PDF container");
    assert!(
        !document.was_recovered(),
        "all object offsets come from the xref table"
    );
    let page = Pages::new(&document).get(0).expect("one page");
    let interpretation = interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "all PDF features in this fixture are supported: {:?}",
        interpretation.unsupported
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, PIXEL_BUDGET)
        .expect("40x40 target fits budget");
    let raster = CpuRasterizer::new()
        .rasterize(&interpretation.display_list, target)
        .expect("CPU supports the display list");
    assert_eq!((raster.width, raster.height), (40, 40));
    raster
}

fn rgba_at(raster: &Raster, x: u32, y: u32) -> [u8; 4] {
    assert!(
        x < raster.width && y < raster.height,
        "sample is in raster bounds"
    );
    let row = raster.height.saturating_sub(1).saturating_sub(y);
    let index = usize::try_from(row.saturating_mul(raster.width).saturating_add(x))
        .expect("small raster")
        .saturating_mul(4);
    [
        raster.data[index],
        raster.data[index + 1],
        raster.data[index + 2],
        raster.data[index + 3],
    ]
}

fn near_rgba(got: [u8; 4], expected: [u8; 4], label: &str) {
    for (channel, (actual, expected)) in got.into_iter().zip(expected).enumerate() {
        assert!(
            i16::from(actual).abs_diff(i16::from(expected)) <= 1,
            "{label}, RGBA channel {channel}: got {actual}, expected {expected} ±1"
        );
    }
}

#[test]
fn explicit_identity_matches_omitted_transfer_for_alpha_and_luminosity() {
    // The isolated group paints its 20x40 BBox, the left half of the page. For Alpha, /Fade sets
    // that mark's group alpha to .25, while outside the BBox the alpha input is 0. For
    // Luminosity, the group mark is .25 grey and /BC is .5 outside the BBox.
    for (subtype, group, backdrop, inside, outside) in [
        (
            "Alpha",
            "q /Fade gs 0 g 0 0 20 40 re f Q\n",
            "[1]",
            [191, 191, 191, 255],
            [255, 255, 255, 255],
        ),
        (
            "Luminosity",
            "0.25 g 0 0 20 40 re f\n",
            "[0.5]",
            [191, 191, 191, 255],
            [128, 128, 128, 255],
        ),
    ] {
        let absent = render(pdf(subtype, "", backdrop, group));
        let explicit = render(pdf(subtype, "/TR /Identity", backdrop, group));
        near_rgba(
            rgba_at(&absent, 10, 20),
            inside,
            "implicit identity, group interior",
        );
        near_rgba(
            rgba_at(&explicit, 10, 20),
            inside,
            "explicit identity, group interior",
        );
        near_rgba(
            rgba_at(&absent, 30, 20),
            outside,
            "implicit identity, outside BBox",
        );
        near_rgba(
            rgba_at(&explicit, 30, 20),
            outside,
            "explicit identity, outside BBox",
        );
        for (x, y) in [(10, 20), (30, 20)] {
            near_rgba(
                rgba_at(&explicit, x, y),
                rgba_at(&absent, x, y),
                "explicit /Identity equals omitted /TR",
            );
        }
    }
}

#[test]
fn transfer_outputs_below_zero_and_above_one_use_nearest_unit_value() {
    // Each valid Type 2 function has a Range allowing its constant out-of-unit output. The
    // first returns -0.5 everywhere; the second returns 1.5 everywhere. Interior grey and
    // outside /BC are both .5, so the independent expected results are respectively the
    // nearest legal mask values 0 and 1 at both locations.
    let group = "0.5 g 0 0 20 40 re f\n";
    let negative = render(pdf(
        "Luminosity",
        "/TR << /FunctionType 2 /Domain [0 1] /Range [-1 2] /N 1 /C0 [-0.5] /C1 [-0.5] >>",
        "[0.5]",
        group,
    ));
    let above_one = render(pdf(
        "Luminosity",
        "/TR << /FunctionType 2 /Domain [0 1] /Range [-1 2] /N 1 /C0 [1.5] /C1 [1.5] >>",
        "[0.5]",
        group,
    ));

    for (x, y) in [(10, 20), (30, 20)] {
        near_rgba(
            rgba_at(&negative, x, y),
            [255, 255, 255, 255],
            "TR output -0.5 is forced to 0 for interior and BC input",
        );
        near_rgba(
            rgba_at(&above_one, x, y),
            [0, 0, 0, 255],
            "TR output 1.5 is forced to 1 for interior and BC input",
        );
    }
}
