//! §12.7's fields, decided into the platform control each one is.
//!
//! **Toolkit-free on purpose**, for the reason `panel.rs` gives: this is the decision, and each
//! host's own `controls` module is what builds the widget. The decision is the interesting half —
//! it is exactly the question ADR 0235's audit asked — could a host that wanted a real
//! `GtkEntry` build one without reaching into `viewer-ui` or re-deriving anything — and the
//! answer is this function. Two hosts have now built widgets from it and neither wanted it
//! changed.
//!
//! `pdf_model::form`'s own module comment states the target: "[a] text field is a `QLineEdit`, an
//! `NSTextField`, a `GtkEntry`; a choice field is a combo box against a list". Two of those three
//! are now written down, against this one enum.
//!
//! # And what a *click* on one comes to
//!
//! [`Clicked`] is the second decision here and it arrived last, in the
//! seven-hundred-and-thirty-fifth session (ADR 0630), for the reason the first one did: it had been
//! written three times. `viewer-ui` decided §12.7.5.2 from a point on the page, `viewer-gtk` from a
//! `GtkCheckButton`'s new state and `viewer-qt` from a `QAbstractButton`'s — and by the time
//! anybody compared them they disagreed, because only one of the three asked Table 227 bit 1 before
//! sending an edit. The others were relying on an insensitive control, which is a fact about a
//! *person's* click and not about the two other ways one now arrives.
//!
//! There are two ways in, because there are two things that know a click happened, and both end at
//! one rule. [`toggling`] is the rule: one widget's flags, and the state the click asked for.
//! [`clicked`] is the walk that finds the widget under a point and applies it — which is what an
//! assistive technology's `Action.DoAction` needs, since §14.7.5.3's object reference names an
//! element rather than a control (ADR 0425).

use pdf_model::form::{ChoiceControl, Control, TextControl};
use pdf_model::view::FieldName;
use viewer_core::{Answer, Query, Viewer};

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
        /// list; if clear, it shall include only a drop-down list".
        ///
        /// **Both halves are `shall`**, which is why this is not a flag a host may report and
        /// leave: a clear bit forbids the text box as plainly as a set one requires it.
        /// [`ControlKind::takes_typed_characters`] is where that sentence is stated once for
        /// every host.
        ///
        /// (This comment said a `GtkDropDown` is not editable and a `QComboBox` is, "the one
        /// place the two hosts differ in what they can obey". The first clause is still true of
        /// the *widget* and the conclusion was never true of the toolkit: `viewer-gtk` composes
        /// an entry and a drop-down list since the seven-hundred-and-seventeenth session, and
        /// nothing about the feature floor moved to allow it.)
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
        /// Table 234's `/TI`: "the index in the Opt array of the first option visible in the
        /// list".
        ///
        /// **Read by `pdf-model` since the three-hundred-and-ninety-eighth session and dropped
        /// here until the six-hundred-and-first**, which is `doc/habits.md`'s fifth sweep exactly:
        /// the model implements it, and the question nobody asked was who calls it. The page's own
        /// appearance obeys the entry (ADR 0407), so a host that started its list at row 0 showed
        /// a different first option than the picture underneath it.
        top: usize,
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

