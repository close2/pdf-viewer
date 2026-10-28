//! A hybrid form's calculations reach a runner at the two moments XFA 3.3 page 379 names, and
//! what `override` asks a host to present (pages 641–642) goes out as the notice a window already
//! draws for a script's question (ADRs 2524, 2525).
//!
//! The runner here is the test's own: it understands one script, a decimal number, and assigns it
//! to the container the script belongs to. Whether the viewer asks at the right moment, applies
//! what the runner computed, withholds what *ask* has not allowed and puts a notice once and in
//! order is what is judged, never what a script language does.

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing"
)]

use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

use pdf_model::view::{ScriptEvent, ScriptResult, ScriptRunner};
use pdf_model::xfa::{Form, FormLanguage, Ran, Scalar};
use viewer_core::{
    AlertButton, AlertButtons, AlertIcon, Answer, Command, DocumentId, Edit, Entered, Event, Query,
    ScriptAnswer, ScriptAsks, ScriptQuestion, ScriptRunners, Scripting, Viewer,
};

const DOCUMENT: DocumentId = DocumentId(1);

/// A hybrid form of one subform `form1` holding a field `Total` whose calculation is `6` with the
/// `override` the attribute text states, and an input `Units`.
fn form(attributes: &str) -> Vec<u8> {
    form_in(attributes, "")
}

/// [`form`], its script element carrying `script_attributes`: a `contentType` names its language
/// (XFA 3.3 page 106).
fn form_in(attributes: &str, script_attributes: &str) -> Vec<u8> {
    let stream = |text: &str| {
        format!(
            "<< /Length {} >>\nstream\n{text}\nendstream",
            text.len().saturating_add(1)
        )
    };
    let template = format!(
        "<template xmlns=\"http://www.xfa.org/schema/xfa-template/3.3/\"><subform name=\"form1\">\
         <field name=\"Units\"><ui><textEdit/></ui><value><text/></value></field>\
         <field name=\"Total\"><ui><textEdit/></ui><value><text/></value>\
         <calculate {attributes}><script {script_attributes}>6</script></calculate></field>\
         </subform></template>"
    );
    let datasets = "<xfa:datasets xmlns:xfa=\"http://www.xfa.org/schema/xfa-data/1.0/\">\
                    <xfa:data><form1><Units>1</Units><Total/></form1></xfa:data></xfa:datasets>";
    let widget = |name: &str, y: u32| {
        format!(
            "<< /Type /Annot /Subtype /Widget /FT /Tx /Parent 4 0 R /T ({name}[0]) /P 3 0 R \
             /V () /Rect [20 {y} 220 {}] /F 4 /DA (/Helv 12 Tf 0 g) >>",
            y.saturating_add(20)
        )
    };
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] \
         /XFA [(template) 5 0 R (datasets) 6 0 R] >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [7 0 R 8 0 R] >>".to_owned(),
        "<< /T (form1[0]) /Kids [7 0 R 8 0 R] >>".to_owned(),
        stream(&template),
        stream(datasets),
        widget("Units", 300),
        widget("Total", 260),
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

/// A runner that understands a form script that is a decimal number.
#[derive(Debug)]
struct Calculator;

impl ScriptRunner for Calculator {
    fn run(&self, _: &ScriptEvent<'_>) -> ScriptResult {
        ScriptResult {
            rc: true,
            value: None,
            change: None,
            edits: Vec::new(),
            report: Vec::new(),
        }
    }

    fn run_form_script(
        &self,
        form: &mut Form<'_>,
        node: usize,
        _language: FormLanguage,
        script: &str,
        _steps_left: u64,
    ) -> Ran {
        let Ok(number) = script.trim().parse::<f64>() else {
            return Ran {
                steps: 0,
                outcome: Err(format!("{script:?} is not a number")),
            };
        };
        if let Err(sentence) = form.set(node, Scalar::Number(number)) {
            return Ran {
                steps: 1,
                outcome: Err(sentence),
            };
        }
        Ran {
            steps: 1,
            outcome: Ok(Scalar::Number(number)),
        }
    }
}

/// Where the runner's questions wait: it puts none, and keeps every answer it is handed.
#[derive(Debug, Default)]
struct Desk {
    answers: Mutex<Vec<ScriptAnswer>>,
}

