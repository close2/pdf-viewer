//! A print-intent raster regression that separates Table 147's `/PrintArea` from `/PrintClip`.
//!
//! `/PrintArea /MediaBox` fixes the
//! output geometry at 400×400 while `/PrintClip /TrimBox` clips only the marks; the two controls
//! state no preferences (both defaults are `CropBox`) and the same print area with a `MediaBox` clip.

#![expect(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::panic,
    reason = "test fixture arithmetic and pixel indexing are bounded by this 400×400 hand-built PDF"
)]

use std::fmt::Write as _;

use pdf_model::page::Boundary;
use pdf_render::{Raster, Rasterizer, TargetSpec};
use render_cpu::CpuRasterizer;
use viewer_core::{Answer, Command, DocumentId, Event, Printing, Query, Rendered, Sheet, Viewer};

const CONTENT: &str = "0 g 30 30 40 40 re f\n0 g 150 150 40 40 re f";
const MEDIA: [f32; 4] = [0.0, 0.0, 400.0, 400.0];

/// A complete PDF 2.0 with exact content length, xref offsets, and a required 16-byte ID pair.
fn pdf(viewer_preferences: &str) -> Vec<u8> {
    let objects = [
        format!("<< /Type /Catalog /Pages 2 0 R {viewer_preferences} >>"),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        String::from(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] \
             /CropBox [20 20 380 380] /TrimBox [100 100 300 300] \
             /Resources << >> /Contents 4 0 R >>",
        ),
        format!(
            "<< /Length {} >>\nstream\n{CONTENT}\nendstream",
            CONTENT.len()
        ),
    ];
    let mut bytes = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = write!(bytes, "{} 0 obj\n{body}\nendobj\n", index + 1);
    }
    let xref_at = bytes.len();
    let _ = write!(
        bytes,
        "xref\n0 {}\n0000000000 65535 f \n",
        objects.len() + 1
    );
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {} /Root 1 0 R \
         /ID [<0123456789abcdef0123456789abcdef> \
         <0123456789abcdef0123456789abcdef>] >>\nstartxref\n{xref_at}\n%%EOF\n",
        objects.len() + 1
    );
    bytes.into_bytes()
}

/// Opens a source, answers its screen render, starts a print operation, and rasterises `PrintPage`.
fn printed(
    viewer_preferences: &str,
) -> (
    pdf_model::viewer_preferences::ViewerPreferences,
    Raster,
    TargetSpec,
) {
    let bytes = pdf(viewer_preferences);
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("the PDF 2.0 fixture parses");
    assert!(
        !parsed.was_recovered(),
        "declared xref is used without repair"
    );

    let mut viewer = Viewer::new(500, 500, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    let request = events
        .iter()
        .find_map(|event| match event {
            Event::NeedsRender(request) => Some(request),
            _ => None,
        })
        .expect("the source page is requested for the screen");
    let rendered = CpuRasterizer::new()
        .rasterize(&request.list, request.target)
        .expect("the screen fixture rasterises");
    let _ = viewer
        .handle(Command::RenderReady {
            token: request.token,
            rendered: Rendered::Raster(rendered),
        })
        .count();

    let Answer::Preferences(preferences) = viewer.query(Query::Preferences) else {
        panic!("the open document answers Table 147 preferences");
    };
    let _ = viewer
        .handle(Command::Print(Printing::Start(Sheet {
            media: Some(MEDIA),
            scale: 1.0,
            page_scale: 1.0,
        })))
        .count();
    let Answer::PrintPage(page) = viewer.query(Query::PrintPage(0)) else {
        panic!("the granted print operation returns page one");
    };
    assert!(page.reports.is_empty(), "fixture marks draw completely");
    let target = page
        .target
        .expect("the 400×400 page is within the pixel budget");
    let raster = CpuRasterizer::new()
        .rasterize(&page.list, target)
        .expect("the print-intent display list rasterises");
    assert_eq!((raster.width, raster.height), (target.width, target.height));
    (preferences, raster, target)
}

/// Samples a pixel whose coordinate is derived independently from the chosen PDF box.
/// One point is one pixel here, and raster y increases downward from the box's top.
fn marked(raster: &Raster, x: u32, y: u32) -> bool {
    let first = usize::try_from((y * raster.width + x) * 4).expect("small pixel offset");
    raster
        .data
        .get(first..first + 3)
        .expect("sample lies inside the raster")
        .iter()
        .all(|channel| *channel < 128)
}

/// Defaults are `CropBox` for both print entries; the named case proves `PrintArea` sets extent
/// independently while `PrintClip` removes marks outside its separately named box.
#[test]
fn print_area_sets_extent_while_print_clip_independently_clips_marks() {
    let (defaults, default_raster, default_target) = printed("");
    assert_eq!(defaults.print_area, Boundary::Crop);
    assert_eq!(defaults.print_clip, Boundary::Crop);
    assert_eq!((default_target.width, default_target.height), (360, 360));
    assert!(
        marked(&default_raster, 30, 330),
        "a mark inside the default crop boundary is visible"
    );

    let (trim_clip, clipped_raster, clipped_target) =
        printed("/ViewerPreferences << /PrintArea /MediaBox /PrintClip /TrimBox >>");
    assert_eq!(trim_clip.print_area, Boundary::Media);
    assert_eq!(trim_clip.print_clip, Boundary::Trim);
    assert_eq!((clipped_target.width, clipped_target.height), (400, 400));
    assert!(
        marked(&clipped_raster, 170, 230),
        "the mark inside the trim clip remains"
    );
    assert!(
        !marked(&clipped_raster, 50, 350),
        "the mark inside PrintArea but outside PrintClip is clipped"
    );

    let (media_clip, control_raster, control_target) =
        printed("/ViewerPreferences << /PrintArea /MediaBox /PrintClip /MediaBox >>");
    assert_eq!(media_clip.print_area, Boundary::Media);
    assert_eq!(media_clip.print_clip, Boundary::Media);
    assert_eq!((control_target.width, control_target.height), (400, 400));
    assert!(
        marked(&control_raster, 170, 230),
        "positive control: the inside mark remains with the wider clip"
    );
    assert!(
        marked(&control_raster, 50, 350),
        "positive control: changing only PrintClip to MediaBox restores the outer mark"
    );
}
