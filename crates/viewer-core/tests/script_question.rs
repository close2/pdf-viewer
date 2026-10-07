//! RFC 0008 section 4.2's `app.alert` and `app.response` reached from a host: a question a
//! document's runner holds goes out as `Event::ScriptAsking` after the command that raised it,
//! once, and the person's `Command::AnswerScript` reaches the runner (ADR 1628).
//!
//! The runner here is the test's own: its calculate script "asks" by leaving a question where its
//! maker said questions wait, as a runner whose script waits in its worker does. Whether the
//! question reaches the host and the answer reaches the runner is what is judged, never what a
//! script does with the answer.

#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing"
)]

use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

use pdf_model::view::{ScriptEvent, ScriptResult, ScriptRunner};
use viewer_core::{
    AlertButton, AlertButtons, AlertIcon, Command, DocumentId, Edit, Entered, Event, ScriptAnswer,
    ScriptAsks, ScriptQuestion, ScriptRunners, Scripting, Viewer,
};

const DOCUMENT: DocumentId = DocumentId(1);

/// The total's calculation: a script, which only a runner runs.
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

/// Where one document's questions wait: what is waiting, and every answer given.
#[derive(Debug, Default)]
struct Desk {
    waiting: Mutex<Option<ScriptQuestion>>,
    answers: Mutex<Vec<ScriptAnswer>>,
}

impl ScriptAsks for Desk {
    fn take_question(&self) -> Option<ScriptQuestion> {
        self.waiting.lock().expect("one test thread").take()
    }

    fn answer(&self, answer: ScriptAnswer) {
        self.answers.lock().expect("one test thread").push(answer);
    }
}

/// A runner whose every script asks one question and changes nothing.
#[derive(Debug)]
struct Asking {
    desk: Arc<Desk>,
}

impl ScriptRunner for Asking {
    fn run(&self, _: &ScriptEvent<'_>) -> ScriptResult {
        *self.desk.waiting.lock().expect("one test thread") = Some(alert());
        ScriptResult {
            rc: true,
            value: None,
            change: None,
            edits: Vec::new(),
            report: Vec::new(),
        }
    }
}

/// The question the runner asks.
fn alert() -> ScriptQuestion {
    ScriptQuestion::Alert {
        message: "Recalculate the total?".to_owned(),
        icon: AlertIcon::Question,
        buttons: AlertButtons::YesNo,
        title: None,
    }
}

/// A maker whose runners ask, keeping the one desk the test reads.
#[derive(Debug, Default)]
struct Askers {
    desk: Arc<Desk>,
}

impl ScriptRunners for Askers {
    fn runner(&self) -> Arc<dyn ScriptRunner> {
        self.runner_that_asks().0
    }

    fn runner_that_asks(&self) -> (Arc<dyn ScriptRunner>, Option<Arc<dyn ScriptAsks>>) {
        let desk: Arc<dyn ScriptAsks> = Arc::clone(&self.desk) as Arc<dyn ScriptAsks>;
        (
            Arc::new(Asking {
                desk: Arc::clone(&self.desk),
            }),
            Some(desk),
        )
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

fn questions(events: &[Event]) -> Vec<ScriptQuestion> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::ScriptAsking { document, question } if *document == DOCUMENT => {
                Some(question.clone())
            }
            _ => None,
        })
        .collect()
}

/// The question a runner holds goes out after the commit that raised it, once.
#[test]
fn a_question_a_script_waits_on_is_put_once_after_its_command() {
    let askers = Arc::new(Askers::default());
    let mut viewer = opened(Scripting::Run(Arc::clone(&askers) as Arc<dyn ScriptRunners>));
    // Handing the runner over walks `/CO`, and the calculation asked: taken there or here.
    askers.desk.waiting.lock().expect("one test thread").take();
    let mut events: Vec<Event> = viewer
        .handle(Command::Edit(Edit::SetField {
            field: "Price1".to_owned(),
            value: Entered::Text("12.5".to_owned()),
        }))
        .collect();
    events.extend(viewer.handle(Command::CommitField {
        field: "Price1".to_owned(),
    }));
    assert_eq!(
        questions(&events),
        vec![alert()],
        "put once, after the commit"
    );
    let later: Vec<Event> = viewer.handle(Command::Tick { millis: 16 }).collect();
    assert!(questions(&later).is_empty(), "and not put again");
}

/// The answer reaches the runner's desk; a document with no desk takes none.
#[test]
fn the_answer_reaches_the_runner_that_asked() {
    let askers = Arc::new(Askers::default());
    let mut viewer = opened(Scripting::Run(Arc::clone(&askers) as Arc<dyn ScriptRunners>));
    viewer
        .handle(Command::AnswerScript {
            document: DOCUMENT,
            answer: ScriptAnswer::Pressed(AlertButton::Yes),
        })
        .for_each(drop);
    viewer
        .handle(Command::AnswerScript {
            document: DocumentId(9),
            answer: ScriptAnswer::Typed(None),
        })
        .for_each(drop);
    assert_eq!(
        *askers.desk.answers.lock().expect("one test thread"),
        vec![ScriptAnswer::Pressed(AlertButton::Yes)],
        "the document's answer, and nothing of the one that is not open"
    );
}

/// At `off` no runner is handed over, so nothing waits and an answer goes nowhere.
#[test]
fn at_off_nothing_asks_and_an_answer_is_dropped() {
    let mut viewer = opened(Scripting::Off);
    let events: Vec<Event> = viewer
        .handle(Command::AnswerScript {
            document: DOCUMENT,
            answer: ScriptAnswer::Unanswerable,
        })
        .collect();
    assert!(questions(&events).is_empty());
}
