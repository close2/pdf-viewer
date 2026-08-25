//! §12.7's form, as real GTK4 controls placed over the page.
//!
//! `doc/todo/30`'s second item, and the one that became possible in the
//! three-hundred-and-ninety-eighth session (ADR 0235): "real controls over `Query::Fields` — a
//! `GtkEntry` for a text field, a `GtkComboBoxText` for §12.7.5.4's combo box, a `GtkCheckButton`
//! for a check box". The combo box is a [`gtk4::DropDown`] here rather than a `GtkComboBoxText`,
//! which GTK deprecated in 4.10.
//!
//! **What this host could not do until the four-hundred-and-ninth session**: ask for the page
//! drawn *without* the widget appearances underneath these controls. §12.5.5's appearance streams
//! are page content, so the entry sat on top of the picture of an entry and a person saw the
//! field twice — the largest thing the boundary turned out to be missing (ADR 0244), and closed
//! by [`viewer_core::Command::Delegate`], which [`crate::Host`] sends on every open (ADR 0245).
//! (**This paragraph said the host "cannot do" it for three rounds after it could**, and it
//! pointed at a `doc/todo/37` the round that closed it deleted: `doc/todo/01`'s eighth sweep
//! found the dead pointer and its first and third found the claim behind it.)

use std::cell::Cell;
use std::rc::Rc;

use gtk4::prelude::*;
use pdf_syntax::ObjectId;
use viewer_core::{Entered, FormField, FormWidget};

use viewer_host::form::{Clicked, ControlKind, control_kind};

/// What a person did to a control, in the vocabulary the viewer takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FieldChange {
    /// [`viewer_core::Edit::SetField`], addressed by §12.7.4.2's fully qualified name.
    Set {
        /// The qualified name.
        field: String,
        /// The new value: characters, §12.7.5.4's chosen options, or nothing to clear it.
        value: Entered,
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
            options,
            selected,
            editable,
        } => combo(field, options, *selected, *editable, suppress, change),
        ControlKind::List {
            options,
            selected,
            multi,
            top,
        } => list(field, options, selected, *multi, *top, suppress, change),
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
                value: Entered::Text(text),
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
                value: Entered::Text(secure.text().to_string()),
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
            value: Entered::Text(entry.text().to_string()),
        });
    });
    entry.upcast()
}

