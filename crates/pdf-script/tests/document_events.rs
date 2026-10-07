//! Table 200's five events of the document as a whole, driven through
//! `pdf_model::view::ViewState::run_document_scripts`: each runs the catalog's `/AA` script for its
//! moment in the document's realm, a script's `event.rc` false is reported and never obeyed, a
//! will-save script's edit is in the file the save writes, and with no runner supplied nothing runs
//! and that is said (ADR 1614).
//!
//! Every expected value is either one the fixture's own script writes or one the standard states:
//! Table 200's value column makes each entry "[a]n ECMAScript action", and the reference's "Event
//! type/name combinations" page pairs each with the type `Doc` and the names asserted here.

#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

use pdf_model::view::{
    DocumentTrigger, ScriptEvent, ScriptResult, ScriptRunner, ScriptSite, ViewState,
};
use pdf_syntax::Document;

/// A one-page document with one text field `Note`, whose catalog's `/AA` holds `actions`; `extra`
/// objects are numbered from 5.
fn document(actions: &str, extra: &[&str]) -> Document {
    let mut bodies = vec![
        format!(
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> /AA << {actions} >> >>"
        ),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [4 0 R] >>".to_owned(),
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Note) /V (draft) /Rect [10 10 210 40] /F 4 \
         /P 3 0 R /DA (/Helv 10 Tf 0 g) >>"
            .to_owned(),
    ];
    bodies.extend(extra.iter().map(|body| (*body).to_owned()));
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

/// One script action stating `script`, for a catalog's `/AA` entry.
fn script(key: &str, script: &str) -> String {
    format!("/{key} << /S /JavaScript /JS ({script}) >>")
}

/// A runner that records each event's site and script and answers `rc` false.
#[derive(Debug, Default)]
struct Recording {
    /// Each event's site and script.
    seen: Mutex<Vec<(ScriptSite, String)>>,
}

impl ScriptRunner for Recording {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        if let Ok(mut seen) = self.seen.lock() {
            seen.push((event.site, event.script.to_owned()));
        }
        ScriptResult {
            rc: false,
            value: None,
            change: None,
            edits: Vec::new(),
            report: Vec::new(),
        }
    }
}

#[test]
fn each_of_the_five_runs_its_own_entry_at_its_own_site() {
    let actions: String = DocumentTrigger::ALL
        .iter()
        .map(|trigger| script(trigger.key(), &format!("var {} = 1;", trigger.key())))
        .collect::<Vec<_>>()
        .join(" ");
    let document = document(&actions, &[]);
    let runner = Arc::new(Recording::default());
    let mut view = ViewState::of(&document);
    view.run_scripts_with(Some(runner.clone()));
    for trigger in DocumentTrigger::ALL {
        assert_eq!(
            view.run_document_scripts(&document, trigger, 0),
            1,
            "{trigger:?}"
        );
    }
    let seen = runner.seen.lock().expect("the record").clone();
    let expected: Vec<(ScriptSite, String)> = DocumentTrigger::ALL
        .iter()
        .map(|trigger| {
            (
                ScriptSite::Document(*trigger),
                format!("var {} = 1;", trigger.key()),
            )
        })
        .collect();
    assert_eq!(seen, expected);
}

#[test]
fn a_false_rc_before_a_close_or_a_save_is_reported_and_the_operation_goes_ahead() {
    let document = document(
        &format!(
            "{} {}",
            script("WC", "event.rc = false;"),
            script("WS", "event.rc = false;")
        ),
        &[],
    );
    let mut view = ViewState::of(&document);
    view.run_scripts_with(Some(Arc::new(Recording::default())));
    assert_eq!(
        view.run_document_scripts(&document, DocumentTrigger::WillClose, 0),
        1
    );
    assert_eq!(
        view.run_document_scripts(&document, DocumentTrigger::WillSave, 0),
        1
    );
    for operation in ["close", "save"] {
        assert!(
            view.script_reports()
                .iter()
                .any(|sentence| sentence.contains(&format!(
                    "set event.rc false, and the {operation} goes ahead"
                ))),
            "{:?}",
            view.script_reports()
        );
    }
    assert!(
        view.save(&document).is_ok(),
        "nothing a will-save script answered stops the save"
    );
}

