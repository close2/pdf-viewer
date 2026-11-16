//! Regression for §12.7.8.3.3 Tables 251-253.
//! One FDF page has two named template pages; both template contents should reach that one page.
#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "fixed exact-xref fixtures with literal independent pixel expectations"
)]

use std::fmt::Write as _;

use pdf_model::{Pages, content, forms_data::FormsData, view::ViewState};
use pdf_render::{Medium, Rasterizer, TargetSpec};
use pdf_syntax::{Document, FileBytes};
use render_cpu::CpuRasterizer;

fn file(header: &str, root: &str, objects: &[String]) -> Vec<u8> {
    let mut out = String::from(header);
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let number = index.saturating_add(1);
        let _ = writeln!(out, "{number} 0 obj\n{object}\nendobj");
    }
    let xref_at = out.len();
    let size = objects.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root {root} >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn target() -> Document {
    let red = "1 0 0 rg 0 0 40 100 re f\n";
    let blue = "0 0 1 rg 60 0 40 100 re f\n";
    let objects = vec![
        "<< /Type /Catalog /Pages 2 0 R /Names << /Templates << /Names [(blue) 6 0 R (red) 4 0 R] >> >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>".to_owned(),
        "<< /Type /Template /MediaBox [0 0 100 100] /Resources << >> /Contents 5 0 R >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{}endstream", red.len(), red),
        "<< /Type /Template /MediaBox [0 0 100 100] /Resources << >> /Contents 7 0 R >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{}endstream", blue.len(), blue),
    ];
    let document = Document::open(FileBytes::from(file("%PDF-1.7\n", "1 0 R", &objects)))
        .expect("complete target PDF with two named template pages");
    assert!(!document.was_recovered(), "target has exact xref offsets");
    document
}

fn fdf(names: &[&str]) -> Document {
    let templates = names
        .iter()
        .map(|name| format!("<< /TRef << /Name ({name}) >> >>"))
        .collect::<Vec<_>>()
        .join(" ");
    let objects = vec![
        "<< /Type /Catalog /FDF 2 0 R >>".to_owned(),
        format!("<< /Pages [<< /Templates [{templates}] >>] >>"),
    ];
    let document = Document::open(FileBytes::from(file("%FDF-1.3\n", "1 0 R", &objects)))
        .expect("complete FDF");
    assert!(!document.was_recovered(), "FDF has exact xref offsets");
    document
}

fn raster(target: &Document, names: &[&str]) -> ([u8; 4], [u8; 4]) {
    let data = FormsData::read(&fdf(names)).expect("FDF page dictionaries");
    let mut view = ViewState::of(target);
    let outcome = view.import(target, &data);
    assert_eq!(outcome.pages, 1, "one FDF page dictionary adds one page");
    assert!(
        outcome.refused.is_empty(),
        "both names resolve: {:?}",
        outcome.refused
    );
    assert_eq!(view.appended_pages().len(), 1);
    let source;
    let dictionary = match &view.appended_pages()[0] {
        pdf_model::view::AppendedPage::Named(id) => {
            source = target.get(*id);
            source.as_dict().expect("named template dictionary")
        }
        pdf_model::view::AppendedPage::Carried(page) => page,
    };
    let page = Pages::new(target).detached(dictionary);
    let interpreted = content::interpret_with(target, &page, &view);
    assert!(interpreted.is_complete(), "{:?}", interpreted.unsupported);
    let target_spec =
        TargetSpec::for_page(&interpreted.display_list, 1.0, 1 << 20).expect("100 by 100 target");
    let image = CpuRasterizer::new()
        .with_medium(Medium::NONE)
        .rasterize(&interpreted.display_list, target_spec)
        .expect("the imported page rasterizes");
    assert_eq!((image.width, image.height), (100, 100));
    let sample = |x: u32, y: u32| {
        let row = image.height - 1 - y;
        let at = usize::try_from((row * image.width + x) * 4).expect("small raster offset");
        [
            image.data[at],
            image.data[at + 1],
            image.data[at + 2],
            image.data[at + 3],
        ]
    };
    (sample(20, 50), sample(80, 50))
}

/// The source supplies one page dictionary with an array of templates that serve on that page.
/// Distinct interior pixels avoid imposing the unspecified transform/overlap order. The one-red
/// case is a positive control; the two-template case must preserve both producers' marks.
#[test]
fn all_templates_of_one_fdf_page_contribute_to_its_single_added_page() {
    let target = target();
    assert_eq!(raster(&target, &["red"]), ([255, 0, 0, 255], [0, 0, 0, 0]));
    assert_eq!(
        raster(&target, &["red", "blue"]),
        ([255, 0, 0, 255], [0, 0, 255, 255])
    );
}
