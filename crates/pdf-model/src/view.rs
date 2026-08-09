//! What a *viewer* holds that a document does not: the state actions change.
//!
//! A [`crate::Interpretation`] is a function of a document and a page. That is true of every
//! rendering this program did before the sixty-second session, and it stops being true the
//! moment §12.6.4's actions are performed: §12.6.4.13 sets the state of an optional content
//! group, §12.6.4.11 sets an annotation's Hidden flag, and both decide what the *next* render
//! of the page draws. Neither is written back to the file.
//!
//! So there is a third input, and this is it. [`ViewState::of`] builds the state a document
//! opens in — §8.11.4.5's initial configuration and no hidden annotations — and
//! [`ViewState::perform`] moves it. `crate::interpret` builds a fresh one per page, so
//! nothing that does not want this pays for it; `crate::interpret_with` takes one that has
//! been moved.
//!
//! # Why the state is not in the `Document`
//!
//! `pdf_syntax::Document` is what the file says. A layer a person switched off is not what
//! the file says, and putting it there would make two renders of one page differ for reasons
//! the file cannot explain — which is exactly the property that makes the oracle's comparison
//! meaningful. §8.11.4.5 draws the same line: the initial state is "the state used by all PDF
//! processors", and everything after it is one processor's.

use std::collections::{BTreeMap, BTreeSet};

use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId};

use crate::action::{
    Action, Change, EmbeddedGoTo, Hide, HideTarget, ImportData, Named, ResetForm, ResetTarget,
    ThreadJump, Uri,
};
use crate::destination::Destination;
use crate::forms_data::Import;
use crate::optional_content::OptionalContent;

/// Deepest nesting of `/Kids` walked when a field name is resolved.
///
/// §12.7.4.1's field tree is a tree a document controls, and `/Kids` may point back up it.
/// Real forms nest two or three levels — a page, a section, a field — and this is far past
/// any of them.
const MAX_FIELD_DEPTH: usize = 32;

/// The document state a viewer holds and §12.6.4's actions change.
#[derive(Debug, Clone, PartialEq)]
pub struct ViewState {
    /// §8.11's layers, in the state the document opened in plus every change performed since.
    ///
    /// `None` for a document with no `/OCProperties`, which §8.11.4.2 makes decisive: without
    /// it "a PDF processor shall ignore any optional content structures in the document".
    optional_content: Option<OptionalContent>,
    /// Annotations §12.6.4.11 has hidden, by object identity.
    ///
    /// A set of *overrides* rather than a copy of every annotation's flags: Table 167's own
    /// Hidden bit is still read from the file, and this is what a hide action added on top.
    /// Table 214's `/H false` removes an entry rather than adding one, which is why showing an
    /// annotation the file itself marks Hidden does nothing — the action "hides or shows one
    /// or more annotations on the screen by setting or clearing their Hidden flags", and this
    /// program does not write to the file.
    hidden: BTreeSet<ObjectId>,
    /// Annotations §12.6.4.11 has shown, by object identity.
    ///
    /// The other half: `/H false` on an annotation whose own `/F` sets Hidden clears the flag
    /// for this session. Two sets rather than a map from identity to boolean because the
    /// common case is that both are empty and neither allocates.
    shown: BTreeSet<ObjectId>,
    /// Widgets §12.7.6.3's reset-form action has reset, by object identity.
    ///
    /// Empty until a reset is performed, which is every document until somebody clicks. A set
    /// rather than a map of new values, because the clause makes the new value a property of
    /// the *field* — its `/DV`, or nothing — rather than of the action.
    reset: BTreeSet<ObjectId>,
    /// Widgets §12.7.8's imported form data has given a new value, by object identity.
    ///
    /// A map rather than a set, which is the whole difference between this and `reset`: a
    /// reset's new value is a property of the field — its own `/DV` — and an import's comes
    /// from another file, so it has to be carried. Kept disjoint from `reset` by construction,
    /// in [`ViewState::import`] and [`ViewState::reset_form`], because both answer the same
    /// question about one widget and the answer is whichever was performed last.
    imported: BTreeMap<ObjectId, Import>,
    /// Pages §12.7.8.3.3's imported templates have added, after the document's own.
    ///
    /// §12.7.7 says what naming a page is *for*: "[a]n import-data action can add the named page
    /// to the document into which FDF is being imported". Adding is the one operation in this
    /// module that changes how many pages there are, and it belongs here for the same reason a
    /// hidden annotation does — the file says nothing about it, and a second render of the
    /// document without this state has the pages the file has.
    ///
    /// In the order the FDF file's `/Pages` array states, each an object of *this* document,
    /// since §12.7.7's trees name pages the target already holds.
    appended: Vec<ObjectId>,
    /// Widgets a *person* has typed a value into, by object identity.
    ///
    /// The fourth statement about a field's value and the only one that comes from outside the
    /// document altogether: Table 226's `/V` is the file's, §12.7.6.3's reset takes `/DV`,
    /// §12.7.8's import takes another file's, and this is what somebody entered. Kept here for
    /// the reason all three others are — `CLAUDE.md`'s rule 1 makes `pdf_syntax::Document`
    /// immutable, so an edit is a log beside the file and never a change to it, and
    /// interpretation stays a pure function of the bytes and this state.
    ///
    /// An entry whose [`Entry::value`] is `None` is a value a person *cleared*, which is a
    /// different thing from a widget nobody has touched: the first shows an empty field and the
    /// second shows the file's own `/V`.
    edited: BTreeMap<ObjectId, Entry>,
    /// Which annotation the pointer is over or pressing, if any (§12.5.5).
    ///
    /// One annotation rather than a set, because a pointer is in one place. `None` is what
    /// every render before the seventy-sixth session assumed: nothing is interacting with the
    /// user, so every annotation shows its normal appearance.
    pointer: Option<(ObjectId, Pointer)>,
    /// How large the page is being drawn, in logical pixels per default user space unit.
    ///
    /// The one thing in this struct that is a property of the *window* rather than of anything
    /// a person did to the document, and it is here because §12.5.3's `NoZoom` makes it decide
    /// a mark: an annotation with that flag "shall always maintain the same fixed size on the
    /// screen". Interpretation therefore has to know the magnification, and rule 1 says the
    /// only way it may is through this state.
    ///
    /// `None` is **not** 1.0 and the distinction is the whole point: it is *nobody has said*,
    /// which is what the corpus gate, the oracle and every caller of [`ViewState::of`] mean.
    /// Under it `NoZoom` changes nothing, so a page rendered at its own scale is the page it
    /// always was.
    ///
    /// (This entry said it was *the* one such thing until the four-hundred-and-ninth session,
    /// which added the second — [`ViewState::widget_appearances`] — for exactly the reason
    /// stated above. Two is not a trend, and each is here because a clause makes interpretation
    /// depend on something only a window knows.)
    magnification: Option<f32>,
    /// Whether §12.7's form widgets are drawn, or left to whoever asked for the page.
    ///
    /// The second thing in this struct that is a property of the *host* rather than of the
    /// document or of anything a person did to it, and it is here for `magnification`'s reason:
    /// it changes what is drawn, and rule 1 makes this state the only channel by which anything
    /// outside the file may.
    widget_appearances: WidgetAppearances,
    /// Annotations a person has **added**, in the order they added them.
    ///
    /// The fifth thing in this struct that comes from outside the document, and the first that
    /// is not a change to something the file already holds: `CLAUDE.md` permits exactly this —
    /// what a *user* does to an open document is not authoring, and §7.5.6's incremental update
    /// is how it goes back into the file with the producer's bytes untouched underneath.
    ///
    /// Each carries the object number it will be written under, allocated when it is added so
    /// that it has an identity for as long as the document is open — which is what the pointer,
    /// a later edit and the writer all need to name it by.
    added: Vec<Added>,
}

/// The resource name the `/DA` of a free text annotation this program creates uses.
///
/// **One of §12.7.4.3's own fourteen abbreviations**, and that is the whole argument for it:
/// `variable_text`'s `STANDARD_ABBREVIATIONS` is a bijection between these names and §9.6.2.2's
/// fourteen standard fonts, so a reader that has never heard of this program still knows what
/// `/Helv` denotes — and where the document's `/DR` already defines the name, that definition is
/// the document's own and wins, which is exactly what the clause asks.
const FREE_TEXT_FONT: &str = "Helv";

/// What [`FREE_TEXT_FONT`] is defined as when the document defines nothing under that name.
const FREE_TEXT_BASE_FONT: &str = "Helvetica";

/// The size the `/DA` of a free text annotation this program creates states, in points.
///
/// **A choice, and the standard states nothing about it**: §12.7.4.3 describes reading a `/DA`
/// and says only that "[a] zero value for size means that the font shall be auto-sized". Zero is
/// therefore available and is not what a person drawing a text box means — auto-sizing grows one
/// character until it fills whatever rectangle was dragged. Twelve points is a note.
const FREE_TEXT_SIZE: f32 = 12.0;

/// One annotation a person added, and the page it belongs to.
#[derive(Debug, Clone, PartialEq)]
pub struct Added {
    /// The object it will be written under, and its identity while the document is open.
    pub id: ObjectId,
    /// The page it belongs to, which is what `Page::id` answers.
    pub page: ObjectId,
    /// The annotation dictionary, whole, as §7.5.6 will write it.
    pub dict: Dictionary,
}

/// Which of §12.5.6.10's four text markup annotations to add.
///
/// The clause's own list, and the four are one construction with four shapes:
///
/// > Text markup annotations shall appear as highlights, underlines, strikeouts (all PDF 1.3),
/// > or jagged ("squiggly") underlines ( PDF 1.4 ) in the text of a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Markup {
    /// `Highlight`, drawn as a wash under `Multiply` so the text stays readable.
    Highlight,
    /// `Underline`, a bar on the bottom edge.
    Underline,
    /// `StrikeOut`, a bar across the middle.
    StrikeOut,
    /// `Squiggly`, a jagged underline.
    Squiggly,
}

impl Markup {
    /// Table 182's `/Subtype` for this markup, which the table requires to be one of
    /// `Highlight`, `Underline`, `Squiggly` or `StrikeOut`. (This cited Table 179 — the line
    /// ending styles — until the three-hundred-and-eighty-seventh session.)
    const fn subtype(self) -> &'static [u8] {
        match self {
            Self::Highlight => b"Highlight",
            Self::Underline => b"Underline",
            Self::StrikeOut => b"StrikeOut",
            Self::Squiggly => b"Squiggly",
        }
    }
}

/// What the pointer is doing to an annotation, in §12.5.5's terms.
///
/// The clause states the three appearances as three situations rather than as a mode:
///
/// > The normal appearance shall be used when the annotation is not interacting with the
/// > user. … The rollover appearance shall be used when the user moves the cursor into the
/// > annotation's active area without pressing the mouse button. … The down appearance shall
/// > be used when the mouse button is pressed or held down within the annotation's active
/// > area.
///
/// NOTE 2 is worth carrying: "the term mouse denotes a generic pointing device that controls
/// the location of a cursor on the screen and has at least one button".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pointer {
    /// The cursor is inside the annotation's active area with no button pressed.
    Over,
    /// A button is pressed or held down inside it.
    Down,
}

/// What performing an action asks of the viewer that a [`ViewState`] cannot do itself.
///
/// The division is the same one this module exists for. A layer's state and an annotation's
/// Hidden flag are *this document's* state and live here; which page is on screen and what a
/// URI opens are the window's, and no part of them is a fact about the file.
#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    /// §12.6.4.2: display this destination. [`Destination::page_index`] turns it into a page.
    Display(Destination),
    /// §12.6.4.12: move as Table 215's command says, from whichever page is being shown.
    Page(Named),
    /// §12.6.4.8: resolve this URI, which means opening it somewhere this program is not.
    Resolve(Uri),
    /// §12.6.4.7: jump to a bead on an article thread.
    ///
    /// Unresolved for [`Self::Display`]'s reason one level further out: a destination needs the
    /// page tree and this needs [`crate::article::Articles`], and neither is part of the state a
    /// click changes. [`crate::action::ThreadJump::bead_in`] turns it into a bead.
    Thread(ThreadJump),
    /// §12.6.4.15: draw the page as it now stands, using this transition.
    ///
    /// §12.6.4.15 is explicit that this is about *when* drawing happens: a processor "shall
    /// normally suspend drawing when such a sequence begins and resume drawing when it ends", and
    /// a transition action in the middle of one says to show the page as the previous action left
    /// it. Suspending and resuming is a window's business — this state has no screen — so the
    /// transition is handed over and the caller decides whether it has one to play.
    Transition(crate::navigation::Transition),
    /// §12.6.4.5: show the page a document part begins at.
    ///
    /// Unresolved for [`Self::Display`]'s reason: finding the page needs the page tree, which is
    /// not part of the state a click changes.
    /// [`crate::action::DocumentPartJump::page_in`] turns it into a page.
    DocumentPart(crate::action::DocumentPartJump),
    /// §12.6.4.4: show a destination in a document embedded in this one.
    ///
    /// Unresolved for [`Self::Display`]'s reason, twice over: the target document has to be
    /// *opened* before its destination means anything, and which document is on screen is the
    /// window's business rather than this state's.
    /// [`crate::action::EmbeddedGoTo::target_in`] opens it.
    Embedded(EmbeddedGoTo),
    /// §12.7.6.4: import this file's form data, which means finding and reading it.
    ///
    /// The same division as [`Self::Resolve`], and for the same reason: a document naming a file
    /// is a document asking this machine for something, and whether to give it is not a
    /// rendering decision. A caller that has the bytes hands them to
    /// [`crate::forms_data::FormsData::read`] and then to [`ViewState::import`], which is where
    /// the values become ink.
    Import(ImportData),
}

/// Which statement about a field's value a widget is currently showing.
///
/// Three, and each is a different clause saying where a value comes from. The default is the
/// file's own, which is every widget in every document until something is performed.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FieldValue<'a> {
    /// Table 226's `/V`, read up §12.7.4.1's `/Parent` chain.
    #[default]
    Stored,
    /// §12.7.6.3: `/DV` instead, because a reset-form action named this widget.
    Default,
    /// §12.7.4: a value a person entered, which replaces every other statement about it.
    ///
    /// Last because it is latest: the file's `/V`, a reset's `/DV` and an import's value are all
    /// statements made before somebody typed. A `value` of `None` is a field a person *cleared*,
    /// which shows nothing — a different thing from [`Self::Stored`], which shows the file's own
    /// value.
    Edited {
        /// Table 226's `/V` as the edit leaves it, already in the object type the entry takes.
        ///
        /// A string for text and for §12.7.5.2's appearance-state names, and **an array of
        /// strings** where §12.7.5.4's choice field has several items selected — the clause's own
        /// two shapes, resolved once by [`ViewState::set_field`] so that the appearance and the
        /// file cannot disagree about what was entered.
        value: Option<&'a Object>,
        /// Table 234's `/I`, where the edit chose among §12.7.5.4's options.
        ///
        /// Zero-based indices into `/Opt`, ascending. `None` is an edit that named no options —
        /// text typed into a field, a button's state, or a clear — and it is what says the entry
        /// must not survive into the file beside the new `/V`.
        indices: Option<&'a [usize]>,
    },
    /// §12.7.8: a value from an FDF file, which replaces `/V` (§12.7.8.3.2).
    Imported {
        /// Table 249's `/V`.
        ///
        /// `None` is an FDF field that states no `/V` at all, which "replace" makes a field
        /// whose value is *removed* — the same state a reset leaves a field with no `/DV` in,
        /// and drawn the same way. It is a different thing from [`Self::Stored`], which is a
        /// widget nothing has imported into.
        value: Option<&'a Object>,
        /// Table 249's `/Ff`, `/SetFf` and `/ClrFf` over Table 227's field flags.
        ///
        /// Carried with the value because the flags decide how the value is *drawn*: Table 231's
        /// multiline, comb and password bits each change §12.7.4.3's layout of the same string.
        flags: crate::forms_data::FlagChange,
    },
}

