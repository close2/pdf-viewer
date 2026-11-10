//! A hybrid form's calculations reach a runner at the two moments XFA 3.3 page 379 names, and
//! what `override` asks a host to present (pages 641–642) goes out as the notice a window already
//! draws for a script's question (ADRs 2524, 2525).
//!
//! The runner here is the test's own: it understands a decimal number, which it assigns to the
//! container the script belongs to, and `same`, which answers that container's value as a
//! validation script's verdict. Whether the viewer asks at the right moment, applies
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

use pdf_model::restriction::Operation;
use pdf_model::view::{ScriptEvent, ScriptResult, ScriptRunner};
use pdf_model::xfa::{Form, FormLanguage, Ran, Scalar, Step};
use viewer_core::{
    AlertButton, AlertButtons, AlertIcon, Answer, Command, Committing, DocumentId, Edit, Entered,
    Event, PointerAction, Query, RestrictionLevel, RestrictionPolicy, RestrictionScope,
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
    hybrid(
        &format!(
            "<field name=\"Units\"><ui><textEdit/></ui><value><text/></value></field>\
             <field name=\"Total\"><ui><textEdit/></ui><value><text/></value>\
             <calculate {attributes}><script {script_attributes}>6</script></calculate></field>"
        ),
        "<Units>1</Units><Total/>",
    )
}

/// A hybrid form of one subform `form1` holding `fields`, two of which are `Units` and `Total`,
/// each with a widget in that order, and the data `data` under `form1`.
fn hybrid(fields: &str, data: &str) -> Vec<u8> {
    hybrid_of(fields, data, &["Units", "Total"])
}

/// [`hybrid`], with a widget for each field `named`, in that order down the page.
fn hybrid_of(fields: &str, data: &str, named: &[&str]) -> Vec<u8> {
    let empty: Vec<(&str, &str)> = named.iter().map(|name| (*name, "")).collect();
    hybrid_valued(fields, data, &empty)
}

/// [`hybrid_of`], each widget's field holding the `/V` given beside its name.
fn hybrid_valued(fields: &str, data: &str, named: &[(&str, &str)]) -> Vec<u8> {
    hybrid_stating(fields, data, named, "")
}

