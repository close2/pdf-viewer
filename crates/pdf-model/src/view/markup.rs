//! Annotations a person places or an import supplies: markup, free text, note text, and the
//! annotations an FDF file carries to the pages Table 254's `/Page` names.
//!
//! Every placement goes into the one log of [`Added`] entries that `writing` saves, so that what is
//! drawn, what is saved and what is undone are the same list.

use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId};

use super::annotations::rectangle_covers;
use super::fields::widgets_at;
use super::{Added, Imported, Markup, ViewState};

impl ViewState {
    /// Places §12.7.8.3.4's annotations on the pages Table 254's `/Page` names.
    ///
    /// The clause is one sentence and it is the whole requirement:
    ///
    /// > Each annotation dictionary in an FDF file shall have a Page entry (see "Table 254
    /// > -Additional entry for annotation dictionaries in an FDF file") that shall indicate the
    /// > page of the source document to which the annotation is attached.
    ///
    /// Table 254 spells the ordinal: "[t]he ordinal page number on which this annotation shall
    /// appear, where page 0 is the first page". Everything else in such a dictionary is §12.5's —
    /// an FDF annotation is an annotation — so once it names nothing of the other file there is
    /// no second reading to do and nothing outside ISO 32000-2 to read it by. ADR 1224.
    ///
    /// **They go into the same log a person's own annotations go into**, which is what makes them
    /// drawn, saved and undone by the code that already does those three: `Self::added` is
    /// §12.5.5's "previously painted annotations" for the page, `Self::write_additions` writes
    /// each into §7.5.6's update with its `/P`, and a replay clears and re-applies the import
    /// like any other log entry.
    ///
    /// **Table 246 excludes six of Table 171's subtypes from an FDF file** — "[t]he array may
    /// include annotations of any of the standard types listed in "Table 171 - Annotation types"
    /// except Link, Movie, Widget, PrinterMark, Screen, and TrapNet" — and one that names an
    /// excluded subtype is refused by name rather than placed, because a file that wrote one has
    /// broken a rule this reader would otherwise act on.
    #[expect(
        clippy::doc_markdown,
        reason = "a verbatim quotation: Table 246 spells the subtype names without backticks"
    )]
    pub(super) fn place_annotations(
        &mut self,
        document: &Document,
        data: &crate::forms_data::FormsData,
        outcome: &mut Imported,
    ) {
        // Table 246's `/EmbeddedFDFs` are files, so their annotations are this import's too, in
        // `FormsData::files`' order (ADR 1185). Each file's `popup` and `parent` positions are
        // positions in that file's own list, so the files are placed one at a time.
        let files = data.files();
        if files.iter().all(|file| file.annotations.is_empty()) {
            return;
        }
        let pages = crate::Pages::new(document);
        for file in files {
            self.place_file_annotations(document, &pages, &file.annotations, outcome);
        }
    }

    /// Places one file's annotations, and writes the links between them as references.
    ///
    /// Three passes, because an annotation may name one later in the list: which are placed and on
    /// which page; which popups lose their parent to a refusal and are refused with it; and only
    /// then the writing, when every placed annotation has the object it will be saved under.
    /// Table 172's `/Popup` and Table 186's `/Parent` become references to those objects, and
    /// Table 172's `/IRT` becomes one to the annotation its text string names (ADR 1297).
    fn place_file_annotations(
        &mut self,
        document: &Document,
        pages: &crate::Pages<'_>,
        annotations: &[crate::forms_data::FdfAnnotation],
        outcome: &mut Imported,
    ) {
        let mut target: Vec<Option<ObjectId>> = vec![None; annotations.len()];
        for (index, annotation) in annotations.iter().enumerate() {
            // A popup the file carried beside its parent states the parent's page where it
            // states none of its own, which `forms_data` has already filled in.
            let (Some(ordinal), Some(_)) = (annotation.page, annotation.dictionary.as_ref()) else {
                // Both absences are already named on `FormsData::owed`, which is the list a host
                // prints; saying them twice would double every line a person reads.
                continue;
            };
            if let Some(subtype) = annotation.subtype.as_deref()
                && EXCLUDED_FDF_SUBTYPES.contains(&subtype)
            {
                outcome.refused.push(format!(
                    "an FDF annotation of subtype {subtype}, which Table 246 excludes from an \
                     FDF file"
                ));
                continue;
            }
            let Some(page) = pages.get(ordinal).and_then(|page| page.id) else {
                outcome.refused.push(format!(
                    "an FDF annotation names page {ordinal}, which this document has not got"
                ));
                continue;
            };
            target[index] = Some(page);
        }
        // A popup exists to show its parent's text (§12.5.6.14), so one whose parent was refused
        // has nothing to show and is refused with it rather than left as a window of nobody's.
        for (index, annotation) in annotations.iter().enumerate() {
            if target[index].is_some()
                && annotation
                    .parent
                    .is_some_and(|parent| target.get(parent).copied().flatten().is_none())
            {
                target[index] = None;
                outcome.refused.push(
                    "an FDF popup annotation whose parent annotation was not placed".to_owned(),
                );
            }
        }
        let placed: Vec<Option<(ObjectId, ObjectId)>> = target
            .iter()
            .map(|page| page.map(|page| (self.allocate(document), page)))
            .collect();
        let link = |at: Option<usize>| at.and_then(|at| placed.get(at).copied().flatten());
        for (index, annotation) in annotations.iter().enumerate() {
            let (Some((id, page)), Some(dict)) = (placed[index], annotation.dictionary.as_ref())
            else {
                continue;
            };
            let mut dict = dict.clone();
            // Table 254's own entry is the FDF file's way of spelling Table 166's `/P`, and the
            // save writes that one: carrying the ordinal into the document would leave two
            // statements about which page this is on, one of them in a key §12.5.2 does not
            // define for an annotation in a PDF file.
            dict.remove("Page");
            if let Some((popup, _)) = link(annotation.popup) {
                dict.insert(Name::new(&b"Popup"[..]), Object::Reference(popup));
            }
            if let Some((parent, _)) = link(annotation.parent) {
                dict.insert(Name::new(&b"Parent"[..]), Object::Reference(parent));
            }
            self.link_reply(document, annotations, &placed, page, &mut dict, outcome);
            self.added.push(Added { id, page, dict });
            outcome.annotations = outcome.annotations.saturating_add(1);
        }
    }

    /// Table 172's `/IRT`, turned from the FDF file's text string into the reference a PDF file
    /// states.
    ///
    /// The table gives an FDF file its own spelling of the entry — "its type shall not be a
    /// dictionary but a text string containing the contents of the NM entry of the annotation
    /// being replied to" — and a PDF file the other: "[a] reference to the annotation that this
    /// annotation is "in reply to." Both annotations shall be on the same page of the document."
    /// So the name is looked for among what this import places on the same page, then among what
    /// was added to that page before it, then among the page's own `/Annots`. One found nowhere
    /// on the page is refused by name, and `/RT` with it, which the same table makes meaningful
    /// only where `/IRT` is present.
    fn link_reply(
        &self,
        document: &Document,
        annotations: &[crate::forms_data::FdfAnnotation],
        placed: &[Option<(ObjectId, ObjectId)>],
        page: ObjectId,
        dict: &mut Dictionary,
        outcome: &mut Imported,
    ) {
        let Some(Object::String(name)) = dict.get("IRT").cloned() else {
            return;
        };
        let named = |candidate: &Dictionary| {
            candidate
                .get("NM")
                .and_then(Object::as_string)
                .is_some_and(|stated| {
                    pdf_syntax::text_string(stated) == pdf_syntax::text_string(&name)
                })
        };
        let in_import = annotations.iter().zip(placed).find_map(|(other, at)| {
            let (id, on) = (*at)?;
            (on == page && other.dictionary.as_ref().is_some_and(named)).then_some(id)
        });
        let added_before = || {
            self.added
                .iter()
                .find(|added| added.page == page && named(&added.dict))
                .map(|added| added.id)
        };
        let on_the_page = || {
            let stated = document.get(page);
            let annots = document.get_key(stated.as_dict()?, "Annots");
            annots.as_array()?.iter().find_map(|entry| {
                let id = entry.as_reference()?;
                document
                    .resolve(entry)
                    .as_dict()
                    .is_some_and(named)
                    .then_some(id)
            })
        };
        if let Some(id) = in_import.or_else(added_before).or_else(on_the_page) {
            dict.insert(Name::new(&b"IRT"[..]), Object::Reference(id));
            return;
        }
        dict.remove("IRT");
        dict.remove("RT");
        outcome.refused.push(format!(
            "an imported annotation replies to one named {:?}, which this page holds nowhere, \
             so Table 172's /IRT and /RT are left out",
            pdf_syntax::text_string(&name)
        ));
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
        let id = self.allocate(document);
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
    /// [`super::writing::Update::state_default_font`].
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
        let id = self.allocate(document);
        self.added.push(Added { id, page, dict });
        self.set_free_text(document, id, text);
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
    /// Returns whether anything took the text: `false` for an object that is not a free text
    /// annotation of this document at all, which is a caller's mistake rather than a document's.
    ///
    /// # The file's own annotation, and why replacing one is the same kind of writing
    ///
    /// **Two annotations reach here and only one of them is this state's own.** An annotation
    /// [`ViewState::add_free_text`] created is a dictionary in the log, and its text is written
    /// into that dictionary. An annotation the *file* states is the producer's object, and what is
    /// recorded is a replacement of its `/Contents` — read back by
    /// [`ViewState::annotation`], drawn by [`crate::annotation::decide`], and written by
    /// [`ViewState::save`] as §7.5.6's "this object now reads like this". Nothing is written to the
    /// file until then and the document itself never changes, which is `CLAUDE.md`'s rule 1.
    ///
    /// Replacing an object the producer wrote is §7.5.6's own second case — the clause's list is
    /// "objects that have been changed, replaced, or deleted" — and the producer's bytes survive
    /// it byte for byte. ADR 0304.
    ///
    /// # What Table 167 says about it, and what it does not
    ///
    /// Bit 10, `LockedContents`, is the flag that forbids this and it is the *document's*
    /// assertion rather than this crate's verdict — so it is read by
    /// [`crate::restriction::asserted`], where a host can apply `CLAUDE.md`'s four levels to it,
    /// and **not** here. What is here is the check that this object is a free text annotation,
    /// which is a fact rather than a permission.
    ///
    /// Bit 8, `Locked`, is deliberately not the same flag and its own row says so: "do not allow
    /// the annotation to be deleted or its properties (including position and size) to be modified
    /// by the user. However, this flag does not restrict changes to the annotation's contents".
    pub fn set_free_text(&mut self, document: &Document, annotation: ObjectId, text: &str) -> bool {
        if let Some(added) = self.added.iter_mut().find(|added| added.id == annotation) {
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
            return true;
        }
        let Some(dict) = document.get(annotation).as_dict().cloned() else {
            return false;
        };
        if document
            .get_key(&dict, "Subtype")
            .as_name()
            .is_none_or(|subtype| subtype.as_bytes() != b"FreeText")
        {
            return false;
        }
        self.retyped.insert(annotation, text.to_owned());
        true
    }

    /// Retypes the text of §12.5.6.4's text annotation the file states — the note its popup
    /// window shows — the way a person typing into that window does; `false` where the
    /// annotation is not one whose own `/Contents` the window shows.
    ///
    /// > When closed, the annotation shall appear as an icon; when open, it shall display a popup
    /// > window containing the text of the note
    ///
    /// So the text is the window's and the icon is the appearance, and **the appearance stays
    /// the producer's**: unlike [`Self::set_free_text`], nothing drawn on the page depends on the
    /// text, and a save writes Table 166's `/Contents` alone. The retyping is held where a free
    /// text annotation's is, read back by the realm and the window alike, and written by §7.5.6's
    /// update (ADR 1721).
    ///
    /// **A subordinate of §12.5.6.2's group is refused**, because its `Contents` is one of the
    /// group attributes whose "corresponding entries in the subordinate annotations shall be
    /// ignored": a retyping the window could never show is not an edit a person could make.
    /// Table 167's `LockedContents` is the document's assertion and is read by
    /// [`crate::restriction::asserted`], as for a free text annotation, not here.
    pub fn set_note_text(&mut self, document: &Document, annotation: ObjectId, text: &str) -> bool {
        let Some(dict) = document.get(annotation).as_dict().cloned() else {
            return false;
        };
        if !is_text_note(document, &dict) {
            return false;
        }
        // `group_source` borrows the annotation's own dictionary unless a group's primary stands
        // in for it.
        if matches!(
            crate::markup::group_source(document, &dict),
            std::borrow::Cow::Owned(_)
        ) {
            return false;
        }
        self.retyped.insert(annotation, text.to_owned());
        true
    }

    /// What a person retyped into an annotation's Table 166 `/Contents`, where they retyped it:
    /// what a popup window showing that annotation's text shows in its place.
    #[must_use]
    pub(crate) fn note_text(&self, annotation: ObjectId) -> Option<&str> {
        self.retyped.get(&annotation).map(String::as_str)
    }

    /// The free text annotation a person added at a point, and what it says now.
    ///
    /// The point is in **default user space**, as every other question here takes it, and the
    /// answer is the **last** annotation covering it — §12.5.2 draws them in the order they were
    /// added and the one on top is the one under the pointer, which is the rule
    /// [`crate::view::annotation_at`] applies to the page's own array.
    ///
    /// **The page's own annotations as well as the ones this state added**, and the added ones win
    /// where both cover the point: §12.5.2 draws an addition after the page's `/Annots`, so the
    /// note a person just drew is the one on top of the note the producer wrote.
    ///
    /// The text is what the annotation says *now* — a retyping where there is one, and Table 166's
    /// `/Contents` otherwise — because that is what a host puts a caret into.
    #[must_use]
    pub fn free_text_at(
        &self,
        document: &Document,
        page: &crate::Page,
        x: f32,
        y: f32,
    ) -> Option<(ObjectId, String)> {
        let (id, dict) = self.free_text_dict_at(document, page, x, y)?;
        let text = match self.retyped.get(&id) {
            Some(retyped) => retyped.clone(),
            None => {
                crate::variable_text::string(document, &[&dict], "Contents").unwrap_or_default()
            }
        };
        Some((id, text))
    }

    /// [`Self::free_text_at`] with the dictionary rather than the text, for the caret's three
    /// questions.
    fn free_text_dict_at(
        &self,
        document: &Document,
        page: &crate::Page,
        x: f32,
        y: f32,
    ) -> Option<(ObjectId, Dictionary)> {
        let added = self
            .added_on(page.id)
            .filter(|added| is_free_text(document, &added.dict))
            .filter(|added| rectangle_covers(document, &added.dict, f64::from(x), f64::from(y)))
            .last()
            .map(|added| (added.id, added.dict.clone()));
        if added.is_some() {
            return added;
        }
        // The file's own, and the **last** covering the point for §12.5.2's reason again: within
        // one `/Annots` array the entry drawn last is the entry on top.
        let annotations = document.get_key(&page.dict, "Annots");
        let annotations = annotations.as_array()?;
        let mut found = None;
        for entry in annotations {
            let Some(id) = entry.as_reference() else {
                continue;
            };
            let resolved = document.resolve(entry);
            let Some(dict) = resolved.as_dict() else {
                continue;
            };
            // §12.5.3's Hidden, NoView and ReadOnly each say "interact with the user", and a
            // keyboard aimed at an annotation is interaction — so the same three bits that decide
            // whether a click reaches one decide whether it can be typed into.
            if !is_free_text(document, dict)
                || !crate::annotation::interacts(document, dict, self.annotation(id))
                || !rectangle_covers(document, dict, f64::from(x), f64::from(y))
            {
                continue;
            }
            found = Some((id, dict.clone()));
        }
        found
    }

    /// The thing at a point that a person can put a caret in, whatever kind it is.
    ///
    /// Two kinds, because §12.7.4.3 lays text out for two: a widget whose field states text, and
    /// §12.5.6.6's free text annotation, whose clause sends it to that same subclause. The added
    /// annotation is asked about **first**, because the interpreter draws what a person added
    /// after the page's own `/Annots` and the last thing drawn is the thing on top.
    pub(super) fn typeable_at(
        &self,
        document: &Document,
        page: &crate::Page,
        x: f32,
        y: f32,
    ) -> Option<(ObjectId, Dictionary)> {
        if let Some(found) = self.free_text_dict_at(document, page, x, y) {
            return Some(found);
        }
        // The last widget covering the point, which is the one drawn on top (§12.5.2's order) and
        // the one `field_at` names — so the caret and the name a host asked for are the same
        // field's.
        let widget = widgets_at(document, page, x, y).last().copied()?;
        let dict = document.get(widget).as_dict().cloned()?;
        Some((widget, dict))
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
    /// What a save writes — not whether there is anything to save. A host that keeps an undo
    /// log knows what is unwritten from the log's cursor and its last save, which is where
    /// `viewer_core::Open::dirty` answers it from; what this offers is the *contents* of the
    /// additions, for a caller that wants to show them rather than count them.
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
        self.release_numbers_if_nothing_is_held();
    }

    /// Forgets every retyping of a free text annotation the file states.
    ///
    /// The third of the three [`crate::view::ViewState`] clears a replay starts from, beside
    /// [`Self::clear_all_fields`] and [`Self::clear_all_additions`], and it is separate from the
    /// second for the reason the two maps are separate: an addition is an object that would stop
    /// existing, and this is a statement about one that exists either way.
    pub fn clear_all_free_text(&mut self) {
        self.retyped.clear();
    }
}

