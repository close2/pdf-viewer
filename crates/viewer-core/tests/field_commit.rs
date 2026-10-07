//! §12.7.4.3's commit reached from a host: a tab out of a field, Enter, and a refusal said.
//!
//! `ViewState::commit_field` runs Table 199's `/K` in its commit form and `/V`; this suite is the
//! viewer's half of it (ADR 1592) — the focus leaving a widget commits its field before Table
//! 197's `/Bl`, `Command::CommitField` commits it for a host whose own control had the keyboard,
//! the commit is an entry of the log so that a replay keeps it, and a refusal at either the commit
//! or a keystroke is a sentence a window shows.
//!
//! The fixture is the shape `crates/pdf-model/tests/aform.rs` and the window drive use: three
//! number fields under `AFNumber_Format` and `AFNumber_Keystroke`, and a total under
//! `AFSimple_Calculate` that Table 224's `/CO` names.

#![expect(
    clippy::panic,
    clippy::expect_used,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly rather than \
              pass by doing nothing"
)]

use std::fmt::Write as _;

use pdf_syntax::{Document, ObjectId};
use viewer_core::{Answer, Command, DocumentId, Edit, Entered, Event, FocusMove, Query, Viewer};

const DOCUMENT: DocumentId = DocumentId(1);

/// The currency format and keystroke every number field states: two decimals, a comma between
/// thousands, the dollar sign before the number.
const CURRENCY: &str = "/F << /S /JavaScript /JS (AFNumber_Format\\(2, 0, 0, 0, \"$\", true\\);) >> \
                        /K << /S /JavaScript /JS (AFNumber_Keystroke\\(2, 0, 0, 0, \"$\", true\\);) >>";

/// Three priced lines and their total, widgets 4 to 7 on one page, in that tab order.
fn form() -> Vec<u8> {
    let field = |name: &str, top: u32, actions: &str| {
        format!(
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T ({name}) /F 4 \
             /Rect [20 {} 220 {top}] /DA (/Helv 12 Tf 0 g) /AA << {actions} >> >>",
            top.saturating_sub(30)
        )
    };
    let total = "/C << /S /JavaScript /JS (AFSimple_Calculate\\(\"SUM\", \
                 [\"Price1\", \"Price2\", \"Price3\"]\\);) >> \
                 /F << /S /JavaScript /JS (AFNumber_Format\\(2, 0, 0, 0, \"$\", true\\);) >>";
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 5 0 R 6 0 R 7 0 R] \
         /CO [7 0 R] /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv 8 0 R >> >> >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [4 0 R 5 0 R 6 0 R 7 0 R] \
         >>"
        .to_owned(),
        field("Price1", 380, CURRENCY),
        field("Price2", 340, CURRENCY),
        field("Price3", 300, CURRENCY),
        format!(
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Total) /Ff 1 /F 4 /Rect [20 230 220 260] \
             /DA (/Helv 12 Tf 0 g) /AA << {total} >> >>"
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
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
    out.into_bytes()
}

fn opened() -> Viewer {
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: form().into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer
}

fn typed(viewer: &mut Viewer, field: &str, text: &str) -> Vec<Event> {
    viewer
        .handle(Command::Edit(Edit::SetField {
            field: field.to_owned(),
            value: Entered::Text(text.to_owned()),
        }))
        .collect()
}

fn value(viewer: &Viewer, field: &str) -> Option<String> {
    let Answer::Fields(fields) = viewer.query(Query::Fields) else {
        return None;
    };
    fields
        .into_iter()
        .find(|candidate| candidate.name.qualified == field)?
        .value
        .map(|shown| shown.text)
}

