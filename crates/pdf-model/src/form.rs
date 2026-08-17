//! ISO 32000-2 §12.7's interactive form, described so that somebody else's widgets can be it.
//!
//! Everything else clause 12 puts on a page is *drawn*: §12.5.5's appearance streams are page
//! content, and a highlight or a stamp has no counterpart in a native toolkit. A form field does.
//! A text field is a `QLineEdit`, an `NSTextField`, a `GtkEntry`; a choice field is a combo box or
//! a list; a check box is the platform's check box. `doc/ui-boundary.md` states the rule this
//! module exists to satisfy — "[i]nteractive chrome crosses as geometry, not pixels" — and
//! §12.5.6.19 states the same division from the standard's side: widget annotations "represent the
//! appearance of fields and **to manage user interactions**", and managing an interaction is what a
//! host does in its own controls.
//!
//! What crosses is therefore a *description*: what kind of control the field is, which of Table
//! 227's, 229's, 231's and 233's flags change that control, what its value is and what a value may
//! be, and where on the page each of its widgets sits. What does not cross is a picture.
//!
//! # Why the field and not the widget
//!
//! §12.7.4.1 makes a value the *field's* — "a field's children in the hierarchy may also include
//! widget annotations ... that define its appearance on the page" — so one field owns any number of
//! rectangles and all of them show the same value. A host places one control per widget and sends
//! one edit per field, which is why [`FormField`] holds a list of [`Widget`] rather than the other
//! way round, and why [`crate::view::ViewState::set_field`] takes a name.
//!
//! # What a host may not work out for itself
//!
//! Two things, and each is why this module reads more than the field dictionary's own entries.
//!
//! **A button's on state.** §12.7.5.2.3 makes `/V` "a name object representing the check box's
//! appearance state, which shall be used to select the appropriate appearance from the appearance
//! dictionary", and the names are the file's own invention — `/Yes`, `/1`, `/On`, or Table 230's
//! positional `/0` and `/1`. A host that guessed would turn nothing on. [`Widget::on_state`] is the
//! name to send.
//!
//! **A choice field's items.** Table 234's `/Opt` is "an array of options that shall be presented
//! to the user", each either a label or an export/label pair, and §12.7.5.4 makes `/V` one of those
//! labels. Nothing else in this crate's interface hands them over.
//!
//! # Read on demand
//!
//! One walk of §12.7.4.1's field tree per call, which is what `CLAUDE.md`'s "nothing eager" asks
//! for: a document with no `/AcroForm` pays a catalog lookup, and one with a form pays for the page
//! being looked at rather than for every page. A host asks when a page appears, not at pointer
//! speed — [`crate::view::field_at`] is the question a pointer asks.

use std::collections::BTreeMap;

use pdf_syntax::{Dictionary, Document, Object, ObjectId};

use crate::Page;
use crate::appearance::{self, Field, FieldKind};
use crate::view::{FieldName, ViewState};

/// One of §12.7's fields, with the widgets of it that are on the page asked about.
#[derive(Debug, Clone, PartialEq)]
pub struct FormField {
    /// The two names §14.9.3 makes a processor distinguish: §12.7.4.2's fully qualified name,
    /// which [`crate::view::ViewState::set_field`] addresses, and Table 226's `/TU`, which is
    /// what a label shows.
    pub name: FieldName,
    /// Table 226's `/T`, the partial name — the last component of the qualified one.
    ///
    /// Carried rather than left to a host to split off, because the split is only obvious once
    /// §12.7.4.2 has been read: "[b]ecause the PERIOD is used as a separator for fully qualified
    /// names, a partial name shall not contain a PERIOD character", which is what makes the last
    /// component the whole of `/T` and not part of it.
    pub partial: String,
    /// What kind of control the field is, and the flags that change the control rather than the
    /// drawing.
    pub control: Control,
    /// What the field says **now**, where §12.7.4.3 lays a text value out for it.
    ///
    /// The same answer [`crate::view::ViewState::field_value`] gives and with the same two
    /// meanings: `None` is a field whose value is not text — a button selects an appearance, a
    /// signature holds a dictionary — and an empty [`crate::view::ShownValue::text`] is a text
    /// field with nothing in it. A password field answers with Table 231 bit 14's bullets rather
    /// than with its characters, and [`crate::view::ShownValue::obscured`] is what says so — see
    /// that field for what a host owes it (ADR 0247).
    ///
    /// **Already through §12.7.5.3's truncation.** A field carrying Table 231 bit 24's
    /// `DoNotScroll` takes only as much of a value as fits its rectangle, so this is what the
    /// document holds rather than what a host last sent — which is the whole reason ADR 0201 has a
    /// host read a value back instead of keeping its own.
    pub value: Option<crate::view::ShownValue>,
    /// Table 227 bit 1: "an interactive PDF processor shall not allow a user to change the value
    /// of the field".
    ///
    /// A host makes its control read-only; this crate refuses the edit anyway
    /// ([`crate::view::ViewState::set_field`]), so the two agree rather than one of them being the
    /// enforcement.
    pub read_only: bool,
    /// Table 227 bit 2: "the field shall have a value at the time it is exported by a submit-form
    /// action".
    ///
    /// About §12.7.6.2's export rather than about drawing, and carried because marking a required
    /// field is something a form's user interface does and nothing else in this crate could say.
    pub required: bool,
    /// Table 227 bit 3: "the field shall not be exported by a submit-form action".
    pub no_export: bool,
    /// The widgets of this field that lie on the page asked about, in `/Annots` order.
    ///
    /// `/Annots` order is painting order, so a host stacking controls may follow it. §12.5.5 is
    /// where ISO 32000-2 says so — an annotation's group "shall be composited with a backdrop
    /// consisting of the page content along with any previously painted annotations" — and the
    /// array is the only order the standard gives those annotations. (This cited a sentence
    /// §12.5.2 does not contain until the four-hundred-and-ninth session; see ADR 0245.)
    pub widgets: Vec<Widget>,
}

