//! A hybrid form's calculations, `FormCalc` and JavaScript, run in the confined worker, through the
//! view state a host supplied a runner to, and answer the form's questions from the host's side of
//! the pipe (XFA 3.3 pages 379–380 and 641–642; ADRs 2524, 2545).
//!
//! The form is a real hybrid file; the path is the real one — the view state activates, the
//! runner starts `pdf-script-worker` at the first script, the worker confines itself and evaluates
//! with `pdf_formcalc` or in its form's JavaScript realm, and every path, value and assignment
//! crosses the pipe as a question.

#![cfg(feature = "engine")]
#![expect(
    clippy::expect_used,
    reason = "a test asserts, and a failed assertion is its report"
)]

use std::fmt::Write as _;
use std::sync::Arc;
use std::time::Duration;

use pdf_model::view::{Entered, FormOverride, FormOverrideMode, ScriptRunner, ViewState};
use pdf_script_worker::{Cause, ScriptWorker};
use pdf_syntax::Document;

/// The worker program Cargo built beside this test.
const WORKER: &str = env!("CARGO_BIN_EXE_pdf-script-worker");

/// A deadline no run of these tests is meant to reach.
const OUT_OF_THE_WAY: Duration = Duration::from_mins(1);

/// A field with a text value, a text edit, and the automation elements `inside` states.
fn field(name: &str, inside: &str) -> String {
    format!("<field name=\"{name}\"><ui><textEdit/></ui><value><text/></value>{inside}</field>")
}

/// A `calculate` element running `script`, with the `override` the attribute text states.
fn calculate(script: &str, attributes: &str) -> String {
    format!("<calculate {attributes}><script>{script}</script></calculate>")
}

/// A `calculate` element running `script` as JavaScript (page 106's `contentType`), with the
/// `override` the attribute text states.
fn javascript(script: &str, attributes: &str) -> String {
    format!(
        "<calculate {attributes}><script contentType=\"application/x-javascript\">{script}\
         </script></calculate>"
    )
}

/// A hybrid form: one subform `form1` of `template`, a widget and a data value for every field
/// of `values`, each at `form1[0].<name>[0]`.
fn document(template: &str, values: &[(&str, &str)]) -> Document {
    let stream = |text: &str| {
        format!(
            "<< /Length {} >>\nstream\n{text}\nendstream",
            text.len().saturating_add(1)
        )
    };
    let template = format!(
        "<template xmlns=\"http://www.xfa.org/schema/xfa-template/3.3/\">\
         <subform name=\"form1\">{template}</subform></template>"
    );
    let mut data = String::new();
    for (name, value) in values {
        let _ = write!(data, "<{name}>{value}</{name}>");
    }
    let datasets = format!(
        "<xfa:datasets xmlns:xfa=\"http://www.xfa.org/schema/xfa-data/1.0/\"><xfa:data><form1>\
         {data}</form1></xfa:data></xfa:datasets>"
    );
    let widgets: Vec<String> = (0..values.len())
        .map(|i| format!("{} 0 R", i.saturating_add(7)))
        .collect();
    let widgets = widgets.join(" ");
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] \
         /XFA [(template) 5 0 R (datasets) 6 0 R] >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [{widgets}] >>"),
        format!("<< /T (form1[0]) /Kids [{widgets}] >>"),
        stream(&template),
        stream(&datasets),
    ];
    for (index, (name, value)) in values.iter().enumerate() {
        let y = 700_usize.saturating_sub(30_usize.saturating_mul(index));
        objects.push(format!(
            "<< /Type /Annot /Subtype /Widget /FT /Tx /Parent 4 0 R /T ({name}[0]) /P 3 0 R \
             /V ({value}) /Rect [72 {y} 300 {}] /DA (/Helv 10 Tf 0 g) >>",
            y.saturating_add(20)
        ));
    }
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let _ = write!(
        out,
        "xref\n0 {}\n0000000000 65535 f \n",
        objects.len().saturating_add(1)
    );
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
        objects.len().saturating_add(1)
    );
    Document::open(out.into_bytes()).expect("the fixture opens")
}

/// The PDF field name of `short`, a template name under `form1`.
fn name(short: &str) -> String {
    format!("form1[0].{short}[0]")
}

