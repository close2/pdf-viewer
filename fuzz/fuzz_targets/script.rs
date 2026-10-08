//! Fuzzes `pdf_script`'s engine: a document's script run in a realm at one of §12.6.3's sites, held
//! to `pdf_script::Budget::FIELD_EVENT` (RFC 0008 section 6.7, ADRs 1590, 1609).
//!
//! **The script is the most programmable input this tree reads**, and every byte of it is the
//! producer's. The engine runs in `pdf-script-worker`, behind the narrowest confinement in the tree;
//! this target runs the very function the worker runs, `pdf_script::Realm::run`, in process, so that
//! what the confinement would contain is found here as a crash rather than as a lost worker nobody
//! reads about.
//!
//! The input is a selector byte and three texts separated by NUL bytes — the script, the event's
//! value and the keystroke's change — read lossily so that every input is one. The selector's low
//! four bits choose the site (the four field triggers, two of an annotation's, a page's open and
//! close, the open action, the document's library, and Table 200's five), bit 4 the unsaved mark,
//! bit 5 a commit by Enter with a full field, bit 6 whether a question is answered — a button or
//! the change as typed text — or answered by nobody, and the top bit `willCommit`. The realm is
//! told of five fields and of the document as a whole — an information dictionary and two groups,
//! one locked — so `this.getField`, a field's properties, `this.info` and `this.getOCGs` are
//! reached (ADRs 1626, 1627). A fresh realm is built for every input, so that a crash is the
//! input's alone.
//!
//! Beyond never panicking — overflow checks stay on in this profile — three properties:
//!
//! - **The budgets hold.** A run spends less than [`ESCAPED`] on a processor: eight times the
//!   worker's deadline, which a run the engine's own budgets stop answers well inside (ADR 1609). A
//!   run past it is one the worker would have killed, and is a budget the engine did not enforce.
//!   The time is the run's thread's own and not the wall clock's, because a campaign runs at nice
//!   19 on a machine its siblings are building on, and a run that waited seconds for a
//!   processor enforced every budget it had (ADR 1717); a run that blocks is libFuzzer's
//!   `-timeout`, which is a wall clock.
//! - **A run that did not finish changed nothing**: `rc` true, no value, no change, no edit.
//! - **The outcome crosses the wire as it is**, which is what the host reads.
//!
//! # What is looked past, and why
//!
//! Boa 0.22's parser and compiler have no depth limit, and the realm stops a script its bracket
//! budget or its depth estimate says would overflow before either sees it (ADRs 1602, 1626). The
//! run still happens on a thread with [`STACK`] of stack, eight times the worker's, so that an
//! estimate that undercounts is found as a run that should have been stopped rather than as a lost
//! fuzzer; and a script whose brackets nest past [`NESTING`] is not run, the realm's own bound being
//! below it.

#![no_main]
#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a fuzz target states its properties by failing: `expect` and `panic!` are how a violated one reaches libFuzzer, and each message names the property"
)]

use std::time::{Duration, Instant};

use libfuzzer_sys::fuzz_target;
use pdf_model::action::{PageTrigger, Trigger as AnnotationTrigger};
use pdf_model::aform::Trigger;
use std::rc::Rc;

use pdf_model::view::{
    Alignment, BorderStyle, Colour, CommitKey, Display, DocumentState, DocumentTrigger, FieldState,
    FieldType, InfoEntry, Layer, ScriptEvent, ScriptSite, WidgetState,
};
use pdf_script::{Answer, Asker, Budget, Button, Ending, Nobody, Question, Realm, Request, wire};

/// The sites the selector chooses among; a keystroke twice, the commonest site there is.
const SITES: [ScriptSite; 16] = [
    ScriptSite::Field(Trigger::Keystroke),
    ScriptSite::Field(Trigger::Format),
    ScriptSite::Field(Trigger::Validate),
    ScriptSite::Field(Trigger::Calculate),
    ScriptSite::Annotation(AnnotationTrigger::Up),
    ScriptSite::Annotation(AnnotationTrigger::Focus),
    ScriptSite::Page(PageTrigger::Open),
    ScriptSite::Page(PageTrigger::Close),
    ScriptSite::OpenAction,
    ScriptSite::Library,
    ScriptSite::Document(DocumentTrigger::WillClose),
    ScriptSite::Document(DocumentTrigger::WillSave),
    ScriptSite::Document(DocumentTrigger::DidSave),
    ScriptSite::Document(DocumentTrigger::WillPrint),
    ScriptSite::Document(DocumentTrigger::DidPrint),
    ScriptSite::Field(Trigger::Keystroke),
];

/// An asker that answers every question: an alert with Yes, a response with the input's change.
#[derive(Debug)]
struct Answering(String);

impl Asker for Answering {
    fn ask(&self, question: &Question) -> Answer {
        match question {
            Question::Alert { .. } => Answer::Pressed(Button::Yes),
            Question::Response { .. } => Answer::Typed(Some(self.0.clone())),
        }
    }
}

/// The document as a whole the realm is told of.
fn document() -> DocumentState {
    let layer = |number, name: &str, locked| Layer {
        number,
        generation: 0,
        name: name.to_owned(),
        on: true,
        initially_on: true,
        locked,
    };
    DocumentState {
        info: vec![
            InfoEntry {
                key: "Title".to_owned(),
                text: "Form".to_owned(),
                moment: None,
            },
            InfoEntry {
                key: "ModDate".to_owned(),
                text: "D:20240105143015Z".to_owned(),
                moment: Some(1_704_465_015_000),
            },
        ],
        layers: vec![layer(7, "Watermark", false), layer(8, "English", true)],
        annotations: Vec::new(),
    }
}