/// One widget annotation of a field, placed.
#[derive(Debug, Clone, PartialEq)]
pub struct Widget {
    /// The annotation itself, which is what [`crate::view::ViewState`] keys its per-widget state
    /// by and what §12.6.3's trigger events name.
    pub annotation: ObjectId,
    /// Table 166's `/Rect`, normalised to `[x0, y0, x1, y1]`, in **default user space**.
    ///
    /// The document's own coordinates. Turning them into pixels is one arithmetic in one place —
    /// see `viewer_core`, which does it for every shape it hands a host.
    pub rect: [f32; 4],
    /// The name that turns *this* widget on, for §12.7.5.2's two toggling kinds.
    ///
    /// Table 170's appearance dictionary is keyed by state, and §12.7.5.2.3 makes `/V` select
    /// among those keys — so this is the entry of `/AP /N` that is not §12.7.5.2.3's `Off`, and it
    /// is what a host sends to [`crate::view::ViewState::set_field`] to check the box. `None` for a
    /// field that is not a toggling button, and for one whose `/AP` states no such entry.
    ///
    /// **One per widget and not one per field**, because §12.7.5.2.4's radio buttons differ in
    /// exactly this: "[t]he parent field's V entry holds a name object corresponding to the
    /// appearance state of whichever child field is currently in the on state".
    pub on_state: Option<String>,
    /// Table 230's `/Opt` entry for this widget, where the field states the array. §12.7.5.2.3:
    ///
    /// > An array containing one entry for each widget annotation in the Kids array of the radio
    /// > button or check box field. Each entry shall be a text string representing the on state of
    /// > the corresponding widget annotation.
    ///
    /// The clause's own reason for it is why it crosses beside [`Widget::on_state`] rather than
    /// instead of it: the two say different things where the entry is present, because "the names
    /// used to represent the on state in the AP dictionary of each annotation may use numerical
    /// position (starting with 0) of the annotation in the Kids array". So `/AP` may say `/0` while
    /// `/Opt` says `Rot`, and only the first selects an appearance while only the second is worth
    /// showing a person.
    pub export: Option<String>,
    /// Whether this widget is currently in its on state, for §12.7.5.2's two toggling kinds.
    ///
    /// `false` for every other field type. [`Control::CheckBox`] carries the same fact for the
    /// field as a whole; this is the per-widget answer a radio set needs.
    pub on: bool,
}

/// Which of §12.7.5's four types the field is, with the flags that change what a host builds.
///
/// The flags carried are the ones that decide the *control* — a text view against a line, a secure
/// entry, a combo box against a list — rather than the ones that decide the drawing, which this
/// crate applies itself in `crate::variable_text`. Table 233 bit 20's `Sort` is deliberately absent
/// for the opposite reason: the table addresses it to a writer and forbids a reader to act on it —
/// "PDF readers shall display the options in the order in which they occur in the Opt array" — so
/// handing it over would be inviting a host to break a `shall`.
#[derive(Debug, Clone, PartialEq)]
pub enum Control {
    /// §12.7.5.2.2: "a purely interactive control that responds immediately to user input without
    /// retaining a permanent value".
    ///
    /// Table 229 bit 17. Nothing about a value is carried here because the clause's own
    /// definition says there is none to carry — a control that responds "without retaining a
    /// permanent value" has nothing for `/V` to hold.
    ///
    /// **The sentence that said so outright is gone**: Errata Collection 3 strikes "Because
    /// this type of retains no permanent value, it shall not use the V and DV entries in the
    /// field dictionary" with no replacement (Issue #386, `/State` `Review` `Completed`), and
    /// this comment quoted it until the four-hundred-and-eighteenth session. What survives is
    /// the definition above, which is the reason the struck sentence gave for itself.
    PushButton,
    /// §12.7.5.2.3: a control that "toggles between two states, on and off".
    CheckBox {
        /// Whether the field is on — Table 226's `/V` where nothing has replaced it, and what a
        /// person or an action put there where something has.
        on: bool,
    },
    /// §12.7.5.2.4: "a set of related buttons", of which typically one is on.
    RadioButton {
        /// Whether any widget of the set is on.
        on: bool,
        /// Table 229 bit 15: "If set, exactly one radio button shall be selected at all times;
        /// selecting the currently selected button has no effect."
        ///
        /// A rule about what a *click* does, which is why a host needs it: with the flag clear,
        /// "clicking the selected button deselects it, leaving no button selected".
        no_toggle_to_off: bool,
        /// Table 229 bit 26: buttons of the set sharing an on state "will turn on and off in
        /// unison".
        in_unison: bool,
    },
    /// §12.7.5.3: "a box or space for text fill-in data typically entered from a keyboard".
    Text(TextControl),
    /// §12.7.5.4: a field that "contains several text items, one or more of which shall be
    /// selected as the field value".
    Choice(ChoiceControl),
    /// §12.7.5.5: "a form field that contains a digital signature".
    ///
    /// No control to build and no value to type: the field's value "shall be a signature
    /// dictionary". What a host does with one is show whether it verifies, which
    /// `pdf_model::signature` answers separately.
    Signature,
    /// Table 226 makes `/FT` "(Required for terminal fields; inheritable)" and this field states
    /// none anywhere in its ancestry.
    ///
    /// Carried rather than dropped: the widget is on the page and has a rectangle, so a host that
    /// left it out would show a form with a hole in it. What it may not do is guess which of the
    /// four controls the file meant.
    Unstated,
}

