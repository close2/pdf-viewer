//! A `Pattern` space given numbers and no pattern name, and a tiling pattern that states no
//! nonzero step: the two answers ADRs 3450 and 3451 give, beside audit 2184's own tests
//! (`colour_8_6_8_7_spec_audit`).

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "small exact-xref fixtures and a centre pixel index"
)]

use pdf_model::Unsupported;
use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use std::fmt::Write as _;

/// A one-page document of 100 by 100 points with `resources` and `content`, and the further
/// objects `extra` numbered from 5, with exact cross-reference offsets.
fn page(resources: &str, content: &str, extra: &[String]) -> Vec<u8> {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
             /Resources {resources} /Contents 4 0 R >>"
        ),
        format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len()
        ),
    ];
    objects.extend_from_slice(extra);
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{object}\nendobj\n", index + 1);
    }
    let xref = out.len();
    let size = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

/// The page's reports and its centre pixel at 72 dpi on the CPU oracle.
fn drawn(bytes: Vec<u8>) -> (Vec<Unsupported>, [u8; 3]) {
    let document = Document::open(bytes).expect("fixture parses");
    assert!(!document.was_recovered(), "fixture has exact xref offsets");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    let list = &interpretation.display_list;
    let target = TargetSpec::for_page(list, 1.0, 1 << 20).expect("100-point target");
    let raster = render_cpu::CpuRasterizer::new()
        .rasterize(list, target)
        .expect("fixture rasterizes");
    let at = ((50 * raster.width + 50) * 4) as usize;
    (
        interpretation.unsupported,
        [raster.data[at], raster.data[at + 1], raster.data[at + 2]],
    )
}

/// ISO 32000-2 §8.6.8 of `scn` in a `Pattern` space: "name shall be the name of an entry in the
/// Pattern subdictionary". Numbers alone in a `Pattern` space with an underlying `DeviceRGB` name
/// no pattern, so they are not read as the underlying space's red: the colour in force — the
/// initial pattern, which paints nothing — stays, and the page says so.
#[test]
fn numbers_without_a_name_in_an_underlying_space_name_no_colour() {
    let (reports, centre) = drawn(page(
        "<< /ColorSpace << /P [/Pattern /DeviceRGB] >> >>",
        "/P cs 1 0 0 scn 0 0 100 100 re f",
        &[],
    ));
    assert_eq!(centre, [255, 255, 255], "nothing is painted");
    assert_eq!(
        reports,
        vec![Unsupported::PatternOperands {
            stroking: false,
            given: 3,
        }]
    );
}

/// A tiling pattern over the whole page, with `steps` written into its dictionary.
fn tiled(steps: &str) -> Vec<u8> {
    let cell = "1 0 0 rg 0 0 10 10 re f";
    page(
        "<< /Pattern << /P0 5 0 R >> >>",
        "/Pattern cs /P0 scn 0 0 100 100 re f",
        &[format!(
            "<< /Type /Pattern /PatternType 1 /PaintType 1 /TilingType 1 /BBox [0 0 10 10] \
             {steps} /Resources << >> /Length {} >>\nstream\n{cell}\nendstream",
            cell.len()
        )],
    )
}

/// ISO 32000-2 Table 74: `/XStep` and `/YStep` are "(Required)" and each "shall not be zero".
/// A pattern stating no nonzero spacing has no lattice a clause gives, so it is refused by name
/// and paints nothing (ADR 3451). Control: the same cell with both steps stated fills the page.
#[test]
fn a_tiling_pattern_with_no_nonzero_step_is_refused_and_said() {
    let (reports, centre) = drawn(tiled("/XStep 10 /YStep 10"));
    assert!(reports.is_empty(), "{reports:?}");
    assert_eq!(centre, [255, 0, 0], "the cells fill the page");
    for (steps, missing) in [
        ("/XStep 0 /YStep 10", "/XStep"),
        ("/XStep 10", "/YStep"),
        ("", "/XStep and /YStep"),
    ] {
        let (reports, centre) = drawn(tiled(steps));
        assert_eq!(centre, [255, 255, 255], "{steps:?} paints nothing");
        assert_eq!(
            reports,
            vec![Unsupported::Shading {
                name: format!(
                    "tiling pattern /P0 states no nonzero {missing}, which Table 74 requires"
                ),
            }],
            "{steps:?}"
        );
    }
}