impl ControlKind {
    /// Whether a keyboard may put characters into this control at all.
    ///
    /// **Here because it is a clause and not a toolkit's manners.** A host that places somebody
    /// else's widgets answers this by *choosing the widget* — a `GtkEntry` takes characters and a
    /// `GtkDropDown` has nowhere to put one — so `viewer-gtk` and `viewer-qt` obey by
    /// construction. A tier-2 host draws the page's own appearance and has no widget to be
    /// constrained by, so it has to ask; and asking a shared function is what stops the third host
    /// answering differently from the other two.
    ///
    /// ISO 32000-2 §12.7.5.4's Table 233 bit 19 decides it for a combo box, and it is a `shall` in
    /// both directions: if the bit is set the box "shall include an editable text box as well as a
    /// drop-down list", and if it is clear it shall include only a drop-down list.
    ///
    /// §12.7.5.4 says the same thing as a permission and names what the text box is *for*: "[t]he
    /// combo box may be accompanied by an editable text box in which the user can type a value
    /// other than the predefined choices, as directed by the value of the Edit bit in the Ff
    /// entry". So a value outside Table 234's `/Opt` is exactly what a set bit admits and a clear
    /// bit excludes.
    ///
    /// §12.7.5.4's list box takes none either, and for a reason the clause states rather than by
    /// analogy: its value "identifies the item or items currently selected", "as given in the
    /// field dictionary's Opt array".
    ///
    /// §12.7.5.2's buttons select an appearance state and §12.7.5.5's signature holds a
    /// dictionary; neither is text a person types.
    #[must_use]
    pub fn takes_typed_characters(&self) -> bool {
        match self {
            Self::Entry { .. } => true,
            Self::Combo { editable, .. } => *editable,
            Self::Check { .. }
            | Self::Radio { .. }
            | Self::Push
            | Self::List { .. }
            | Self::Signature
            | Self::Unstated => false,
        }
    }
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
            top: choice.top,
        }
    }
}

/// What a click on a point of the page comes to, for §12.7's widgets.
///
/// **One definition of §12.7.5.2's rule for three windows, and it exists because there were
/// three.** `viewer-ui` decided it from a point, `viewer-gtk` from a `GtkCheckButton`'s new state
/// and `viewer-qt` from a `QAbstractButton`'s, and the three copies had already drifted: only one
/// of them consulted Table 227 bit 1 before sending an edit. This crate's founding sentence is
/// that the third copy of a decision is where two hosts stop agreeing about a document.
///
/// **Matched exhaustively in every host**, which is `doc/todo/30`'s "all three hosts stay level"
/// made a compile error: a case added here fails to build in three windows until each says what it
/// does about it.
///
/// Every variant but [`Clicked::Page`] names the field, because every one of them is something to
/// say to a person — §14.9.3 makes [`pdf_model::view::FieldName::shown`] the name to say it with,
/// and [`pdf_model::view::FieldName::qualified`] is what [`viewer_core::Edit::SetField`]
/// addresses. [`Clicked::note`] is the sentence, worded once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Clicked {
    /// §12.7.5.2's two toggling kinds: give the field this appearance-state name.
    ///
    /// The value is a name Table 170's appearance dictionary is keyed by, which §12.7.5.2.3 makes
    /// `/V` — "a name object representing the check box's appearance state, which shall be used to
    /// select the appropriate appearance from the appearance dictionary" — or `Off`, which
    /// §12.7.5.2.3 names and §12.7.5.2.4 gives as the default value.
    Toggles {
        /// The field to address the edit to, and to name in a report.
        name: FieldName,
        /// The appearance state to set.
        value: String,
    },
    /// Table 227 bit 1: "the field shall not be modified by the user".
    ReadOnly {
        /// The field the document locked.
        name: FieldName,
    },
    /// Table 229 bit 15 on a button that is already on. ISO 32000-2 §12.7.5.2.1's Table 229:
    ///
    /// > If set, exactly one radio button shall be selected at all times; selecting the currently
    /// > selected button has no effect.
    Stays {
        /// The button that stays on.
        name: FieldName,
    },
    /// §12.7.5.2.3's on state, which this widget's appearance dictionary does not name.
    ///
    /// The clause makes each state's appearance "an appearance stream in the appearance dictionary
    /// of the field's widget annotation", and the names are the file's own invention — so a widget
    /// whose `/AP /N` holds nothing but `Off` states no on state and there is no name to send.
    Unnamed {
        /// The widget that states no on state.
        name: FieldName,
    },
    /// A control a click reaches through the pointer and gives no value to.
    ///
    /// §12.7.5.2.2's push-button, which "responds immediately to user input without retaining a
    /// permanent value"; §12.7.5.5's signature field, whose value is a dictionary; and the field
    /// Table 226 requires a `/FT` of and whose ancestry states none. What a click does to one of
    /// these is §12.6.3's triggers and §12.5.5's appearance, which `Command::Pointer` already
    /// carries in all three hosts.
    Pointed {
        /// The field the pointer went to.
        name: FieldName,
    },
    /// A control a click aims a **keyboard** or a **list** at rather than giving a value to.
    ///
    /// §12.7.5.3's text field and both of §12.7.5.4's — what a click on one asks for is a caret at
    /// the point it landed, or Table 234's options on the screen. A host drawing the page's own
    /// appearance does that itself; a host that sent `Command::Delegate` has a real `GtkEntry` or
    /// `QComboBox` over the widget and a synthetic press at a page coordinate goes *past* it to the
    /// page underneath, which is what [`Clicked::note`] says out loud rather than leaving silent.
    Aimed {
        /// The control the click was aimed at.
        name: FieldName,
    },
    /// No §12.7 widget under the point: the click belongs to the page.
    Page,
}

