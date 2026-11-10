//! Integration tests for ISO 32000-2 §7.8.3 resource scoping.
//!
//! These fixtures are complete PDFs with explicit xref entries. They prove resource dictionaries
//! are consumed at the content stream that owns them, rather than only checking parsed page data.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "tests use complete synthetic PDFs and assert the known fill records"
)]

use std::fmt::Write as _;

use pdf_render::{Command, Paint};
use pdf_syntax::Document;

fn document(body: &str) -> Document {
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(bytes.len());
        bytes.push_str(object);
    }
    let xref_at = bytes.len();
    let size = offsets.len().saturating_add(1);
    let _ = writeln!(bytes, "xref\n0 {size}");
    bytes.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    let document = Document::open(bytes.into_bytes()).expect("complete fixture opens");
    assert!(
        !document.was_recovered(),
        "fixture has a complete xref table"
    );
    document
}

fn interpretation(bytes: &str) -> pdf_model::Interpretation {
    let document = document(bytes);
    let pages = pdf_model::Pages::new(&document);
    assert_eq!(pages.len(), 1);
    let page = pages.get(0).expect("one page exists in the tree");
    pdf_model::interpret(&document, &page)
}

fn solid_fills(bytes: &str) -> Vec<[u32; 4]> {
    let interpretation = interpretation(bytes);
    assert!(
        interpretation.unsupported.is_empty(),
        "valid fixtures should interpret completely: {:?}",
        interpretation.unsupported
    );
    interpretation
        .display_list
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Fill {
                paint: Paint::Solid(colour),
                ..
            } => Some([
                colour.r.to_bits(),
                colour.g.to_bits(),
                colour.b.to_bits(),
                colour.a.to_bits(),
            ]),
            _ => None,
        })
        .collect()
}

fn form(id: usize, resources: &str, content: &str) -> String {
    format!(
        "{id} 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /Resources << {resources} >> /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
        content.len().saturating_add(1)
    )
}

/// The page and its nested form both define `/Nested`, but to different forms.
///
/// The page-level invocation is a positive control: it must paint red. The nested invocation
/// must paint green from the outer form's `/Resources`. A lookup accidentally shared by resource
/// name or made against the page for every stream paints red twice. Invoking `/Nested` again after
/// the form also verifies that returning from the nested stream restores the page's dictionary.
#[test]
fn each_content_stream_uses_its_own_same_named_xobject_resource() {
    let outer_content = "/Nested Do";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Resources << /XObject << /Nested 5 0 R /Outer 6 0 R >> >> \
         /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n/Nested Do /Outer Do /Nested Do\nendstream\nendobj\n\
         {}{}{}",
        "/Nested Do /Outer Do /Nested Do".len().saturating_add(1),
        form(5, "", "1 0 0 rg 0 0 100 100 re f"),
        form(6, "/XObject << /Nested 7 0 R >>", outer_content),
        form(7, "", "0 1 0 rg 0 0 100 100 re f"),
    );

    let fills = solid_fills(&body);
    assert_eq!(
        fills.len(),
        3,
        "the sequence paints the direct control, nested target and restored control"
    );
    assert_eq!(
        fills[0],
        [1.0_f32, 0.0, 0.0, 1.0].map(f32::to_bits),
        "the page's /Nested resolves to its red form"
    );
    assert_eq!(
        fills[1],
        [0.0_f32, 1.0, 0.0, 1.0].map(f32::to_bits),
        "the outer form's /Nested resolves to its green form"
    );
    assert_eq!(
        fills[2],
        [1.0_f32, 0.0, 0.0, 1.0].map(f32::to_bits),
        "returning from the form restores the page's red /Nested"
    );
}

/// A page with no local `/Resources` uses the nearest ancestor's dictionary in actual execution.
#[test]
fn inherited_page_resources_resolve_the_page_content_stream() {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 \
         /Resources << /XObject << /Mark 5 0 R >> >> >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length {} >>\nstream\n/Mark Do\nendstream\nendobj\n\
         {}",
        "/Mark Do".len().saturating_add(1),
        form(5, "", "0 0 1 rg 0 0 100 100 re f"),
    );
    let fills = solid_fills(&body);
    assert_eq!(
        fills,
        vec![[0.0_f32, 0.0, 1.0, 1.0].map(f32::to_bits)],
        "the inherited resource is consumed to paint one blue fill"
    );
}