/// A view state of `document` that runs its scripts in `worker`.
fn view_with(document: &Document, worker: &Arc<ScriptWorker>) -> ViewState {
    let mut view = ViewState::of(document);
    let runner: Arc<dyn ScriptRunner> = worker.clone();
    view.run_scripts_with(Some(runner));
    view
}

fn shown(view: &ViewState, document: &Document, short: &str) -> Option<String> {
    view.field_value(document, &name(short)).map(|v| v.text)
}

/// The merge's calculations run in the worker, which is confined, and what they computed is in the
/// view state as a typed value is.
#[test]
fn a_merge_runs_the_calculations_in_the_confined_worker() {
    let template = [
        field("Units", ""),
        field("Price", ""),
        field("Total", &calculate("Units * Price", "")),
    ]
    .concat();
    let document = document(&template, &[("Units", "3"), ("Price", "4"), ("Total", "")]);
    let worker = Arc::new(ScriptWorker::with_program(WORKER).with_deadline(OUT_OF_THE_WAY));
    let mut view = view_with(&document, &worker);

    assert!(view.activate_form_on_merge(&document), "a value changed");
    assert_eq!(shown(&view, &document, "Total").as_deref(), Some("12"));
    assert_eq!(worker.spawns(), 1, "one worker serves the whole activation");
    let confinement = worker.confinement().expect("a worker is running");
    assert!(confinement.is_enforced(), "{confinement:?}");

    // A person's entry into an input runs the calculation that reads it, in the same worker.
    assert!(view.set_field(&document, &name("Units"), &Entered::Text("5".to_owned())) > 0);
    view.commit_field(&document, &name("Units"));
    assert_eq!(shown(&view, &document, "Total").as_deref(), Some("20"));
    assert_eq!(worker.spawns(), 1);
}

/// A JavaScript calculation runs in the same confined worker, reads its inputs by their naked
/// names and `rawValue` (pages 106 and 413), and is run again at a person's entry (page 379).
#[test]
fn a_javascript_calculation_runs_in_the_confined_worker() {
    let template = [
        field("Units", ""),
        field("Price", ""),
        field("Total", &javascript("Units.rawValue * Price.rawValue", "")),
    ]
    .concat();
    let document = document(&template, &[("Units", "3"), ("Price", "4"), ("Total", "")]);
    let worker = Arc::new(ScriptWorker::with_program(WORKER).with_deadline(OUT_OF_THE_WAY));
    let mut view = view_with(&document, &worker);

    assert!(
        view.activate_form_on_merge(&document),
        "{:?}",
        view.script_reports()
    );
    assert_eq!(shown(&view, &document, "Total").as_deref(), Some("12"));
    let confinement = worker.confinement().expect("a worker is running");
    assert!(confinement.is_enforced(), "{confinement:?}");

    assert!(view.set_field(&document, &name("Units"), &Entered::Text("5".to_owned())) > 0);
    view.commit_field(&document, &name("Units"));
    assert_eq!(shown(&view, &document, "Total").as_deref(), Some("20"));
    assert_eq!(worker.spawns(), 1, "one worker serves both languages' runs");
}

/// A container object's child is a path from that container, which the worker asks the host for
/// as one (page 107's `resolveNode` of a node, the basic references of page 90), and an assignment
/// to a container's `rawValue` is an assignment the host's form makes.
#[test]
fn a_child_of_a_container_object_is_resolved_from_it_by_the_host() {
    let script = "var form = xfa.resolveNode(\"form1\");\n\
                  form.Price.rawValue = 7;\n\
                  form.Units.rawValue * form.Price.rawValue";
    let template = [
        field("Units", ""),
        field("Price", ""),
        field("Total", &javascript(script, "")),
    ]
    .concat();
    let document = document(&template, &[("Units", "3"), ("Price", "4"), ("Total", "")]);
    let worker = Arc::new(ScriptWorker::with_program(WORKER).with_deadline(OUT_OF_THE_WAY));
    let mut view = view_with(&document, &worker);
    assert!(
        view.activate_form_on_merge(&document),
        "{:?}",
        view.script_reports()
    );
    assert_eq!(shown(&view, &document, "Price").as_deref(), Some("7"));
    assert_eq!(shown(&view, &document, "Total").as_deref(), Some("21"));
}

