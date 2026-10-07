//! RFC 0008 section 6.3's level reached from a host: `off` runs nothing, `run` hands every document
//! a runner of its own, and `ask` puts one question per document at its first script and holds the
//! answer (ADR 1616).
//!
//! The fixture is round 1371's three-field shape with one change: the total's Table 199 `/C` is a
//! script rather than one call of the `AF` library, so Tier 0 does not run it and only a runner
//! can. The runner here is the test's own, keeping a realm of field values as the real one does
//! (ADR 1602) and computing the sum a calculation asks for; whether a script runs is what is
//! judged, never how.

#![expect(
    clippy::panic,
    clippy::expect_used,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing"
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use pdf_model::aform::Trigger;
use pdf_model::view::{
    DocumentTrigger, ScriptEdit, ScriptEvent, ScriptResult, ScriptRunner, ScriptSite,
};
use viewer_core::{
    Answer, Command, DocumentId, Edit, Entered, Event, Query, ScriptRunners, Scripting, Viewer,
};

const DOCUMENT: DocumentId = DocumentId(1);

/// The total's calculation: a script, which Tier 0 reports and does not run.
const SUM: &str = "event.value = Number(this.getField(\"Price1\").value) + \
                   Number(this.getField(\"Price2\").value);";

/// Two priced lines and their total, widgets 4 to 6 on one page, the total in Table 224's `/CO`.
fn form() -> Vec<u8> {
    form_with("")
}

/// [`form`], with `catalog` added to the catalog dictionary.
fn form_with(catalog: &str) -> Vec<u8> {
    let field = |name: &str, top: u32| {
        format!(
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T ({name}) /F 4 \
             /Rect [20 {} 220 {top}] /DA (/Helv 12 Tf 0 g) >>",
            top.saturating_sub(30)
        )
    };
    let script = SUM.replace('(', "\\(").replace(')', "\\)");
    let bodies = [
        format!(
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 5 0 R 6 0 R] /CO [6 0 R] \
             /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv 7 0 R >> >> >> {catalog} >>"
        ),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [4 0 R 5 0 R 6 0 R] >>"
            .to_owned(),
        field("Price1", 380),
        field("Price2", 340),
        format!(
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Total) /Ff 1 /F 4 /Rect [20 230 220 260] \
             /DA (/Helv 12 Tf 0 g) /AA << /C << /S /JavaScript /JS ({script}) >> >> >>"
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
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

/// A runner holding one document's realm: the fields as it was last told of them.
#[derive(Debug, Default)]
struct Summing {
    /// Every field's value, by name.
    realm: Mutex<BTreeMap<String, String>>,
}

impl ScriptRunner for Summing {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        let mut realm = self.realm.lock().expect("one test thread");
        for field in event.fields {
            realm.insert(field.name.clone(), field.value.clone());
        }
        let number = |name: &str| {
            realm
                .get(name)
                .and_then(|value| value.parse::<f64>().ok())
                .unwrap_or(0.0)
        };
        let value = (event.site == ScriptSite::Field(Trigger::Calculate))
            .then(|| (number("Price1") + number("Price2")).to_string());
        ScriptResult {
            rc: true,
            value,
            change: None,
            edits: Vec::new(),
            report: Vec::new(),
        }
    }
}

/// The host's maker of runners, counting how many it made.
#[derive(Debug, Default)]
struct Runners {
    made: AtomicUsize,
}

impl ScriptRunners for Runners {
    fn runner(&self) -> Arc<dyn ScriptRunner> {
        self.made.fetch_add(1, Ordering::SeqCst);
        Arc::new(Summing::default())
    }
}