/// The resource name the `/DA` of a free text annotation this program creates uses.
///
/// **One of §12.7.4.3's own fourteen abbreviations**, and that is the whole argument for it:
/// `variable_text`'s `STANDARD_ABBREVIATIONS` is a bijection between these names and §9.6.2.2's
/// fourteen standard fonts, so a reader that has never heard of this program still knows what
/// `/Helv` denotes — and where the document's `/DR` already defines the name, that definition is
/// the document's own and wins, which is exactly what the clause asks.
pub(super) const FREE_TEXT_FONT: &str = "Helv";

/// What [`FREE_TEXT_FONT`] is defined as when the document defines nothing under that name.
pub(super) const FREE_TEXT_BASE_FONT: &str = "Helvetica";

/// The size the `/DA` of a free text annotation this program creates states, in points.
///
/// **A choice, and the standard states nothing about it**: §12.7.4.3 describes reading a `/DA`
/// and says only that "[a] zero value for size means that the font shall be auto-sized". Zero is
/// therefore available and is not what a person drawing a text box means — auto-sizing grows one
/// character until it fills whatever rectangle was dragged. Twelve points is a note.
const FREE_TEXT_SIZE: f32 = 12.0;

/// The six of Table 171's subtypes an FDF file's `/Annots` may not carry.
///
/// ISO 32000-2 §12.7.8.3.1, Table 246, on the entry:
///
/// > The array may include annotations of any of the standard types listed in "Table 171 -
/// > Annotation types" except Link, Movie, Widget, PrinterMark, Screen, and TrapNet .
///
/// A list of six rather than a condition, because the table states six. ADR 1224.
const EXCLUDED_FDF_SUBTYPES: [&str; 6] = [
    "Link",
    "Movie",
    "Widget",
    "PrinterMark",
    "Screen",
    "TrapNet",
];

/// Whether this dictionary is §12.5.6.6's annotation, which Table 177 makes `/Subtype /FreeText`.
pub(super) fn is_free_text(document: &Document, annotation: &Dictionary) -> bool {
    document
        .get_key(annotation, "Subtype")
        .as_name()
        .is_some_and(|subtype| subtype.as_bytes() == b"FreeText")
}

/// Whether an annotation is §12.5.6.4's text annotation.
pub(super) fn is_text_note(document: &Document, annotation: &Dictionary) -> bool {
    document
        .get_key(annotation, "Subtype")
        .as_name()
        .is_some_and(|subtype| subtype.as_bytes() == b"Text")
}
