//! Screen magnification must not determine annotation geometry on the printed page.
#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "the controlled fixture and sample raster should fail loudly if this path changes"
)]

use std::fmt::Write as _;

use pdf_render::Rasterizer;
use render_cpu::CpuRasterizer;
use viewer_core::{Answer, Command, DocumentId, Event, Printing, Query, Rendered, Sheet, Viewer};

const ID: DocumentId = DocumentId(71);

fn appearance(content: &str) -> String {
    format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 80 80] /Length {} >>\nstream\n{content}\nendstream",
        content.len()
    )
}

fn document(flags: u32) -> Vec<u8> {
    let red = appearance("1 0 0 rg 0 0 80 80 re f");
    let green = appearance("0 1 0 rg 0 0 80 80 re f");
    let blue = appearance("0 0 1 rg 0 0 80 80 re f");
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Square /Rect [10 10 90 90] /F {flags} \
         /AP << /N 6 0 R /R 7 0 R /D 8 0 R >> >>\nendobj\n\
         6 0 obj\n{red}\nendobj\n7 0 obj\n{green}\nendobj\n8 0 obj\n{blue}\nendobj\n"
    );
    let mut output = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(output.len());
        output.push_str(object);
    }
    let xref = output.len();
    let size = offsets.len() + 1;
    let _ = writeln!(output, "xref\n0 {size}");
    output.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(output, "{offset:010} 00000 n ");
    }
    let _ = write!(
        output,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    output.into_bytes()
}

fn settle(viewer: &mut Viewer, events: Vec<Event>) {
    for event in events {
        let Event::NeedsRender(request) = event else {
            continue;
        };
        let raster = CpuRasterizer::new()
            .rasterize(&request.list, request.target)
            .expect("screen render");
        let more = viewer
            .handle(Command::RenderReady {
                token: request.token,
                rendered: Rendered::Raster(raster),
            })
            .collect();
        settle(viewer, more);
    }
}

fn printed_extent(flags: u32, zoom: f32) -> (u32, u32) {
    let bytes = document(flags);
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("complete fixture");
    assert!(!parsed.was_recovered());
    let mut viewer = Viewer::new(400, 400, 1.0);
    let opened = viewer
        .handle(Command::Open {
            id: ID,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    settle(&mut viewer, opened);
    let zoomed = viewer
        .handle(Command::Zoom {
            zoom: viewer_core::Zoom::Scale(zoom),
            at: None,
        })
        .collect();
    settle(&mut viewer, zoomed);
    assert!(
        matches!(viewer.query(Query::View), Answer::View(view) if view.zoom == viewer_core::Zoom::Scale(zoom))
    );
    let begun = viewer
        .handle(Command::Print(Printing::Start(Sheet {
            media: None,
            scale: 2.0,
            page_scale: 1.0,
        })))
        .collect();
    settle(&mut viewer, begun);
    let Answer::PrintPage(page) = viewer.query(Query::PrintPage(0)) else {
        panic!("granted print page");
    };
    let target = page.target.expect("small print page fits");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&page.list, target)
        .expect("print render");
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (u32::MAX, u32::MAX, 0, 0);
    for y in 0..raster.height {
        for x in 0..raster.width {
            let i = ((y * raster.width + x) as usize) * 4;
            if raster.data[i] > 200
                && raster.data[i + 1] < 20
                && raster.data[i + 2] < 20
                && raster.data[i + 3] > 0
            {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    assert!(
        min_x <= max_x && min_y <= max_y,
        "red annotation is nonempty"
    );
    (max_x - min_x + 1, max_y - min_y + 1)
}

#[test]
fn a_screen_nozoom_flag_does_not_shrink_the_printed_annotation_at_view_zoom() {
    // At this job's scale of two pixels per PDF unit, the 80-unit rectangle occupies 160 pixels.
    // Both ordinary controls distinguish job scale from viewport zoom. NoZoom describes screen
    // magnification; it must not make otherwise identical print jobs depend on the last view zoom.
    let ordinary_one = printed_extent(4, 1.0);
    let ordinary_two = printed_extent(4, 2.0);
    let no_zoom_one = printed_extent(12, 1.0);
    let no_zoom_two = printed_extent(12, 2.0);
    assert_eq!(ordinary_one, (160, 160));
    assert_eq!(ordinary_two, ordinary_one);
    assert_eq!(no_zoom_one, ordinary_one);
    assert_eq!(
        no_zoom_two, no_zoom_one,
        "screen zoom leaked into the print display list"
    );
}
