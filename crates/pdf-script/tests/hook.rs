//! RFC 0008 section 6.3's hook, driven through `pdf_model::view::ViewState`: a field script Tier 0
//! does not run goes to a supplied runner, and with none supplied — the level `off`, the default —
//! nothing runs (ADR 1591).

#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

use pdf_model::aform::Trigger;
use pdf_model::view::{Entered, FieldEvent, FieldResult, ScriptRunner, ViewState};
use pdf_syntax::Document;

/// A one-page document with one text field, `Amount`, whose `/AA` holds `actions`.
fn document(actions: &str) -> Document {
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [4 0 R] >>".to_owned(),
        format!(
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Amount) /Rect [10 10 210 40] \
             /DA (/Helv 10 Tf 0 g) /AA << {actions} >> >>"
        ),
    ];
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

/// A runner that records what it was handed and answers as told.
#[derive(Debug, Default)]
struct Recording {
    /// Each event's trigger, script, value and change.
    seen: Mutex<Vec<(Trigger, String, String, String)>>,
}

impl ScriptRunner for Recording {
    fn run(&self, event: &FieldEvent<'_>) -> FieldResult {
        if let Ok(mut seen) = self.seen.lock() {
            seen.push((
                event.trigger,
                event.script.to_owned(),
                event.value.to_owned(),
                event.change.to_owned(),
            ));
        }
        FieldResult {
            rc: event.change != "x",
            value: (event.trigger == Trigger::Format).then(|| format!("<{}>", event.value)),
            change: None,
            report: vec!["the runner ran".to_owned()],
        }
    }
}

const SCRIPTS: &str = "/K << /S /JavaScript /JS (if \\(event.change == 'x'\\) event.rc = false;) >> \
                       /F << /S /JavaScript /JS (event.value = '<' + event.value + '>';) >>";

#[test]
fn with_no_runner_supplied_nothing_runs_and_each_script_is_reported() {
    let document = document(SCRIPTS);
    let mut view = ViewState::of(&document);
    assert!(view.set_field(&document, "Amount", &Entered::Text("x".to_owned())) > 0);
    assert_eq!(
        view.displayed_value(&document, "Amount").as_deref(),
        Some("x")
    );
    assert!(
        view.script_reports()
            .iter()
            .all(|sentence| sentence.contains("a script this tier does not run")),
        "{:?}",
        view.script_reports()
    );
}

#[test]
fn a_supplied_runner_is_handed_the_keystroke_and_the_format() {
    let document = document(SCRIPTS);
    let runner = Arc::new(Recording::default());
    let mut view = ViewState::of(&document);
    view.run_scripts_with(Some(runner.clone()));
    assert_eq!(
        view.set_field(&document, "Amount", &Entered::Text("x".to_owned())),
        0,
        "the runner's rc false rejects the keystroke"
    );
    assert!(view.set_field(&document, "Amount", &Entered::Text("12".to_owned())) > 0);
    view.commit_field(&document, "Amount");
    assert_eq!(
        view.displayed_value(&document, "Amount").as_deref(),
        Some("<12>")
    );
    let seen = runner.seen.lock().expect("the record").clone();
    let triggers: Vec<Trigger> = seen.iter().map(|(trigger, ..)| *trigger).collect();
    assert_eq!(
        triggers,
        vec![
            Trigger::Keystroke,
            Trigger::Keystroke,
            Trigger::Keystroke,
            Trigger::Format
        ]
    );
    assert!(seen.iter().all(|(_, script, ..)| !script.is_empty()));
    assert!(
        view.script_reports()
            .iter()
            .any(|sentence| sentence == "Amount: the runner ran"),
        "{:?}",
        view.script_reports()
    );
    view.run_scripts_with(None);
    assert_eq!(
        view.displayed_value(&document, "Amount").as_deref(),
        Some("12"),
        "taking the runner back is the level off again"
    );
}

#[cfg(feature = "engine")]
mod engine {
    use super::{Entered, ViewState, document};
    use pdf_model::view::Committed;
    use pdf_script::{Budget, Engine};

    #[test]
    fn the_engine_rejects_formats_and_refuses_a_commit_through_the_view_state() {
        let document = document(
            "/K << /S /JavaScript /JS (if \\(!event.willCommit && !/^[0-9.]*$/.test\\(event.change\\)\\) \
             event.rc = false; if \\(event.willCommit && event.value == '13'\\) event.rc = false;) >> \
             /F << /S /JavaScript /JS (if \\(event.value != ''\\) AFNumber_Format\\(2, 0, 0, 0, '$', true\\);) >>",
        );
        let mut view = ViewState::of(&document);
        view.run_scripts_with(Some(Engine::runner(Budget::FIELD_EVENT)));
        assert_eq!(
            view.set_field(&document, "Amount", &Entered::Text("1a".to_owned())),
            0
        );
        assert!(view.set_field(&document, "Amount", &Entered::Text("1234.5".to_owned())) > 0);
        assert_eq!(view.commit_field(&document, "Amount"), Committed::Accepted);
        assert_eq!(
            view.displayed_value(&document, "Amount").as_deref(),
            Some("$1,234.50")
        );
        assert!(view.set_field(&document, "Amount", &Entered::Text("13".to_owned())) > 0);
        assert!(matches!(
            view.commit_field(&document, "Amount"),
            Committed::Refused(_)
        ));
        assert_eq!(
            view.field_value(&document, "Amount")
                .map(|shown| shown.text)
                .as_deref(),
            Some("1234.5"),
            "a refused commit puts back what was there"
        );
    }

    #[test]
    fn a_runaway_format_leaves_the_value_as_it_stands() {
        let document = document("/F << /S /JavaScript /JS (while \\(true\\) {}) >>");
        let mut view = ViewState::of(&document);
        view.run_scripts_with(Some(Engine::runner(Budget::FIELD_EVENT)));
        assert!(view.set_field(&document, "Amount", &Entered::Text("7".to_owned())) > 0);
        view.commit_field(&document, "Amount");
        assert_eq!(
            view.displayed_value(&document, "Amount").as_deref(),
            Some("7")
        );
    }
}
