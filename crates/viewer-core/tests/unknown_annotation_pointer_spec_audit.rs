//! Public pointer-state test for §12.5.5's unsupported-annotation N fallback.
#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "controlled PDF fixture and literal solid-color assertions should fail loudly"
)]

use std::fmt::Write as _;

use pdf_render::Rasterizer;
use render_cpu::CpuRasterizer;
use viewer_core::{Answer, Command, DocumentId, Event, PointerAction, Query, Rendered, Viewer};

const ID: DocumentId = DocumentId(77);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pigment {
    Red,
    Green,
    Blue,
}

fn appearance(content: &str) -> String {
    format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 80 80] /Resources << >> /Length {} >>\nstream\n{content}\nendstream",
        content.len() + 1
    )
}

fn pdf(subtype: &str, extension: bool) -> Vec<u8> {
    let red = appearance("1 0 0 rg 0 0 80 80 re f");
    let green = appearance("0 1 0 rg 0 0 80 80 re f");
    let blue = appearance("0 0 1 rg 0 0 80 80 re f");
    let extensions = if extension {
        // GLGR is a registered-prefix example in Annex E / Table 49. The test's synthetic
        // extension defines only an otherwise-unknown annotation type with a stored AP fallback.
        "/Extensions << /GLGR << /BaseVersion /1.7 /ExtensionLevel 1002 >> >>"
    } else {
        ""
    };
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R {extensions} >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /Resources << >> >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /{subtype} /Rect [10 10 90 90] /F 4 /AP << /N 6 0 R /R 7 0 R /D 8 0 R >> >>\nendobj\n\
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

fn pigment(raster: &pdf_render::Raster) -> Pigment {
    let mut red = 0usize;
    let mut green = 0usize;
    let mut blue = 0usize;
    for pixel in raster.data.as_chunks::<4>().0 {
        if pixel[0] > 200 && pixel[1] < 20 && pixel[2] < 20 && pixel[3] > 0 {
            red += 1;
        }
        if pixel[1] > 200 && pixel[0] < 20 && pixel[2] < 20 && pixel[3] > 0 {
            green += 1;
        }
        if pixel[2] > 200 && pixel[0] < 20 && pixel[1] < 20 && pixel[3] > 0 {
            blue += 1;
        }
    }
    match (red > 100, green > 100, blue > 100) {
        (true, false, false) => Pigment::Red,
        (false, true, false) => Pigment::Green,
        (false, false, true) => Pigment::Blue,
        colors => panic!("expected exactly one full-annotation color, got {colors:?}"),
    }
}

fn settle(viewer: &mut Viewer, events: Vec<Event>) -> Option<Pigment> {
    let mut pending = events;
    let mut final_pigment = None;
    for _ in 0..8 {
        if pending.is_empty() {
            break;
        }
        let mut next = Vec::new();
        for event in pending {
            let Event::NeedsRender(request) = event else {
                continue;
            };
            let raster = CpuRasterizer::new()
                .rasterize(&request.list, request.target)
                .expect("screen display list renders");
            final_pigment = Some(pigment(&raster));
            next.extend(viewer.handle(Command::RenderReady {
                token: request.token,
                rendered: Rendered::Raster(raster),
            }));
        }
        pending = next;
    }
    assert!(
        pending.is_empty(),
        "render loop settles within the fixture bound"
    );
    final_pigment
}

fn public_pointer_states(subtype: &str, extension: bool) -> [Pigment; 3] {
    let bytes = pdf(subtype, extension);
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("fixture parses");
    assert!(
        !parsed.was_recovered(),
        "fixture uses exact classic xref offsets"
    );

    let mut viewer = Viewer::new(400, 400, 1.0);
    let opened = viewer
        .handle(Command::Open {
            id: ID,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    let normal = settle(&mut viewer, opened).expect("open renders N");
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("one page has geometry");
    };
    let at = (
        geometry.origin.0 + 50.0 * geometry.scale,
        geometry.origin.1 + 50.0 * geometry.scale,
    );
    let moved = viewer
        .handle(Command::Pointer {
            at,
            action: PointerAction::Moved,
        })
        .collect();
    let rollover = settle(&mut viewer, moved).unwrap_or(normal);
    let pressed = viewer
        .handle(Command::Pointer {
            at,
            action: PointerAction::Pressed,
        })
        .collect();
    let down = settle(&mut viewer, pressed).unwrap_or(rollover);
    [normal, rollover, down]
}

#[test]
fn unsupported_extension_annotation_uses_n_while_supported_square_uses_r_and_d() {
    // The extension subtype has no handler in this reader; only Table 166 common fields and the
    // Table 170 AP fallback are defined in this fixture. §12.5.5 says such a type is rendered
    // with N. The Square case is a positive control for actual public pointer dispatch and for
    // distinct R/D stream wiring.
    assert_eq!(
        public_pointer_states("Square", false),
        [Pigment::Red, Pigment::Green, Pigment::Blue],
        "known annotation follows Table 170 N/R/D"
    );
    assert_eq!(
        public_pointer_states("GLGR_R2PointerProbe", true),
        [Pigment::Red, Pigment::Red, Pigment::Red],
        "unknown-type fallback stays on N"
    );
}