/// Missing-resource reporting must account for the state an operator leaves behind.
///
/// §8.11.3.1 requires graphics-state operations in hidden optional content to be applied, and
/// parameters that persist past EMC to be the same whether the section is visible or hidden.
/// The existing hidden `/Do` case is safe to suppress because it cannot leave graphics state.
/// A missing `/GS0 gs` is different: it leaves the current state in force and the visible fill
/// after EMC depends on that unresolved state. This test checks the project report policy, not a malformed-input ISO requirement, and the
/// positive control where a defined `ExtGState` sets alpha to 0.25.
#[test]
fn hidden_gs_lookup_that_changes_visible_state_is_reported() {
    let stream = "/OC /oc BDC /GS0 gs 0 0 10 10 re f EMC 0 0 100 100 re f";
    let common = |resources: &str| {
        format!(
            concat!(
                "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [5 0 R] /D << /OFF [5 0 R] >> >> >>\nendobj\n",
                "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n",
                "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /Properties << /oc 5 0 R >> {resources} >> /Contents 4 0 R >>\nendobj\n",
                "4 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
                "5 0 obj\n<< /Type /OCG /Name (Layer) >>\nendobj\n"
            ),
            stream.len().saturating_add(1),
            stream,
            resources = resources,
        )
    };

    let defined_body = format!(
        "{}6 0 obj\n<< /ca 0.25 >>\nendobj\n",
        common("/ExtGState << /GS0 6 0 R >>")
    );
    let defined = interpretation(&defined_body);
    assert!(defined.unsupported.is_empty());
    assert_eq!(
        defined.display_list.commands().len(),
        1,
        "the inner fill is hidden and the post-EMC fill is visible"
    );
    match defined.display_list.commands().first() {
        Some(Command::Fill {
            paint: Paint::Solid(colour),
            ..
        }) => {
            assert_eq!(colour.a.to_bits(), 0.25_f32.to_bits());
        }
        other => panic!("expected the visible fill to inherit /ca 0.25, got {other:?}"),
    }
    let missing = interpretation(&common(""));
    assert_eq!(
        missing.display_list.commands().len(),
        1,
        "the post-EMC fill remains visible"
    );
    assert_eq!(
        missing.unsupported,
        vec![pdf_model::Unsupported::MissingResource {
            category: "ExtGState",
            detail: "/GS0 is not in /ExtGState".to_owned(),
        }],
        "this unresolved state lookup can affect visible marks after EMC"
    );
}

/// §7.3.5 and §7.8.3 require byte-distinct names even when neither name is UTF-8.
/// Both names resolve through an indirect category table, with a repeated lookup after the
/// other name. This also exercises ADR 0287's category-table memo without relying on reports
/// alone to prove that the resource painted.
#[test]
fn byte_distinct_names_paint_distinct_forms_through_an_indirect_category() {
    let content = "/N#f4 Do /N#f5 Do /N#f4 Do";
    let body = format!(
        concat!(
            "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
            "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n",
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << /XObject 6 0 R >> /Contents 4 0 R >>\nendobj\n",
            "4 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
            "{}6 0 obj\n<< /N#f4 5 0 R /N#f5 7 0 R >>\nendobj\n{}"
        ),
        content.len().saturating_add(1),
        content,
        form(5, "", "1 0 0 rg 0 0 100 100 re f"),
        form(7, "", "0 1 0 rg 0 0 100 100 re f"),
    );
    assert_eq!(
        solid_fills(&body),
        vec![
            [1.0_f32, 0.0, 0.0, 1.0].map(f32::to_bits),
            [0.0_f32, 1.0, 0.0, 1.0].map(f32::to_bits),
            [1.0_f32, 0.0, 0.0, 1.0].map(f32::to_bits),
        ]
    );
}
