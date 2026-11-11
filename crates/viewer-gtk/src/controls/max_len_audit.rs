//! Real GTK controls and their change callbacks must respect the PDF character limit.
//! Run under a display, for example `xvfb-run -a cargo test -p viewer-gtk --lib max_len_audit`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk4::prelude::*;
use pdf_model::form::{Control, TextControl};
use pdf_model::view::FieldName;
use pdf_syntax::ObjectId;
use viewer_core::{Entered, FormField, FormWidget};

use super::{FieldChange, build};

fn field(multiline: bool, password: bool, max_len: Option<u32>) -> FormField {
    FormField {
        name: FieldName {
            qualified: String::from("field"),
            alternative: None,
        },
        partial: String::from("field"),
        control: Control::Text(TextControl {
            multiline,
            password,
            max_len,
            ..TextControl::default()
        }),
        value: None,
        displayed: None,
        read_only: false,
        required: false,
        no_export: false,
        widgets: vec![widget()],
    }
}

fn widget() -> FormWidget {
    FormWidget {
        annotation: ObjectId {
            number: 17,
            generation: 0,
        },
        quad: [0.0, 0.0, 100.0, 0.0, 100.0, 30.0, 0.0, 30.0],
        on_state: None,
        export: None,
        on: false,
    }
}

fn built(
    multiline: bool,
    password: bool,
    max_len: Option<u32>,
) -> (gtk4::Widget, Rc<RefCell<Vec<String>>>) {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let target = Rc::clone(&seen);
    let change: Rc<dyn Fn(FieldChange)> = Rc::new(move |event| {
        if let FieldChange::Set {
            value: Entered::Text(text),
            ..
        } = event
        {
            target.borrow_mut().push(text);
        }
    });
    let placed = build(
        &field(multiline, password, max_len),
        &widget(),
        &Rc::new(Cell::new(false)),
        &change,
    )
    .expect("text field builds an editable control");
    assert_eq!(placed.kind.max_len(), max_len, "build maps the PDF limit");
    (placed.widget, seen)
}

fn last(seen: &RefCell<Vec<String>>) -> String {
    seen.borrow()
        .last()
        .cloned()
        .expect("native insertion emitted a change")
}

#[gtk4::test]
fn ordinary_entry_is_a_limited_positive_control() {
    let (widget, seen) = built(false, false, Some(2));
    let entry = widget.downcast::<gtk4::Entry>().expect("single-line Entry");
    let mut position = 0;
    entry.insert_text("ab", &mut position);
    entry.insert_text("c", &mut position);
    assert_eq!(entry.text(), "ab");
    assert_eq!(last(&seen), "ab");
}

#[gtk4::test]
fn multiline_entry_obeys_maxlen_and_remains_unlimited_when_unspecified() {
    // Positive control for this branch: no /MaxLen means the TextView accepts the full insertion.
    let (widget, seen) = built(true, false, None);
    let scroller = widget
        .downcast::<gtk4::ScrolledWindow>()
        .expect("multiline ScrolledWindow");
    let view = scroller
        .child()
        .expect("TextView child")
        .downcast::<gtk4::TextView>()
        .expect("multiline TextView");
    view.buffer().insert_at_cursor("abc");
    let (start, end) = view.buffer().bounds();
    assert_eq!(view.buffer().text(&start, &end, false), "abc");
    assert_eq!(last(&seen), "abc");

    // Regression: with /MaxLen 2, the same native insertion path and callback must retain only
    // two characters. Current GTK branch reports all three.
    let (widget, seen) = built(true, false, Some(2));
    let scroller = widget
        .downcast::<gtk4::ScrolledWindow>()
        .expect("multiline ScrolledWindow");
    let view = scroller
        .child()
        .expect("TextView child")
        .downcast::<gtk4::TextView>()
        .expect("multiline TextView");
    view.buffer().insert_at_cursor("ab");
    assert_eq!(last(&seen), "ab", "legal prefix is accepted");
    view.buffer().insert_at_cursor("c");
    let (start, end) = view.buffer().bounds();
    assert_eq!(
        (
            view.buffer().text(&start, &end, false).to_string(),
            last(&seen)
        ),
        ("ab".to_owned(), "ab".to_owned())
    );
}

#[gtk4::test]
fn password_entry_obeys_maxlen_and_remains_unlimited_when_unspecified() {
    // Positive control for the secure branch: without /MaxLen, all native input reaches its
    // opaque control and its text-change callback.
    let (widget, seen) = built(false, true, None);
    let password = widget
        .downcast::<gtk4::PasswordEntry>()
        .expect("PasswordEntry");
    let mut position = 0;
    password.insert_text("abc", &mut position);
    assert_eq!(password.text(), "abc");
    assert_eq!(last(&seen), "abc");

    let (widget, seen) = built(false, true, Some(2));
    let password = widget
        .downcast::<gtk4::PasswordEntry>()
        .expect("PasswordEntry");
    let mut position = 0;
    password.insert_text("ab", &mut position);
    assert_eq!(last(&seen), "ab", "legal prefix is accepted");
    password.insert_text("c", &mut position);
    assert_eq!(
        (password.text().to_string(), last(&seen)),
        ("ab".to_owned(), "ab".to_owned())
    );
}

#[gtk4::test]
fn zero_maxlen_rejects_insertion_instead_of_using_gtk_zero_as_unlimited() {
    let (widget, seen) = built(false, false, Some(0));
    let entry = widget.downcast::<gtk4::Entry>().expect("single-line Entry");
    let mut position = 0;
    entry.insert_text("abc", &mut position);
    assert_eq!(entry.text(), "", "the PDF's maximum is zero characters");
    assert!(
        seen.borrow().iter().all(String::is_empty),
        "if the host reports a change, it must report the empty accepted value: {:?}",
        seen.borrow()
    );
}

// `ControlKind` is produced by the public builder; this assertion documents that the widget tests
// exercise the mapped descriptor, rather than a manually chosen entry configuration.
trait MaxLen {
    fn max_len(&self) -> Option<u32>;
}

impl MaxLen for viewer_host::form::ControlKind {
    fn max_len(&self) -> Option<u32> {
        match self {
            Self::Entry { max_len, .. } => *max_len,
            _ => None,
        }
    }
}
