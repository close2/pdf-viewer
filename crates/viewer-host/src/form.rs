//! §12.7's fields, decided into the platform control each one is.
//!
//! **Toolkit-free on purpose**, for the reason `panel.rs` gives: this is the decision, and each
//! host's own `controls` module is what builds the widget. The decision is the interesting half —
//! it is exactly the question `doc/todo/37` asked, "could a host that wanted a real `GtkEntry`
//! build one without reaching into `viewer-ui` or re-deriving anything", and the answer is this
//! function. Two hosts have now built widgets from it and neither wanted it changed.
//!
//! `pdf_model::form`'s own module comment states the target: "[a] text field is a `QLineEdit`, an
//! `NSTextField`, a `GtkEntry`; a choice field is a combo box against a list". Two of those three
//! are now written down, against this one enum.

use pdf_model::form::{ChoiceControl, Control, TextControl};

/// Which platform control a field is.
///
/// One variant per control a toolkit has for the job, rather than one per §12.7.5 type: the clause's
/// choice field is two controls (Table 233 bit 18 decides which) and its button field is three
/// (§12.7.5.2 splits them), and a host that carried the clause's taxonomy would have to split it
/// again at the point where it builds a widget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlKind {
    /// §12.7.5.3's text field — a `GtkEntry` or a `QLineEdit`, or a `GtkTextView` or a
    /// `QPlainTextEdit` where Table 231 bit 13 makes it multiline.
    Entry {
        /// Bit 13: "the field may contain multiple lines of text".
        multiline: bool,
        /// Bit 14: "intended for entering a secure password that should not be echoed visibly to
        /// the screen". Both toolkits have an answer for this and they are different answers:
        /// GTK4 has a `GtkPasswordEntry`, a control of its own; Qt has `QLineEdit::Password`, an
        /// echo mode on the ordinary one.
        password: bool,
        /// Table 232's `/MaxLen`, which caps the control's own length.
        max_len: Option<u32>,
    },
    /// §12.7.5.2.3's check box — a `GtkCheckButton` or a `QCheckBox`.
    Check {
        /// Whether it is on.
        on: bool,
    },
    /// §12.7.5.2.4's radio button — a `GtkCheckButton` in a group, or a `QRadioButton`.
    Radio {
        /// Whether any widget of the set is on.
        on: bool,
        /// Table 229 bit 15, which decides what a click on the *selected* button does:
        /// "selecting the currently selected button has no effect".
        no_toggle_to_off: bool,
    },
    /// §12.7.5.2.2's push button — a `GtkButton` or a `QPushButton`, with no value at all.
    Push,
    /// §12.7.5.4's combo box — a `GtkDropDown` or a `QComboBox`.
    Combo {
        /// Table 234's `/Opt` labels, in the array's own order, which Table 233 bit 20 requires.
        options: Vec<String>,
        /// Which one is selected, where the value names one.
        selected: Option<usize>,
        /// Bit 19: "the combo box shall include an editable text box as well as a drop-down
        /// list". A `GtkDropDown` is not editable and a `QComboBox` is, which is the one place
        /// the two hosts differ in what they can obey rather than in how they spell it.
        editable: bool,
    },
    /// §12.7.5.4's list box — a `GtkListView` or a `QListWidget` over its items.
    List {
        /// Table 234's `/Opt` labels.
        options: Vec<String>,
        /// Which are selected.
        selected: Vec<usize>,
        /// Bit 22: "more than one of the field's option items may be selected simultaneously".
        multi: bool,
    },
    /// §12.7.5.5's signature field, which has no control and no value to type.
    Signature,
    /// Table 226 makes `/FT` required and this field states none anywhere in its ancestry.
    ///
    /// A row rather than a guess: the widget is on the page and has a rectangle, so leaving it
    /// out would show a form with a hole in it, and choosing one of the four would be inventing
    /// what the file did not say.
    Unstated,
}

/// The control a field is, from what `viewer_core::Query::Fields` said about it.
#[must_use]
pub fn control_kind(control: &Control) -> ControlKind {
    match control {
        Control::PushButton => ControlKind::Push,
        Control::CheckBox { on } => ControlKind::Check { on: *on },
        Control::RadioButton {
            on,
            no_toggle_to_off,
            ..
        } => ControlKind::Radio {
            on: *on,
            no_toggle_to_off: *no_toggle_to_off,
        },
        Control::Text(text) => entry_of(*text),
        Control::Choice(choice) => choice_of(choice),
        Control::Signature => ControlKind::Signature,
        Control::Unstated => ControlKind::Unstated,
    }
}

/// §12.7.5.3's text field.
///
/// Table 232's comb count is deliberately not a separate control: §12.7.4.3 lays a comb's value
/// out in equally spaced cells and that is a fact about how the *page* draws it, not about what
/// a person types into — so a native entry over a comb field is still an entry, and the cells
/// stay in the appearance underneath it.
fn entry_of(text: TextControl) -> ControlKind {
    ControlKind::Entry {
        multiline: text.multiline,
        password: text.password,
        max_len: text.max_len,
    }
}

/// §12.7.5.4's choice field, which Table 233 bit 18 splits into two controls.
fn choice_of(choice: &ChoiceControl) -> ControlKind {
    let options: Vec<String> = choice
        .options
        .iter()
        .map(|option| option.label.clone())
        .collect();
    if choice.combo {
        ControlKind::Combo {
            options,
            // A combo box holds one value; §12.7.5.4's `/I` may name several and the clause lets
            // it, because bit 22 is a *list* box's flag. The first is what a drop-down can show.
            selected: choice.selected.first().copied(),
            editable: choice.editable,
        }
    } else {
        ControlKind::List {
            options,
            selected: choice.selected.clone(),
            multi: choice.multi_select,
        }
    }
}
