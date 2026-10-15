//! An ECMAScript action whose text this program does not run is said, with why, at every chain a
//! host hands a runner (ADR 1950).
//!
//! A host drops the action path's refusal of a chain's scripts where the reader's level lets them
//! run (ADR 1752), so a script the view state does not read — one past the length a script may be
//! here, or one whose `/JS` is neither of Table 221's two forms — was said by nobody. The fixture
//! puts one of each in the chains the open sequence walks, beside a script that runs, and a runner
//! of this test's own records what it was handed.

#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

use pdf_model::view::{ScriptEvent, ScriptResult, ScriptRunner, ViewState};
use pdf_syntax::Document;

/// The longest script, in bytes, the view state hands a runner: `view::scripts::MAX_SCRIPT_BYTES`.
const LONGEST: usize = 1 << 20;

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

/// A stream object of `length` bytes of script: a statement padded with spaces.
fn script_stream(length: usize) -> String {
    let statement = "ran();";
    let padding = " ".repeat(length.saturating_sub(statement.len()));
    format!("<< /Length {length} >>\nstream\n{statement}{padding}\nendstream")
}

/// One page whose Table 198 `/O` chain is a script of `page_script` bytes followed by `after();`,
/// whose widget's `/PO` is a script of `widget_script` bytes, and whose catalog's `/OpenAction`
/// states its `/JS` as a number.
fn document(page_script: usize, widget_script: usize) -> Document {
    assembled(&[
        "<< /Type /Catalog /Pages 2 0 R /OpenAction << /S /JavaScript /JS 12 >> \
         /AcroForm << /Fields [4 0 R] >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [4 0 R] \
         /AA << /O << /S /JavaScript /JS 5 0 R /Next << /S /JavaScript /JS (after\\(\\);) >> >> >> >>"
            .to_owned(),
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Field) /Rect [10 100 210 130] /P 3 0 R \
         /AA << /PO << /S /JavaScript /JS 6 0 R >> >> >>"
            .to_owned(),
        script_stream(page_script),
        script_stream(widget_script),
    ])
}

/// A runner that records the length of every script it is handed.
#[derive(Debug, Default)]
struct Recording {
    /// Each script's length in bytes, in the order handed.
    handed: Mutex<Vec<usize>>,
}

impl ScriptRunner for Recording {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        if let Ok(mut handed) = self.handed.lock() {
            handed.push(event.script.len());
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

/// The open sequence over `document`: what the runner was handed, and what the view state said.
fn opened(document: &Document) -> (Vec<usize>, Vec<String>) {
    let runner = Arc::new(Recording::default());
    let mut view = ViewState::of(document);
    view.run_scripts_with(Some(Arc::clone(&runner) as Arc<dyn ScriptRunner>));
    view.run_open_scripts(document, 0);
    let handed = runner.handed.lock().expect("the record").clone();
    (handed, view.script_reports().to_vec())
}

/// The page's script past the bound and the widget's are each said with their length and the
/// bound, the open action's with Table 221's two forms, and the script after the long one in the
/// same chain still runs; no sentence gives Tier 0's reason, which is not why any of them did not
/// run.
#[test]
fn a_script_past_the_bound_is_said_with_its_length_and_the_rest_of_its_chain_runs() {
    let long = LONGEST.saturating_add(1);
    let (handed, reports) = opened(&document(long, long));
    assert_eq!(handed, vec!["after();".len()], "{reports:#?}");
    let expected = [
        format!(
            "page 1's /O chain holds an ECMAScript action of {long} bytes, past the {LONGEST} a \
             script may be in this program, and it was not run"
        ),
        format!(
            "the /PO event of the annotation of object 4 holds an ECMAScript action of {long} \
             bytes, past the {LONGEST} a script may be in this program, and it was not run"
        ),
        "the document's open action holds an ECMAScript action whose /JS is neither a text \
         string nor a text stream this program can decode, and it was not run"
            .to_owned(),
    ];
    for sentence in &expected {
        assert!(reports.contains(sentence), "{sentence:?} in {reports:#?}");
    }
    assert!(
        reports.iter().all(|report| !report.contains("Tier 0")),
        "{reports:#?}"
    );
}

/// The control: a script of exactly the bound is handed over and nothing is said of it.
#[test]
fn a_script_at_the_bound_runs_and_is_not_said() {
    let (handed, reports) = opened(&document(LONGEST, LONGEST));
    assert_eq!(
        handed,
        vec![LONGEST, "after();".len(), LONGEST],
        "{reports:#?}"
    );
    assert!(
        reports
            .iter()
            .all(|report| !report.contains("bytes, past the")),
        "{reports:#?}"
    );
}
