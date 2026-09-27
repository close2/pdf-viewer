//! A stencil painted through a pattern under the graphics state's soft mask, on the two device
//! backends, against the value ISO 32000-2 §11.3.5 gives by hand.
//!
//! §8.9.6.2 paints a stencil's places with the current colour, which §8.7.2 lets be a pattern,
//! and §11.6.4.3 displaces the state's soft mask only by a mask in the image dictionary, which a
//! stencil is not. So the mark has three parts: the stencil its shape, the pattern its colour,
//! the state's mask its opacity. The display list states the product of the two masks as one
//! soft mask whose command is drawn through the other (ADR 1334); what this file asks is that
//! both device backends evaluate a mask nested in a mask's group, and that they arrive where the
//! CPU oracle's own test (`pdf-model/tests/image_masks.rs`) does.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: an explanatory panic is the intended failure mode, and the indices are \
              into a 40 × 40 raster at points chosen inside it"
)]

use pdf_render::{DisplayList, Raster, Rasterizer, TargetSpec};
use render_gpu::GpuRasterizer;
use render_raster::QuorraRasterizer;

/// A red shading pattern: an axial shading whose function is the one colour at both ends.
const RED_SHADING_PATTERN: &[u8] = b"<< /PatternType 2 /Shading << /ShadingType 2 \
    /ColorSpace /DeviceRGB /Coords [0 0 40 0] /Extend [true true] /Function << /FunctionType 2 \
    /Domain [0 1] /C0 [1 0 0] /C1 [1 0 0] /N 1 >> >> >>";

/// A stream object body from a dictionary's inside and its data.
fn stream(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut out = format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

/// A red coloured tiling pattern whose one cell fills its whole step.
fn red_tiling_pattern() -> Vec<u8> {
    stream(
        "/PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 10 10] /XStep 10 /YStep 10 \
         /Resources << >>",
        b"1 0 0 rg 0 0 10 10 re f",
    )
}

/// A 40-point white page painting a 4 × 2 stencil — `0 1 1 1` over `0 0 0 1` — through
/// `pattern` under a luminosity soft mask of grey 0.5 over the left half and white over the right.
fn file(pattern: &[u8]) -> Vec<u8> {
    let content = b"1 g 0 0 40 40 re f /GS gs /Pattern cs /P0 scn 40 0 0 40 0 0 cm /Im Do";
    let group = b"1 g 0 0 40 40 re f 0.5 g 0 0 20 40 re f";
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] /Resources << /XObject << /Im 5 0 \
           R >> /ExtGState << /GS 6 0 R >> /Pattern << /P0 8 0 R >> >> /Contents 4 0 R >>"
            .to_vec(),
        stream("", content),
        stream(
            "/Type /XObject /Subtype /Image /Width 4 /Height 2 /ImageMask true",
            b"\x70\x10",
        ),
        b"<< /Type /ExtGState /SMask << /S /Luminosity /G 7 0 R >> >>".to_vec(),
        stream(
            "/Type /XObject /Subtype /Form /BBox [0 0 40 40] \
             /Group << /S /Transparency /CS /DeviceGray >>",
            group,
        ),
        pattern.to_vec(),
    ];
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (number, body) in (1_usize..).zip(&objects) {
        offsets.push(out.len());
        out.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    let entries = objects.len() + 1;
    out.extend_from_slice(format!("xref\n0 {entries}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size {entries} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n")
            .as_bytes(),
    );
    out
}

/// The page's display list, which must have drawn without a report.
fn list(pattern: &[u8]) -> DisplayList {
    let document = pdf_syntax::Document::open(file(pattern)).expect("the fixture is a PDF");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("the fixture has a page");
    let interpretation = pdf_model::content::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "the two masks compose rather than refuse: {:?}",
        interpretation.unsupported
    );
    interpretation.display_list
}

/// The RGBA at a point given in PDF coordinates, at one device pixel a point.
fn pixel(raster: &Raster, x: u32, y: u32) -> [u8; 4] {
    let row = raster.height - 1 - y;
    let at = ((row * raster.width + x) * 4) as usize;
    [
        raster.data[at],
        raster.data[at + 1],
        raster.data[at + 2],
        raster.data[at + 3],
    ]
}

/// The four cells §11.3.5 decides by hand: shape 1 under opacity 0.5 over white is
/// (255, 127.5, 127.5), shape 1 under opacity 1 is the pattern's red, shape 0 is the page.
fn assert_composed(raster: &Raster, what: &str) {
    for (x, y, want) in [
        (5, 5, [255, 128, 128]),
        (25, 5, [255, 0, 0]),
        (35, 5, [255, 255, 255]),
        (15, 30, [255, 255, 255]),
    ] {
        let got = pixel(raster, x, y);
        assert!(
            got[3] == 255
                && got
                    .iter()
                    .zip(want)
                    .all(|(got, want)| got.abs_diff(want) <= 2),
            "{what}: at ({x}, {y}) §11.3.5 gives {want:?}, drawn {got:?}"
        );
    }
}

#[test]
fn raster_composes_the_two_masks_of_a_stencil_painted_through_a_pattern() {
    let mut device = QuorraRasterizer::new_headless().unwrap_or_else(|e| {
        panic!("no adapter for raster: {e} — these tests do not skip (ADR 0004)")
    });
    for (pattern, what) in [
        (RED_SHADING_PATTERN.to_vec(), "raster, shading pattern"),
        (red_tiling_pattern(), "raster, tiling pattern"),
    ] {
        let list = list(&pattern);
        let target = TargetSpec::for_page(&list, 1.0, u64::MAX).expect("the target fits");
        let raster = device
            .rasterize(&list, target)
            .unwrap_or_else(|e| panic!("{what}: refused: {e}"));
        assert_composed(&raster, what);
    }
}

#[test]
fn vello_composes_the_two_masks_of_a_stencil_painted_through_a_pattern() {
    let mut device = GpuRasterizer::new_headless().unwrap_or_else(|e| {
        panic!("no adapter for vello: {e} — these tests do not skip (ADR 0004)")
    });
    for (pattern, what) in [
        (RED_SHADING_PATTERN.to_vec(), "vello, shading pattern"),
        (red_tiling_pattern(), "vello, tiling pattern"),
    ] {
        let list = list(&pattern);
        let target = TargetSpec::for_page(&list, 1.0, u64::MAX).expect("the target fits");
        let raster = device
            .rasterize(&list, target)
            .unwrap_or_else(|e| panic!("{what}: refused: {e}"));
        assert_composed(&raster, what);
    }
}