fn opened(scripting: Scripting) -> Viewer {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer.handle(Command::Scripts(scripting)).for_each(drop);
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

/// Types `text` into `field` and commits it, which is the moment `/CO` is walked with scripts.
fn entered(viewer: &mut Viewer, field: &str, text: &str) -> Vec<Event> {
    let mut events: Vec<Event> = viewer
        .handle(Command::Edit(Edit::SetField {
            field: field.to_owned(),
            value: Entered::Text(text.to_owned()),
        }))
        .collect();
    events.extend(viewer.handle(Command::CommitField {
        field: field.to_owned(),
    }));
    events
}

fn value(viewer: &Viewer, field: &str) -> String {
    let Answer::Fields(fields) = viewer.query(Query::Fields) else {
        panic!("a form answers its fields");
    };
    fields
        .into_iter()
        .find(|candidate| candidate.name.qualified == field)
        .and_then(|field| field.value)
        .map(|shown| shown.text)
        .unwrap_or_default()
}

fn sentences(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Reported { notes, .. } => Some(notes.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

fn questions(events: &[Event]) -> Vec<(String, String)> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::AskingToRunScripts {
                script, first_line, ..
            } => Some((script.clone(), first_line.clone())),
            _ => None,
        })
        .collect()
}

/// `off`: nothing runs, nothing is made, and the commit says the total's script was not run.
#[test]
fn at_off_no_script_runs_and_the_sentence_says_so() {
    let mut viewer = opened(Scripting::Off);
    let events = entered(&mut viewer, "Price1", "12.5");
    assert_eq!(value(&viewer, "Total"), "", "no script computed the total");
    assert!(
        questions(&events).is_empty(),
        "off asks nothing: {events:?}"
    );
    let said = sentences(&events);
    assert!(
        said.iter()
            .any(|sentence| sentence.starts_with("Total: ") && sentence.contains("does not run")),
        "{said:?}"
    );
}

/// `run`: one runner per document, and the commit computes the total through it.
#[test]
fn at_run_the_total_is_computed_by_the_script() {
    let runners = Arc::new(Runners::default());
    let mut viewer = opened(Scripting::Run(runners.clone()));
    assert_eq!(runners.made.load(Ordering::SeqCst), 1, "one per document");
    entered(&mut viewer, "Price1", "12.5");
    entered(&mut viewer, "Price2", "7");
    assert_eq!(value(&viewer, "Total"), "19.5");
}

/// `ask`, answered `yes`: one question at the first script, naming it and showing its first line;
/// the commit that raised it changes nothing, and the answer computes the total from what is typed.
#[test]
fn at_ask_one_question_is_put_and_a_yes_runs_what_was_withheld() {
    let runners = Arc::new(Runners::default());
    let mut viewer = opened(Scripting::Ask(runners.clone()));
    assert_eq!(
        runners.made.load(Ordering::SeqCst),
        0,
        "nothing runs unasked"
    );
    let events = entered(&mut viewer, "Price1", "12.5");
    let asked = questions(&events);
    assert_eq!(asked.len(), 1, "{events:?}");
    assert_eq!(asked[0].0, "the calculate script of Total");
    assert!(asked[0].1.starts_with("event.value = Number("), "{asked:?}");
    assert_eq!(value(&viewer, "Total"), "", "withheld until the answer");
    let again = entered(&mut viewer, "Price2", "7");
    assert!(questions(&again).is_empty(), "asked once per document");
    viewer
        .handle(Command::AnswerScripts {
            document: DOCUMENT,
            proceed: true,
        })
        .for_each(drop);
    assert_eq!(runners.made.load(Ordering::SeqCst), 1);
    assert_eq!(
        value(&viewer, "Total"),
        "19.5",
        "the yes ran the calculation"
    );
    // The level sent again keeps the answer: no second question, and the scripts still run.
    let resent: Vec<Event> = viewer
        .handle(Command::Scripts(Scripting::Ask(runners.clone())))
        .collect();
    assert!(questions(&resent).is_empty(), "{resent:?}");
    entered(&mut viewer, "Price2", "8");
    assert_eq!(value(&viewer, "Total"), "20.5");
}

