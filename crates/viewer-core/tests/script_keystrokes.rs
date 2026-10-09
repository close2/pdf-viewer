//! Table 199's `/K` handed to the document's runner once for each change a person makes — a
//! character typed into a text field and a selection made in a list box — and not again at every
//! later edit, with the arrows a host said read at the selection they made (ADRs 1786, 1787).
//!
//! ISO 32000-2 §12.6.3, Table 199: the keystroke action "shall be performed when the user modifies
//! a character in a text field or combo box or modifies the selection in a scrollable list box".
//! The runner is the test's own and records what it is handed, so what is judged is the viewer's
//! half: which change reaches the runner, how often, and with which keys.

use std::fmt::Write as _;
use std::sync::{Arc, Mutex, PoisonError};

use pdf_model::aform::Trigger;
use pdf_model::view::{Entered, Keys, ScriptEvent, ScriptResult, ScriptRunner, ScriptSite};
use viewer_core::{Command, DocumentId, Edit, ScriptRunners, Scripting, Viewer};

const DOCUMENT: DocumentId = DocumentId(1);

/// One page holding the list box `Picked` and the text field `Typed`, each with a `/K`.
fn form() -> Vec<u8> {
    let keystroke = "/AA << /K << /S /JavaScript /JS (k) >> >>";
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 5 0 R] >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [4 0 R 5 0 R] >>".to_owned(),
        format!(
            "<< /Type /Annot /Subtype /Widget /FT /Ch /T (Picked) /F 4 /P 3 0 R \
             /Rect [100 100 300 180] /Opt [(Alpha) (Beta) (Gamma)] {keystroke} >>"
        ),
        format!(
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Typed) /F 4 /P 3 0 R \
             /Rect [100 250 300 280] {keystroke} >>"
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
    out.into_bytes()
}

/// Every keystroke a runner was handed: the field, the change and whether the arrows were down.
type Handed = Arc<Mutex<Vec<(String, String, bool)>>>;

/// A runner that records each keystroke it is handed and lets it stand.
#[derive(Debug)]
struct Recording(Handed);

impl ScriptRunner for Recording {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        if event.site == ScriptSite::Field(Trigger::Keystroke) {
            self.0.lock().unwrap_or_else(PoisonError::into_inner).push((
                event.field.to_owned(),
                event.change.to_owned(),
                event.keys.arrows,
            ));
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

#[derive(Debug)]
struct Recorders(Handed);

impl ScriptRunners for Recorders {
    fn runner(&self) -> Arc<dyn ScriptRunner> {
        Arc::new(Recording(Arc::clone(&self.0)))
    }
}

/// A viewer with the form open under a recording runner.
fn opened(handed: &Handed) -> Viewer {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Scripts(Scripting::Run(Arc::new(Recorders(
            Arc::clone(handed),
        )))))
        .for_each(drop);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: form().into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer.handle(Command::Presented).for_each(drop);
    viewer
}

/// Sends `value` to `field`, as a host's control does.
fn set(viewer: &mut Viewer, field: &str, value: Entered) {
    viewer
        .handle(Command::Edit(Edit::SetField {
            field: field.to_owned(),
            value,
        }))
        .for_each(drop);
}

/// What the runner was handed, as owned triples.
fn handed_of(handed: &Handed) -> Vec<(String, String, bool)> {
    handed
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

/// Three selections are three keystrokes, each handed the option it selected, and none is handed
/// again when the next edit is made.
#[test]
fn each_selection_runs_the_keystroke_once() {
    let handed = Handed::default();
    let mut viewer = opened(&handed);
    for option in 0..3 {
        set(&mut viewer, "Picked", Entered::Chosen(vec![option]));
    }
    let picked = |change: &str| ("Picked".to_owned(), change.to_owned(), false);
    assert_eq!(
        handed_of(&handed),
        [picked("Alpha"), picked("Beta"), picked("Gamma")]
    );
}

/// The arrows a host says are down around a selection are read at that selection alone.
#[test]
fn the_arrows_are_read_at_the_selection_they_made() {
    let handed = Handed::default();
    let mut viewer = opened(&handed);
    set(&mut viewer, "Picked", Entered::Chosen(vec![0]));
    let keys = |arrows| {
        Command::Keys(Keys {
            shift: false,
            modifier: false,
            arrows,
        })
    };
    viewer.handle(keys(true)).for_each(drop);
    set(&mut viewer, "Picked", Entered::Chosen(vec![1]));
    viewer.handle(keys(false)).for_each(drop);
    set(&mut viewer, "Picked", Entered::Chosen(vec![2]));
    let arrows: Vec<(String, bool)> = handed_of(&handed)
        .into_iter()
        .map(|(_, change, arrows)| (change, arrows))
        .collect();
    assert_eq!(
        arrows,
        [
            ("Alpha".to_owned(), false),
            ("Beta".to_owned(), true),
            ("Gamma".to_owned(), false)
        ]
    );
}

/// A text field typed into character by character runs its keystroke once per value a host sends,
/// and a choice made after the typing does not run the typing's keystrokes again.
#[test]
fn typed_characters_run_the_keystroke_once_each() {
    let handed = Handed::default();
    let mut viewer = opened(&handed);
    set(&mut viewer, "Typed", Entered::Text("a".to_owned()));
    set(&mut viewer, "Typed", Entered::Text("ab".to_owned()));
    set(&mut viewer, "Picked", Entered::Chosen(vec![2]));
    let changes: Vec<String> = handed_of(&handed)
        .into_iter()
        .map(|(_, change, _)| change)
        .collect();
    assert_eq!(changes, ["a", "ab", "Gamma"]);
}

/// An undo rebuilds the state from the log's surviving entries, so their keystrokes run again —
/// the one place they do (ADR 1787).
#[test]
fn an_undo_replays_the_surviving_entries() {
    let handed = Handed::default();
    let mut viewer = opened(&handed);
    set(&mut viewer, "Picked", Entered::Chosen(vec![0]));
    set(&mut viewer, "Picked", Entered::Chosen(vec![1]));
    handed
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clear();
    viewer.handle(Command::Undo).for_each(drop);
    let changes: Vec<String> = handed_of(&handed)
        .into_iter()
        .map(|(_, change, _)| change)
        .collect();
    assert_eq!(changes, ["Alpha"]);
}