fn sentences(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Reported {
                page: None, notes, ..
            } => Some(notes.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

/// The decoded normal appearance a save writes for widget `number`.
fn saved_appearance(viewer: &mut Viewer, number: u32) -> String {
    let Some(Event::Saved { bytes, .. }) = viewer
        .handle(Command::Save)
        .find(|event| matches!(event, Event::Saved { .. }))
    else {
        panic!("a save hands the file back");
    };
    let saved = Document::open(bytes).expect("the update reads back");
    let object = saved.get(ObjectId {
        number,
        generation: 0,
    });
    let widget = object.as_dict().expect("a widget");
    let appearances = saved.get_key(widget, "AP");
    let normal = appearances
        .as_dict()
        .map(|appearances| saved.get_key(appearances, "N"))
        .expect("an /AP");
    let stream = normal.as_stream().expect("a stream");
    String::from_utf8_lossy(&saved.decoded_stream_data(stream).expect("decodes")).into_owned()
}

/// Table 197's `/Bl`: a tab out of the field commits it — what the field's script refuses goes
/// back at once, and what it accepts is saved with its format.
///
/// The refusal is the witness that the commit ran: a save draws every value through its format
/// whether or not it was committed, while only a commit puts a refused value back.
#[test]
fn a_tab_out_of_a_field_commits_it() {
    let mut viewer = opened();
    viewer
        .handle(Command::Focused(FocusMove::Next))
        .for_each(drop);
    let Answer::Focus { object, .. } = viewer.query(Query::Focus) else {
        panic!("the first tab focuses the first widget");
    };
    assert_eq!(object.number, 4);
    typed(&mut viewer, "Price1", "-");
    assert_eq!(value(&viewer, "Price1").as_deref(), Some("-"));
    let events: Vec<Event> = viewer.handle(Command::Focused(FocusMove::Next)).collect();
    let said = sentences(&events);
    assert_eq!(said.len(), 1, "{events:?}");
    assert!(said[0].starts_with("Price1: "), "{said:?}");
    assert_eq!(value(&viewer, "Price1").unwrap_or_default(), "");

    typed(&mut viewer, "Price2", "12.5");
    let events: Vec<Event> = viewer.handle(Command::Focused(FocusMove::Next)).collect();
    assert!(sentences(&events).is_empty(), "{events:?}");
    let content = saved_appearance(&mut viewer, 5);
    assert!(content.contains("($12.50) Tj"), "{content}");
    let total = saved_appearance(&mut viewer, 7);
    assert!(total.contains("($12.50) Tj"), "{total}");
}

/// A commit is an entry of the log: the next edit replays it rather than losing it, and an undo
/// takes it back to the typing it committed.
#[test]
fn a_commit_survives_the_next_edit_and_an_undo_takes_it_back() {
    let mut viewer = opened();
    typed(&mut viewer, "Price1", "-");
    viewer
        .handle(Command::CommitField {
            field: "Price1".to_owned(),
        })
        .for_each(drop);
    typed(&mut viewer, "Price2", "4");
    assert_eq!(
        value(&viewer, "Price1").unwrap_or_default(),
        "",
        "the replay the second edit made refused the first field's value again"
    );
    viewer.handle(Command::Undo).for_each(drop);
    viewer.handle(Command::Undo).for_each(drop);
    assert_eq!(
        value(&viewer, "Price1").as_deref(),
        Some("-"),
        "the commit undone leaves the typing"
    );
    assert_eq!(
        viewer
            .handle(Command::CommitField {
                field: "Price3".to_owned(),
            })
            .count(),
        0,
        "a field nobody typed into commits nothing and says nothing"
    );
}

/// A value the keystroke's commit form refuses goes back, and the window is told why.
#[test]
fn a_refused_commit_is_said_and_the_field_goes_back() {
    let mut viewer = opened();
    typed(&mut viewer, "Price2", "-");
    let events: Vec<Event> = viewer
        .handle(Command::CommitField {
            field: "Price2".to_owned(),
        })
        .collect();
    let said = sentences(&events);
    assert_eq!(said.len(), 1, "{events:?}");
    assert!(
        said[0].starts_with("Price2: the value entered does not match the format of the field"),
        "{said:?}"
    );
    assert_eq!(value(&viewer, "Price2").unwrap_or_default(), "");
}

/// Table 199's `/K` rejecting a character: said, and no entry of the log.
#[test]
fn a_refused_keystroke_is_said_and_changes_nothing() {
    let mut viewer = opened();
    typed(&mut viewer, "Price3", "7");
    let events = typed(&mut viewer, "Price3", "7x");
    let said = sentences(&events);
    assert_eq!(said.len(), 1, "{events:?}");
    assert!(
        said[0].contains("AFNumber_Keystroke refused \"7x\""),
        "{said:?}"
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::NeedsRender(_) | Event::Dirty { .. })),
        "nothing changed, so nothing is drawn again: {events:?}"
    );
    assert_eq!(value(&viewer, "Price3").as_deref(), Some("7"));
    viewer.handle(Command::Undo).for_each(drop);
    assert_eq!(
        value(&viewer, "Price3").unwrap_or_default(),
        "",
        "one undo takes back the one keystroke the log holds"
    );
}