/// §12.7.5.3's text field, and the five of Table 231's flags that change the control.
#[expect(
    clippy::struct_excessive_bools,
    reason = "one field per flag Table 231 defines, which is what the table is. Folding them into \
              enums would invent a taxonomy the standard does not have and would make a reader \
              look up which of this crate's names a bit went into"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextControl {
    /// Bit 13: "the field may contain multiple lines of text; if clear, the field's text shall be
    /// restricted to a single line".
    pub multiline: bool,
    /// Bit 14: "intended for entering a secure password that should not be echoed visibly to the
    /// screen".
    ///
    /// [`FormField::value`] is already bullets for such a field, so a host that draws what it is
    /// given cannot leak the characters; this is what tells it to build a *secure* entry, which is
    /// a different control on every platform.
    pub password: bool,
    /// Bit 21: "the text entered in the field represents the pathname of a file whose contents
    /// shall be submitted as the value of the field".
    pub file_select: bool,
    /// Bit 23: "text entered in the field shall not be spell-checked".
    pub do_not_spell_check: bool,
    /// Bit 24: "the field shall not scroll ... to accommodate more text than fits within its
    /// annotation rectangle".
    ///
    /// The flag behind ADR 0197 and the reason a host reads [`FormField::value`] back after every
    /// keystroke rather than keeping its own buffer: past the edge, "no further text shall be
    /// accepted", and what the field took is what this crate answers with.
    pub do_not_scroll: bool,
    /// Bit 25 and Table 232's `/MaxLen` together: how many equally spaced cells the field is
    /// divided into, where it is a comb field.
    ///
    /// `Some(n)` only where the flag is set *and* Table 231's own condition on it holds — "[m]ay
    /// be set only if the `MaxLen` entry is present ... and if the Multiline, Password, and
    /// `FileSelect` flags are clear" — so a file claiming a comb it is not entitled to gets an
    /// ordinary entry rather than a cell count this crate invented.
    pub comb: Option<u32>,
    /// Table 232's `/MaxLen`: "[t]he maximum length of the field's text, in characters."
    ///
    /// Beside `comb` rather than folded into it, because the entry binds a field that is not a
    /// comb as well: a host caps its control's length from this.
    pub max_len: Option<u32>,
    /// Bit 26: "the value of this field shall be a rich text string".
    ///
    /// Carried so that a host can decline. `CLAUDE.md` excludes XFA, so the rich text is not
    /// interpreted here and what [`FormField::value`] answers with is Table 226's plain `/V`.
    pub rich_text: bool,
}

/// §12.7.5.4's choice field: its items, which of them are selected, and the flags that decide
/// whether it is a list or a combo box.
#[expect(
    clippy::struct_excessive_bools,
    reason = "one field per flag Table 233 defines; see [`TextControl`]"
)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChoiceControl {
    /// Table 233 bit 18: "If set, the field is a combo box; if clear, the field is a list box."
    pub combo: bool,
    /// Bit 19: "the combo box shall include an editable text box as well as a drop-down list".
    ///
    /// The table restricts it — "[t]his flag shall be used only if the Combo flag is set" — and
    /// the restriction is carried rather than enforced: a file that sets it on a list box has said
    /// something Table 233 forbids, and silently clearing it would hide the file's mistake.
    pub editable: bool,
    /// Bit 22: "more than one of the field's option items may be selected simultaneously".
    pub multi_select: bool,
    /// Bit 23: "text entered in the field shall not be spell-checked".
    pub do_not_spell_check: bool,
    /// Bit 27: "the new value shall be committed as soon as a selection is made".
    ///
    /// A rule about *when* a host reports an edit — with it clear, "the new value is not committed
    /// until the user exits the field" — and one nothing in this crate could apply on a host's
    /// behalf, because leaving a field is an event only the host has.
    pub commit_on_selection: bool,
    /// Table 234's `/Opt`, in the array's own order.
    ///
    /// The order matters and Table 233 bit 20 says why: a reader shows them "in the order in which
    /// they occur in the Opt array". An empty list is the table's own answer for a field with no
    /// `/Opt` — "[i]f this entry is not present, no choices should be presented to the user".
    pub options: Vec<Choice>,
    /// Which entries of `options` §12.7.5.4 says are selected, as indices into it.
    ///
    /// Derived from the value, because the clause makes the value decide: "[i]f the items
    /// identified by this entry differ from those in the V entry of the field dictionary ... the V
    /// entry shall be used", of Table 234's `/I`. `/I` is read only where it breaks a tie the
    /// value cannot — two options with the same label — which is the case the clause states it
    /// for.
    pub selected: Vec<usize>,
    /// Table 234's `/TI`: "the index in the Opt array of the first option visible in the list".
    ///
    /// "For scrollable list boxes", and the table's default is 0.
    pub top: usize,
}

