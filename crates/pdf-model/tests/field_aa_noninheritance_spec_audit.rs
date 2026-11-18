//! Table 226 does not mark field `/AA` inheritable. An ancestor-only `/K` must not become the
//! terminal field's keystroke action; a terminal-local `/K` is the positive control.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "small exact-xref PDF fixtures with explanatory failures"
)]

use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

use pdf_model::view::{Entered, ScriptEvent, ScriptResult, ScriptRunner, ViewState};
use pdf_syntax::Document;

fn pdf(parent_action: bool, terminal_action: bool) -> Document {
    let parent_aa = if parent_action {
        "/AA << /K 8 0 R >>"
    } else {
        ""
    };
    let terminal_aa = if terminal_action {
        "/AA << /K 9 0 R >>"
    } else {
        ""
    };
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] /NeedAppearances true /DA (/Helv 10 Tf 0 g) /DR << /Font << /Helv 7 0 R >> >> >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> /Annots [6 0 R] >>".to_owned(),
        format!("<< /T (group) /Kids [5 0 R] {parent_aa} >>"),
        format!("<< /FT /Tx /T (leaf) /V (old) /Parent 4 0 R /Kids [6 0 R] /DA (/Helv 10 Tf 0 g) {terminal_aa} >>"),
        "<< /Type /Annot /Subtype /Widget /Parent 5 0 R /P 3 0 R /Rect [10 10 90 30] /F 4 >>".to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        "<< /S /JavaScript /JS (var marker = 1;) >>".to_owned(),
        "<< /S /JavaScript /JS (var marker = 2;) >>".to_owned(),
    ];
    let mut bytes = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let _ = write!(bytes, "{} 0 obj\n{body}\nendobj\n", i + 1);
    }
    let xref = bytes.len();
    let _ = write!(
        bytes,
        "xref\n0 {}\n0000000000 65535 f \n",
        offsets.len() + 1
    );
    for offset in offsets {
        let _ = writeln!(bytes, "{offset:010} 00000 n ");
    }
    let _ = write!(
        bytes,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    );
    Document::open(bytes.into_bytes()).expect("complete PDF with exact xref")
}

#[derive(Debug, Default)]
struct Recording(Mutex<Vec<String>>);

impl ScriptRunner for Recording {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        self.0
            .lock()
            .expect("recording mutex is not poisoned")
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

fn keystroke_scripts(parent_action: bool, terminal_action: bool) -> Vec<String> {
    let document = pdf(parent_action, terminal_action);
    assert!(!document.was_recovered(), "the exact xref is authoritative");
    let runner = Arc::new(Recording::default());
    let mut view = ViewState::of(&document);
    view.run_scripts_with(Some(Arc::clone(&runner) as Arc<dyn ScriptRunner>));
    assert!(view.set_field(&document, "group.leaf", &Entered::Text("new".to_owned())) > 0);
    runner.0.lock().expect("runner lock").clone()
}

#[test]
fn a_nonterminal_field_aa_is_not_inherited_by_its_terminal_descendant() {
    assert_eq!(
        keystroke_scripts(true, false),
        Vec::<String>::new(),
        "nonterminal AA must not become a terminal keystroke action"
    );
}

#[test]
fn a_terminal_field_aa_is_used_for_its_keystroke_event() {
    assert_eq!(keystroke_scripts(false, true), vec!["var marker = 2;"]);
    assert_eq!(
        keystroke_scripts(true, true),
        vec!["var marker = 2;"],
        "local field action is selected when both dictionaries state AA"
    );
}

#[test]
fn an_absent_field_aa_does_not_dispatch_a_keystroke_script() {
    assert_eq!(keystroke_scripts(false, false), [] as [String; 0]);
}
