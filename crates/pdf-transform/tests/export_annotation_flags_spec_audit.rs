//! Raster export follows the explicit screen-visibility policy from ADR 1179.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

mod support;

use std::fmt::Write as _;

use pdf_transform::render::{ImageFormat, RenderPlan, Sizing};
use pdf_transform::{Budget, Exit, MemorySinks, Plan, Policy, Source, apply};

fn exact_pdf(annotation: Option<i64>) -> Vec<u8> {
    let annots = annotation.map_or(String::new(), |_| "/Annots [5 0 R]".to_owned());
    let appearance = "1 0 0 rg 0 0 80 80 re f";
    let mut body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << >> /Contents 4 0 R {annots} >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\nendstream\nendobj\n"
    );
    let mut count = 4;
    if let Some(flags) = annotation {
        count = 6;
        let _ = write!(
            body,
            "5 0 obj\n<< /Type /Annot /Subtype /Square /Rect [10 10 90 90] /F {flags} \
             /AP << /N 6 0 R >> >>\nendobj\n\
             6 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 80 80] /Length {} >>\n\
             stream\n{appearance}\nendstream\nendobj\n",
            appearance.len() + 1
        );
    }
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(bytes.len());
        bytes.push_str(object);
    }
    let xref = bytes.len();
    let size = count + 1;
    let _ = writeln!(bytes, "xref\n0 {size}");
    bytes.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    bytes.into_bytes()
}

fn rendered_png(bytes: Vec<u8>) -> Vec<u8> {
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("valid PDF fixture");
    assert!(!parsed.was_recovered(), "the explicit xref is exact");
    let sinks = MemorySinks::new();
    let plan = Plan::Render(RenderPlan {
        source: 0,
        pages: "1".parse().expect("one page"),
        size: Sizing::Dpi(72.0),
        format: ImageFormat::Png,
        page_box: None,
        annotations: true,
        names: "page-%d.png".parse().expect("valid output pattern"),
        strips: Some(render_cpu::MAX_STRIPS),
    });
    let report = apply(
        &plan,
        &[Source::new(bytes)],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the render plan applies");
    assert_eq!(report.exit(false, false), Exit::Success, "{report:?}");
    let outputs = sinks.into_outputs();
    assert_eq!(outputs.len(), 1);
    outputs.into_iter().next().expect("one PNG").1
}

#[test]
fn raster_export_uses_screen_no_view_flags_not_print_flags() {
    // Table 167 says NoView suppresses screen rendering but Print allows paper. The raster verb
    // is an Export event, not paper; ADR 1179 explicitly chooses the screen column for flags.
    // Compare its PNG against a no-annotation control, then show that Print-only paints.
    let no_annotation = support::decode_png(&rendered_png(exact_pdf(None)));
    let no_view_print = support::decode_png(&rendered_png(exact_pdf(Some(36))));
    let print_only = support::decode_png(&rendered_png(exact_pdf(Some(4))));
    assert_eq!((no_annotation.0, no_annotation.1), (100, 100));
    assert_eq!(&no_annotation.2[20_200..20_204], [255, 255, 255, 255]);
    assert_eq!(&print_only.2[20_200..20_204], [255, 0, 0, 255]);
    assert_eq!(&print_only.2[..4], [255, 255, 255, 255]);
    assert_eq!(no_view_print, no_annotation);
    assert_ne!(print_only, no_annotation, "positive AP control paints red");
}
