//! ISO 32000-2 §12.7.5.4, Table 233 bit 27: a list box's selection is committed at once where
//! `CommitOnSelChange` is set and waits for the field to be left where it is clear (ADR 2816).
//! Run under a display: `xvfb-run -a cargo test -p viewer-gtk --lib selection_commits`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk4::prelude::*;
use pdf_model::form::{Choice, ChoiceControl, Control};
use pdf_model::view::FieldName;
use pdf_syntax::ObjectId;
use viewer_core::{FormField, FormWidget};

use super::{FieldChange, build};

fn list_field(commit_on_selection: bool) -> FormField {
    let option = |label: &str| Choice {
        export: None,
        label: label.to_owned(),
    };
    FormField {
        name: FieldName {
            qualified: String::from("pick"),
            alternative: None,
        },
        partial: String::from("pick"),
        control: Control::Choice(ChoiceControl {
            commit_on_selection,
            options: vec![option("5"), option("7"), option("1")],
            ..ChoiceControl::default()
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
            quad: [0.0, 0.0, 100.0, 0.0, 100.0, 60.0, 0.0, 60.0],
            on_state: None,
            export: None,
            on: false,
        }],
    }
}

/// What the list box placed for the field sends when its third option is selected.
fn sent_on_selecting(commit_on_selection: bool) -> Vec<FieldChange> {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let target = Rc::clone(&seen);
    let change: Rc<dyn Fn(FieldChange)> = Rc::new(move |event| target.borrow_mut().push(event));
    let field = list_field(commit_on_selection);
    let placed = build(
        &field,
        &field.widgets[0],
        &Rc::new(Cell::new(false)),
        &change,
    )
    .expect("a list box builds a control");
    let view = placed
        .widget
        .downcast::<gtk4::ScrolledWindow>()
        .expect("the list sits in a scroller")
        .child()
        .and_downcast::<gtk4::ListView>()
        .expect("a ListView");
    let model = view.model().expect("the list's selection model");
    model.select_item(2, true);
    seen.take()
}

#[gtk4::test]
fn a_set_bit_commits_the_selection_and_a_clear_bit_does_not() {
    let set = FieldChange::Set {
        field: String::from("pick"),
        value: viewer_core::Entered::Chosen(vec![2]),
    };
    let commit = FieldChange::Commit {
        field: String::from("pick"),
        enter: false,
    };
    assert_eq!(sent_on_selecting(false), std::slice::from_ref(&set));
    assert_eq!(sent_on_selecting(true), [set, commit]);
}
