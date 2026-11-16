//! EC3 #200: a mismatched Scope cell is skipped, including its /Headers route, by the adopted
//! implicit-search algorithm. The source says this algorithm "may be used" and the correction
//! says conflicting Scope "should not" terminate/contribute; this is not a universal mandate to
//! use implicit search in every reader.
#![expect(
    clippy::panic,
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    reason = "the exact-xref fixture has twenty objects and must fail loudly if it does not reach the public tree"
)]
use std::fmt::Write as _;

use pdf_render::Rasterizer;
use render_cpu::CpuRasterizer;
use viewer_core::{Answer, Command, DocumentId, Event, Query, Rendered, Viewer};

fn table_pdf(conflict_scope: &str, conflicting_headers: bool) -> Vec<u8> {
    let content = "BT /F1 12 Tf 10 140 Td\n\
        /TH <</MCID 0>> BDC 1 0 0 1 10 150 Tm (Far) Tj EMC\n\
        /TD <</MCID 1>> BDC 1 0 0 1 110 150 Tm (filler A) Tj EMC\n\
        /TH <</MCID 2>> BDC 1 0 0 1 10 110 Tm (Middle) Tj EMC\n\
        /TH <</MCID 3>> BDC 1 0 0 1 110 110 Tm (Named) Tj EMC\n\
        /TH <</MCID 4>> BDC 1 0 0 1 10 70 Tm (Conflict) Tj EMC\n\
        /TD <</MCID 5>> BDC 1 0 0 1 110 70 Tm (filler B) Tj EMC\n\
        /TD <</MCID 6>> BDC 1 0 0 1 10 30 Tm (Target) Tj EMC ET\n";
    let headers = if conflicting_headers {
        " /Headers [(named)]"
    } else {
        ""
    };
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /MarkInfo << /Marked true >> \
            /StructTreeRoot 6 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 180] /StructParents 0 \
            /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n{content}endstream\nendobj\n\
         5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n\
         6 0 obj\n<< /Type /StructTreeRoot /K [7 0 R] /ParentTree 20 0 R /IDTree 19 0 R /ParentTreeNextKey 1 >>\nendobj\n\
         7 0 obj\n<< /Type /StructElem /S /Table /P 6 0 R /Pg 3 0 R \
            /K [8 0 R 11 0 R 14 0 R 17 0 R] >>\nendobj\n\
         8 0 obj\n<< /Type /StructElem /S /TR /P 7 0 R /Pg 3 0 R /K [9 0 R 10 0 R] >>\nendobj\n\
         9 0 obj\n<< /Type /StructElem /S /TH /P 8 0 R /Pg 3 0 R /ID (far) \
            /A << /O /Table /Scope /Column >> /K [<< /Type /MCR /Pg 3 0 R /MCID 0 >>] >>\nendobj\n\
         10 0 obj\n<< /Type /StructElem /S /TD /P 8 0 R /Pg 3 0 R /K [<< /Type /MCR /Pg 3 0 R /MCID 1 >>] >>\nendobj\n\
         11 0 obj\n<< /Type /StructElem /S /TR /P 7 0 R /Pg 3 0 R /K [12 0 R 13 0 R] >>\nendobj\n\
         12 0 obj\n<< /Type /StructElem /S /TH /P 11 0 R /Pg 3 0 R /A << /O /Table /Scope /Column >> /K [<< /Type /MCR /Pg 3 0 R /MCID 2 >>] >>\nendobj\n\
         13 0 obj\n<< /Type /StructElem /S /TH /P 11 0 R /Pg 3 0 R /ID (named) \
            /A << /O /Table /Scope /Column >> /K [<< /Type /MCR /Pg 3 0 R /MCID 3 >>] >>\nendobj\n\
         14 0 obj\n<< /Type /StructElem /S /TR /P 7 0 R /Pg 3 0 R /K [15 0 R 16 0 R] >>\nendobj\n\
         15 0 obj\n<< /Type /StructElem /S /TH /P 14 0 R /Pg 3 0 R /ID (conflict) \
            /A << /O /Table /Scope /{conflict_scope}{headers} >> \
            /K [<< /Type /MCR /Pg 3 0 R /MCID 4 >>] >>\nendobj\n\
         16 0 obj\n<< /Type /StructElem /S /TD /P 14 0 R /Pg 3 0 R /K [<< /Type /MCR /Pg 3 0 R /MCID 5 >>] >>\nendobj\n\
         17 0 obj\n<< /Type /StructElem /S /TR /P 7 0 R /Pg 3 0 R /K [18 0 R] >>\nendobj\n\
         18 0 obj\n<< /Type /StructElem /S /TD /P 17 0 R /Pg 3 0 R /K [<< /Type /MCR /Pg 3 0 R /MCID 6 >>] >>\nendobj\n\
         19 0 obj\n<< /Names [(conflict) 15 0 R (far) 9 0 R (named) 13 0 R] >>\nendobj\n\
         20 0 obj\n<< /Nums [0 [9 0 R 10 0 R 12 0 R 13 0 R 15 0 R 16 0 R 18 0 R]] >>\nendobj\n",
        content.len(),
    );

    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(bytes.len());
        bytes.push_str(object);
    }
    let xref = bytes.len();
    let size = offsets.len() + 1;
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

