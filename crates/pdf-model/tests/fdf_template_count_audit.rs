//! Regression: the FDF /Pages array counts FDF page dictionaries; each dictionary may have multiple
//! template sources for its one described new page.
#![expect(
    clippy::expect_used,
    reason = "exact-xref fixtures and named-page controls must fail loudly"
)]

use std::fmt::Write as _;

use pdf_model::{forms_data::FormsData, view::ViewState};
use pdf_syntax::{Document, FileBytes};

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
        "trailer\n<< /Size {size} /Root {root} /ID [<30313233343536373839616263646566> <66656463626139383736353433323130>] >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn target() -> Document {
    let objects = vec![
        "<< /Type /Catalog /Pages 2 0 R /Names << /Templates << /Names [(blue) 5 0 R (red) 4 0 R] >> >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>".to_owned(),
        "<< /Type /Template /MediaBox [0 0 100 100] /Resources << >> >>".to_owned(),
        "<< /Type /Template /MediaBox [0 0 100 100] /Resources << >> >>".to_owned(),
    ];
    let document = Document::open(FileBytes::from(file("%PDF-2.0\n", "1 0 R", &objects)))
        .expect("complete PDF with two named template pages");
    assert!(!document.was_recovered(), "exact target xref");
    document
}

fn template(name: &str) -> String {
    format!("<< /TRef << /Name ({name}) >> >>")
}

fn import(page_dictionaries: &[String], templates_per_page: &[usize]) -> usize {
    let templates = page_dictionaries.join(" ");
    let fdf = file(
        "%FDF-1.3\n",
        "1 0 R",
        &[format!(
            "<< /Version /1.3 /FDF << /Pages [{templates}] >> >>"
        )],
    );
    let fdf = Document::open(FileBytes::from(fdf)).expect("complete FDF");
    assert!(!fdf.was_recovered(), "exact FDF xref");
    let data = FormsData::read(&fdf).expect("FDF page dictionaries");
    assert_eq!(
        data.pages.len(),
        page_dictionaries.len(),
        "all FDF page dictionaries parsed"
    );
    assert_eq!(templates_per_page.len(), data.pages.len());
    for (page, expected) in data.pages.iter().zip(templates_per_page) {
        assert_eq!(
            page.templates.len(),
            *expected,
            "each page's Templates array parsed"
        );
    }
    let target = target();
    let mut view = ViewState::of(&target);
    let outcome = view.import(&target, &data);
    assert!(
        outcome.refused.is_empty(),
        "both names resolve: {:?}",
        outcome.refused
    );
    assert_eq!(view.appended_pages().len(), outcome.pages);
    outcome.pages
}

/// Table 251's /Pages array contains FDF page dictionaries, each describing one new page.
/// Table 251's Templates array names the one or more template pages serving on that FDF page.
/// Thus two templates in one FDF page dictionary still add one page; two FDF page dictionaries
/// each using a template are the positive cardinality control and add two.
#[test]
fn template_array_members_do_not_multiply_the_fdf_page_count() {
    // Positive control: two FDF page dictionaries add two pages.
    let two_fdf_pages_one_template_each = [
        format!("<< /Templates [{}] >>", template("red")),
        format!("<< /Templates [{}] >>", template("blue")),
    ];
    assert_eq!(import(&two_fdf_pages_one_template_each, &[1, 1]), 2);

    // Two named template sources on the same FDF page describe that single page.
    let one_fdf_page_two_templates = format!(
        "<< /Templates [{} {}] >>",
        template("red"),
        template("blue")
    );
    assert_eq!(import(&[one_fdf_page_two_templates], &[2]), 1);
}
