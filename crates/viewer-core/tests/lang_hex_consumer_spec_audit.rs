//! Public language regression for EC3 issue #330.
//!
//! Exact-xref PDF. The catalog language is UTF-16BE hex (`fr-FR`); a literal language on an
//! ancestor structure element supplies an overridden value, while a hex language on its child overrides
//! that ancestor (`es-MX`). A second branch omits structure `/Lang` and therefore inherits the
//! catalog language. This reaches the same public accessibility-node consumer as the existing
//! phrase-language test rather than stopping at generic PDF string decoding.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "the exact-xref fixture and positive controls should fail loudly"
)]

use std::fmt::Write as _;

use pdf_render::Rasterizer;
use pdf_syntax::Document;
use viewer_core::{Answer, Command, DocumentId, Query, Viewer};

fn fixture() -> Vec<u8> {
    let content = "BT /F1 12 Tf 10 60 Td \
        /P << /MCID 0 >> BDC (explicit structure Lang) Tj EMC \
        /P << /MCID 1 >> BDC (catalog Lang fallback) Tj EMC ET";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R \
          /Lang <FEFF00660072002D00460052> /StructTreeRoot 6 0 R \
          /MarkInfo << /Marked true >> >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] \
          /Resources << /Font << /F1 5 0 R >> >> /StructParents 0 /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n\
         6 0 obj\n<< /Type /StructTreeRoot /K [7 0 R 9 0 R] /ParentTree 11 0 R >>\nendobj\n\
         7 0 obj\n<< /Type /StructElem /S /Sect /P 6 0 R /Lang (en-US) /Pg 3 0 R /K 8 0 R >>\nendobj\n\
         8 0 obj\n<< /Type /StructElem /S /P /P 7 0 R /Pg 3 0 R /Alt (explicit structure Lang) \
          /Lang <FEFF00650073002D004D0058> \
          /K [<< /Type /MCR /Pg 3 0 R /MCID 0 >>] >>\nendobj\n\
         9 0 obj\n<< /Type /StructElem /S /Sect /P 6 0 R /Pg 3 0 R /K 10 0 R >>\nendobj\n\
         10 0 obj\n<< /Type /StructElem /S /P /P 9 0 R /Pg 3 0 R /Alt (catalog Lang fallback) \
          /K [<< /Type /MCR /Pg 3 0 R /MCID 1 >>] >>\nendobj\n\
         11 0 obj\n<< /Nums [0 [8 0 R 10 0 R]] >>\nendobj\n",
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
    bytes.into_bytes()
}

#[test]
fn hexadecimal_lang_strings_reach_accessibility_with_catalog_and_structure_precedence() {
    let bytes = fixture();
    let syntax = Document::open(bytes.clone()).expect("exact-xref fixture opens");
    assert!(!syntax.was_recovered(), "all xref offsets are exact");
    assert!(!syntax.cross_reference_chain_was_cut());
    let page = pdf_model::Pages::new(&syntax).get(0).expect("one page");
    let interpretation = pdf_model::interpret(&syntax, &page);
    assert!(
        interpretation.is_complete(),
        "complete fixture interpretation"
    );
    assert!(interpretation.glyphs > 0, "positive glyph/render control");

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
        panic!("open document exposes an accessibility tree");
    };
    let nodes: Vec<_> = pages.iter().flat_map(|page| page.nodes.iter()).collect();
    let inherited = nodes
        .iter()
        .find(|node| node.name == "catalog Lang fallback");
    let overridden = nodes
        .iter()
        .find(|node| node.name == "explicit structure Lang");
    assert_eq!(
        inherited.map(|node| node.language.as_deref()),
        Some(Some("fr-FR")),
        "catalog hexadecimal Lang is the fallback when no structure Lang is present"
    );
    assert_eq!(
        overridden.map(|node| node.language.as_deref()),
        Some(Some("es-MX")),
        "structure hexadecimal Lang overrides its literal en-US ancestor"
    );
}