/// What a person put into one field, as [`ViewState::set_field`] takes it.
///
/// Three variants, because §12.7.5 gives a field's value three shapes and a host has to be able to
/// say all three. Two of them were `Option<String>` until the four-hundred-and-twelfth session, and
/// the third is why that stopped being enough: Table 233 bit 22 — *"(PDF 1.4) If set, more than one
/// of the field's option items may be selected simultaneously; if clear, at most one item shall be
/// selected"* — lets §12.7.5.4's list box hold several items at once, and one string cannot say
/// which several. ADR 0248.
///
/// **A selection is named by index and not by label**, which is the decision worth stating. The
/// clause makes `/V` hold the *labels* — "the name string is the second of the two array elements"
/// — so labels are what reaches the file; but two of Table 234's `/Opt` entries may carry the same
/// name string, and a host that answered with labels could not say which of them a person clicked.
/// [`crate::form::ChoiceControl::selected`] already answers in indices, so a host reads and writes
/// the same coordinate.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Entered {
    /// The field is cleared: §12.7.6.3's own words for a value that is gone are "its V entry shall
    /// be removed", and this is what a person emptying a field asks for.
    ///
    /// A different state from never having touched the field, which shows Table 226's `/V`.
    #[default]
    Cleared,
    /// Characters, as §7.9.2.2's text string.
    ///
    /// §12.7.5.3's text field, §12.7.5.4's editable combo box — Table 233 bit 19 lets one "include
    /// an editable text box as well as a drop-down list", so its value need not be one of the
    /// options at all — and §12.7.5.2's two toggling buttons, whose value is the appearance-state
    /// name [`crate::form::Widget::on_state`] hands over.
    Text(String),
    /// §12.7.5.4: which of Table 234's `/Opt` entries are selected, as zero-based indices into it.
    ///
    /// An empty list means no item is selected, which §12.7.5.4 makes the same state as
    /// [`Self::Cleared`]: "[t]he default value of V is null , indicating that no item is currently
    /// selected."
    ///
    /// An index past the end of `/Opt` names nothing and is dropped, and more than one index on a
    /// field whose `MultiSelect` flag is clear is cut to the first — Table 233 bit 22's "at most
    /// one item shall be selected" binds this program because this program is what selects.
    Chosen(Vec<usize>),
}

/// One widget's edit, resolved against the document into what Table 226's `/V` will say.
///
/// Resolved once, at [`ViewState::set_field`], rather than at each of the three places that read it
/// — the appearance, the description a host reads back, and the file a save writes. §12.7.5.4's
/// mapping from an index to a label needs the field's `/Opt`, and doing it three times would be
/// three chances for a picture and a file to disagree about what a person chose.
#[derive(Debug, Clone, PartialEq)]
struct Entry {
    /// Table 226's `/V`, or `None` for a field that was cleared.
    value: Option<Object>,
    /// Table 234's `/I`, where the edit chose among §12.7.5.4's options.
    indices: Option<Vec<usize>>,
}

/// What one import of one FDF file did to this document.
///
/// Every field here is something a caller should be able to say out loud, and none of them is an
/// error. §12.7.8.3.2 matches by fully qualified name, so a name the form has not got is either
/// the wrong FDF for this document or a form that has changed since the data was exported — and
/// only somebody who can see both files can say which.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Imported {
    /// How many widget annotations took a value.
    pub widgets: usize,
    /// Fully qualified names the FDF file states that this document has no field for.
    pub unmatched: Vec<String>,
    /// How many §12.7.7 template pages were added to the document.
    pub pages: usize,
    /// Templates the file named and this document could not add, each with the reason.
    pub refused: Vec<String>,
}

/// Everything this state says about one annotation, gathered in one walk.
///
/// A struct rather than four arguments because the four are asked together, once per annotation
/// per page, and because three of them default to "the file's own answer" — which is what
/// [`Default`] here means and what every annotation in a document nothing has interacted with
/// gets.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AnnotationView<'a> {
    /// §12.6.4.11: `Some(true)` where a hide action hid this annotation, `Some(false)` where one
    /// showed it, `None` where none named it.
    pub hidden_by_action: Option<bool>,
    /// Which of Table 170's three appearances §12.5.5 asks for, given where the pointer is.
    pub appearance: Appearance,
    /// Where this widget's value comes from.
    pub value: FieldValue<'a>,
    /// §12.7.8's `/F`, `/SetF` and `/ClrF`, where an FDF file stated one of them.
    ///
    /// `None` is not "no flags": it is *this state has nothing to say*, and the annotation's own
    /// `/F` stands unchanged. The change is carried rather than the result because Table 249's
    /// two modifying entries are defined *against* the flags the document states, which only the
    /// reader of that dictionary has.
    pub flags: Option<crate::forms_data::FlagChange>,
}

/// Which of Table 170's appearances an annotation shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Appearance {
    /// `/N`, the only entry Table 170 requires, and the one a printer uses.
    #[default]
    Normal,
    /// `/R`, the rollover appearance.
    Rollover,
    /// `/D`, the down appearance.
    Down,
}

/// Who draws §12.7's form fields: this crate, or whoever asked for the page.
///
/// A native host places a real `GtkEntry`, `QLineEdit` or `NSTextField` over the page at each
/// widget's own rectangle, which is what [`crate::form`] describes a form *for*. Unless it can
/// also ask for the page **without** the widgets' own pictures, a person sees every field twice:
/// the control, and the appearance stream underneath it. ADR 0244 measured that at 76 controls
/// over one corpus document's 67 fields, every one sitting on the picture of itself.
///
/// # This is an instruction, not a departure
///
/// §6.3.2.2 places the obligation on a rendering processor and states its own exception in the
/// same sentence:
///
/// > A PDF processor shall also render the appropriate appearance stream for all annotations
/// > (12.5.5, "Appearance streams") which have appearance streams designated for this purpose as
/// > indicated by the annotation flags (see 12.5.3, "Annotation flags"), unless otherwise
/// > instructed.
///
/// [`Self::Delegated`] is that instruction, and it can only come from a host that has undertaken
/// to draw those appearances itself. It is never this crate's own choice: [`ViewState::of`] is
/// [`Self::Drawn`], so a caller that does not ask gets the page §6.3.2.2 describes.
///
/// # Why the widgets and nothing else
///
/// §12.5.6.19 is what makes them separable from everything else clause 12 puts on a page:
///
/// > Interactive forms (see 12.7, "Forms") use widget annotations (PDF 1.2) to represent the
/// > appearance of fields and to manage user interactions.
///
/// A widget annotation **is** a field's appearance, so a host that draws the field has replaced
/// exactly it. Nothing else has a counterpart in a toolkit — §12.5.6.10's markups, §12.5.6.4's
/// icons and §12.5.6.12's stamps are page content and stay on the page — and §12.5.1 draws the
/// same line from the standard's side, where Table 31's `/Tabs` has a value for the widgets alone:
///
/// > W (widgets order): Widget annotations shall be visited in the order in which they appear in
/// > the page Annots array, followed by other annotation types in row order.
///
/// So "leave out the widgets" and "leave out the annotations" are different requests, and only
/// the first is one a form host has any business making.
///
/// # And only the widgets a host was told about
///
/// Narrower than `/Subtype /Widget`, deliberately: §12.7.4.2 leaves a dictionary whose `/Parent`
/// chain never reaches the form "simply a Widget annotation", [`crate::form::fields`] answers
/// nothing for it, no host was handed a control for it, and dropping its appearance would take
/// ink off the page that nothing replaces. [`crate::form::delegated_widgets`] is the set, built
/// from that same call so the two cannot drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WidgetAppearances {
    /// This crate draws them, which is what §6.3.2.2 asks of a processor nobody has instructed.
    #[default]
    Drawn,
    /// The host draws them, so the page is interpreted without them.
    Delegated,
}

impl ViewState {
    /// The state a document opens in.
    ///
    /// §8.11.4.5's initial configuration — "[t]his state shall be the initial state used by
    /// all PDF processors" — and nothing hidden beyond what each annotation's own `/F` says.
    #[must_use]
    pub fn of(document: &Document) -> Self {
        Self {
            optional_content: OptionalContent::read(document),
            hidden: BTreeSet::new(),
            shown: BTreeSet::new(),
            reset: BTreeSet::new(),
            imported: BTreeMap::new(),
            edited: BTreeMap::new(),
            appended: Vec::new(),
            pointer: None,
            magnification: None,
            widget_appearances: WidgetAppearances::default(),
            added: Vec::new(),
        }
    }

    /// Whether §12.7's form widgets are drawn on the page, or left to the host.
    #[must_use]
    pub fn widget_appearances(&self) -> WidgetAppearances {
        self.widget_appearances
    }

    /// Says who draws §12.7's form widgets.
    ///
    /// [`WidgetAppearances::Drawn`] until a host says otherwise, so a caller that never calls
    /// this draws the page §6.3.2.2 describes — which is what makes the corpus gate, the oracle
    /// and every other existing caller produce the display list they produced before this
    /// existed, by construction rather than by comparison.
    ///
    /// Returns whether the value changed, so a caller can decide whether the page has to be
    /// interpreted again — the same shape [`ViewState::set_magnification`] has, and for the same
    /// reason: this decides what is *drawn*, so a change invalidates a display list rather than
    /// only the pixels made from it.
    pub fn set_widget_appearances(&mut self, appearances: WidgetAppearances) -> bool {
        let changed = self.widget_appearances != appearances;
        self.widget_appearances = appearances;
        changed
    }

    /// How large the page is being drawn, where a caller has said.
    ///
    /// See [`ViewState::set_magnification`] for why `None` is not 1.0.
    #[must_use]
    pub fn magnification(&self) -> Option<f32> {
        self.magnification
    }

    /// Says how large the page is being drawn, in logical pixels per default user space unit.
    ///
    /// Only §12.5.3's `NoZoom` reads it, and only an annotation setting that flag changes when
    /// it does — which is why a host that never zooms need never call this and a host that does
    /// may call it on every frame. `Interpretation::view_dependent` says whether this page has
    /// anything that would notice.
    ///
    /// Returns whether the value changed, so a caller can decide whether the page has to be
    /// interpreted again rather than comparing two floats itself.
    pub fn set_magnification(&mut self, magnification: Option<f32>) -> bool {
        let changed = self.magnification != magnification;
        self.magnification = magnification;
        changed
    }

    /// The optional content configuration, as the state currently stands.
    #[must_use]
    pub fn optional_content(&self) -> Option<&OptionalContent> {
        self.optional_content.as_ref()
    }

    /// Puts the pointer on an annotation, or takes it off every annotation.
    ///
    /// A viewer calls this from its own hit testing: §12.5.5 speaks of the cursor being "into
    /// the annotation's active area", which is a question about a window's coordinates and a
    /// `/Rect` that this crate has no events to answer. What it decides here is which of
    /// Table 170's appearances [`Self::appearance_for`] chooses, and therefore what the next
    /// render draws.
    pub fn set_pointer(&mut self, at: Option<(ObjectId, Pointer)>) {
        self.pointer = at;
    }

    /// Which of Table 170's appearances this annotation shows now.
    ///
    /// [`Appearance::Normal`] for every annotation the pointer is not on, which §12.5.5 makes
    /// the case rather than a default: the normal appearance "shall be used when the
    /// annotation is not interacting with the user".
    #[must_use]
    pub fn appearance_for(&self, annotation: ObjectId) -> Appearance {
        match self.pointer {
            Some((id, Pointer::Over)) if id == annotation => Appearance::Rollover,
            Some((id, Pointer::Down)) if id == annotation => Appearance::Down,
            _ => Appearance::Normal,
        }
    }

    /// Whether §12.6.4.11 has hidden this annotation, shown it, or said nothing about it.
    ///
    /// `Some(true)` means an action hid it, `Some(false)` that an action showed one the file
    /// marks hidden, and `None` that the annotation's own `/F` decides (§12.5.3).
    #[must_use]
    pub fn annotation_hidden(&self, annotation: ObjectId) -> Option<bool> {
        if self.hidden.contains(&annotation) {
            Some(true)
        } else if self.shown.contains(&annotation) {
            Some(false)
        } else {
            None
        }
    }

