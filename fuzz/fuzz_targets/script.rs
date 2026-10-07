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
//! three bits choose the site (the four field triggers, an annotation's, a page's, the open action,
//! the document's library), its top bit `willCommit`. The realm is told of four fields, so
//! `this.getField` and a field's properties are reached. A fresh realm is built for every input, so
//! that a crash is the input's alone.
//!
//! Beyond never panicking — overflow checks stay on in this profile — three properties:
//!
//! - **The budgets hold.** A run returns within [`ESCAPED`]: eight times the worker's deadline,
//!   which a run the engine's own budgets stop answers well inside (ADR 1609). A run past it is one
//!   the worker would have killed, and is a budget the engine did not enforce.
//! - **A run that did not finish changed nothing**: `rc` true, no value, no change, no edit.
//! - **The outcome crosses the wire as it is**, which is what the host reads.
//!
//! # What is looked past, and why
//!
//! Boa 0.22's parser has no depth limit, and five hundred nested parentheses overflowed an 8 MiB
//! stack (ADR 1609). In the worker that is a lost worker, named by its trigger and replaced, and
//! `pdf-script-worker`'s `a_script_nested_past_the_parsers_depth_is_contained` is its test. So the
//! run happens on a thread with [`STACK`] of stack, which no script within the campaign's
//! `-max_len` can nest past except by brackets, and a script whose brackets nest past
//! [`NESTING`] is not run: what the target is for is everything else.

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
use pdf_model::view::{
    Alignment, BorderStyle, Colour, Display, FieldState, FieldType, ScriptEvent, ScriptSite,
};
use pdf_script::{Budget, Ending, Realm, Request, wire};

/// The sites the selector chooses among.
const SITES: [ScriptSite; 8] = [
    ScriptSite::Field(Trigger::Keystroke),
    ScriptSite::Field(Trigger::Format),
    ScriptSite::Field(Trigger::Validate),
    ScriptSite::Field(Trigger::Calculate),
    ScriptSite::Annotation(AnnotationTrigger::Up),
    ScriptSite::Page(PageTrigger::Open),
    ScriptSite::OpenAction,
    ScriptSite::Library,
];

/// How long a run may take before it is a budget the engine did not enforce: eight times the
/// worker's deadline per trigger.
const ESCAPED: Duration = Duration::from_secs(2);

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
        display: Display::Visible,
        text_color: Some(Colour::Gray(0.0)),
        fill_color: Some(Colour::Rgb([1.0, 1.0, 0.5])),
        stroke_color: None,
        border_style: BorderStyle::Solid,
        alignment: Alignment::Left,
        char_limit: Some(12),
        page: Some(0),
        rect: [10.0, 10.0, 210.0, 40.0],
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
    ];
    let site = SITES[usize::from(selector & 0x07)];
    let event = ScriptEvent {
        site,
        field: "Amount",
        label: "Library",
        script: &script,
        value: &value,
        change: &change,
        selection: (0, value.len()),
        will_commit: selector & 0x80 != 0,
        source: "Total",
        fields: &fields,
        page: 0,
        pages: 3,
    };
    let request = Request::of(&event, 1_704_465_015_000, 0);

    let started = Instant::now();
    let outcome = match Realm::new(Budget::FIELD_EVENT) {
        Ok(mut realm) => realm.run(&request),
        Err(why) => panic!("a realm could not be constructed: {why}"),
    };
    let spent = started.elapsed();
    assert!(
        spent < ESCAPED,
        "a run took {spent:?}, past every budget the engine holds a run to"
    );

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
