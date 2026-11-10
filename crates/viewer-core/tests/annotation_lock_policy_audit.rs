//! Public contents-edit policy with Table 167 bits 8 and 10 as distinct inputs.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::panic,
    reason = "controlled fixture and public answers must fail loudly"
)]

use pdf_model::restriction::Operation;
use pdf_syntax::ObjectId;
use viewer_core::{
    Answer, Command, DocumentId, Edit, Event, Query, RestrictionLevel, RestrictionPolicy,
    RestrictionScope, Viewer,
};

const DOCUMENT: DocumentId = DocumentId(1);
const NOTE: ObjectId = ObjectId::new(5, 0);

fn pdf_with_note_flags(flags: i64) -> Vec<u8> {
    let body = format!(
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
         2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
         3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] \
         /Resources << >> /Contents 4 0 R /Annots [5 0 R] >>\nendobj\n\
         4 0 obj\n<< /Length 0 >>\nstream\nendstream\nendobj\n\
         5 0 obj\n<< /Type /Annot /Subtype /Text /Rect [10 260 30 280] \
         /Contents (before) /Open true /F {flags} >>\nendobj\n"
    );
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for object in body.split_inclusive("endobj\n") {
        offsets.push(bytes.len());
        bytes.extend_from_slice(object.as_bytes());
    }
    let xref = bytes.len();
    let size = offsets.len() + 1;
    bytes.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {size} /Root 1 0 R \
         /ID [<0123456789abcdef0123456789abcdef> \
              <0123456789abcdef0123456789abcdef>] >>\n\
         startxref\n{xref}\n%%EOF\n"
        )
        .as_bytes(),
    );
    bytes
}

fn opened(flags: i64, level: Option<RestrictionLevel>) -> Viewer {
    let mut viewer = Viewer::new(400, 400, 1.0);
    if let Some(level) = level {
        let policy = RestrictionPolicy::default().with(Operation::Annotate, level);
        viewer
            .handle(Command::Restrict(RestrictionScope::Window(policy)))
            .for_each(drop);
    }
    let bytes = pdf_with_note_flags(flags);
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("complete PDF fixture");
    assert!(!parsed.was_recovered(), "exact xref offsets");
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

fn note_text(viewer: &Viewer) -> String {
    let Answer::Popups(windows) = viewer.query(Query::Popups) else {
        panic!("a popup query answers with windows");
    };
    windows
        .iter()
        .find(|window| window.annotation == NOTE)
        .and_then(|window| window.text.as_deref())
        .expect("the text annotation's /Open window exposes its contents")
        .to_owned()
}

fn retype(viewer: &mut Viewer, text: &str) -> Vec<Event> {
    viewer
        .handle(Command::Edit(Edit::SetNoteText {
            annotation: NOTE,
            text: text.to_owned(),
        }))
        .collect()
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "parallel policy cases retain their public controls together"
)]
fn locked_contents_obeys_the_reader_policy_and_locked_alone_does_not_prompt() {
    const LOCKED: i64 = 1 << 7; // Table 167 bit 8.
    const LOCKED_CONTENTS: i64 = 1 << 9; // Table 167 bit 10.

    // The host's default policy is Off (ADR 1144): a LockedContents annotation remains editable
    // until the reader chooses to obey the document's assertion.
    let mut off = opened(LOCKED_CONTENTS, None);
    assert_eq!(note_text(&off), "before");
    let events = retype(&mut off, "off proceeds");
    assert_eq!(note_text(&off), "off proceeds");
    assert!(!events.iter().any(|event| matches!(
        event,
        Event::Asking {
            operation: Operation::Annotate,
            ..
        } | Event::Refused {
            operation: Operation::Annotate,
            ..
        } | Event::Warned {
            operation: Operation::Annotate,
            ..
        }
    )));

    // On refuses before changing the text and names the actual table flag.
    let mut on = opened(LOCKED_CONTENTS, Some(RestrictionLevel::On));
    let events = retype(&mut on, "must not appear");
    assert_eq!(note_text(&on), "before");
    assert!(
        events.iter().any(|event| matches!(
            event,
            Event::Refused {
                operation: Operation::Annotate,
                notes,
                ..
            } if notes.iter().any(|note| note.contains("LockedContents") && note.contains("bit 10"))
        )),
        "{events:?}"
    );

    // Ask holds the resolved edit: the viewer does not change the visible contents before the
    // answer, and a no leaves it alone.
    let mut ask_no = opened(LOCKED_CONTENTS, Some(RestrictionLevel::Ask));
    let events = retype(&mut ask_no, "held text");
    assert_eq!(note_text(&ask_no), "before");
    assert!(
        events.iter().any(|event| matches!(
            event,
            Event::Asking { operation: Operation::Annotate, notes, .. }
                if notes.iter().any(|note| note.contains("LockedContents"))
        )),
        "{events:?}"
    );
    ask_no
        .handle(Command::Answer {
            document: DOCUMENT,
            proceed: false,
        })
        .for_each(drop);
    assert_eq!(note_text(&ask_no), "before");

    // A fresh Ask answered yes performs exactly the edit that was held.
    let mut ask_yes = opened(LOCKED_CONTENTS, Some(RestrictionLevel::Ask));
    let events = retype(&mut ask_yes, "yes proceeds");
    assert!(
        events.iter().any(|event| matches!(
            event,
            Event::Asking {
                operation: Operation::Annotate,
                ..
            }
        )),
        "{events:?}"
    );
    assert_eq!(
        note_text(&ask_yes),
        "before",
        "yes has not yet been supplied"
    );
    ask_yes
        .handle(Command::Answer {
            document: DOCUMENT,
            proceed: true,
        })
        .for_each(drop);
    assert_eq!(note_text(&ask_yes), "yes proceeds");

    // Warn commits and reports; it is distinct from both Ask and On.
    let mut warned = opened(LOCKED_CONTENTS, Some(RestrictionLevel::Warn));
    let events = retype(&mut warned, "warn proceeds");
    assert_eq!(note_text(&warned), "warn proceeds");
    let dirty = events
        .iter()
        .position(|event| matches!(event, Event::Dirty { .. }))
        .expect("edit dirties the document");
    let warning = events
        .iter()
        .position(|event| matches!(event, Event::Warned { .. }))
        .expect("Warn reports after commit");
    assert!(
        dirty < warning,
        "the host sees the committed change before its warning"
    );

    assert!(events.iter().any(|event| matches!(event,
        Event::Warned { operation: Operation::Annotate, notes, .. }
        if notes.iter().any(|note| note.contains("LockedContents"))
    )));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Asking { .. } | Event::Refused { .. }))
    );

    // Table 167 bit 8 expressly does not restrict contents. Under Ask it creates no question;
    // this test says nothing about deletion or property-edit operations.
    let mut locked_only = opened(LOCKED, Some(RestrictionLevel::Ask));
    let events = retype(&mut locked_only, "bit eight allows contents");
    assert_eq!(note_text(&locked_only), "bit eight allows contents");
    assert!(
        !events.iter().any(|event| matches!(
            event,
            Event::Asking {
                operation: Operation::Annotate,
                ..
            }
        )),
        "Locked alone is not a contents restriction: {events:?}"
    );
}