fn target_headers(bytes: Vec<u8>) -> Vec<String> {
    let syntax = pdf_syntax::Document::open(bytes.clone()).expect("the generated PDF opens");
    assert!(!syntax.was_recovered(), "all xref offsets are exact");
    let mut viewer = Viewer::new(500, 300, 1.0);
    let events: Vec<_> = viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    let render = events
        .iter()
        .find_map(|event| match event {
            Event::NeedsRender(request) => Some(request.clone()),
            _ => None,
        })
        .expect("the fixture page is rendered");
    let raster = CpuRasterizer::new()
        .rasterize(&render.list, render.target)
        .expect("CPU renderer accepts the positive-control page");
    viewer
        .handle(Command::RenderReady {
            token: render.token,
            rendered: Rendered::Raster(raster),
        })
        .for_each(drop);

    let Answer::Accessibility(pages) = viewer.query(Query::AccessibilityTree) else {
        panic!("the public accessibility query answers");
    };
    assert_eq!(
        pages.len(),
        1,
        "header indices are relative to this one page"
    );
    let nodes: Vec<_> = pages.into_iter().flat_map(|page| page.nodes).collect();
    for name in ["Far", "Middle", "Named", "Conflict", "Target"] {
        assert!(
            nodes.iter().any(|node| node.name.trim() == name),
            "{name} is exposed before comparing header associations: {:?}",
            nodes.iter().map(|node| &node.name).collect::<Vec<_>>()
        );
    }
    // Layout inserts leading whitespace into some cell names. This test compares header
    // associations, not the text-spacing policy; retain every non-whitespace character.
    let target = nodes
        .iter()
        .find(|node| node.name.trim() == "Target")
        .expect("the final TD is exposed by the page's structure tree");
    target
        .headers
        .iter()
        .map(|index| {
            nodes
                .get(*index)
                .unwrap_or_else(|| panic!("header index {index} resolves within the public page"))
                .name
                .trim()
                .to_owned()
        })
        .collect()
}

#[test]
fn conflicting_scope_without_headers_is_skipped_while_matching_headers_are_collected() {
    // No intervening data cell: this does not adjudicate how a conflicting TH affects the
    // separate data-after-header stopping condition.
    assert_eq!(target_headers(table_pdf("Row", false)), ["Middle", "Far"]);
}

#[test]
fn conflicting_scope_with_headers_does_not_terminate_the_adopted_column_search() {
    let headers = target_headers(table_pdf("Row", true));
    // EC3 #200 explicitly says the conflicting cell should not terminate the search,
    // irrespective of Headers. The two eligible headers beyond it must still be reached.
    assert!(
        headers.iter().any(|name| name == "Middle") && headers.iter().any(|name| name == "Far"),
        "the column search continues beyond the conflicting row header: {headers:?}"
    );
    assert!(!headers.iter().any(|name| name == "Conflict"));
    // Whether the conflicting cell's named Headers may ALSO contribute is left open.
    // This witness neither requires nor forbids Named; it isolates premature termination.
}

#[test]
fn matching_scope_without_headers_collects_the_eligible_column_headers() {
    assert_eq!(
        target_headers(table_pdf("Column", false)),
        ["Conflict", "Middle", "Far"]
    );
}

#[test]
fn matching_scope_with_headers_appends_its_named_header_and_terminates() {
    assert_eq!(
        target_headers(table_pdf("Column", true)),
        ["Conflict", "Named"]
    );
}
