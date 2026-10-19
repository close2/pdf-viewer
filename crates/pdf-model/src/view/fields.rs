//! Form fields as a person fills them: typing a value, choosing a file, reading what a field
//! shows, the caret and selection inside it, clearing, and the form data an import brings.
//!
//! The walk from a field's name to its widgets is `field_tree`'s; this module is what is done with
//! a field once it is found.

use std::collections::{BTreeMap, BTreeSet};

use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId};

use super::field_tree::{Omitted, field_table, widgets_by_field_name};
use super::pages::NamedPageAppearances;
use super::{
    ChosenFile, Entered, Entry, FieldGlyph, FieldName, FieldValue, Imported, MAX_FIELD_DEPTH,
    Property, ShownValue, ViewState, scripts,
};

impl ViewState {
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
    /// Nothing is written to the file here: the import is a statement in this log, and
    /// [`Self::save`] is where it reaches §7.5.6's update with the edits beside it (ADR 1661).
    pub fn import(&mut self, document: &Document, data: &crate::forms_data::FormsData) -> Imported {
        let table = widgets_by_field_name(document);
        let (matched, unmatched) = crate::forms_data::match_to_document(data, &table);
        let mut outcome = Imported {
            widgets: matched.len(),
            unmatched,
            ..Imported::default()
        };
        let mut named = NamedPageAppearances::default();
        let mut awaiting = Vec::new();
        // An import is the later statement about every field it replaces, after a reset and after
        // what a person typed alike (ADR 1796).
        let replaced: BTreeSet<ObjectId> = matched.iter().map(|(widget, _)| *widget).collect();
        self.forget_statements(&table, &replaced);
        for (widget, import) in &matched {
            let mut import = import.clone();
            named.resolve(document, *widget, &mut import, &mut outcome, &mut awaiting);
            self.imported.insert(*widget, import);
        }
        self.awaiting.append(&mut awaiting);
        self.append_templates(document, data, &table, &mut outcome);
        self.place_annotations(document, data, &mut outcome);
        // An import changes values, and Table 224's `/CO` recalculates on any change.
        self.recalculate(document, &widgets_by_field_name(document));
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
    /// [`Entered::Cleared`] clears the field, which is not the same as never having touched it:
    /// the first shows nothing and the second shows Table 226's `/V`.
    ///
    /// Returns how many widgets took the value. Zero means the document has no field of that
    /// name — a caller's mistake rather than a document's — that every widget of it is Table 227's
    /// `ReadOnly`, which is the document refusing, that [`Entered::Chosen`] named §12.7.5.4's
    /// options on a field that is not one of §12.7.5.4's, which is a caller's mistake again, or
    /// that the field's keystroke script refused the characters or the selection — Table 199's
    /// `/K` "may check the added text for validity and reject or modify it", which is the document
    /// refusing again, one character at a time (ADR 1579).
    ///
    /// # Table 199's scripts, and the commit
    ///
    /// A text value is *typed*: `/K` judges it here, the field shows it as typed, and its format,
    /// its validation and the keystroke's commit form wait for [`Self::commit_field`]. A choice is
    /// judged by `/K` too, since the entry is performed when the user "modifies the selection in a
    /// scrollable list box": the change handed to it is the option the selection names, and a
    /// script that rewrites the change to another option's text selects that option (ADR 1786).
    /// Every value
    /// this takes is a value change, so Table 224's `/CO` is walked after it, which is what keeps a
    /// total following its lines while they are typed (ADR 1579).
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
    ///
    /// and its next sentence, a parenthesis, says which of a two-element `/Opt` entry that string
    /// is: the second. **It is stated here in prose rather than quoted**, because the published
    /// wording is *the name string is the second of the two array elements* and Errata Collection
    /// 3's Issue #214 replaces the term *name string* with *string* wherever ISO 32000-2:2020
    /// prints it. The erratum is one `Text` note and a handful of illustrative strikes on Table
    /// 32, so nothing is struck here for `spec-errata check` to land on and `doc/md/` carries the
    /// published wording; the meaning is the same under either printing, and this tree quotes
    /// neither.
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
    /// §12.8.2.2's `/DocMDP` and §7.6.4.2's Table 22 restrict this operation, and neither is
    /// checked here. It is not a question this function can answer: `CLAUDE.md` makes how much of a
    /// document's restrictions a reader obeys the *reader's* policy, with four levels of it, and
    /// two of those levels have to describe the operation to a person before it happens. A refusal
    /// expressed as a count of widgets can become none of that. So [`crate::restriction::asserted`]
    /// states what the document asserts — with its clause and its level — and the host that has the
    /// policy decides, once per operation, before calling this. ADR 0212.
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
        // A script's `readonly` is the document changing its own flag, and it stands over the
        // file's either way (ADR 1603).
        let read_only = self.script_read_only(name);
        let taking: Vec<ObjectId> = widgets
            .iter()
            .copied()
            .filter(|widget| !read_only.unwrap_or_else(|| is_read_only(document, *widget)))
            .collect();
        // Table 199's `/K`, which "may check the added text for validity and reject or modify it":
        // a keystroke script judges the characters before any widget takes them, and a refusal
        // leaves the field as it was (ADR 1579). The entry's trigger is also when the user
        // "modifies the selection in a scrollable list box", so a choice is judged the same way,
        // its change being the option the selection names (ADR 1786).
        let rewritten;
        let value = match value {
            Entered::Text(text) => match self.keystroke_verdict(document, &taking, text) {
                scripts::Verdict::Stands => value,
                scripts::Verdict::Rejected => return 0,
                scripts::Verdict::Rewritten(text) => {
                    rewritten = Entered::Text(text);
                    &rewritten
                }
            },
            Entered::Chosen(indices) => match self.selection_verdict(document, &taking, indices) {
                scripts::Selected::Stands => value,
                scripts::Selected::Rejected => return 0,
                scripts::Selected::Chooses(option) => {
                    rewritten = Entered::Chosen(vec![option]);
                    &rewritten
                }
            },
            Entered::Cleared => value,
        };
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
            Entered::Chosen(indices) => {
                let scripted = taking
                    .first()
                    .map_or(&[][..], |widget| self.scripted(*widget));
                match chosen(document, taking.first().copied(), scripted, indices) {
                    Some(entry) => entry,
                    None => return 0,
                }
            }
        };
        // §12.7.5.3's file-select control keeps the contents beside the pathname, so a value set
        // any other way takes them with it: a pathname and bytes that no longer belong to it
        // would be the wrong file submitted under the right name.
        self.chosen.remove(name);
        // Characters are typed and later committed; a choice or a clear is whole when it is made.
        if matches!(value, Entered::Text(_)) {
            self.begin_typing(name, &taking);
        } else {
            self.uncommitted.remove(name);
        }
        let mut applied = 0_usize;
        for widget in &taking {
            // The four statements about a value answer one question, so a widget belongs to
            // exactly one of them: what a person typed is the latest of the four.
            self.reset.remove(widget);
            self.imported.remove(widget);
            self.edited.insert(*widget, entry.clone());
            applied = applied.saturating_add(1);
        }
        // Table 224's `/CO` orders the recalculation "when the value of any field changes".
        if applied > 0 {
            self.recalculate(document, &table);
        }
        applied
    }

    /// Fills §12.7.5.3's file-select control with a file a person chose.
    ///
    /// Table 231 bit 21 says what such a field is:
    ///
    /// > If the FileSelect flag ( PDF 1.4 ) is set, the field shall function as a file-select
    /// > control. In this case, the field's text represents the pathname of a file whose contents
    /// > shall be submitted as the field's value
    ///
    /// — so this does both halves at once, which is why it is one method rather than
    /// [`Self::set_field`] and a second call: the value becomes the pathname, laid out and drawn
    /// like any other text, and [`ChosenFile::bytes`] are kept beside it for
    /// `crate::submission::compose` to put in §12.7.6.2's body. Setting the value any other way
    /// afterwards drops the contents, so the two cannot disagree.
    ///
    /// **The bytes come from outside and that is the point.** `CLAUDE.md` principle 3 gives this
    /// process no filesystem, and a *document* naming a path would be a document asking to read
    /// an arbitrary file — which `crate::submission` refuses by name and still does. What this
    /// takes is a file a **person** chose, read by whoever ran the chooser, which is the same
    /// division `viewer_host::policy::read_import` draws one clause over (ADR 1216).
    ///
    /// Returns how many widgets took the value, and **zero where the field is not a file-select
    /// control**: the flag is what makes a pathname a value at all, so a field without it takes
    /// nothing here rather than taking the pathname as ordinary text.
    pub fn choose_file(&mut self, document: &Document, name: &str, chosen: ChosenFile) -> usize {
        if !is_file_select(document, name) {
            return 0;
        }
        let applied = self.set_field(document, name, &Entered::Text(chosen.pathname.clone()));
        if applied == 0 {
            return 0;
        }
        self.chosen.insert(name.to_owned(), chosen);
        applied
    }

    /// The file a person chose for one field, or `None` where nobody chose one.
    ///
    /// Asked by `crate::submission::compose`, which is the one clause the contents are for.
    #[must_use]
    pub fn chosen_file(&self, name: &str) -> Option<&ChosenFile> {
        self.chosen.get(name)
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

    /// Where each character of a field's value is drawn, in **default user space**.
    ///
    /// The positions §12.7.4.3's layout gave the glyphs while it wrote the widget's appearance — the
    /// same walk [`Self::caret_at`] and [`Self::field_selection`] are answered from — so a
    /// platform's text interface can say where one character is without laying the value out a
    /// second time. One [`FieldGlyph`] per glyph, line by line and in the order each line is
    /// displayed: a right-to-left run comes left to right, as a page's own readback does (ADR 1501).
    ///
    /// `widget` names the widget annotation, which is what an accessibility node holds rather than
    /// a point. `None` for a widget whose field §12.7.4.3 lays no characters out for — a button, a
    /// list box, a signature — and for a value that could not be laid out at all.
    #[must_use]
    pub fn field_glyphs(&self, document: &Document, widget: ObjectId) -> Option<Vec<FieldGlyph>> {
        let object = document.get(widget);
        let dict = object.as_dict()?;
        let glyphs = crate::appearance::glyphs(document, dict, self.annotation(widget))?;
        Some(
            glyphs
                .into_iter()
                .map(|glyph| FieldGlyph {
                    line: glyph.line,
                    bytes: glyph.bytes,
                    quad: glyph.quad,
                })
                .collect(),
        )
    }

    /// Forgets what a person typed into one field, leaving whatever the file and the actions say.
    ///
    /// Deliberately not "set it back to the old value": the old value may have been the file's
    /// own, and re-stating that as an edit would make every later save carry a change nobody
    /// made. That distinction is the reason this exists and it is invisible until a document is
    /// *written*, which is why `saving.rs` is where it is asserted.
    ///
    /// **This is not what this tree's undo uses.** `viewer_core::Open` makes undo a *replay* rather
    /// than an inverse — the log is cleared with [`Self::clear_all_fields`] and re-applied up to
    /// the cursor — so forgetting one field is what a replay does by not reaching that entry, and a
    /// second route to the same state is not what a host is missing. What is left here is the
    /// per-field operation itself, for a caller that wants it without a log.
    pub fn clear_field(&mut self, document: &Document, name: &str) -> usize {
        let table = widgets_by_field_name(document);
        let Some(widgets) = table.get(name) else {
            return 0;
        };
        for widget in widgets {
            self.edited.remove(widget);
        }
        self.uncommitted.remove(name);
        self.recalculate(document, &table);
        widgets.len()
    }

    /// Forgets every value a person typed, leaving the file's own and whatever actions did.
    ///
    /// What a replay of the edit log starts from: an undo re-applies the log's surviving prefix
    /// rather than inverting its last entry, so the state it applies to has to be the one before
    /// any of it. See `viewer-core`'s `Open::replay` for why replaying beats inverting.
    pub fn clear_all_fields(&mut self) {
        self.edited.clear();
        self.uncommitted.clear();
        // §12.7.5.3's contents go with the pathnames they belong to: a replay that put one back
        // without the other would submit a file nobody chose.
        self.chosen.clear();
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
}

/// The field a widget belongs to: the nearest dictionary of its `/Parent` chain, itself first, that
/// states §12.7.4.2's `/T` — "[a] field dictionary that does not have a partial field name ( T
/// entry) of its own shall not be considered a field but simply a Widget annotation" — or the
/// widget itself where none does, for `holder`'s bound.
pub(super) fn named_field(document: &Document, widget: ObjectId) -> ObjectId {
    let mut id = widget;
    for _ in 0..MAX_FIELD_DEPTH {
        let object = document.get(id);
        let Some(current) = object.as_dict() else {
            return widget;
        };
        if current.get("T").is_some() {
            return id;
        }
        let Some(parent) = current.get("Parent").and_then(Object::as_reference) else {
            return widget;
        };
        id = parent;
    }
    widget
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
/// so every widget of it walks to the same ancestry. `scripted` is what scripts set on it, read so
/// that an index names an option of the list a script rewrote (ADR 1737).
pub(super) fn chosen(
    document: &Document,
    widget: Option<ObjectId>,
    scripted: &[Property],
    indices: &[usize],
) -> Option<Entry> {
    let object = document.get(widget?);
    let stored = object.as_dict()?;
    let annotation = crate::appearance::with_rewritten_options(document, stored, scripted);
    let field = crate::appearance::Field::read(document, &annotation, FieldValue::Stored);
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

pub(super) fn accepted(document: &Document, widgets: &[ObjectId], value: &str) -> String {
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

/// Whether one field is §12.7.5.3's file-select control — Table 231 bit 21, inherited.
///
/// The flag belongs to the *field* and §12.7.4.2 makes a field a name, so any of its widgets
/// answers for all of them: `/Ff` is inheritable, which is why this walks `/Parent` the way
/// [`is_read_only`] does. A name the document has no field for is not one.
///
/// Public because [`ViewState::choose_file`] takes nothing for a field without the flag, and a
/// caller owes a person a sentence rather than a silent nothing (trap 5) — `viewer_core` asks
/// this before it logs the edit.
#[must_use]
pub fn is_file_select(document: &Document, name: &str) -> bool {
    let table = widgets_by_field_name(document);
    let Some(widget) = table.get(name).and_then(|widgets| widgets.first()) else {
        return false;
    };
    let mut current = match document.get(*widget).as_dict() {
        Some(dict) => dict.clone(),
        None => return false,
    };
    for _ in 0..MAX_FIELD_DEPTH {
        if let Some(flags) = document.get_key(&current, "Ff").as_integer() {
            return flags & crate::appearance::FLAG_FILE_SELECT != 0;
        }
        let Some(parent) = document.get_key(&current, "Parent").as_dict().cloned() else {
            return false;
        };
        current = parent;
    }
    false
}

/// Whether a widget belongs to one of §12.7.5.2's two toggling buttons: Table 226's `/FT` `/Btn`,
/// inherited, without Table 229 bit 17's `Pushbutton`, which "holds no value".
pub(super) fn toggles(document: &Document, widget: &Dictionary) -> bool {
    let mut current = widget.clone();
    let (mut kind, mut flags) = (None, None);
    for _ in 0..MAX_FIELD_DEPTH {
        if kind.is_none() {
            kind = document
                .get_key(&current, "FT")
                .as_name()
                .map(|name| name.as_bytes() == b"Btn");
        }
        if flags.is_none() {
            flags = document.get_key(&current, "Ff").as_integer();
        }
        let Some(parent) = document.get_key(&current, "Parent").as_dict().cloned() else {
            break;
        };
        current = parent;
    }
    kind == Some(true) && flags.unwrap_or(0) & (1 << 16) == 0
}

/// A toggling button's value as §12.7.5.2.3's name: the state name the edit carries as text.
pub(super) fn state_name(value: &Object) -> Object {
    match value {
        Object::String(bytes) => Object::Name(Name::new(
            pdf_syntax::text_string::text_string(bytes).as_bytes(),
        )),
        other => other.clone(),
    }
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

/// §12.7.4.2's qualified name of the field a widget belongs to, or the widget's own object.
///
/// A save is not a hot path and this walks the field tree to invert it, which is the honest way
/// to answer a question the edit log does not hold: the log is keyed by *widget*, because
/// §12.7.4.1 shares one value among a field's widgets and a replay has to reach each of them.
/// The fallback names the object rather than saying nothing, because a note a person cannot act
/// on is worse than a number they can look up.
pub(super) fn field_name_of(document: &Document, widget: ObjectId) -> String {
    widgets_by_field_name(document)
        .into_iter()
        .find(|(_, widgets)| widgets.contains(&widget))
        .map_or_else(
            || format!("the field of object {}", widget.number),
            |(name, _)| name,
        )
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
    // The pointer is over this page, so this page's omitted roots are the ones it can reach.
    for (name, widgets) in field_table(document, Omitted::OnPage(&page.dict)) {
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
pub(super) fn widgets_at(document: &Document, page: &crate::Page, x: f32, y: f32) -> Vec<ObjectId> {
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
