//! A genuine signed `/Lock` reaches the public edit boundary under the reader's On policy.
//! Policy Off remains an explicit reader override (ADR 0284), not a signature-validity claim.
#![expect(
    clippy::panic,
    clippy::expect_used,
    reason = "fixture failures must fail the test"
)]

use pdf_model::restriction::Operation;
use viewer_core::{
    Answer, Command, DocumentId, Edit, Entered, Event, Query, RestrictionLevel, RestrictionPolicy,
    RestrictionScope, Viewer,
};

const SIGNED: &[u8] =
    include_bytes!("../../pdf-model/tests/fixtures/signed-field-lock/signed-lock.pdf");
const UNSIGNED: &[u8] =
    include_bytes!("../../pdf-model/tests/fixtures/signed-field-lock/unsigned-lock.pdf");

fn open(bytes: &[u8], level: RestrictionLevel) -> Viewer {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Restrict(RestrictionScope::Window(
            RestrictionPolicy::default().with(Operation::FillInForm, level),
        )))
        .for_each(drop);
    viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.to_vec().into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer
}

fn checked(viewer: &Viewer, name: &str) -> bool {
    let Answer::Fields(fields) = viewer.query(Query::Fields) else {
        panic!("fields answer")
    };
    let field = fields
        .iter()
        .find(|field| field.name.qualified == name)
        .expect("named field");
    assert_eq!(field.widgets.len(), 1);
    field.widgets[0].on
}

fn turn_on(viewer: &mut Viewer, name: &str) -> Vec<Event> {
    viewer
        .handle(Command::Edit(Edit::SetField {
            field: name.to_owned(),
            value: Entered::Text("On".to_owned()),
        }))
        .collect()
}

#[test]
fn signed_lock_refuses_only_the_named_field_when_the_reader_enforces_it() {
    let mut signed = open(SIGNED, RestrictionLevel::On);
    assert!(!checked(&signed, "name"));
    let events = turn_on(&mut signed, "name");
    assert!(events.iter().any(|event| matches!(
        event,
        Event::Refused {
            document: DocumentId(1),
            operation: Operation::FillInForm,
            ..
        }
    )));
    assert!(
        !checked(&signed, "name"),
        "refusal must preserve the actual field value"
    );
    let events = turn_on(&mut signed, "other");
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Refused { .. }))
    );
    assert!(checked(&signed, "other"), "unlisted field remains editable");

    let mut unsigned = open(UNSIGNED, RestrictionLevel::On);
    assert!(!checked(&unsigned, "name"));
    let events = turn_on(&mut unsigned, "name");
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Refused { .. }))
    );
    assert!(
        checked(&unsigned, "name"),
        "unsigned Lock is not an active restriction"
    );

    let mut overridden = open(SIGNED, RestrictionLevel::Off);
    let events = turn_on(&mut overridden, "name");
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Refused { .. }))
    );
    assert!(
        checked(&overridden, "name"),
        "explicit reader policy overrides the lock"
    );
}