impl ScriptAsks for Desk {
    fn take_question(&self) -> Option<ScriptQuestion> {
        None
    }

    fn answer(&self, answer: ScriptAnswer) {
        self.answers.lock().expect("one test thread").push(answer);
    }
}

#[derive(Debug, Default)]
struct Calculators {
    desk: Arc<Desk>,
}

impl ScriptRunners for Calculators {
    fn runner(&self) -> Arc<dyn ScriptRunner> {
        Arc::new(Calculator)
    }

    fn runner_that_asks(&self) -> (Arc<dyn ScriptRunner>, Option<Arc<dyn ScriptAsks>>) {
        let desk: Arc<dyn ScriptAsks> = Arc::clone(&self.desk) as Arc<dyn ScriptAsks>;
        (Arc::new(Calculator), Some(desk))
    }
}

fn opened(scripting: Scripting, attributes: &str) -> Viewer {
    opened_on(scripting, form(attributes))
}

fn opened_on(scripting: Scripting, bytes: Vec<u8>) -> Viewer {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer.handle(Command::Scripts(scripting)).for_each(drop);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer
}

fn value(viewer: &Viewer, short: &str) -> String {
    let Answer::Fields(fields) = viewer.query(Query::Fields) else {
        panic!("a form answers its fields");
    };
    fields
        .into_iter()
        .find(|candidate| candidate.name.qualified == format!("form1[0].{short}[0]"))
        .and_then(|field| field.value)
        .map(|shown| shown.text)
        .unwrap_or_default()
}

fn enter(viewer: &mut Viewer, short: &str, text: &str) -> Vec<Event> {
    let field = format!("form1[0].{short}[0]");
    let mut events: Vec<Event> = viewer
        .handle(Command::Edit(Edit::SetField {
            field: field.clone(),
            value: Entered::Text(text.to_owned()),
        }))
        .collect();
    events.extend(viewer.handle(Command::CommitField { field }));
    events
}

fn notices(events: &[Event]) -> Vec<ScriptQuestion> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::ScriptAsking { question, .. } => Some(question.clone()),
            _ => None,
        })
        .collect()
}

/// The form's calculations run when the form is shown, which is when the merge is complete, and
/// not before.
#[test]
fn the_calculations_run_when_the_form_is_presented() {
    let mut viewer = opened(Scripting::Run(Arc::new(Calculators::default())), "");
    assert_eq!(value(&viewer, "Total"), "", "nothing runs at the open");
    viewer.handle(Command::Presented).for_each(drop);
    assert_eq!(value(&viewer, "Total"), "6");
}

/// At `off` no runner exists, so nothing runs; at *ask* the withheld runner is handed the first
/// script, which is the question, and nothing is calculated until the person says yes.
#[test]
fn off_calculates_nothing_and_ask_waits_for_the_person() {
    let mut viewer = opened(Scripting::Off, "");
    viewer.handle(Command::Presented).for_each(drop);
    assert_eq!(value(&viewer, "Total"), "");

    let mut viewer = opened(Scripting::Ask(Arc::new(Calculators::default())), "");
    let events: Vec<Event> = viewer.handle(Command::Presented).collect();
    assert_eq!(value(&viewer, "Total"), "");
    let asked = events.iter().find_map(|event| match event {
        Event::AskingToRunScripts {
            script, first_line, ..
        } => Some((script.clone(), first_line.clone())),
        _ => None,
    });
    let (script, first_line) = asked.expect("the question is put");
    assert!(script.contains("FormCalc"), "{script}");
    assert_eq!(first_line, "6");
    viewer
        .handle(Command::AnswerScripts {
            document: DOCUMENT,
            proceed: true,
        })
        .for_each(drop);
    assert_eq!(value(&viewer, "Total"), "6");
}

/// A calculation in JavaScript reaches the runner as one (page 106's `contentType`): *ask* puts the
/// question naming its language, and once allowed the runner is handed it as `FormCalc`'s is.
#[test]
fn a_javascript_calculation_is_asked_about_and_run_as_javascript() {
    let bytes = form_in("", "contentType=\"application/x-javascript\"");
    let mut viewer = opened_on(Scripting::Ask(Arc::new(Calculators::default())), bytes);
    let events: Vec<Event> = viewer.handle(Command::Presented).collect();
    assert_eq!(value(&viewer, "Total"), "");
    let asked = events.iter().find_map(|event| match event {
        Event::AskingToRunScripts { script, .. } => Some(script.clone()),
        _ => None,
    });
    let script = asked.expect("the question is put");
    assert!(script.contains("a JavaScript calculation"), "{script}");
    viewer
        .handle(Command::AnswerScripts {
            document: DOCUMENT,
            proceed: true,
        })
        .for_each(drop);
    assert_eq!(value(&viewer, "Total"), "6");
}