/// One entry of Table 234's `/Opt`. ISO 32000-2 §12.7.5.4:
///
/// > Each element of the array is either a text string representing one of the available options
/// > or an array consisting of two text strings: the option's export value and the text that shall
/// > be displayed as the name of the option.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    /// The first of the pair, where the entry is one: what §12.7.6.2 exports for this item.
    pub export: Option<String>,
    /// What is shown, and what §12.7.5.4 makes `/V` name: "the name string is the second of the
    /// two array elements".
    pub label: String,
}

/// The fields of the document's interactive form that have a widget on this page.
///
/// In the page's `/Annots` order, taken from each field's *first* widget on the page — which is
/// painting order, by §12.5.5's "along with any previously painted annotations" over the only
/// order the standard states — so a host laying controls over the page in this order stacks them
/// the way the document stacks its appearances.
///
/// The state is `view`'s, so a field a person has typed into answers with what they typed, one an
/// FDF import replaced answers with the imported value, and one §12.7.6.3 reset answers with `/DV`
/// — the same four statements about a value the drawing path resolves, asked in the same order.
///
/// An empty list for a document with no `/AcroForm`, for a page with no widget on it, and for a
/// form whose `/Fields` array is not an array. §12.7.3 makes that entry required, and a form
/// without it has described no field.
#[must_use]
pub fn fields(document: &Document, page: &Page, view: &ViewState) -> Vec<FormField> {
    let on_page = widgets_on(document, page);
    if on_page.is_empty() {
        return Vec::new();
    }
    let mut by_widget: BTreeMap<ObjectId, (String, usize)> = BTreeMap::new();
    for (name, widgets) in crate::view::widgets_by_field_name(document) {
        for (index, widget) in widgets.iter().enumerate() {
            by_widget.insert(*widget, (name.clone(), index));
        }
    }

    let mut order: Vec<String> = Vec::new();
    let mut grouped: BTreeMap<String, Vec<(ObjectId, usize)>> = BTreeMap::new();
    for widget in on_page {
        let Some((name, index)) = by_widget.get(&widget) else {
            // A widget annotation the field tree does not reach: §12.7.4.2 makes a dictionary
            // with no `/T` "simply a Widget annotation", and one whose `/Parent` chain leaves the
            // form is not part of any field. It is drawn — `crate::annotation` needs no field for
            // that — and there is nothing here for a host to control.
            continue;
        };
        let entry = grouped.entry(name.clone()).or_default();
        if entry.is_empty() {
            order.push(name.clone());
        }
        entry.push((widget, *index));
    }

    order
        .into_iter()
        .filter_map(|name| {
            let widgets = grouped.get(&name)?;
            read(document, view, &name, widgets)
        })
        .collect()
}

/// The widget annotations on this page whose appearance a host has undertaken to draw itself.
///
/// **Exactly the widgets [`fields`] answered for**, and that identity is the whole point rather
/// than an implementation detail: what [`crate::view::WidgetAppearances::Delegated`] leaves out
/// of the page is what a host was handed a control for, so no appearance can disappear without a
/// control taking its place. Built from the same call for that reason —
/// `every_delegated_widget_is_one_a_host_was_told_about` is the assertion, and a second traversal
/// that agreed today could stop agreeing.
///
/// Three kinds of widget are therefore **not** here and keep their appearance. One the field tree
/// does not reach, which §12.7.4.2 makes "simply a Widget annotation"; one belonging to a field
/// whose `/Parent` chain runs past this crate's bound, which [`fields`] refuses for the same
/// reason `crate::appearance::field_text_value` does; and one with no `/Rect`, which is a widget
/// nothing could place a control over.
///
/// A [`std::collections::BTreeSet`] rather than a list because the interpreter asks about one
/// annotation at a time, in `/Annots` order, and this is asked once per page.
#[must_use]
pub fn delegated_widgets(
    document: &Document,
    page: &Page,
    view: &ViewState,
) -> std::collections::BTreeSet<ObjectId> {
    fields(document, page, view)
        .iter()
        .flat_map(|field| field.widgets.iter())
        .map(|widget| widget.annotation)
        .collect()
}

/// Every widget annotation the page lists, in `/Annots` order.
///
/// Unfiltered by §12.5.3's flags, and deliberately: a `Hidden` widget draws nothing, and a host
/// that placed no control over it would agree — but `NoView` is about *this* device and
/// `ToggleNoView` changes with the pointer, so a list that dropped them would be a list that
/// changed while a person moved the mouse. The flags a host needs are the field's, and Table 227's
/// `ReadOnly` is the one that says a control may not be typed into.
fn widgets_on(document: &Document, page: &Page) -> Vec<ObjectId> {
    let annotations = document.get_key(&page.dict, "Annots");
    let Some(annotations) = annotations.as_array() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in annotations {
        let Some(id) = entry.as_reference() else {
            continue;
        };
        let resolved = document.resolve(entry);
        let Some(dict) = resolved.as_dict() else {
            continue;
        };
        if document
            .get_key(dict, "Subtype")
            .as_name()
            .map(pdf_syntax::Name::as_bytes)
            == Some(b"Widget")
        {
            out.push(id);
        }
    }
    out
}