    /// Everything this state says about one annotation, gathered once.
    ///
    /// The four questions are asked together, per annotation and per page, so asking them in one
    /// call is one lookup per set rather than four walks of the page's annotation array.
    #[must_use]
    pub fn annotation(&self, annotation: ObjectId) -> AnnotationView<'_> {
        AnnotationView {
            hidden_by_action: self.annotation_hidden(annotation),
            appearance: self.appearance_for(annotation),
            value: if let Some(edit) = self.edited.get(&annotation) {
                edit.as_field_value()
            } else if let Some(import) = self.imported.get(&annotation) {
                FieldValue::Imported {
                    value: import.value.as_ref(),
                    flags: import.field_flags,
                }
            } else if self.reset.contains(&annotation) {
                FieldValue::Default
            } else {
                FieldValue::Stored
            },
            flags: self.imported.get(&annotation).and_then(|import| {
                (!import.annotation_flags.is_unchanged()).then_some(import.annotation_flags)
            }),
        }
    }

    /// Imports one FDF file's field values into this view of the document (§12.7.8).
    ///
    /// §12.7.8.3.2 states the operation in one sentence — "importing a field causes the values
    /// of the entries in the FDF field dictionary to replace those of the corresponding entries
    /// in the field with the same fully qualified name in the target document" — and every word
    /// of it is here. *Replace*: an imported widget's value is the FDF file's, whatever the
    /// document's own `/V` says and whatever a reset said before. *The same fully qualified
    /// name*: the pairing runs through the same §12.7.4.2 name table §12.6.4.11's hide action
    /// and §12.7.6.3's reset use, so all three agree about what a field is called.
    ///
    /// Nothing is written to the file, for [`Self::reset_form`]'s reason: this is a viewer's
    /// state and an FDF import that reached the document would be this program creating a PDF.
    pub fn import(&mut self, document: &Document, data: &crate::forms_data::FormsData) -> Imported {
        let table = widgets_by_field_name(document);
        let (matched, unmatched) = crate::forms_data::match_to_document(data, &table);
        for (widget, import) in &matched {
            // The two sets answer the same question, so a widget belongs to exactly one of
            // them: an import after a reset is the later statement about this field's value.
            self.reset.remove(widget);
            self.imported.insert(*widget, import.clone());
        }
        let mut outcome = Imported {
            widgets: matched.len(),
            unmatched,
            ..Imported::default()
        };
        self.append_templates(document, data, &mut outcome);
        outcome
    }

    /// Adds one of §12.5.6.10's text markup annotations over a run of quadrilaterals.
    ///
    /// The quadrilaterals are in **default user space**, which is what Table 182's
    /// `/QuadPoints` is defined in — a caller holding a selection in the display list's own
    /// coordinates maps them back through `crate::content::page_transform`'s inverse. Each is
    /// `[x0, y0, … x3, y3]`, and the order does not matter: `crate::appearance`'s reader sorts
    /// the four corners by where they fall along the text's own direction, because the clause's
    /// "counterclockwise" has two readings and producers use both.
    ///
    /// (**Both numbers said something else until the four-hundred-and-thirteenth session**:
    /// the first was Table 179, which is the line ending styles, and the second put
    /// `/QuadPoints` in Table 166, which states it for no annotation. The three-hundred-and-
    /// eighty-seventh corrected this file's *other* `/QuadPoints` sentence and
    /// `viewer-core/src/open.rs`'s, and left these two. `doc/todo/01`'s ninth sweep.)
    ///
    /// Returns the object the annotation will be written under, or `None` where there is
    /// nothing to mark up.
    ///
    /// **What is written and what is left out.** Table 166's `/Subtype`, `/Rect` and `/C`,
    /// Table 182's `/QuadPoints`, plus `/F 4`. The `/Rect` is the quadrilaterals' bounding box, which is
    /// §12.5.2's "the annotation shall be positioned by its `/Rect`" and what every reader clips
    /// the appearance to. `/F 4` is Table 167's `Print` bit and it is **a choice**: the flag's
    /// own default is "never print the annotation", and a person who marks up a document to send
    /// it on means the mark to survive printing. Table 166's `/M` is *not* written, because
    /// `CLAUDE.md`'s rule 3 gives this crate no clock; a host with one may add it. Nor is `/T`,
    /// which is a person's name and something no part of this program knows.
    ///
    /// **Nothing is written to the file**, like every other edit here: the annotation is a log
    /// entry beside an immutable document until [`ViewState::save`] turns it into §7.5.6's
    /// incremental update.
    ///
    /// **What a document may say about this is [`crate::restriction::asserted`]'s to state and a
    /// host's to decide**, exactly as for [`ViewState::set_field`] — and this operation is the
    /// one that separates §12.8.2.2's levels, because Table 257's `/P` 2 permits filling in a
    /// field and not annotating. ADR 0212.
    pub fn add_markup(
        &mut self,
        document: &Document,
        page: ObjectId,
        kind: Markup,
        colour: [f32; 3],
        quads: &[[f32; 8]],
    ) -> Option<ObjectId> {
        if quads.is_empty() {
            return None;
        }
        let mut points = Vec::with_capacity(quads.len().saturating_mul(8));
        let (mut left, mut bottom) = (f32::MAX, f32::MAX);
        let (mut right, mut top) = (f32::MIN, f32::MIN);
        for quad in quads {
            for corner in quad.chunks_exact(2) {
                let (x, y) = (corner[0], corner[1]);
                if !x.is_finite() || !y.is_finite() {
                    return None;
                }
                left = left.min(x);
                right = right.max(x);
                bottom = bottom.min(y);
                top = top.max(y);
                points.push(Object::Real(f64::from(x)));
                points.push(Object::Real(f64::from(y)));
            }
        }
        let mut dict = Dictionary::default();
        dict.insert(
            Name::new(&b"Type"[..]),
            Object::Name(Name::new(&b"Annot"[..])),
        );
        dict.insert(
            Name::new(&b"Subtype"[..]),
            Object::Name(Name::new(kind.subtype())),
        );
        dict.insert(
            Name::new(&b"Rect"[..]),
            Object::Array(
                [left, bottom, right, top]
                    .into_iter()
                    .map(|value| Object::Real(f64::from(value)))
                    .collect(),
            ),
        );
        dict.insert(Name::new(&b"QuadPoints"[..]), Object::Array(points));
        dict.insert(
            Name::new(&b"C"[..]),
            Object::Array(
                colour
                    .into_iter()
                    .map(|value| Object::Real(f64::from(value.clamp(0.0, 1.0))))
                    .collect(),
            ),
        );
        // Table 167 bit 3.
        dict.insert(Name::new(&b"F"[..]), Object::Integer(4));
        let id = self.next_free_object(document);
        self.added.push(Added { id, page, dict });
        Some(id)
    }

    /// Adds §12.5.6.6's free text annotation over a rectangle a person drew.
    ///
    /// > A free text annotation ( PDF 1.3 ) displays text directly on the page. Unlike an ordinary
    /// > text annotation (see 12.5.6.4, "Text annotations"), a free text annotation has no open or
    /// > closed state; instead of being displayed in a popup window, the text shall be always
    /// > visible.
    ///
    /// The one markup subtype whose text *is* the annotation, which is why the geometry comes from
    /// a **drag** rather than from a selection the way [`ViewState::add_markup`]'s does: there is
    /// no text on the page for it to be over. `rect` is in **default user space**, in either corner
    /// order, and the two corners are normalised here for the reason §12.5.2 gives — Table 166's
    /// `/Rect` "shall be two opposite corners", and states no order for them.
    ///
    /// Returns the object the annotation will be written under — its identity for as long as the
    /// document is open, which is what [`ViewState::set_free_text`] names it by — or `None` for a
    /// rectangle with a non-finite corner or no area at all. **The second is a choice**: a press
    /// that never moved has drawn no box, and an annotation of zero extent would be one nothing
    /// could be typed into and nothing could be seen in.
    ///
    /// # What is written, and what the standard requires of it
    ///
    /// Table 177's `/Subtype` and `/DA`, both Required; Table 166's `/Rect`, `/Contents` and
    /// `/F 4`. Three of them are decisions rather than readings and are recorded as such:
    ///
    /// - **The `/DA`.** Table 177 makes it "[t]he default appearance string that shall be used in
    ///   formatting the text", and §12.7.4.3 states what it must contain: "[a]t a minimum, the
    ///   string shall include a Tf (text font) operator along with its two operands, font and
    ///   size." The standard describes *reading* one and states nothing about what a processor
    ///   creating an annotation should write, so the colour is the caller's, the size is
    ///   [`FREE_TEXT_SIZE`] and the resource name is [`FREE_TEXT_FONT`]. A size of 0 would be the
    ///   clause's auto-sizing, which grows the first character to fill whatever box was dragged;
    ///   a fixed size is what a person drawing a text box means by drawing it.
    /// - **The colour goes in the `/DA` and not in Table 166's `/C`.** That entry is "the
    ///   background of the annotation's icon when closed, the title bar of the annotation's popup
    ///   window, [and] the border of a link annotation", none of which this subtype has — the
    ///   colour of the *text* is §12.7.4.3's, which is the `/DA`.
    /// - **`/F 4` is Table 167's `Print` bit**, for [`ViewState::add_markup`]'s reason exactly: a
    ///   note a person adds to send a document on means the note to survive printing.
    ///
    /// `/M` and `/T` are left out for the same two reasons `add_markup` leaves them out — rule 3
    /// gives this crate no clock, and a person's name is not something this program knows.
    ///
    /// **Nothing is written to the file** until [`ViewState::save`] turns the log into §7.5.6's
    /// incremental update, which is where §12.7.4.3's `/DR` obligation is met — see
    /// [`Update::state_default_font`].
    pub fn add_free_text(
        &mut self,
        document: &Document,
        page: ObjectId,
        rect: [f32; 4],
        text: &str,
        colour: [f32; 3],
    ) -> Option<ObjectId> {
        if !rect.iter().all(|edge| edge.is_finite()) {
            return None;
        }
        let (left, right) = (rect[0].min(rect[2]), rect[0].max(rect[2]));
        let (bottom, top) = (rect[1].min(rect[3]), rect[1].max(rect[3]));
        if left >= right || bottom >= top {
            return None;
        }
        let [red, green, blue] = colour.map(|value| value.clamp(0.0, 1.0));
        let mut dict = Dictionary::default();
        dict.insert(
            Name::new(&b"Type"[..]),
            Object::Name(Name::new(&b"Annot"[..])),
        );
        dict.insert(
            Name::new(&b"Subtype"[..]),
            Object::Name(Name::new(&b"FreeText"[..])),
        );
        dict.insert(
            Name::new(&b"Rect"[..]),
            Object::Array(
                [left, bottom, right, top]
                    .into_iter()
                    .map(|edge| Object::Real(f64::from(edge)))
                    .collect(),
            ),
        );
        dict.insert(
            Name::new(&b"DA"[..]),
            Object::String(
                format!("{red} {green} {blue} rg /{FREE_TEXT_FONT} {FREE_TEXT_SIZE} Tf")
                    .into_bytes()
                    .into(),
            ),
        );
        // Table 167 bit 3.
        dict.insert(Name::new(&b"F"[..]), Object::Integer(4));
        // Table 168's `/W`: "If this value is 0, no border shall be drawn." Stated rather than
        // left out, and that is the point — Table 166's `/Border` defaults to `[0 0 1]`, so an
        // annotation saying *nothing* about its border has one a point wide, and no clause
        // anywhere says what colour to draw it. A file this program writes may not leave that
        // question open: `appearance`'s `undrawn_decoration` would report the annotation this
        // program itself just created, which is a program telling a person it cannot do what it
        // has done. So the annotation says it has no border, which is a **choice** and the honest
        // one available — the alternative is inventing a colour.
        let mut style = Dictionary::default();
        style.insert(
            Name::new(&b"Type"[..]),
            Object::Name(Name::new(&b"Border"[..])),
        );
        style.insert(Name::new(&b"W"[..]), Object::Integer(0));
        dict.insert(Name::new(&b"BS"[..]), Object::Dictionary(style));
        let id = self.next_free_object(document);
        self.added.push(Added { id, page, dict });
        self.set_free_text(id, text);
        Some(id)
    }

    /// Puts §12.5.6.6's text into an annotation a person added, as Table 166's `/Contents`.
    ///
    /// Table 166 gives that entry two jobs — text to be displayed for the annotation, or, where
    /// the subtype displays none, a description of what it holds — and §12.5.6.6 is the first
    /// kind: the text is what the annotation *is*, so the entry is its
    /// value in the sense Table 226's `/V` is a field's — and this is [`ViewState::set_field`]'s
    /// counterpart, named by object rather than by §12.7.4.2's qualified name because an
    /// annotation has no such name and nothing inherits from it.
    ///
    /// Returns whether anything took the text. **`false` for an annotation this state did not
    /// add**, which includes every free text annotation the file itself states: changing one of
    /// those means replacing an object the producer wrote, and what this crate holds is a log
    /// beside an immutable document. `doc/todo/33` carries what that would cost.
    pub fn set_free_text(&mut self, annotation: ObjectId, text: &str) -> bool {
        let Some(added) = self.added.iter_mut().find(|added| added.id == annotation) else {
            return false;
        };
        if added
            .dict
            .get("Subtype")
            .and_then(Object::as_name)
            .is_none_or(|subtype| subtype.as_bytes() != b"FreeText")
        {
            return false;
        }
        added.dict.insert(
            Name::new(&b"Contents"[..]),
            Object::String(pdf_syntax::text_string::encode_text_string(text).into()),
        );
        true
    }

    /// The free text annotation a person added at a point, and what it says now.
    ///
    /// The point is in **default user space**, as every other question here takes it, and the
    /// answer is the **last** annotation covering it — §12.5.2 draws them in the order they were
    /// added and the one on top is the one under the pointer, which is the rule
    /// [`crate::view::annotation_at`] applies to the page's own array.
    ///
    /// **Only annotations this state added**, for [`ViewState::set_free_text`]'s reason: aiming a
    /// keyboard at one whose text nothing can change would be an interface that looks like it
    /// works.
    #[must_use]
    pub fn free_text_at(
        &self,
        document: &Document,
        page: &crate::Page,
        x: f32,
        y: f32,
    ) -> Option<(ObjectId, String)> {
        let (id, dict) = self.added_free_text_at(document, page, x, y)?;
        let text = crate::variable_text::string(document, &[&dict], "Contents").unwrap_or_default();
        Some((id, text))
    }

    /// [`Self::free_text_at`] with the dictionary rather than the text, for the caret's three
    /// questions.
    fn added_free_text_at(
        &self,
        document: &Document,
        page: &crate::Page,
        x: f32,
        y: f32,
    ) -> Option<(ObjectId, Dictionary)> {
        self.added_on(page.id)
            .filter(|added| {
                added
                    .dict
                    .get("Subtype")
                    .and_then(Object::as_name)
                    .is_some_and(|subtype| subtype.as_bytes() == b"FreeText")
            })
            .filter(|added| rectangle_covers(document, &added.dict, f64::from(x), f64::from(y)))
            .last()
            .map(|added| (added.id, added.dict.clone()))
    }

    /// The thing at a point that a person can put a caret in, whatever kind it is.
    ///
    /// Two kinds, because §12.7.4.3 lays text out for two: a widget whose field states text, and
    /// §12.5.6.6's free text annotation, whose clause sends it to that same subclause. The added
    /// annotation is asked about **first**, because the interpreter draws what a person added
    /// after the page's own `/Annots` and the last thing drawn is the thing on top.
    fn typeable_at(
        &self,
        document: &Document,
        page: &crate::Page,
        x: f32,
        y: f32,
    ) -> Option<(ObjectId, Dictionary)> {
        if let Some(found) = self.added_free_text_at(document, page, x, y) {
            return Some(found);
        }
        // The last widget covering the point, which is the one drawn on top (§12.5.2's order) and
        // the one `field_at` names — so the caret and the name a host asked for are the same
        // field's.
        let widget = widgets_at(document, page, x, y).last().copied()?;
        let dict = document.get(widget).as_dict().cloned()?;
        Some((widget, dict))
    }

    /// A number no object in the file and no annotation already added is using.
    ///
    /// The same rule [`Update::beside`] applies, and for the same reason: §7.5.5 makes `/Size`
    /// "one greater than the highest object number used in the file" and 68 of the corpus's 974
    /// documents write a cross-reference entry beyond their own, so the larger of the two wins.
    /// The count of annotations already added is added on top, because those numbers are spoken
    /// for even though nothing has been written yet.
    fn next_free_object(&self, document: &Document) -> ObjectId {
        let highest = document.xref().object_numbers().max().unwrap_or_default();
        let stated = document
            .trailer()
            .get("Size")
            .and_then(Object::as_integer)
            .and_then(|size| u32::try_from(size).ok())
            .unwrap_or_default();
        let base = highest.saturating_add(1).max(stated);
        let number = base.saturating_add(u32::try_from(self.added.len()).unwrap_or(u32::MAX));
        ObjectId {
            number,
            generation: 0,
        }
    }

    /// Every annotation a person added to one page, in the order they added them.
    ///
    /// What the interpreter draws after the page's own `/Annots`, which is where §12.5.5 puts
    /// them: an annotation is composited over "the page content along with any previously painted
    /// annotations", and one added later was painted later.
    /// A page reached without an identity — [`crate::Pages::detached`]'s, which is §12.7.7's
    /// template — has none of these by construction.
    pub fn added_on(&self, page: Option<ObjectId>) -> impl Iterator<Item = &Added> {
        let page = page.filter(|_| !self.added.is_empty());
        self.added
            .iter()
            .filter(move |added| Some(added.page) == page)
    }

    /// Every annotation a person added, in order.
    ///
    /// What a save writes and what a host asks to know whether there is anything to save.
    #[must_use]
    pub fn additions(&self) -> &[Added] {
        &self.added
    }

    /// Forgets every annotation a person added.
    ///
    /// The other half of [`ViewState::clear_all_fields`]: an undo replays the log's surviving
    /// prefix rather than inverting its last entry, so the state it replays onto has to be the
    /// one before any of it.
    pub fn clear_all_additions(&mut self) {
        self.added.clear();
    }

    /// Sets the value of every widget of one field, the way a person typing into it does.
    ///
    /// §12.7.4.2 makes a field's identity its *fully qualified name*, and §12.7.4.1 lets one
    /// field own several widget annotations — "a field's value is shared by all of its widgets"
    /// is the practical consequence, and it is why this takes a name and applies to a set. The
    /// name table is the same §12.7.4.2 walk §12.6.4.11's hide action, §12.7.6.3's reset and
    /// §12.7.8's import all use, so all four agree about what a field is called.
    ///
    /// [`Entered::Cleared`] clears the field, which is not the same as never having touched it:
    /// the first shows nothing and the second shows Table 226's `/V`.
    ///
    /// Returns how many widgets took the value. Zero means the document has no field of that
    /// name — a caller's mistake rather than a document's — that every widget of it is Table 227's
    /// `ReadOnly`, which is the document refusing, or that [`Entered::Chosen`] named §12.7.5.4's
    /// options on a field that is not one of §12.7.5.4's, which is a caller's mistake again.
    ///
    /// # What §12.7.5.4's two value shapes cost, and where they are decided
    ///
    /// [`Entered::Chosen`] names Table 234's `/Opt` entries by index and the clause states what
    /// `/V` then says:
    ///
    /// > If the field does not allow multiple selection -that is, if the MultiSelect flag ( PDF
    /// > 1.4 ) is not set -or if multiple selection is supported but only one item is currently
    /// > selected, V is a text string representing the selected item, as given in the field
    /// > dictionary's Opt array. If multiple items are selected, V is an array of such strings.
    /// > (For items represented in the Opt array by a two-element array, the name string is the
    /// > second of the two array elements.)
    ///
    /// So one index becomes a string, several become an array of strings, and none becomes no `/V`
    /// at all — "[t]he default value of V is null , indicating that no item is currently selected."
    /// That resolution happens **here**, once, because it needs the field's own `/Opt` and because
    /// the appearance, the description a host reads back and the file a save writes must not each
    /// do it separately. Table 234's `/I` is kept beside the value for the same reason: the entry
    /// is the indices, and deriving them again from labels that may repeat is exactly the tie `/I`
    /// exists for.
    ///
    /// Table 233 bit 22 is obeyed rather than carried — "if clear, at most one item shall be
    /// selected" — by taking the first index in `/Opt` order, which is the same shape ADR 0197
    /// gave Table 231 bit 24: this program is what selects, so the `shall` binds it, and cutting
    /// what will not fit keeps the control usable where refusing the whole edit would not.
    ///
    /// # What this does **not** consult, and where it went
    ///
    /// §12.8.2.2's `/DocMDP` and §7.6.4.2's Table 22 restrict this operation, and until the
    /// three-hundred-and-seventy-third session the first of them was checked here. It is not a
    /// question this function can answer: `CLAUDE.md` makes how much of a document's restrictions
    /// a reader obeys the *reader's* policy, with four levels of it, and two of those levels have
    /// to describe the operation to a person before it happens. A refusal expressed as a count of
    /// widgets can become none of that. So [`crate::restriction::asserted`] states what the
    /// document asserts — with its clause and its level — and the host that has the policy
    /// decides, once per operation, before calling this. ADR 0212.
    ///
    /// **Nothing is written to the file.** `CLAUDE.md`'s rule 1 makes the document immutable;
    /// what a person did is a log beside it, and turning that log into §7.5.6's incremental
    /// update is a separate operation with its own clause.
    ///
    /// # What Table 231 bit 24 takes away
    ///
    /// §12.7.5.3's `DoNotScroll` makes a full field stop accepting text — "[o]nce the field is
    /// full, no further text shall be accepted for interactive form filling" — and this is the
    /// one place in the tree a person's text is accepted, so it is where the `shall` binds. The
    /// value a widget takes is the **longest prefix of `value` that fits its annotation
    /// rectangle**, which is what "no further text" means for a host that sends whole values
    /// rather than keystrokes. Where the flag is clear, or the field is not a text field, or
    /// nothing about the widget can be laid out, the value is taken whole.
    ///
    /// **One field, one value, so the shortest prefix wins.** §12.7.4.1 makes a field's value
    /// shared by all of its widgets and Table 231's flag belongs to the *field*; a value that
    /// overflowed one widget's rectangle while fitting another's would be a field that is full
    /// and not full at once, and the reading that keeps every widget showing the whole value is
    /// the one that keeps the clause's sentence true of the field.
    pub fn set_field(&mut self, document: &Document, name: &str, value: &Entered) -> usize {
        let table = widgets_by_field_name(document);
        let Some(widgets) = table.get(name) else {
            return 0;
        };
        // Table 227 bit 1: an interactive processor shall not allow a *user* to change the
        // value. A person is exactly who this refuses, which is what separates it from
        // §12.7.6.3's reset and §12.7.8's import — both of those are the *document* changing
        // its own value, and neither is a user.
        let taking: Vec<ObjectId> = widgets
            .iter()
            .copied()
            .filter(|widget| !is_read_only(document, *widget))
            .collect();
        let entry = match value {
            Entered::Cleared => Entry {
                value: None,
                indices: None,
            },
            Entered::Text(text) => Entry {
                // §7.9.2.2's text string, which is what Table 226 makes `/V` for a text field and
                // so what §12.7.4.3 lays out. Encoded once, here, rather than again at the save:
                // the appearance and the file then carry the same bytes by construction.
                value: Some(Object::String(
                    pdf_syntax::text_string::encode_text_string(&accepted(document, &taking, text))
                        .into(),
                )),
                indices: None,
            },
            Entered::Chosen(indices) => match chosen(document, taking.first().copied(), indices) {
                Some(entry) => entry,
                None => return 0,
            },
        };
        let mut applied = 0_usize;
        for widget in &taking {
            // The four statements about a value answer one question, so a widget belongs to
            // exactly one of them: what a person typed is the latest of the four.
            self.reset.remove(widget);
            self.imported.remove(widget);
            self.edited.insert(*widget, entry.clone());
            applied = applied.saturating_add(1);
        }
        applied
    }

    /// What one field's value is *now*, as §12.7.4.3 would lay it out.
    ///
    /// The four statements about a value in their own order — what a person typed, what §12.7.8's
    /// import replaced it with, what §12.7.6.3's reset put back, and Table 226's `/V` — which is
    /// the order [`ViewState::annotation`] already resolves them in, asked here by *field* rather
    /// than by widget because §12.7.4.1 makes a value shared by all of a field's widgets.
    ///
    /// `None` means the field's value is **not text** — a button selects an appearance, a
    /// signature holds a dictionary — and `Some("")` means a text field with nothing in it. A host
    /// deciding where to send the keyboard needs those to be two answers, which is why an empty
    /// field is not folded into the absent one.
    ///
    /// A field a person **cleared** and one that never had a value both answer `Some("")`: the
    /// difference between them is a fact about the edit log rather than about what is on the page,
    /// and a host asking "what does this field say" is asking the second question.
    ///
    /// **Why a host needs this at all.** Since ADR 0197 a field carrying §12.7.5.3's `DoNotScroll`
    /// takes only as much of a value as fits its rectangle, so a host that kept its own buffer of
    /// what it had typed would diverge from the field on the first character past the edge. A host
    /// that reads this back after every keystroke cannot: the value it appends to is the value the
    /// document has.
    ///
    /// **And the string may not be the field's characters**, which is what the second half of the
    /// answer is for. Table 231 bit 14 — *"[c]haracters typed from the keyboard shall instead be
    /// echoed in some unreadable form, such as asterisks or bullet characters"* — makes a password
    /// field answer with bullets, and a host obeying the paragraph above would then write those
    /// bullets back as the next value. [`ShownValue::obscured`] is `true` for exactly that string,
    /// so the exception is something a caller reads rather than something it has to know. ADR 0247.
    #[must_use]
    pub fn field_value(&self, document: &Document, name: &str) -> Option<ShownValue> {
        let table = widgets_by_field_name(document);
        let widget = table.get(name)?.first().copied()?;
        let object = document.get(widget);
        let dict = object.as_dict()?;
        // The same `Field::read` the appearance takes, so that a host is told what will be drawn
        // rather than a second reading of the same four statements.
        crate::appearance::field_text_value(document, dict, self.annotation(widget).value)
    }

    /// Where the caret sits in whatever is at a point, as a segment in **default user space**.
    ///
    /// **Two kinds of thing answer**, and §12.5.6.6 is why the second does: a widget whose field
    /// §12.7.4.3 lays text out for, and a free text annotation a person added, whose own clause
    /// sends it to that same subclause. The rest of this comment is written of a field because
    /// that is where it started, and every word of it holds for the other.
    ///
    /// `[x0, y0, x1, y1]`: the end on the descent side of the baseline, then the end on the
    /// ascent side. Two points rather than a rectangle because a caret has no width — how thick
    /// it is drawn is a platform's convention and not a fact about the document — and because a
    /// widget's `/R`, its appearance's `/Matrix` or a `/DA`'s `Tm` can turn it, and a rectangle
    /// could only describe the cases where none of them does.
    ///
    /// The offset is a byte offset into the value [`Self::field_value`] answers with, clamped to
    /// its length: the two are the same string by construction, so a host that appends a
    /// character to what it read back can ask where the next one goes without translating
    /// anything. An offset inside a character counts that character as still to come.
    ///
    /// `None` where no widget covers the point, where the field is not one §12.7.4.3 lays text
    /// out for, and where the value could not be laid out at all — which is the same condition
    /// that makes the page report the field.
    ///
    /// **The standard states no caret.** §12.5.6.11's caret *annotation* is a different object
    /// entirely; what this is derived from is where §12.7.4.3 puts the next glyph, and what a
    /// cursor looks like is the host's. ADR 0211.
    #[must_use]
    pub fn caret_at(
        &self,
        document: &Document,
        page: &crate::Page,
        x: f32,
        y: f32,
        offset: usize,
    ) -> Option<[f32; 4]> {
        let (id, dict) = self.typeable_at(document, page, x, y)?;
        crate::appearance::caret(document, &dict, self.annotation(id), offset)
    }

    /// Which byte of a field's value a point inside it falls nearest, as an offset into it.
    ///
    /// [`Self::caret_at`]'s inverse, and what a click inside a value needs: that one takes an
    /// offset and answers a place, this takes a place and answers an offset. Both are computed
    /// inside §12.7.4.3's own layout walk, so an offset this answers with, handed straight back to
    /// [`Self::caret_at`], puts the cursor where the click was.
    ///
    /// `at` names the widget, in default user space, exactly as [`Self::caret_at`]'s point does;
    /// `point` is the place inside it to measure, in the same space. They are the same point on a
    /// click and different ones during a drag, which is the whole reason there are two: a drag
    /// that leaves the widget's rectangle is still a drag inside that field's value.
    ///
    /// **The answer is the nearest boundary and never a refusal**, which is a choice: a point past
    /// the end of a line answers that line's end and a point below every line answers the last.
    /// A press a host has already decided is a press into a field has to leave the cursor
    /// somewhere. ADR 0225.
    ///
    /// `None` in exactly the cases [`Self::caret_at`] answers `None` in — no widget under `at`, a
    /// field §12.7.4.3 lays no text out for, or a value that could not be laid out.
    #[must_use]
    pub fn offset_at(
        &self,
        document: &Document,
        page: &crate::Page,
        at: (f32, f32),
        point: (f32, f32),
    ) -> Option<usize> {
        let (id, dict) = self.typeable_at(document, page, at.0, at.1)?;
        crate::appearance::offset_at(document, &dict, self.annotation(id), point)
    }

    /// The shapes covering a byte range of a field's value, in **default user space**.
    ///
    /// One quadrilateral per line the range touches, `[x0, y0, … x3, y3]`, between the same ascent
    /// and descent the caret stands between — so a host draws a highlight the height of its own
    /// cursor. Four corners rather than a rectangle for the reason [`Self::caret_at`] answers two
    /// points: Table 192's `/R`, an appearance's `/Matrix` and a `/DA`'s `Tm` can each turn it.
    ///
    /// **Not two carets**, and that is the whole reason this exists: §12.7.5.3's Table 231 bit 13
    /// lets the layout break a value into lines a caller cannot see, so the lines *between* the
    /// two ends of a selection are this crate's to name. ADR 0225.
    ///
    /// The two offsets are into the value [`Self::field_value`] answers with, in either order, and
    /// the range covers `from` up to but not including `to`. Empty where it covers no glyph.
    #[must_use]
    pub fn field_selection(
        &self,
        document: &Document,
        page: &crate::Page,
        at: (f32, f32),
        range: (usize, usize),
    ) -> Option<Vec<[f32; 8]>> {
        let (id, dict) = self.typeable_at(document, page, at.0, at.1)?;
        crate::appearance::selection(document, &dict, self.annotation(id), range)
    }

    /// Forgets what a person typed into one field, leaving whatever the file and the actions say.
    ///
    /// The operation an undo needs, and it is deliberately not "set it back to the old value":
    /// the old value may have been the file's own, and re-stating that as an edit would make
    /// every later save carry a change nobody made.
    pub fn clear_field(&mut self, document: &Document, name: &str) -> usize {
        let table = widgets_by_field_name(document);
        let Some(widgets) = table.get(name) else {
            return 0;
        };
        for widget in widgets {
            self.edited.remove(widget);
        }
        widgets.len()
    }

    /// §7.5.6's incremental update for everything a person changed.
    ///
    /// The bytes of the *whole file*: the document as it was opened, unchanged, with an update
    /// appended. `CLAUDE.md` permits exactly this form of writing — the producer's bytes stay in
    /// the file, byte for byte, under what the person added — and the host writes them somewhere.
    ///
    /// # What is written
    ///
    /// One replacement object per field a value was typed into, carrying Table 226's `/V`, and
    /// the interactive form dictionary with Table 224's `/NeedAppearances` set true.
    ///
    /// **The flag is the honest half of this, and it is a decision with a cost.** A widget's
    /// appearance stream still says what the field said before, and a writer has two ways to fix
    /// that: regenerate every affected stream, or tell the next reader to. Table 224 exists for
    /// the second — "a flag specifying whether to construct appearance streams and appearance
    /// dictionaries for all widget annotations in the document" — and it is what this writes,
    /// because regenerating means writing content streams into somebody else's file and this
    /// program's own reading of them is what it would be writing. The cost, written down: a
    /// reader that ignores the flag shows the value the field had before. Every reader this
    /// project compares against honours it.
    ///
    /// # What is deliberately **not** written
    ///
    /// A value typed into a Table 231 bit 14 password field, and [`Written::withheld`] names every
    /// field it happened to. §12.7.5.3's own words, under that bit:
    ///
    /// > NOTE To protect password confidentiality, it is imperative that PDF processors never
    /// > store the value of the text field in the PDF file if this flag is set.
    ///
    /// A NOTE is informative and this one is obeyed anyway, because the alternative is this
    /// program writing a person's password into a file in clear text — and it did, until the
    /// four-hundred-and-eleventh session found the sentence while reading the clause ADR 0247's
    /// third amendment names. **Neither half of the edit is written**, which is what keeps the
    /// file consistent with itself: the producer's `/V` and the producer's `/AP` stay as they
    /// were and go on agreeing, where writing the appearance without the value would leave a
    /// widget drawing something §12.7.2 says its `/V` should decide. What a person typed lives in
    /// this log until the document is closed and nowhere else.
    ///
    /// # Errors
    ///
    /// [`pdf_syntax::write::UpdateError`], which names every document this refuses: one whose
    /// cross-reference table was rebuilt by scanning, one whose own encryption cannot be applied
    /// to what is written, and one whose trailer is missing what §7.5.5 requires. An *encrypted*
    /// document is no longer among them: §7.6.2's ciphers run on the way out, so the `/V` this
    /// writes reaches the file in the form the document's own key expects.
    pub fn save(&self, document: &Document) -> Result<Written, pdf_syntax::write::UpdateError> {
        let mut update = Update::beside(document);
        let mut withheld = Vec::new();
        self.write_additions(document, &mut update);
        for (widget, entered) in &self.edited {
            let widget = *widget;
            let Some(dict) = document.get(widget).as_dict().cloned() else {
                continue;
            };
            let value = entered.as_field_value();
            // Table 231 bit 14's NOTE, above. The same reading that makes `field_value` answer
            // with bullets is what decides it, so a field whose value a host is not allowed to
            // read back is a field whose value this does not store — one predicate, not two.
            if crate::appearance::field_text_value(document, &dict, value)
                .is_some_and(|shown| shown.obscured)
            {
                withheld.push(field_name_of(document, widget));
                continue;
            }
            // §12.7.4.1's `/V` is inheritable, so the field that *holds* the value may be an
            // ancestor of the widget — and writing it onto the widget would leave the ancestor's
            // stale value inherited by the field's other widgets. The value goes where the
            // document already keeps one, or on the widget where the document keeps none.
            let (id, mut field) = holder(document, widget, dict.clone());
            match entered.value.as_ref() {
                // Already the object §12.7.5.4 and §12.7.5.3 say `/V` is — a string, or an array
                // of strings for several selected items — because `set_field` resolved it against
                // the field's own `/Opt`. Encoding it a second time here is how the file and the
                // appearance would come to disagree.
                Some(object) => {
                    field.insert(Name::new(&b"V"[..]), object.clone());
                }
                // §12.7.6.3's own words for a value that is gone: "its V entry shall be removed".
                None => {
                    field.remove("V");
                }
            }
            // Table 234's `/I` is written for a single selection too, because it is the only
            // entry that says *which* `/Opt` element was chosen where two of them carry the same
            // name string. This line used to justify that with the entry's own "shall be used ...
            // when the value of the choice field is an array" and to read as a stretch of it;
            // Errata Collection 3 struck that trigger out and opened the row to choice fields at
            // large rather than to multiple-selection ones — Issue #468, `/State` `Review`
            // `Accepted`, unreadable from `doc/md/` until ADR 0253. The stretch was the rule. An
            // edit that named no options takes it out: an `/I` left standing beside a `/V` it does
            // not describe is a file contradicting itself, and §12.7.5.4's tie-break would then be
            // arbitrating a disagreement this program wrote.
            match entered.indices.as_ref() {
                Some(indices) => {
                    // "sorted in ascending order", which `set_field` established, and every index
                    // is a position in the `/Opt` array it bounded them by — so what is converted
                    // here is a `Vec` position and the filter drops nothing on any target this
                    // builds for.
                    let entries: Vec<Object> = indices
                        .iter()
                        .filter_map(|index| i64::try_from(*index).ok())
                        .map(Object::Integer)
                        .collect();
                    field.insert(Name::new(&b"I"[..]), Object::Array(entries));
                }
                None => {
                    field.remove("I");
                }
            }
            update.put(id, Object::Dictionary(field));
            update.write_appearance(document, widget, &dict, value);
        }
        if !update.is_empty()
            && let Some((id, mut form)) = interactive_form(document)
        {
            // Table 224's flag is written only for what this program could not write itself.
            // §12.7.2 states what setting it admits — "[i]f such an object defines an appearance
            // stream, the appearance shall be consistent with the object's current value as a
            // field" — so a document whose every changed widget got a new stream is one where
            // that obligation is *kept*, and asking the next reader to redo the work would be
            // saying otherwise.
            if update.needs_appearances {
                form.insert(Name::new(&b"NeedAppearances"[..]), Object::Boolean(true));
                update.put(id, Object::Dictionary(form));
            }
        }
        if !update.is_empty()
            && let Some((id, catalog)) = withdrawn_usage_rights(document)
        {
            update.put(id, Object::Dictionary(catalog));
        }
        let bytes = pdf_syntax::write::incremental_update(document, &update.replacements)?;
        withheld.sort_unstable();
        withheld.dedup();
        Ok(Written { bytes, withheld })
    }

    /// Writes every annotation a person added, and attaches each to its page.
    ///
    /// Two objects per annotation, because §12.5.2 says where an annotation lives: "each page
    /// object shall contain an `/Annots` entry … an array of annotation dictionaries". So the
    /// annotation is written under the number it was given when it was added, and the *page* is
    /// rewritten with the reference appended — which is the half of §7.5.6's "changed, replaced,
    /// or deleted" that a new object needs, and the same half Table 224's widgets already use.
    ///
    /// **Appended rather than inserted**, because the array's order is the drawing order the
    /// same clause states, and a mark a person made last belongs on top of what was there.
    ///
    /// Table 166's `/P` is added here rather than at [`ViewState::add_markup`]: it is "an
    /// indirect reference to the page object with which this annotation is associated", which is
    /// a statement about the *file* and so belongs to the writing rather than to the log.
    fn write_additions(&self, document: &Document, update: &mut Update) {
        for added in &self.added {
            // The numbers were allocated when the annotation was added, so nothing else in this
            // update may reach for them.
            update.reserve(added.id);
        }
        // §12.7.4.3's `shall` about the `/DA` [`ViewState::add_free_text`] writes, met by the
        // writing rather than left to the next reader to recover from.
        if self.added.iter().any(|added| {
            added
                .dict
                .get("Subtype")
                .and_then(Object::as_name)
                .is_some_and(|subtype| subtype.as_bytes() == b"FreeText")
        }) {
            update.state_default_font(document);
        }
        for added in &self.added {
            let mut dict = added.dict.clone();
            dict.insert(Name::new(&b"P"[..]), Object::Reference(added.page));
            write_added_appearance(document, update, &mut dict);
            update.put(added.id, Object::Dictionary(dict));

            let Some(mut page) = update.current(document, added.page) else {
                continue;
            };
            // **Where the array is matters.** `/Annots` may be written inline or as a reference
            // to an array object, and both are ordinary — so an inline array is rewritten in the
            // page and a referenced one is rewritten *where it is*. Inlining a referenced array
            // would leave the original object in the file saying something else, which §7.5.6's
            // "most recent copy" rule would then have to arbitrate for no reason.
            match page.get("Annots").cloned() {
                Some(Object::Reference(id)) => {
                    let mut entries = match update.current_object(document, id) {
                        Object::Array(entries) => entries,
                        _ => Vec::new(),
                    };
                    entries.push(Object::Reference(added.id));
                    update.put(id, Object::Array(entries));
                }
                other => {
                    let mut entries = match other {
                        Some(Object::Array(entries)) => entries,
                        _ => Vec::new(),
                    };
                    entries.push(Object::Reference(added.id));
                    page.insert(Name::new(&b"Annots"[..]), Object::Array(entries));
                    update.put(added.page, Object::Dictionary(page));
                }
            }
        }
    }

    /// Forgets every value a person typed, leaving the file's own and whatever actions did.
    ///
    /// What a replay of the edit log starts from: an undo re-applies the log's surviving prefix
    /// rather than inverting its last entry, so the state it applies to has to be the one before
    /// any of it. See `viewer-core`'s `Open::replay` for why replaying beats inverting.
    pub fn clear_all_fields(&mut self) {
        self.edited.clear();
    }

    /// Every field a person has typed into or chosen among, by widget, in object order.
    ///
    /// What a host asks to know whether there is anything to save. The value is the statement
    /// [`ViewState::annotation`] would answer with, which is the one the appearance is drawn from
    /// — so a caller looking at this and a caller looking at the page are looking at one reading.
    pub fn edits(&self) -> impl Iterator<Item = (ObjectId, FieldValue<'_>)> {
        self.edited
            .iter()
            .map(|(widget, entered)| (*widget, entered.as_field_value()))
    }

    /// Adds §12.7.8.3.3's template pages, resolved through §12.7.7's name trees.
    ///
    /// A template page is a page **this document already holds** — §12.7.7 puts it in the
    /// catalog's `/Templates` name tree, outside the page tree so that it is not displayed until
    /// something asks for it — so adding one costs a name lookup and no page content at all.
    ///
    /// The name trees are read only when the FDF file states a page, which is never for any
    /// document anyone has opened: `CLAUDE.md`'s "nothing eager" applies, and this is the one
    /// caller either tree has.
    ///
    /// Two refusals, each named rather than dropped. Table 253's `/F` names a template in
    /// *another file*, which this reader has no filesystem to open — `GoToR`'s reason exactly. A
    /// `/TRef` naming no page in either tree is a file asking for something the document does
    /// not contain, which is the one case §12.7.7's own invariants cannot catch.
    fn append_templates(
        &mut self,
        document: &Document,
        data: &crate::forms_data::FormsData,
        outcome: &mut Imported,
    ) {
        if data.pages.is_empty() {
            return;
        }
        let named = crate::named_page::NamedPages::read(document);
        for page in &data.pages {
            for template in &page.templates {
                let reference = &template.reference;
                if let Some(file) = &reference.file {
                    outcome.refused.push(format!(
                        "the template {} is in {file}, which this reader has no filesystem to                          open",
                        reference.name
                    ));
                    continue;
                }
                let Some(id) = named.lookup(&reference.name) else {
                    outcome.refused.push(format!(
                        "this document names no page {}, in either §12.7.7 tree",
                        reference.name
                    ));
                    continue;
                };
                self.appended.push(id);
                outcome.pages = outcome.pages.saturating_add(1);
            }
        }
    }

    /// The pages §12.7.8.3.3's imported templates have added, in the order they were added.
    ///
    /// Empty for every document until an import-data action names an FDF file with a `/Pages`
    /// entry. A caller showing them puts them after the document's own, which is the only order
    /// the clause's "add … to the document" leaves available: §12.7.8.3.3 states no position.
    #[must_use]
    pub fn appended_pages(&self) -> &[ObjectId] {
        &self.appended
    }

    /// Sets a group's state the way a *layer panel* does, and answers whether it changed.
    ///
    /// This is §8.11.4.5's other half — "[t]he user may manipulate optional content group states
    /// manually or by triggering set-OCG-state actions" — and the two differ in exactly one
    /// respect, which is why they are two functions. Table 99's `/Locked` says "[t]he state of a
    /// locked group cannot be changed through the user interface of an interactive PDF
    /// processor", and the clause's next sentence permits the other route: "[a]n interactive PDF
    /// processor may allow the states of optional content groups to be changed by means other
    /// than the user interface, such as ECMAScript or items in the AS entry". So a lock stops
    /// this and does not stop [`Action::SetOcgState`].
    ///
    /// Radio-button collections apply either way. `/PreserveRB` is a set-OCG-state action's
    /// entry and has no counterpart for a person; Table 99 states the paradigm unconditionally,
    /// so a panel that let two members of one collection be on would be showing a state the
    /// configuration says cannot exist.
    ///
    /// `false` for a group the document never declared, one the configuration's `/Intent` does
    /// not cover, and one `/Locked` names — three different reasons a switch does nothing, and
    /// a panel that wants to distinguish them has [`OptionalContent::is_locked`] and
    /// [`OptionalContent::state`].
    pub fn set_group(&mut self, group: ObjectId, on: bool) -> bool {
        let Some(content) = self.optional_content.as_mut() else {
            return false;
        };
        if content.is_locked(group) || content.state(group).is_none_or(|was| was == on) {
            return false;
        }
        content.apply(&[(group, if on { Change::On } else { Change::Off })], true);
        true
    }

    /// Performs one action, and answers what it asks of the viewer.
    ///
    /// `Some` for the three types that change something this state does not hold: which page
    /// is shown (§12.6.4.2's destination and §12.6.4.12's page commands) and what is outside
    /// the document altogether (§12.6.4.8's URI). Everything else is performed here and
    /// answers `None`, because a layer's state and an annotation's Hidden flag are what a
    /// `ViewState` *is*.
    ///
    /// A [`Action::Refused`] does nothing and is not an error: the action is named so that a
    /// caller may say what it declined to do, and doing nothing is what §12.6.1's list of
    /// twenty types leaves a renderer that implements five of them.
    pub fn perform(&mut self, document: &Document, action: &Action) -> Option<Request> {
        match action {
            Action::GoTo(destination) => return Some(Request::Display(*destination)),
            Action::Named(named) => return Some(Request::Page(*named)),
            Action::Uri(uri) => return Some(Request::Resolve(uri.clone())),
            Action::Thread(jump) => return Some(Request::Thread(jump.clone())),
            Action::GoToDp(jump) => return Some(Request::DocumentPart(*jump)),
            Action::ImportData(import) => return Some(Request::Import(import.clone())),
            Action::GoToE(target) => return Some(Request::Embedded(target.clone())),
            Action::Trans(transition) => {
                return Some(Request::Transition(transition.clone()));
            }
            Action::SetOcgState(state) => {
                if let Some(content) = self.optional_content.as_mut() {
                    content.apply(&state.changes, state.preserve_radio_buttons);
                }
            }
            Action::Hide(hide) => self.hide(document, hide),
            Action::ResetForm(reset) => self.reset_form(document, reset),
            Action::Refused(_) => {}
        }
        None
    }

    /// Performs a whole `/Next` chain and answers everything it asks of the viewer, in order.
    ///
    /// A list rather than one answer, because §12.6.2 makes a chain a sequence of actions and
    /// two of them may each ask for something: a link may open a URI and then jump. Every
    /// action is performed even after one that moves the page — a `/SetOCGState` after a
    /// `/GoTo` is a layer change for the page being navigated *to*, and the clause states no
    /// ordering that would drop it — while §12.6.2's NOTE 1 leaves the caller to decide what
    /// to do with two requests, since it is the caller that knows whether the first made the
    /// second impossible.
    pub fn perform_all(&mut self, document: &Document, actions: &[Action]) -> Vec<Request> {
        let mut requests = Vec::new();
        for action in actions {
            if let Some(request) = self.perform(document, action) {
                requests.push(request);
            }
        }
        requests
    }

    /// §12.6.4.11 applied: every annotation the action names, in either of `/T`'s two forms.
    fn hide(&mut self, document: &Document, action: &Hide) {
        let mut fields: Option<BTreeMap<String, Vec<ObjectId>>> = None;
        for target in &action.targets {
            match target {
                HideTarget::Annotation(id) => self.set_hidden(*id, action.hide),
                HideTarget::Field(name) => {
                    // Built at most once per action, and only for an action that names a
                    // field: the walk is over `/AcroForm /Fields`, which most documents do
                    // not have and no document needs for the reference form.
                    let table = fields.get_or_insert_with(|| widgets_by_field_name(document));
                    for id in table.get(name).into_iter().flatten() {
                        self.set_hidden(*id, action.hide);
                    }
                }
            }
        }
    }

    /// Performs §12.7.6.3's reset over the widgets the action names.
    ///
    /// The clause resets *fields*; what this program draws is *widgets*, and one field may have
    /// several — §12.7.4.1's field tree ends in the annotations that show it. So the set kept
    /// here is of widget annotations, resolved once through the same table §12.6.4.11's hide
    /// action uses.
    ///
    /// Three shapes, all Table 241's and Table 242's:
    ///
    /// - no `/Fields` at all: "all fields in the document's interactive form are reset";
    /// - `/Fields` with the flag clear: those fields, "[a]ll descendants of the specified fields
    ///   in the field hierarchy" included — which §12.7.4.2's naming makes a prefix test, since
    ///   a descendant's fully qualified name is its ancestor's with `.` and more appended;
    /// - `/Fields` with the flag set: everything *except* those.
    fn reset_form(&mut self, document: &Document, action: &ResetForm) {
        let table = widgets_by_field_name(document);
        if action.fields.is_empty() {
            self.reset.extend(table.values().flatten().copied());
            self.imported.clear();
            return;
        }
        let named: BTreeSet<ObjectId> = action
            .fields
            .iter()
            .flat_map(|target| match target {
                ResetTarget::Field(id) => vec![*id],
                ResetTarget::Name(name) => table
                    .iter()
                    .filter(|(candidate, _)| {
                        *candidate == name
                            || candidate
                                .strip_prefix(name.as_str())
                                .is_some_and(|rest| rest.starts_with('.'))
                    })
                    .flat_map(|(_, widgets)| widgets.iter().copied())
                    .collect(),
            })
            .collect();
        if action.exclude {
            for widget in table.values().flatten() {
                if !named.contains(widget) {
                    self.reset.insert(*widget);
                }
            }
        } else {
            self.reset.extend(named);
        }
        // §12.7.8's import and this are two statements about one field's value, so the later
        // one stands alone — see the `imported` field. A reset performed after an import is the
        // person asking for the document's own defaults back.
        self.imported
            .retain(|widget, _| !self.reset.contains(widget));
    }

    /// Whether this widget's value has been reset to its default (§12.7.6.3).
    ///
    /// A widget rather than a field, for [`Self::reset_form`]'s reason. `false` for every
    /// annotation in every document that has performed no reset, which is all of them until a
    /// person clicks a button.
    #[must_use]
    pub fn is_reset(&self, annotation: ObjectId) -> bool {
        self.reset.contains(&annotation)
    }

    /// Records one annotation's new state, keeping the two sets disjoint.
    fn set_hidden(&mut self, annotation: ObjectId, hide: bool) {
        if hide {
            self.shown.remove(&annotation);
            self.hidden.insert(annotation);
        } else {
            self.hidden.remove(&annotation);
            self.shown.insert(annotation);
        }
    }
}

