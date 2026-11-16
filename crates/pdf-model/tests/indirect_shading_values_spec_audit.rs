//! Exact-xref public-pixel tests for §7.3.10 references in §8.7.4 shading arrays.
//! The literal colors follow Table 79/80's scalar interpolation and the Type 2 function;
//! direct fixtures are positive controls, not the expected-value oracle.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "small exact PDF fixture and in-bounds sample pixels should fail loudly"
)]

use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;
use std::fmt::Write as _;

const CONTENT: &str = "/Sh0 sh";
const GENEROUS: u64 = 1 << 24;

fn pdf(shading: &str, extras: &[&str]) -> Vec<u8> {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
          /Resources << /Shading << /Sh0 5 0 R >> >> /Contents 4 0 R >>"
            .to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{CONTENT}\nendstream",
            CONTENT.len() + 1
        ),
        shading.to_owned(),
    ];
    objects.extend(extras.iter().map(|object| (*object).to_owned()));

    let mut body = String::new();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(body.len());
        let _ = writeln!(body, "{} 0 obj\n{object}\nendobj", index + 1);
    }
    let xref = body.len();
    let mut out = format!("%PDF-1.7\n{body}");
    let size = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let absolute = "%PDF-1.7\n".len() + offset;
        let _ = writeln!(out, "{absolute:010} 00000 n ");
    }
    let xref = "%PDF-1.7\n".len() + xref;
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

fn render(bytes: Vec<u8>) -> pdf_render::Raster {
    let document = Document::open(bytes).expect("valid exact-xref shading PDF");
    assert!(
        !document.was_recovered(),
        "the fixture has exact xref offsets"
    );
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "the valid shading is fully interpreted: {:?}",
        interpretation.unsupported
    );
    let target = TargetSpec::for_page(&interpretation.display_list, 1.0, GENEROUS)
        .expect("small page target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&interpretation.display_list, target)
        .expect("CPU rasterizer supports the shading")
}

// Raster rows are top-to-bottom. The selected points avoid clip edges and gradient boundaries.
fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> [u8; 4] {
    let at = usize::try_from((y * raster.width + x) * 4).expect("bounded pixel index");
    [
        raster.data[at],
        raster.data[at + 1],
        raster.data[at + 2],
        raster.data[at + 3],
    ]
}

fn axial(indirect: bool, reverse: bool) -> Vec<u8> {
    let function = "<< /FunctionType 2 /Domain [0 1] /C0 [0 0 1] /C1 [1 0 0] /N 1 >>";
    if indirect {
        let shading = "<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords 7 0 R \
            /Domain 12 0 R /Extend 15 0 R /BBox 18 0 R /Function 6 0 R >>";
        let extras = [
            function,
            "[8 0 R 9 0 R 10 0 R 11 0 R]",
            "10",
            "50",
            "90",
            "50",
            "[13 0 R 14 0 R]",
            ".2",
            ".8",
            "[16 0 R 17 0 R]",
            if reverse { "false" } else { "true" },
            if reverse { "true" } else { "false" },
            "[19 0 R 20 0 R 21 0 R 22 0 R]",
            "5",
            "20",
            "95",
            "80",
        ];
        pdf(shading, &extras)
    } else {
        let shading = "<< /ShadingType 2 /ColorSpace /DeviceRGB /Coords [10 50 90 50] \
            /Domain [.2 .8] /Extend [true false] /BBox [5 20 95 80] /Function 6 0 R >>";
        let shading = if reverse {
            shading.replace("/Extend [true false]", "/Extend [false true]")
        } else {
            shading.to_owned()
        };
        pdf(&shading, &[function])
    }
}

#[test]
fn axial_indirect_coords_domain_extend_bbox_and_array_members_match_direct_control() {
    let direct = render(axial(false, false));
    let indirect = render(axial(true, false));

    // At pixel center x=30.5, x'=(30.5-10)/80=.25625 and t=.2+.6*x'=.35375.
    // The function maps t to (t, 0, 1-t), so device RGB is approximately (90,0,165).
    let direct_interior = pixel(&direct, 30, 50);
    let indirect_interior = pixel(&indirect, 30, 50);
    for (label, sample) in [("direct", direct_interior), ("indirect", indirect_interior)] {
        assert_eq!(
            sample[3], 255,
            "{label} point inside BBox and axis paints: {sample:?}"
        );
        assert!(
            sample[0].abs_diff(90) <= 4 && sample[1] <= 2 && sample[2].abs_diff(165) <= 4,
            "{label} follows the nondefault Domain and linear function: {sample:?}"
        );
    }
    assert_eq!(
        indirect_interior, direct_interior,
        "indirect representation is equivalent"
    );

    // The first extension is true: the before-axis point in BBox receives t0=.2 -> (51,0,204).
    let before_axis = pixel(&indirect, 7, 50);
    assert_eq!(before_axis[3], 255, "first Extend boolean is honored");
    assert!(
        before_axis[0].abs_diff(51) <= 3 && before_axis[2].abs_diff(204) <= 3,
        "the start colour comes from Domain t0=.2: {before_axis:?}"
    );
    // The second extension is false; this point is beyond the axis but still inside BBox.
    assert_eq!(
        pixel(&indirect, 94, 50)[3],
        0,
        "second Extend boolean is honored"
    );
    // Axial geometry is unbounded perpendicular to its axis; BBox supplies this clip.
    assert_eq!(
        pixel(&indirect, 30, 10)[3],
        0,
        "indirect BBox clips the shading"
    );
    // Reverse both booleans: each reference must be consumed, including a true second entry.
    for indirect in [false, true] {
        let reversed = render(axial(indirect, true));
        assert_eq!(pixel(&reversed, 7, 50)[3], 0, "false first extension");
        let after = pixel(&reversed, 94, 50);
        assert_eq!(after[3], 255, "true second extension");
        assert!(
            after[0].abs_diff(204) <= 3 && after[1] <= 2 && after[2].abs_diff(51) <= 3,
            "the extended end uses the nondefault domain end: {after:?}"
        );
    }
}