impl Clicked {
    /// What to say about a click this did not carry out, or `None` where it did.
    ///
    /// **The refusals are worded here rather than in a host** for the reason every other sentence
    /// in this crate is: three windows saying three things about one clause is how a person
    /// discovers that two of them read it differently. Trap 5's rule — an input this program will
    /// not act on is named rather than dropped — with the clause number beside each.
    ///
    /// `placed` is whether the caller puts a **real control** over the widget, and it changes
    /// exactly one answer: [`Clicked::Aimed`]. A host drawing the page's own appearance puts a
    /// caret in the value or opens Table 234's list itself and has nothing to report; a host whose
    /// control is a `GtkEntry` or a `QComboBox` cannot reach it from a page coordinate at all,
    /// which is the half of `doc/todo/31` a bus measurement found (ADR 0623).
    ///
    /// It is deliberately **not** [`viewer_core::Command::Delegate`]'s value: both native hosts
    /// place their controls whether or not the page's own appearance is drawn underneath, so what
    /// decides this is which host is asking rather than which picture it asked for.
    #[must_use]
    pub fn note(&self, placed: bool) -> Option<String> {
        match self {
            Self::Toggles { .. } | Self::Pointed { .. } | Self::Page => None,
            Self::ReadOnly { name } => Some(format!(
                "the field {} is read-only (Table 227), so a click gives it no value",
                name.shown()
            )),
            Self::Stays { name } => Some(format!(
                "the field {} keeps NoToggleToOff set, and \"selecting the currently selected \
                 button has no effect\" (Table 229 bit 15, §12.7.5.2.4)",
                name.shown()
            )),
            Self::Unnamed { name } => Some(format!(
                "the field {} states no appearance for an on state (§12.7.5.2.3), so there is no \
                 name to give it",
                name.shown()
            )),
            Self::Aimed { name } => placed.then(|| {
                format!(
                    "the field {} is a control this window places rather than a picture on the \
                     page, so a click asked for at a page coordinate does not reach it \
                     (§12.7.5.3, §12.7.5.4, doc/todo/31)",
                    name.shown()
                )
            }),
        }
    }
}