/// The plant of ADR 2545's carrying: a JavaScript calculation that never ends, run at a person's
/// entry, is stopped in the worker by the budget of a field's event (page 380's means of ending a
/// seemingly infinite loop), the activation's calculated values are dropped, the person's entry is
/// kept, and the report names the limit the stopped run spent past. The worker is not lost: the realm stopped
/// the script, and the next run is served by the same process.
#[test]
fn a_javascript_calculation_that_never_ends_is_stopped_and_the_entry_is_kept() {
    for script in [
        "for (;;) {}",
        "var n = 0; while (true) { for (var i = 0; i &lt; 1000000; i++) { n += i; } }",
        "function f() { return f() + 1; } f()",
        "for (var i = 0; i &lt; 90000; i++) { for (var j = 0; j &lt; 90000; j++) {} }",
    ] {
        let template = [
            field("Units", ""),
            field("Double", &javascript("Units.rawValue * 2", "")),
            field("Total", &javascript(script, "")),
        ]
        .concat();
        let document = document(&template, &[("Units", "3"), ("Double", ""), ("Total", "")]);
        let worker = Arc::new(ScriptWorker::with_program(WORKER).with_deadline(OUT_OF_THE_WAY));
        let mut view = view_with(&document, &worker);
        let started = std::time::Instant::now();
        view.activate_form_on_merge(&document);
        assert!(view.set_field(&document, &name("Units"), &Entered::Text("5".to_owned())) > 0);
        view.commit_field(&document, &name("Units"));
        let took = started.elapsed();
        assert!(took < Duration::from_secs(5), "{script}: {took:?}");
        assert_eq!(
            shown(&view, &document, "Units").as_deref(),
            Some("5"),
            "{script}: the person's entry is kept"
        );
        assert_eq!(
            shown(&view, &document, "Total").as_deref(),
            Some(""),
            "{script}"
        );
        assert_eq!(
            shown(&view, &document, "Double").as_deref(),
            Some(""),
            "{script}: what the stopped activation calculated is dropped"
        );
        let said = view.script_reports().join("\n");
        assert!(
            said.contains("ended on its steps limit"),
            "{script}: {said}"
        );
        assert!(
            worker.deaths().is_empty(),
            "{script}: {:?}",
            worker.deaths()
        );
        assert_eq!(worker.spawns(), 1, "{script}");
    }
}

/// With no runner supplied, which is the level `off`, no worker starts and nothing is calculated.
#[test]
fn without_a_runner_nothing_runs_and_nothing_starts() {
    let template = [
        field("Units", ""),
        field("Total", &calculate("Units * 2", "")),
    ]
    .concat();
    let document = document(&template, &[("Units", "3"), ("Total", "")]);
    let worker = Arc::new(ScriptWorker::with_program(WORKER));
    let mut view = ViewState::of(&document);
    assert!(!view.activate_form_on_merge(&document));
    assert_eq!(shown(&view, &document, "Total").as_deref(), Some(""));
    assert_eq!(worker.spawns(), 0);
}

/// Page 642: a person's entry into a calculated field is a message for the host to present, an
/// error where the calculation's `override` is `error` (the entry is replaced) and a warning where
/// it is `warning` (the entry stands); `ignore` and `disabled` ask for none.
#[test]
fn an_entry_into_a_calculated_field_is_flagged_for_the_host_by_its_override() {
    for (attributes, flagged, shown_after) in [
        ("override=\"error\"", Some(FormOverrideMode::Error), "6"),
        ("", Some(FormOverrideMode::Error), "6"),
        (
            "override=\"warning\"",
            Some(FormOverrideMode::Warning),
            "99",
        ),
        ("override=\"ignore\"", None, "6"),
        ("override=\"disabled\"", None, "99"),
    ] {
        let template = [
            field("Units", ""),
            field("Total", &calculate("Units * 2", attributes)),
        ]
        .concat();
        let document = document(&template, &[("Units", "3"), ("Total", "")]);
        let worker = Arc::new(ScriptWorker::with_program(WORKER).with_deadline(OUT_OF_THE_WAY));
        let mut view = view_with(&document, &worker);
        view.activate_form_on_merge(&document);
        assert!(
            view.take_form_overrides().is_empty(),
            "a merge enters nothing"
        );

        assert!(view.set_field(&document, &name("Total"), &Entered::Text("99".to_owned())) > 0);
        view.commit_field(&document, &name("Total"));
        assert_eq!(
            view.take_form_overrides(),
            flagged
                .map(|mode| FormOverride {
                    field: name("Total"),
                    mode
                })
                .into_iter()
                .collect::<Vec<_>>(),
            "{attributes:?}"
        );
        assert_eq!(
            shown(&view, &document, "Total").as_deref(),
            Some(shown_after),
            "{attributes:?}"
        );
    }
}

