//! Writes the `script_wire` fuzz target's seeds: every shape a run and a reply cross the worker's
//! wire in, encoded by the encoders the host and the worker use, so the seeds follow the format
//! when it moves.
//!
//! ```text
//! cargo run -p pdf-script-worker --example wire_seeds -- <directory>
//! ```
//!
//! A run is written as it crosses, starting with its version byte; a reply as its frame kind
//! followed by its payload, which is how the target reads one (`fuzz/fuzz_targets/script_wire.rs`).
//! Each seed is named by the shape it holds.

#![expect(clippy::print_stdout, reason = "an example prints what it wrote")]

use std::time::Duration;

use pdf_model::action::{PageTrigger, Trigger as AnnotationTrigger};
use pdf_model::aform::Trigger;
use pdf_model::view::{
    Alignment, BorderStyle, Colour, Display, FieldState, FieldType, Property, ScriptEdit,
    ScriptEvent, ScriptSite,
};
use pdf_script::{Ending, Exceeded, Outcome, Refusal, RefusalKind, Request};
use pdf_script_worker::wire::{MAX_SCRIPT_BYTES, Reply, Run, encode_reply, encode_run};

/// A field as a realm is told of one.
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

/// A request at `site`, with `fields` told and the script left out, as a run carries one.
fn request(site: ScriptSite, fields: &[FieldState], value: &str, change: &str) -> Request {
    let event = ScriptEvent {
        site,
        field: if matches!(site, ScriptSite::Library | ScriptSite::OpenAction) {
            ""
        } else {
            "Total"
        },
        label: if site == ScriptSite::Library {
            "Library"
        } else {
            ""
        },
        script: "",
        value,
        change,
        selection: (0, value.len()),
        will_commit: change.is_empty(),
        source: if site == ScriptSite::Field(Trigger::Calculate) {
            "Amount"
        } else {
            ""
        },
        fields,
        page: 0,
        pages: 3,
    };
    Request::of(&event, 1_704_465_015_000, -3600)
}

/// A run at every site, with a script crossing and without, and one carrying a script at the
/// wire's bound.
fn runs() -> Vec<(String, Vec<u8>)> {
    let fields = [
        field("Total", FieldType::Text, "12.50"),
        field("Amount", FieldType::Text, "3"),
        field("Agree", FieldType::CheckBox, "Off"),
        field("Choice", FieldType::ComboBox, "é"),
    ];
    let sites = [
        ("keystroke", ScriptSite::Field(Trigger::Keystroke)),
        ("format", ScriptSite::Field(Trigger::Format)),
        ("validate", ScriptSite::Field(Trigger::Validate)),
        ("calculate", ScriptSite::Field(Trigger::Calculate)),
        ("annotation", ScriptSite::Annotation(AnnotationTrigger::Up)),
        ("page", ScriptSite::Page(PageTrigger::Open)),
        ("open-action", ScriptSite::OpenAction),
        ("library", ScriptSite::Library),
    ];
    let mut seeds: Vec<(String, Vec<u8>)> = Vec::new();
    for (name, site) in sites {
        seeds.push((
            format!("run-{name}-new-script-all-fields"),
            encode_run(&Run {
                new_script: Some((0, "event.value = '<' + event.value + '>';".to_owned())),
                script: 0,
                request: request(site, &fields, "12", "x"),
            }),
        ));
        seeds.push((
            format!("run-{name}-held-script-no-fields"),
            encode_run(&Run {
                new_script: None,
                script: 3,
                request: request(site, &[], "", ""),
            }),
        ));
    }
    seeds.push((
        "run-script-at-the-bound".to_owned(),
        encode_run(&Run {
            new_script: Some((u32::MAX, "x".repeat(MAX_SCRIPT_BYTES))),
            script: u32::MAX,
            request: request(ScriptSite::Field(Trigger::Format), &fields[..1], "1", ""),
        }),
    ));
    seeds
}