/// Reads one field, given the widgets of it that are on the page and each one's index in
/// §12.7.4.1's `/Kids`.
fn read(
    document: &Document,
    view: &ViewState,
    qualified: &str,
    widgets: &[(ObjectId, usize)],
) -> Option<FormField> {
    let (first, _) = *widgets.first()?;
    let object = document.get(first);
    let dict = object.as_dict()?;
    let field = Field::read(document, dict, view.annotation(first).value);
    // A `/Parent` chain past the bound is a field nobody can name the type or the flags of, and
    // answering from a half-walked ancestry would be a description of a field this crate could not
    // draw either. `crate::appearance::field_text_value` refuses the same case.
    if field.too_deep {
        return None;
    }
    let control = control(document, &field, dict);
    Some(FormField {
        name: FieldName {
            qualified: qualified.to_owned(),
            alternative: crate::view::alternative_name(document, first),
        },
        // §12.7.4.2: the fully qualified name is the ancestors' names and this field's, "separated
        // by a PERIOD (2Eh)", and "a partial name shall not contain a PERIOD character" — so the
        // last component is `/T` whole.
        partial: qualified
            .rsplit_once('.')
            .map_or(qualified, |(_, last)| last)
            .to_owned(),
        value: appearance::field_text_value(document, dict, view.annotation(first).value),
        read_only: field.flags & appearance::FLAG_READ_ONLY != 0,
        required: field.flags & appearance::FLAG_REQUIRED != 0,
        no_export: field.flags & appearance::FLAG_NO_EXPORT != 0,
        widgets: widgets
            .iter()
            .filter_map(|(widget, kid)| {
                self::widget(document, view, &field, &control, *widget, *kid)
            })
            .collect(),
        control,
    })
}

/// Reads one widget of a field.
fn widget(
    document: &Document,
    view: &ViewState,
    field: &Field,
    control: &Control,
    annotation: ObjectId,
    kid: usize,
) -> Option<Widget> {
    let object = document.get(annotation);
    let dict = object.as_dict()?;
    let rect = crate::annotation::rectangle(document, dict, "Rect")?;
    let toggling = matches!(
        control,
        Control::CheckBox { .. } | Control::RadioButton { .. }
    );
    Some(Widget {
        annotation,
        rect: [
            rect[0].min(rect[2]),
            rect[1].min(rect[3]),
            rect[0].max(rect[2]),
            rect[1].max(rect[3]),
        ],
        on_state: toggling.then(|| on_state(document, dict)).flatten(),
        export: toggling.then(|| export(document, field, kid)).flatten(),
        // Per widget, because §12.7.5.2.4's set shares one value and differs in which state name
        // it names — and read through `crate::appearance`'s own answer rather than a second one,
        // so that a description and a picture cannot disagree about which button is checked.
        on: toggling
            && Field::read(document, dict, view.annotation(annotation).value).is_on(document, dict),
    })
}

/// The entry of `/AP /N` that is not `Off`. ISO 32000-2 §12.7.5.2.3:
///
/// > The appearance for the off state is optional but, if present, shall be stored in the
/// > appearance dictionary under the name Off .
///
/// Each state "can have a separate appearance, which shall be defined by an appearance stream in
/// the appearance dictionary of the field's widget annotation", so the states are exactly the keys
/// of that dictionary — one of which is the off state and the rest of which turn the
/// widget on. A well-formed widget states one of each; where a file states several on states this
/// takes the first in the dictionary's own order, which `pdf_syntax::Dictionary` makes the name
/// order — a choice, because the standard gives no second on state a meaning and a host has to be
/// handed one name.
fn on_state(document: &Document, annotation: &Dictionary) -> Option<String> {
    let appearances = document.get_key(annotation, "AP");
    let normal = document.get_key(appearances.as_dict()?, "N");
    let states = normal.as_dict()?;
    states
        .iter()
        .map(|(name, _)| name.as_bytes())
        .find(|name| *name != appearance::OFF)
        .map(|name| String::from_utf8_lossy(name).into_owned())
}

/// Table 230's `/Opt` entry for the widget at this index of `/Kids`.
///
/// The entry is inheritable, so it is looked for up the ancestry [`Field::read`] already walked
/// rather than in the widget's own dictionary alone.
fn export(document: &Document, field: &Field, kid: usize) -> Option<String> {
    let stated = field
        .ancestry
        .iter()
        .map(|source| document.get_key(source, "Opt"))
        .find(|value| value.as_array().is_some())?;
    let items = stated.as_array()?;
    let item = document.resolve(items.get(kid)?);
    item.as_string().map(pdf_syntax::text_string)
}