fn radial(indirect: bool, reverse: bool) -> Vec<u8> {
    let function = "<< /FunctionType 2 /Domain [0 1] /C0 [0 0 1] /C1 [1 0 0] /N 1 >>";
    if indirect {
        let shading = "<< /ShadingType 3 /ColorSpace /DeviceRGB /Coords 7 0 R \
            /Domain 14 0 R /Extend 17 0 R /BBox 20 0 R /Function 6 0 R >>";
        let extras = [
            function,
            "[8 0 R 9 0 R 10 0 R 11 0 R 12 0 R 13 0 R]",
            "50.5",
            "49.5",
            "10",
            "50.5",
            "49.5",
            "40",
            "[15 0 R 16 0 R]",
            ".1",
            ".7",
            "[18 0 R 19 0 R]",
            if reverse { "false" } else { "true" },
            if reverse { "true" } else { "false" },
            "[21 0 R 22 0 R 23 0 R 24 0 R]",
            "20",
            "5",
            "95",
            "95",
        ];
        pdf(shading, &extras)
    } else {
        let shading = "<< /ShadingType 3 /ColorSpace /DeviceRGB /Coords [50.5 49.5 10 50.5 49.5 40] \
            /Domain [.1 .7] /Extend [true false] /BBox [20 5 95 95] /Function 6 0 R >>";
        let shading = if reverse {
            shading.replace("/Extend [true false]", "/Extend [false true]")
        } else {
            shading.to_owned()
        };
        pdf(&shading, &[function])
    }
}

#[test]
fn radial_indirect_coords_domain_extend_bbox_and_array_members_match_direct_control() {
    let direct = render(radial(false, false));
    let indirect = render(radial(true, false));

    // At page point (75.5, 49.5), radius is exactly 25, so s=.5 and t=.4.
    // Table 80's linear function gives approximately (102,0,153).
    let direct_mid = pixel(&direct, 75, 50);
    let indirect_mid = pixel(&indirect, 75, 50);
    for (label, sample) in [("direct", direct_mid), ("indirect", indirect_mid)] {
        assert_eq!(
            sample[3], 255,
            "{label} point is inside the blend and BBox: {sample:?}"
        );
        assert!(
            sample[0].abs_diff(102) <= 4 && sample[1] <= 2 && sample[2].abs_diff(153) <= 4,
            "{label} follows nondefault Domain [.1,.7]: {sample:?}"
        );
    }
    assert_eq!(
        indirect_mid, direct_mid,
        "indirect representation is equivalent"
    );

    // The first circle extends inward and carries its t0=.1 colour; this would be transparent
    // if the indirect true value were ignored in favor of the default false.
    let inside_start = pixel(&indirect, 50, 50);
    assert_eq!(
        inside_start[3], 255,
        "first radial Extend boolean paints inside r0"
    );
    assert!(
        inside_start[0].abs_diff(26) <= 3 && inside_start[2].abs_diff(230) <= 3,
        "the starting colour comes from t0=.1: {inside_start:?}"
    );
    // Point is inside the BBox but past r1; false second Extend leaves it untouched.
    assert_eq!(
        pixel(&indirect, 94, 50)[3],
        0,
        "second radial Extend boolean is false"
    );
    // Radius 34.5 lies within the two circles, but x=15.5 is outside the BBox.
    assert_eq!(
        pixel(&indirect, 15, 50)[3],
        0,
        "indirect BBox clips a painted radius"
    );
    // Reverse both booleans: each reference must be consumed, including a true second entry.
    for indirect in [false, true] {
        let reversed = render(radial(indirect, true));
        assert_eq!(pixel(&reversed, 50, 50)[3], 0, "false first extension");
        let after = pixel(&reversed, 94, 50);
        assert_eq!(after[3], 255, "true second extension");
        assert!(
            after[0].abs_diff(179) <= 3 && after[1] <= 2 && after[2].abs_diff(77) <= 3,
            "the extended end uses the nondefault domain end: {after:?}"
        );
    }
}