/// §12.7.5.2's rule, given what one widget of a button field is and what the click asked of it.
///
/// **The rule alone, so that the two ways into it cannot answer differently.** A host drawing the
/// page decides from a point ([`clicked`]); a host placing real controls is told by the toolkit
/// that a `GtkCheckButton` or a `QAbstractButton` has just gone to `wanted`. Those are different
/// questions about the same clause, and both arrive here.
///
/// `no_toggle_to_off` is Table 229 bit 15 and is a **radio button's only**, which the table says in
/// its own first three words: a check box passes `false` however its flags read.
///
/// Table 229 bit 15 states both halves of the last decision, and it is printed under ISO 32000-2
/// §12.7.5.2.1:
///
/// > If set, exactly one radio button shall be selected at all times; selecting the currently
/// > selected button has no effect. If clear, clicking the selected button deselects it, leaving
/// > no button selected.
#[must_use]
pub fn toggling(
    name: &FieldName,
    read_only: bool,
    wanted: bool,
    no_toggle_to_off: bool,
    on_state: Option<&str>,
) -> Clicked {
    // Table 227 bit 1 first, and checked here as well as in the core: the core refuses the edit,
    // and a host that sent one anyway would be a program that looks broken rather than one that
    // obeys the document.
    if read_only {
        return Clicked::ReadOnly { name: name.clone() };
    }
    if !wanted {
        if no_toggle_to_off {
            return Clicked::Stays { name: name.clone() };
        }
        return Clicked::Toggles {
            name: name.clone(),
            value: OFF.to_owned(),
        };
    }
    let Some(state) = on_state else {
        return Clicked::Unnamed { name: name.clone() };
    };
    Clicked::Toggles {
        name: name.clone(),
        value: state.to_owned(),
    }
}

/// §12.7.5.2.3's name for the state a button is *not* in.
///
/// ISO 32000-2 §12.7.5.2.3: "[t]he appearance for the off state is optional but, if present, shall
/// be stored in the appearance dictionary under the name Off." §12.7.5.2.4 gives the same name as
/// a radio button field's default value, which is this string read forwards.
const OFF: &str = "Off";

/// What a click at a point of the page's viewport comes to, asked of the viewer.
///
/// **Two questions and not one**, for the reason `viewer-ui` has asked them in this order since
/// ADR 0235: [`viewer_core::Query::FieldAt`] is the *model's own* hit test — a second one here
/// could disagree with it — and [`viewer_core::Query::Fields`] is what says which of §12.7.5's
/// types the field is and what Tables 227 and 229 make of it, which nothing in the first answer
/// carries.
///
/// Which widget: the **last** one covering the point, because §12.5.2 draws them in `/Annots`
/// order and the one on top is the one under the pointer — the same rule
/// `pdf_model::view::field_at` applies one level down. That is the question §12.7.5.2.4's set makes
/// real: "at most one button in a set may be on at any given time", and the one that goes on is the
/// one under the pointer rather than the field's first.
///
/// Answers [`Clicked::Page`] for a point on no widget, which is not a refusal: a click on the page
/// is what almost every click is.
#[must_use]
pub fn clicked(viewer: &Viewer, at: (f32, f32)) -> Clicked {
    let Answer::Field { name, .. } = viewer.query(Query::FieldAt(at)) else {
        return Clicked::Page;
    };
    let Answer::Fields(fields) = viewer.query(Query::Fields) else {
        return Clicked::Page;
    };
    let Some(field) = fields
        .iter()
        .find(|field| field.name.qualified == name.qualified)
    else {
        return Clicked::Page;
    };
    let no_toggle_to_off = match &field.control {
        // Table 229 bit 15 is "(Radio buttons only)", so a check box's flags cannot reach it.
        Control::CheckBox { .. } => false,
        Control::RadioButton {
            no_toggle_to_off, ..
        } => *no_toggle_to_off,
        Control::PushButton | Control::Signature | Control::Unstated => {
            return Clicked::Pointed {
                name: field.name.clone(),
            };
        }
        Control::Text(_) | Control::Choice(_) => {
            return Clicked::Aimed {
                name: field.name.clone(),
            };
        }
    };
    let Some(widget) = field
        .widgets
        .iter()
        .rev()
        .find(|widget| crate::geometry::covers(widget.quad, at))
    else {
        // The model's hit test found the field and none of its widgets on *this* page covers the
        // point, which a column of pages can produce: the field is one and its widgets are many.
        return Clicked::Page;
    };
    toggling(
        &field.name,
        field.read_only,
        !widget.on,
        no_toggle_to_off,
        widget.on_state.as_deref(),
    )
}

