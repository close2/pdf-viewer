//! A sequence of scripts stopped by its budget is counted, and said even once the report is full
//! (ADR 3469).
//!
//! ISO 32000-2 §12.7.3, Table 224's `/CO`: "the order in which the document's fields are
//! recalculated". ADR 1603 holds a commit's walk of it to a time budget; the stop is the reason
//! the values after it are what they are, so the report's cap may not hide it, and a caller that
//! holds a figure to the bytes needs the count.

#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use std::fmt::Write as _;
use std::time::Duration;

use pdf_model::view::{Entered, ViewState};
use pdf_syntax::Document;

/// Calculated fields in the fixture: more than the report keeps (256 sentences), so that one
/// commit's walk fills it with each field's "not run" sentence.
const CALCULATED: usize = 300;

/// One page: the text field `In` (object 4), then `CALCULATED` text fields `C0`… each with a `/C`
/// script that is not one call of the library, all of them in `/CO`.
fn form() -> Document {
    let first = 5_usize;
    let calculated: Vec<usize> = (first..first + CALCULATED).collect();
    let references = |ids: &[usize]| {
        ids.iter().fold(String::new(), |mut out, id| {
            let _ = write!(out, "{id} 0 R ");
            out
        })
    };
    let mut fields = vec![4_usize];
    fields.extend(&calculated);
    let mut bodies = vec![
        format!(
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [{}] /CO [{}] >> >>",
            references(&fields),
            references(&calculated)
        ),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /Annots [{}] >>",
            references(&fields)
        ),
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (In) /Rect [10 10 100 30] /F 4 /P 3 0 R \
         /DA (/Helv 10 Tf 0 g) >>"
            .to_owned(),
    ];
    for (index, _) in calculated.iter().enumerate() {
        bodies.push(format!(
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (C{index}) /Rect [10 40 100 60] /F 4 \
             /P 3 0 R /DA (/Helv 10 Tf 0 g) /AA << /C << /S /JavaScript \
             /JS (event.value = 1 + 1;) >> >> >>"
        ));
    }
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = bodies.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    Document::open(out.into_bytes()).expect("the fixture opens")
}

/// Types into `In` and commits it, which walks `/CO` with `In` as the source.
fn commit(view: &mut ViewState, document: &Document, text: &str) {
    assert_eq!(
        view.set_field(document, "In", &Entered::Text(text.to_owned())),
        1,
        "the text field takes the value"
    );
    view.commit_field(document, "In");
}

#[test]
fn a_walk_the_budget_stops_is_counted_and_said_past_a_full_report() {
    let document = form();
    let mut view = ViewState::of(&document);
    commit(&mut view, &document, "one");
    assert_eq!(
        view.sequences_stopped(),
        0,
        "the default second is not reached"
    );
    let full = view.script_reports().len();
    assert!(
        full >= 256,
        "{CALCULATED} unrun calculations fill the report: {full} sentence(s)"
    );

    view.hold_sequences_to(Duration::ZERO);
    commit(&mut view, &document, "two");
    assert_eq!(
        view.sequences_stopped(),
        1,
        "the zero budget stops the walk"
    );
    let stop = view
        .script_reports()
        .iter()
        .find(|sentence| sentence.starts_with("the calculation order was stopped at entry"))
        .expect("the stop is said although the report was full");
    assert!(stop.contains(&format!("of {CALCULATED}")), "{stop}");
}