/// Every widget annotation in the document, keyed by its field's fully qualified name.
///
/// §12.7.4.1 defines the tree and §12.7.4.2 the name built over it:
///
/// > For a field with no parent, the partial and fully qualified names are the same. For a
/// > field that is the child of another field, the fully qualified name shall be formed by
/// > appending the child field's partial name to the parent's fully qualified name, separated
/// > by a PERIOD (2Eh)
///
/// The two subtleties are both in the clause. A field's widget annotation may be the field
/// dictionary *itself* — §12.5.6.19 says the two "may be merged into a single dictionary" —
/// so a leaf with no `/Kids` is its own widget. And a kid with no `/T` "shall not be
/// considered a field but simply a Widget annotation", so it belongs to its parent's name
/// rather than starting a new one.
pub fn widgets_by_field_name(document: &Document) -> BTreeMap<String, Vec<ObjectId>> {
    let mut out = BTreeMap::new();
    let Ok(catalog) = document.catalog() else {
        return out;
    };
    let form = document.get_key(&catalog, "AcroForm");
    let Some(form) = form.as_dict() else {
        return out;
    };
    let fields = document.get_key(form, "Fields");
    let Some(fields) = fields.as_array().map(<[Object]>::to_vec) else {
        return out;
    };
    let mut seen = BTreeSet::new();
    for field in &fields {
        walk(document, field, "", &mut out, &mut seen, 0);
    }
    out
}