/// Page 642: an `error` or a `warning` is presented to the person once, as the notice a window
/// draws for a script's question, and its answer is not mistaken for a script's.
#[test]
fn a_person_s_entry_into_a_calculated_field_puts_a_notice_once() {
    for (attributes, icon, shown_after) in [
        ("override=\"error\"", AlertIcon::Error, "6"),
        ("override=\"warning\"", AlertIcon::Warning, "99"),
    ] {
        let calculators = Arc::new(Calculators::default());
        let mut viewer = opened(
            Scripting::Run(Arc::clone(&calculators) as Arc<dyn ScriptRunners>),
            attributes,
        );
        viewer.handle(Command::Presented).for_each(drop);
        let events = enter(&mut viewer, "Total", "99");
        let put = notices(&events);
        assert_eq!(put.len(), 1, "{attributes}: {put:?}");
        let ScriptQuestion::Alert {
            message,
            icon: shown,
            buttons,
            ..
        } = &put[0]
        else {
            panic!("a notice is an alert");
        };
        assert!(message.contains("form1[0].Total[0]"), "{message}");
        assert_eq!((*shown, *buttons), (icon, AlertButtons::Ok));
        assert_eq!(value(&viewer, "Total"), shown_after, "{attributes}");

        // Put once: a command that follows does not put it again, and its answer reaches no runner.
        let later: Vec<Event> = viewer.handle(Command::Tick { millis: 16 }).collect();
        assert!(notices(&later).is_empty());
        viewer
            .handle(Command::AnswerScript {
                document: DOCUMENT,
                answer: ScriptAnswer::Pressed(AlertButton::Ok),
            })
            .for_each(drop);
        assert!(
            calculators
                .desk
                .answers
                .lock()
                .expect("one test thread")
                .is_empty(),
            "{attributes}: a notice's answer is not a script's"
        );
    }
}

/// Two entries before the first notice is answered put the second after the answer to the first.
#[test]
fn notices_wait_for_each_other() {
    let mut viewer = opened(
        Scripting::Run(Arc::new(Calculators::default())),
        "override=\"error\"",
    );
    viewer.handle(Command::Presented).for_each(drop);
    let first = notices(&enter(&mut viewer, "Total", "98"));
    let second = notices(&enter(&mut viewer, "Total", "99"));
    assert_eq!((first.len(), second.len()), (1, 0), "held behind the first");
    let after: Vec<Event> = viewer
        .handle(Command::AnswerScript {
            document: DOCUMENT,
            answer: ScriptAnswer::Pressed(AlertButton::Ok),
        })
        .collect();
    let next: Vec<Event> = viewer.handle(Command::Tick { millis: 16 }).collect();
    assert_eq!(
        notices(&after).len() + notices(&next).len(),
        1,
        "the second is put once the first is answered"
    );
}

/// Page 642: what a person entered while the runner was withheld — the level `ask` — is an entry
/// when the person allows the scripts, so a `warning` keeps it and an `error` replaces it, and
/// the merge does not write the calculated value over it unasked.
#[test]
fn an_entry_made_while_the_scripts_were_withheld_survives_their_allowing() {
    for (attributes, shown_after) in [
        ("override=\"warning\"", "99"),
        ("override=\"disabled\"", "99"),
        ("override=\"error\"", "6"),
    ] {
        let mut viewer = opened(Scripting::Ask(Arc::new(Calculators::default())), attributes);
        viewer.handle(Command::Presented).for_each(drop);
        enter(&mut viewer, "Total", "99");
        assert_eq!(value(&viewer, "Total"), "99", "{attributes}: typed");
        viewer
            .handle(Command::AnswerScripts {
                document: DOCUMENT,
                proceed: true,
            })
            .for_each(drop);
        assert_eq!(value(&viewer, "Total"), shown_after, "{attributes}");
    }
}
