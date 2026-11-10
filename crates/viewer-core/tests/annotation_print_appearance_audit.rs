//! Print appearance selection after a real pointer press through the public Viewer API.
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

fn document() -> Vec<u8> {
    let red = appearance("1 0 0 rg 0 0 80 80 re f");
    let green = appearance("0 1 0 rg 0 0 80 80 re f");
    let blue = appearance("0 0 1 rg 0 0 80 80 re f");
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Square /Rect [10 10 90 90] /F 4 \
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

fn settle(viewer: &mut Viewer, events: Vec<Event>) -> Option<(usize, usize, usize)> {
    let mut colors = None;
    for event in events {
        let Event::NeedsRender(request) = event else {
            continue;
        };
        let raster = CpuRasterizer::new()
            .rasterize(&request.list, request.target)
            .expect("screen render is supported");
        colors = Some(count_colors(&raster));
        let more: Vec<_> = viewer
            .handle(Command::RenderReady {
                token: request.token,
                rendered: Rendered::Raster(raster),
            })
            .collect();
        colors = settle(viewer, more).or(colors);
    }
    colors
}

fn sheet() -> Sheet {
    Sheet {
        media: None,
        scale: 1.0,
        page_scale: 1.0,
    }
}

fn pigment_counts(viewer: &Viewer) -> (usize, usize, usize) {
    let Answer::PrintPage(page) = viewer.query(Query::PrintPage(0)) else {
        panic!("Print::Start was granted and the one page is available");
    };
    let target = page.target.expect("fixture fits printer budget");
    let raster = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&page.list, target)
        .expect("print display list rasterizes");
    count_colors(&raster)
}

fn count_colors(raster: &pdf_render::Raster) -> (usize, usize, usize) {
    let (mut red, mut green, mut blue) = (0, 0, 0);
    for pixel in raster.data.chunks_exact(4) {
        if pixel[0] > 200 && pixel[1] < 20 && pixel[2] < 20 {
            red += 1;
        }
        if pixel[1] > 200 && pixel[0] < 20 && pixel[2] < 20 {
            green += 1;
        }
        if pixel[2] > 200 && pixel[0] < 20 && pixel[1] < 20 {
            blue += 1;
        }
    }
    (red, green, blue)
}

#[test]
fn public_print_query_uses_normal_after_pointer_press_selected_down() {
    let mut viewer = Viewer::new(400, 400, 1.0);
    let bytes = document();
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("complete PDF fixture");
    assert!(!parsed.was_recovered(), "fixture has exact xref offsets");
    let opened: Vec<_> = viewer
        .handle(Command::Open {
            id: ID,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    let screen = settle(&mut viewer, opened).expect("opening produces a screen render");
    assert!(
        screen.0 > 0 && screen.1 == 0 && screen.2 == 0,
        "normal screen control: {screen:?}"
    );
    let begun: Vec<_> = viewer
        .handle(Command::Print(Printing::Start(sheet())))
        .collect();
    assert!(
        begun
            .iter()
            .any(|event| matches!(event, Event::Printing { .. }))
    );
    settle(&mut viewer, begun);
    let control = pigment_counts(&viewer);
    assert!(
        control.0 > 0 && control.1 == 0 && control.2 == 0,
        "normal print control: {control:?}"
    );
    let finished = viewer.handle(Command::Print(Printing::Finish)).collect();
    settle(&mut viewer, finished);

    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the open one-page document has geometry");
    };
    let at = (
        geometry.origin.0 + 50.0 * geometry.scale,
        geometry.origin.1 + 50.0 * geometry.scale,
    );
    let pressed: Vec<_> = viewer
        .handle(Command::Pointer {
            at,
            action: viewer_core::PointerAction::Pressed,
        })
        .collect();
    let screen = settle(&mut viewer, pressed).expect("press redraws the down appearance");
    assert!(
        screen.2 > 0 && screen.0 == 0 && screen.1 == 0,
        "press reached blue D on screen: {screen:?}"
    );

    let begun: Vec<_> = viewer
        .handle(Command::Print(Printing::Start(sheet())))
        .collect();
    settle(&mut viewer, begun);
    let colors = pigment_counts(&viewer);
    assert!(
        colors.0 > 0,
        "the print page should show red /N: {colors:?}"
    );
    assert_eq!(colors.1, 0, "print should not choose green /R: {colors:?}");
    assert_eq!(colors.2, 0, "print should not choose blue /D: {colors:?}");
}
