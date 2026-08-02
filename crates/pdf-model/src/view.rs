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
    /// `None` against a widget is a value a person *cleared*, which is a different thing from
    /// a widget nobody has touched: the first shows an empty field and the second shows the
    /// file's own `/V`.
    edited: BTreeMap<ObjectId, Option<String>>,
    /// Which annotation the pointer is over or pressing, if any (§12.5.5).
    ///
    /// One annotation rather than a set, because a pointer is in one place. `None` is what
    /// every render before the seventy-sixth session assumed: nothing is interacting with the
    /// user, so every annotation shows its normal appearance.
    pointer: Option<(ObjectId, Pointer)>,
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
    /// statements made before somebody typed. `None` is a field a person *cleared*, which shows
    /// nothing — a different thing from [`Self::Stored`], which shows the file's own value.
    Edited(Option<&'a str>),
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
        }
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
                FieldValue::Edited(edit.as_deref())
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

    /// Sets the value of every widget of one field, the way a person typing into it does.
    ///
    /// §12.7.4.2 makes a field's identity its *fully qualified name*, and §12.7.4.1 lets one
    /// field own several widget annotations — "a field's value is shared by all of its widgets"
    /// is the practical consequence, and it is why this takes a name and applies to a set. The
    /// name table is the same §12.7.4.2 walk §12.6.4.11's hide action, §12.7.6.3's reset and
    /// §12.7.8's import all use, so all four agree about what a field is called.
    ///
    /// `None` clears the field, which is not the same as never having touched it: the first
    /// shows nothing and the second shows Table 226's `/V`.
    ///
    /// Returns how many widgets took the value. Zero means the document has no field of that
    /// name — a caller's mistake rather than a document's — or that every widget of it is
    /// Table 227's `ReadOnly`, which is the document refusing.
    ///
    /// **Nothing is written to the file.** `CLAUDE.md`'s rule 1 makes the document immutable;
    /// what a person did is a log beside it, and turning that log into §7.5.6's incremental
    /// update is a separate operation with its own clause.
    pub fn set_field(&mut self, document: &Document, name: &str, value: Option<&str>) -> usize {
        let table = widgets_by_field_name(document);
        let Some(widgets) = table.get(name) else {
            return 0;
        };
        let mut applied = 0_usize;
        for widget in widgets {
            // Table 227 bit 1: an interactive processor shall not allow a *user* to change the
            // value. A person is exactly who this refuses, which is what separates it from
            // §12.7.6.3's reset and §12.7.8's import — both of those are the *document* changing
            // its own value, and neither is a user.
            if is_read_only(document, *widget) {
                continue;
            }
            // The four statements about a value answer one question, so a widget belongs to
            // exactly one of them: what a person typed is the latest of the four.
            self.reset.remove(widget);
            self.imported.remove(widget);
            self.edited.insert(*widget, value.map(ToOwned::to_owned));
            applied = applied.saturating_add(1);
        }
        applied
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
    /// # Errors
    ///
    /// [`pdf_syntax::write::UpdateError`], which names every document this refuses: one whose
    /// cross-reference table was rebuilt by scanning, one whose own encryption cannot be applied
    /// to what is written, and one whose trailer is missing what §7.5.5 requires. An *encrypted*
    /// document is no longer among them: §7.6.2's ciphers run on the way out, so the `/V` this
    /// writes reaches the file in the form the document's own key expects.
    pub fn save(&self, document: &Document) -> Result<Vec<u8>, pdf_syntax::write::UpdateError> {
        let mut update = Update::beside(document);
        for (widget, value) in self.edits() {
            let Some(dict) = document.get(widget).as_dict().cloned() else {
                continue;
            };
            // §12.7.4.1's `/V` is inheritable, so the field that *holds* the value may be an
            // ancestor of the widget — and writing it onto the widget would leave the ancestor's
            // stale value inherited by the field's other widgets. The value goes where the
            // document already keeps one, or on the widget where the document keeps none.
            let (id, mut field) = holder(document, widget, dict.clone());
            match value {
                Some(text) => {
                    field.insert(
                        Name::new(&b"V"[..]),
                        Object::String(pdf_syntax::text_string::encode_text_string(text).into()),
                    );
                }
                // §12.7.6.3's own words for a value that is gone: "its V entry shall be removed".
                None => {
                    field.remove("V");
                }
            }
            update.put(id, Object::Dictionary(field));
            update.write_appearance(document, widget, &dict, FieldValue::Edited(value));
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
        pdf_syntax::write::incremental_update(document, &update.replacements)
    }

    /// Forgets every value a person typed, leaving the file's own and whatever actions did.
    ///
    /// What a replay of the edit log starts from: an undo re-applies the log's surviving prefix
    /// rather than inverting its last entry, so the state it applies to has to be the one before
    /// any of it. See `viewer-core`'s `Open::replay` for why replaying beats inverting.
    pub fn clear_all_fields(&mut self) {
        self.edited.clear();
    }

    /// Every field a person has typed into, by widget, in object order.
    ///
    /// What a save writes and what a host asks to know whether there is anything to save.
    pub fn edits(&self) -> impl Iterator<Item = (ObjectId, Option<&str>)> {
        self.edited
            .iter()
            .map(|(widget, value)| (*widget, value.as_deref()))
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
/// appearance §12.5.5 shows while a button is down, and Table 192's `/H` is the highlighting
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
#[must_use]
pub fn press_changes_appearance(document: &Document, annotation: ObjectId) -> bool {
    let object = document.get(annotation);
    let Some(dict) = object.as_dict() else {
        return false;
    };
    crate::annotation::press_changes(document, dict)
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

    /// Records what one object now says, replacing anything already recorded for it.
    fn put(&mut self, id: ObjectId, object: Object) {
        self.replacements.insert(id, object);
    }

    /// Whether this update writes nothing.
    fn is_empty(&self) -> bool {
        self.replacements.is_empty()
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

/// Whether Table 227's `ReadOnly` flag reaches this widget, through §12.7.4.1's inheritance.
///
/// > If set, an interactive PDF processor shall not allow a user to change the value of the
/// > field.
///
/// The `/Ff` walk is the one §12.7.4.1 describes and the bound is this module's own: a `/Parent`
/// chain in a hostile file can be a cycle, and a field nobody can reach the root of is refused
/// rather than followed for ever.
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

/// §12.7.4.2's fully qualified name of the form field at a point in default user space.
#[must_use]
pub fn field_at(document: &Document, page: &crate::Page, x: f32, y: f32) -> Option<String> {
    let (x, y) = (f64::from(x), f64::from(y));
    let annotations = document.get_key(&page.dict, "Annots");
    let annotations = annotations.as_array()?;
    let mut names: BTreeMap<ObjectId, String> = BTreeMap::new();
    for (name, widgets) in widgets_by_field_name(document) {
        for widget in widgets {
            names.insert(widget, name.clone());
        }
    }
    let mut found = None;
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
            found = names.get(&id).cloned().or(found);
        }
    }
    found
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