/// Walks one node of the field tree, extending the qualified name as §12.7.4.2 states.
fn walk(
    document: &Document,
    node: &Object,
    prefix: &str,
    out: &mut BTreeMap<String, Vec<ObjectId>>,
    seen: &mut BTreeSet<ObjectId>,
    depth: usize,
) {
    if depth > MAX_FIELD_DEPTH {
        return;
    }
    let Some(id) = node.as_reference() else {
        // A field has to be an indirect object for anything to name it; a direct dictionary
        // in `/Kids` has no identity a hide action could reach.
        return;
    };
    if !seen.insert(id) {
        return;
    }
    let resolved = document.get(id);
    let Some(dict) = resolved.as_dict() else {
        return;
    };

    let name = qualified_name(document, dict, prefix);
    let kids = document.get_key(dict, "Kids");
    match kids.as_array().map(<[Object]>::to_vec) {
        Some(kids) if !kids.is_empty() => {
            for kid in &kids {
                walk(document, kid, &name, out, seen, depth.saturating_add(1));
            }
        }
        // A leaf: the field dictionary and its widget annotation merged into one.
        _ => out.entry(name).or_default().push(id),
    }
}

/// This node's fully qualified name: the prefix, and its own `/T` if it has one.
fn qualified_name(document: &Document, dict: &Dictionary, prefix: &str) -> String {
    let Object::String(bytes) = document.get_key(dict, "T") else {
        return prefix.to_owned();
    };
    let partial = pdf_syntax::text_string(&bytes);
    if prefix.is_empty() {
        partial
    } else {
        format!("{prefix}.{partial}")
    }
}

