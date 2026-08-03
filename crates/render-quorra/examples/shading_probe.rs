//! Scratch: what paints a point of issue10572, and with which shading geometry.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::collapsible_if,
    missing_docs
)]

use pdf_render::{ShadingKind, TargetSpec};
use pdf_syntax::Document;

fn main() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../pdf-viewer/test/pdfs/issue10572.pdf");
    let path = if path.exists() {
        path
    } else {
        // corpus location
        std::path::Path::new("/home/cl/projects/pdf-viewer/doc/pdf.js/test/pdfs/issue10572.pdf")
            .into()
    };
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let document = Document::open(bytes).expect("opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0).expect("exists");
    let list = pdf_model::content::interpret(&document, &page).display_list;
    let target = TargetSpec::for_page(&list, 1.0, 1 << 30).unwrap();
    println!(
        "target {}x{} transform {:?}",
        target.width, target.height, target.transform
    );
    let probe = (61.0_f32, 151.0_f32);
    for (i, command) in list.commands().iter().enumerate() {
        if let Some(bounds) = command.device_bounds(target.transform) {
            if bounds.min.x <= probe.0
                && probe.0 <= bounds.max.x
                && bounds.min.y <= probe.1
                && probe.1 <= bounds.max.y
            {
                match command {
                    pdf_render::Command::Fill {
                        paint: pdf_render::Paint::Shading(shading),
                        transform,
                        ..
                    } => {
                        println!(
                            "#{i}: Fill shading transform={:?} cmd-transform={transform:?}",
                            shading.transform
                        );
                        match shading.kind.as_ref() {
                            ShadingKind::Axial {
                                start,
                                end,
                                ramp,
                                extend,
                            } => println!(
                                "  Axial start=({},{}) end=({},{}) stops={} extend={extend:?}",
                                start.x,
                                start.y,
                                end.x,
                                end.y,
                                ramp.stops.len()
                            ),
                            ShadingKind::Radial { .. } => println!("  Radial"),
                            ShadingKind::Sampled {
                                domain,
                                width,
                                height,
                                ..
                            } => println!("  Sampled domain={domain:?} grid={width}x{height}"),
                            ShadingKind::Mesh { triangles } => {
                                println!("  Mesh {} triangles", triangles.len());
                            }
                            other => println!("  {other:?}"),
                        }
                    }
                    other => {
                        let name = match other {
                            pdf_render::Command::Fill { .. } => "Fill(solid)",
                            pdf_render::Command::Stroke { .. } => "Stroke",
                            pdf_render::Command::Image { .. } => "Image",
                            pdf_render::Command::Group { .. } => "Group",
                            _ => "?",
                        };
                        let w = bounds.max.x - bounds.min.x;
                        let h = bounds.max.y - bounds.min.y;
                        if w < 3000.0 && h < 3000.0 {
                            println!("#{i}: {name} {w:.0}x{h:.0}");
                        }
                    }
                }
            }
        }
    }
}