/// A script that puts more questions than a run may is ended by the host with the error the
/// refusal is: the script is the one that cannot spend the pipe, and the form is never grown.
#[test]
fn a_script_that_puts_too_many_questions_is_ended_with_a_sentence() {
    let script = "var i = 0\nwhile (i &lt; 20000) do\ni = i + 1\n$ = i\nendwhile\n$";
    let template = field("Total", &calculate(script, ""));
    let document = document(&template, &[("Total", "")]);
    let worker = Arc::new(ScriptWorker::with_program(WORKER).with_deadline(OUT_OF_THE_WAY));
    let mut view = view_with(&document, &worker);
    view.activate_form_on_merge(&document);
    let said = view.script_reports().join("\n");
    assert!(said.contains("more than 10000 questions"), "{said}");
    assert_eq!(
        worker.spawns(),
        1,
        "the worker was not lost: it was told no"
    );
    assert!(worker.deaths().is_empty());
}

/// A run that reaches the deadline loses its worker, the activation ends there, and the next run
/// starts another worker.
#[test]
fn a_run_past_the_deadline_loses_its_worker_and_the_next_run_starts_another() {
    let script = "var i = 0\nwhile (i &lt; 20000) do\ni = i + 1\n$ = i\nendwhile\n$";
    let template = field("Total", &calculate(script, ""));
    let document = document(&template, &[("Total", "")]);
    let worker =
        Arc::new(ScriptWorker::with_program(WORKER).with_deadline(Duration::from_millis(30)));
    let mut view = view_with(&document, &worker);
    view.activate_form_on_merge(&document);
    let deaths = worker.deaths();
    assert_eq!(deaths.len(), 1, "{:?}", view.script_reports());
    assert!(matches!(deaths[0].cause, Cause::Deadline(_)), "{deaths:?}");
    view.activate_form_on_merge(&document);
    assert_eq!(worker.spawns(), 2, "the next run started another worker");
}

/// A script that builds a megabyte and assigns it again and again is held by the activation's
/// limit on assigned text (ADR 2507), which a conversation does not move: the host's `Form`
/// charges every assignment, so the worker is no way round it.
#[test]
fn a_script_that_assigns_megabytes_is_held_to_the_activations_text_limit() {
    let script = "var s = \"x\"\nvar i = 0\nwhile (i &lt; 20) do\ns = Concat(s, s)\ni = i + 1\nendwhile\n\
                  var j = 0\nwhile (j &lt; 200) do\n$ = s\nj = j + 1\nendwhile\n$";
    let template = field("Total", &calculate(script, ""));
    let document = document(&template, &[("Total", "")]);
    let worker = Arc::new(ScriptWorker::with_program(WORKER).with_deadline(OUT_OF_THE_WAY));
    let mut view = view_with(&document, &worker);
    view.activate_form_on_merge(&document);
    let said = view.script_reports().join("\n");
    assert!(
        said.contains("bytes limit") || said.contains("ran out of room for text"),
        "{said}"
    );
    assert_eq!(shown(&view, &document, "Total").as_deref(), Some(""));
}

/// A runner that spends a fixed time on each form script before handing it to the worker, as a
/// hostile form of many calculations each just under the deadline does.
#[derive(Debug)]
struct Slow {
    inner: Arc<ScriptWorker>,
    each: Duration,
}

impl ScriptRunner for Slow {
    fn run(&self, event: &pdf_model::view::ScriptEvent<'_>) -> pdf_model::view::ScriptResult {
        self.inner.run(event)
    }

    fn run_form_script(
        &self,
        form: &mut pdf_model::xfa::Form<'_>,
        node: usize,
        language: pdf_model::xfa::FormLanguage,
        script: &str,
        steps_left: u64,
    ) -> pdf_model::xfa::Ran {
        std::thread::sleep(self.each);
        self.inner
            .run_form_script(form, node, language, script, steps_left)
    }
}

