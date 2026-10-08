//! A test host that supplies a level and drives triggers end to end through the confined worker.
//!
//! No window supplies a level for scripts yet — RFC 0008 section 11 item 3 (e) — so the host here is
//! the smallest that does: it holds a level, and at `on` hands a view state a [`ScriptWorker`]
//! through `ViewState::run_scripts_with`, the one place a host's level reaches the view (ADR 1591);
//! at `off` it hands nothing. Everything after that is the real path: the view state raises Table
//! 199's `/K` and `/F`, the runner starts `pdf-script-worker` at the first of them, the worker
//! confines itself under `pdf_sandbox::lockdown::Profile::Script` and runs the script, and the
//! outcome crosses back (ADRs 1608, 1609).
//!
//! The losses are driven through the runner directly, with a hand-made event, because a script
//! written to kill its worker is not a form anyone fills in.

#![cfg(feature = "engine")]
#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "test code: an explanatory panic is the intended failure, and the open cost is printed \
              for the record"
)]

use std::fmt::Write as _;
use std::sync::Arc;
use std::time::{Duration, Instant};

use pdf_model::aform::Trigger;
use pdf_model::view::{Entered, ScriptEvent, ScriptRunner, ScriptSite, ViewState};
use pdf_script_worker::{Cause, DEADLINE, MAX_DEATHS, ScriptWorker};
use pdf_syntax::Document;

/// The worker program Cargo built beside this test.
const WORKER: &str = env!("CARGO_BIN_EXE_pdf-script-worker");

/// RFC 0008 section 6.3's level, as far as a host that has no dialogue needs it: `ask` and `warn`
/// supply a runner as `on` does, and differ in what the host says around it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Level {
    Off,
    On,
}

/// The smallest host that supplies a level.
struct Host {
    document: Document,
    view: ViewState,
    worker: Arc<ScriptWorker>,
}

impl Host {
    /// Opens `document` at `level`. The worker is constructed either way, so that a test can ask
    /// whether it was started; at `off` the view state is never handed it.
    fn open(document: Document, level: Level) -> Self {
        let worker = Arc::new(ScriptWorker::with_program(WORKER));
        let mut view = ViewState::of(&document);
        if level == Level::On {
            let runner: Arc<dyn ScriptRunner> = worker.clone();
            view.run_scripts_with(Some(runner));
        }
        Self {
            document,
            view,
            worker,
        }
    }

    fn type_into(&mut self, field: &str, text: &str) -> usize {
        self.view
            .set_field(&self.document, field, &Entered::Text(text.to_owned()))
    }

    fn commit(&mut self, field: &str) {
        self.view.commit_field(&self.document, field);
    }

    fn displayed(&self, field: &str) -> Option<String> {
        self.view.displayed_value(&self.document, field)
    }
}

/// A one-page document with one text field, `Amount`, whose `/AA` holds `actions`.
fn document(actions: &str) -> Document {
    document_with(actions, "")
}