/// Whether pressing this annotation changes what is drawn.
///
/// Two clauses can make it so, and a caller has to ask about both. Table 170's `/D` is the
/// appearance §12.5.5 shows while a button is down, and Table 191's `/H` is the highlighting
/// mode §12.5.6.19 states for the same moment — and `/H`'s default is `I`, so an annotation that
/// states *neither* entry still inverts.
///
/// Asked before the pointer state is changed at all, because changing it invalidates the page's
/// display list: a cursor crossing an annotation for which a press would change nothing would
/// otherwise re-interpret the page for a picture that cannot differ. That is a real cost — 2 000 M
/// instructions on the benchmark page — and it is why this exists rather than the caller looking
/// for an `/AP` `/D` and stopping there, which is what `viewer-core` did until the
/// hundred-and-thirty-eighth session and which left §12.5.6.19 unreachable from the one program
/// that has a mouse.
///
/// **A third clause joined them in the two-hundred-and-fifty-third session**: §12.5.3's
/// `ToggleNoView` decides whether the annotation is drawn *at all* while the pointer is on it,
/// which is the largest change a press can make to a picture.
#[must_use]
pub fn press_changes_appearance(
    document: &Document,
    annotation: ObjectId,
    view: AnnotationView<'_>,
) -> bool {
    let object = document.get(annotation);
    let Some(dict) = object.as_dict() else {
        return false;
    };
    crate::annotation::press_changes(document, dict, view)
}

/// Whether the cursor arriving on this annotation changes what is drawn.
///
/// The hovering half of [`press_changes_appearance`], and it exists for the same reason: the
/// pointer state invalidates the page's display list, so it is only changed where the picture can
/// differ. Table 170's `/R` is the appearance §12.5.5 shows "when the user moves the cursor into
/// the annotation's active area without pressing the mouse button", and §12.5.3's `ToggleNoView`
/// decides whether the annotation is drawn at all while it is there.
#[must_use]
pub fn hover_changes_appearance(
    document: &Document,
    annotation: ObjectId,
    view: AnnotationView<'_>,
) -> bool {
    let object = document.get(annotation);
    let Some(dict) = object.as_dict() else {
        return false;
    };
    crate::annotation::hover_changes(document, dict, view)
}

/// The objects one save writes, and the object numbers it is allowed to invent.
///
/// §7.5.6's update may *add* objects as well as replace them — the clause's own list is "objects
/// that have been changed, replaced, or deleted" — and a widget with no `/AP` needs one added,
/// because §7.3.8.1 makes every stream an indirect object and there is nowhere else to put it.
struct Update {
    /// What the update writes, by object.
    replacements: BTreeMap<ObjectId, Object>,
    /// The next object number nothing in the file uses.
    next: u32,
    /// Whether any widget's appearance had to be left to the next reader.
    needs_appearances: bool,
}

impl Update {
    /// An empty update beside a document, knowing which object numbers are free.
    ///
    /// **Both sources of the answer are consulted and the larger wins.** §7.5.5 makes `/Size`
    /// "one greater than the highest object number used in the file", but 68 of the corpus's 974
    /// documents write at least one cross-reference entry beyond their own `/Size` — see the
    /// §7.5.5 ledger row, where the same understatement costs 66 documents their page tree if the
    /// clause's ignore rule is applied. Trusting `/Size` alone here would be worse than that: a
    /// new object would land on an existing one's number and silently replace it.
    fn beside(document: &Document) -> Self {
        let highest = document.xref().object_numbers().max().unwrap_or_default();
        let stated = document
            .trailer()
            .get("Size")
            .and_then(Object::as_integer)
            .and_then(|size| u32::try_from(size).ok())
            .unwrap_or_default();
        Self {
            replacements: BTreeMap::new(),
            next: highest.saturating_add(1).max(stated),
            needs_appearances: false,
        }
    }

    /// States [`FREE_TEXT_FONT`] in Table 224's `/DR`, where the document states nothing there.
    ///
    /// §12.7.4.3 puts a `shall` on the `/DA` this program writes, and it is about a *different*
    /// dictionary from the one the `/DA` is in:
    ///
    /// > The specified font value shall match a resource name in the Font entry of the default
    /// > resource dictionary (referenced from the DR entry of the interactive form dictionary; see
    /// > "Table 224 -Entries in the interactive form dictionary").
    ///
    /// So a file carrying a free text annotation whose `/DA` names a font `/DR` does not define is
    /// a file that breaks the clause — which is a thing this program *reads* six of in the corpus
    /// and recovers from by name, and a thing it may not *write*. `/AP` taking precedence over
    /// `/DA` (Table 177) does not settle it: the next reader to regenerate the appearance is
    /// exactly the one the entry is for.
    ///
    /// **The document's own definition always wins**, because the clause's sentence is satisfied
    /// the moment `/DR` states the name and the definition is then the document's opinion about
    /// its own resource, which is the same rule `variable_text`'s `Resolution::Named` follows when
    /// drawing. Nothing is written where `/Font` already has the key.
    ///
    /// **Where the file keeps each level is where each is rewritten**, which is `/Annots`' rule one
    /// clause over: the innermost indirect object on the path is the one replaced, and the levels
    /// above it are folded into it only where the file wrote them inline. A document with no
    /// interactive form dictionary at all gets one, with Table 224's Required `/Fields` as the
    /// empty array — a form with no fields, which is what the document has.
    fn state_default_font(&mut self, document: &Document) {
        let Ok(catalog) = document.catalog() else {
            return;
        };
        let held = |update: &Self, entry: Option<&Object>| match entry {
            Some(Object::Reference(id)) => (Some(*id), update.current(document, *id)),
            other => (None, other.and_then(Object::as_dict).cloned()),
        };
        let (form_id, form) = held(self, catalog.get("AcroForm"));
        let mut form = form.unwrap_or_else(|| {
            let mut form = Dictionary::default();
            // Table 224: "(Required) An array of references to the document's root fields."
            form.insert(Name::new(&b"Fields"[..]), Object::Array(Vec::new()));
            form
        });
        let (resources_id, resources) = held(self, form.get("DR"));
        let mut resources = resources.unwrap_or_default();
        let (fonts_id, fonts) = held(self, resources.get("Font"));
        let mut fonts = fonts.unwrap_or_default();
        if fonts.get(FREE_TEXT_FONT).is_some() {
            return;
        }
        let mut font = Dictionary::default();
        for (key, value) in [
            (&b"Type"[..], &b"Font"[..]),
            (&b"Subtype"[..], &b"Type1"[..]),
            (&b"BaseFont"[..], FREE_TEXT_BASE_FONT.as_bytes()),
        ] {
            font.insert(Name::new(key), Object::Name(Name::new(value)));
        }
        fonts.insert(
            Name::new(FREE_TEXT_FONT.as_bytes()),
            Object::Dictionary(font),
        );
        if let Some(id) = fonts_id {
            self.put(id, Object::Dictionary(fonts));
            return;
        }
        resources.insert(Name::new(&b"Font"[..]), Object::Dictionary(fonts));
        if let Some(id) = resources_id {
            self.put(id, Object::Dictionary(resources));
            return;
        }
        form.insert(Name::new(&b"DR"[..]), Object::Dictionary(resources));
        if let Some(id) = form_id {
            self.put(id, Object::Dictionary(form));
            return;
        }
        let Some(root) = document
            .trailer()
            .get("Root")
            .and_then(Object::as_reference)
        else {
            return;
        };
        let mut catalog = catalog;
        catalog.insert(Name::new(&b"AcroForm"[..]), Object::Dictionary(form));
        self.put(root, Object::Dictionary(catalog));
    }

    /// Records what one object now says, replacing anything already recorded for it.
    fn put(&mut self, id: ObjectId, object: Object) {
        self.replacements.insert(id, object);
    }

    /// Whether this update writes nothing.
    fn is_empty(&self) -> bool {
        self.replacements.is_empty()
    }

    /// Keeps [`Self::allocate`] away from a number already spoken for.
    ///
    /// An annotation a person added was given its number when they added it — it is that
    /// annotation's identity for as long as the document is open — so the update's own
    /// allocation has to start past it, or an appearance stream would land on top of it.
    fn reserve(&mut self, id: ObjectId) {
        self.next = self.next.max(id.number.saturating_add(1));
    }

    /// A number no object in the file or in this update has.
    fn allocate(&mut self) -> ObjectId {
        let number = self.next;
        self.next = self.next.saturating_add(1);
        ObjectId {
            number,
            generation: 0,
        }
    }

    /// What this object says now, whatever kind it is: the update's copy, else the file's.
    fn current_object(&self, document: &Document, id: ObjectId) -> Object {
        self.replacements
            .get(&id)
            .cloned()
            .unwrap_or_else(|| document.get(id))
    }

    /// What this object says now: the update's own copy if it has one, else the file's.
    fn current(&self, document: &Document, id: ObjectId) -> Option<Dictionary> {
        match self.replacements.get(&id) {
            Some(object) => object.as_dict().cloned(),
            None => document.get(id).as_dict().cloned(),
        }
    }

    /// Writes §12.7.4.3's appearance stream for a widget whose value this update changed.
    ///
    /// The clause states where it goes and nothing about who writes it out:
    ///
    /// > The new appearance stream becomes the normal appearance ( N ) in the appearance
    /// > dictionary associated with the field's widget annotation
    ///
    /// — which is the same sentence whether the stream is constructed for one render or kept in
    /// the file, and the difference is only whether the next reader has to do it again.
    ///
    /// The bytes are the ones this program would *draw*, from
    /// [`crate::appearance::for_saving`], so a saved file shows what the viewer showed. Two
    /// things are deliberately kept from whatever stream was there: its dictionary, so that
    /// `/Matrix` and anything else its producer stated survives, and its object number, so the
    /// update replaces one object rather than adding one and orphaning another.
    ///
    /// A widget this cannot produce a stream for — and one it can produce only part of — leaves
    /// [`Self::needs_appearances`] set, which is what Table 224's flag is for. Writing a stream
    /// that is missing a glyph *and* setting the flag says both true things: here is what could
    /// be laid out, and it is not all of it.
    fn write_appearance(
        &mut self,
        document: &Document,
        widget: ObjectId,
        dict: &Dictionary,
        value: FieldValue<'_>,
    ) {
        let built = match crate::appearance::for_saving(document, dict, value) {
            crate::appearance::ForSaving::Stream(built) => built,
            crate::appearance::ForSaving::Selected => return,
            crate::appearance::ForSaving::Owed => {
                self.needs_appearances = true;
                return;
            }
        };
        self.needs_appearances |= built.report.is_some();

        let added = built.existing.is_none();
        let (stream_id, mut stream_dict) = match built.existing {
            Some((id, dict)) => (id, dict),
            None => (self.allocate(), Dictionary::new()),
        };
        // §8.10.2's three required entries, plus the resources §12.7.4.3 builds from `/DR`.
        stream_dict.insert(
            Name::new(&b"Type"[..]),
            Object::Name(Name::new(&b"XObject"[..])),
        );
        stream_dict.insert(
            Name::new(&b"Subtype"[..]),
            Object::Name(Name::new(&b"Form"[..])),
        );
        stream_dict.insert(
            Name::new(&b"BBox"[..]),
            Object::Array(
                built
                    .bbox
                    .iter()
                    .map(|edge| Object::Real(f64::from(*edge)))
                    .collect(),
            ),
        );
        stream_dict.insert(
            Name::new(&b"Resources"[..]),
            Object::Dictionary(built.resources),
        );
        stream_dict.insert(
            Name::new(&b"Length"[..]),
            Object::Integer(i64::try_from(built.content.len()).unwrap_or(i64::MAX)),
        );
        // The bytes written are the decoded ones, so whatever the old stream said it was encoded
        // with is now a lie about it. §7.4's `/Filter` and `/DecodeParms` go with the data they
        // described — a stream keeping a `/FlateDecode` it no longer has would not decode at all.
        stream_dict.remove("Filter");
        stream_dict.remove("DecodeParms");
        self.put(
            stream_id,
            Object::Stream(std::sync::Arc::new(pdf_syntax::Stream {
                dict: stream_dict,
                data: built.content.into(),
                decryption_failed: false,
            })),
        );

        if added {
            // A widget that had no `/AP` needs one pointing at the stream just written. Table
            // 168's `/N` is "the annotation's normal appearance"; a widget with one state has it
            // as the stream itself rather than as a subdictionary of states.
            let Some(mut widget_dict) = self.current(document, widget) else {
                return;
            };
            let mut appearances = document
                .get_key(&widget_dict, "AP")
                .as_dict()
                .cloned()
                .unwrap_or_default();
            appearances.insert(Name::new(&b"N"[..]), Object::Reference(stream_id));
            widget_dict.insert(Name::new(&b"AP"[..]), Object::Dictionary(appearances));
            self.put(widget, Object::Dictionary(widget_dict));
        }
    }
}

/// The object a widget's value belongs on, walking §12.7.4.1's `/Parent` chain.
///
/// Two stopping conditions, in this order. A dictionary that already states a `/V` is the one
/// whose value the inheritance is reading, so replacing it is replacing what the widget shows. A
/// dictionary that states a `/FT` is the *field* — Table 226 makes the type an entry of the
/// field rather than of its widgets — and where nothing in the chain has a value yet, the field
/// is where one belongs: writing it onto one widget of a field with several would leave the
/// others reading the old inheritance.
///
/// The bound is this module's own, for the reason §12.7.4.1's row gives: a `/Parent` chain in a
/// hostile file can be a cycle, and a chain nobody can reach the top of stops rather than being
/// followed for ever.
fn holder(document: &Document, widget: ObjectId, dict: Dictionary) -> (ObjectId, Dictionary) {
    let (mut id, mut current) = (widget, dict);
    for _ in 0..MAX_FIELD_DEPTH {
        if !document.get_key(&current, "V").is_null() || current.get("FT").is_some() {
            break;
        }
        let Some(parent) = current.get("Parent").and_then(Object::as_reference) else {
            break;
        };
        let Some(next) = document.get(parent).as_dict().cloned() else {
            break;
        };
        (id, current) = (parent, next);
    }
    (id, current)
}

/// The catalog's `/AcroForm`, with the object it is, where the document states one indirectly.
///
/// An interactive form stated *directly* in the catalog has no identity of its own to replace, so
/// it answers `None` and the flag is not written — which leaves the appearance streams as the
/// file's own. §12.7.3 makes `/AcroForm` a dictionary rather than a reference in principle; every
/// real document writes it indirectly, and 0 of the 974 corpus documents do otherwise.
fn interactive_form(document: &Document) -> Option<(ObjectId, Dictionary)> {
    let catalog = document.catalog().ok()?;
    let id = catalog.get("AcroForm").and_then(Object::as_reference)?;
    Some((id, document.get(id).as_dict().cloned()?))
}