#[test]
fn a_chain_runs_its_scripts_in_order_and_names_an_action_table_200_does_not_admit() {
    let document = document(
        "/DP << /S /JavaScript /JS (var one = 1;) /Next [5 0 R 6 0 R] >>",
        &[
            "<< /S /URI /URI (https://example.org/printed) >>",
            "<< /S /JavaScript /JS (var two = 2;) >>",
        ],
    );
    let runner = Arc::new(Recording::default());
    let mut view = ViewState::of(&document);
    view.run_scripts_with(Some(runner.clone()));
    assert_eq!(
        view.run_document_scripts(&document, DocumentTrigger::DidPrint, 2),
        2
    );
    let scripts: Vec<String> = runner
        .seen
        .lock()
        .expect("the record")
        .iter()
        .map(|(_, script)| script.clone())
        .collect();
    assert_eq!(scripts, ["var one = 1;", "var two = 2;"]);
    assert!(
        view.script_reports().iter().any(
            |sentence| sentence.contains("/DP chain holds 1 action(s) that are not ECMAScript")
        ),
        "{:?}",
        view.script_reports()
    );
}

#[test]
fn with_no_runner_a_trigger_s_script_is_reported_as_not_run() {
    let document = document(&script("WS", "var saved = 1;"), &[]);
    let mut view = ViewState::of(&document);
    assert_eq!(
        view.run_document_scripts(&document, DocumentTrigger::WillSave, 0),
        0
    );
    assert!(
        view.script_reports().iter().any(|sentence| sentence
            .contains("the document's /WS script was not run, before or after the save")),
        "{:?}",
        view.script_reports()
    );
    assert_eq!(
        view.run_document_scripts(&document, DocumentTrigger::DidSave, 0),
        0,
        "an entry the catalog does not state runs nothing"
    );
}

#[cfg(feature = "engine")]
mod engine {
    use super::{DocumentTrigger, ViewState, document, script};
    use pdf_script::{Budget, Engine};

    #[test]
    fn a_will_save_script_s_edit_is_in_the_file_the_save_writes() {
        let document = document(
            &script(
                "WS",
                "this.getField\\('Note'\\).value = 'final, ' + event.type + '/' + event.name + \
                 ', ' + \\(event.target === this\\);",
            ),
            &[],
        );
        let mut view = ViewState::of(&document);
        view.run_scripts_with(Some(Engine::runner(Budget::FIELD_EVENT)));
        assert_eq!(
            view.run_document_scripts(&document, DocumentTrigger::WillSave, 0),
            1
        );
        assert_eq!(
            view.field_value(&document, "Note")
                .map(|shown| shown.text)
                .as_deref(),
            Some("final, Doc/WillSave, true"),
            "{:?}",
            view.script_reports()
        );
        let written = view.save(&document).expect("the save writes");
        let reopened = pdf_syntax::Document::open(written.bytes).expect("the saved file opens");
        let fresh = ViewState::of(&reopened);
        assert_eq!(
            fresh
                .field_value(&reopened, "Note")
                .map(|shown| shown.text)
                .as_deref(),
            Some("final, Doc/WillSave, true")
        );
    }

    #[test]
    fn a_script_that_set_rc_false_before_a_close_changed_what_it_wrote_and_nothing_else() {
        let document = document(
            &script(
                "WC",
                "this.getField\\('Note'\\).value = 'closing'; event.rc = false;",
            ),
            &[],
        );
        let mut view = ViewState::of(&document);
        view.run_scripts_with(Some(Engine::runner(Budget::FIELD_EVENT)));
        assert_eq!(
            view.run_document_scripts(&document, DocumentTrigger::WillClose, 0),
            1
        );
        assert_eq!(
            view.field_value(&document, "Note")
                .map(|shown| shown.text)
                .as_deref(),
            Some("closing")
        );
        assert!(
            view.script_reports()
                .iter()
                .any(|sentence| sentence.contains("the close goes ahead")),
            "{:?}",
            view.script_reports()
        );
    }
}
