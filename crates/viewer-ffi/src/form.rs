//! §12.7's form, flattened so that a C caller can build real controls over the page.
//!
//! **The sixth chrome population, and the last one to reach this ABI.** `viewer_core::Query::Fields`
//! answers with every field that has a widget on the page being shown — §12.7.5's four types, the
//! flags of Tables 227, 229, 231 and 233 that decide what kind of control it is, Table 234's items
//! and selection, Table 232's `/MaxLen`, and the appearance-state name §12.7.5.2.3 makes a check
//! box's value (ADR 0235). Until this module a C caller could only take a form as pixels off the
//! raster, which is exactly the thing ADR 0235's audit of the six chrome populations found and the
//! two Rust hosts fixed. (This sentence cited `doc/todo/37`, which the four-hundred-and-ninth
//! session deleted when `Command::Delegate` closed the last item in it — the eighth sweep's own
//! shape, and the pointer was written a hundred sessions after the file went.)
//!
//! Three decisions, each because C takes something away that Rust gave.
//!
//! **The control is `viewer_host::ControlKind`, not `pdf_model::form::Control`.** ADR 0246's third
//! decision is that a native host on this boundary is mostly not toolkit code, and that crate is
//! where the mapping from four clause types to eight platform controls already lives — the choice
//! field is two controls and the button field is three. A C host is a native host; it takes the
//! decision rather than making a fourth copy of it. This is the first consumer of `viewer-host`
//! outside the three Rust hosts, and the finding is that it wanted the module unchanged.
//!
//! **Every boolean is one `uint32_t`, not a field apiece.** Sixteen flags across four tables would
//! be sixteen entry points or a struct passed by value, and a struct passed by value is the one
//! change this ABI cannot make cheaply (`PDFV_ABI_VERSION`). A bit added later is a bit an old
//! caller does not read, which costs it nothing — the same property the whole shape was chosen for.
//!
//! **`None` and `Some("")` stay two answers.** A field with no text value at all — a button selects
//! an appearance, a signature holds a dictionary — answers [`Status::NoAnswer`], and a text field
//! with nothing in it answers the empty string. A host deciding where to send the keyboard needs
//! exactly that distinction, and folding them together is the silence trap 5 is about.

use viewer_core::{FormField, FormWidget};
use viewer_host::control_kind;

use crate::kinds::{ControlKind, TextKind};
use crate::status::Status;

/// Table 227 bit 1: the document forbidding a person to change this field's value.
pub const FLAG_READ_ONLY: u32 = 1 << 0;
/// Table 227 bit 2: §12.7.6.2 requires a value at export.
pub const FLAG_REQUIRED: u32 = 1 << 1;
/// Table 227 bit 3: §12.7.6.2 may not export this field.
pub const FLAG_NO_EXPORT: u32 = 1 << 2;
/// Table 231 bit 13: "the field may contain multiple lines of text".
pub const FLAG_MULTILINE: u32 = 1 << 3;
/// Table 231 bit 14: a secure entry, whose value crosses as bullets.
pub const FLAG_PASSWORD: u32 = 1 << 4;
/// Table 231 bit 21: the text "represents the pathname of a file".
pub const FLAG_FILE_SELECT: u32 = 1 << 5;
/// Table 231 bit 23 and Table 233 bit 23: "shall not be spell-checked".
pub const FLAG_DO_NOT_SPELL_CHECK: u32 = 1 << 6;
/// Table 231 bit 24: "the field shall not scroll … to accommodate more text than fits".
pub const FLAG_DO_NOT_SCROLL: u32 = 1 << 7;
/// Table 231 bit 25: the field is divided into `pdfv_field_limits`' cells.
pub const FLAG_COMB: u32 = 1 << 8;
/// Table 231 bit 26: "the value of this field shall be a rich text string". `CLAUDE.md` excludes
/// XFA, so the value answered is Table 226's plain `/V` and this bit is how a host may decline.
pub const FLAG_RICH_TEXT: u32 = 1 << 9;
/// Table 229 bit 15: "selecting the currently selected button has no effect".
pub const FLAG_NO_TOGGLE_TO_OFF: u32 = 1 << 10;
/// Table 229 bit 26: buttons of the set sharing an on state "will turn on and off in unison".
pub const FLAG_RADIOS_IN_UNISON: u32 = 1 << 11;
/// §12.7.5.2: the field is in its on state.
pub const FLAG_ON: u32 = 1 << 12;
/// Table 233 bit 19: "the combo box shall include an editable text box".
pub const FLAG_EDITABLE: u32 = 1 << 13;
/// Table 233 bit 22: "more than one of the field's option items may be selected simultaneously".
pub const FLAG_MULTI_SELECT: u32 = 1 << 14;
/// Table 233 bit 27: "the new value shall be committed as soon as a selection is made".
pub const FLAG_COMMIT_ON_SELECTION: u32 = 1 << 15;
/// The value `pdfv_field_value` answers with is Table 231 bit 14's echo, not the characters.
///
/// **A host obeying ADR 0201's read-back rule must consult it**: writing the bullets back into a
/// password control would send those bullets as the next value, which is the bug ADR 0247 found in
/// `viewer-ui` and the reason `ShownValue` carries the flag beside the string it describes.
pub const FLAG_OBSCURED: u32 = 1 << 16;

