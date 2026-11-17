//! Annex O public coordinate-transform integration control for the current top-left convention.
//! The expected rectangle is derived from `CropBox`, `/Rotate`, and `/UserUnit`.
//! The source states default-user-space units and a page top-left origin; this test pins
//! the current policy of using the unrotated display box top-left before page rotation.
//! It does not establish that this interpretation of the rotated-page origin is uniquely required.

#![expect(clippy::expect_used, clippy::panic, clippy::arithmetic_side_effects)]

use pdf_render::Rasterizer;
use render_cpu::CpuRasterizer;
use std::fmt::Write as _;
use viewer_core::{Answer, Command, DocumentId, Event, Query, Rendered, Viewer};

fn page_pdf(rotate: u16) -> Vec<u8> {
    let page = format!(
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 500] /CropBox [20 30 220 330] /Rotate {rotate} /UserUnit 2 /Resources << >> >>"
    );
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_owned(),
        page,
    ];
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index + 1);
    }
    let xref_at = out.len();
    let _ = write!(out, "xref\n0 4\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size 4 /Root 1 0 R /ID [<00112233445566778899AABBCCDDEEFF> <00112233445566778899AABBCCDDEEFF>] >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn inspect(rotate: u16) -> ([f32; 8], viewer_core::PageGeometry) {
    let bytes = page_pdf(rotate);
    let document = pdf_syntax::Document::open(bytes.clone()).expect("valid exact xref");
    assert!(!document.was_recovered());
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    assert!(pdf_model::interpret(&document, &page).is_complete());

    let mut viewer = Viewer::new(800, 1000, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: Some("highlight=10,60,40,80".to_owned()),
        })
        .collect();
    let mut outstanding: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            Event::NeedsRender(request) => Some(request.clone()),
            _ => None,
        })
        .collect();
    for _ in 0..4 {
        let Some(request) = outstanding.pop() else {
            break;
        };
        let raster = CpuRasterizer::new()
            .rasterize(&request.list, request.target)
            .expect("valid page raster");
        let next: Vec<Event> = viewer
            .handle(Command::RenderReady {
                token: request.token,
                rendered: Rendered::Raster(raster),
            })
            .collect();
        outstanding.extend(next.iter().filter_map(|event| match event {
            Event::NeedsRender(request) => Some(request.clone()),
            _ => None,
        }));
    }
    assert!(outstanding.is_empty(), "all requested renders settled");
    let Answer::Highlighted(quads) = viewer.query(Query::Highlight) else {
        panic!("one requested highlight")
    };
    let [quad] = quads.as_slice() else {
        panic!("one rectangle: {quads:?}")
    };
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("page geometry")
    };
    (*quad, geometry)
}

fn bounds(quad: &[f32; 8]) -> (f32, f32, f32, f32) {
    let xs = [quad[0], quad[2], quad[4], quad[6]];
    let ys = [quad[1], quad[3], quad[5], quad[7]];
    (
        xs.into_iter().fold(f32::INFINITY, f32::min),
        ys.into_iter().fold(f32::INFINITY, f32::min),
        xs.into_iter().fold(f32::NEG_INFINITY, f32::max),
        ys.into_iter().fold(f32::NEG_INFINITY, f32::max),
    )
}

#[test]
fn annex_o_highlight_uses_unrotated_crop_top_left_then_page_transform() {
    let (rotated, rotated_page) = inspect(90);
    assert_eq!(
        (rotated_page.page.width, rotated_page.page.height),
        (600.0, 400.0)
    );
    // In default user space, the URI's rectangle is x=[30,80], y=[250,290].
    // CropBox shift then clockwise display rotation maps its displayed bounds to
    // x=[440,520], y-up=[280,380] on a 600x400 page; screen-top y is 400-y=[20,120].
    let expected = (
        rotated_page.origin.0 + 440.0 * rotated_page.scale,
        rotated_page.origin.1 + 20.0 * rotated_page.scale,
        rotated_page.origin.0 + 520.0 * rotated_page.scale,
        rotated_page.origin.1 + 120.0 * rotated_page.scale,
    );
    let actual = bounds(&rotated);
    for (got, want) in [actual.0, actual.1, actual.2, actual.3]
        .into_iter()
        .zip([expected.0, expected.1, expected.2, expected.3])
    {
        assert!(
            (got - want).abs() < 0.75,
            "rotated bounds {actual:?}, expected {expected:?}"
        );
    }

    let (plain, plain_page) = inspect(0);
    assert_eq!(
        (plain_page.page.width, plain_page.page.height),
        (400.0, 600.0)
    );
    // Without rotation, offset from the cropped top-left is (10,40), at two physical units each.
    let expected = (
        plain_page.origin.0 + 20.0 * plain_page.scale,
        plain_page.origin.1 + 80.0 * plain_page.scale,
        plain_page.origin.0 + 120.0 * plain_page.scale,
        plain_page.origin.1 + 160.0 * plain_page.scale,
    );
    let actual = bounds(&plain);
    for (got, want) in [actual.0, actual.1, actual.2, actual.3]
        .into_iter()
        .zip([expected.0, expected.1, expected.2, expected.3])
    {
        assert!(
            (got - want).abs() < 0.75,
            "unrotated control bounds {actual:?}, expected {expected:?}"
        );
    }
}
