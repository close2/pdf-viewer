//! §12.7's form, as real GTK4 controls placed over the page.
//!
//! `doc/todo/30`'s second item, and the one that became possible in the
//! three-hundred-and-ninety-eighth session (ADR 0235): "real controls over `Query::Fields` — a
//! `GtkEntry` for a text field, a `GtkComboBoxText` for §12.7.5.4's combo box, a `GtkCheckButton`
//! for a check box". The combo box is a [`gtk4::DropDown`] here rather than a `GtkComboBoxText`,
//! which GTK deprecated in 4.10.
//!
//! **What this host cannot do, and it is `doc/todo/37`'s open decision**: ask for the page drawn
//! *without* the widget appearances underneath these controls. §12.5.5's appearance streams are
//! page content, so the entry sits on top of the picture of an entry and a person sees the field
//! twice. That is the largest thing the boundary turned out to be missing and ADR 0244 records
//! what it costs.

use std::cell::Cell;
use std::rc::Rc;

use gtk4::prelude::*;
use pdf_syntax::ObjectId;
use viewer_core::{FormField, FormWidget};

use viewer_host::form::{ControlKind, control_kind};

/// What a person did to a control, in the vocabulary the viewer takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FieldChange {
    /// [`viewer_core::Edit::SetField`], addressed by §12.7.4.2's fully qualified name.
    Set {
        /// The qualified name.
        field: String,
        /// The new value, or nothing to clear it.
        value: Option<String>,
    },
    /// [`viewer_core::Command::Activate`] on a widget, which is what a push button is for.
    Activate(ObjectId),
}

/// A control on the screen, and enough to know whether the next frame may keep it.
pub(crate) struct Placed {
    /// §12.7.4.2's qualified name and the widget annotation, which together name this control.
    pub(crate) key: (String, ObjectId),
    /// The widget, so that the next frame can move it rather than rebuild it.
    pub(crate) widget: gtk4::Widget,
    /// What it is, so that a value read back after a keystroke goes to the right place.
    pub(crate) kind: ControlKind,
}