/// Where one widget of a field sits, and whether it is on.
///
/// A struct rather than three values loose, because three of anything in a return type is where a
/// reader starts having to count positions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacedWidget {
    /// The annotation, as §7.3.10's two numbers.
    pub object: (u32, u16),
    /// Its `/Rect` on the screen, `[x0, y0, … x3, y3]` in device pixels, y downwards.
    pub quad: [f32; 8],
    /// Whether this widget is in its on state.
    pub on: bool,
}

/// §12.7's fields on the page being shown, owned.
#[derive(Debug, Clone, PartialEq)]
pub struct Form {
    /// One entry per field, in the `/Annots` order `pdf_model::form::fields` answers in.
    fields: Vec<Entry>,
}

/// One field.
#[derive(Debug, Clone, PartialEq)]
struct Entry {
    /// §12.7.4.2's fully qualified name.
    qualified: String,
    /// What §14.9.3 requires a user interface to show.
    shown: String,
    /// Table 226's `/T`.
    partial: String,
    /// Which platform control it is.
    kind: ControlKind,
    /// Every boolean of Tables 227, 229, 231 and 233 that crosses, as one word.
    flags: u32,
    /// What the field says now, where §12.7.4.3 lays a text value out for it.
    value: Option<String>,
    /// Table 232's `/MaxLen`, and Table 231 bit 25's cell count where the field is a comb.
    limits: (Option<u32>, Option<u32>),
    /// Table 234's `/Opt`, in the array's own order, which Table 233 bit 20 requires.
    options: Vec<Option_>,
    /// The widgets of this field on the page being shown, in `/Annots` order.
    widgets: Vec<Placed>,
}

/// One of Table 234's options.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Option_ {
    /// What is shown.
    label: String,
    /// What §12.7.6.2 exports, where the entry is a pair; the label otherwise.
    export: String,
    /// Whether §12.7.5.4's value selects it.
    selected: bool,
}

/// One widget annotation, placed on the screen.
#[derive(Debug, Clone, PartialEq)]
struct Placed {
    /// The annotation, which `pdfv_activate` and `pdfv_set_group` name.
    object: (u32, u16),
    /// Its `/Rect` on the screen, `[x0, y0, … x3, y3]`, y downwards.
    quad: [f32; 8],
    /// Whether this widget is in its on state.
    on: bool,
    /// The `/AP /N` entry that turns it on — what `pdfv_set_field_text` sends to check the box.
    on_state: String,
    /// Table 230's `/Opt` entry for this widget: what §12.7.6.2 would export for it.
    export: String,
}

impl Form {
    /// Flattens what [`viewer_core::Answer::Fields`] answered with.
    #[must_use]
    pub fn of(fields: &[FormField]) -> Self {
        Self {
            fields: fields.iter().map(Entry::of).collect(),
        }
    }