/// §12.7.5.2.3's check box and §12.7.5.2.4's radio button, which are one control with two rules.
///
/// **What the rules *are* is [`viewer_host::toggling`] since the seven-hundred-and-thirty-fifth
/// session** (ADR 0630), shared with `viewer-qt`, with `viewer-ui` and with this host's own
/// accessibility click. This function had a copy of them and the copy had drifted: it never asked
/// Table 227 bit 1, on the reasoning that an insensitive control cannot be clicked — true of a
/// person's click and not of the two other ways one arrives.
///
/// **And a refused click puts the button back.** `connect_toggled` fires *after* GTK has flipped
/// the widget, so returning without an edit used to leave a `GtkCheckButton` showing a state the
/// field does not hold — a radio button of a set with `NoToggleToOff` unchecked itself on the
/// screen while `/V` still named it, which is the opposite of what Table 229 bit 15 requires.
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
    let name = field.name.clone();
    let read_only = field.read_only;
    let on_state = widget.on_state.clone();
    let suppress = Rc::clone(suppress);
    let change = Rc::clone(change);
    button.connect_toggled(move |button| {
        if suppress.get() {
            return;
        }
        let clicked = viewer_host::toggling(
            &name,
            read_only,
            button.is_active(),
            no_toggle_to_off,
            on_state.as_deref(),
        );
        // `false`: the click reached the control — it *is* the control's signal — so there is
        // nothing about a page coordinate to report.
        if let Some(said) = clicked.note(false) {
            eprintln!("note: {said}");
        }
        match clicked {
            Clicked::Toggles { name, value } => change(FieldChange::Set {
                field: name.qualified,
                value: Entered::Text(value),
            }),
            // Undo what GTK already did to the picture. Written as the inverse of what the button
            // now shows rather than as the field's own state, because the field is exactly what
            // this click did not change.
            Clicked::ReadOnly { .. }
            | Clicked::Stays { .. }
            | Clicked::Unnamed { .. }
            | Clicked::Pointed { .. }
            | Clicked::Aimed { .. }
            | Clicked::Page => {
                suppress.set(true);
                button.set_active(!button.is_active());
                suppress.set(false);
            }
        }
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

/// Which of a selection model's items are selected, as indices into Table 234's `/Opt`.
///
/// Asked position by position rather than through `GtkBitset`, because the two selection models
/// this host builds answer the same way and the list is as long as `/Opt` — 10 list-box widgets
/// over 8 corpus documents, and no `/Opt` in any of them is long enough for the difference to be
/// measurable.
fn chosen(selection: &gtk4::SelectionModel) -> Vec<usize> {
    (0..selection.n_items())
        .filter(|position| selection.is_selected(*position))
        .filter_map(|position| usize::try_from(position).ok())
        .collect()
}

/// §12.7.5.4's combo box, which Table 233 bit 18 distinguishes from a list.
///
/// **Table 233 bit 19 decides which of two controls this is, and it binds both ways** (ISO 32000-2
/// §12.7.5.4). If the bit is set the combo box "shall include an editable text box as well as a
/// drop-down list", and if it is clear it shall include only a drop-down list.
///
/// So a clear flag is not the absence of a requirement: only a drop-down list is what a
/// [`gtk4::DropDown`] is, and that is the control below. A set flag asks for two things at once,
/// and GTK4 has no single widget that is both — which is what `doc/todo/30` item 7 and ADR 0509
/// §3 called a toolkit floor for thirty-nine sessions.
///
/// **It is not a floor, and ADR 0508's rule is what found that: call the API before writing that
/// something is blocked on it.** The floor was read off the *widget list* — `GtkDropDown` has no
/// entry and `GtkComboBoxText` is deprecated in the release this crate binds — and the clause does
/// not ask for a widget. It asks for an editable text box *and* a drop-down list, which
/// [`gtk4::Entry`] beside a [`gtk4::MenuButton`] over a [`gtk4::ListBox`] is, in one `linked` box,
/// with nothing deprecated and the `v4_10` feature floor untouched. The composition is what GTK's
/// own migration note points at, and it is the same answer ADR 0508 reached one entry further up
/// this file: a toolkit that will not hand over a *widget* has usually not withheld the
/// *capability*.
fn combo(
    field: &FormField,
    options: &[String],
    selected: Option<usize>,
    editable: bool,
    suppress: &Rc<Cell<bool>>,
    change: &Rc<dyn Fn(FieldChange)>,
) -> gtk4::Widget {
    if editable {
        return editable_combo(field, options, selected, suppress, change);
    }
    let shown: Vec<&str> = options.iter().map(String::as_str).collect();
    let drop = gtk4::DropDown::from_strings(&shown);
    match selected.and_then(|index| u32::try_from(index).ok()) {
        Some(index) => drop.set_selected(index),
        None => drop.set_selected(gtk4::INVALID_LIST_POSITION),
    }
    let name = field.name.qualified.clone();
    let count = options.len();
    let suppress = Rc::clone(suppress);
    let change = Rc::clone(change);
    drop.connect_selected_notify(move |drop| {
        if suppress.get() {
            return;
        }
        let Ok(index) = usize::try_from(drop.selected()) else {
            return;
        };
        if index >= count {
            return;
        }
        // §12.7.5.4's item, named by its position in Table 234's `/Opt` rather than by the label
        // `/V` will hold. The clause makes the label the value — "the name string is the second of
        // the two array elements" — and the position is what says *which* label, where two entries
        // carry the same one. A drop-down list has no other value to send, which is Table 233 bit
        // 19 clear stated as a control rather than reported: there is nowhere here to type.
        change(FieldChange::Set {
            field: name.clone(),
            value: Entered::Chosen(vec![index]),
        });
    });
    drop.upcast()
}

/// Table 233 bit 19 set: "an editable text box as well as a drop-down list", composed.
///
/// The two halves send **different** values, and that is the clause rather than a convenience.
/// Typing sends characters, because §12.7.5.4 lets an editable combo box hold "a value other than
/// the predefined choices" and there is no index for such a value to be. Picking a row sends the
/// row's *position* in Table 234's `/Opt`, because two entries may carry the same name string and
/// a label could not say which was picked — the same reason the plain drop-down above sends one.
///
/// The entry is what the value is read back into ([`crate::host`]'s `write_back`), so §12.7.5.3's
/// truncation and a row picked from the list both arrive by the one road every other control's
/// value takes.
fn editable_combo(
    field: &FormField,
    options: &[String],
    selected: Option<usize>,
    suppress: &Rc<Cell<bool>>,
    change: &Rc<dyn Fn(FieldChange)>,
) -> gtk4::Widget {
    let name = field.name.qualified.clone();
    let entry = gtk4::Entry::new();
    entry.set_hexpand(true);
    // Before the signal is connected, so that seeding the control is not a keystroke.
    entry.set_text(field.value.as_ref().map_or("", |shown| shown.text.as_str()));
    {
        let name = name.clone();
        let suppress = Rc::clone(suppress);
        let change = Rc::clone(change);
        entry.connect_changed(move |entry| {
            if suppress.get() {
                return;
            }
            change(FieldChange::Set {
                field: name.clone(),
                value: Entered::Text(entry.text().to_string()),
            });
        });
    }
    let list = gtk4::ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::Single);
    for option in options {
        let label = gtk4::Label::new(Some(option));
        label.set_xalign(0.0);
        let row = gtk4::ListBoxRow::new();
        row.set_child(Some(&label));
        list.append(&row);
    }
    if let Some(row) = selected
        .and_then(|index| i32::try_from(index).ok())
        .and_then(|index| list.row_at_index(index))
    {
        list.select_row(Some(&row));
    }
    let scroller = gtk4::ScrolledWindow::new();
    scroller.set_child(Some(&list));
    scroller.set_propagate_natural_width(true);
    scroller.set_propagate_natural_height(true);
    // A widget's `/Rect` says nothing about how many options are behind it, and a form with a
    // hundred of them would otherwise ask for a popover taller than the screen.
    scroller.set_max_content_height(320);
    let popover = gtk4::Popover::new();
    popover.set_child(Some(&scroller));
    popover.set_has_arrow(false);
    let button = gtk4::MenuButton::new();
    button.set_icon_name("pan-down-symbolic");
    button.set_popover(Some(&popover));
    {
        let count = options.len();
        let change = Rc::clone(change);
        list.connect_row_activated(move |_, row| {
            let Ok(index) = usize::try_from(row.index()) else {
                return;
            };
            if index >= count {
                return;
            }
            popover.popdown();
            change(FieldChange::Set {
                field: name.clone(),
                value: Entered::Chosen(vec![index]),
            });
        });
    }
    let composed = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    // GTK's own style class for "these are one control", which is what makes the pair read as a
    // combo box rather than as an entry that happens to have a button beside it.
    composed.add_css_class("linked");
    composed.append(&entry);
    composed.append(&button);
    composed.upcast()
}

/// §12.7.5.4's list box, which the page draws nothing for.
///
/// ADR 0235's second smaller item: the clause states which items are selected and states no
/// highlight, so `variable_text` refuses it. A host with the items and the selection can draw a
/// real list — which is the point. This is that list, and it is the one control here that adds
/// something the page does not already show.
///
/// **And since the four-hundred-and-twelfth session it obeys Table 233 bit 22 rather than
/// reporting it.** The bit — "(PDF 1.4) If set, more than one of the field's option items may be
/// selected simultaneously; if clear, at most one item shall be selected" — decides which of GTK's
/// two selection models this is, and the model is the whole difference: `viewer_core::Edit` carries
/// a set of indices now, so a `GtkMultiSelection`'s answer has somewhere to go. ADR 0248.
///
/// **And since the six-hundred-and-seventy-eighth it obeys Table 234's `/TI`**, which is the last
/// thing this host owed §12.7.5.4 and which was owed to a *binding* rather than to a decision:
/// `GtkListView::scroll_to` is GTK 4.12 and this workspace bound `v4_10`. ADR 0508 raised the
/// floor. The entry is "the index in the Opt array of the first option visible in the list" — where
/// a scrollable list *starts*, which the clause makes a different question from which item is
/// selected, and which the page's own appearance has obeyed since ADR 0407.
fn list(
    field: &FormField,
    options: &[String],
    selected: &[usize],
    multi: bool,
    top: usize,
    suppress: &Rc<Cell<bool>>,
    change: &Rc<dyn Fn(FieldChange)>,
) -> gtk4::Widget {
    let shown: Vec<&str> = options.iter().map(String::as_str).collect();
    let strings = gtk4::StringList::new(&shown);
    let selection: gtk4::SelectionModel = if multi {
        gtk4::MultiSelection::new(Some(strings)).upcast()
    } else {
        let one = gtk4::SingleSelection::new(Some(strings));
        // "at most one item shall be selected", and a list box whose value is null has none —
        // which `GtkSingleSelection` only permits when told both of these.
        one.set_autoselect(false);
        one.set_can_unselect(true);
        one.upcast()
    };
    selection.unselect_all();
    for (nth, index) in selected.iter().enumerate() {
        if let Ok(index) = u32::try_from(*index) {
            // The first call clears whatever the model started with and the rest add to it, which
            // is one code path for both models: a single selection has exactly one iteration.
            selection.select_item(index, nth == 0);
        }
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
    let suppress = Rc::clone(suppress);
    let change = Rc::clone(change);
    selection.connect_selection_changed(move |selection, _, _| {
        if suppress.get() {
            return;
        }
        // Read out of the model rather than accumulated from the signal's range: the signal says
        // *which positions changed*, and what the edit needs is what is selected now.
        change(FieldChange::Set {
            field: name.clone(),
            value: Entered::Chosen(chosen(selection)),
        });
    });
    let scroller = gtk4::ScrolledWindow::new();
    scroller.set_child(Some(&view));
    scroll_to_top_index(&scroller, top, options.len());
    scroller.upcast()
}

/// Table 234's `/TI`, as a position for the list's own scrollbar (ISO 32000-2 §12.7.5.4).
///
/// > (Optional) For scrollable list boxes, the top index (the index in the Opt array of the first
/// > option visible in the list).
///
/// **The first *visible* option, which is not the selected one.** The clause makes them two
/// entries, and a control that read either for the other agrees with itself on every list where
/// they happen to be the same row — which is why the fixture states them differently. The page's
/// own appearance has obeyed the entry since ADR 0407, so a control placed over that picture
/// starting somewhere else is the same disagreement a stray mark would be.
///
/// **`GtkListView::scroll_to` is not this**, and that is the finding rather than the code (ADR
/// 0508). `doc/todo/30` recorded this host's debt as a *binding floor* — the method is GTK 4.12
/// and this workspace bound `v4_10` — but the method GTK gained does not say what Table 234 says:
/// its `GtkScrollInfo` argument is documented as "%NULL to scroll into view", `GtkScrollInfo`
/// carries only two booleans about which axes may move, and *into view* leaves an option that is
/// already visible exactly where it is. Qt's `QAbstractItemView::PositionAtTop` states the
/// position outright and GTK has no equivalent. Raised to `v4_12` and measured on the corpus's own
/// witness, the call moved nothing at `/TI 1` and nothing at `/TI 5`.
///
/// So the entry is applied where a scrollbar's position is stated in GTK, which is the
/// [`gtk4::Adjustment`] the [`gtk4::ScrolledWindow`] already owns:
///
/// - **the first `changed` with something to scroll is when the geometry exists.** `upper` and
///   `page-size` are nothing until the view is allocated, and the handler stands down for good
///   once it has acted, so a person who scrolls the list afterwards is not fought with. It also
///   stands down where `upper <= page-size`, which is the clause's own condition — the entry is
///   stated "[f]or scrollable list boxes", and a list showing all of its options is not one;
/// - **the row height is `upper / options`**, and that is exact rather than an estimate *here*:
///   every row of this control is built by one factory into one [`gtk4::Label`], so the rows are
///   uniform by construction. A list whose rows differed would need the position of the row
///   itself, which GTK exposes to nobody outside the widget;
/// - **`/TI` is a number the document states and nothing clamps.** An index the array does not
///   have is declined outright, because the page's own appearance clamps such an entry to the last
///   option (ADR 0111's rule, and
///   `variable_text.rs::the_top_index_says_which_option_the_list_starts_at`) and a control that
///   scrolled to its own end would be saying the same thing twice. The `min` is the other end of
///   the same arithmetic and is not that case: an index near the end of a long list asks for a
///   position past the furthest a viewport can scroll to, which is a valid entry and a clamp GTK
///   would apply anyway;
/// - **and the value is set on an idle rather than in the handler**, which is the one line here
///   that a reader would otherwise delete. `GtkListView` is a `GtkListBase`, and a `GtkListBase`
///   holds an *anchor item* and recomputes the adjustment from it every time it is allocated — so
///   a value written while the geometry is still being computed is overwritten by the anchor,
///   which is item 0 until somebody scrolls. Setting it from an idle puts it after GTK's layout
///   phase, where the adjustment moving is what *updates* the anchor. Measured both ways on the
///   corpus's witness: written in the handler the list still starts at option 0 with the trace
///   showing the value set correctly, and written from the idle it starts where `/TI` says.
fn scroll_to_top_index(scroller: &gtk4::ScrolledWindow, top: usize, options: usize) {
    if top == 0 || top >= options {
        return;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "an index into one field's /Opt array, far below 2^53"
    )]
    let fraction = top as f64 / options as f64;
    let applied = Cell::new(false);
    scroller.vadjustment().connect_changed(move |adjustment| {
        if applied.get() {
            return;
        }
        let (upper, page) = (adjustment.upper(), adjustment.page_size());
        if page <= 0.0 || upper <= page {
            return;
        }
        applied.set(true);
        let adjustment = adjustment.clone();
        gtk4::glib::idle_add_local_once(move || {
            adjustment.set_value((upper * fraction).min(upper - page));
        });
    });
}