/// [`hybrid_valued`], each widget's field stating the entries `entries` as well.
fn hybrid_stating(fields: &str, data: &str, named: &[(&str, &str)], entries: &str) -> Vec<u8> {
    let stream = |text: &str| {
        format!(
            "<< /Length {} >>\nstream\n{text}\nendstream",
            text.len().saturating_add(1)
        )
    };
    let template = format!(
        "<template xmlns=\"http://www.xfa.org/schema/xfa-template/3.3/\"><subform name=\"form1\">\
         {fields}</subform></template>"
    );
    let datasets = format!(
        "<xfa:datasets xmlns:xfa=\"http://www.xfa.org/schema/xfa-data/1.0/\">\
         <xfa:data><form1>{data}</form1></xfa:data></xfa:datasets>"
    );
    let widget = |(name, v): (&str, &str), y: u32| {
        format!(
            "<< /Type /Annot /Subtype /Widget /FT /Tx /Parent 4 0 R /T ({name}[0]) /P 3 0 R \
             /V ({v}) /Rect [20 {y} 220 {}] /F 4 /DA (/Helv 12 Tf 0 g) {entries} >>",
            y.saturating_add(20)
        )
    };
    let kids: Vec<String> = (0..named.len())
        .map(|at| format!("{} 0 R", at.saturating_add(7)))
        .collect();
    let kids = kids.join(" ");
    let mut bodies = vec![
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] \
         /XFA [(template) 5 0 R (datasets) 6 0 R] >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [{kids}] >>"),
        format!("<< /T (form1[0]) /Kids [{kids}] >>"),
        stream(&template),
        stream(&datasets),
    ];
    let mut y = 300_u32;
    for &field in named {
        bodies.push(widget(field, y));
        y = y.saturating_sub(40);
    }
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
        if script.trim() == "same" {
            return Ran {
                steps: 1,
                outcome: Ok(form.value(node)),
            };
        }
        // `cancel` is FormCalc's `$event.cancelAction = 1`: it sets page 402's flag and nothing
        // else.
        if script.trim() == "cancel" {
            let set = Step::path("$event.cancelAction")
                .and_then(|path| form.resolve(node, &path))
                .and_then(|nodes| {
                    let flag = *nodes.first().ok_or("no action is under way")?;
                    form.set(flag, Scalar::Number(1.0))
                });
            return Ran {
                steps: 1,
                outcome: set.map(|()| Scalar::Number(1.0)),
            };
        }
        // A script that is a path into `$event` sets its field to what the path reads.
        if script.trim().starts_with("$event.") {
            let read = Step::path(script.trim())
                .and_then(|path| form.resolve(node, &path))
                .and_then(|nodes| {
                    let value = nodes
                        .first()
                        .map(|&at| form.value(at))
                        .ok_or_else(|| format!("{script} resolves to nothing"))?;
                    form.set(node, value.clone())?;
                    Ok(value)
                });
            return Ran {
                steps: 1,
                outcome: read,
            };
        }
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
    events.extend(viewer.handle(Command::CommitField {
        field,
        by: Committing::Unstated,
    }));
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
    for (attributes, icon, offered, shown_after) in [
        (
            "override=\"error\"",
            AlertIcon::Error,
            AlertButtons::Ok,
            "6",
        ),
        (
            "override=\"warning\"",
            AlertIcon::Warning,
            AlertButtons::YesNo,
            "99",
        ),
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
        assert_eq!((*shown, *buttons), (icon, offered));
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

/// XFA 3.3 page 642: a `warning` gives the person two choices. "Yes", to override, keeps the
/// value entered (`99`); "No", to dismiss, leaves the calculated value (`6`) undisturbed, and so
/// does a dialogue closed without a press, whose answer is "No" (ADR 2697).
#[test]
fn a_warning_s_no_restores_the_calculated_value_and_its_yes_keeps_the_entry() {
    for (answer, kept) in [
        (ScriptAnswer::Pressed(AlertButton::Yes), "99"),
        (ScriptAnswer::Pressed(AlertButton::No), "6"),
        (ScriptAnswer::Unanswerable, "6"),
    ] {
        let mut viewer = opened(
            Scripting::Run(Arc::new(Calculators::default())),
            "override=\"warning\"",
        );
        viewer.handle(Command::Presented).for_each(drop);
        assert_eq!(notices(&enter(&mut viewer, "Total", "99")).len(), 1);
        assert_eq!(value(&viewer, "Total"), "99");
        viewer
            .handle(Command::AnswerScript {
                document: DOCUMENT,
                answer: answer.clone(),
            })
            .for_each(drop);
        assert_eq!(value(&viewer, "Total"), kept, "{answer:?}");
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

/// XFA 3.3 pages 390–391: a tab that moves the keyboard focus runs the `exit` event of the field
/// it leaves and the `enter` event of the field it reaches, through the runner the reader's level
/// supplied (ADR 2566). The test's runner assigns a script's number to the script's own field.
#[test]
fn a_tab_runs_the_exit_event_of_the_field_left_and_the_enter_event_of_the_field_reached() {
    let fields = "<field name=\"Units\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"enter\"><script>4</script></event></field>\
                  <field name=\"Total\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"exit\"><script>9</script></event></field>";
    let data = "<Units>1</Units><Total/>";
    for (scripting, units, total) in [
        (Scripting::Run(Arc::new(Calculators::default())), "4", "9"),
        (Scripting::Off, "", ""),
    ] {
        let mut viewer = opened_on(scripting, hybrid(fields, data));
        viewer.handle(Command::Presented).for_each(drop);
        let tab = |viewer: &mut Viewer| {
            viewer
                .handle(Command::Focused(viewer_core::FocusMove::Next))
                .for_each(drop);
        };
        tab(&mut viewer);
        assert_eq!(value(&viewer, "Units"), units, "the focus reached Units");
        assert_eq!(value(&viewer, "Total"), "");
        tab(&mut viewer);
        assert_eq!(value(&viewer, "Total"), "", "Total is entered, not left");
        tab(&mut viewer);
        assert_eq!(value(&viewer, "Total"), total, "the focus left Total");
    }
}

/// XFA 3.3 pages 398 and 402: a field's `exit` reads `$event.commitKey`, how its value was
/// committed as the focus left it — 3 where a tab moved the focus to a new field, 1 where a press
/// of the mouse outside the field took it (ADR 2726). The runner sets the field to what its script
/// reads.
#[test]
fn a_fields_exit_reads_whether_a_tab_or_a_click_committed_it() {
    let fields = "<field name=\"Units\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"exit\"><script>$event.commitKey</script></event></field>\
                  <field name=\"Total\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"exit\"><script>$event.commitKey</script></event></field>";
    let mut viewer = opened_on(
        Scripting::Run(Arc::new(Calculators::default())),
        hybrid(fields, "<Units/><Total/>"),
    );
    viewer.handle(Command::Presented).for_each(drop);
    for _ in 0..2 {
        viewer
            .handle(Command::Focused(viewer_core::FocusMove::Next))
            .for_each(drop);
    }
    assert_eq!(value(&viewer, "Units"), "3", "a tab left Units");
    assert_eq!(value(&viewer, "Total"), "", "Total holds the focus");
    // `hybrid` puts no widget at (320, 100).
    press_and_release(&mut viewer, (320.0, 100.0), (320.0, 100.0));
    assert_eq!(
        value(&viewer, "Total"),
        "1",
        "a click outside Total left it"
    );
}

/// XFA 3.3 pages 398, 402 and 403: a field's `click` reads `$event.shift` and `$event.modifier`
/// as the keys a host said it held, `Command::Keys` before the pointer, 1 or 0; and a field's
/// `change` reads `$event.keyDown`, whether an arrow key made the selection, as the host said it
/// at the edit rather than at the commit, since the arrow made the edit (ADR 2732).
#[test]
fn a_fields_events_read_the_keys_a_host_said_it_held() {
    let fields = "<field name=\"Units\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"click\"><script>$event.shift</script></event></field>\
                  <field name=\"Total\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"click\"><script>$event.modifier</script></event>\
                  <event activity=\"change\" ref=\"Units\"><script>$event.keyDown</script>\
                  </event></field>";
    let mut viewer = opened_on(
        Scripting::Run(Arc::new(Calculators::default())),
        hybrid(fields, "<Units/><Total/>"),
    );
    viewer.handle(Command::Presented).for_each(drop);
    let keys = |shift, modifier, arrows| {
        Command::Keys(pdf_model::view::Keys {
            shift,
            modifier,
            arrows,
        })
    };
    // `hybrid` puts `Units` at [20 300 220 320] and `Total` at [20 260 220 280].
    let (units, total) = ((120.0, 310.0), (120.0, 270.0));
    viewer.handle(keys(true, false, false)).for_each(drop);
    press_and_release(&mut viewer, units, units);
    assert_eq!(value(&viewer, "Units"), "1", "Shift held at the click");
    press_and_release(&mut viewer, total, total);
    assert_eq!(value(&viewer, "Total"), "0", "Control not held");
    viewer.handle(keys(false, true, false)).for_each(drop);
    press_and_release(&mut viewer, total, total);
    assert_eq!(value(&viewer, "Total"), "1", "Control held at the click");
    let field = "form1[0].Units[0]".to_owned();
    for (at_the_edit, expected) in [(true, "1"), (false, "0")] {
        viewer
            .handle(keys(false, false, at_the_edit))
            .for_each(drop);
        viewer
            .handle(Command::Edit(Edit::SetField {
                field: field.clone(),
                value: Entered::Text("7".to_owned()),
            }))
            .for_each(drop);
        viewer
            .handle(keys(false, false, !at_the_edit))
            .for_each(drop);
        viewer
            .handle(Command::CommitField {
                field: field.clone(),
                by: Committing::Unstated,
            })
            .for_each(drop);
        assert_eq!(
            value(&viewer, "Total"),
            expected,
            "an arrow key at the edit: {at_the_edit}"
        );
    }
}

/// The point of page one at `(x, y)` in its own space, in the window's pixels.
fn on_page(viewer: &Viewer, (x, y): (f32, f32)) -> (f32, f32) {
    /// The height of `hybrid`'s `/MediaBox`.
    const PAGE_HEIGHT: f32 = 400.0;
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the page on the screen has a geometry");
    };
    (
        geometry.origin.0 + x * geometry.scale,
        geometry.origin.1 + (PAGE_HEIGHT - y) * geometry.scale,
    )
}

/// A press at `from` and a release at `to`, both in page one's space.
fn press_and_release(viewer: &mut Viewer, from: (f32, f32), to: (f32, f32)) {
    let (from, to) = (on_page(viewer, from), on_page(viewer, to));
    for (at, action) in [
        (from, PointerAction::Moved),
        (from, PointerAction::Pressed),
        (to, PointerAction::Dragged),
        (to, PointerAction::Released),
    ] {
        viewer
            .handle(Command::Pointer { at, action })
            .for_each(drop);
    }
}

/// XFA 3.3 page 391: a `click` fires "when a mouse click occurs within the region", which a window
/// reports as a press and a release on the field's widget, Table 197's `/U` (ADR 2584). It runs
/// through the runner the reader's level supplied, and at `off` it runs nothing. A press outside
/// every widget, and a press on the widget released outside it, are no click within the field.
#[test]
fn a_press_and_release_on_a_widget_runs_the_click_event_of_its_field() {
    let fields = "<field name=\"Units\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"click\"><script>4</script></event></field>\
                  <field name=\"Total\"><ui><textEdit/></ui><value><text/></value></field>";
    let data = "<Units/><Total/>";
    // `hybrid` puts `Units` at [20 300 220 320] and `Total` at [20 260 220 280].
    let (units, total, nowhere) = ((120.0, 310.0), (120.0, 270.0), (320.0, 100.0));
    for (scripting, clicked) in [
        (Scripting::Run(Arc::new(Calculators::default())), "4"),
        (Scripting::Off, ""),
    ] {
        let mut viewer = opened_on(scripting, hybrid(fields, data));
        viewer.handle(Command::Presented).for_each(drop);
        press_and_release(&mut viewer, nowhere, nowhere);
        assert_eq!(
            value(&viewer, "Units"),
            "",
            "a press on no widget clicks nothing"
        );
        press_and_release(&mut viewer, units, nowhere);
        assert_eq!(value(&viewer, "Units"), "", "released outside the field");
        press_and_release(&mut viewer, total, units);
        assert_eq!(value(&viewer, "Units"), "", "pressed outside the field");
        press_and_release(&mut viewer, units, units);
        assert_eq!(value(&viewer, "Units"), clicked, "a click within Units");
        assert_eq!(value(&viewer, "Total"), "");
    }
}

/// [`press_and_release`], answering every event the four commands caused.
fn clicked(viewer: &mut Viewer, at: (f32, f32)) -> Vec<Event> {
    let at = on_page(viewer, at);
    [
        PointerAction::Moved,
        PointerAction::Pressed,
        PointerAction::Released,
    ]
    .into_iter()
    .flat_map(|action| {
        viewer
            .handle(Command::Pointer { at, action })
            .collect::<Vec<_>>()
    })
    .collect()
}

/// XFA 3.3 pages 450–453 and 858–859: a click on a field whose `click` event holds a `submit` hands
/// the host the submission the `submit`'s `format` names — `pdf`, Table 240 bit 9's whole document
/// — to its `target`, by POST, after the form's `preSubmit` events have run (page 446's step 2), so
/// that what they set is in it (ADR 2738). It is a submit-form action's `Event::Submit`, under the
/// host's submit level. Page 703 gives the event the `submit` in place of a `script`, so it is no
/// script, and at `off`, where no script runs (ADR 1616), the click still hands the host the same
/// submission, while the `preSubmit` script, which is one, leaves `Total` empty (ADR 2767).
#[test]
fn a_click_on_a_field_whose_event_submits_hands_the_host_the_submission_it_names() {
    let fields = "<field name=\"Units\"><ui><button/></ui><value><text/></value>\
                  <event activity=\"click\"><submit format=\"pdf\" \
                  target=\"https://example.invalid/form\"/></event></field>\
                  <field name=\"Total\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"preSubmit\" ref=\"$form\"><script>7</script></event></field>";
    let data = "<Units/><Total/>";
    let units = (120.0, 310.0);
    for (scripting, pre_submit) in [
        (Scripting::Run(Arc::new(Calculators::default())), "7"),
        (Scripting::Off, ""),
    ] {
        let mut viewer = opened_on(scripting, hybrid(fields, data));
        viewer.handle(Command::Presented).for_each(drop);
        let events = clicked(&mut viewer, units);
        let submissions: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                Event::Submit { submission, .. } => Some(submission),
                _ => None,
            })
            .collect();
        let [submission] = submissions.as_slice() else {
            panic!("one submission: {events:?}");
        };
        assert_eq!(submission.url, "https://example.invalid/form");
        assert_eq!(submission.format, pdf_model::submission::Format::Pdf);
        assert_eq!(submission.method, pdf_model::submission::Method::Post);
        assert!(submission.body.starts_with(b"%PDF-"));
        assert_eq!(
            value(&viewer, "Total"),
            pre_submit,
            "the preSubmit script ran first where scripts run, and not at off"
        );
    }
}