/// A reply of every ending, with every refusal kind and an edit of each kind, and a refusal.
fn replies() -> Vec<(String, Vec<u8>)> {
    let mut seeds = Vec::new();
    let refusals = vec![
        Refusal {
            member: "app.launchURL".to_owned(),
            kind: RefusalKind::Excluded("leaves the machine".to_owned()),
        },
        Refusal {
            member: "event.commitKey".to_owned(),
            kind: RefusalKind::NotBridged,
        },
        Refusal {
            member: "this.getField(\"Elsewhere\")".to_owned(),
            kind: RefusalKind::Unreachable("no such field".to_owned()),
        },
        Refusal {
            member: "AFNumber_Format".to_owned(),
            kind: RefusalKind::Library("two arguments".to_owned()),
        },
    ];
    let edits = vec![
        ScriptEdit::Value {
            field: "Amount".to_owned(),
            value: "4".to_owned(),
        },
        ScriptEdit::Property {
            field: "Total".to_owned(),
            property: Property::FillColor(Colour::Cmyk([0.0, 0.1, 0.2, 0.3])),
        },
    ];
    let endings = [
        ("finished", Ending::Finished),
        (
            "wall",
            Ending::Exceeded(Exceeded::Wall(Duration::from_millis(100))),
        ),
        ("steps", Ending::Exceeded(Exceeded::Steps(40_000_000))),
        ("loops", Ending::Exceeded(Exceeded::LoopIterations(100_000))),
        ("recursion", Ending::Exceeded(Exceeded::Recursion(512))),
        ("stack", Ending::Exceeded(Exceeded::Stack(10_240))),
        (
            "elements",
            Ending::Exceeded(Exceeded::Elements {
                asked: 5_000_000,
                ceiling: 1 << 20,
            }),
        ),
        (
            "units",
            Ending::Exceeded(Exceeded::StringUnits {
                asked: 1 << 30,
                ceiling: 1 << 24,
            }),
        ),
        ("threw", Ending::Threw("TypeError: null".to_owned())),
        ("unparsed", Ending::Unparsed("unexpected token".to_owned())),
        ("declined", Ending::Declined("not run".to_owned())),
    ];
    for (name, ending) in endings {
        let outcome = Outcome {
            rc: name != "finished",
            value: Some("$12.50".to_owned()),
            change: (name == "finished").then(|| "1".to_owned()),
            edits: if name == "finished" {
                edits.clone()
            } else {
                Vec::new()
            },
            ending,
            refusals: refusals.clone(),
            log: vec!["one".to_owned(), "two".to_owned()],
        };
        let (kind, payload) = encode_reply(&Reply::Outcome(outcome));
        let mut bytes = vec![kind];
        bytes.extend_from_slice(&payload);
        seeds.push((format!("reply-outcome-{name}"), bytes));
    }
    let (kind, payload) = encode_reply(&Reply::Refused(
        "the run names script 3, which this worker does not hold".to_owned(),
    ));
    let mut bytes = vec![kind];
    bytes.extend_from_slice(&payload);
    seeds.push(("reply-refused".to_owned(), bytes));
    seeds
}

fn main() {
    let Some(directory) = std::env::args_os().nth(1) else {
        eprintln!("usage: wire_seeds <directory>");
        std::process::exit(2);
    };
    let directory = std::path::PathBuf::from(directory);
    if let Err(error) = std::fs::create_dir_all(&directory) {
        eprintln!("wire_seeds: {}: {error}", directory.display());
        std::process::exit(1);
    }
    let mut written = 0usize;
    for (name, bytes) in runs().iter().chain(&replies()) {
        match std::fs::write(directory.join(name), bytes) {
            Ok(()) => written = written.saturating_add(1),
            Err(error) => eprintln!("wire_seeds: {name}: {error}"),
        }
    }
    println!("wire_seeds: {written} seeds in {}", directory.display());
}