/// §12.8.2.3's withdrawal: the object to rewrite so that `/UR3` is gone, if it has to go.
///
/// > A PDF processor that modifies a PDF, with a UR signature in excess of the rights that are
/// > granted by that signature, should remove that signature prior to writing the newly modified
/// > PDF.
///
/// A `should`, and the only clause in §12.8 that binds this program because it *writes* rather
/// than because it reads. What it modifies is a form field's value and then the file itself, so
/// the two rights to check are Table 258's `/Form /FillIn` and `/Document /FullSave`; where
/// either is withheld, the signature stops applying to the file this program is about to
/// produce, and leaving it there would leave a statement about bytes nobody signed.
///
/// # What "remove that signature" is, in a file that may only be appended to
///
/// `CLAUDE.md` permits §7.5.6's incremental update and nothing else, so the signature *object*
/// stays in the file — the producer's bytes always do. What is removed is the permissions
/// dictionary's `/UR3` entry, which is the only thing that makes the object a grant: §12.8.6
/// defines a usage rights signature as the one "referred to from the UR3 entry in the
/// permissions dictionary", so a `/Perms` without that key refers to none. The object is
/// unreachable as a usage rights signature the moment the entry is, which is the same
/// construction §7.5.6 uses for a deletion.
///
/// `/Perms` is rewritten where the catalog states it indirectly and the catalog itself where it
/// does not, because an update replaces objects and a direct dictionary has no identity to
/// replace. That is the same distinction `interactive_form` draws for `/AcroForm`, and unlike
/// there it cannot fail: the catalog always has an object number.
///
/// # Measured, not assumed: the condition has no members in the corpus
///
/// Four of the 974 documents carry a `/UR3` — `160F-2019.pdf`, `issue6127.pdf`,
/// `prefilled_f1040.pdf` and `xfa_filled_imm1344e.pdf` — and **all four grant `/Form /FillIn`
/// and `/Document /FullSave`**, which is precisely what this program does. All four also come
/// out `/P false` — two say so and two leave it to Table 258's default — and that entry says
/// outright that "any possible restriction may be ignored", so the arrays are not even reached.
/// Nothing this program can do to a corpus document exceeds its rights, and this function
/// returns `None` for all four. It is written because the
/// clause is, not because a file asked — trap 11's discipline with the answer the other way up.
/// ADR 0159.
fn withdrawn_usage_rights(document: &Document) -> Option<(ObjectId, Dictionary)> {
    use crate::signature::Right;

    let rights = crate::signature::permissions(document).usage_rights?;
    if rights.grants(Right::FillInForm) && rights.grants(Right::FullSave) {
        return None;
    }
    let catalog = document.catalog().ok()?;
    if let Some(id) = catalog.get("Perms").and_then(Object::as_reference) {
        let mut perms = document.get(id).as_dict().cloned()?;
        perms.remove("UR3");
        return Some((id, perms));
    }
    let id = document.trailer().get("Root")?.as_reference()?;
    let mut catalog = catalog;
    let mut perms = catalog.get("Perms")?.as_dict().cloned()?;
    perms.remove("UR3");
    catalog.insert(Name::new(&b"Perms"[..]), Object::Dictionary(perms));
    Some((id, catalog))
}

/// Whether Table 227's `ReadOnly` flag reaches this widget, through §12.7.4.1's inheritance.
///
/// > If set, an interactive PDF processor shall not allow a user to change the value of the
/// > field.
///
/// The `/Ff` walk is the one §12.7.4.1 describes and the bound is this module's own: a `/Parent`
/// chain in a hostile file can be a cycle, and a field nobody can reach the root of is refused
/// rather than followed for ever.
/// The part of a typed value §12.7.5.3's Table 231 bit 24 leaves room for, over one field's
/// widgets.
///
/// The shortest prefix any of them accepts, and the whole value where none of them constrains
/// it — `crate::appearance::accepted_prefix` answers `None` for a widget the flag does not bind.
/// §12.7.5.4's selection, turned into the `/V` and the `/I` the clause says it is.
///
/// `None` where the field is not one of §12.7.5.4's — an index into Table 234's `/Opt` names
/// nothing there, and Table 230's entry of the same name is a *button's* export values, so
/// resolving against it would write an export value where §12.7.5.2.3 wants an appearance-state
/// name. A caller that sends [`Entered::Chosen`] to a text field or a check box has made a mistake
/// this refuses rather than reinterprets.
///
/// The widget is any one of the field's: §12.7.4.1 makes `/Opt`, `/Ff` and the value the *field's*,
/// so every widget of it walks to the same ancestry.
fn chosen(document: &Document, widget: Option<ObjectId>, indices: &[usize]) -> Option<Entry> {
    let object = document.get(widget?);
    let annotation = object.as_dict()?;
    let field = crate::appearance::Field::read(document, annotation, FieldValue::Stored);
    if !matches!(
        field.kind,
        Some(crate::appearance::FieldKind::Choice { .. })
    ) {
        return None;
    }
    let options = crate::form::options(document, &field);
    let mut wanted: Vec<usize> = indices
        .iter()
        .copied()
        // An index past the end of `/Opt` names no option, and Table 234 makes the array the whole
        // list of them: "[i]f this entry is not present, no choices should be presented to the
        // user".
        .filter(|index| *index < options.len())
        .collect();
    wanted.sort_unstable();
    wanted.dedup();
    // Table 233 bit 22: "if clear, at most one item shall be selected".
    if field.flags & crate::appearance::FLAG_MULTI_SELECT == 0 {
        wanted.truncate(1);
    }
    let labels: Vec<Object> = wanted
        .iter()
        .filter_map(|index| options.get(*index))
        .map(|option| {
            Object::String(pdf_syntax::text_string::encode_text_string(&option.label).into())
        })
        .collect();
    Some(match labels.len() {
        // "The default value of V is null , indicating that no item is currently selected."
        0 => Entry {
            value: None,
            indices: None,
        },
        // "V is a text string representing the selected item, as given in the field dictionary's
        // Opt array."
        1 => Entry {
            value: labels.into_iter().next(),
            indices: Some(wanted),
        },
        // "If multiple items are selected, V is an array of such strings."
        _ => Entry {
            value: Some(Object::Array(labels)),
            indices: Some(wanted),
        },
    })
}

impl Entry {
    /// This entry as the statement about a value the appearance path reads.
    fn as_field_value(&self) -> FieldValue<'_> {
        FieldValue::Edited {
            value: self.value.as_ref(),
            indices: self.indices.as_deref(),
        }
    }
}

fn accepted(document: &Document, widgets: &[ObjectId], value: &str) -> String {
    let mut limit = value.len();
    for widget in widgets {
        let object = document.get(*widget);
        let Some(annotation) = object.as_dict() else {
            continue;
        };
        if let Some(prefix) = crate::appearance::accepted_prefix(document, annotation, value) {
            limit = limit.min(prefix);
        }
    }
    value.get(..limit).unwrap_or(value).to_owned()
}

fn is_read_only(document: &Document, widget: ObjectId) -> bool {
    let mut current = match document.get(widget).as_dict() {
        Some(dict) => dict.clone(),
        None => return false,
    };
    for _ in 0..MAX_FIELD_DEPTH {
        if let Some(flags) = document.get_key(&current, "Ff").as_integer() {
            return flags & 1 != 0;
        }
        let Some(parent) = document.get_key(&current, "Parent").as_dict().cloned() else {
            return false;
        };
        current = parent;
    }
    false
}

/// The field a point in default user space is on, with its fully qualified name.
///
/// **Any subtype**, which is what separates this from [`field_at`] and from
/// `crate::link::at`: §12.6.3's trigger events are Table 197's, and Table 197 belongs to every
/// annotation dictionary rather than to links or widgets.
///
/// The **last** match, because a page lists its annotations in painting order and the one on top
/// is the one under the pointer.
#[must_use]
pub fn annotation_at(
    document: &Document,
    page: &crate::Page,
    view: &ViewState,
    x: f32,
    y: f32,
) -> Option<ObjectId> {
    let (x, y) = (f64::from(x), f64::from(y));
    let annotations = document.get_key(&page.dict, "Annots");
    let annotations = annotations.as_array()?;
    let mut found = None;
    for annotation in annotations {
        let Some(id) = annotation.as_reference() else {
            continue;
        };
        let resolved = document.resolve(annotation);
        let Some(dict) = resolved.as_dict() else {
            continue;
        };
        // §12.5.3's Hidden, NoView and ReadOnly each say "interact with the user", and
        // `annotation::interacts` is those three bits and nothing else — an annotation that
        // renders no ink still has an activation region, which is what a link is.
        if !crate::annotation::interacts(document, dict, view.annotation(id)) {
            continue;
        }
        if rectangle_covers(document, dict, x, y) {
            found = Some(id);
        }
    }
    found
}

/// Whether Table 166's `/Rect` covers a point in default user space.
///
/// The rectangle "shall be two opposite corners" and states no order, so both are normalised
/// before the point is tested against them.
fn rectangle_covers(document: &Document, annotation: &Dictionary, x: f64, y: f64) -> bool {
    let rect = document.get_key(annotation, "Rect");
    let Some(rect) = rect.as_array() else {
        return false;
    };
    let mut corners = rect
        .iter()
        .filter_map(|value| document.resolve(value).as_number());
    let (Some(x0), Some(y0), Some(x1), Some(y1)) = (
        corners.next(),
        corners.next(),
        corners.next(),
        corners.next(),
    ) else {
        return false;
    };
    x >= x0.min(x1) && x <= x0.max(x1) && y >= y0.min(y1) && y <= y0.max(y1)
}

/// A saved document, and what this program declined to put in it.
///
/// Two values rather than bytes alone because the second is not an error and must not be silent:
/// a save that quietly dropped what a person typed would be their work lost without a word, and
/// this one drops exactly one thing on purpose. See [`ViewState::save`] for the clause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    /// The whole file: the document as it was opened, with §7.5.6's update appended.
    pub bytes: Vec<u8>,
    /// §12.7.4.2's qualified name of every field whose typed value was **not** stored.
    ///
    /// Table 231 bit 14's NOTE, quoted in full on [`ViewState::save`]. Empty for every document
    /// with no password field in it, which is every document in this project's corpus.
    pub withheld: Vec<String>,
}

/// §12.7.4.2's qualified name of the field a widget belongs to, or the widget's own object.
///
/// A save is not a hot path and this walks the field tree to invert it, which is the honest way
/// to answer a question the edit log does not hold: the log is keyed by *widget*, because
/// §12.7.4.1 shares one value among a field's widgets and a replay has to reach each of them.
/// The fallback names the object rather than saying nothing, because a note a person cannot act
/// on is worse than a number they can look up.
fn field_name_of(document: &Document, widget: ObjectId) -> String {
    widgets_by_field_name(document)
        .into_iter()
        .find(|(_, widgets)| widgets.contains(&widget))
        .map_or_else(
            || format!("the field of object {}", widget.number),
            |(name, _)| name,
        )
}

/// What a field says now, and whether saying it was Table 231 bit 14's substitution.
///
/// **One value carrying both, deliberately.** The flag is not a second reading of the field: it is
/// a statement about *this string*, and a caller that had to ask a separate question could get an
/// answer that disagreed with the characters it is holding. That is the defect ADR 0247's third
/// amendment closes — a value that is not the value, in a type that does not say so, with the
/// exception discoverable only by reading two doc comments and noticing they interact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShownValue {
    /// The characters, or the bullets standing in for them.
    ///
    /// `""` is a field with nothing in it. Whether the field has a text value *at all* is the
    /// `Option` around this struct, not a property of it: a button selects an appearance and a
    /// signature holds a dictionary, and neither answers with a [`ShownValue`].
    pub text: String,
    /// Whether [`Self::text`] is Table 231 bit 14's echo rather than the field's own characters.
    ///
    /// ISO 32000-2 §12.7.5.3, Table 231, bit position 14:
    ///
    /// > If set, the field is intended for entering a secure password that should not be echoed
    /// > visibly to the screen. Characters typed from the keyboard shall instead be echoed in some
    /// > unreadable form, such as asterisks or bullet characters.
    ///
    /// **What a host does with it**: it does not write [`Self::text`] back into its control. ADR
    /// 0201 has a host read a field's value back after every keystroke, because §12.7.5.3's
    /// `DoNotScroll` means the field can take less than was typed; doing that here would replace
    /// what a person typed with a row of bullets and send *those* as the next value. A native
    /// password control draws its own echo from the characters it holds, so there is nothing it
    /// needs this string for.
    pub obscured: bool,
}

/// What a form field is called: the name that identifies it, and the name to show a person.
///
/// Two names because the standard states two, for two different jobs. §12.7.4.2's fully
/// qualified name is the field's *identity* — it is what [`crate::view::ViewState::set_field`]
/// addresses and what §12.7.6.2 exports — and §14.9.3 makes the other one a `shall`:
///
/// > An alternative name may be specified for an interactive form field (see 12.7, "Forms")
/// > which, if present, shall be used in place of the actual field name when an interactive PDF
/// > processor identifies the field in a user-interface. This alternative name, if provided,
/// > shall be specified using the TU entry of the field dictionary.
///
/// A single string could not carry both: whichever meaning it took, the other would be lost at
/// the caller. This is why the two are handed over together rather than chosen here — the caller
/// knows whether it is addressing the field or naming it to a person, and this type does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldName {
    /// §12.7.4.2's fully qualified name, which identifies the field.
    pub qualified: String,
    /// Table 226's `/TU`, where the field states one.
    pub alternative: Option<String>,
}

impl FieldName {
    /// The name §14.9.3 says a user interface shall show, which is `/TU` where there is one.
    #[must_use]
    pub fn shown(&self) -> &str {
        self.alternative.as_deref().unwrap_or(&self.qualified)
    }
}

/// Table 226's `/TU` for the field a widget annotation belongs to.
///
/// **Not inheritable**, which decides where to look: Table 226 marks `/FT`, `/Ff`, `/V` and
/// `/DV` inheritable and does not mark this one, so it belongs to the *terminal field* and to no
/// ancestor of it. That field is the widget's own dictionary where §12.5.6.19's merge applies,
/// and its `/Parent` where the widget is a kid with no `/T` — which is the same distinction
/// `walk` makes one level up, from the other end.
pub(crate) fn alternative_name(document: &Document, widget: ObjectId) -> Option<String> {
    let mut node = widget;
    let mut seen = BTreeSet::new();
    for _ in 0..MAX_FIELD_DEPTH {
        if !seen.insert(node) {
            return None;
        }
        let resolved = document.get(node);
        let dict = resolved.as_dict()?;
        if matches!(document.get_key(dict, "T"), Object::String(_)) {
            let Object::String(bytes) = document.get_key(dict, "TU") else {
                return None;
            };
            return Some(pdf_syntax::text_string(&bytes));
        }
        node = document.get_key(dict, "Parent").as_reference()?;
    }
    None
}

/// The name of the form field at a point in default user space, where there is one.
#[must_use]
pub fn field_at(document: &Document, page: &crate::Page, x: f32, y: f32) -> Option<FieldName> {
    let mut names: BTreeMap<ObjectId, String> = BTreeMap::new();
    for (name, widgets) in widgets_by_field_name(document) {
        for widget in widgets {
            names.insert(widget, name.clone());
        }
    }
    let mut found = None;
    for id in widgets_at(document, page, x, y) {
        found = names
            .get(&id)
            .map(|qualified| FieldName {
                qualified: qualified.clone(),
                alternative: alternative_name(document, id),
            })
            .or(found);
    }
    found
}

/// Every widget annotation on the page whose `/Rect` covers a point in default user space.
///
/// In `/Annots` order, which is the order §12.5.2 has them drawn in — so the last one is the one
/// on top where two overlap, and a caller wanting one takes the last. Split out of [`field_at`]
/// because a caret needs the widget itself rather than the name of the field behind it, and two
/// walks would be two hit tests that could disagree.
fn widgets_at(document: &Document, page: &crate::Page, x: f32, y: f32) -> Vec<ObjectId> {
    let (x, y) = (f64::from(x), f64::from(y));
    let annotations = document.get_key(&page.dict, "Annots");
    let Some(annotations) = annotations.as_array() else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for annotation in annotations {
        let Some(id) = annotation.as_reference() else {
            continue;
        };
        let resolved = document.resolve(annotation);
        let Some(dict) = resolved.as_dict() else {
            continue;
        };
        if document
            .get_key(dict, "Subtype")
            .as_name()
            .is_none_or(|subtype| subtype.as_bytes() != b"Widget")
        {
            continue;
        }
        let rect = document.get_key(dict, "Rect");
        let Some(rect) = rect.as_array() else {
            continue;
        };
        let mut corners = rect
            .iter()
            .filter_map(|value| document.resolve(value).as_number());
        let (Some(x0), Some(y0), Some(x1), Some(y1)) = (
            corners.next(),
            corners.next(),
            corners.next(),
            corners.next(),
        ) else {
            continue;
        };
        // Table 166's rectangle is two opposite corners and states no order, so both are
        // normalised before the point is tested against them.
        if x >= x0.min(x1) && x <= x0.max(x1) && y >= y0.min(y1) && y <= y0.max(y1) {
            found.push(id);
        }
    }
    found
}