/// XFA 3.3 page 382: "\[w]hen an XFA-processing application initializes a form, it executes all
/// validation tests specified for the form", and the format test is a picture clause, no script, so
/// a form whose `Code` holds `12` against the picture `AAA-999` is refused when it is presented, at
/// `off` as at a level that runs scripts, with page 385's `error` message (ADR 2767). Page 396
/// applies the test to no field holding empty content, so the empty `Note` puts nothing.
#[test]
fn the_merge_puts_every_field_to_its_format_test_at_every_level() {
    let bytes = || {
        hybrid_valued(
            "<field name=\"Code\"><ui><textEdit/></ui><value><text/></value>\
             <validate formatTest=\"error\"><picture>AAA-999</picture></validate></field>\
             <field name=\"Note\"><ui><textEdit/></ui><value><text/></value>\
             <validate formatTest=\"error\"><picture>AAA-999</picture></validate></field>",
            "<Code>12</Code><Note/>",
            &[("Code", "12"), ("Note", "")],
        )
    };
    for (level, scripting) in [
        ("off", Scripting::Off),
        ("run", Scripting::Run(Arc::new(Calculators::default()))),
    ] {
        let mut viewer = opened_on(scripting, bytes());
        let events: Vec<Event> = viewer.handle(Command::Presented).collect();
        let put = notices(&events);
        assert_eq!(put.len(), 1, "{level}: {events:?}");
        let ScriptQuestion::Alert { message, icon, .. } = &put[0] else {
            panic!("a validation message is an alert");
        };
        assert_eq!(
            message,
            "form1[0].Code[0]: the form does not accept this value, which does not conform to \
             its input mask."
        );
        assert_eq!(*icon, AlertIcon::Error);
        assert_eq!(value(&viewer, "Code"), "12", "the value stands (page 383)");
    }
}

/// XFA 3.3 page 703: an `event` holds a `script` or a `submit`, and at `off` only the second is
/// carried out. A field's `mouseUp` that submits in page 858's `urlencoded` packaging hands the host
/// its submission where no script runs, and the click script beside it changes nothing (ADR 2767).
#[test]
fn at_off_an_event_s_submit_is_carried_out_and_its_neighbour_s_script_is_not() {
    let fields = "<field name=\"Units\"><ui><button/></ui><value><text/></value>\
                  <event activity=\"mouseUp\"><submit format=\"urlencoded\" \
                  target=\"https://example.invalid/form\"/></event>\
                  <event activity=\"click\"><script>Total = 3</script></event></field>\
                  <field name=\"Total\"><ui><textEdit/></ui><value><text/></value></field>";
    let mut viewer = opened_on(Scripting::Off, hybrid(fields, "<Units/><Total/>"));
    viewer.handle(Command::Presented).for_each(drop);
    let events = clicked(&mut viewer, (120.0, 310.0));
    let submissions: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            Event::Submit { submission, .. } => Some(submission),
            _ => None,
        })
        .collect();
    let [submission] = submissions.as_slice() else {
        panic!("one submission: {events:?}");
    };
    assert_eq!(submission.url, "https://example.invalid/form");
    assert_eq!(submission.format, pdf_model::submission::Format::HtmlForm);
    assert_eq!(value(&viewer, "Total"), "", "the click script did not run");
}

/// The one submission a click on `Units` hands the host, `Units`' `click` event submitting with the
/// attributes given, after `Total` has been entered as `5`; and every note the click reported.
fn packaged_submission(attributes: &str) -> (pdf_model::submission::Submission, Vec<String>) {
    let fields = format!(
        "<field name=\"Units\"><ui><button/></ui><value><text/></value>\
         <event activity=\"click\"><submit {attributes} \
         target=\"https://example.invalid/data\"/></event></field>\
         <field name=\"Total\"><ui><textEdit/></ui><value><text/></value></field>"
    );
    let mut viewer = opened_on(Scripting::Off, hybrid(&fields, "<Units/><Total/>"));
    viewer.handle(Command::Presented).for_each(drop);
    enter(&mut viewer, "Total", "5");
    let events = clicked(&mut viewer, (120.0, 310.0));
    let mut notes = Vec::new();
    let mut submissions = Vec::new();
    for event in events {
        match event {
            Event::Submit { submission, .. } => submissions.push(*submission),
            Event::Reported { notes: said, .. } => notes.extend(said),
            _ => {}
        }
    }
    let [submission] = <[_; 1]>::try_from(submissions).unwrap_or_else(|all| {
        panic!("one submission: {all:?}, {notes:?}");
    });
    (submission, notes)
}

/// XFA 3.3 page 859: `xdp` packages "in XDP format, as described in XDP Specification", and page
/// 858 underlines it, so a `submit` stating no `format` asks for it; page 861 lets `xdpContent`
/// choose the packets, and an omitted one is read as its first value, `datasets pdf xfdf` (ADR
/// 2771). The body is page 1041's `xdp:xdp` root, declared as page 1035 asks, holding the
/// `datasets` packet with the value the person entered in it — what a save writes, §K.2's
/// consistency — and no `template`; page 858's `embedPDF` is `0` by its underline and this process
/// holds no path for the URI, so the `pdf` packet is empty, and a document holding no markup
/// annotation has an `xfdf` packet with nothing in it (the XFDF specification's content model makes
/// every child of `xfdf` optional). Page 1041 registers the media type, and page 858 the POST. At
/// `off`, since a `submit` is no script (page 703).
#[test]
fn an_xdp_submit_hands_the_host_the_form_s_packets_with_what_was_entered() {
    let (submission, _) = packaged_submission("");
    assert_eq!(submission.url, "https://example.invalid/data");
    assert_eq!(submission.format, pdf_model::submission::Format::Xdp);
    assert_eq!(submission.media_type, "application/vnd.adobe.xdp+xml");
    assert_eq!(submission.method, pdf_model::submission::Method::Post);
    let body = String::from_utf8(submission.body).expect("UTF-8");
    assert!(
        body.starts_with(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<xdp:xdp xmlns:xdp=\"http://ns.adobe.com/xdp/\">\n\
             <xfa:datasets xmlns:xfa=\"http://www.xfa.org/schema/xfa-data/1.0/\">"
        ),
        "{body}"
    );
    assert!(body.contains("<Total>5</Total>"), "{body}");
    assert!(!body.contains("<template"), "{body}");
    assert!(
        body.ends_with(
            "<pdf xmlns=\"http://ns.adobe.com/xdp/pdf/\"/>\n\
             <xfdf xmlns=\"http://ns.adobe.com/xfdf/\" xml:space=\"preserve\"/>\n</xdp:xdp>\n"
        ),
        "{body}"
    );
}

/// XFA 3.3 pages 858, 861 and 1039: with `embedPDF="1"` "\[a] copy of the associated PDF document is
/// embedded in the submitted XDP", as page 1039's `chunk` of base64 (RFC 2045), whose first four
/// characters `JVBE` are `%PD`; `xdpContent` names `template` and `config` beside it, so the
/// template packet goes, and the form's resource holds no `config` packet, which is said.
#[test]
fn an_xdp_submit_embeds_the_document_and_sends_the_packets_its_content_names() {
    let (submission, notes) =
        packaged_submission("format=\"xdp\" embedPDF=\"1\" xdpContent=\"template pdf config\"");
    let body = String::from_utf8(submission.body).expect("UTF-8");
    assert!(body.contains("<template xmlns=\"http://www.xfa.org/schema/xfa-template/3.3/\">"));
    assert!(!body.contains("<xfa:datasets"), "{body}");
    assert!(
        body.contains("<pdf xmlns=\"http://ns.adobe.com/xdp/pdf/\"><document><chunk>\nJVBE"),
        "{body}"
    );
    assert!(!body.contains("<xfdf"), "{body}");
    assert!(
        notes
            .iter()
            .any(|note| note.contains("xdpContent names config")),
        "{notes:?}"
    );
}

