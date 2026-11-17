//! Table 383's flattened-field semantics must survive the public accessibility answer.
//! These are information-loss regressions, not a prescribed platform role or interaction API.
//! A future fix needs further exact role/state/name assertions at its chosen consumer boundary.
#![expect(
    clippy::panic,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "bounded exact-xref test fixtures and loud fixture failures"
)]

use pdf_render::Rasterizer;
use render_cpu::CpuRasterizer;
use std::fmt::Write as _;
use viewer_core::{AccessibilityNode, Answer, Command, DocumentId, Event, Query, Rendered, Viewer};

fn fixture(role: &str, checked: &str, description: &str, title: &str) -> Vec<u8> {
    let marks = if checked == "on" {
        "10 10 20 20 re S 15 15 10 10 re f"
    } else {
        "10 10 20 20 re S"
    };
    let content = format!("/Form << /MCID 0 >> BDC {marks} EMC");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 5 0 R /MarkInfo << /Marked true >> >>".to_owned(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Contents 4 0 R /StructParents 0 >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        "<< /Type /StructTreeRoot /K [6 0 R] /ParentTree 8 0 R /ParentTreeNextKey 1 >>".to_owned(),
        "<< /Type /StructElem /S /Document /P 5 0 R /K [7 0 R] >>".to_owned(),
        format!("<< /Type /StructElem /S /Form /P 6 0 R /Pg 3 0 R /K 0 /T ({title}) /A << /O /PrintField /Role /{role} /Checked /{checked} /Desc ({description}) >> >>"),
        "<< /Nums [0 [7 0 R]] >>".to_owned(),
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index + 1);
    }
    let xref = out.len();
    let _ = write!(out, "xref\n0 9\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size 9 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

fn answer(role: &str, checked: &str, description: &str, title: &str) -> Vec<AccessibilityNode> {
    let bytes = fixture(role, checked, description, title);
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("exact fixture");
    assert!(!parsed.was_recovered());
    let page = pdf_model::Pages::new(&parsed).get(0).expect("page");
    assert!(pdf_model::interpret(&parsed, &page).is_complete());
    let mut viewer = Viewer::new(800, 1000, 1.0);
    let events: Vec<_> = viewer
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
        .expect("render request");
    let raster = CpuRasterizer::new()
        .rasterize(&request.list, request.target)
        .expect("CPU render");
    viewer
        .handle(Command::RenderReady {
            token: request.token,
            rendered: Rendered::Raster(raster),
        })
        .for_each(drop);
    let Answer::Accessibility(pages) = viewer.query(Query::AccessibilityTree) else {
        panic!("accessibility answer")
    };
    assert_eq!(pages.len(), 1);
    let nodes: Vec<_> = pages.into_iter().flat_map(|page| page.nodes).collect();
    assert_eq!(nodes.len(), 2, "Document and Form are both reached");
    assert_eq!(nodes[1].role, "Form");
    nodes
}

#[test]
fn flattened_field_title_control_reaches_the_public_answer() {
    assert_ne!(
        answer("cb", "off", "Agree", "first"),
        answer("cb", "off", "Agree", "second")
    );
}

#[test]
fn flattened_field_role_is_not_lost_before_the_accessibility_consumer() {
    assert_ne!(
        answer("cb", "off", "Agree", ""),
        answer("rb", "off", "Agree", ""),
        "a check box and radio button are distinct Table 383 field identities"
    );
}

#[test]
fn flattened_field_checked_state_is_not_lost_before_the_accessibility_consumer() {
    assert_ne!(
        answer("cb", "off", "Agree", ""),
        answer("cb", "on", "Agree", ""),
        "the public answer must distinguish a checked field from an unchecked field"
    );
}

#[test]
fn flattened_field_description_is_not_lost_before_the_accessibility_consumer() {
    assert_ne!(
        answer("cb", "off", "Agree", ""),
        answer("cb", "off", "Decline", ""),
        "the field's alternate name must reach a nonvisual consumer"
    );
}