/// Writes the constructed appearance of an annotation a person added into its `/AP`.
///
/// ADR 0130's argument, one clause over: this program can produce the bytes, so a file that
/// carried the annotation without them would be asking the next reader to do work this one has
/// already done — and a reader that constructs nothing would show the page unmarked. The bytes
/// are the ones this program draws, so writing them states no new opinion about the file.
///
/// **`/BBox` is the annotation's own `/Rect`**, and that is not a coincidence to be tidied away.
/// §12.5.6.10's `/QuadPoints` is "in default user space", so the marks `crate::appearance`
/// constructs are already in the page's coordinates; §12.5.5's algorithm maps a form's `/BBox`
/// onto its `/Rect`, and giving it the same rectangle twice makes that map the identity. Any
/// other box would move the marks off the words they are over.
///
/// Silent where the construction produces nothing — there is no such markup, since
/// `ViewState::add_markup` refuses an empty set of quadrilaterals and Table 166's `/C` is always
/// written — and deliberately so: an annotation with no `/AP` is legal, and every reader this
/// project compares against constructs one.
fn write_added_appearance(document: &Document, update: &mut Update, dict: &mut Dictionary) {
    let Some(subtype) = document
        .get_key(dict, "Subtype")
        .as_name()
        .map(|name| name.as_bytes().to_vec())
    else {
        return;
    };
    let Some(rect) = crate::annotation::rectangle(document, dict, "Rect") else {
        return;
    };
    let built = crate::appearance::construct(document, dict, &subtype, FieldValue::Stored, rect);
    let Some(content) = built.content else {
        return;
    };
    let mut stream = Dictionary::new();
    stream.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"XObject"[..])),
    );
    stream.insert(
        Name::new(&b"Subtype"[..]),
        Object::Name(Name::new(&b"Form"[..])),
    );
    stream.insert(
        Name::new(&b"BBox"[..]),
        Object::Array(
            rect.iter()
                .map(|edge| Object::Real(f64::from(*edge)))
                .collect(),
        ),
    );
    stream.insert(
        Name::new(&b"Resources"[..]),
        Object::Dictionary(built.resources),
    );
    stream.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(content.len()).unwrap_or(i64::MAX)),
    );
    let id = update.allocate();
    update.put(
        id,
        Object::Stream(std::sync::Arc::new(pdf_syntax::Stream {
            dict: stream,
            data: content.into(),
            decryption_failed: false,
        })),
    );
    let mut appearances = Dictionary::new();
    appearances.insert(Name::new(&b"N"[..]), Object::Reference(id));
    dict.insert(Name::new(&b"AP"[..]), Object::Dictionary(appearances));
}

#[cfg(test)]
mod tests {
    use super::{ViewState, widgets_by_field_name};
    use crate::action::read;
    use crate::optional_content::{ListMode, OptionalContent, Presented};
    use pdf_syntax::{Document, Object, ObjectId};

    fn document(objects: &[&str]) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let _ = write!(
            out,
            "xref\n0 {}\n0000000000 65535 f \n",
            objects.len().saturating_add(1)
        );
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len().saturating_add(1)
        );
        Document::open(out.into_bytes()).expect("a valid file")
    }

    fn id(number: u32) -> ObjectId {
        ObjectId {
            number,
            generation: 0,
        }
    }

    /// §12.6.4.13 turns a layer off, and the next render of the page hides its content.
    #[test]
    fn a_set_ocg_state_action_turns_a_layer_off() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [4 0 R] /D << >> >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /S /SetOCGState /State [/OFF 4 0 R] >>",
            "<< /Type /OCG /Name (layer) >>",
        ]);
        let mut state = ViewState::of(&doc);
        assert_eq!(
            state.optional_content().and_then(|oc| oc.state(id(4))),
            Some(true),
            "§8.11.4.5's default BaseState is ON"
        );

        let actions = read(&doc, &Object::Reference(id(3)));
        assert!(
            state.perform_all(&doc, &actions).is_empty(),
            "a layer change asks the viewer for nothing"
        );
        assert_eq!(
            state.optional_content().and_then(|oc| oc.state(id(4))),
            Some(false)
        );
    }

    /// Table 217's `/PreserveRB`, default true: turning one group on turns its siblings off.
    ///
    /// Table 99 states the paradigm — "the state of at most one optional content group in
    /// each array shall be ON at a time. If one group is turned ON, all others shall be
    /// turned OFF" — and Table 217's default is what makes it apply without the file asking.
    #[test]
    fn turning_a_radio_button_group_on_turns_its_siblings_off() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [4 0 R 5 0 R] \
             /D << /BaseState /OFF /ON [4 0 R] /RBGroups [[4 0 R 5 0 R]] >> >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /S /SetOCGState /State [/ON 5 0 R] >>",
            "<< /Type /OCG /Name (first) >>",
            "<< /Type /OCG /Name (second) >>",
        ]);
        let mut state = ViewState::of(&doc);
        let both = |state: &ViewState| {
            (
                state.optional_content().and_then(|oc| oc.state(id(4))),
                state.optional_content().and_then(|oc| oc.state(id(5))),
            )
        };
        assert_eq!(both(&state), (Some(true), Some(false)));

        state.perform_all(&doc, &read(&doc, &Object::Reference(id(3))));
        assert_eq!(
            both(&state),
            (Some(false), Some(true)),
            "the collection admits one"
        );
    }

    /// `/PreserveRB false` leaves the siblings alone, which is the entry's whole purpose.
    #[test]
    fn preserve_rb_false_ignores_the_collections() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [4 0 R 5 0 R] \
             /D << /BaseState /OFF /ON [4 0 R] /RBGroups [[4 0 R 5 0 R]] >> >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /S /SetOCGState /State [/ON 5 0 R] /PreserveRB false >>",
            "<< /Type /OCG /Name (first) >>",
            "<< /Type /OCG /Name (second) >>",
        ]);
        let mut state = ViewState::of(&doc);
        state.perform_all(&doc, &read(&doc, &Object::Reference(id(3))));
        assert_eq!(
            (
                state.optional_content().and_then(|oc| oc.state(id(4))),
                state.optional_content().and_then(|oc| oc.state(id(5))),
            ),
            (Some(true), Some(true)),
            "both on, which the collection would forbid and this entry permits"
        );
    }

    /// §12.7.4.2's own example, built as a field tree and looked up by a hide action.
    ///
    /// > If a field with the partial field name PersonalData has a child whose partial name
    /// > is Address, which in turn has a child with the partial name ZipCode, the fully
    /// > qualified name of this last field is PersonalData.Address.ZipCode
    #[test]
    fn a_hide_action_finds_a_field_by_its_fully_qualified_name() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /S /Hide /T (PersonalData.Address.ZipCode) >>",
            "<< /T (PersonalData) /Kids [5 0 R] >>",
            "<< /T (Address) /Parent 4 0 R /Kids [6 0 R] >>",
            "<< /T (ZipCode) /Parent 5 0 R /FT /Tx >>",
        ]);
        let table = widgets_by_field_name(&doc);
        assert_eq!(
            table.get("PersonalData.Address.ZipCode").map(Vec::as_slice),
            Some([id(6)].as_slice())
        );

        let mut state = ViewState::of(&doc);
        assert_eq!(state.annotation_hidden(id(6)), None);
        state.perform_all(&doc, &read(&doc, &Object::Reference(id(3))));
        assert_eq!(state.annotation_hidden(id(6)), Some(true));
    }

    /// A kid with no `/T` is a widget of its parent's field, not a field of its own.
    ///
    /// §12.7.4.2: "A field dictionary that does not have a partial field name (T entry) of its
    /// own shall not be considered a field but simply a Widget annotation." So one name
    /// reaches two annotations, and hiding it hides both — which is what Table 214 means by
    /// "whose associated widget annotation or annotations are to be affected".
    #[test]
    fn one_field_name_may_reach_several_widgets() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /S /Hide /T (Signature) /H true >>",
            "<< /T (Signature) /FT /Sig /Kids [5 0 R 6 0 R] >>",
            "<< /Parent 4 0 R /Subtype /Widget >>",
            "<< /Parent 4 0 R /Subtype /Widget >>",
        ]);
        let mut state = ViewState::of(&doc);
        state.perform_all(&doc, &read(&doc, &Object::Reference(id(3))));
        assert_eq!(state.annotation_hidden(id(5)), Some(true));
        assert_eq!(state.annotation_hidden(id(6)), Some(true));
    }

    /// Table 99's `/Locked` stops a panel and not §12.6.4.13's action.
    ///
    /// The two halves of §8.11.4.5's interactive paragraph, and the clause draws the line
    /// itself: a locked group "cannot be changed through the user interface", and a processor
    /// "may allow the states … to be changed by means other than the user interface".
    #[test]
    fn a_locked_group_refuses_a_panel_and_not_an_action() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [4 0 R] \
             /D << /Locked [4 0 R] >> >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /S /SetOCGState /State [/OFF 4 0 R] >>",
            "<< /Type /OCG /Name (layer) >>",
        ]);
        let mut state = ViewState::of(&doc);
        assert!(
            state
                .optional_content()
                .is_some_and(|oc| oc.is_locked(id(4)))
        );
        assert!(!state.set_group(id(4), false), "a panel is refused");
        assert_eq!(
            state.optional_content().and_then(|oc| oc.state(id(4))),
            Some(true)
        );

        state.perform_all(&doc, &read(&doc, &Object::Reference(id(3))));
        assert_eq!(
            state.optional_content().and_then(|oc| oc.state(id(4))),
            Some(false),
            "the action is not bound by the lock"
        );
    }

    /// A panel honours Table 99's radio-button collections without being asked to.
    ///
    /// `/PreserveRB` is a *set-OCG-state action's* entry; Table 99 states the paradigm
    /// unconditionally, so a panel that let two members of one collection be on at once would
    /// be showing a state the configuration says cannot exist.
    #[test]
    fn a_panel_keeps_a_radio_button_collection_to_one() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [3 0 R 4 0 R] \
             /D << /BaseState /OFF /ON [3 0 R] /RBGroups [[3 0 R 4 0 R]] >> >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Type /OCG /Name (first) >>",
            "<< /Type /OCG /Name (second) >>",
        ]);
        let mut state = ViewState::of(&doc);
        assert!(state.set_group(id(4), true));
        assert_eq!(
            (
                state.optional_content().and_then(|oc| oc.state(id(3))),
                state.optional_content().and_then(|oc| oc.state(id(4))),
            ),
            (Some(false), Some(true))
        );
        assert!(!state.set_group(id(4), true), "already on: nothing changed");
    }

    /// §8.11.4.3's EXAMPLE 1 and EXAMPLE 2, which are the two shapes `/Order` has.
    ///
    /// EXAMPLE 1 labels its nested arrays — `[(Frog Anatomy) 1 0 R 2 0 R]` — and the clause says
    /// those labels "shall be used to present collections of related optional content groups,
    /// and not to communicate actual nesting". EXAMPLE 2 nests without a label, which is what
    /// "actual nesting of groups in the content, such as for layers with sublayers" looks like.
    /// A panel that drew both the same way would tell a person that a heading is a layer, which
    /// is why the label is an `Option` and not a `String`.
    #[test]
    fn the_order_array_keeps_a_label_apart_from_a_nesting() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [3 0 R 4 0 R 5 0 R] \
             /D << /Order [[(Frog Anatomy) 3 0 R 4 0 R] 5 0 R [3 0 R]] >> >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Type /OCG /Name (Skin) >>",
            "<< /Type /OCG /Name (Bones) >>",
            "<< /Type /OCG /Name (Layer 1) >>",
        ]);
        let state = ViewState::of(&doc);
        let content = state.optional_content().expect("a configuration");
        assert_eq!(
            content.presentation(),
            [
                Presented::Collection {
                    label: Some("Frog Anatomy".to_owned()),
                    children: vec![Presented::Group(id(3)), Presented::Group(id(4))],
                },
                Presented::Group(id(5)),
                Presented::Collection {
                    label: None,
                    children: vec![Presented::Group(id(3))],
                },
            ]
        );
        assert_eq!(content.name(&doc, id(5)).as_deref(), Some("Layer 1"));
        assert_eq!(content.list_mode(), ListMode::AllPages);
    }

    /// A group `/Order` names that the document never declared governs nothing and is not shown.
    ///
    /// §8.11.3.2 makes membership of `/OCGs` the test for whether content is optional content at
    /// all, so a switch for a group outside it would do nothing at all when a person moved it.
    #[test]
    fn an_undeclared_group_is_not_presented() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [3 0 R] \
             /D << /Order [3 0 R 4 0 R] >> >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /Type /OCG /Name (declared) >>",
            "<< /Type /OCG /Name (not declared) >>",
        ]);
        let state = ViewState::of(&doc);
        assert_eq!(
            state.optional_content().map(OptionalContent::presentation),
            Some([Presented::Group(id(3))].as_slice())
        );
    }

    /// A cycle in `/Kids` terminates rather than exhausting the stack.
    ///
    /// `/Kids` is a reference the document controls and §12.7.4.1 states no acyclicity rule,
    /// so a field pointing back at its own ancestor is a file a reader has to survive. The
    /// assertion that matters is that this returns at all — without the `seen` set it
    /// recurses until the stack ends — and the leaf beside the cycle is still found, which is
    /// what says the guard skips the loop rather than the subtree.
    #[test]
    fn a_cycle_in_the_field_tree_terminates() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [3 0 R] >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /T (a) /Kids [4 0 R] >>",
            "<< /T (b) /Kids [3 0 R 5 0 R] >>",
            "<< /T (c) /FT /Tx >>",
        ]);
        let table = widgets_by_field_name(&doc);
        assert_eq!(
            table.get("a.b.c").map(Vec::as_slice),
            Some([id(5)].as_slice())
        );
        assert!(
            !table.keys().any(|name| name.matches('a').count() > 1),
            "the cycle produced no repeated name: {:?}",
            table.keys().collect::<Vec<_>>()
        );
    }

    /// §12.7.6.3's three shapes of `/Fields`, over one field tree.
    ///
    /// Table 241 makes the absent entry decisive — "all fields in the document's interactive
    /// form are reset" — Table 242's clear flag names what to reset "[a]ll descendants … as
    /// well", and its set flag names what to spare. The tree here has a named subtree so that
    /// the descendant rule is what passes rather than an exact-name match.
    #[test]
    fn a_reset_form_action_names_fields_to_reset_or_fields_to_spare() {
        let doc = document(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [7 0 R 9 0 R] >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "<< /S /ResetForm >>",
            "<< /S /ResetForm /Fields [(PersonalData)] >>",
            "<< /S /ResetForm /Fields [(PersonalData)] /Flags 1 >>",
            "<< /S /ResetForm /Fields [8 0 R] >>",
            "<< /T (PersonalData) /Kids [8 0 R] >>",
            "<< /T (ZipCode) /Parent 7 0 R /FT /Tx /Subtype /Widget >>",
            "<< /T (Signature) /FT /Sig /Subtype /Widget >>",
        ]);

        let all = |action: u32| {
            let mut state = ViewState::of(&doc);
            state.perform_all(&doc, &read(&doc, &Object::Reference(id(action))));
            (state.is_reset(id(8)), state.is_reset(id(9)))
        };

        assert_eq!(
            all(3),
            (true, true),
            "no /Fields resets every field there is"
        );
        assert_eq!(
            all(4),
            (true, false),
            "naming the parent resets its descendants and nothing else"
        );
        assert_eq!(
            all(5),
            (false, true),
            "the Include/Exclude flag turns the same array into what to spare"
        );
        assert_eq!(
            all(6),
            (true, false),
            "a field named by reference rather than by name"
        );
    }
}
