//! ISO 32000-2 §12.6.4.14's rendition action and its Table 218 `/JS`.
//!
//! > Either the JS entry or the OP entry shall be present. If both are present, OP is considered a
//! > fallback that shall be executed if the interactive PDF processor is unable to execute
//! > ECMAScripts.
//!
//! The script is "an ECMAScript script that shall be executed when the action is triggered", so a
//! view state holding a runner hands it over at the trigger and does not perform `/OP`; one holding
//! none performs `/OP` (ADR 2988). The wrong answers each test excludes: a rendition's script never
//! handed over, its `/OP` performed beside a script that ran, and `/OP` dropped where nothing can
//! run the script.

#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

use pdf_model::action::{Action, Trigger};
use pdf_model::view::{Request, ScriptEvent, ScriptResult, ScriptRunner, ViewState};
use pdf_syntax::{Document, ObjectId};

/// A document of `objects`, numbered from 1, the first the catalog.
fn assembled(objects: &[&str]) -> Document {
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
    let document = Document::open(out.into_bytes()).expect("the fixture opens");
    assert!(!document.was_recovered(), "the fixture's offsets are exact");
    document
}

/// One page whose link (4) activates `action` (6), a rendition of a clip into the screen
/// annotation (5).
fn linked(action: &str) -> Document {
    assembled(&[
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Annots [4 0 R 5 0 R] >>",
        "<< /Type /Annot /Subtype /Link /P 3 0 R /Rect [50 0 100 50] /A 6 0 R >>",
        "<< /Type /Annot /Subtype /Screen /P 3 0 R /Rect [0 0 50 50] >>",
        action,
        "<< /Type /Rendition /S /MR /N (clip) /C 8 0 R >>",
        "<< /Type /MediaClip /S /MCD /CT (video/mp4) /D (clip.mp4) >>",
    ])
}

/// A runner that records the text of every script it is handed.
#[derive(Debug, Default)]
struct Recording(Mutex<Vec<String>>);

impl ScriptRunner for Recording {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        self.0
            .lock()
            .expect("the record")
            .push(event.script.to_owned());
        ScriptResult {
            rc: true,
            value: None,
            change: None,
            edits: Vec::new(),
            report: Vec::new(),
        }
    }
}

/// Activates the link under `runner`, after the open sequence, and answers the scripts the runner
/// was handed and how many rendition requests the chain performed.
fn activated(document: &Document, runner: Option<Arc<Recording>>) -> (Vec<String>, usize) {
    let mut view = ViewState::of(document);
    view.run_scripts_with(runner.clone().map(|runner| runner as Arc<dyn ScriptRunner>));
    view.run_open_scripts(document, 0);
    let link = ObjectId::new(4, 0);
    let dictionary = document.get(link).as_dict().cloned().expect("the link");
    let actions = pdf_model::action::read(document, &document.get_key(&dictionary, "A"));
    let (_, requests) = view.perform_annotation_chain(document, link, Trigger::Up, &actions);
    let renditions = requests
        .iter()
        .filter(|request| matches!(request, Request::Rendition(_)))
        .count();
    let scripts = runner.map_or_else(Vec::new, |runner| {
        runner.0.lock().expect("the record").clone()
    });
    (scripts, renditions)
}

#[test]
fn a_rendition_states_whether_it_carries_a_script() {
    let document = linked("<< /S /Rendition /OP 0 /JS (play\\(\\);) /AN 5 0 R /R 7 0 R >>");
    let link = document
        .get(ObjectId::new(4, 0))
        .as_dict()
        .cloned()
        .expect("the link");
    let actions = pdf_model::action::read(&document, &document.get_key(&link, "A"));
    assert!(
        matches!(actions.as_slice(), [Action::Rendition(rendition)] if rendition.script),
        "{actions:?}"
    );
    let plain = linked("<< /S /Rendition /OP 0 /AN 5 0 R /R 7 0 R >>");
    let link = plain
        .get(ObjectId::new(4, 0))
        .as_dict()
        .cloned()
        .expect("the link");
    let actions = pdf_model::action::read(&plain, &plain.get_key(&link, "A"));
    assert!(
        matches!(actions.as_slice(), [Action::Rendition(rendition)] if !rendition.script),
        "{actions:?}"
    );
}

#[test]
fn a_runner_is_handed_the_rendition_script_and_the_op_is_not_performed() {
    let document = linked("<< /S /Rendition /OP 0 /JS (play\\(\\);) /AN 5 0 R /R 7 0 R >>");
    let runner = Arc::new(Recording::default());
    assert_eq!(
        activated(&document, Some(runner)),
        (vec!["play();".to_owned()], 0)
    );
}

#[test]
fn without_a_runner_the_op_is_the_fallback_performed() {
    let document = linked("<< /S /Rendition /OP 0 /JS (play\\(\\);) /AN 5 0 R /R 7 0 R >>");
    assert_eq!(activated(&document, None), (Vec::new(), 1));
}

#[test]
fn a_rendition_without_a_script_performs_its_op_under_a_runner() {
    let document = linked("<< /S /Rendition /OP 0 /AN 5 0 R /R 7 0 R >>");
    let runner = Arc::new(Recording::default());
    assert_eq!(activated(&document, Some(runner)), (Vec::new(), 1));
}

#[test]
fn a_rendition_script_runs_in_its_place_in_the_chain() {
    // Table 196's `/Next` order: the JavaScript action, then the rendition's script.
    let document = linked(
        "<< /S /JavaScript /JS (first\\(\\);) /Next << /S /Rendition /OP 0 /JS (second\\(\\);) \
         /AN 5 0 R /R 7 0 R >> >>",
    );
    let runner = Arc::new(Recording::default());
    assert_eq!(
        activated(&document, Some(runner)),
        (vec!["first();".to_owned(), "second();".to_owned()], 0)
    );
}