/// [`document`], its catalog stating `catalog` as well.
fn document_with(actions: &str, catalog: &str) -> Document {
    let bodies = [
        format!("<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> {catalog} >>"),
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

/// A keystroke script that refuses an `x`, and a format script that brackets the value: neither is
/// one `AF*` call, so Tier 0 runs neither and both go to the runner.
const SCRIPTS: &str = "/K << /S /JavaScript /JS (if \\(event.change == 'x'\\) event.rc = false;) >> \
                       /F << /S /JavaScript /JS (event.value = '<' + event.value + '>';) >>";

/// One format event on `Total`, for the tests that drive the runner directly.
fn format_event(script: &str) -> ScriptEvent<'_> {
    ScriptEvent {
        site: ScriptSite::Field(Trigger::Format),
        field: "Total",
        label: "",
        script,
        value: "12",
        change: "",
        selection: (0, 0),
        will_commit: false,
        source: "",
        fields: &[],
        page: 0,
        pages: 1,
        commit_key: None,
        field_full: false,
        change_ex: "",
        dirty: false,
        document: None,
        view: pdf_model::view::WindowView::default(),
        keys: pdf_model::view::Keys::default(),
        rich_value: "",
    }
}

#[test]
fn at_off_no_worker_is_started_and_each_script_is_reported() {
    let mut host = Host::open(document(SCRIPTS), Level::Off);
    assert!(host.type_into("Amount", "x") > 0, "nothing refused the x");
    host.commit("Amount");
    assert_eq!(host.displayed("Amount").as_deref(), Some("x"));
    assert_eq!(host.worker.spawns(), 0);
    assert!(
        host.view
            .script_reports()
            .iter()
            .all(|sentence| sentence.contains("a script this tier does not run")),
        "{:?}",
        host.view.script_reports()
    );
}

#[test]
fn at_on_the_first_trigger_starts_the_worker_and_the_scripts_run_in_it() {
    let mut host = Host::open(document(SCRIPTS), Level::On);
    assert_eq!(
        host.worker.spawns(),
        0,
        "opening the document started nothing"
    );

    assert_eq!(
        host.type_into("Amount", "x"),
        0,
        "the keystroke script, run in the worker, refused the x"
    );
    assert_eq!(
        host.worker.spawns(),
        1,
        "the first trigger started the worker"
    );
    assert!(host.type_into("Amount", "12") > 0);
    host.commit("Amount");
    assert_eq!(
        host.displayed("Amount").as_deref(),
        Some("<12>"),
        "the format script ran in the worker"
    );
    assert_eq!(
        host.worker.spawns(),
        1,
        "every later trigger used the same worker"
    );
    assert!(
        host.worker.deaths().is_empty(),
        "{:?}",
        host.worker.deaths()
    );

    let cost = host
        .worker
        .open_cost()
        .expect("the first run completed the open cost");
    println!("{}", cost.line());
    assert!(cost.spawn > Duration::ZERO && cost.first_run > Duration::ZERO);
}

/// The kernel granted what the profile asks for, on a kernel that has all three mechanisms.
#[cfg(target_os = "linux")]
#[test]
fn the_worker_reports_the_confinement_its_profile_asks_for() {
    let worker = ScriptWorker::with_program(WORKER);
    let _ = worker.run(&format_event("event.value = 'a';"));
    let confinement = worker.confinement().expect("a worker is running");
    assert!(confinement.is_enforced(), "{confinement:?}");
    assert_eq!(
        confinement.landlock,
        pdf_sandbox::lockdown::LandlockLevel::Enforced
    );
    assert_eq!(confinement.address_space_limit, 96 << 20);
}

/// The window's view crosses to the confined worker with the event, and a script's change to it
/// comes back as the edit a host carries out (ADR 1736).
#[test]
fn the_window_s_view_crosses_to_the_worker_and_a_change_to_it_comes_back() {
    use pdf_model::view::{ScriptEdit, ViewChange, WindowView, ZoomType};
    use pdf_model::viewer_preferences::PageLayout;
    let worker = ScriptWorker::with_program(WORKER);
    let result = worker.run(&ScriptEvent {
        view: WindowView {
            zoom: Some(125.0),
            zoom_type: ZoomType::FitWidth,
            layout: PageLayout::TwoColumnLeft,
        },
        ..format_event("event.value = [zoom, zoomType, layout].join('|'); zoom = 250;")
    });
    assert_eq!(
        result.value.as_deref(),
        Some("125|FitWidth|TwoColumnLeft"),
        "{result:?}"
    );
    assert_eq!(
        result.edits,
        vec![ScriptEdit::View {
            change: ViewChange::Zoom(250.0)
        }]
    );
}

/// The allocation a budget admits at face value runs under the ceiling, and a repeat past the
/// loop-iteration limit is stopped by name before Boa reserves room for it — which, unstopped,
/// was 384 MiB and the worker's abort (ADR 1609).
#[test]
fn what_the_budgets_admit_runs_and_what_they_refuse_is_named_not_fatal() {
    let worker = ScriptWorker::with_program(WORKER);
    let buffer = worker.run(&format_event(
        "var b = new ArrayBuffer(16777216); event.value = String(b.byteLength);",
    ));
    assert_eq!(buffer.value.as_deref(), Some("16777216"), "{buffer:?}");

    let repeat = worker.run(&format_event(
        "var s = 'x'.repeat(16777216); event.value = String(s.length);",
    ));
    assert_eq!(repeat.value, None);
    assert!(
        repeat
            .report
            .iter()
            .any(|sentence| sentence.contains("100000 iterations")),
        "{repeat:?}"
    );
    assert!(worker.deaths().is_empty(), "{:?}", worker.deaths());
    assert_eq!(worker.spawns(), 1);
}

/// A run the engine's budgets cannot see — a native call whose callback loops just under the
/// per-frame limit, a hundred thousand times — is killed at the deadline, named, and the next
/// trigger is run by another worker.
#[test]
fn a_run_past_its_deadline_is_killed_named_and_the_next_trigger_starts_another() {
    let worker = ScriptWorker::with_program(WORKER);
    let started = Instant::now();
    let lost = worker.run(&format_event(
        "var a = new Array(100000).fill(0); \
         a.map(function () { for (var i = 0; i < 99999; i++) {} return 0; }); \
         event.value = 'finished';",
    ));
    let spent = started.elapsed();
    assert_eq!(lost.value, None, "the field keeps its value");
    assert!(
        spent < DEADLINE * 4,
        "killed near the deadline, not after {spent:?}"
    );
    let deaths = worker.deaths();
    assert_eq!(deaths.len(), 1, "{deaths:?}");
    assert_eq!(deaths[0].cause, Cause::Deadline(DEADLINE));
    assert_eq!(deaths[0].site, ScriptSite::Field(Trigger::Format));
    assert_eq!(deaths[0].subject, "the format script of Total");
    assert!(
        lost.report
            .iter()
            .any(|sentence| sentence.contains("format script of Total")),
        "{lost:?}"
    );

    let next = worker.run(&format_event("event.value = 'again';"));
    assert_eq!(next.value.as_deref(), Some("again"), "{next:?}");
    assert_eq!(worker.spawns(), 2);
}

/// Growth through an operator, which no per-call budget sees, meets the ceiling: the worker aborts
/// with the allocator's sentence, the host names the trigger, and the next trigger is run.
#[cfg(target_os = "linux")]
#[test]
fn growth_past_the_ceiling_ends_the_worker_and_is_named() {
    let worker = ScriptWorker::with_program(WORKER);
    let lost = worker.run(&format_event(
        "var s = 'x'; for (var i = 0; i < 30; i++) { s += s; } event.value = String(s.length);",
    ));
    assert_eq!(lost.value, None, "{lost:?}");
    let deaths = worker.deaths();
    assert_eq!(deaths.len(), 1, "{deaths:?}");
    let Cause::Died(detail) = &deaths[0].cause else {
        unreachable!("the worker died rather than missing its deadline: {deaths:?}")
    };
    assert!(
        detail.contains("signal 6") && detail.contains("memory allocation"),
        "the abort and the allocator's sentence: {detail}"
    );
    let next = worker.run(&format_event("event.value = 'again';"));
    assert_eq!(next.value.as_deref(), Some("again"), "{next:?}");
}

/// After as many losses as a document is allowed, no worker is started again and every trigger is
/// told so, with the field as it was.
#[test]
fn after_its_losses_a_document_starts_no_further_worker() {
    let worker = ScriptWorker::with_program(WORKER).with_deadline(Duration::from_millis(50));
    let endless = "var a = new Array(100000).fill(0); \
                   a.map(function () { for (var i = 0; i < 99999; i++) {} return 0; });";
    for _ in 0..MAX_DEATHS {
        let _ = worker.run(&format_event(endless));
    }
    assert_eq!(worker.spawns(), MAX_DEATHS);
    let refused = worker.run(&format_event("event.value = 'never';"));
    assert_eq!(refused.value, None);
    assert!(
        refused
            .report
            .iter()
            .any(|sentence| sentence.starts_with("scripts stopped running for this document")),
        "{refused:?}"
    );
    assert_eq!(worker.spawns(), MAX_DEATHS, "no fifth worker");
}

/// A script crosses once per worker: a second run of the same text names it by index, and a worker
/// started after a loss is sent it again.
#[test]
fn a_script_crosses_once_per_worker() {
    let worker = ScriptWorker::with_program(WORKER);
    let script = "event.value = event.value + '!';";
    assert_eq!(
        worker.run(&format_event(script)).value.as_deref(),
        Some("12!")
    );
    assert_eq!(
        worker.run(&format_event(script)).value.as_deref(),
        Some("12!")
    );
    assert_eq!(worker.spawns(), 1);
}

/// What the engine reaches when a script uses its library: each script runs inside the filter and
/// none is the filter's kill.
///
/// The allow-list of `pdf_sandbox::lockdown::Profile::Script` is what a worker issued under
/// `strace` running these (ADR 1608); a call the engine needs and the list lacks shows here as a
/// lost worker rather than as a field that stopped formatting in somebody's form.
#[test]
fn the_engines_library_runs_inside_the_filter() {
    let worker = ScriptWorker::with_program(WORKER);
    let scripts = [
        "event.value = String(Math.random() < 1);",
        "event.value = new Date(2024, 0, 5).toISOString() + Date.now();",
        "event.value = '1,234.50'.replace(/[^0-9.]/g, '').match(/(\\d+)\\.(\\d+)/)[2];",
        "event.value = JSON.stringify(JSON.parse('{\"a\": [1, 2, {\"b\": null}]}'));",
        "var m = new Map(); var s = new Set(); for (var i = 0; i < 5000; i++) { m.set('k' + i, i); \
         s.add(i % 7); } event.value = String(m.size + s.size);",
        "var a = []; for (var i = 0; i < 20000; i++) a.push((i * 7919) % 10007); \
         a.sort(function (x, y) { return x - y; }); event.value = String(a[0]);",
        "function f(n) { return n < 2 ? n : f(n - 1) + f(n - 2); } event.value = String(f(18));",
        "try { null.x; } catch (e) { event.value = e.name; }",
        "console.println('logged'); event.value = 'logged';",
        "AFNumber_Format(2, 0, 0, 0, '$', true);",
        "var s = ''; for (var i = 0; i < 2000; i++) { s = s + String.fromCharCode(65 + i % 26); \
         s = s.substring(1) + s.charAt(0); } event.value = String(s.length);",
        "var t = new Float64Array(100000); for (var i = 0; i < t.length; i++) t[i] = Math.sqrt(i); \
         event.value = t[99999].toFixed(2);",
        "event.value = escape('ä b') + unescape('%41') + 'abc'.substr(1);",
        "throw new Error('thrown on purpose');",
        "this.getField('Elsewhere');",
        "app.launchURL('https://example.com');",
        "while (true) {}",
        "event.value = util.printd(0, new Date()) + util.printd('dddd mmmm d, yyyy h:MM tt', \
         new Date(2024, 0, 5)) + util.printx('>AAA-999', 'abc123');",
        "event.value = [app.viewerType, app.viewerVersion, app.platform, app.language].join();",
        "var f = this.getField('Total'); if (f) { f.getArray(); f.setFocus(); }",
    ];
    for script in scripts {
        let result = worker.run(&format_event(script));
        assert!(
            worker.deaths().is_empty(),
            "{script}: {:?}",
            worker.deaths()
        );
        assert!(
            result
                .report
                .iter()
                .all(|sentence| !sentence.contains("did not finish")),
            "{script}: {result:?}"
        );
    }
    assert_eq!(worker.spawns(), 1);
}

/// Table 200's will-save script runs in the worker at the moment the host marks, its edit lands in
/// the view state before the save, and its `event.rc` false is reported and not obeyed (ADR 1614).
#[test]
fn a_will_save_script_runs_in_the_worker_and_cannot_refuse_the_save() {
    let document = document_with(
        "",
        "/AA << /WS << /S /JavaScript /JS (this.getField\\('Amount'\\).value = \
         util.printx\\('9-9', '12'\\) + ' ' + event.name; event.rc = false;) >> >>",
    );
    let mut host = Host::open(document, Level::On);
    assert_eq!(host.worker.spawns(), 0);
    assert_eq!(
        host.view.run_document_scripts(
            &host.document,
            pdf_model::view::DocumentTrigger::WillSave,
            0
        ),
        1
    );
    assert_eq!(host.worker.spawns(), 1, "the trigger started the worker");
    assert_eq!(
        host.view
            .field_value(&host.document, "Amount")
            .map(|shown| shown.text)
            .as_deref(),
        Some("1-2 WillSave"),
        "{:?}",
        host.view.script_reports()
    );
    assert!(
        host.view
            .script_reports()
            .iter()
            .any(|sentence| sentence.contains("the save goes ahead")),
        "{:?}",
        host.view.script_reports()
    );
    assert!(host.view.save(&host.document).is_ok());
    assert!(
        host.worker.deaths().is_empty(),
        "{:?}",
        host.worker.deaths()
    );
}

/// A script nested deeper than the engine's parser can recurse is stopped by name before it is
/// parsed, and the worker lives: Boa 0.22's parser has no depth limit, and five hundred nested
/// parentheses overflowed the worker's 8 MiB main-thread stack in a release build (ADR 1609), so
/// `pdf_script`'s nesting budget refuses a script past it (ADR 1602).
#[test]
fn a_script_nested_past_the_parsers_depth_is_contained() {
    let worker = ScriptWorker::with_program(WORKER);
    let deep = format!("event.value = {}1{};", "(".repeat(5000), ")".repeat(5000));
    let result = worker.run(&format_event(&deep));
    assert_eq!(result.value, None, "{result:?}");
    assert!(
        result
            .report
            .iter()
            .any(|sentence| sentence.contains("brackets nest deeper than its budget")),
        "{result:?}"
    );
    assert!(worker.deaths().is_empty(), "{:?}", worker.deaths());
    let next = worker.run(&format_event("event.value = 'again';"));
    assert_eq!(next.value.as_deref(), Some("again"), "{next:?}");
    assert_eq!(worker.spawns(), 1);
}

/// A run the engine's own budgets stop answers inside the deadline, so the deadline kills only what
/// those budgets cannot see. Ten runs, each stopped by its wall or step budget, the slowest printed
/// for the record ADR 1609 quotes.
#[test]
fn a_run_the_engines_budgets_stop_answers_inside_the_deadline() {
    let worker = ScriptWorker::with_program(WORKER);
    let spinning = "function f() { for (var i = 0; i < 90000; i++) {} } for (;;) f();";
    let mut slowest = Duration::ZERO;
    for _ in 0..10 {
        let started = Instant::now();
        let result = worker.run(&format_event(spinning));
        slowest = slowest.max(started.elapsed());
        assert!(
            result
                .report
                .iter()
                .any(|sentence| sentence.starts_with("the script was stopped")),
            "{result:?}"
        );
    }
    println!(
        "a run the budgets stop: slowest of ten {} ms",
        slowest.as_millis()
    );
    assert!(worker.deaths().is_empty(), "{:?}", worker.deaths());
    assert!(slowest < DEADLINE, "{slowest:?}");
}

/// A validation that asks the person is held in its worker, the commit's view state carries on
/// without waiting, and the answer resumes the script: its refusal then puts back what the field
/// showed before the typing began (ADRs 1627, 1628).
#[test]
fn a_question_holds_its_script_in_the_worker_and_the_answer_resumes_it() {
    let document = document(
        "/V << /S /JavaScript /JS (if \\(app.alert\\('Keep ' + event.value + '?', 2, 2\\) == 3\\) \
         event.rc = false;) >> \
         /F << /S /JavaScript /JS (event.value = '<' + event.value + '>';) >>",
    );
    let mut host = Host::open(document, Level::On);
    host.type_into("Amount", "5");
    let started = Instant::now();
    host.commit("Amount");
    assert!(
        started.elapsed() < DEADLINE.saturating_mul(4),
        "the commit did not wait on the person"
    );
    let question = host.worker.take_question().expect("the script asked");
    assert_eq!(
        question,
        pdf_script::Question::Alert {
            message: "Keep 5?".to_owned(),
            icon: pdf_script::Icon::Question,
            buttons: pdf_script::Buttons::YesNo,
            title: None,
        }
    );
    assert_eq!(host.worker.take_question(), None, "handed over once");
    assert_eq!(
        host.view
            .field_value(&host.document, "Amount")
            .map(|shown| shown.text)
            .as_deref(),
        Some("5"),
        "the value stands while the script waits"
    );
    host.worker
        .answer(pdf_script::Answer::Pressed(pdf_script::Button::No));
    assert!(host.view.apply_resumed(&host.document));
    assert_eq!(
        host.view
            .field_value(&host.document, "Amount")
            .map(|shown| shown.text)
            .as_deref(),
        Some(""),
        "{:?}",
        host.view.script_reports()
    );
    assert!(
        host.view
            .script_reports()
            .iter()
            .any(|sentence| sentence.contains("once its question was answered")),
        "{:?}",
        host.view.script_reports()
    );
    assert!(
        host.worker.deaths().is_empty(),
        "{:?}",
        host.worker.deaths()
    );
    assert_eq!(host.worker.spawns(), 1);
}

/// Triggers handed over while a script waits are queued behind it and run, in order, once it has
/// its answer; a question nobody answers is withdrawn after its wait, and answered as a closed
/// dialogue answers.
#[test]
fn triggers_wait_behind_a_question_and_a_question_nobody_answers_is_withdrawn() {
    let worker = ScriptWorker::with_program(WORKER).with_answer_wait(Duration::from_millis(200));
    let asked = worker.run(&format_event(
        "global.n = app.response('Name?'); event.value = String(global.n);",
    ));
    assert!(worker.waiting(), "{asked:?}");
    assert_eq!(asked.value, None);
    let queued = worker.run(&format_event("event.value = 'after ' + global.n;"));
    assert!(worker.waiting(), "{queued:?}");
    assert!(worker.take_resumed().is_none());
    assert!(
        worker.take_question().is_some(),
        "the host puts the question"
    );
    std::thread::sleep(Duration::from_millis(300));
    assert!(worker.question_withdrawn());
    assert!(!worker.question_withdrawn(), "said once");
    let first = worker.take_resumed().expect("the held run finished");
    assert_eq!(first.result.value.as_deref(), Some("null"), "{first:?}");
    assert!(
        first.result.report[0].contains("was not answered within 200 ms"),
        "{first:?}"
    );
    let second = worker.take_resumed().expect("the queued run ran");
    assert_eq!(
        second.result.value.as_deref(),
        Some("after null"),
        "{second:?}"
    );
    assert!(worker.take_resumed().is_none());
    // An answer that comes after the wait finds nothing waiting, and is kept nowhere.
    worker.answer(pdf_script::Answer::Typed(Some("late".to_owned())));
    assert!(worker.take_resumed().is_none());
    assert!(worker.deaths().is_empty(), "{:?}", worker.deaths());
}

/// The text a response question asks.
fn asked(question: &pdf_script::Question) -> &str {
    match question {
        pdf_script::Question::Response { question, .. } => question,
        pdf_script::Question::Alert { message, .. } => message,
    }
}

/// A card is dropped once its question's wait runs out, and a press on it that arrives after is
/// never handed to the question a queued script asked since, which nobody has read (ADR 1641).
#[test]
fn a_late_answer_is_never_handed_to_the_question_asked_after_it() {
    let worker = ScriptWorker::with_program(WORKER).with_answer_wait(Duration::from_millis(200));
    worker.run(&format_event(
        "event.value = String(app.response('First?'));",
    ));
    worker.run(&format_event(
        "event.value = String(app.response('Second?'));",
    ));
    assert!(
        worker.question_deadline().is_none(),
        "no host took a question"
    );
    let first = worker.take_question().expect("the first script asked");
    assert_eq!(asked(&first), "First?");
    let deadline = worker
        .question_deadline()
        .expect("a taken question has a deadline");
    assert!(deadline > Instant::now(), "{deadline:?}");
    std::thread::sleep(Duration::from_millis(300));

    // The first card's answer, after its wait ran out and before the host polled again.
    worker.answer(pdf_script::Answer::Typed(Some("late".to_owned())));
    let withdrawn = worker
        .take_resumed()
        .expect("the first run was answered for it");
    assert_eq!(
        withdrawn.result.value.as_deref(),
        Some("null"),
        "{withdrawn:?}"
    );
    assert!(worker.take_resumed().is_none(), "the second still waits");
    assert!(worker.question_withdrawn(), "the first card is dropped");
    assert!(
        worker.question_deadline().is_none(),
        "the second is not taken"
    );

    let second = worker.take_question().expect("the queued script asked");
    assert_eq!(asked(&second), "Second?");
    worker.answer(pdf_script::Answer::Typed(Some("on time".to_owned())));
    let answered = worker.take_resumed().expect("the second run finished");
    assert_eq!(
        answered.result.value.as_deref(),
        Some("on time"),
        "{answered:?}"
    );
    assert!(
        !worker.question_withdrawn(),
        "an answered question is not withdrawn"
    );
    assert!(worker.deaths().is_empty(), "{:?}", worker.deaths());
}

/// A chain of operators with no bracket — twenty thousand terms overflowed the worker's 8 MiB
/// stack at about eleven thousand — is stopped by name before it is parsed, and the worker lives
/// (ADR 1626).
#[test]
fn an_expression_past_the_depth_budget_is_stopped_and_the_worker_lives() {
    let worker = ScriptWorker::with_program(WORKER);
    for deep in [
        format!("event.value = 1{};", "+1".repeat(20_000)),
        format!("event.value = {}1;", "!".repeat(5_000)),
    ] {
        let result = worker.run(&format_event(&deep));
        assert!(
            result
                .report
                .iter()
                .any(|sentence| sentence.contains("nest so deep")),
            "{result:?}"
        );
    }
    assert!(worker.deaths().is_empty(), "{:?}", worker.deaths());
    assert_eq!(worker.spawns(), 1);
}

/// A worker whose input ends exits cleanly: status 0, no signal, so no core (ADR 1627).
#[test]
fn a_worker_whose_input_ends_exits_zero() {
    use std::io::Read as _;
    let mut child = std::process::Command::new(WORKER)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("the worker starts");
    drop(child.stdin.take());
    let mut greeting = Vec::new();
    if let Some(mut stdout) = child.stdout.take() {
        stdout.read_to_end(&mut greeting).expect("its output reads");
    }
    let status = child.wait().expect("it ends");
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stderr.take() {
        let _ = pipe.read_to_string(&mut stderr);
    }
    assert!(!greeting.is_empty(), "it greeted before it read");
    assert!(status.success(), "{status:?}: {stderr}");
}
