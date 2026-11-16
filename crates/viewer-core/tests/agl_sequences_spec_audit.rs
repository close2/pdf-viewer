//! ISO 32000-2 §9.10.2: AGL strings reach public text and search consumers.
//! Real Type 3 paths, exact xref and a blank raster control isolate text mapping.
#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "fixture invariants and bounded RGBA chunks must fail loudly"
)]

use std::fmt::Write as _;

use pdf_render::Rasterizer;
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;
use viewer_core::{Answer, Command, DocumentId, Event, Query, Rendered, Viewer};

const DOCUMENT: DocumentId = DocumentId(1);

fn one_page_with_named_type3_glyphs(page_content: &str) -> Vec<u8> {
    let charproc = "500 0 d0 0 0 m 500 0 l 500 500 l 0 500 l h f\n";
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
            .to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{page_content}\nendstream",
            page_content.len().saturating_add(1)
        ),
        "<< /Type /Font /Subtype /Type3 /Name /F1 /FontBBox [0 0 500 500] \
         /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs << /A 6 0 R /dalethatafpatah 7 0 R >> \
         /Encoding << /Type /Encoding /Differences [65 /A /dalethatafpatah] >> \
         /FirstChar 65 /LastChar 66 /Widths [500 500] /Resources << >> >>"
            .to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{charproc}endstream",
            charproc.len()
        ),
        format!(
            "<< /Length {} >>\nstream\n{charproc}endstream",
            charproc.len()
        ),
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = bodies.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn opened_and_drawn(id: DocumentId, content: &str) -> Viewer {
    let bytes = one_page_with_named_type3_glyphs(content);
    let document = Document::open(bytes.clone()).expect("exact PDF");
    assert!(!document.was_recovered());
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let interpreted = pdf_model::interpret(&document, &page);
    assert!(interpreted.is_complete(), "{:?}", interpreted.unsupported);
    let mut viewer = Viewer::new(400, 300, 1.0);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id,
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
        .expect("opening this page asks the host to render it");
    let raster = CpuRasterizer::new()
        .rasterize(&request.list, request.target)
        .expect("the small valid Type 3 paths render");
    let _ = viewer
        .handle(Command::RenderReady {
            token: request.token,
            rendered: Rendered::Raster(raster),
        })
        .count();
    viewer
}

fn controlled_viewer() -> Viewer {
    let viewer = opened_and_drawn(DOCUMENT, "BT /F1 12 Tf 20 40 Td (AB) Tj ET");

    // Both glyphs have actual closed, filled CharProcs above. Inspect the public frame so a
    // failed-font placeholder cannot satisfy this mapping test while silently skipping drawing.
    let Answer::Frame(frames) = viewer.query(Query::Frame) else {
        panic!("the valid Type 3 page produces a public frame");
    };
    let [frame] = frames.as_slice() else {
        panic!("the one-page viewer has one frame");
    };
    let ink = frame
        .raster
        .data
        .chunks_exact(4)
        .filter(|rgba| rgba[..3] != [255, 255, 255])
        .count();

    // Same page, font, resources and frame dimensions with no text painting. This controls for
    // page/background pixels and makes the independently authored Type 3 outlines observable.
    let blank = opened_and_drawn(DocumentId(2), " ");
    let Answer::Frame(blank_frames) = blank.query(Query::Frame) else {
        panic!("the blank control also produces a frame");
    };
    let [blank_frame] = blank_frames.as_slice() else {
        panic!("the blank control has one frame");
    };
    let blank_ink = blank_frame
        .raster
        .data
        .chunks_exact(4)
        .filter(|rgba| rgba[..3] != [255, 255, 255])
        .count();
    assert!(
        ink > blank_ink,
        "actual Type 3 paths add visible frame pixels"
    );
    let Answer::Text(pages) = viewer.query(Query::Text) else {
        panic!("shown page exposes text");
    };
    assert!(pages[0].text.starts_with('A'), "single-scalar control");
    viewer
}

#[test]
fn query_text_keeps_the_full_agl_string_for_one_glyph() {
    let viewer = controlled_viewer();
    let Answer::Text(pages) = viewer.query(Query::Text) else {
        panic!("shown page exposes text");
    };
    assert_eq!(pages[0].text.trim_end(), "A\u{05D3}\u{05B2}");
}

#[test]
fn find_matches_the_full_agl_string_for_one_glyph() {
    let viewer = controlled_viewer();
    assert!(matches!(viewer.query(Query::Find("A")), Answer::Found(matches) if matches.len() == 1));
    assert!(matches!(
        viewer.query(Query::Find("\u{05D3}\u{05B2}")),
        Answer::Found(matches) if matches.len() == 1
    ));
}