/// Many runs, each far under the deadline of one, are held to the sequence budget together: the
/// activation stops, every value it calculated is dropped and the report says it was the clock
/// (ADR 2525).
#[test]
fn many_runs_each_under_the_deadline_are_held_to_the_sequence_budget() {
    for language in [calculate, javascript] {
        held_to_the_sequence_budget(language);
    }
}

/// The sequence budget's test for the calculations `language` writes.
fn held_to_the_sequence_budget(language: fn(&str, &str) -> String) {
    let template: String = (0..20)
        .map(|i| field(&format!("C{i}"), &language("1", "")))
        .collect();
    let names: Vec<String> = (0..20).map(|i| format!("C{i}")).collect();
    let values: Vec<(&str, &str)> = names.iter().map(|n| (n.as_str(), "")).collect();
    let document = document(&template, &values);
    let worker = Arc::new(ScriptWorker::with_program(WORKER).with_deadline(OUT_OF_THE_WAY));
    let mut view = ViewState::of(&document);
    let slow: Arc<dyn ScriptRunner> = Arc::new(Slow {
        inner: worker,
        each: Duration::from_millis(150),
    });
    view.run_scripts_with(Some(slow));

    let started = std::time::Instant::now();
    assert!(!view.activate_form_on_merge(&document), "nothing is kept");
    let took = started.elapsed();
    assert!(
        took < Duration::from_secs(3),
        "twenty runs of 150 ms were stopped at the budget, not after {took:?}"
    );
    for short in &names {
        assert_eq!(
            shown(&view, &document, short).as_deref(),
            Some(""),
            "{short}"
        );
    }
    let said = view.script_reports().join("\n");
    assert!(said.contains("ran longer than their budget"), "{said}");
}

/// Page 642: a person's entry made before the merge ran — the runner arrived late, as at `ask` — is
/// kept where the calculation's `override` is `warning`, and replaced where it is `error`, as it
/// would have been had the merge come first.
#[test]
fn an_entry_made_before_a_late_merge_is_treated_as_an_entry() {
    for (attributes, shown_after, flagged) in [
        (
            "override=\"warning\"",
            "99",
            Some(FormOverrideMode::Warning),
        ),
        ("override=\"error\"", "6", Some(FormOverrideMode::Error)),
        ("override=\"disabled\"", "99", None),
    ] {
        let template = [
            field("Units", ""),
            field("Total", &calculate("Units * 2", attributes)),
        ]
        .concat();
        let document = document(&template, &[("Units", "3"), ("Total", "")]);
        let mut view = ViewState::of(&document);
        assert!(view.set_field(&document, &name("Total"), &Entered::Text("99".to_owned())) > 0);
        view.commit_field(&document, &name("Total"));
        assert_eq!(shown(&view, &document, "Total").as_deref(), Some("99"));

        let worker = Arc::new(ScriptWorker::with_program(WORKER).with_deadline(OUT_OF_THE_WAY));
        let runner: Arc<dyn ScriptRunner> = worker;
        view.run_scripts_with(Some(runner));
        view.activate_form_on_merge(&document);
        assert_eq!(
            shown(&view, &document, "Total").as_deref(),
            Some(shown_after),
            "{attributes}"
        );
        assert_eq!(
            view.take_form_overrides(),
            flagged
                .map(|mode| FormOverride {
                    field: name("Total"),
                    mode
                })
                .into_iter()
                .collect::<Vec<_>>(),
            "{attributes}"
        );
    }
}

