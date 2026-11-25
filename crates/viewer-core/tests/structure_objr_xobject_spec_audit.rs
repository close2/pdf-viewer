//! Specification audit of ISO 32000-2 §14.7.5.3 (review 2229): an element whose content item is
//! a whole form `XObject` is spoken with that `XObject`'s text.
//!
//! §14.7.5.3 identifies a structure element's content that is an entire PDF object, such as an
//! `XObject` the page draws, by an object reference dictionary in the element's `/K`.
//! §14.7.5.4's EXAMPLE 1 is this shape exactly — a form stating `/StructParent`, drawn by the
//! page's content stream, named by an `/OBJR`. The text the form draws is the element's content,
//! as the text inside `/P <</MCID 0>> BDC /Fm1 Do EMC` is (§14.7.5.2's first way). The control is
//! that second arrangement; the failing half is the object reference, whose element is named by
//! nothing.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "exact-xref fixture and accessibility assertions"
)]

use std::fmt::Write as _;

use pdf_render::Rasterizer;
use viewer_core::{Answer, Command, DocumentId, Query, Viewer};

/// A tagged page drawing a form whose text is `Body`, as the one `P` element's content: through
/// an `/OBJR` naming the form (`as_object`) or through a marked-content sequence around its `Do`.
fn document(as_object: bool) -> Vec<u8> {
    let (content, form, kids, nums) = if as_object {
        (
            "/Fm1 Do",
            "/StructParent 1 ",
            "<< /Type /OBJR /Obj 8 0 R /Pg 3 0 R >>",
            "0 [] 1 7 0 R",
        )
    } else {
        ("/P <</MCID 0>> BDC /Fm1 Do EMC", "", "0", "0 [7 0 R]")
    };
    let drawn = "BT /F1 12 Tf 10 50 Td (Body) Tj ET";
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 6 0 R /MarkInfo << /Marked true >> \
         /Lang (en) >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 100] \
         /Resources << /XObject << /Fm1 8 0 R >> >> /Contents 4 0 R /StructParents 0 >>"
            .to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len().saturating_add(1)
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        format!(
            "<< /Type /StructTreeRoot /K 7 0 R /ParentTree << /Nums [{nums}] >> \
             /ParentTreeNextKey 2 >>"
        ),
        format!("<< /Type /StructElem /S /P /P 6 0 R /Pg 3 0 R /K {kids} >>"),
        format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 300 100] {form}\
             /Resources << /Font << /F1 5 0 R >> >> /Length {} >>\nstream\n{drawn}\nendstream",
            drawn.len().saturating_add(1)
        ),
    ];
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{body}\nendobj", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// The names of the page's accessibility nodes, after the first page has been drawn.
fn names(bytes: Vec<u8>) -> Vec<String> {
    let syntax = pdf_syntax::Document::open(bytes.clone()).expect("exact-xref fixture opens");
    assert!(!syntax.was_recovered(), "all xref offsets are exact");
    let mut viewer = Viewer::new(800, 600, 1.0);
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
            viewer_core::Event::NeedsRender(request) => Some(request),
            _ => None,
        })
        .expect("opening the document requests a render");
    let raster = render_cpu::CpuRasterizer::new()
        .rasterize(&request.list, request.target)
        .expect("CPU render");
    viewer
        .handle(Command::RenderReady {
            token: request.token,
            rendered: viewer_core::Rendered::Raster(raster),
        })
        .for_each(drop);
    let Answer::Accessibility(pages) = viewer.query(Query::AccessibilityTree) else {
        panic!("an open document answers with an accessibility tree");
    };
    pages
        .iter()
        .flat_map(|page| page.nodes.iter())
        .map(|node| node.name.clone())
        .collect()
}

/// 2229-B01. The control: the form drawn inside the element's marked-content sequence is spoken
/// as the element. The failing half: the same form named by the element's object reference.
#[test]
fn an_element_whose_content_is_a_form_xobject_is_spoken_with_its_text_spec_audit() {
    let marked = names(document(false));
    assert!(
        marked.iter().any(|name| name.contains("Body")),
        "control: the form drawn inside /P <</MCID 0>> BDC … EMC is the element's text: {marked:?}"
    );
    let referenced = names(document(true));
    assert!(
        referenced.iter().any(|name| name.contains("Body")),
        "§14.7.5.3: the form an /OBJR names is the element's content, and its text is spoken \
         with nothing: {referenced:?}"
    );
}
