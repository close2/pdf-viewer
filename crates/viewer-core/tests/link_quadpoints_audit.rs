//! Table 176: link activation and pressed highlighting share the active region.
#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    reason = "a broken PDF fixture or pixel assertion must fail loudly"
)]

use std::fmt::Write as _;

use pdf_render::Rasterizer;
use render_cpu::CpuRasterizer;
use viewer_core::{Answer, Command, DocumentId, Event, PointerAction, Query, Viewer};

fn assemble(body: &str) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
    writeln!(out, "xref\n0 {size}").expect("string write");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        writeln!(out, "{offset:010} 00000 n ").expect("string write");
    }
    writeln!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF"
    )
    .expect("string write");
    out.into_bytes()
}

fn pdf(fallback: bool) -> Vec<u8> {
    let annotation = if fallback { 9 } else { 6 };
    let appearance = "1 0 0 rg 0 0 180 180 re f";
    let page_paint = "0 1 0 rg 0 0 200 200 re f";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
         /Contents 5 0 R /Annots [{annotation} 0 R] >>\nendobj\n\
         4 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> \
         /Contents 7 0 R >>\nendobj\n\
         5 0 obj\n<< /Length {} >>\nstream\n{page_paint}\nendstream\nendobj\n\
         6 0 obj\n<< /Type /Annot /Subtype /Link /Rect [10 10 190 190] /F 4 /H /I \
         /QuadPoints [20 120 60 120 60 160 20 160] /Dest [4 0 R /Fit] \
         /AP << /N 8 0 R >> >>\nendobj\n\
         7 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         8 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 180 180] /Resources << >> \
         /Length {} >>\nstream\n{appearance}\nendstream\nendobj\n\
         9 0 obj\n<< /Type /Annot /Subtype /Link /Rect [110 10 190 90] /F 4 /H /I \
         /QuadPoints [100 20 150 20 150 40 100 40] /Dest [4 0 R /Fit] /AP << /N 8 0 R >> >>\nendobj\n",
        page_paint.len() + 1,
        appearance.len() + 1
    );
    assemble(&body)
}

fn device_point(viewer: &Viewer, x: f32, y: f32) -> (f32, f32) {
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("opened page has geometry");
    };
    // PDF user space is bottom-up. The test viewport and page are both 200×200 at fit scale;
    // use the returned origin and scale, not assumptions about centering.
    (
        geometry.origin.0 + x * geometry.scale,
        geometry.origin.1 + (200.0 - y) * geometry.scale,
    )
}

fn raster_after_press(
    user_point: (f32, f32),
    is_active_link: bool,
    fallback: bool,
) -> pdf_render::Raster {
    let bytes = pdf(fallback);
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("complete valid PDF 1.7 fixture");
    assert!(!parsed.was_recovered(), "fixture has exact xref offsets");
    let mut viewer = Viewer::new(200, 200, 1.0);
    let opened: Vec<_> = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        opened
            .iter()
            .any(|e| matches!(e, Event::Opened { pages: 2, .. }))
    );
    let point = device_point(&viewer, user_point.0, user_point.1);
    assert!(
        matches!(viewer.query(Query::LinkAt(point)), Answer::Link(active) if active == is_active_link),
        "Query::LinkAt follows the effective activation region",
    );
    let events: Vec<_> = viewer
        .handle(Command::Pointer {
            at: point,
            action: PointerAction::Pressed,
        })
        .collect();
    let request = events
        .iter()
        .chain(opened.iter())
        .find_map(|e| match e {
            Event::NeedsRender(request) => Some(request),
            _ => None,
        })
        .expect("press produces an updated page render");
    CpuRasterizer::new()
        .rasterize(&request.list, request.target)
        .expect("valid complete page rasterizes")
}

fn pixel(raster: &pdf_render::Raster, x: u32, y: u32) -> (u8, u8, u8) {
    let row = raster.height - 1 - y;
    let index = ((row * raster.width + x) * 4) as usize;
    (
        raster.data[index],
        raster.data[index + 1],
        raster.data[index + 2],
    )
}

#[test]
fn escaped_quadpoints_use_rect_without_enlarging_the_active_region() {
    let inside = raster_after_press((180.0, 70.0), true, true);
    assert_eq!(pixel(&inside, 180, 70), (0, 255, 255));
    let outside = raster_after_press((105.0, 30.0), false, true);
    assert_eq!(pixel(&outside, 105, 30), (0, 255, 0));
}

#[test]
fn link_pressed_highlight_uses_quadpoints_active_region_not_the_larger_rect() {
    // The AP fills Rect red. Valid CCW QuadPoints define a smaller active region.
    let inside_quad = raster_after_press((40.0, 140.0), true, false);
    assert_eq!(
        pixel(&inside_quad, 40, 140),
        (0, 255, 255),
        "inside active QuadPoints, /H /I applies"
    );
    let inside_rect_only = raster_after_press((150.0, 140.0), false, false);
    assert_eq!(
        pixel(&inside_rect_only, 150, 140),
        (255, 0, 0),
        "outside active QuadPoints, no link highlight"
    );
}
