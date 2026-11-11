//! ISO 32000-2 Annex O: multiword search ordering and zoom offset coordinates.

#![expect(
    clippy::panic,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "audit integration fixture"
)]

use pdf_render::Rasterizer;
use render_cpu::CpuRasterizer;
use viewer_core::{Answer, Command, DocumentId, Event, Find, Query, Rendered, Viewer};

const DOC: DocumentId = DocumentId(1);

fn one_page_pdf(text: &str) -> Vec<u8> {
    let content = format!("BT /F1 12 Tf 10 350 Td ({text}) Tj ET");
    let stream = format!(
        "<< /Length {} >>\nstream\n{}\nendstream",
        content.len(),
        content
    );
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 400] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        stream,
    ];
    assembled(&objects)
}

fn assembled(objects: &[String]) -> Vec<u8> {
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = vec![0_usize];
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
    }
    let xref = bytes.len();
    bytes.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets.iter().skip(1) {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    bytes
}

fn open(fragment: &str) -> (Viewer, Vec<Event>) {
    open_bytes(one_page_pdf("late early"), fragment)
}

fn open_bytes(bytes: Vec<u8>, fragment: &str) -> (Viewer, Vec<Event>) {
    let mut viewer = Viewer::new(100, 100, 1.0);
    let document = pdf_syntax::Document::open(bytes.clone()).expect("valid page fixture");
    assert!(!document.was_recovered(), "the declared xref is used");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    assert!(pdf_model::interpret(&document, &page).is_complete());
    let events = viewer
        .handle(Command::Open {
            id: DOC,
            bytes: bytes.into(),
            password: None,
            fragment: Some(fragment.to_owned()),
        })
        .collect();
    (viewer, events)
}

fn settle(viewer: &mut Viewer, events: &[Event]) {
    let mut pending: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            Event::NeedsRender(request) => Some(request.clone()),
            _ => None,
        })
        .collect();
    for _ in 0..4 {
        let Some(request) = pending.pop() else { break };
        let raster = CpuRasterizer::new()
            .rasterize(&request.list, request.target)
            .expect("the positive-control PDF page rasterizes");
        let answered: Vec<Event> = viewer
            .handle(Command::RenderReady {
                token: request.token,
                rendered: Rendered::Raster(raster),
            })
            .collect();
        pending.extend(answered.iter().filter_map(|event| match event {
            Event::NeedsRender(next) => Some(next.clone()),
            _ => None,
        }));
    }
    assert!(pending.is_empty(), "all requested renders have settled");
}

fn origin(viewer: &Viewer) -> (f32, f32) {
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("page geometry is available after opening the positive control");
    };
    geometry.origin
}

#[test]
fn fragment_search_word_list_selects_earliest_text_occurrence() {
    let (mut viewer, mut events) = open("search=%22early%20late%22");
    let mut found = None;
    for _ in 0..2 {
        settle(&mut viewer, &events);
        for event in &events {
            if let Event::Searched {
                found: at, wrapped, ..
            } = event
            {
                assert!(!wrapped, "Annex O search does not wrap");
                found = *at;
            }
        }
        if found.is_some() {
            break;
        }
        events = viewer.handle(Command::Find(Find::Continue)).collect();
    }
    assert!(
        found.is_some(),
        "one of the positive-control words is found"
    );
    let Answer::Selected(selected) = viewer.query(Query::Selection) else {
        panic!("the first matching word is selected");
    };
    assert_eq!(
        selected.text.to_lowercase(),
        "late",
        "the earliest content occurrence wins although `early` is listed first"
    );
}

#[test]
fn zoom_offsets_use_both_page_top_left_coordinates() {
    for (percent, scale) in [(100, 1.0_f32), (200, 2.0)] {
        let (mut bare, bare_events) = open(&format!("zoom={percent}"));
        settle(&mut bare, &bare_events);
        let (mut positioned, events) = open(&format!("zoom={percent},20,30"));
        settle(&mut positioned, &events);
        let (base_x, base_y) = origin(&bare);
        let (at_x, at_y) = origin(&positioned);
        assert!(
            base_x.abs() < 0.01 && base_y.abs() < 0.01,
            "unscrolled page begins at viewport origin"
        );
        assert!(
            (at_x + 20.0 * scale).abs() < 0.01,
            "left offset scales from page units: {at_x}"
        );
        assert!(
            (at_y + 30.0 * scale).abs() < 0.01,
            "top offset scales from page units: {at_y}"
        );
    }
}

#[test]
fn fragment_page_overrides_the_document_open_action() {
    // O.2.2 recommends applying open parameters after the document's own.
    // The paired fixture must first prove its OpenAction really selects page two.
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /OpenAction [4 0 R /Fit] >>",
        "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 400] /Resources << >> >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 400] /Resources << >> >>",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<Vec<_>>();
    let bytes = assembled(&objects);
    let (mut control, events) = open_bytes(bytes.clone(), "");
    settle(&mut control, &events);
    assert!(
        matches!(
            control.query(Query::CurrentPage),
            Answer::Page {
                index: 1,
                of: 2,
                ..
            }
        ),
        "document OpenAction selects page two"
    );
    let (mut overridden, events) = open_bytes(bytes, "page=1");
    settle(&mut overridden, &events);
    assert!(
        matches!(
            overridden.query(Query::CurrentPage),
            Answer::Page {
                index: 0,
                of: 2,
                ..
            }
        ),
        "fragment selects page one after the document's action"
    );
}
