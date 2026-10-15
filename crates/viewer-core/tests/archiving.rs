//! `Command::Archive` across the boundary: Table 22's bit 11 asked at the reader's level, and the
//! grant carrying the file as it was opened.
//!
//! The conversion itself is `pdf_transform`'s and the host's to run (`viewer_host::archiving`);
//! what this crate owns is the one place the level for assembling a document into a new one is
//! asked, and the grant. §7.6.4.2's Table 22 states the bit:
//!
//! > ( Security handlers of revision 3 or greater ) Assemble the document (insert, rotate, or delete
//! > pages and create document outline items or thumbnail images), even if bit 4 is clear.
//!
//! — which is the operation `quorra-transform archive` asks too (ADR 1954).

#![expect(
    clippy::panic,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing"
)]

use std::path::Path;

use pdf_model::restriction::Operation;
use viewer_core::{
    Archiving, Command, DocumentId, Event, RestrictionLevel, RestrictionPolicy, RestrictionScope,
    Viewer,
};

/// The document every test here opens.
const DOCUMENT: DocumentId = DocumentId(1);

/// What every test asks for: PDF/A-2b under the profile a printed copy answers.
fn asked() -> Archiving {
    Archiving {
        target: pdf_archive::Target::Two(pdf_archive::Level::B),
        profile: "as-if-printed".to_owned(),
    }
}

/// A viewer at one policy with `bytes` open.
fn reading(bytes: Vec<u8>, policy: RestrictionPolicy) -> Viewer {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Restrict(RestrictionScope::Window(policy)))
        .for_each(drop);
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

/// `bug1815476.pdf`: `/R 4` and `/P −1084`, which is `0xFFFFFBC4` — **bit 11 clear**, so Table 22
/// withholds assembling the document. `None` where `doc/pdf.js` is not checked out.
fn restricted_bytes() -> Option<Vec<u8>> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs/bug1815476.pdf"),
    )
    .ok()
}

/// The PDF Association's note, which is committed and states no `/Encrypt` at all.
fn unrestricted_bytes() -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/PDF20_AN001-BPC.pdf");
    std::fs::read(&path).unwrap_or_else(|error| panic!("{} is committed: {error}", path.display()))
}

/// The grant among some events, as its file's length and what it was asked for.
fn granted(events: &[Event]) -> Option<(usize, Archiving)> {
    events.iter().find_map(|event| match event {
        Event::Archiving {
            document,
            bytes,
            asked,
            ..
        } if *document == DOCUMENT => Some((bytes.len(), asked.clone())),
        _ => None,
    })
}

/// A document that restricts nothing is granted at every level, with the file it was opened from.
#[test]
fn a_document_that_restricts_nothing_is_granted_with_its_own_file() {
    let bytes = unrestricted_bytes();
    let length = bytes.len();
    for level in [
        RestrictionLevel::Off,
        RestrictionLevel::On,
        RestrictionLevel::Ask,
        RestrictionLevel::Warn,
    ] {
        let mut viewer = reading(bytes.clone(), RestrictionPolicy::uniform(level));
        let events: Vec<Event> = viewer.handle(Command::Archive(asked())).collect();
        assert_eq!(
            granted(&events),
            Some((length, asked())),
            "{level:?}: {events:?}"
        );
        assert!(
            !events.iter().any(|event| matches!(
                event,
                Event::Refused { .. } | Event::Asking { .. } | Event::Warned { .. }
            )),
            "{level:?}: nothing restricted, nothing said: {events:?}"
        );
    }
}

/// Bit 11 withheld, at each of the four levels: `Off` converts, `On` refuses by name, `Ask` holds
/// the request until a `yes`, `Warn` converts and says what the document asserted.
#[test]
fn a_conversion_obeys_the_level_the_reader_set_for_assembling() {
    let Some(bytes) = restricted_bytes() else {
        eprintln!("skipped: doc/pdf.js is not checked out");
        return;
    };
    let at = |level| RestrictionPolicy::default().with(Operation::Assemble, level);

    let mut viewer = reading(bytes.clone(), at(RestrictionLevel::Off));
    let events: Vec<Event> = viewer.handle(Command::Archive(asked())).collect();
    assert!(granted(&events).is_some(), "`Off` converts: {events:?}");

    let mut viewer = reading(bytes.clone(), at(RestrictionLevel::On));
    let events: Vec<Event> = viewer.handle(Command::Archive(asked())).collect();
    assert!(granted(&events).is_none(), "nothing granted: {events:?}");
    assert!(
        events.iter().any(|event| matches!(
            event,
            Event::Refused {
                operation: Operation::Assemble,
                ..
            }
        )),
        "refused by name: {events:?}"
    );

    let mut viewer = reading(bytes.clone(), at(RestrictionLevel::Ask));
    let events: Vec<Event> = viewer.handle(Command::Archive(asked())).collect();
    assert!(
        granted(&events).is_none(),
        "nothing until answered: {events:?}"
    );
    assert!(
        events.iter().any(|event| matches!(
            event,
            Event::Asking {
                operation: Operation::Assemble,
                ..
            }
        )),
        "the question is asked: {events:?}"
    );
    let events: Vec<Event> = viewer
        .handle(Command::Answer {
            document: DOCUMENT,
            proceed: true,
        })
        .collect();
    assert_eq!(
        granted(&events).map(|(_, asked)| asked),
        Some(asked()),
        "a yes releases what was asked about: {events:?}"
    );

    let mut viewer = reading(bytes.clone(), at(RestrictionLevel::Ask));
    viewer.handle(Command::Archive(asked())).for_each(drop);
    let events: Vec<Event> = viewer
        .handle(Command::Answer {
            document: DOCUMENT,
            proceed: false,
        })
        .collect();
    assert!(granted(&events).is_none(), "a no forgets it: {events:?}");

    let mut viewer = reading(bytes, at(RestrictionLevel::Warn));
    let events: Vec<Event> = viewer.handle(Command::Archive(asked())).collect();
    assert!(granted(&events).is_some(), "`Warn` converts: {events:?}");
    assert!(
        events.iter().any(|event| matches!(
            event,
            Event::Warned {
                operation: Operation::Assemble,
                ..
            }
        )),
        "and says so: {events:?}"
    );
}