/// Which of §12.7.5's four types the field is, with each type's own flag table applied.
fn control(document: &Document, field: &Field, annotation: &Dictionary) -> Control {
    let flags = field.flags;
    let set = |bit: i64| flags & bit != 0;
    match field.kind {
        None => Control::Unstated,
        Some(FieldKind::Signature) => Control::Signature,
        Some(FieldKind::Button { toggling: false }) => Control::PushButton,
        Some(FieldKind::Button { toggling: true }) => {
            let on = field.value_is_on();
            if set(appearance::FLAG_RADIO) {
                Control::RadioButton {
                    on,
                    no_toggle_to_off: set(appearance::FLAG_NO_TOGGLE_TO_OFF),
                    in_unison: set(appearance::FLAG_RADIOS_IN_UNISON),
                }
            } else {
                Control::CheckBox { on }
            }
        }
        Some(FieldKind::Text) => {
            let max_len = inherited_number(document, field, annotation, "MaxLen");
            Control::Text(TextControl {
                multiline: field.is_multiline(),
                password: field.is_password(),
                file_select: set(appearance::FLAG_FILE_SELECT),
                do_not_spell_check: set(appearance::FLAG_DO_NOT_SPELL_CHECK),
                do_not_scroll: field.does_not_scroll(),
                comb: field.comb_cells(document, annotation),
                max_len,
                rich_text: set(appearance::FLAG_RICH_TEXT),
            })
        }
        Some(FieldKind::Choice { combo }) => {
            let options = options(document, field);
            Control::Choice(ChoiceControl {
                combo,
                editable: set(appearance::FLAG_EDIT),
                multi_select: set(appearance::FLAG_MULTI_SELECT),
                do_not_spell_check: set(appearance::FLAG_DO_NOT_SPELL_CHECK),
                commit_on_selection: set(appearance::FLAG_COMMIT_ON_SELECTION),
                selected: selected(document, field, &options),
                top: inherited_number(document, field, annotation, "TI")
                    .and_then(|top| usize::try_from(top).ok())
                    .unwrap_or_default(),
                options,
            })
        }
    }
}

/// Table 234's `/Opt`, in both the forms the table gives an element.
pub(crate) fn options(document: &Document, field: &Field) -> Vec<Choice> {
    let Some(stated) = field
        .ancestry
        .iter()
        .map(|source| document.get_key(source, "Opt"))
        .find(|value| value.as_array().is_some())
    else {
        return Vec::new();
    };
    let Some(items) = stated.as_array() else {
        return Vec::new();
    };
    items
        .iter()
        .map(|item| {
            let resolved = document.resolve(item);
            match &resolved {
                Object::Array(pair) => {
                    let text = |index: usize| {
                        pair.get(index)
                            .map(|entry| document.resolve(entry))
                            .and_then(|entry| entry.as_string().map(pdf_syntax::text_string))
                    };
                    Choice {
                        export: text(0),
                        label: text(1).or_else(|| text(0)).unwrap_or_default(),
                    }
                }
                other => Choice {
                    export: None,
                    label: other
                        .as_string()
                        .map(pdf_syntax::text_string)
                        .unwrap_or_default(),
                },
            }
        })
        .collect()
}

/// Which options §12.7.5.4 says the value names.
///
/// > If the field does not allow multiple selection ... or if multiple selection is supported but
/// > only one item is currently selected, V is a text string representing the selected item, as
/// > given in the field dictionary's Opt array. If multiple items are selected, V is an array of
/// > such strings.
///
/// Table 234's `/I` is consulted for the one case the clause states it for — "when two or more
/// elements in the Opt array have different names but the same export value" — because a label
/// that occurs twice cannot say which occurrence the value means. Where they disagree the clause is
/// explicit and the value wins: "[i]f the items identified by this entry differ from those in the V
/// entry of the field dictionary ... the V entry shall be used."
pub(crate) fn selected(document: &Document, field: &Field, options: &[Choice]) -> Vec<usize> {
    let mut wanted: Vec<String> = Vec::new();
    match field.value.as_ref() {
        Some(Object::String(bytes)) => wanted.push(pdf_syntax::text_string(bytes)),
        Some(Object::Array(values)) => {
            for value in values {
                if let Some(bytes) = document.resolve(value).as_string() {
                    wanted.push(pdf_syntax::text_string(bytes));
                }
            }
        }
        // "The default value of V is null , indicating that no item is currently selected."
        _ => return Vec::new(),
    }

    let indices: Vec<usize> = field
        .ancestry
        .iter()
        .map(|source| document.get_key(source, "I"))
        .find_map(|value| {
            let items = value.as_array()?;
            Some(
                items
                    .iter()
                    .filter_map(|entry| document.resolve(entry).as_integer())
                    .filter_map(|index| usize::try_from(index).ok())
                    .filter(|index| *index < options.len())
                    .collect(),
            )
        })
        .unwrap_or_default();

    let mut out: Vec<usize> = Vec::new();
    for label in &wanted {
        let matches: Vec<usize> = options
            .iter()
            .enumerate()
            .filter(|(_, option)| option.label == *label)
            .map(|(index, _)| index)
            .collect();
        // The tie the clause names `/I` for, and the only case it is allowed to decide.
        let chosen = if matches.len() > 1 {
            matches
                .iter()
                .copied()
                .find(|index| indices.contains(index))
                .or_else(|| matches.first().copied())
        } else {
            matches.first().copied()
        };
        if let Some(index) = chosen
            && !out.contains(&index)
        {
            out.push(index);
        }
    }
    out.sort_unstable();
    out
}