/// XFA 3.3 page 859: `xml` packages the data "in XML format … according to the same rules used for
/// data unloading", and page 149's unloader writes "an XML data document which reflects the
/// contents of the XML Data DOM": the one element `xfa:data` holds, with the person's entry in it,
/// under an XML declaration, as `text/xml` (the type page 1038 gives the `datasets` packet).
#[test]
fn an_xml_submit_hands_the_host_the_form_s_data_document() {
    let (submission, _) = packaged_submission("format=\"xml\"");
    assert_eq!(submission.format, pdf_model::submission::Format::Xml);
    assert_eq!(submission.media_type, "text/xml");
    assert_eq!(
        String::from_utf8(submission.body).expect("UTF-8"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <form1 xmlns:xdp=\"http://ns.adobe.com/xdp/\" xmlns:xfa=\"http://www.xfa.org/schema/xfa-data/1.0/\">\
         <Units/><Total>5</Total></form1>\n"
    );
}

/// XFA 3.3 page 392: a `mouseDown` fires when the button is pressed within the field's region and a
/// `mouseUp` when it is released within it, wherever it was pressed (ADR 2627). A press on `Units`
/// dragged to `Total` and released there runs the one's `mouseDown` and the other's `mouseUp`,
/// which no `/U` names; at `off` neither runs.
#[test]
fn a_press_and_a_release_run_the_mouse_events_of_the_fields_they_land_on() {
    let fields = "<field name=\"Units\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"mouseDown\"><script>1</script></event>\
                  <event activity=\"mouseUp\"><script>5</script></event></field>\
                  <field name=\"Total\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"mouseUp\"><script>2</script></event></field>";
    let data = "<Units/><Total/>";
    // `hybrid` puts `Units` at [20 300 220 320] and `Total` at [20 260 220 280].
    let (units, total) = ((120.0, 310.0), (120.0, 270.0));
    for (scripting, pressed, released) in [
        (Scripting::Run(Arc::new(Calculators::default())), "1", "2"),
        (Scripting::Off, "", ""),
    ] {
        let mut viewer = opened_on(scripting, hybrid(fields, data));
        viewer.handle(Command::Presented).for_each(drop);
        press_and_release(&mut viewer, units, total);
        assert_eq!(value(&viewer, "Units"), pressed, "pressed within Units");
        assert_eq!(value(&viewer, "Total"), released, "released within Total");
    }
}

/// XFA 3.3 page 391: a field's `mouseEnter` fires as the pointer moves into its region and its
/// `mouseExit` as it moves out, with a button up or down (ADR 2647) — unlike Table 197's `/E`,
/// which §12.6.3's first constraint holds back while the button is down. A pointer dragged onto
/// `Units` still enters it, and a move from it to `Total` exits it; at `off` neither runs.
#[test]
fn the_pointer_crossing_a_widget_runs_the_mouse_enter_and_mouse_exit_of_its_field() {
    let fields = "<field name=\"Units\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"mouseEnter\"><script>7</script></event>\
                  <event activity=\"mouseExit\"><script>8</script></event></field>\
                  <field name=\"Total\"><ui><textEdit/></ui><value><text/></value></field>";
    let data = "<Units/><Total/>";
    // `hybrid` puts `Units` at [20 300 220 320] and `Total` at [20 260 220 280].
    let (units, total, nowhere) = ((120.0, 310.0), (120.0, 270.0), (320.0, 100.0));
    for (scripting, entered, exited) in [
        (Scripting::Run(Arc::new(Calculators::default())), "7", "8"),
        (Scripting::Off, "", ""),
    ] {
        let mut viewer = opened_on(scripting, hybrid(fields, data));
        viewer.handle(Command::Presented).for_each(drop);
        let point = |viewer: &mut Viewer, to: (f32, f32), action: PointerAction| {
            let at = on_page(viewer, to);
            viewer
                .handle(Command::Pointer { at, action })
                .for_each(drop);
        };
        point(&mut viewer, units, PointerAction::Moved);
        assert_eq!(value(&viewer, "Units"), entered, "moved into Units");
        point(&mut viewer, total, PointerAction::Moved);
        assert_eq!(value(&viewer, "Units"), exited, "moved out of Units");
        assert_eq!(
            value(&viewer, "Total"),
            "",
            "Total states no crossing event"
        );
        point(&mut viewer, nowhere, PointerAction::Pressed);
        point(&mut viewer, units, PointerAction::Dragged);
        assert_eq!(value(&viewer, "Units"), entered, "dragged into Units");
        point(&mut viewer, nowhere, PointerAction::Dragged);
        assert_eq!(value(&viewer, "Units"), exited, "dragged out of Units");
    }
}

/// XFA 3.3 page 388: the Form DOM's `ready` runs once the form is shown, which is the open
/// sequence's end, and the Layout DOM's `ready` "when the layout is complete but rendering has
/// not yet begun", which for a static form's layout, the pages the file holds, is the same end
/// (ADR 2783), through the runner the reader's level supplied, and at `off` nothing runs; an
/// event bound to `$data`, which page 388 says the event does not apply to, is named in the
/// document's report as not run (ADR 2606).
#[test]
fn the_ready_event_runs_when_the_form_is_presented_and_an_unfollowed_one_is_reported() {
    let fields = "<field name=\"Units\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"ready\" ref=\"$form\"><script>4</script></event></field>\
                  <field name=\"Total\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"ready\" ref=\"$layout\"><script>9</script></event></field>\
                  <field name=\"Spare\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"ready\" ref=\"$data\"><script>5</script></event></field>";
    let data = "<Units>1</Units><Total/><Spare/>";
    for (scripting, units, total, reported) in [
        (
            Scripting::Run(Arc::new(Calculators::default())),
            "4",
            "9",
            true,
        ),
        (Scripting::Off, "", "", false),
    ] {
        let mut viewer = opened_on(
            scripting,
            hybrid_of(fields, data, &["Units", "Total", "Spare"]),
        );
        assert_eq!(value(&viewer, "Units"), "", "nothing runs at the open");
        let mut events: Vec<Event> = viewer.handle(Command::Presented).collect();
        events.extend(viewer.handle(Command::Presented));
        assert_eq!(value(&viewer, "Units"), units);
        assert_eq!(value(&viewer, "Total"), total, "the Layout DOM's ready");
        assert_eq!(value(&viewer, "Spare"), "", "a $data ready is not run");
        let said: Vec<String> = events
            .iter()
            .filter_map(|event| match event {
                Event::Reported { notes, .. } => Some(notes.join("\n")),
                _ => None,
            })
            .collect();
        let said = said.join("\n");
        assert_eq!(
            said.contains("its ready event is bound to $data"),
            reported,
            "{said}"
        );
        assert!(!said.contains("$layout"), "{said}");
    }
}

/// XFA 3.3 page 391: a field's `full` event "fires when the user has entered the maximum allowed
/// amount of content into the field and tries to enter more content", and page 406's second rule
/// fires it as the last allowed character is typed. The field's Table 232 `/MaxLen` of 3 is the
/// form's maximum, so typing `12` runs no `full`, and typing `123` and `1234` each run it, whose
/// script writes `7`; at `off` no script runs (ADR 2783).
#[test]
fn a_field_typed_to_its_max_len_runs_its_full_event() {
    let fields = "<field name=\"Units\"><ui><textEdit/></ui><value><text maxChars=\"3\"/></value>\
                  <event activity=\"full\"><script>7</script></event></field>\
                  <field name=\"Total\"><ui><textEdit/></ui><value><text/></value></field>";
    let data = "<Units/><Total/>";
    for (scripting, typed, expected) in [
        (Scripting::Run(Arc::new(Calculators::default())), "12", "12"),
        (Scripting::Run(Arc::new(Calculators::default())), "123", "7"),
        (
            Scripting::Run(Arc::new(Calculators::default())),
            "1234",
            "7",
        ),
        (Scripting::Off, "123", "123"),
    ] {
        let bytes = hybrid_stating(fields, data, &[("Units", ""), ("Total", "")], "/MaxLen 3");
        let mut viewer = opened_on(scripting, bytes);
        viewer.handle(Command::Presented).for_each(drop);
        enter(&mut viewer, "Units", typed);
        assert_eq!(value(&viewer, "Units"), expected, "typed {typed}");
    }
}

/// XFA 3.3 page 383: a container whose script test fails is presented to the person in the
/// designer's message for the test, as the notice a window draws for a script's question, at
/// page 881's default level, `error`. Page 396 leaves the empty field the form is presented with
/// to the null test, so the merge puts nothing; the entry, which the script refuses, puts one
/// notice, and the entry stands (page 383 recommends that a person not be held in the field).
/// The runner sets the field to the number the script is, which a validation may not keep
/// (pages 372 and 385).
#[test]
fn an_entry_its_validation_script_refuses_puts_the_designers_message() {
    let bytes = hybrid(
        "<field name=\"Units\"><ui><textEdit/></ui><value><text/></value>\
         <validate><message><text name=\"scriptTest\">Units must be positive.</text></message>\
         <script>0</script></validate></field>\
         <field name=\"Total\"><ui><textEdit/></ui><value><text/></value></field>",
        "<Units/><Total/>",
    );
    let mut viewer = opened_on(Scripting::Run(Arc::new(Calculators::default())), bytes);
    let presented: Vec<Event> = viewer.handle(Command::Presented).collect();
    assert!(notices(&presented).is_empty());
    let put = notices(&enter(&mut viewer, "Units", "5"));
    assert_eq!(put.len(), 1, "{put:?}");
    let ScriptQuestion::Alert {
        message,
        icon,
        buttons,
        title,
    } = &put[0]
    else {
        panic!("a validation message is an alert");
    };
    assert_eq!(message, "form1[0].Units[0]: Units must be positive.");
    assert_eq!((*icon, *buttons), (AlertIcon::Error, AlertButtons::Ok));
    assert_eq!(title.as_deref(), Some("Form validation"));
    assert_eq!(value(&viewer, "Units"), "5");
}

/// XFA 3.3 page 396: the format test is re-evaluated when content changes, and page 382 has the
/// `validate` picture format the canonical value; page 385: at `error`, the message says the value
/// is required to conform to the input mask. The format test is no script, so it runs at `off` as
/// at a level that runs scripts, and an entry that conforms puts nothing (ADR 2727).
#[test]
fn an_entry_its_format_test_refuses_puts_a_message_at_every_level() {
    let bytes = || {
        hybrid_of(
            "<field name=\"Code\"><ui><textEdit/></ui><value><text/></value>\
             <validate formatTest=\"error\"><picture>AAA-999</picture></validate></field>",
            "<Code/>",
            &["Code"],
        )
    };
    for (level, scripting) in [
        ("off", Scripting::Off),
        ("run", Scripting::Run(Arc::new(Calculators::default()))),
    ] {
        let mut viewer = opened_on(scripting, bytes());
        viewer.handle(Command::Presented).for_each(drop);
        assert!(notices(&enter(&mut viewer, "Code", "ABC123")).is_empty());
        let events = enter(&mut viewer, "Code", "12");
        let put = notices(&events);
        assert_eq!(put.len(), 1, "{level}: {events:?}");
        let ScriptQuestion::Alert {
            message,
            icon,
            buttons,
            ..
        } = &put[0]
        else {
            panic!("a validation message is an alert");
        };
        assert_eq!(
            message,
            "form1[0].Code[0]: the form does not accept this value, which does not conform to \
             its input mask."
        );
        assert_eq!((*icon, *buttons), (AlertIcon::Error, AlertButtons::Ok));
        assert_eq!(value(&viewer, "Code"), "12", "the entry stands (page 383)");
    }
}

/// XFA 3.3 page 385: a script test whose `scriptTest` is `warning` puts a message with two
/// choices, to dismiss — the person "wishes to return to the form, so that s/he may satisfy this
/// constraint" — and to override, contravening it. The notice asks them as one question: "Yes"
/// overrides and the entry stands where it is; "No", or a dialogue closed without a press, returns
/// the keyboard focus to the field's widget (object 7), and the entry still stands (ADR 2698).
#[test]
fn a_script_test_s_warning_asks_and_its_no_returns_the_person_to_the_field() {
    for (answer, returned) in [
        (ScriptAnswer::Pressed(AlertButton::Yes), false),
        (ScriptAnswer::Pressed(AlertButton::No), true),
        (ScriptAnswer::Unanswerable, true),
    ] {
        let bytes = hybrid(
            "<field name=\"Units\"><ui><textEdit/></ui><value><text/></value>\
             <validate scriptTest=\"warning\"><script>0</script></validate></field>\
             <field name=\"Total\"><ui><textEdit/></ui><value><text/></value></field>",
            "<Units/><Total/>",
        );
        let mut viewer = opened_on(Scripting::Run(Arc::new(Calculators::default())), bytes);
        viewer.handle(Command::Presented).for_each(drop);
        let put = notices(&enter(&mut viewer, "Units", "5"));
        assert_eq!(put.len(), 1, "{put:?}");
        let ScriptQuestion::Alert {
            message,
            icon,
            buttons,
            ..
        } = &put[0]
        else {
            panic!("a validation message is an alert");
        };
        assert!(
            message.ends_with("Keep the value you entered anyway?"),
            "{message}"
        );
        assert_eq!((*icon, *buttons), (AlertIcon::Warning, AlertButtons::YesNo));
        viewer
            .handle(Command::AnswerScript {
                document: DOCUMENT,
                answer: answer.clone(),
            })
            .for_each(drop);
        let focused = match viewer.query(Query::Focus) {
            Answer::Focus { object, .. } => Some(object.number),
            _ => None,
        };
        assert_eq!(focused == Some(7), returned, "{answer:?}: {focused:?}");
        assert_eq!(value(&viewer, "Units"), "5", "{answer:?}");
    }
}

/// The file a save writes, from the events the save put.
fn saved(events: &[Event]) -> Vec<u8> {
    events
        .iter()
        .find_map(|event| match event {
            Event::Saved { bytes, .. } => Some(bytes.clone()),
            _ => None,
        })
        .expect("a hybrid form saves")
}

/// XFA 3.3 page 388: `preSave` fires "just before the form data is written out in PDF", and
/// `postSave` "just after the form has been written out". So what a `preSave` script writes is in
/// the file the save writes, and what a `postSave` script writes is on the screen and not in that
/// file; at `off` neither runs (ADR 1616).
#[test]
fn a_save_writes_what_its_pre_save_event_set_and_not_what_its_post_save_event_set() {
    let fields = "<field name=\"Units\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"preSave\" ref=\"$host\"><script>4</script></event></field>\
                  <field name=\"Total\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"postSave\" ref=\"$host\"><script>9</script></event></field>";
    let data = "<Units/><Total/>";
    for (scripting, units, total) in [
        (Scripting::Run(Arc::new(Calculators::default())), "4", "9"),
        (Scripting::Off, "", ""),
    ] {
        let mut viewer = opened_on(scripting, hybrid(fields, data));
        viewer.handle(Command::Presented).for_each(drop);
        assert_eq!(value(&viewer, "Units"), "", "nothing runs before the save");
        let events: Vec<Event> = viewer.handle(Command::Save).collect();
        assert_eq!(value(&viewer, "Units"), units);
        assert_eq!(value(&viewer, "Total"), total);
        let reopened = opened_on(Scripting::Off, saved(&events));
        assert_eq!(
            value(&reopened, "Units"),
            units,
            "the preSave value is written"
        );
        assert_eq!(value(&reopened, "Total"), "", "the postSave value is not");
    }
}

/// Opens a hybrid form whose `preSave` script declines the save, under one level for the reader's
/// `save`, and saves it.
fn saved_under(level: RestrictionLevel) -> (Viewer, Vec<Event>) {
    let fields = "<field name=\"Units\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"preSave\" ref=\"$host\"><script>cancel</script></event>\
                  </field>\
                  <field name=\"Total\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"postSave\" ref=\"$host\"><script>9</script></event></field>";
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Restrict(RestrictionScope::Window(
            RestrictionPolicy::default().with(Operation::Save, level),
        )))
        .for_each(drop);
    viewer
        .handle(Command::Scripts(Scripting::Run(Arc::new(
            Calculators::default(),
        ))))
        .for_each(drop);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: hybrid(fields, "<Units/><Total/>").into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer.handle(Command::Presented).for_each(drop);
    let events = viewer.handle(Command::Save).collect();
    (viewer, events)
}