#[cfg(test)]
mod tests {
    use super::{Clicked, toggling};
    use pdf_model::view::FieldName;

    /// A field with both of §14.9.3's names, so that a note is checked against the one it shows.
    fn named() -> FieldName {
        FieldName {
            qualified: "form1[0].page1[0].box[0]".to_owned(),
            alternative: Some("Check box, unchecked".to_owned()),
        }
    }

    /// §12.7.5.2.3: the on state is a name the file invented, and it is what `/V` becomes.
    #[test]
    fn turning_a_box_on_sends_the_name_its_appearance_dictionary_is_keyed_by() {
        assert_eq!(
            toggling(&named(), false, true, false, Some("Yes")),
            Clicked::Toggles {
                name: named(),
                value: "Yes".to_owned(),
            }
        );
    }

    /// §12.7.5.2.3 names the off state and §12.7.5.2.4 gives it as the default value.
    #[test]
    fn turning_a_box_off_sends_the_name_the_clause_states() {
        assert_eq!(
            toggling(&named(), false, false, false, Some("Yes")),
            Clicked::Toggles {
                name: named(),
                value: "Off".to_owned(),
            }
        );
    }

    /// Table 229 bit 15: "selecting the currently selected button has no effect".
    ///
    /// The flag reaches this function only for a radio button, which is the table's own first
    /// three words — so the same call with the flag clear is the arm above and not this one.
    #[test]
    fn a_radio_button_with_no_toggle_to_off_stays_on() {
        assert_eq!(
            toggling(&named(), false, false, true, Some("1")),
            Clicked::Stays { name: named() }
        );
        // And it does not stand in the way of turning one *on*, which is the half of the flag a
        // refusal keyed on the flag alone would have broken.
        assert_eq!(
            toggling(&named(), false, true, true, Some("1")),
            Clicked::Toggles {
                name: named(),
                value: "1".to_owned(),
            }
        );
    }

    /// Table 227 bit 1 comes before every other answer, including the one that changes nothing.
    #[test]
    fn a_read_only_field_is_refused_before_the_clause_is_asked() {
        assert_eq!(
            toggling(&named(), true, true, false, Some("Yes")),
            Clicked::ReadOnly { name: named() }
        );
        assert_eq!(
            toggling(&named(), true, false, true, None),
            Clicked::ReadOnly { name: named() }
        );
    }

    /// A widget whose `/AP` names no on state has no name to send, and says so.
    #[test]
    fn a_widget_that_names_no_on_state_is_reported_rather_than_invented() {
        assert_eq!(
            toggling(&named(), false, true, false, None),
            Clicked::Unnamed { name: named() }
        );
    }

    /// The refusals say §14.9.3's name and the clause; a click that went through says nothing.
    #[test]
    fn every_refusal_is_named_and_nothing_else_is() {
        for placed in [false, true] {
            assert!(
                Clicked::ReadOnly { name: named() }
                    .note(placed)
                    .is_some_and(
                        |said| said.contains("Check box, unchecked") && said.contains("Table 227")
                    )
            );
            assert!(Clicked::Stays { name: named() }.note(placed).is_some());
            assert!(Clicked::Unnamed { name: named() }.note(placed).is_some());
            assert!(
                Clicked::Toggles {
                    name: named(),
                    value: "Yes".to_owned(),
                }
                .note(placed)
                .is_none()
            );
            assert!(Clicked::Pointed { name: named() }.note(placed).is_none());
            assert!(Clicked::Page.note(placed).is_none());
        }
        // The one answer that depends on who is asking: a host drawing the page's own appearance
        // aims a caret at the value itself, and a host with a real `GtkEntry` over the widget
        // cannot reach it from a page coordinate (ADR 0623).
        assert!(Clicked::Aimed { name: named() }.note(false).is_none());
        assert!(Clicked::Aimed { name: named() }.note(true).is_some());
    }
}