/// An inheritable integer entry, taken from the nearest ancestor that states it.
///
/// The ancestry is the one [`Field::read`] walked, so §12.7.4.1's inheritance is applied once for
/// the field rather than once per entry. The widget's own dictionary is checked last for the case
/// §12.5.6.19 describes, where the field and the annotation "may be merged into a single
/// dictionary" that the walk has already visited — which costs one lookup and covers a file whose
/// merge the walk could not confirm.
pub(crate) fn inherited_number(
    document: &Document,
    field: &Field,
    annotation: &Dictionary,
    key: &str,
) -> Option<u32> {
    let stated = field
        .ancestry
        .iter()
        .find_map(|source| document.get_key(source, key).as_integer())
        .or_else(|| document.get_key(annotation, key).as_integer())?;
    u32::try_from(stated).ok()
}

#[cfg(test)]
mod tests {
    use pdf_syntax::Document;

    use super::{Control, fields};
    use crate::view::ViewState;

    /// A one-page document with the `/AcroForm` and annotations the caller spells.
    fn document(form: &str, annotations: &str, objects: &str) -> Document {
        let body = format!(
            "%PDF-1.7\n\
             1 0 obj << /Type /Catalog /Pages 2 0 R /AcroForm << {form} >> >> endobj\n\
             2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n\
             3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [{annotations}] \
             >> endobj\n\
             {objects}\
             trailer << /Root 1 0 R /Size 64 >>\n"
        );
        Document::open(body.into_bytes()).expect("the fixture parses")
    }

    fn page(document: &Document) -> crate::Page {
        crate::Pages::new(document).get(0).expect("one page")
    }

    /// §12.7.5.3's flags and Table 232's `/MaxLen`, as a host would build the control from them.
    #[test]
    fn a_text_fields_flags_cross_as_the_control_they_describe() {
        // Table 231 bits 13, 14, 21, 23, 24 and 26 are 0x1000, 0x2000, 0x100000, 0x400000,
        // 0x800000 and 0x2000000; this sets 13, 23 and 24 with Table 227 bit 2 beside them.
        let flags = (1 << 12) | (1 << 22) | (1 << 23) | (1 << 1);
        let document = document(
            "/Fields [4 0 R]",
            "4 0 R",
            &format!(
                "4 0 obj << /Type /Annot /Subtype /Widget /Rect [10 20 210 60] /FT /Tx \
                 /T (address) /TU (Where to send it) /Ff {flags} /MaxLen 40 /V (Elm Street) >> \
                 endobj\n"
            ),
        );
        let view = ViewState::of(&document);
        let read = fields(&document, &page(&document), &view);
        assert_eq!(read.len(), 1, "{read:?}");
        let field = &read[0];
        assert_eq!(field.name.qualified, "address");
        assert_eq!(field.partial, "address");
        assert_eq!(field.name.shown(), "Where to send it");
        assert!(field.required, "Table 227 bit 2");
        assert!(!field.read_only);
        assert_eq!(
            field.value.as_ref().map(|shown| shown.text.as_str()),
            Some("Elm Street")
        );
        let Control::Text(text) = &field.control else {
            panic!("{:?}", field.control)
        };
        assert!(text.multiline, "bit 13");
        assert!(text.do_not_spell_check, "bit 23");
        assert!(text.do_not_scroll, "bit 24");
        assert!(!text.password);
        assert_eq!(text.max_len, Some(40));
        assert_eq!(text.comb, None, "bit 25 is clear");
        assert_eq!(field.widgets.len(), 1);
        assert!(
            field.widgets[0]
                .rect
                .iter()
                .eq([10.0_f32, 20.0, 210.0, 60.0].iter()),
            "{:?}",
            field.widgets[0].rect
        );
    }

    /// Table 231 bit 25: a comb field's cell count is `/MaxLen`, and only where the bit is allowed.
    ///
    /// The table's own condition on the bit, §12.7.5.3: it may be set only if `/MaxLen` is present
    /// and the `Multiline`, `Password` and `FileSelect` flags are clear.
    #[test]
    fn a_comb_field_states_its_cell_count_and_a_multiline_one_does_not() {
        let comb = (1 << 24) | (1 << 23);
        let both = comb | (1 << 12);
        for (flags, expected) in [(comb, Some(6_u32)), (both, None)] {
            let document = document(
                "/Fields [4 0 R]",
                "4 0 R",
                &format!(
                    "4 0 obj << /Type /Annot /Subtype /Widget /Rect [10 20 210 60] /FT /Tx \
                     /T (code) /Ff {flags} /MaxLen 6 >> endobj\n"
                ),
            );
            let view = ViewState::of(&document);
            let read = fields(&document, &page(&document), &view);
            let Control::Text(text) = &read[0].control else {
                panic!("{:?}", read[0].control)
            };
            assert_eq!(text.comb, expected, "flags {flags:#x}");
            assert_eq!(text.max_len, Some(6), "the entry is stated either way");
        }
    }

