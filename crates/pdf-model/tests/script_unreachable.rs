//! What of a page's `/Annots` no script reaches is said once, under the sentence the file breaks
//! to put it there: an annotation written into the array as a dictionary (Table 31), and widgets
//! in a document whose catalog names no interactive form, or whose form states no `/Fields`
//! (§12.7.3, Table 224).
//!
//! Each fixture is one page with one widget annotation, assembled so that one condition holds;
//! the open sequence is run twice with a runner of this test's own, and every sentence is counted.

#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

use pdf_model::view::{ScriptEvent, ScriptResult, ScriptRunner, ViewState};
use pdf_syntax::Document;

/// Table 31's sentence for page one's one direct annotation.
const DIRECT: &str = "page 1's /Annots holds 1 annotation(s) written into the array as \
                      dictionaries, where Table 31 says it \"shall contain indirect references to \
                      all annotations associated with the page\"; no script reaches them and none \
                      of their events is run";

/// §12.7.3's sentence for page one's one widget.
const NO_FORM: &str = "page 1 holds 1 widget annotation(s), and the catalog names no interactive \
                       form dictionary, which §12.7.3 says \"shall be referenced from the AcroForm \
                       entry in the document catalog dictionary\"; none of them is a field, so no \
                       script reaches one as a field";

/// Table 224's sentence for page one's one widget.
const NO_FIELDS: &str = "page 1 holds 1 widget annotation(s), and the interactive form states no \
                         /Fields, which Table 224 makes Required: \"An array of references to the \
                         document's root fields\"; none of them is a field, so no script reaches \
                         one as a field";

/// The widget, with a `/PO` script that records it ran.
const WIDGET: &str = "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Field) /Rect [10 100 210 130] \
                      /AA << /PO << /S /JavaScript /JS (opened\\(\\);) >> >> >>";

/// A document of `objects`, numbered from 1, the first the catalog.
fn assembled(objects: &[String]) -> Document {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = objects.len().saturating_add(1);
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

/// One page whose `/Annots` is `annots`, under a catalog stating `form` after `/Pages`; object 4
/// is [`WIDGET`].
fn document(form: &str, annots: &str) -> Document {
    assembled(&[
        format!("<< /Type /Catalog /Pages 2 0 R {form} >>"),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [{annots}] >>"),
        WIDGET.to_owned(),
    ])
}

/// A runner that records every script it is handed.
#[derive(Debug, Default)]
struct Recording {
    /// Each script, in the order handed.
    handed: Mutex<Vec<String>>,
}

impl ScriptRunner for Recording {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        if let Ok(mut handed) = self.handed.lock() {
            handed.push(event.script.to_owned());
        }
        ScriptResult {
            rc: true,
            value: None,
            change: None,
            edits: Vec::new(),
            report: Vec::new(),
        }
    }
}

/// The open sequence over `document`, run twice: what the runner was handed, and what the view
/// state said.
fn opened(document: &Document) -> (Vec<String>, Vec<String>) {
    let runner = Arc::new(Recording::default());
    let mut view = ViewState::of(document);
    view.run_scripts_with(Some(Arc::clone(&runner) as Arc<dyn ScriptRunner>));
    view.run_open_scripts(document, 0);
    view.run_open_scripts(document, 0);
    let handed = runner.handed.lock().expect("the record").clone();
    (handed, view.script_reports().to_vec())
}

/// How many times `sentence` was said.
fn said(reports: &[String], sentence: &str) -> usize {
    reports.iter().filter(|report| *report == sentence).count()
}

/// A widget written into `/Annots` as a dictionary in a document with no form: both sentences,
/// once each over two open sequences, and its `/PO` handed to nobody.
#[test]
fn a_direct_widget_with_no_form_is_said_under_both_sentences_once() {
    let (handed, reports) = opened(&document("", WIDGET));
    assert!(handed.is_empty(), "{handed:?}");
    assert_eq!(said(&reports, DIRECT), 1, "{reports:#?}");
    assert_eq!(said(&reports, NO_FORM), 1, "{reports:#?}");
}

/// An indirect widget under a form with no `/Fields` array: Table 224's sentence, and not Table
/// 31's; the widget's own `/PO` still runs, because an annotation's events are not a field's.
#[test]
fn a_form_with_no_fields_is_said_and_the_widget_s_event_still_runs() {
    let (handed, reports) = opened(&document("/AcroForm << /DA (/Helv 0 Tf) >>", "4 0 R"));
    assert_eq!(handed, vec!["opened();".to_owned(); 2], "{reports:#?}");
    assert_eq!(said(&reports, NO_FIELDS), 1, "{reports:#?}");
    assert_eq!(said(&reports, DIRECT), 0, "{reports:#?}");
}

/// The controls: a widget `/Fields` lists, and one `/Fields` omits, which the page recovers (ADR
/// 1653); neither is said.
#[test]
fn a_widget_the_form_reaches_or_recovers_is_not_said() {
    for form in [
        "/AcroForm << /Fields [4 0 R] >>",
        "/AcroForm << /Fields [] >>",
    ] {
        let (_, reports) = opened(&document(form, "4 0 R"));
        assert!(
            reports
                .iter()
                .all(|report| !report.contains("Table 31") && !report.contains("none of them")),
            "{form}: {reports:#?}"
        );
    }
}