impl std::fmt::Debug for Placed {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Placed")
            .field("key", &self.key)
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

/// Which fields and widgets a frame's controls are for.
///
/// Compared between frames so that a scroll or a zoom *moves* the controls instead of rebuilding
/// them: rebuilding would take the keyboard away from whatever a person is typing into, and the
/// page moves under them every time the window is resized.
pub(crate) fn signature(fields: &[FormField]) -> Vec<(String, ObjectId)> {
    fields
        .iter()
        .flat_map(|field| {
            field
                .widgets
                .iter()
                .map(|widget| (field.name.qualified.clone(), widget.annotation))
        })
        .collect()
}

/// Builds one control per widget of one field, or nothing where the field is not a control.
///
/// The suppression flag is what keeps a value read back after a keystroke from looking like a
/// second keystroke: `viewer-core` answers with what the field *took* (§12.7.5.3's truncation,
/// ADR 0197), so a host writes that back into its entry, and an entry whose text changes emits
/// the same signal a person's typing does.
pub(crate) fn build(
    field: &FormField,
    widget: &FormWidget,
    suppress: &Rc<Cell<bool>>,
    change: &Rc<dyn Fn(FieldChange)>,
) -> Option<Placed> {
    let kind = control_kind(&field.control);
    let control: gtk4::Widget = match &kind {
        ControlKind::Entry {
            multiline,
            password,
            max_len,
        } => entry(field, *multiline, *password, *max_len, suppress, change),
        ControlKind::Check { on } => toggle(field, widget, *on, false, suppress, change),
        ControlKind::Radio {
            on,
            no_toggle_to_off,
        } => toggle(field, widget, *on, *no_toggle_to_off, suppress, change),
        ControlKind::Push => push(field, widget, change),
        ControlKind::Combo {
            options, selected, ..
        } => combo(field, options, *selected, suppress, change),
        ControlKind::List {
            options,
            selected,
            multi,
        } => list(field, options, selected, *multi, suppress, change),
        // §12.7.5.5's signature has no control to build and Table 226's absent `/FT` names none,
        // so this host places nothing and the page's own appearance stands. Inventing a control
        // for either would be a statement about the document that the document did not make.
        ControlKind::Signature | ControlKind::Unstated => return None,
    };
    // Table 227 bit 1: "the field shall not be modified by the user". The platform's own way of
    // saying so, rather than a refusal after the fact.
    control.set_sensitive(!field.read_only);
    if let Some(shown) = tooltip(field) {
        control.set_tooltip_text(Some(&shown));
    }
    Some(Placed {
        key: (field.name.qualified.clone(), widget.annotation),
        widget: control,
        kind,
    })
}

/// §14.9.3's name, and Table 227 bit 2 beside it. ISO 32000-2 §14.9.3:
///
/// > An alternative name may be specified for an interactive form field (see 12.7, "Forms")
/// > which, if present, shall be used in place of the actual field name when an interactive PDF
/// > processor identifies the field in a user-interface.
///
/// A tooltip is where a control that has no room for a label puts it, and a control placed over
/// the page has none: the page is already showing whatever caption the document drew.
fn tooltip(field: &FormField) -> Option<String> {
    let shown = field.name.shown();
    if shown.is_empty() {
        return None;
    }
    if field.required {
        return Some(format!("{shown} (required)"));
    }
    Some(shown.to_owned())
}

/// §12.7.5.3's text field.
fn entry(
    field: &FormField,
    multiline: bool,
    password: bool,
    max_len: Option<u32>,
    suppress: &Rc<Cell<bool>>,
    change: &Rc<dyn Fn(FieldChange)>,
) -> gtk4::Widget {
    let value = field
        .value
        .as_ref()
        .map(|shown| shown.text.clone())
        .unwrap_or_default();
    let name = field.name.qualified.clone();
    if multiline {
        // Table 231 bit 13: "the field may contain multiple lines of text". A `GtkEntry` cannot
        // hold one, which is why the clause's flag decides the *control* and not a property of it.
        let view = gtk4::TextView::new();
        view.buffer().set_text(&value);
        let suppress = Rc::clone(suppress);
        let change = Rc::clone(change);
        view.buffer().connect_changed(move |buffer| {
            if suppress.get() {
                return;
            }
            let (start, end) = buffer.bounds();
            let text = buffer.text(&start, &end, false).to_string();
            change(FieldChange::Set {
                field: name.clone(),
                value: Some(text),
            });
        });
        let scroller = gtk4::ScrolledWindow::new();
        scroller.set_child(Some(&view));
        return scroller.upcast();
    }
    if password {
        // Table 231 bit 14: "intended for entering a secure password that should not be echoed
        // visibly to the screen". GTK4 has a control *for* that rather than an entry with a mask,
        // and it is the one to use — a native host that drew bullets itself would be reproducing
        // what the platform already knows how to do.
        //
        // **This is the one control whose value is not read back**, and the reason is a fact
        // about the boundary rather than about GTK: `Answer::Field` answers a password field with
        // Table 231 bit 14's bullets rather than with its characters, so writing the answer into
        // the control would replace what a person typed with a row of dots and send *those* as
        // the next value. ADR 0244 found it; since ADR 0247 `write_back` refuses it on the
        // answer's own `ShownValue::obscured` rather than on this control's kind.
        let secure = gtk4::PasswordEntry::new();
        secure.set_show_peek_icon(true);
        let name = field.name.qualified.clone();
        let suppress = Rc::clone(suppress);
        let change = Rc::clone(change);
        secure.connect_changed(move |secure| {
            if suppress.get() {
                return;
            }
            change(FieldChange::Set {
                field: name.clone(),
                value: Some(secure.text().to_string()),
            });
        });
        return secure.upcast();
    }
    let entry = gtk4::Entry::new();
    entry.set_text(&value);
    // Table 232's `/MaxLen`: "[t]he maximum length of the field's text, in characters". The
    // control enforces it, which is what a native one is for — the field enforces it too, and a
    // person who cannot type the eleventh character is better served than one whose eleventh
    // character disappears.
    if let Some(max) = max_len.and_then(|max| i32::try_from(max).ok()) {
        entry.set_max_length(max);
    }
    let suppress = Rc::clone(suppress);
    let change = Rc::clone(change);
    entry.connect_changed(move |entry| {
        if suppress.get() {
            return;
        }
        change(FieldChange::Set {
            field: name.clone(),
            value: Some(entry.text().to_string()),
        });
    });
    entry.upcast()
}

/// §12.7.5.2.3's check box and §12.7.5.2.4's radio button, which are one control with two rules.
fn toggle(
    field: &FormField,
    widget: &FormWidget,
    on: bool,
    no_toggle_to_off: bool,
    suppress: &Rc<Cell<bool>>,
    change: &Rc<dyn Fn(FieldChange)>,
) -> gtk4::Widget {
    let button = gtk4::CheckButton::new();
    button.set_active(widget.on || (on && widget.on_state.is_none()));
    let name = field.name.qualified.clone();
    let on_state = widget.on_state.clone();
    let suppress = Rc::clone(suppress);
    let change = Rc::clone(change);
    button.connect_toggled(move |button| {
        if suppress.get() {
            return;
        }
        let value = if button.is_active() {
            // §12.7.5.2.3: "[t]he value of the V key shall also be the value of the AS key",
            // and the on state is a name the *file* invented — which is why it crosses.
            let Some(state) = on_state.clone() else {
                eprintln!(
                    "note: the field {name} states no appearance for an on state (§12.7.5.2.3)"
                );
                return;
            };
            state
        } else {
            if no_toggle_to_off {
                // Table 229 bit 15: "selecting the currently selected button has no effect".
                return;
            }
            "Off".to_owned()
        };
        change(FieldChange::Set {
            field: name.clone(),
            value: Some(value),
        });
    });
    button.upcast()
}

/// §12.7.5.2.2's push button, "a purely interactive control … without retaining a permanent
/// value".
fn push(field: &FormField, widget: &FormWidget, change: &Rc<dyn Fn(FieldChange)>) -> gtk4::Widget {
    let label = field.name.shown();
    let button = gtk4::Button::with_label(if label.is_empty() { " " } else { label });
    let annotation = widget.annotation;
    let change = Rc::clone(change);
    button.connect_clicked(move |_| change(FieldChange::Activate(annotation)));
    button.upcast()
}

/// §12.7.5.4's combo box, which Table 233 bit 18 distinguishes from a list.
fn combo(
    field: &FormField,
    options: &[String],
    selected: Option<usize>,
    suppress: &Rc<Cell<bool>>,
    change: &Rc<dyn Fn(FieldChange)>,
) -> gtk4::Widget {
    let shown: Vec<&str> = options.iter().map(String::as_str).collect();
    let drop = gtk4::DropDown::from_strings(&shown);
    match selected.and_then(|index| u32::try_from(index).ok()) {
        Some(index) => drop.set_selected(index),
        None => drop.set_selected(gtk4::INVALID_LIST_POSITION),
    }
    let name = field.name.qualified.clone();
    let options: Vec<String> = options.to_vec();
    let suppress = Rc::clone(suppress);
    let change = Rc::clone(change);
    drop.connect_selected_notify(move |drop| {
        if suppress.get() {
            return;
        }
        let Ok(index) = usize::try_from(drop.selected()) else {
            return;
        };
        let Some(label) = options.get(index) else {
            return;
        };
        // §12.7.5.4: "the name string is the second of the two array elements", and `/V` names
        // that string rather than the export value.
        change(FieldChange::Set {
            field: name.clone(),
            value: Some(label.clone()),
        });
    });
    drop.upcast()
}

/// §12.7.5.4's list box, which the page draws nothing for.
///
/// `doc/todo/37`'s second smaller item: the clause "states which items are selected and states no
/// highlight, so `variable_text` refuses it. A host with the items and the selection can now draw
/// a real list — which is the point". This is that list, and it is the one control here that adds
/// something the page does not already show.
fn list(
    field: &FormField,
    options: &[String],
    selected: &[usize],
    multi: bool,
    suppress: &Rc<Cell<bool>>,
    change: &Rc<dyn Fn(FieldChange)>,
) -> gtk4::Widget {
    let shown: Vec<&str> = options.iter().map(String::as_str).collect();
    let strings = gtk4::StringList::new(&shown);
    let selection = gtk4::SingleSelection::new(Some(strings));
    selection.set_autoselect(false);
    selection.set_can_unselect(true);
    match selected
        .first()
        .and_then(|index| u32::try_from(*index).ok())
    {
        Some(index) => selection.set_selected(index),
        None => selection.set_selected(gtk4::INVALID_LIST_POSITION),
    }
    let factory = gtk4::SignalListItemFactory::new();
    factory.connect_bind(|_, item| {
        let Some(item) = item.downcast_ref::<gtk4::ListItem>() else {
            return;
        };
        let text = item
            .item()
            .and_downcast::<gtk4::StringObject>()
            .map(|held| held.string().to_string())
            .unwrap_or_default();
        let label = gtk4::Label::new(Some(&text));
        label.set_xalign(0.0);
        item.set_child(Some(&label));
    });
    let view = gtk4::ListView::new(Some(selection.clone()), Some(factory));
    let name = field.name.qualified.clone();
    let options: Vec<String> = options.to_vec();
    let suppress = Rc::clone(suppress);
    let change = Rc::clone(change);
    selection.connect_selected_notify(move |selection| {
        if suppress.get() {
            return;
        }
        let Ok(index) = usize::try_from(selection.selected()) else {
            return;
        };
        let Some(label) = options.get(index) else {
            return;
        };
        change(FieldChange::Set {
            field: name.clone(),
            value: Some(label.clone()),
        });
    });
    if multi {
        // Table 233 bit 22 permits several items at once and `Edit::SetField` carries one value,
        // so this host selects one and says so rather than pretending. The clause is not refused
        // here — it is a host that has not built the control for it.
        eprintln!(
            "note: the field {} allows several selections (Table 233 bit 22) and this host \
             offers one",
            field.name.shown()
        );
    }
    let scroller = gtk4::ScrolledWindow::new();
    scroller.set_child(Some(&view));
    scroller.upcast()
}