/// What `Query::Fields` answers a field displays: the characters while they are being typed, Table
/// 199's `/F` applied once they are committed, and the characters where the field states no format
/// — and `value` stays the characters throughout, which is what a host's control holding the
/// keyboard shows (ADR 1604).
///
/// `AFNumber_Format(2, 0, 0, 0, "$", true)` is two decimals, a comma between thousands and the
/// dollar sign before the number, so 12.5 is displayed `$12.50` — the library's own reading of its
/// arguments, held against Adobe's reference by `crates/pdf-model/tests/aform.rs`.
#[test]
fn a_committed_value_is_answered_as_the_field_displays_it() {
    let shown = |viewer: &Viewer, field: &str| {
        let Answer::Fields(fields) = viewer.query(Query::Fields) else {
            panic!("the page has a form");
        };
        let field = fields
            .into_iter()
            .find(|candidate| candidate.name.qualified == field)
            .expect("the field is on the page");
        (field.value.map(|shown| shown.text), field.displayed)
    };
    let mut viewer = opened();
    assert_eq!(
        shown(&viewer, "Price1"),
        (Some(String::new()), Some(String::new())),
        "an empty field displays nothing"
    );
    typed(&mut viewer, "Price1", "12.5");
    assert_eq!(
        shown(&viewer, "Price1"),
        (Some("12.5".to_owned()), Some("12.5".to_owned())),
        "a field being typed into displays as typed"
    );
    viewer
        .handle(Command::CommitField {
            field: "Price1".to_owned(),
        })
        .for_each(drop);
    assert_eq!(
        shown(&viewer, "Price1"),
        (Some("12.5".to_owned()), Some("$12.50".to_owned()))
    );
    assert_eq!(
        shown(&viewer, "Total"),
        (Some("12.5".to_owned()), Some("$12.50".to_owned())),
        "Table 224's /CO recalculated the total, which displays through its own /F"
    );
}

/// An accessibility node's value is what the field displays — `$12.50` once `12.5` is committed
/// under `AFNumber_Format` — and what a person typed while they are typing, so that the text a
/// screen reader is given is the text whose characters the node places (ADR 1617).
#[test]
fn an_accessible_value_is_the_displayed_one() {
    let accessible = |viewer: &Viewer, field: &str| {
        let Answer::Accessibility(pages) = viewer.query(Query::AccessibilityTree) else {
            panic!("an open document answers the question");
        };
        pages
            .iter()
            .flat_map(|page| page.widgets.iter())
            .find(|node| node.name == field)
            .and_then(|node| node.value.as_ref())
            .map(|shown| shown.text.clone())
    };
    let mut viewer = opened();
    typed(&mut viewer, "Price1", "12.5");
    assert_eq!(
        accessible(&viewer, "Price1").as_deref(),
        Some("12.5"),
        "typing: what was typed"
    );
    viewer
        .handle(Command::CommitField {
            field: "Price1".to_owned(),
        })
        .for_each(drop);
    assert_eq!(accessible(&viewer, "Price1").as_deref(), Some("$12.50"));
    assert_eq!(
        accessible(&viewer, "Total").as_deref(),
        Some("$12.50"),
        "a calculated field is read as it displays too"
    );
}