/// A late merge over many entered fields is one sequence: the entries it re-raises share the
/// merge's budget and do not each begin another (ADR 2525).
#[test]
fn a_late_merge_and_its_entries_share_one_sequence_budget() {
    let count = 12;
    let names: Vec<String> = (0..count).map(|i| format!("C{i}")).collect();
    let template: String = names
        .iter()
        .map(|n| field(n, &calculate("1", "override=\"warning\"")))
        .collect();
    let values: Vec<(&str, &str)> = names.iter().map(|n| (n.as_str(), "")).collect();
    let document = document(&template, &values);
    let mut view = ViewState::of(&document);
    for short in &names {
        assert!(view.set_field(&document, &name(short), &Entered::Text("9".to_owned())) > 0);
        view.commit_field(&document, &name(short));
    }
    let worker = Arc::new(ScriptWorker::with_program(WORKER).with_deadline(OUT_OF_THE_WAY));
    let slow: Arc<dyn ScriptRunner> = Arc::new(Slow {
        inner: worker,
        each: Duration::from_millis(150),
    });
    view.run_scripts_with(Some(slow));

    let started = std::time::Instant::now();
    view.activate_form_on_merge(&document);
    let took = started.elapsed();
    assert!(
        took < Duration::from_millis(2500),
        "the merge and twelve re-raised entries shared one budget of a second, not {took:?}"
    );
    for short in &names {
        assert_eq!(
            shown(&view, &document, short).as_deref(),
            Some("9"),
            "{short}"
        );
    }
}

/// An `event` element of `activity` running `script` as JavaScript (page 106's `contentType`).
fn javascript_event(activity: &str, script: &str) -> String {
    format!(
        "<event activity=\"{activity}\"><script contentType=\"application/x-javascript\">\
         {script}</script></event>"
    )
}

/// XFA 3.3 pages 390–391: the focus reaching a field runs its `enter` event in the confined
/// worker, and a value the event sets wakes the calculation that reads it (page 379).
#[test]
fn an_enter_event_runs_in_the_confined_worker_and_wakes_a_calculation() {
    let template = [
        field("Units", &javascript_event("enter", "this.rawValue = 4;")),
        field("Double", &javascript("Units.rawValue * 2", "")),
    ]
    .concat();
    let document = document(&template, &[("Units", "1"), ("Double", "")]);
    let worker = Arc::new(ScriptWorker::with_program(WORKER).with_deadline(OUT_OF_THE_WAY));
    let mut view = view_with(&document, &worker);
    view.activate_form_on_merge(&document);
    assert_eq!(shown(&view, &document, "Double").as_deref(), Some("2"));
    assert!(view.activate_form_on_enter(&document, &name("Units"), None));
    assert_eq!(shown(&view, &document, "Units").as_deref(), Some("4"));
    assert_eq!(shown(&view, &document, "Double").as_deref(), Some("8"));
    assert!(worker.deaths().is_empty(), "{:?}", worker.deaths());
}

/// The plant: an `exit` event that never ends, raised after the commit a loss of focus makes, as
/// `viewer-core` raises it. The worker stops it within the activation's budget, the person's entry
/// stands, and what the stopped activation set is dropped.
#[test]
fn an_exit_event_that_never_ends_is_stopped_and_the_entry_is_kept() {
    for script in [
        "Other.rawValue = 9; for (;;) {}",
        "Other.rawValue = 9; function f() { return f() + 1; } f()",
        "Other.rawValue = 9; for (var i = 0; i &lt; 90000; i++) { for (var j = 0; j &lt; 90000; j++) {} }",
    ] {
        let template = [
            field("Units", &javascript_event("exit", script)),
            field("Other", ""),
        ]
        .concat();
        let document = document(&template, &[("Units", "3"), ("Other", "")]);
        let worker = Arc::new(ScriptWorker::with_program(WORKER).with_deadline(OUT_OF_THE_WAY));
        let mut view = view_with(&document, &worker);
        view.activate_form_on_merge(&document);
        let started = std::time::Instant::now();
        assert!(view.set_field(&document, &name("Units"), &Entered::Text("5".to_owned())) > 0);
        view.commit_field(&document, &name("Units"));
        let changed = view.activate_form_on_exit(&document, &name("Units"), Some(&name("Other")));
        let took = started.elapsed();
        assert!(took < Duration::from_secs(5), "{script}: {took:?}");
        assert!(!changed, "{script}");
        assert_eq!(
            shown(&view, &document, "Units").as_deref(),
            Some("5"),
            "{script}: the person's entry is kept"
        );
        assert_eq!(
            shown(&view, &document, "Other").as_deref(),
            Some(""),
            "{script}: what the stopped event set is dropped"
        );
        let said = view.script_reports().join("\n");
        assert!(said.contains("its exit event"), "{script}: {said}");
        assert!(
            said.contains("nothing it calculated is kept"),
            "{script}: {said}"
        );
        assert!(
            worker.deaths().is_empty(),
            "{script}: {:?}",
            worker.deaths()
        );
    }
}