    /// §12.7.5.2.4's set: one field, three widgets, and a state name apiece.
    #[test]
    fn a_radio_set_states_one_on_state_per_widget() {
        // Table 229 bit 16 (Radio) and bit 15 (NoToggleToOff).
        let flags = (1 << 15) | (1 << 14);
        let document = document(
            "/Fields [4 0 R]",
            "5 0 R 6 0 R",
            &format!(
                "4 0 obj << /FT /Btn /T (card) /Ff {flags} /V /visa /Kids [5 0 R 6 0 R] \
                 /Opt [(Visa) (Amex)] >> endobj\n\
                 5 0 obj << /Type /Annot /Subtype /Widget /Rect [10 10 30 30] /Parent 4 0 R \
                 /AS /visa /AP << /N << /visa 9 0 R /Off 9 0 R >> >> >> endobj\n\
                 6 0 obj << /Type /Annot /Subtype /Widget /Rect [40 10 60 30] /Parent 4 0 R \
                 /AS /Off /AP << /N << /amex 9 0 R /Off 9 0 R >> >> >> endobj\n\
                 9 0 obj << /Type /XObject /Subtype /Form /BBox [0 0 20 20] /Length 0 >> \
                 stream\n\nendstream endobj\n"
            ),
        );
        let view = ViewState::of(&document);
        let read = fields(&document, &page(&document), &view);
        assert_eq!(read.len(), 1, "one field, two widgets: {read:?}");
        let field = &read[0];
        assert_eq!(
            field.control,
            Control::RadioButton {
                on: true,
                no_toggle_to_off: true,
                in_unison: false,
            }
        );
        assert_eq!(field.widgets.len(), 2);
        assert_eq!(field.widgets[0].on_state.as_deref(), Some("visa"));
        assert_eq!(field.widgets[1].on_state.as_deref(), Some("amex"));
        assert!(field.widgets[0].on, "/AS names its own state");
        assert!(!field.widgets[1].on, "/AS is Off");
        // Table 230's `/Opt`: "one entry for each widget annotation in the Kids array".
        assert_eq!(field.widgets[0].export.as_deref(), Some("Visa"));
        assert_eq!(field.widgets[1].export.as_deref(), Some("Amex"));
    }

    /// Table 234's `/Opt` in both forms, and §12.7.5.4's rule about which item `/V` names.
    #[test]
    fn a_choice_fields_items_and_selection_cross() {
        // Table 233 bit 18 (Combo) and bit 22 (MultiSelect).
        let flags = (1 << 17) | (1 << 21);
        let document = document(
            "/Fields [4 0 R]",
            "4 0 R",
            &format!(
                "4 0 obj << /Type /Annot /Subtype /Widget /Rect [10 20 210 60] /FT /Ch \
                 /T (colour) /Ff {flags} /TI 1 /V [(Blue) (Red)] \
                 /Opt [(Red) [(b) (Blue)] (Green)] >> endobj\n"
            ),
        );
        let view = ViewState::of(&document);
        let read = fields(&document, &page(&document), &view);
        let Control::Choice(choice) = &read[0].control else {
            panic!("{:?}", read[0].control)
        };
        assert!(choice.combo && choice.multi_select);
        assert_eq!(choice.top, 1, "Table 234's /TI");
        assert_eq!(choice.options.len(), 3);
        assert_eq!(choice.options[0].label, "Red");
        assert_eq!(
            choice.options[0].export, None,
            "a bare string has no export"
        );
        assert_eq!(choice.options[1].label, "Blue");
        assert_eq!(
            choice.options[1].export.as_deref(),
            Some("b"),
            "the pair's first element"
        );
        assert_eq!(choice.selected, vec![0, 1], "both, in the array's order");
    }

    /// §12.7.4.2's qualified name, and the partial one it ends with.
    #[test]
    fn a_nested_field_carries_both_of_its_names() {
        let document = document(
            "/Fields [4 0 R]",
            "5 0 R",
            "4 0 obj << /T (PersonalData) /Kids [5 0 R] >> endobj\n\
             5 0 obj << /Type /Annot /Subtype /Widget /Rect [10 10 90 30] /Parent 4 0 R \
             /FT /Tx /T (ZipCode) >> endobj\n",
        );
        let view = ViewState::of(&document);
        let read = fields(&document, &page(&document), &view);
        assert_eq!(read.len(), 1, "{read:?}");
        assert_eq!(read[0].name.qualified, "PersonalData.ZipCode");
        assert_eq!(read[0].partial, "ZipCode");
    }

    /// Table 226 makes `/FT` required of a terminal field, and a file that omits it is described
    /// rather than dropped.
    #[test]
    fn a_field_with_no_type_is_still_on_the_page() {
        let document = document(
            "/Fields [4 0 R]",
            "4 0 R",
            "4 0 obj << /Type /Annot /Subtype /Widget /Rect [10 10 90 30] /T (mystery) >> endobj\n",
        );
        let view = ViewState::of(&document);
        let read = fields(&document, &page(&document), &view);
        assert_eq!(read.len(), 1);
        assert_eq!(read[0].control, Control::Unstated);
    }

    /// A document with no interactive form has no fields, and asking costs one catalog lookup.
    #[test]
    fn a_page_with_no_widget_answers_with_nothing() {
        let document = document(
            "/Fields []",
            "4 0 R",
            "4 0 obj << /Type /Annot /Subtype /Text /Rect [10 10 30 30] >> endobj\n",
        );
        let view = ViewState::of(&document);
        assert!(fields(&document, &page(&document), &view).is_empty());
    }
}
