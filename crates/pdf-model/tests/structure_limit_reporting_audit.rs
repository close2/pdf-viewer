//! Integration tests for §14.7.2 hierarchy reader truncation reporting.
//!
//! The effective specification gives no maximum /K fan-out or hierarchy depth. A reader may
//! impose resource limits, but the public reading must distinguish a complete tree from a
//! prefix. This is the public `Tree::walk` reporting contract; no mandatory ISO error
//! response for a resource limit is inferred.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "bounded exact-xref fixtures and explicit assertion failures"
)]

use std::fmt::Write as _;

use pdf_model::structure::Tree;
use pdf_syntax::{Document, FileBytes};

fn exact_pdf(objects: &[String]) -> Document {
    let mut bytes = String::from("%PDF-2.0\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        writeln!(bytes, "{} 0 obj\n{object}\nendobj", index + 1)
            .expect("String writes do not fail");
    }
    let xref = bytes.len();
    writeln!(bytes, "xref\n0 {}\n0000000000 65535 f ", objects.len() + 1)
        .expect("String writes do not fail");
    for offset in offsets {
        writeln!(bytes, "{offset:010} 00000 n ").expect("String writes do not fail");
    }
    write!(
        bytes,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    )
    .expect("String writes do not fail");
    let document =
        Document::open(FileBytes::from(bytes.into_bytes())).expect("complete PDF fixture");
    assert!(!document.was_recovered(), "fixture uses exact xref offsets");
    document
}

fn fanout_document(children: usize) -> Document {
    let first = 5usize;
    let entries = (0..children)
        .map(|index| format!("{} 0 R", first + index))
        .collect::<Vec<_>>()
        .join(" ");
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>".to_owned(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << >> >>".to_owned(),
        format!("<< /Type /StructTreeRoot /K [{entries}] >>"),
    ];
    objects.extend((0..children).map(|_| "<< /Type /StructElem /S /P /P 4 0 R >>".to_owned()));
    exact_pdf(&objects)
}

fn deep_document(depth: usize) -> Document {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>".to_owned(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << >> >>".to_owned(),
    ];
    let first = 5usize;
    let last = first + depth.saturating_sub(1);
    objects.push(format!("<< /Type /StructTreeRoot /K {first} 0 R >>"));
    for id in first..=last {
        let parent = if id == first { 4 } else { id - 1 };
        let k = if id == last {
            String::new()
        } else {
            format!("/K {} 0 R", id + 1)
        };
        objects.push(format!("<< /Type /StructElem /S /P /P {parent} 0 R {k} >>"));
    }
    exact_pdf(&objects)
}

#[test]
fn a_complete_small_hierarchy_is_not_reported_as_truncated() {
    let document = fanout_document(3);
    let tree = Tree::of(&document).expect("structure tree root");
    let reading = tree.walk(&document);
    assert_eq!(reading.items.len(), 3);
    assert!(!reading.truncated);
}

#[test]
fn a_large_k_array_is_either_read_whole_or_reported_truncated() {
    // Positive control above stays well below the reader's bound. The large file is still a
    // syntactically complete PDF; each /K member is a structure-element dictionary whose /P
    // names the tree root. If a reader chooses a smaller output bound it must expose the cut.
    let count = 65_537;
    let document = fanout_document(count);
    let tree = Tree::of(&document).expect("structure tree root");
    let reading = tree.walk(&document);
    assert!(
        reading.items.len() == count || reading.truncated,
        "a reader may bound /K fan-out, but must not present a prefix as a complete hierarchy: read {}, truncated={}",
        reading.items.len(),
        reading.truncated
    );
}

#[test]
fn a_deep_hierarchy_is_either_read_whole_or_reported_truncated() {
    // Every /P is an indirect reference to the immediate parent. A finite acyclic chain is
    // allowed by the tree model; the fixture has 70 structure elements.
    let depth = 70;
    let control = deep_document(3);
    let small = Tree::of(&control).expect("small root").walk(&control);
    assert_eq!(small.items.len(), 3);
    assert!(!small.truncated);
    let document = deep_document(depth);
    let tree = Tree::of(&document).expect("structure tree root");
    let reading = tree.walk(&document);
    assert!(
        reading.items.len() == depth || reading.truncated,
        "a reader may bound depth, but must not present a prefix as complete: read {}, truncated={}",
        reading.items.len(),
        reading.truncated
    );
}
