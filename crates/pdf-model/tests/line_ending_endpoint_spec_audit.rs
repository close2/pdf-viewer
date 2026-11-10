//! Table 178's literal LE-to-L endpoint mapping when LL is present.
//! This tests missing-AP construction policy, not a PDF 2.0 producer conformance witness.
//! ADR 0192 chooses the displaced line's endpoints instead. Figure 80 supplies no arrowhead
//! on the main line to substantiate its stated justification; this regression records the
//! table's explicit endpoint mapping and deliberately exposes the current disagreement.

#![expect(
    clippy::expect_used,
    reason = "fixed fixture and renderer premises must fail loudly"
)]
use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;
use std::fmt::Write as _;

fn line_with(extra: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Line /Rect [0 0 100 100] /F 4 \
         /L [20 50 80 50] /C [0 0 0] /BS << /W 1 >> \
         /IC [1 0 0] {extra} >>\nendobj\n"
    );
    assemble(&body)
}

fn assemble(body: &str) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn draw(extra: &str) -> pdf_render::Raster {
    let document = Document::open(line_with(extra)).expect("complete PDF syntax");
    assert!(!document.was_recovered(), "exact cross references");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpretation = pdf_model::interpret(&document, &page);
    assert!(
        interpretation.is_complete(),
        "{:?}",
        interpretation.unsupported
    );
    let list = interpretation.display_list;
    let target = TargetSpec::for_page(&list, 4.0, 1 << 30).expect("small target");
    CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&list, target)
        .expect("supported drawing")
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "indices and sums are bounded by the fixed 400 by 400 raster"
)]
fn filled_ending_center(extra: &str) -> (f32, f32) {
    let raster = draw(extra);
    let mut bounds = (u32::MAX, 0, u32::MAX, 0);
    let mut count = 0;
    for row in 0..raster.height {
        for x in 0..raster.width {
            let index = (row * raster.width + x) as usize * 4;
            let pixel = &raster.data[index..index + 4];
            if pixel[0] > 200 && pixel[1] < 20 && pixel[2] < 20 && pixel[3] > 200 {
                bounds.0 = bounds.0.min(x);
                bounds.1 = bounds.1.max(x);
                let y = raster.height - 1 - row;
                bounds.2 = bounds.2.min(y);
                bounds.3 = bounds.3.max(y);
                count += 1;
            }
        }
    }
    assert!(count > 0, "the requested ending has visible red fill");
    // Pixel centers converted back to page units. The center is independent of the
    // implementation's chosen square size and excludes the black line/leader stroke.
    #[expect(
        clippy::cast_precision_loss,
        reason = "400-pixel fixture coordinates are exact"
    )]
    (
        (bounds.0 + bounds.1 + 1) as f32 / 8.0,
        (bounds.2 + bounds.3 + 1) as f32 / 8.0,
    )
}

#[test]
fn line_endings_use_the_l_coordinate_pairs_even_with_leader_lines() {
    let first = "/LE [/Square /None]";
    let second = "/LE [/None /Square]";
    // Independent endpoint-to-slot controls run before either disputed LL assertion.
    assert_eq!(filled_ending_center(first), (20.0, 50.0));
    assert_eq!(filled_ending_center(second), (80.0, 50.0));
    let actual = [
        filled_ending_center(&format!("{first} /LL 20")),
        filled_ending_center(&format!("{second} /LL 20")),
    ];
    assert_eq!(
        actual,
        [(20.0, 50.0), (80.0, 50.0)],
        "Table 178 LE names L coordinates; the line proper is a different segment when LL exists"
    );
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "fixed raster scale and bounded sample coordinates"
)]
fn painted(raster: &pdf_render::Raster, x: u32, y: u32) -> bool {
    let row = raster.height - 1 - y * 4;
    raster.data[((row * raster.width + x * 4) * 4 + 3) as usize] > 200
}

#[test]
fn positive_ll_places_the_proper_line_opposite_the_clockwise_leaders() {
    let absent = draw("");
    let zero = draw("/LL 0");
    assert_eq!(absent.data, zero.data, "zero means no leaders");
    assert!(painted(&absent, 50, 50));
    assert!(!painted(&absent, 50, 30));
    assert!(!painted(&absent, 50, 70));

    // Table 178: leaders extend FROM the line proper in the signed direction. L holds
    // their far endpoints. Figure 80's positive example puts the proper line above these
    // endpoints; its negative example puts it below. For horizontal left-to-right L,
    // clockwise is (0,-1), so proper = L - LL*(0,-1), not L + LL*(0,-1).
    let positive = draw("/LL 20");
    let negative = draw("/LL -20");
    let positions = [
        [painted(&positive, 50, 70), painted(&positive, 50, 30)],
        [painted(&negative, 50, 70), painted(&negative, 50, 30)],
    ];
    assert_eq!(
        positions,
        [[true, false], [false, true]],
        "LL describes leader direction from the line proper, not displacement from L to it"
    );
}