/// How long a run may spend on a processor before it is a budget the engine did not enforce: eight
/// times the worker's deadline per trigger.
const ESCAPED: Duration = Duration::from_secs(2);

/// This thread's time on a processor and its time waiting in the run queue for one, the first two
/// fields of `/proc/thread-self/schedstat` in nanoseconds; `None` where the kernel offers no such
/// file, and the run is then judged by the wall clock, which the property's message says.
fn on_processor() -> Option<(Duration, Duration)> {
    let stat = std::fs::read_to_string("/proc/thread-self/schedstat").ok()?;
    let mut fields = stat.split_whitespace().map(str::parse::<u64>);
    let ran = fields.next()?.ok()?;
    let waited = fields.next()?.ok()?;
    Some((Duration::from_nanos(ran), Duration::from_nanos(waited)))
}

/// The stack the run is given: eight times the worker's main thread's, so that what overflows here
/// is not what the worker's own stack would.
const STACK: usize = 64 << 20;

/// Deepest bracket nesting a script is run with; see the module's last section.
const NESTING: usize = 256;

/// A field as the realm is told of one.
fn field(name: &str, kind: FieldType, value: &str) -> FieldState {
    FieldState {
        name: name.to_owned(),
        kind,
        value: value.to_owned(),
        flags: 0,
        char_limit: Some(12),
        page: Some(0),
        widgets: vec![WidgetState {
            display: Display::Visible,
            text_color: Some(Colour::Gray(0.0)),
            fill_color: Some(Colour::Rgb([1.0, 1.0, 0.5])),
            stroke_color: None,
            border_style: BorderStyle::Solid,
            alignment: Alignment::Left,
            rect: [10.0, 10.0, 210.0, 40.0],
            captions: ["Send".to_owned(), String::new(), String::new()],
            on_state: None,
        }],
    }
}

/// The deepest the script's brackets nest, counted over every byte: a bracket inside a string is
/// counted too, which errs towards not running a script rather than towards overflowing.
fn nesting(script: &str) -> usize {
    let (mut depth, mut deepest) = (0usize, 0usize);
    for byte in script.bytes() {
        match byte {
            b'(' | b'[' | b'{' => {
                depth = depth.saturating_add(1);
                deepest = deepest.max(depth);
            }
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    deepest
}

/// Runs one input and checks the three properties.
fn run(data: &[u8]) {
    let Some((&selector, rest)) = data.split_first() else {
        return;
    };
    let mut parts = rest.splitn(3, |&byte| byte == 0);
    let script = String::from_utf8_lossy(parts.next().unwrap_or_default());
    if nesting(&script) > NESTING {
        return;
    }
    let value = String::from_utf8_lossy(parts.next().unwrap_or_default());
    let change = String::from_utf8_lossy(parts.next().unwrap_or_default());
    let fields = [
        field("Total", FieldType::Text, "12.50"),
        field("Amount", FieldType::Text, &value),
        field("Agree", FieldType::CheckBox, "Off"),
        field("Choice", FieldType::ComboBox, "B"),
        field("Send", FieldType::PushButton, ""),
    ];
    let site = SITES[usize::from(selector & 0x0F)];
    let committed = selector & 0x20 != 0;
    let whole = document();
    let event = ScriptEvent {
        site,
        field: "Amount",
        label: "Library",
        script: &script,
        value: &value,
        change: &change,
        selection: (0, value.len()),
        will_commit: selector & 0x80 != 0,
        commit_key: committed.then_some(CommitKey::Enter),
        field_full: committed,
        change_ex: &value,
        source: "Total",
        fields: &fields,
        page: 0,
        pages: 3,
        dirty: selector & 0x10 != 0,
        document: Some(&whole),
    };
    let request = Request::of(&event, 1_704_465_015_000, 0);

    let started = Instant::now();
    let before = on_processor();
    let asker: Rc<dyn Asker> = if selector & 0x40 != 0 {
        Rc::new(Answering(change.clone().into_owned()))
    } else {
        Rc::new(Nobody)
    };
    let outcome = match Realm::with_asker(Budget::FIELD_EVENT, asker) {
        Ok(mut realm) => realm.run(&request),
        Err(why) => panic!("a realm could not be constructed: {why}"),
    };
    let wall = started.elapsed();
    match before.zip(on_processor()) {
        Some(((ran_before, waited_before), (ran_after, waited_after))) => {
            let spent = ran_after.saturating_sub(ran_before);
            let waited = waited_after.saturating_sub(waited_before);
            assert!(
                spent < ESCAPED,
                "a run spent {spent:?} on a processor ({wall:?} of wall time, {waited:?} of it \
                 waiting for one), past every budget the engine holds a run to"
            );
        }
        None => assert!(
            wall < ESCAPED,
            "a run took {wall:?} of wall time, measured by the wall clock because this kernel \
             offers no /proc/thread-self/schedstat, past every budget the engine holds a run to"
        ),
    }

    if outcome.ending != Ending::Finished {
        assert!(
            outcome.rc
                && outcome.value.is_none()
                && outcome.change.is_none()
                && outcome.edits.is_empty(),
            "a run that ended {:?} changed something: {outcome:?}",
            outcome.ending
        );
    }

    let crossed = wire::decode_outcome(&wire::encode_outcome(&outcome))
        .expect("an outcome the engine made crosses the wire");
    assert_eq!(
        format!("{outcome:?}"),
        format!("{crossed:?}"),
        "an outcome changed on its way across the wire"
    );
}

fuzz_target!(|data: &[u8]| {
    let data = data.to_vec();
    let ran = std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(move || run(&data))
        .expect("the run's thread starts")
        .join();
    if let Err(panic) = ran {
        std::panic::resume_unwind(panic);
    }
});
