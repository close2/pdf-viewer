//! Structure Alt/E language escapes reach the public accessibility consumer.
#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "bounded exact-xref fixture and loud positive controls"
)]
use pdf_render::Rasterizer;

fn check_phrase(entry: &str) {
    use std::fmt::Write as _;

    use pdf_syntax::Document;
    use viewer_core::{Answer, Command, DocumentId, Query, Viewer};

    let content = "BT /F1 12 Tf 10 50 Td \
        /P << /MCID 0 >> BDC (control) Tj EMC \
        /P << /MCID 1 >> BDC (target) Tj EMC ET";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /Lang (fr-FR) \
          /StructTreeRoot 6 0 R /MarkInfo << /Marked true >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
          /Resources << /Font << /F1 5 0 R >> >> /StructParents 0 /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n\
         6 0 obj\n<< /Type /StructTreeRoot /K 7 0 R /ParentTree 10 0 R >>\nendobj\n\
         7 0 obj\n<< /Type /StructElem /S /Sect /P 6 0 R /Lang (en-US) /Pg 3 0 R \
          /K [8 0 R 9 0 R] >>\nendobj\n\
         8 0 obj\n<< /Type /StructElem /S /P /P 7 0 R /Pg 3 0 R \
          /{entry} (ordinary phrase) /K [<< /Type /MCR /Pg 3 0 R /MCID 0 >>] >>\nendobj\n\
         9 0 obj\n<< /Type /StructElem /S /P /P 7 0 R /Pg 3 0 R \
          /{entry} <FEFF001B00650073001B0074006F006D00610074006F> \
          /K [<< /Type /MCR /Pg 3 0 R /MCID 1 >>] >>\nendobj\n\
         10 0 obj\n<< /Nums [0 [8 0 R 9 0 R]] >>\nendobj\n",
        content.len(),
    );
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(bytes.len());
        bytes.push_str(object);
    }
    let xref_at = bytes.len();
    let size = offsets.len() + 1;
    let _ = writeln!(bytes, "xref\n0 {size}");
    bytes.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    let bytes = bytes.into_bytes();
    let syntax = Document::open(bytes.clone()).expect("exact-xref PDF fixture opens");
    assert!(!syntax.was_recovered(), "fixture offsets are exact");
    let page = pdf_model::Pages::new(&syntax).get(0).expect("one page");
    let interpreted = pdf_model::interpret(&syntax, &page);
    assert!(interpreted.is_complete(), "complete interpretation");
    assert!(
        interpreted.glyphs > 0,
        "ordinary text is a positive control"
    );
    assert!(interpreted.text.contains("control"));
    assert!(
        interpreted.text.contains("target"),
        "Alt is a spoken substitution, not extraction"
    );

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
        .expect("render request");
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
        panic!("an open document answers the accessibility tree");
    };
    let nodes: Vec<_> = pages.iter().flat_map(|page| page.nodes.iter()).collect();
    let control = nodes.iter().find(|node| node.name == "ordinary phrase");
    let target = nodes.iter().find(|node| node.name == "tomato");
    assert_eq!(
        control.map(|node| node.language.as_deref()),
        Some(Some("en-US"))
    );
    assert_eq!(
        target.map(|node| node.language.as_deref()),
        Some(Some("es")),
        "the {entry} string's language escape overrides its element Lang"
    );
}

#[test]
fn structure_alt_escape_overrides_ancestral_language() {
    check_phrase("Alt");
}

#[test]
fn structure_expansion_escape_overrides_ancestral_language() {
    check_phrase("E");
}