/// Whether the events hold a written file.
fn wrote(events: &[Event]) -> bool {
    events
        .iter()
        .any(|event| matches!(event, Event::Saved { .. }))
}

/// XFA 3.3 page 402: a `preSave` script that sets `$event.cancelAction` "causes the action to abort
/// and a cleanup process to take place". A save is the reader's own, so the document declining it
/// is asked at the reader's level for `save` (`CLAUDE.md`, ADR 2763): `off` writes and says
/// nothing, `warn` writes and says why, `on` writes nothing and says why, and `ask` writes on a
/// `yes` and not on a `no`. Wherever the file is not written, page 400's cleanup still fires the
/// `postSave` its `preSave` pushed — "[t]he cleanup process pops stacked post-events and fires
/// them" — so `Total`, which `postSave` sets to 9, reads 9 at every level and every answer.
#[test]
fn a_pre_save_cancel_action_is_asked_at_the_reader_s_level_for_save() {
    let said = |events: &[Event]| {
        events
            .iter()
            .filter_map(|event| match event {
                Event::Warned { operation, .. } => Some(("warned", *operation)),
                Event::Refused { operation, .. } => Some(("refused", *operation)),
                Event::Asking { operation, .. } => Some(("asking", *operation)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let (viewer, events) = saved_under(RestrictionLevel::Off);
    assert!(wrote(&events), "off writes");
    assert_eq!(said(&events), [], "off says nothing");
    assert_eq!(value(&viewer, "Total"), "9");

    let (viewer, events) = saved_under(RestrictionLevel::Warn);
    assert!(wrote(&events), "warn writes");
    assert_eq!(said(&events), [("warned", Operation::Save)]);
    assert_eq!(value(&viewer, "Total"), "9");

    let (viewer, events) = saved_under(RestrictionLevel::On);
    assert!(!wrote(&events), "on writes nothing");
    assert_eq!(said(&events), [("refused", Operation::Save)]);
    assert_eq!(value(&viewer, "Total"), "9", "the cleanup fires postSave");

    for (proceed, written) in [(true, true), (false, false)] {
        let (mut viewer, events) = saved_under(RestrictionLevel::Ask);
        assert!(!wrote(&events), "ask writes nothing before the answer");
        assert_eq!(said(&events), [("asking", Operation::Save)]);
        assert_eq!(value(&viewer, "Total"), "", "postSave waits on the answer");
        let answered: Vec<Event> = viewer
            .handle(Command::Answer {
                document: DOCUMENT,
                proceed,
            })
            .collect();
        assert_eq!(wrote(&answered), written, "answered {proceed}");
        assert_eq!(value(&viewer, "Total"), "9", "answered {proceed}");
    }
}

/// A sheet of A4 at 300 dots per inch, the one every print here asks for.
fn a4() -> viewer_core::Sheet {
    viewer_core::Sheet {
        media: Some([0.0, 0.0, 595.0, 842.0]),
        scale: 300.0 / 72.0,
        page_scale: 1.0,
    }
}

/// Whether the events hold a print's grant.
fn printing(events: &[Event]) -> bool {
    events
        .iter()
        .any(|event| matches!(event, Event::Printing { .. }))
}

/// XFA 3.3 page 388: `prePrint` "fires just before rendering for print begins", and `postPrint`
/// "just after the rendered form has been sent to the printer, spooler, or output destination". So
/// `Units`, which a `prePrint` bound to `$host` sets to 4, reads 4 once the print begins and before
/// a page is pulled, and `Total`, which a `postPrint` sets to 9, stays empty until the print
/// operation ends; at `off` neither runs (ADR 1616), and the print goes ahead the same.
#[test]
fn a_print_runs_its_pre_print_event_before_the_pages_and_its_post_print_event_after() {
    let fields = "<field name=\"Units\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"prePrint\" ref=\"$host\"><script>4</script></event></field>\
                  <field name=\"Total\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"postPrint\" ref=\"$host\"><script>9</script></event></field>";
    for (scripting, units, total) in [
        (Scripting::Run(Arc::new(Calculators::default())), "4", "9"),
        (Scripting::Off, "", ""),
    ] {
        let mut viewer = opened_on(scripting, hybrid(fields, "<Units/><Total/>"));
        viewer.handle(Command::Presented).for_each(drop);
        let began: Vec<Event> = viewer
            .handle(Command::Print(viewer_core::Printing::Start(a4())))
            .collect();
        assert!(printing(&began), "the print goes ahead");
        assert_eq!(
            value(&viewer, "Units"),
            units,
            "prePrint ran before the pages"
        );
        assert_eq!(value(&viewer, "Total"), "", "postPrint waits for the end");
        viewer
            .handle(Command::Print(viewer_core::Printing::Finish))
            .for_each(drop);
        assert_eq!(value(&viewer, "Total"), total);
    }
}

/// Opens a hybrid form whose `prePrint` script declines the print, under one level for the
/// reader's `print`, and asks to print it.
fn printed_under(level: RestrictionLevel) -> (Viewer, Vec<Event>) {
    let fields = "<field name=\"Units\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"prePrint\" ref=\"$host\"><script>cancel</script></event>\
                  </field>\
                  <field name=\"Total\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"postPrint\" ref=\"$host\"><script>9</script></event></field>";
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Restrict(RestrictionScope::Window(
            RestrictionPolicy::default().with(Operation::Print, level),
        )))
        .for_each(drop);
    viewer
        .handle(Command::Scripts(Scripting::Run(Arc::new(
            Calculators::default(),
        ))))
        .for_each(drop);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: hybrid(fields, "<Units/><Total/>").into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer.handle(Command::Presented).for_each(drop);
    let events = viewer
        .handle(Command::Print(viewer_core::Printing::Start(a4())))
        .collect();
    (viewer, events)
}

/// XFA 3.3 page 399: "[t]he cancelAction flag is used to cancel whatever action fired the event",
/// and `prePrint` is the pre-event of a print. A print is the reader's own, so the document
/// declining it is asked at the reader's level for `print` (`CLAUDE.md`, ADR 2777): `off` prints
/// and says nothing, `warn` prints and says why, `on` prints nothing and says why, and `ask` prints
/// on a `yes` and not on a `no`. Wherever nothing is printed, page 400's cleanup fires the
/// `postPrint` its `prePrint` pushed, so `Total`, which `postPrint` sets to 9, reads 9.
#[test]
fn a_pre_print_cancel_action_is_asked_at_the_reader_s_level_for_print() {
    let said = |events: &[Event]| {
        events
            .iter()
            .filter_map(|event| match event {
                Event::Warned { operation, .. } => Some(("warned", *operation)),
                Event::Refused { operation, .. } => Some(("refused", *operation)),
                Event::Asking { operation, .. } => Some(("asking", *operation)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let (viewer, events) = printed_under(RestrictionLevel::Off);
    assert!(printing(&events), "off prints");
    assert_eq!(said(&events), [], "off says nothing");
    assert_eq!(value(&viewer, "Total"), "", "postPrint waits for the end");

    let (_, events) = printed_under(RestrictionLevel::Warn);
    assert!(printing(&events), "warn prints");
    assert_eq!(said(&events), [("warned", Operation::Print)]);

    let (viewer, events) = printed_under(RestrictionLevel::On);
    assert!(!printing(&events), "on prints nothing");
    assert_eq!(said(&events), [("refused", Operation::Print)]);
    assert_eq!(value(&viewer, "Total"), "9", "the cleanup fires postPrint");

    for (proceed, printed, total) in [(true, true, ""), (false, false, "9")] {
        let (mut viewer, events) = printed_under(RestrictionLevel::Ask);
        assert!(!printing(&events), "ask prints nothing before the answer");
        assert_eq!(said(&events), [("asking", Operation::Print)]);
        let answered: Vec<Event> = viewer
            .handle(Command::Answer {
                document: DOCUMENT,
                proceed,
            })
            .collect();
        assert_eq!(printing(&answered), printed, "answered {proceed}");
        assert_eq!(value(&viewer, "Total"), total, "answered {proceed}");
    }
}

/// The sentences reported as the document closes.
fn closing(viewer: &mut Viewer) -> Vec<String> {
    viewer
        .handle(Command::Close(DOCUMENT))
        .filter_map(|event| match event {
            Event::Reported { notes, .. } => Some(notes),
            _ => None,
        })
        .flatten()
        .collect()
}

/// XFA 3.3 page 388: `docClose` "fires at the very end of processing if and only if all
/// validations succeeded", success being "nothing worse than a warning (no errors)". The event's
/// script here is `unrunnable`, which this test's runner refuses by name, so the report says
/// whether it ran. `Units` carries a null test at the level `error` (page 384): empty, it fails,
/// and the event does not run and the report says why; filled, every validation succeeds and the
/// event runs. At `off` no script runs, and nothing is validated for it (ADR 1616).
#[test]
fn the_close_runs_the_doc_close_event_only_where_every_validation_succeeded() {
    let fields = "<field name=\"Units\"><ui><textEdit/></ui><value><text/></value>\
                  <validate nullTest=\"error\"/>\
                  <event activity=\"docClose\" ref=\"$host\"><script>unrunnable</script>\
                  </event></field>\
                  <field name=\"Total\"><ui><textEdit/></ui><value><text/></value></field>";
    let ran = |notes: &[String]| notes.iter().any(|note| note.contains("\"unrunnable\""));
    let withheld = |notes: &[String]| {
        notes
            .iter()
            .any(|note| note.contains("docClose events are not run") && note.contains("Units"))
    };

    let mut viewer = opened_on(
        Scripting::Run(Arc::new(Calculators::default())),
        hybrid(fields, "<Units/><Total/>"),
    );
    viewer.handle(Command::Presented).for_each(drop);
    let notes = closing(&mut viewer);
    assert!(!ran(&notes) && withheld(&notes), "{notes:?}");

    let mut viewer = opened_on(
        Scripting::Run(Arc::new(Calculators::default())),
        hybrid(fields, "<Units/><Total/>"),
    );
    viewer.handle(Command::Presented).for_each(drop);
    enter(&mut viewer, "Units", "3");
    let notes = closing(&mut viewer);
    assert!(ran(&notes) && !withheld(&notes), "{notes:?}");

    let mut viewer = opened_on(Scripting::Off, hybrid(fields, "<Units/><Total/>"));
    viewer.handle(Command::Presented).for_each(drop);
    let notes = closing(&mut viewer);
    assert!(!ran(&notes) && !withheld(&notes), "{notes:?}");
}

/// XFA 3.3 page 447: "after any attempted submit operation", the client runs the form's
/// `postSubmit` events "\[w]hether or not the submit succeeded". A host whose submission ended
/// with no form data to hand back — declined, unsent, unanswered or answered with something else —
/// says so with `Command::Unanswered`, and the events run: `Total`,
/// which a `postSubmit` bound to `$form` (page 389: "applies only to the Form DOM") sets to 9,
/// reads 9 afterwards. At `off` no script runs (ADR 1616), and a document no longer open takes
/// nothing (ADR 2764).
#[test]
fn a_submission_that_ended_unanswered_runs_the_post_submit_events() {
    let fields = "<field name=\"Units\"><ui><textEdit/></ui><value><text/></value></field>\
                  <field name=\"Total\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"postSubmit\" ref=\"$form\"><script>9</script></event>\
                  </field>";
    for (scripting, total) in [
        (Scripting::Run(Arc::new(Calculators::default())), "9"),
        (Scripting::Off, ""),
    ] {
        let mut viewer = opened_on(scripting, hybrid(fields, "<Units/><Total/>"));
        viewer.handle(Command::Presented).for_each(drop);
        assert_eq!(value(&viewer, "Total"), "", "nothing was submitted yet");
        viewer
            .handle(Command::Unanswered {
                document: DocumentId(9),
            })
            .for_each(drop);
        assert_eq!(value(&viewer, "Total"), "", "another document's submission");
        viewer
            .handle(Command::Unanswered { document: DOCUMENT })
            .for_each(drop);
        assert_eq!(value(&viewer, "Total"), total);
    }
}

/// XFA 3.3 page 392, from XFA 2.4: a click on an item of a choice list or Enter, which a host
/// sends as the field's commit while the keyboard focus stays on its widget, fires the list's
/// `exit`; the next commit while the focus stays fires its `enter` again first; and the focus then
/// leaving fires no second `exit` of the list, since the commit's paired the last `enter` (ADR
/// 2707). `Enters` and `Exits` hold events bound by `ref` to the list (page 706), each assigning
/// its number to its own field, so each shows whether the list's event fired since it was cleared.
#[test]
fn a_selection_committed_in_a_focused_choice_list_exits_it_and_the_next_reenters_it() {
    let fields = "<field name=\"Units\"><ui><choiceList/></ui><value><text/></value></field>\
                  <field name=\"Enters\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"enter\" ref=\"Units\"><script>4</script></event></field>\
                  <field name=\"Exits\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"exit\" ref=\"Units\"><script>9</script></event></field>";
    let data = "<Units/><Enters/><Exits/>";
    let mut viewer = opened_on(
        Scripting::Run(Arc::new(Calculators::default())),
        hybrid_of(fields, data, &["Units", "Enters", "Exits"]),
    );
    viewer.handle(Command::Presented).for_each(drop);
    viewer
        .handle(Command::Focused(viewer_core::FocusMove::Next))
        .for_each(drop);
    assert_eq!(value(&viewer, "Enters"), "4", "the focus reached Units");
    let clear = |viewer: &mut Viewer| {
        enter(viewer, "Enters", "0");
        enter(viewer, "Exits", "0");
    };
    clear(&mut viewer);
    enter(&mut viewer, "Units", "a");
    assert_eq!(
        (value(&viewer, "Enters"), value(&viewer, "Exits")),
        ("0".to_owned(), "9".to_owned()),
        "the first selection exits the list and enters nothing"
    );
    clear(&mut viewer);
    enter(&mut viewer, "Units", "b");
    assert_eq!(
        (value(&viewer, "Enters"), value(&viewer, "Exits")),
        ("4".to_owned(), "9".to_owned()),
        "the second selection enters the list again and exits it"
    );
    clear(&mut viewer);
    viewer
        .handle(Command::Focused(viewer_core::FocusMove::Next))
        .for_each(drop);
    assert_eq!(
        value(&viewer, "Exits"),
        "0",
        "the focus leaving fires no second exit"
    );
}

/// XFA 3.3 pages 392 and 402: a host that says Enter committed a focused choice list's selection
/// gives the list's `exit` `$event.commitKey` 2, "[t]he value was committed by pressing the enter
/// key"; a commit the host cannot place — a click on an item, within the field and so none of page
/// 402's clicks outside it — leaves the property unoffered, and the event's script is refused by
/// name and assigns nothing (ADR 2755). `Exits` holds an event bound by `ref` to the list (page
/// 706), which assigns what it reads to its own field.
#[test]
fn enter_in_a_focused_choice_list_is_commit_key_2_to_its_exit() {
    let fields = "<field name=\"Units\"><ui><choiceList/></ui><value><text/></value></field>\
                  <field name=\"Exits\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"exit\" ref=\"Units\"><script>$event.commitKey</script>\
                  </event></field>";
    let mut viewer = opened_on(
        Scripting::Run(Arc::new(Calculators::default())),
        hybrid_of(fields, "<Units/><Exits/>", &["Units", "Exits"]),
    );
    viewer.handle(Command::Presented).for_each(drop);
    viewer
        .handle(Command::Focused(viewer_core::FocusMove::Next))
        .for_each(drop);
    let units = "form1[0].Units[0]".to_owned();
    let commit = |viewer: &mut Viewer, text: &str, by| {
        viewer
            .handle(Command::Edit(Edit::SetField {
                field: units.clone(),
                value: Entered::Text(text.to_owned()),
            }))
            .for_each(drop);
        viewer
            .handle(Command::CommitField {
                field: units.clone(),
                by,
            })
            .for_each(drop);
    };
    commit(&mut viewer, "a", Committing::Enter);
    assert_eq!(
        value(&viewer, "Exits"),
        "2",
        "Enter committed the selection"
    );
    enter(&mut viewer, "Exits", "x");
    commit(&mut viewer, "b", Committing::Unstated);
    assert_eq!(
        value(&viewer, "Exits"),
        "x",
        "a commit the host cannot place offers no commitKey"
    );
}

/// XFA 3.3 page 402: `$event.commitKey` describes "what happened to the value in the field when the
/// form field lost focus", and its 0 is "[t]he value was not committed (for example, the escape key
/// was pressed)". `Command::CancelField`, a host's Escape, puts back what `Units` showed before the
/// typing began and takes the keyboard focus off its widget, so its `exit` reads 0; a field that
/// does not hold the focus fires nothing (ADR 2756).
#[test]
fn escape_in_a_focused_field_abandons_the_typing_and_its_exit_reads_commit_key_0() {
    let fields = "<field name=\"Units\"><ui><textEdit/></ui><value><text/></value></field>\
                  <field name=\"Exits\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"exit\" ref=\"Units\"><script>$event.commitKey</script>\
                  </event></field>";
    let mut viewer = opened_on(
        Scripting::Run(Arc::new(Calculators::default())),
        hybrid_of(fields, "<Units/><Exits/>", &["Units", "Exits"]),
    );
    viewer.handle(Command::Presented).for_each(drop);
    let units = "form1[0].Units[0]".to_owned();
    // Not focused: the typing goes, and no `exit` fires.
    let typed = |viewer: &mut Viewer| {
        viewer
            .handle(Command::Edit(Edit::SetField {
                field: units.clone(),
                value: Entered::Text("7".to_owned()),
            }))
            .for_each(drop);
        assert_eq!(value(viewer, "Units"), "7", "the typing shows");
    };
    typed(&mut viewer);
    viewer
        .handle(Command::CancelField {
            field: units.clone(),
        })
        .for_each(drop);
    assert_eq!(value(&viewer, "Units"), "", "the typing is abandoned");
    assert_eq!(value(&viewer, "Exits"), "", "Units never held the focus");
    viewer
        .handle(Command::Focused(viewer_core::FocusMove::Next))
        .for_each(drop);
    typed(&mut viewer);
    viewer
        .handle(Command::CancelField {
            field: units.clone(),
        })
        .for_each(drop);
    assert_eq!(value(&viewer, "Units"), "", "the typing is abandoned");
    assert_eq!(
        value(&viewer, "Exits"),
        "0",
        "Escape left Units uncommitted"
    );
}

/// XFA 3.3 page 396: an object's `validationState` event fires when the form is initialized and
/// afterwards only where its validation state changed, so the window keeps each container's state
/// from one activation to the next (ADR 2706). `Amount`'s script test is its own value, which an
/// entry of `5` passes and of `0` fails; `Flag` holds an event bound by `ref` to `Amount`.
#[test]
fn the_window_fires_a_validation_state_event_only_where_an_entry_changes_the_state() {
    let fields = "<field name=\"Amount\"><ui><textEdit/></ui><value><text/></value>\
                  <validate><script>same</script></validate></field>\
                  <field name=\"Flag\"><ui><textEdit/></ui><value><text/></value>\
                  <event activity=\"validationState\" ref=\"Amount\"><script>7</script></event>\
                  </field>";
    let data = "<Amount/><Flag/>";
    let mut viewer = opened_on(
        Scripting::Run(Arc::new(Calculators::default())),
        hybrid_of(fields, data, &["Amount", "Flag"]),
    );
    viewer.handle(Command::Presented).for_each(drop);
    assert_eq!(
        value(&viewer, "Flag"),
        "7",
        "the initialization announces it"
    );
    for (typed, flag) in [("5", "0"), ("0", "7"), ("0", "0"), ("5", "7")] {
        enter(&mut viewer, "Flag", "0");
        enter(&mut viewer, "Amount", typed);
        assert_eq!(value(&viewer, "Flag"), flag, "after {typed:?}");
    }
}

/// Whether `events` grant the drop-down list of the field `Units`.
fn list_opens(events: &[Event]) -> bool {
    events.iter().any(
        |event| matches!(event, Event::ListOpens { field, .. } if field == "form1[0].Units[0]"),
    )
}

/// Opens a hybrid form whose drop-down list `Units` runs `pre_open` as its `preOpen` and sets
/// itself to 9 at its `postOpen`, under one level for the reader's `fill`, and asks for its list.
fn list_asked_under(level: RestrictionLevel, pre_open: &str) -> (Viewer, Vec<Event>) {
    let fields = format!(
        "<field name=\"Units\"><ui><choiceList/></ui><value><text/></value>\
         <event activity=\"preOpen\"><script>{pre_open}</script></event>\
         <event activity=\"postOpen\"><script>9</script></event></field>\
         <field name=\"Total\"><ui><textEdit/></ui><value><text/></value></field>"
    );
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Restrict(RestrictionScope::Window(
            RestrictionPolicy::default().with(Operation::FillInForm, level),
        )))
        .for_each(drop);
    viewer
        .handle(Command::Scripts(Scripting::Run(Arc::new(
            Calculators::default(),
        ))))
        .for_each(drop);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: hybrid(&fields, "<Units/><Total/>").into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer.handle(Command::Presented).for_each(drop);
    let events = viewer
        .handle(Command::OpeningList {
            field: "form1[0].Units[0]".to_owned(),
        })
        .collect();
    (viewer, events)
}

/// XFA 3.3 page 392: `preOpen` fires when a person asks for a drop-down list and before it is
/// displayed, and `postOpen` "fires after the combo box is displayed". So `Units`, which its
/// `preOpen` sets to 4, reads 4 at the grant, before the host displays the list, and 9 once the
/// host says the list is shown; at `off` neither runs (ADR 1616), and the list is granted the
/// same (ADR 2791).
#[test]
fn a_drop_down_lists_opening_runs_pre_open_before_the_grant_and_post_open_after_the_display() {
    for (scripting, before, after) in [
        (Scripting::Run(Arc::new(Calculators::default())), "4", "9"),
        (Scripting::Off, "", ""),
    ] {
        let fields = "<field name=\"Units\"><ui><choiceList/></ui><value><text/></value>\
                      <event activity=\"preOpen\"><script>4</script></event>\
                      <event activity=\"postOpen\"><script>9</script></event></field>\
                      <field name=\"Total\"><ui><textEdit/></ui><value><text/></value></field>";
        let mut viewer = opened_on(scripting, hybrid(fields, "<Units/><Total/>"));
        viewer.handle(Command::Presented).for_each(drop);
        let asked: Vec<Event> = viewer
            .handle(Command::OpeningList {
                field: "form1[0].Units[0]".to_owned(),
            })
            .collect();
        assert!(list_opens(&asked), "the list is granted");
        assert_eq!(
            value(&viewer, "Units"),
            before,
            "preOpen ran before the grant"
        );
        viewer
            .handle(Command::OpenedList {
                field: "form1[0].Units[0]".to_owned(),
            })
            .for_each(drop);
        assert_eq!(
            value(&viewer, "Units"),
            after,
            "postOpen ran after the display"
        );
    }
}

/// XFA 3.3 page 399: `cancelAction` cancels "whatever action fired the event", and the action a
/// `preOpen` fires for is the display of a list a person asked for in order to fill the field in.
/// So the document declining it is asked at the reader's level for `fill` (`CLAUDE.md`, ADR 2791):
/// `off` grants the list and says nothing, `warn` grants it and says why, `on` withholds it and
/// says why, and `ask` grants it on a `yes` and not on a `no`. Wherever the list is withheld, page
/// 400's cleanup fires the `postOpen` its `preOpen` pushed, so `Units` reads 9.
#[test]
fn a_pre_open_cancel_action_is_asked_at_the_reader_s_level_for_fill() {
    let said = |events: &[Event]| {
        events
            .iter()
            .filter_map(|event| match event {
                Event::Warned { operation, .. } => Some(("warned", *operation)),
                Event::Refused { operation, .. } => Some(("refused", *operation)),
                Event::Asking { operation, .. } => Some(("asking", *operation)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let (viewer, events) = list_asked_under(RestrictionLevel::Off, "cancel");
    assert!(list_opens(&events), "off grants the list");
    assert_eq!(said(&events), [], "off says nothing");
    assert_eq!(
        value(&viewer, "Units"),
        "",
        "postOpen waits for the display"
    );

    let (_, events) = list_asked_under(RestrictionLevel::Warn, "cancel");
    assert!(list_opens(&events), "warn grants the list");
    assert_eq!(said(&events), [("warned", Operation::FillInForm)]);

    let (viewer, events) = list_asked_under(RestrictionLevel::On, "cancel");
    assert!(!list_opens(&events), "on withholds the list");
    assert_eq!(said(&events), [("refused", Operation::FillInForm)]);
    assert_eq!(value(&viewer, "Units"), "9", "the cleanup fires postOpen");

    let (_, events) = list_asked_under(RestrictionLevel::On, "4");
    assert!(
        list_opens(&events),
        "a preOpen that does not cancel restricts nothing"
    );
    assert_eq!(said(&events), []);

    for (proceed, granted, units) in [(true, true, ""), (false, false, "9")] {
        let (mut viewer, events) = list_asked_under(RestrictionLevel::Ask, "cancel");
        assert!(!list_opens(&events), "ask grants nothing before the answer");
        assert_eq!(said(&events), [("asking", Operation::FillInForm)]);
        let answered: Vec<Event> = viewer
            .handle(Command::Answer {
                document: DOCUMENT,
                proceed,
            })
            .collect();
        assert_eq!(list_opens(&answered), granted, "answered {proceed}");
        assert_eq!(value(&viewer, "Units"), units, "answered {proceed}");
    }
}