    /// How many fields there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Whether there are none, which is most pages of most documents.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// One of the three names a field carries.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such field, [`Status::WrongKind`] for a `which`
    /// that names an option's string rather than a field's.
    pub fn name(&self, field: usize, which: TextKind) -> Result<&str, Status> {
        let entry = self.fields.get(field).ok_or(Status::OutOfRange)?;
        Ok(match which {
            TextKind::Qualified => entry.qualified.as_str(),
            TextKind::Shown => entry.shown.as_str(),
            TextKind::Partial => entry.partial.as_str(),
            TextKind::Label | TextKind::Export => return Err(Status::WrongKind),
        })
    }

    /// Which control the field is, and every boolean that decides how it is built.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such field.
    pub fn control(&self, field: usize) -> Result<(ControlKind, u32), Status> {
        self.fields
            .get(field)
            .map(|entry| (entry.kind, entry.flags))
            .ok_or(Status::OutOfRange)
    }

    /// Table 232's `/MaxLen` and Table 231 bit 25's cell count, zero where the field states none.
    ///
    /// Zero rather than a sentinel because both are lengths and neither may be zero: Table 231
    /// bit 25 is permitted only where `/MaxLen` "is present", and a comb of no cells is not a comb.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such field.
    pub fn limits(&self, field: usize) -> Result<(u32, u32), Status> {
        self.fields
            .get(field)
            .map(|entry| {
                (
                    entry.limits.0.unwrap_or_default(),
                    entry.limits.1.unwrap_or_default(),
                )
            })
            .ok_or(Status::OutOfRange)
    }

    /// What the field says now.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such field, and [`Status::NoAnswer`] where the
    /// field has no text value at all — which is a different answer from the empty string.
    pub fn value(&self, field: usize) -> Result<&str, Status> {
        self.fields
            .get(field)
            .ok_or(Status::OutOfRange)?
            .value
            .as_deref()
            .ok_or(Status::NoAnswer)
    }

    /// How many of Table 234's options the field states.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such field.
    pub fn option_count(&self, field: usize) -> Result<usize, Status> {
        self.fields
            .get(field)
            .map(|entry| entry.options.len())
            .ok_or(Status::OutOfRange)
    }

    /// One option's label or export value.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such field or option, [`Status::WrongKind`] for a
    /// `which` that names a field's string rather than an option's.
    pub fn option(&self, field: usize, option: usize, which: TextKind) -> Result<&str, Status> {
        let entry = self.fields.get(field).ok_or(Status::OutOfRange)?;
        let option = entry.options.get(option).ok_or(Status::OutOfRange)?;
        match which {
            TextKind::Label => Ok(option.label.as_str()),
            TextKind::Export => Ok(option.export.as_str()),
            TextKind::Qualified | TextKind::Shown | TextKind::Partial => Err(Status::WrongKind),
        }
    }

    /// Whether §12.7.5.4's value selects the option.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such field or option.
    pub fn option_selected(&self, field: usize, option: usize) -> Result<bool, Status> {
        self.fields
            .get(field)
            .ok_or(Status::OutOfRange)?
            .options
            .get(option)
            .map(|option| option.selected)
            .ok_or(Status::OutOfRange)
    }

    /// How many widgets of the field are on the page being shown.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such field.
    pub fn widget_count(&self, field: usize) -> Result<usize, Status> {
        self.fields
            .get(field)
            .map(|entry| entry.widgets.len())
            .ok_or(Status::OutOfRange)
    }

    /// One widget's object, its place on the screen, and whether it is on.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such field or widget.
    pub fn widget(&self, field: usize, widget: usize) -> Result<PlacedWidget, Status> {
        self.placed(field, widget).map(|placed| PlacedWidget {
            object: placed.object,
            quad: placed.quad,
            on: placed.on,
        })
    }

    /// A widget's on-state name, or Table 230's `/Opt` entry for it.
    ///
    /// # Errors
    ///
    /// [`Status::OutOfRange`] where there is no such field or widget, [`Status::WrongKind`] for a
    /// `which` that names something else.
    pub fn widget_text(
        &self,
        field: usize,
        widget: usize,
        which: TextKind,
    ) -> Result<&str, Status> {
        let placed = self.placed(field, widget)?;
        match which {
            TextKind::Label => Ok(placed.on_state.as_str()),
            TextKind::Export => Ok(placed.export.as_str()),
            TextKind::Qualified | TextKind::Shown | TextKind::Partial => Err(Status::WrongKind),
        }
    }

