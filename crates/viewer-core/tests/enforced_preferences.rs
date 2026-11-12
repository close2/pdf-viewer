//! ISO 32000-2 §12.2's `/Enforce`, a document restriction at the reader's four levels.
//!
//! Table 147's `/Enforce` is "\[a]n array of names of Viewer preference settings that shall be
//! enforced by PDF processors and that shall not be overridden by subsequent selections in the
//! application user interface", and Table 148 lets it name `/PrintScaling` only where that entry
//! states a valid value other than `AppDefault`. A document telling its reader what they may not choose is a restriction on the
//! reader, so `CLAUDE.md` makes obeying it the reader's level for
//! `pdf_model::restriction::Operation::OverridePreference`, default `warn` — the owner's answer
//! `doc/questions/A834`, item 2, and ADR 2840. What a level decides is whether the print grant
//! holds the dialogue's scaling to the document's; no level stops the print, because the
//! operation the entry restricts is the reader's choice in the dialogue, not printing.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "test code: a fixture that cannot be built must fail loudly, and its offsets are \
              within a fixture written in this file"
)]

use std::fmt::Write as _;

use pdf_model::restriction::Operation;
use viewer_core::{
    Command, DocumentId, Event, Fidelity, Printing, RestrictionLevel, RestrictionPolicy,
    RestrictionScope, Sheet, Viewer,
};

const DOCUMENT: DocumentId = DocumentId(1);

/// A one-page document whose `/ViewerPreferences` are `preferences`.
fn document(preferences: &str) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /ViewerPreferences << {preferences} >> >>\n\
         endobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> \
         /Contents 4 0 R >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n"
    );
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let size = offsets.len() + 1;
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

/// The document that enforces its `/PrintScaling /None` (Table 148's condition met).
const ENFORCING: &str = "/PrintScaling /None /Enforce [/PrintScaling]";

/// A viewer at `policy` with `preferences`' document open, the print asked for, and the events
/// the request raised.
fn printing(policy: Option<RestrictionPolicy>, preferences: &str) -> (Viewer, Vec<Event>) {
    let mut viewer = Viewer::new(100, 100, 1.0);
    if let Some(policy) = policy {
        viewer
            .handle(Command::Restrict(RestrictionScope::Window(policy)))
            .for_each(drop);
    }
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: document(preferences).into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    let sheet = Sheet {
        media: Some([0.0, 0.0, 595.0, 842.0]),
        scale: 300.0 / 72.0,
        page_scale: 1.0,
    };
    let events = viewer
        .handle(Command::Print(Printing::Start(sheet)))
        .collect();
    (viewer, events)
}

/// What the grant said of the scaling, where a print was granted.
fn held(events: &[Event]) -> Option<bool> {
    events.iter().find_map(|event| match event {
        Event::Printing { scaling_held, .. } => Some(*scaling_held),
        _ => None,
    })
}

/// How the events speak of `/Enforce`'s operation.
fn about(events: &[Event]) -> Vec<&'static str> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Refused { operation, .. } if *operation == Operation::OverridePreference => {
                Some("refused")
            }
            Event::Asking { operation, .. } if *operation == Operation::OverridePreference => {
                Some("asking")
            }
            Event::Warned { operation, .. } if *operation == Operation::OverridePreference => {
                Some("warned")
            }
            _ => None,
        })
        .collect()
}

/// The default is the owner's `warn`: the print goes ahead, the scaling is the reader's to
/// change, and what the document asked is said.
#[test]
fn by_default_an_enforced_scaling_is_warned_of_and_left_to_the_reader() {
    assert_eq!(
        RestrictionPolicy::default().level(Operation::OverridePreference),
        RestrictionLevel::Warn
    );
    assert_eq!(
        RestrictionPolicy::default().level(Operation::Print),
        RestrictionLevel::Off,
        "every other operation stays off by default"
    );
    let (_, events) = printing(None, ENFORCING);
    assert_eq!(held(&events), Some(false));
    assert_eq!(about(&events), ["warned"]);
}

/// `on` keeps Table 147's `shall`: the scaling is held to the document's and the reader is told
/// why; the print itself is granted.
#[test]
fn at_on_the_scaling_is_held_to_the_documents() {
    let policy =
        RestrictionPolicy::default().with(Operation::OverridePreference, RestrictionLevel::On);
    let (_, events) = printing(Some(policy), ENFORCING);
    assert_eq!(held(&events), Some(true));
    assert_eq!(about(&events), ["refused"]);
}

/// `off` asks nothing and says nothing.
#[test]
fn at_off_nothing_is_held_or_said() {
    let policy =
        RestrictionPolicy::default().with(Operation::OverridePreference, RestrictionLevel::Off);
    let (_, events) = printing(Some(policy), ENFORCING);
    assert_eq!(held(&events), Some(false));
    assert!(about(&events).is_empty());
}

/// `ask` holds the print on the question, and either answer starts it: a `no` holds the scaling.
#[test]
fn at_ask_either_answer_prints_and_a_no_holds_the_scaling() {
    let policy =
        RestrictionPolicy::default().with(Operation::OverridePreference, RestrictionLevel::Ask);
    for (proceed, expected) in [(true, false), (false, true)] {
        let (mut viewer, events) = printing(Some(policy), ENFORCING);
        assert_eq!(about(&events), ["asking"]);
        assert_eq!(held(&events), None, "nothing starts until it is answered");
        let answered: Vec<Event> = viewer
            .handle(Command::Answer {
                document: DOCUMENT,
                proceed,
            })
            .collect();
        assert_eq!(held(&answered), Some(expected), "proceed = {proceed}");
        assert!(answered.iter().any(|event| matches!(
            event,
            Event::Printing {
                fidelity: Fidelity::Faithful,
                ..
            }
        )));
    }
}

/// Table 148's condition: `/Enforce` may name `/PrintScaling` "only if" it states a valid value
/// other than `AppDefault`, so a document whose scaling is `AppDefault` restricts nothing, at
/// any level.
#[test]
fn an_enforce_over_app_default_restricts_nothing() {
    let policy =
        RestrictionPolicy::default().with(Operation::OverridePreference, RestrictionLevel::On);
    let (_, events) = printing(
        Some(policy),
        "/PrintScaling /AppDefault /Enforce [/PrintScaling]",
    );
    assert_eq!(held(&events), Some(false));
    assert!(about(&events).is_empty());
}
