//! Reference target page-label search must import the page or report its limit truthfully.
//!
//! Table 95 permits a target page label. `/Page (near)` is page index 4095 (page 4096), just
//! inside `MAX_LABELS_SEARCHED`; `/Page (late)` is page index 4096 (page 4097), just outside.
//! Independent controls verify the source file really has the final page and that numeric index
//! 4096 still reaches its blue content. At the boundary, accept either importing the present page
//! or an explicit resource-limit report, but reject fallback reported as “the file does not have
//! that page.”

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "bounded exact-xref fixture with a fixed 4097-page tree"
)]

use std::fmt::Write as _;

use pdf_model::reference::Supply;
use pdf_render::{Rasterizer, TargetSpec};
use pdf_syntax::Document;
use render_cpu::CpuRasterizer;

const TARGET_ID: (&str, &str) = (
    "00112233445566778899aabbccddeeff",
    "00112233445566778899aabbccddeeff",
);
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const PROXY_RED: [u8; 4] = [255, 0, 0, 255];

fn target() -> Vec<u8> {
    const PAGE_COUNT: usize = 4097;
    const FIRST_PAGE_OBJECT: usize = 4;
    let last_page_object = FIRST_PAGE_OBJECT + PAGE_COUNT - 1;
    let green_content_object = last_page_object + 1;
    let blue_content_object = last_page_object + 2;

    let kids = (FIRST_PAGE_OBJECT..=last_page_object)
        .map(|object| format!("{object} 0 R"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /PageLabels 3 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [{kids}] /Count {PAGE_COUNT} >>\nendobj\n\
         3 0 obj\n<< /Nums [0 << /S /D >> 4095 << /P (near) >> 4096 << /P (late) >>] >>\nendobj\n"
    );
    for index in 0..PAGE_COUNT {
        let object = FIRST_PAGE_OBJECT + index;
        let content = match index {
            4095 => format!(" /Contents {green_content_object} 0 R"),
            4096 => format!(" /Contents {blue_content_object} 0 R"),
            _ => String::new(),
        };
        let _ = writeln!(
            body,
            "{object} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Resources << >>{content} >>\nendobj"
        );
    }
    let green = "0 1 0 rg 0 0 10 10 re f";
    let blue = "0 0 1 rg 0 0 10 10 re f";
    let _ = write!(
        body,
        "{green_content_object} 0 obj\n<< /Length {} >>\nstream\n{green}\nendstream\nendobj\n\
         {blue_content_object} 0 obj\n<< /Length {} >>\nstream\n{blue}\nendstream\nendobj\n",
        green.len() + 1,
        blue.len() + 1
    );
    exact_xref_pdf(&body, Some(TARGET_ID))
}

fn containing(page: &str) -> Vec<u8> {
    let proxy = "1 0 0 rg 0 0 10 10 re f";
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] \
         /Resources << /XObject << /Fm 5 0 R >> >> /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length 7 >>\nstream\n/Fm Do\nendstream\nendobj\n\
         5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] \
         /Ref << /F << /Type /Filespec /F (target.pdf) >> /Page {page} \
         /ID [<{}> <{}>] >> /Length {} >>\nstream\n{proxy}\nendstream\nendobj\n",
        TARGET_ID.0,
        TARGET_ID.1,
        proxy.len() + 1
    );
    exact_xref_pdf(&body, None)
}

fn exact_xref_pdf(body: &str, id: Option<(&str, &str)>) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref = out.len();
    let size = offsets.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}");
    out.push_str("0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let id = id.map_or_else(String::new, |(first, last)| {
        format!(" /ID [<{first}> <{last}>]")
    });
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R{id} >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

fn supply() -> Supply {
    let bytes = target();
    let parsed = Document::open(bytes.clone()).expect("target parses");
    assert!(
        !parsed.was_recovered(),
        "target fixture has exact xref offsets"
    );
    assert!(!parsed.cross_reference_chain_was_cut());
    let (supply, refused) = Supply::read([("target.pdf", bytes)], "test's target file");
    assert!(
        refused.is_empty(),
        "target has an ID and is valid: {refused:?}"
    );
    supply
}

fn interpret(page: &str, supply: &Supply) -> pdf_model::Interpretation {
    let bytes = containing(page);
    let parsed = Document::open(bytes.clone()).expect("container parses");
    assert!(
        !parsed.was_recovered(),
        "container fixture has exact xref offsets"
    );
    assert!(!parsed.cross_reference_chain_was_cut());
    let page = pdf_model::Pages::new(&parsed)
        .get(0)
        .expect("container page");
    pdf_model::content::interpret_importing(
        &parsed,
        &page,
        &pdf_model::view::ViewState::of(&parsed),
        &pdf_model::FontCache::new(),
        supply,
    )
}

fn center(result: &pdf_model::Interpretation) -> [u8; 4] {
    let target = TargetSpec::for_page(&result.display_list, 1.0, 1 << 20).expect("10x10 page");
    let raster = CpuRasterizer::new()
        .rasterize(&result.display_list, target)
        .expect("supported fixture rasterizes");
    let offset = ((5 * raster.width + 5) * 4) as usize;
    [
        raster.data[offset],
        raster.data[offset + 1],
        raster.data[offset + 2],
        raster.data[offset + 3],
    ]
}

fn report(result: &pdf_model::Interpretation) -> String {
    result
        .unsupported
        .iter()
        .map(|item| format!("{item:?}"))
        .collect::<Vec<_>>()
        .join("; ")
}

#[test]
fn a_present_page_label_beyond_the_search_bound_is_not_reported_as_a_missing_page() {
    let supply = supply();

    let near = interpret("(near)", &supply);
    assert!(
        near.is_complete(),
        "near label fully imported: {}",
        report(&near)
    );
    assert_eq!(center(&near), GREEN);
    let numeric = interpret("4096", &supply);
    assert!(
        numeric.is_complete(),
        "last page fully imported by index: {}",
        report(&numeric)
    );
    assert_eq!(center(&numeric), BLUE);

    let late = interpret("(late)", &supply);
    if center(&late) == BLUE {
        assert!(
            !report(&late).contains("file does not have"),
            "a successful import must not carry a false missing-page report: {}",
            report(&late)
        );
    } else {
        assert_eq!(
            center(&late),
            PROXY_RED,
            "fallback is only valid with an explicit limit"
        );
        let report = report(&late).to_ascii_lowercase();
        assert!(
            report.contains("limit") || report.contains("4096"),
            "{report}"
        );
        assert!(
            !report.contains("file does not have"),
            "present page was misreported: {report}"
        );
    }
}