    /// One widget, by field and index.
    fn placed(&self, field: usize, widget: usize) -> Result<&Placed, Status> {
        self.fields
            .get(field)
            .ok_or(Status::OutOfRange)?
            .widgets
            .get(widget)
            .ok_or(Status::OutOfRange)
    }
}

impl Entry {
    /// One field of the answer.
    fn of(field: &FormField) -> Self {
        let control = &field.control;
        Self {
            qualified: field.name.qualified.clone(),
            shown: field.name.shown().to_owned(),
            partial: field.partial.clone(),
            kind: ControlKind::of(&control_kind(control)),
            flags: flags_of(field),
            value: field.value.as_ref().map(|shown| shown.text.clone()),
            limits: match control {
                pdf_model::form::Control::Text(text) => (text.max_len, text.comb),
                _ => (None, None),
            },
            options: match control {
                pdf_model::form::Control::Choice(choice) => choice
                    .options
                    .iter()
                    .enumerate()
                    .map(|(index, option)| Option_ {
                        label: option.label.clone(),
                        // "[e]ach element of the array is either a text string representing one of
                        // the available options or an array consisting of two text strings: the
                        // option's export value and the text that shall be displayed" — so a bare
                        // string is both, and answering an empty export for one would be this
                        // boundary inventing a distinction §12.7.5.4 does not make.
                        export: option
                            .export
                            .clone()
                            .unwrap_or_else(|| option.label.clone()),
                        selected: choice.selected.contains(&index),
                    })
                    .collect(),
                _ => Vec::new(),
            },
            widgets: field.widgets.iter().map(Placed::of).collect(),
        }
    }
}

impl Placed {
    /// One widget of a field.
    fn of(widget: &FormWidget) -> Self {
        Self {
            object: (widget.annotation.number, widget.annotation.generation),
            quad: widget.quad,
            on: widget.on,
            on_state: widget.on_state.clone().unwrap_or_default(),
            export: widget.export.clone().unwrap_or_default(),
        }
    }
}

/// Every boolean of Tables 227, 229, 231 and 233 that crosses, as one word.
fn flags_of(field: &FormField) -> u32 {
    let mut flags = 0_u32;
    let mut set = |bit: u32, on: bool| {
        if on {
            flags |= bit;
        }
    };
    set(FLAG_READ_ONLY, field.read_only);
    set(FLAG_REQUIRED, field.required);
    set(FLAG_NO_EXPORT, field.no_export);
    set(
        FLAG_OBSCURED,
        field.value.as_ref().is_some_and(|shown| shown.obscured),
    );
    match &field.control {
        pdf_model::form::Control::Text(text) => {
            set(FLAG_MULTILINE, text.multiline);
            set(FLAG_PASSWORD, text.password);
            set(FLAG_FILE_SELECT, text.file_select);
            set(FLAG_DO_NOT_SPELL_CHECK, text.do_not_spell_check);
            set(FLAG_DO_NOT_SCROLL, text.do_not_scroll);
            set(FLAG_COMB, text.comb.is_some());
            set(FLAG_RICH_TEXT, text.rich_text);
        }
        pdf_model::form::Control::Choice(choice) => {
            set(FLAG_EDITABLE, choice.editable);
            set(FLAG_MULTI_SELECT, choice.multi_select);
            set(FLAG_DO_NOT_SPELL_CHECK, choice.do_not_spell_check);
            set(FLAG_COMMIT_ON_SELECTION, choice.commit_on_selection);
        }
        pdf_model::form::Control::CheckBox { on } => set(FLAG_ON, *on),
        pdf_model::form::Control::RadioButton {
            on,
            no_toggle_to_off,
            in_unison,
        } => {
            set(FLAG_ON, *on);
            set(FLAG_NO_TOGGLE_TO_OFF, *no_toggle_to_off);
            set(FLAG_RADIOS_IN_UNISON, *in_unison);
        }
        pdf_model::form::Control::PushButton
        | pdf_model::form::Control::Signature
        | pdf_model::form::Control::Unstated => {}
    }
    flags
}
