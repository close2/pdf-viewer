//! ISO 32000-2 §12.7.5.3, Table 231 bit 21: a file-select control's file is read once, when the
//! person finishes with the field, never per keystroke (ADR 1216, ADR 3041).
//! Run under a display: `xvfb-run -a cargo test -p viewer-gtk --lib file_select_reads`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk4::prelude::*;
use pdf_model::form::{Control, TextControl};
use pdf_model::view::FieldName;
use pdf_syntax::ObjectId;
use viewer_core::{FormField, FormWidget};

use super::{FieldChange, build};

fn upload_field() -> FormField {
    FormField {
        name: FieldName {
            qualified: String::from("upload"),
            alternative: None,
        },
        partial: String::from("upload"),
        control: Control::Text(TextControl {
            file_select: true,
            ..TextControl::default()
        }),
        value: None,
        displayed: None,
        read_only: false,
        required: false,
        no_export: false,
        widgets: vec![FormWidget {
            annotation: ObjectId {
                number: 9,
                generation: 0,
            },
            quad: [0.0, 0.0, 200.0, 0.0, 200.0, 20.0, 0.0, 20.0],
            on_state: None,
            export: None,
            on: false,
        }],
    }
}

/// A path typed into the entry one character at a time and then Enter, carried through
/// `viewer_host::TypedPaths` as `host::forms`' field sink carries it: one read, at the commit.
#[gtk4::test]
fn a_typed_path_is_read_once_when_the_field_is_committed() {
    let written =
        std::env::temp_dir().join(format!("quorra-gtk-file-select-{}.txt", std::process::id()));
    std::fs::write(&written, b"typed").expect("the test writes its own file");
    let path = written.to_string_lossy().into_owned();

    let seen = Rc::new(RefCell::new(Vec::new()));
    let target = Rc::clone(&seen);
    let change: Rc<dyn Fn(FieldChange)> = Rc::new(move |event| target.borrow_mut().push(event));
    let field = upload_field();
    let kind = viewer_host::control_kind(&field.control);
    let placed = build(
        &field,
        &field.widgets[0],
        &Rc::new(Cell::new(false)),
        &change,
    )
    .expect("a text field builds a control");
    let entry = placed
        .widget
        .downcast::<gtk4::Entry>()
        .expect("a single-line text field is an Entry");
    for end in 1..=path.len() {
        if let Some(prefix) = path.get(..end) {
            entry.set_text(prefix);
        }
    }
    entry.emit_activate();

    let mut paths = viewer_host::TypedPaths::default();
    let before = viewer_host::chosen_reads();
    let mut sent = Vec::new();
    for change in seen.take() {
        match change {
            FieldChange::Set { field, value } => {
                sent.push(viewer_core::Command::Edit(paths.typed(
                    Some(&kind),
                    &field,
                    value,
                )));
            }
            FieldChange::Commit { field, enter } => {
                let by = if enter {
                    viewer_core::Committing::Enter
                } else {
                    viewer_core::Committing::Unstated
                };
                let commit = paths.commit(Some(&kind), &field, by);
                assert_eq!(commit.refusal, None);
                sent.extend(commit.commands);
            }
            _ => {}
        }
    }
    let _ = std::fs::remove_file(&written);
    assert_eq!(
        viewer_host::chosen_reads(),
        before.saturating_add(1),
        "one read for the typed path and its commit"
    );
    let chosen = sent
        .iter()
        .filter(|command| {
            matches!(
                command,
                viewer_core::Command::Edit(viewer_core::Edit::ChooseFile { .. })
            )
        })
        .count();
    assert_eq!(chosen, 1, "one file chosen: {sent:?}");
    assert!(
        matches!(sent.last(), Some(viewer_core::Command::CommitField { .. })),
        "the commit comes last: {sent:?}"
    );
}
