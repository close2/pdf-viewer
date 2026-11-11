//! ADR 2440's explicit limitation: requested guidelines are reported and omitted when the
//! page's compositing context would change their `DeviceRGB` colours. This is a product policy
//! test, not a claim that ISO 32000-2 requires that limitation.
#![expect(
    clippy::expect_used,
    reason = "fixtures and the positive control must reach the intended interpretation path"
)]

use std::fmt::Write as _;

use pdf_model::content::{Interpretation, Unsupported};
use pdf_model::view::ViewState;
use pdf_render::Command;
use pdf_syntax::Document;

fn document(group: &str) -> Document {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> \
             /TrimBox [10 10 90 90] /BoxColorInfo << /TrimBox << /C [1 0 0] /W 2 >> >> \
             {group} >>"
        ),
    ];
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        writeln!(bytes, "{} 0 obj\n{object}\nendobj", index.saturating_add(1))
            .expect("String write");
    }
    let xref = bytes.len();
    bytes.push_str("xref\n0 4\n0000000000 65535 f \n");
    for offset in offsets {
        writeln!(bytes, "{offset:010} 00000 n ").expect("String write");
    }
    write!(
        bytes,
        "trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    )
    .expect("String write");
    let document = Document::open(bytes.into_bytes()).expect("valid source");
    assert!(!document.was_recovered());
    document
}

fn interpreted(group: &str, shown: bool, separations: bool) -> Interpretation {
    let document = document(group);
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let mut view = ViewState::of(&document);
    view.set_guidelines(shown);
    view.set_separation_simulation(separations);
    pdf_model::content::interpret_with(&document, &page, &view)
}

fn guideline_strokes(result: &Interpretation) -> usize {
    result
        .display_list
        .commands()
        .iter()
        .filter(|command| matches!(command, Command::Stroke { .. }))
        .count()
}

fn guideline_reports(result: &Interpretation) -> usize {
    result
        .unsupported
        .iter()
        .filter(|note| matches!(note, Unsupported::Guidelines { .. }))
        .count()
}

#[test]
fn unsupported_guideline_contexts_are_reported_only_when_requested() {
    let supported = interpreted("", true, false);
    assert_eq!(
        guideline_strokes(&supported),
        1,
        "ordinary view draws the requested trim boundary"
    );
    assert_eq!(guideline_reports(&supported), 0);

    for (group, separations) in [
        ("/Group << /S /Transparency /CS /DeviceGray >>", false),
        ("", true),
    ] {
        let shown = interpreted(group, true, separations);
        assert_eq!(
            guideline_strokes(&shown),
            0,
            "limited context does not silently recolour guidelines"
        );
        assert_eq!(
            guideline_reports(&shown),
            1,
            "requested feature omission is reported"
        );

        let hidden = interpreted(group, false, separations);
        assert_eq!(guideline_strokes(&hidden), 0);
        assert_eq!(
            guideline_reports(&hidden),
            0,
            "an unrequested guideline is not a loss to report"
        );
    }
}