/// `ask`, answered `no`: the scripts stay withheld for the document's lifetime, said each time.
#[test]
fn at_ask_a_no_holds_until_the_document_closes() {
    let runners = Arc::new(Runners::default());
    let mut viewer = opened(Scripting::Ask(runners.clone()));
    let events = entered(&mut viewer, "Price1", "12.5");
    assert_eq!(questions(&events).len(), 1);
    viewer
        .handle(Command::AnswerScripts {
            document: DOCUMENT,
            proceed: false,
        })
        .for_each(drop);
    let later = entered(&mut viewer, "Price2", "7");
    assert!(questions(&later).is_empty(), "{later:?}");
    assert_eq!(value(&viewer, "Total"), "");
    assert_eq!(runners.made.load(Ordering::SeqCst), 0);
    assert!(
        sentences(&later)
            .iter()
            .any(|sentence| sentence.contains("you declined")),
        "{later:?}"
    );
}

/// A level changed while a document is open reaches it: `off` to `run` computes the total from
/// what was typed under `off`.
#[test]
fn a_level_sent_while_open_reaches_the_open_document() {
    let mut viewer = opened(Scripting::Off);
    entered(&mut viewer, "Price1", "12.5");
    assert_eq!(value(&viewer, "Total"), "");
    viewer
        .handle(Command::Scripts(Scripting::Run(Arc::new(
            Runners::default(),
        ))))
        .for_each(drop);
    assert_eq!(value(&viewer, "Total"), "12.5");
}

/// A runner that remembers every site it was handed, and answers a document's open action by
/// asking for the focus on `Price2`.
#[derive(Debug, Default)]
struct Recording {
    sites: Mutex<Vec<ScriptSite>>,
}

impl ScriptRunner for Recording {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        self.sites.lock().expect("one test thread").push(event.site);
        let edits = if event.site == ScriptSite::OpenAction {
            vec![ScriptEdit::Focus {
                field: "Price2".to_owned(),
            }]
        } else {
            Vec::new()
        };
        ScriptResult {
            rc: true,
            value: None,
            change: None,
            edits,
            report: Vec::new(),
        }
    }
}

#[derive(Debug, Default)]
struct OneRecording(Arc<Recording>);

impl ScriptRunners for OneRecording {
    fn runner(&self) -> Arc<dyn ScriptRunner> {
        self.0.clone()
    }
}

/// Table 200's five moments reach the runner where the viewer does each operation — around a
/// save, around a print and before a close — and a script's `setFocus` moves the keyboard (ADRs
/// 1614, 1615, 1616).
#[test]
fn the_documents_moments_and_a_focus_request_reach_the_runner() {
    let script = |key: &str| format!("/{key} << /S /JavaScript /JS (moment\\(\\);) >>");
    let catalog = format!(
        "/OpenAction << /S /JavaScript /JS (focus\\(\\);) >> /AA << {} {} {} {} {} >>",
        script("WC"),
        script("WS"),
        script("DS"),
        script("WP"),
        script("DP")
    );
    let recording = Arc::new(Recording::default());
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Scripts(Scripting::Run(Arc::new(OneRecording(
            recording.clone(),
        )))))
        .for_each(drop);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: form_with(&catalog).into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer.handle(Command::Presented).for_each(drop);
    let Answer::Focus { object, .. } = viewer.query(Query::Focus) else {
        panic!("the open action's setFocus put the keyboard on a widget");
    };
    assert_eq!(object.number, 5, "Price2's widget");
    viewer.handle(Command::Save).for_each(drop);
    viewer
        .handle(Command::Print(viewer_core::Printing::Start(
            viewer_core::Sheet {
                media: None,
                scale: 1.0,
                page_scale: 1.0,
            },
        )))
        .for_each(drop);
    viewer
        .handle(Command::Print(viewer_core::Printing::Finish))
        .for_each(drop);
    viewer.handle(Command::Close(DOCUMENT)).for_each(drop);
    let moments: Vec<DocumentTrigger> = recording
        .sites
        .lock()
        .expect("one test thread")
        .iter()
        .filter_map(|site| match site {
            ScriptSite::Document(trigger) => Some(*trigger),
            _ => None,
        })
        .collect();
    assert_eq!(
        moments,
        [
            DocumentTrigger::WillSave,
            DocumentTrigger::DidSave,
            DocumentTrigger::WillPrint,
            DocumentTrigger::DidPrint,
            DocumentTrigger::WillClose
        ]
    );
}
