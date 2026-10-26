//! The save: §7.5.6's incremental update built from the log a person's edits left, and the seven
//! `write_*` steps that feed it.
//!
//! The producer's bytes stay in the file. This module appends new objects and a new
//! cross-reference section, and reads the document it appends to without changing it.

use std::collections::BTreeMap;

use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId};

use super::fields::{field_name_of, named_field, state_name, toggles};
use super::markup::{FREE_TEXT_BASE_FONT, FREE_TEXT_FONT, is_free_text, is_text_note};
use super::{
    AnnotationView, Displayed, FieldValue, FilingHome, MAX_FIELD_DEPTH, Property, ViewState,
    Written,
};

impl ViewState {
    /// §7.5.6's incremental update for everything a person changed.
    ///
    /// The bytes of the *whole file*: the document as it was opened, unchanged, with an update
    /// appended. `CLAUDE.md` permits exactly this form of writing — the producer's bytes stay in
    /// the file, byte for byte, under what the person added — and the host writes them somewhere.
    ///
    /// # What is written
    ///
    /// One replacement object per field a value was typed into, carrying Table 226's `/V`, and
    /// §12.7.4.3's appearance stream for each of its widgets (`Update::write_appearance`). Every
    /// annotation a person added, with its page's `/Annots` extended
    /// (`ViewState::write_additions`); and every free text annotation of the file's **own** that
    /// a person retyped, with Table 166's `/Contents` and a replaced appearance
    /// (`ViewState::write_retypings`) — which is §7.5.6's second case, "objects that have been
    /// changed, replaced, or deleted", and the producer's own bytes are still in the file beneath
    /// it. Every field an import replaced and nobody typed into since, with the entries Table 249
    /// states and the appearance the value is drawn with (`ViewState::write_imported_values`);
    /// and every field §12.7.6.3's reset set back to a default its `/V` was not already
    /// (`ViewState::write_resets`).
    ///
    /// **Table 224's `/NeedAppearances` is written for what is left over, and the entry's own row
    /// states that condition** (§12.7.3):
    ///
    /// > A PDF writer shall include this key, with a value of true , if it has not provided
    /// > appearance streams for all visible widget annotations present in the document.
    ///
    /// So the flag goes in exactly where a widget's stream could not be produced, or could be
    /// produced only in part, and [`Written::unconstructed`] names every field that happened to:
    /// the entry says something is owed and the report says by whom. A document whose every
    /// changed widget got a stream is written without it, which is what the same row's NOTE
    /// expects of a file of this vintage — "Appearance streams are required in PDF 2.0 and later"
    /// — and the entry itself is deprecated in PDF 2.0.
    ///
    /// **Table 166's `/M` goes on every annotation this update writes, and only where a host has
    /// said what time it is.** The entry is "\[t]he date and time when the annotation was most
    /// recently modified"; `CLAUDE.md`'s rule 3 gives this crate no clock, and an instant invented
    /// here would be a claim about a machine nothing in this crate can see.
    /// [`ViewState::set_modification_time`] is where a host states one, and a host that states
    /// none writes the bytes this program wrote before it could be asked. ADR 1160.
    ///
    /// # What is deliberately **not** written
    ///
    /// A value typed into a Table 231 bit 14 password field, and [`Written::withheld`] names every
    /// field it happened to. §12.7.5.3's own words, under that bit:
    ///
    /// > NOTE To protect password confidentiality, it is imperative that PDF processors never
    /// > store the value of the text field in the PDF file if this flag is set.
    ///
    /// A NOTE is informative and this one is obeyed anyway, because the alternative is this program
    /// writing a person's password into a file in clear text (ADR 0247). **Neither half of the edit
    /// is written**, which is what keeps the file consistent with itself: the producer's `/V` and
    /// the producer's `/AP` stay as they were and go on agreeing, where writing the appearance
    /// without the value would leave a widget drawing something §12.7.2 says its `/V` should
    /// decide. What a person typed lives in this log until the document is closed and nowhere else.
    ///
    /// **A hybrid form's `datasets` packet, wherever a `/V` written above changed a field's
    /// value.** §K.2's first writer rule:
    ///
    /// > The XFA field values shall be consistent with the corresponding V entries of the PDF
    /// > field objects.
    ///
    /// So the packet's stream is replaced, with the data value each changed field binds holding
    /// what its `/V` now says and every other byte as the producer wrote it
    /// (`ViewState::write_xfa_data`, ADR 2396). [`Written::xfa_unkept`] names every changed field
    /// whose data value this could not find or write, with the reason, because the file then
    /// states two values for it.
    ///
    /// A §12.7.8.3.3 template page an import added, and [`Written::unwritten_pages`] counts them.
    /// The import shows the page ([`ViewState::appended_pages`]) and this update writes no page
    /// into the page tree, so the page is in the view and not in the file; the count is what keeps
    /// that from being silent (ADR 2154).
    ///
    /// # Errors
    ///
    /// [`pdf_syntax::write::UpdateError`], which names every document this refuses: one whose
    /// cross-reference table was rebuilt by scanning, one whose own encryption cannot be applied
    /// to what is written, and one whose trailer is missing what §7.5.5 requires. An *encrypted*
    /// document is no longer among them: §7.6.2's ciphers run on the way out, so the `/V` this
    /// writes reaches the file in the form the document's own key expects.
    pub fn save(&self, document: &Document) -> Result<Written, pdf_syntax::write::UpdateError> {
        let mut update = Update::beside(document, self.modified);
        let mut withheld = Vec::new();
        let (freed, still_reached) = self.write_filings(document, &mut update);
        let mut unappeared = self.write_additions(document, &mut update);
        self.write_imported_appearances(document, &mut update);
        unappeared.extend(self.write_retypings(document, &mut update));
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
            let toggling = toggles(document, &dict);
            match entered.value.as_ref() {
                // §12.7.5.2.3 and §12.7.5.2.4 make a check box's and a radio button's `/V` a *name*:
                // the edit carries the appearance-state name as text, and it is written as the
                // name it is.
                Some(object) if toggling => {
                    field.insert(Name::new(&b"V"[..]), state_name(object));
                }
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
            // Table 231 bit 26's `/RV`, beside the `/V` it specifies (ADR 1635).
            crate::rich_text::write_beside(document, &dict, entered.value.as_ref(), &mut field);
            // Table 234's `/I` is written for a single selection too, because it is the only
            // entry that says *which* `/Opt` element was chosen where two of them carry the same
            // label, and Errata Collection 3 opens the entry to choice fields at large rather than
            // to multiple-selection ones (Issue #468). `doc/md/` still prints the wording the
            // erratum struck, so the reading is ADR 0253's and `doc/errata-read.md`'s row for
            // Table 234. An edit that named no options takes it out: an `/I` left standing beside
            // a `/V` it does not describe is a file contradicting itself, and §12.7.5.4's
            // tie-break would then be arbitrating a disagreement this program wrote.
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
            update.write_appearance(
                document,
                widget,
                &dict,
                value,
                self.displayed(widget),
                self.scripted(widget),
            );
            if toggling {
                update.write_state(document, widget, entered.value.as_ref());
            }
            // Table 166's `/M` is the *annotation's*, so it goes on the widget rather than on
            // whichever ancestor §12.7.4.1 keeps the value on.
            update.stamp_annotation(document, widget);
        }
        self.write_resets(document, &mut update);
        self.write_imported_values(document, &mut update, &mut withheld);
        self.write_scripted(document, &mut update);
        self.write_scripted_annotations(document, &mut update);
        // After every `write_*` above, because each of them can write a `/V`, and §K.2's rule
        // is about the value the file ends up stating (ADR 2396).
        let xfa_unkept = Self::write_xfa_data(document, &mut update);
        // Table 224's flag is written only for what this program could not write itself. §12.7.2
        // states what setting it admits — "[i]f such an object defines an appearance stream, the
        // appearance shall be consistent with the object's current value as a field" — so a
        // document whose every changed widget got a new stream is one where that obligation is
        // *kept*, and asking the next reader to redo the work would be saying otherwise.
        if !update.owed.is_empty() {
            update.owe_appearances(document);
        }
        if !update.is_empty()
            && let Some((id, catalog)) = withdrawn_usage_rights(document, &update)
        {
            update.put(id, Object::Dictionary(catalog));
        }
        // Named by field rather than by widget, because that is what a person typed into: a
        // field §12.7.4.1 spreads over two widgets is one name and one sentence.
        let mut unconstructed: Vec<String> = update
            .owed
            .iter()
            .map(|widget| field_name_of(document, *widget))
            .collect();
        let bytes =
            pdf_syntax::write::incremental_update_freeing(document, &update.replacements, &freed)?;
        withheld.sort_unstable();
        withheld.dedup();
        unconstructed.sort_unstable();
        unconstructed.dedup();
        Ok(Written {
            bytes,
            withheld,
            unappeared,
            unconstructed,
            still_reached,
            unwritten_pages: self.appended.len(),
            xfa_unkept,
        })
    }

    /// Writes §K.2's consistency: the data value of every field whose value the update changed.
    ///
    /// A field changed is one whose inherited `/V` (§12.7.4.1) reads differently through the
    /// update than through the file, so whichever `write_*` step wrote it — a typed value, a
    /// reset, an import, a script — is kept, and a field nothing changed is not touched. The
    /// packet is read only where the document states an XFA resource and some field changed, so
    /// a save of any other document costs one catalog lookup.
    ///
    /// A form XFA 3.3 does not call XFAF — Table 29's `/NeedsRendering`, or page 333's test on
    /// its template (ADR 2380) — is the dynamic half's, which waits on Q639, and its fields are
    /// named rather than half-written (trap 5).
    fn write_xfa_data(document: &Document, update: &mut Update) -> Vec<(String, String)> {
        let Some(stated) = crate::xfa::stated(document).filter(|stated| stated.resource) else {
            return Vec::new();
        };
        let mut unkept = Vec::new();
        let mut changes = Vec::new();
        for (name, widgets) in super::widgets_by_field_name(document) {
            let Some(&widget) = widgets.first() else {
                continue;
            };
            let after = inherited_value(document, Some(update), widget);
            if inherited_value(document, None, widget) == after {
                continue;
            }
            match xfa_change(document, update, &widgets, &after) {
                Some(change) => changes.push((name, change)),
                None => unkept.push((name, "its /V is of a type no XFA field holds".to_owned())),
            }
        }
        if changes.is_empty() {
            return unkept;
        }
        let refuse_all = |reason: String, unkept: &mut Vec<(String, String)>| {
            unkept.extend(
                changes
                    .iter()
                    .map(|(name, _)| (name.clone(), reason.clone())),
            );
        };
        if stated.needs_rendering {
            refuse_all(
                "the document asks to be regenerated when opened (Table 29's /NeedsRendering), \
                 so its XFA form is a dynamic one"
                    .to_owned(),
                &mut unkept,
            );
            return unkept;
        }
        let resource = match crate::xfa::Resource::read(document) {
            Some(Ok(resource)) => resource,
            Some(Err(error)) => {
                refuse_all(error.to_string(), &mut unkept);
                return unkept;
            }
            None => return unkept,
        };
        if let Some(why) = resource.template.dynamic() {
            refuse_all(format!("its XFA form is a dynamic one: {why}"), &mut unkept);
            return unkept;
        }
        let kept = resource.datasets.keep(&resource.template, &changes);
        if let Some(text) = kept.text {
            let stream = replaced_stream(document, resource.datasets_stream, text.into_bytes());
            update.put(resource.datasets_stream, stream);
        }
        unkept.extend(kept.unkept);
        unkept
    }

    /// Writes §12.7.6.3's reset onto the fields it reached and nothing has replaced since.
    ///
    /// > Upon invocation of a reset-form action, an interactive PDF processor shall reset selected
    /// > interactive form fields to their default values; that is, it shall set the value of the V
    /// > entry in the field dictionary to that of the DV entry … If no default value is defined for
    /// > a field, its V entry shall be removed.
    ///
    /// The log holds the reset and a save is where the sentence reaches the file, so a document
    /// saved after one opens again showing the defaults the viewer showed (ADR 1796). The value goes
    /// where [`holder`] finds the document keeping one, with the appearance it is drawn with, as a
    /// typed value's does. A field whose `/V` already is its `/DV` is not written: the reset changed
    /// nothing there, and a save that rewrote every field of a form reset whole would be a file
    /// stating changes nobody made. Table 234's `/I` goes, as it goes for a typed value, because it
    /// describes the value the reset replaced.
    fn write_resets(&self, document: &Document, update: &mut Update) {
        for widget in &self.reset {
            let widget = *widget;
            // [`Self::forget_statements`] keeps the four statements disjoint; a later one is
            // written by its own pass.
            if self.edited.contains_key(&widget) || self.imported.contains_key(&widget) {
                continue;
            }
            let Some(dict) = document.get(widget).as_dict().cloned() else {
                continue;
            };
            let stored = crate::appearance::Field::read(document, &dict, FieldValue::Stored).value;
            let default =
                crate::appearance::Field::read(document, &dict, FieldValue::Default).value;
            if stored == default {
                continue;
            }
            let (id, mut field) = holder(document, widget, dict.clone());
            let toggling = toggles(document, &dict);
            match default.as_ref() {
                Some(object) if toggling => {
                    field.insert(Name::new(&b"V"[..]), state_name(object));
                }
                Some(object) => {
                    field.insert(Name::new(&b"V"[..]), object.clone());
                }
                None => {
                    field.remove("V");
                }
            }
            crate::rich_text::write_beside(document, &dict, default.as_ref(), &mut field);
            field.remove("I");
            update.put(id, Object::Dictionary(field));
            update.write_appearance(
                document,
                widget,
                &dict,
                FieldValue::Default,
                self.displayed(widget),
                self.scripted(widget),
            );
            if toggling {
                update.write_state(document, widget, default.as_ref());
            }
            update.stamp_annotation(document, widget);
        }
    }

    /// Writes §12.7.8's imported values onto the fields that took them: Table 249's `/V`, `/RV`,
    /// `/Ff` and `/F`, and the appearance the value is drawn with.
    ///
    /// §12.7.8.3.2's one sentence is the rule — "importing a field causes the values of the
    /// entries in the FDF field dictionary to replace those of the corresponding entries in the
    /// field with the same fully qualified name in the target document" — and a save is where the
    /// replacement reaches the file. The values are the FDF producer's, carried unreinterpreted
    /// under the keys that file named, which is `CLAUDE.md`'s provenance test passed the way
    /// [`Self::write_imported_appearances`] passes it; and a save that wrote the imported `/AP`
    /// and left the old `/V` beside it would write a widget whose value and appearance §12.7.2
    /// says must agree and do not (ADR 1661).
    ///
    /// The entries are the ones [`crate::forms_data::Import`] holds, each written where the
    /// file reads it: the value on the dictionary [`holder`] finds, as a typed value is; Table
    /// 249's `/RV` beside it where the FDF field states one, and the target's own standing where
    /// it states none (ADR 1648); `/Ff` on the field §12.7.4.2 names, as Table 249 says, "the
    /// form's corresponding field dictionary"; `/F` on the widget. Table 234's `/I` goes, as it
    /// goes for a typed value: an index left beside a value it does not describe is a file
    /// contradicting itself. A field a person typed into after the import is written by the
    /// edits above and not here — the later statement stands — and a password field's value is
    /// withheld by the same predicate.
    fn write_imported_values(
        &self,
        document: &Document,
        update: &mut Update,
        withheld: &mut Vec<String>,
    ) {
        for (widget, import) in &self.imported {
            let widget = *widget;
            if self.edited.contains_key(&widget) {
                continue;
            }
            let Some(dict) = document.get(widget).as_dict().cloned() else {
                continue;
            };
            let value = FieldValue::Imported {
                value: import.value.as_ref(),
                flags: import.field_flags,
                rich: import.rich_value.as_deref(),
            };
            if crate::appearance::field_text_value(document, &dict, value)
                .is_some_and(|shown| shown.obscured)
            {
                withheld.push(field_name_of(document, widget));
                continue;
            }
            let (id, _) = holder(document, widget, dict.clone());
            let Some(mut field) = update.current(document, id) else {
                continue;
            };
            let toggling = toggles(document, &dict);
            match import.value.as_ref() {
                Some(object) if toggling => {
                    field.insert(Name::new(&b"V"[..]), state_name(object));
                }
                Some(object) => {
                    field.insert(Name::new(&b"V"[..]), object.clone());
                }
                // ADR 0090: an FDF field stating no `/V` removes the value it replaces.
                None => {
                    field.remove("V");
                }
            }
            if let Some(rich) = import.rich_value.as_deref() {
                field.insert(
                    Name::new(&b"RV"[..]),
                    Object::String(pdf_syntax::text_string::encode_text_string(rich).into()),
                );
            }
            field.remove("I");
            update.put(id, Object::Dictionary(field));
            if !import.field_flags.is_unchanged() {
                let stated = crate::appearance::Field::read(document, &dict, FieldValue::Stored);
                let named = named_field(document, widget);
                if let Some(mut current) = update.current(document, named) {
                    current.insert(
                        Name::new(&b"Ff"[..]),
                        Object::Integer(import.field_flags.applied_to(stated.flags)),
                    );
                    update.put(named, Object::Dictionary(current));
                }
            }
            if !import.annotation_flags.is_unchanged()
                && let Some(mut current) = update.current(document, widget)
            {
                let stated = document.get_key(&current, "F").as_integer().unwrap_or(0);
                current.insert(
                    Name::new(&b"F"[..]),
                    Object::Integer(import.annotation_flags.applied_to(stated)),
                );
                update.put(widget, Object::Dictionary(current));
            }
            // An imported `/AP` is the FDF producer's own appearance and was written as it came;
            // one constructed here would replace it with this program's reading of the value.
            if import.appearance.is_none() {
                update.write_appearance(
                    document,
                    widget,
                    &dict,
                    value,
                    self.displayed(widget),
                    self.scripted(widget),
                );
            }
            if toggling {
                update.write_state(document, widget, import.value.as_ref());
            }
            update.stamp_annotation(document, widget);
        }
    }

    /// Writes what scripts set on each widget's field as the entries the standard draws a widget
    /// from: `/MK` and `/BS` on the widget, `/DA`, `/Q`, `/MaxLen` and `/Ff` on the field that
    /// holds them, and the appearance constructed from them (ADR 1617).
    ///
    /// **After the values**, and through `Update::current`, because a widget a person also typed
    /// into has had its field rewritten above, and this adds to that rewrite rather than replacing
    /// it. The field's entries go on the nearest dictionary of the widget's chain that states a
    /// `/T` — the field §12.7.4.2 names, which is the one the script named — so its other widgets
    /// inherit them as they inherited what they replace. A widget no one typed into gets its
    /// appearance written here; one a person typed into got it above, from the same properties.
    ///
    /// **A member a script set on one widget of several writes no field entry** (ADR 1664): `/DA`
    /// and `/Q` are the field's, and its other widgets would inherit the one widget's colour or
    /// alignment from them. That widget's saved appearance carries what was set.
    fn write_scripted(&self, document: &Document, update: &mut Update) {
        for (widget, properties) in &self.scripting.drawn {
            let widget = *widget;
            let Some(original) = document.get(widget).as_dict().cloned() else {
                continue;
            };
            let mut entries = crate::appearance::scripted_entries(document, &original, properties);
            let whole_field: Vec<Property> = properties
                .iter()
                .filter(|property| {
                    !self
                        .scripting
                        .one_widget
                        .contains(&(widget, property.member()))
                })
                .cloned()
                .collect();
            if whole_field.len() < properties.len() {
                entries.field =
                    crate::appearance::scripted_entries(document, &original, &whole_field).field;
            }
            if !entries.widget.is_empty()
                && let Some(mut current) = update.current(document, widget)
            {
                for (key, value) in entries.widget {
                    current.insert(key, value);
                }
                update.put(widget, Object::Dictionary(current));
            }
            if !entries.field.is_empty() {
                let field = named_field(document, widget);
                if let Some(mut current) = update.current(document, field) {
                    for (key, value) in entries.field {
                        current.insert(key, value);
                    }
                    update.put(field, Object::Dictionary(current));
                }
            }
            if !self.edited.contains_key(&widget) {
                let view = self.annotation(widget);
                update.write_appearance(
                    document,
                    widget,
                    &original,
                    view.value,
                    view.displayed,
                    view.scripted,
                );
            }
            update.stamp_annotation(document, widget);
        }
    }

    /// Writes every file a person attached and takes every detached entry out of the tree.
    ///
    /// Three objects per file attached to the document — §7.11.4's stream, §7.11.3's
    /// specification and the tree that names them — and two per file attached to a page, whose
    /// annotation [`Self::write_additions`] writes with the rest. The tree is rewritten once,
    /// holding the document's entries less the detached ones plus the attached ones, and the
    /// objects a detached entry alone reached are the answer's first half: §7.5.6, "\[d]eleted
    /// objects shall be left unchanged in the PDF file, but shall be marked as deleted by means
    /// of their cross-reference entries". The second half names every detached file whose stream
    /// another home still reaches, and which therefore stays in use.
    ///
    /// **First in the update, and through `Update::current`**, because the catalog is a holder
    /// this may rewrite and [`withdrawn_usage_rights`] may rewrite it after: two rewrites of one
    /// object compose only if the second reads the first.
    fn write_filings(
        &self,
        document: &Document,
        update: &mut Update,
    ) -> (Vec<ObjectId>, Vec<(String, String)>) {
        use crate::attachment::filing;

        let mut freed = Vec::new();
        let mut still_reached = Vec::new();
        for filed in &self.filed {
            update.reserve(filed.stream);
            update.reserve(filed.specification);
        }
        for filed in &self.filed {
            update.put(
                filed.stream,
                filing::embedded_file_stream(
                    &filed.filing.bytes,
                    filed.filing.media_type.as_deref(),
                    None,
                ),
            );
            update.put(
                filed.specification,
                filing::file_specification(
                    &filed.key,
                    filed.stream,
                    filed.filing.description.as_deref(),
                ),
            );
        }
        let tree_changes = self
            .filed
            .iter()
            .any(|filed| matches!(filed.filing.home, FilingHome::Document));
        if !tree_changes && self.unfiled.is_empty() {
            return (freed, still_reached);
        }
        let (Ok(catalog), Some(root_id)) = (
            document.catalog(),
            document
                .trailer()
                .get("Root")
                .and_then(Object::as_reference),
        ) else {
            // A document with no catalog or no `/Root` is one `incremental_update` refuses by
            // name, so nothing is lost by writing no tree for it.
            return (freed, still_reached);
        };
        let catalog = update.current(document, root_id).unwrap_or(catalog);
        let tree = filing::Tree::read(document, &catalog, &|id| {
            update.current_object(document, id)
        });
        let mut entries = tree.entries;
        for key in &self.unfiled {
            let Some(at) = entries.iter().position(|(existing, _)| existing == key) else {
                continue;
            };
            let (_, value) = entries.remove(at);
            match filing::freed_by_removing(document, &value) {
                Ok(objects) => freed.extend(objects),
                Err(homes) => still_reached.push((pdf_syntax::text_string(key), homes)),
            }
        }
        for filed in &self.filed {
            if matches!(filed.filing.home, FilingHome::Document) {
                entries.push((filed.key.clone(), Object::Reference(filed.specification)));
            }
        }
        let tree_id = update.allocate();
        update.put(tree_id, filing::tree_root(entries));
        filing::point_holder_at_tree(
            &mut update.replacements,
            tree_id,
            filing::Holder {
                catalog,
                root_id,
                names_entry: tree.names_entry,
                names_dict: tree.names_dict,
                tree_entry: tree.tree_entry,
            },
        );
        // The old root is what the holder rewrite reuses where the tree was indirect, so it is
        // never both replaced and freed.
        freed.retain(|id| !update.replacements.contains_key(id));
        (freed, still_reached)
    }

    /// Writes every free text annotation and text note of the file's own that a person retyped.
    ///
    /// Two entries of one object, and the second is what keeps the file consistent with itself.
    /// Table 166's `/Contents` is what the person typed; Table 166's `/AP` is **replaced** with the
    /// appearance this program draws for that text, because §12.5.6.6's Table 177 makes the file's
    /// own `/AP` decisive in its `/DA` row —
    ///
    /// > The annotation dictionary's AP entry, if present, shall take precedence over the DA entry
    ///
    /// — so a stream left as it was would be the file's own instruction to go on showing the
    /// producer's text under the person's `/Contents`. §12.5.6.6 sends this subtype's appearance to
    /// §12.7.4.3 to be *generated* from these entries, and this writes the generation.
    ///
    /// **Replaced rather than spliced, and that is measured rather than assumed.** §12.7.4.3's
    /// closing paragraph updates an existing stream by replacing "the existing contents of the
    /// appearance stream from /Tx BMC to the matching EMC", appending where there is no such
    /// region — which is right for a widget, whose stream is background and border with the text in
    /// a marked region, and wrong here: of the corpus's 67 free text annotations carrying a stream,
    /// **11 hold a marked-content region and 56 do not** (`examples/free_text_census`), so
    /// appending would draw the new text *on top of* the old for five annotations in six. The
    /// subtype's whole appearance is its text, so the whole appearance is what is regenerated.
    ///
    /// The stream dictionary is built afresh rather than kept, which is the other half of the same
    /// point: these marks are in the page's own default user space with `/BBox` the annotation's
    /// `/Rect` — [`write_added_appearance`]'s argument — and a `/Matrix` the producer wrote for
    /// their own marks would move them.
    ///
    /// Returns every annotation left without an appearance at all, which is
    /// [`Written::unappeared`]. That is not the same as an empty note: it is one whose text this
    /// program could not lay out, and the annotation is written with `/Contents` and no `/AP` so
    /// that the next reader constructs from Table 177's Required `/DA` rather than drawing text
    /// nothing in the file claims any more.
    ///
    /// **That is a departure from Table 166 and is taken deliberately.** The entry's own row binds
    /// a writer — "\[a] PDF writer shall include an appearance dictionary when writing or updating
    /// the PDF file except for the two cases listed below" — and a free text annotation is neither
    /// exception (its `/Rect` is not degenerate and its `/Subtype` is not `Popup`, `Projection` or
    /// `Link`); Errata Collection 3's Issue #22 raises the same requirement into the entry's
    /// column for PDF 2.0. The cost is an annotation the amended table calls non-conforming; the
    /// alternative is keeping a stream that draws words the file no longer states, which Table 177
    /// makes decisive over the `/DA` a reader would otherwise construct from. It is **reported**
    /// rather than silent, which is what separates the two: [`Written::unappeared`] names every
    /// annotation this happened to. §12.5.2's ledger row carries the argument.
    fn write_retypings(&self, document: &Document, update: &mut Update) -> Vec<ObjectId> {
        let mut unappeared = Vec::new();
        for (annotation, text) in &self.retyped {
            let Some(mut dict) = update.current(document, *annotation) else {
                continue;
            };
            let free_text = is_free_text(document, &dict);
            if !free_text && !is_text_note(document, &dict) {
                continue;
            }
            dict.insert(
                Name::new(&b"Contents"[..]),
                Object::String(pdf_syntax::text_string::encode_text_string(text).into()),
            );
            update.stamp(&mut dict);
            // A text note's appearance is its icon, which §12.5.6.4 draws whatever the note says,
            // so the producer's `/AP` stays (`ViewState::set_note_text`).
            if free_text && !write_retyped_appearance(document, update, &mut dict, text) {
                unappeared.push(*annotation);
            }
            update.put(*annotation, Object::Dictionary(dict));
        }
        unappeared
    }

    /// Writes §12.7.8's imported appearances onto the widgets that took them.
    ///
    /// §12.7.8.3.2: "importing a field causes the values of the entries in the FDF field
    /// dictionary to replace those of the corresponding entries in the field with the same fully
    /// qualified name in the target document". Table 249's `/AP` is an appearance dictionary "as
    /// shown in "Table 170 - Entries in an appearance dictionary"", and Table 170 is the widget's
    /// own `/AP` — so the corresponding entry is that one, and a save that left it alone would
    /// write a file that draws something different from what the reader was shown.
    ///
    /// **These are the FDF producer's own marks, which is why writing them is not authoring.**
    /// `CLAUDE.md`'s boundary asks whether the operation invents marks: every byte of a carried
    /// stream was written by whoever produced the FDF file, carried across unreinterpreted, and
    /// placed under the key that file named. Nothing here composes anything. ADR 1223.
    ///
    /// **The streams become objects here and not before.** A carried appearance holds its streams
    /// *directly*, because a direct object names nothing and that is the whole point of the copy;
    /// §7.3.8.1 requires a stream in a file to be an indirect object, so [`promote_streams`] gives
    /// each one a number in this update as it is written.
    fn write_imported_appearances(&self, document: &Document, update: &mut Update) {
        for (widget, import) in &self.imported {
            let Some(appearance) = import.appearance.as_ref() else {
                continue;
            };
            let Some(mut dict) = update.current(document, *widget) else {
                continue;
            };
            let placed = promote_streams(update, Object::Dictionary(appearance.clone()));
            dict.insert(Name::new(&b"AP"[..]), placed);
            update.stamp(&mut dict);
            update.put(*widget, Object::Dictionary(dict));
        }
    }

    /// Writes every annotation a person added, and attaches each to its page.
    ///
    /// Two objects per annotation, because §12.5.2 says where an annotation lives: "each page
    /// object shall contain an `/Annots` entry … an array of annotation dictionaries". So the
    /// annotation is written under the number it was given when it was added, and the *page* is
    /// rewritten with the reference appended — which is the half of §7.5.6's "changed, replaced,
    /// or deleted" that a new object needs, and the same half Table 224's widgets already use.
    ///
    /// **Appended rather than inserted**, because the array's order is the order this tree paints
    /// in (§12.5.5 states that annotations are painted one over another and no order in words, ADR
    /// 0245), and a mark a person made last belongs on top of what was there.
    ///
    /// Table 166's `/P` is added here rather than at [`ViewState::add_markup`]: it is "an
    /// indirect reference to the page object with which this annotation is associated", which is
    /// a statement about the *file* and so belongs to the writing rather than to the log.
    ///
    /// Returns every added annotation written with no appearance, which is only one whose text is
    /// set in a face from this machine whose licence forbids embedding it (ADR 1425) — the same
    /// shortfall [`Written::unappeared`] already names for a retyped note.
    fn write_additions(&self, document: &Document, update: &mut Update) -> Vec<ObjectId> {
        let mut unappeared = Vec::new();
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
            // Table 166's `/M`, for [`Update::stamp`]'s reason and at [`Self::write_retypings`]'s
            // site: an annotation a person made this viewing session was most recently modified
            // when the host says it was.
            update.stamp(&mut dict);
            // A carried annotation may hold a stream elsewhere than its `/AP` — Table 187's `/FS`
            // with its embedded file, Table 188's `/Sound` — and §7.3.8.1 makes every stream an
            // indirect object, so each becomes one before the dictionary is written.
            let carried: Vec<(Name, Object)> = dict
                .iter()
                .filter(|(key, _)| key.as_bytes() != b"AP")
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();
            for (key, value) in carried {
                dict.insert(key, promote_streams(update, value));
            }
            if !write_added_appearance(document, update, &mut dict) {
                unappeared.push(added.id);
            }
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
        unappeared
    }
}

/// The objects one save writes, and the object numbers it is allowed to invent.
///
/// §7.5.6's update may *add* objects as well as replace them — the clause's own list is "objects
/// that have been changed, replaced, or deleted" — and a widget with no `/AP` needs one added,
/// because §7.3.8.1 makes every stream an indirect object and there is nowhere else to put it.
pub(super) struct Update {
    /// What the update writes, by object.
    replacements: BTreeMap<ObjectId, Object>,
    /// The next object number nothing in the file uses.
    next: u32,
    /// Every widget whose appearance had to be left to the next reader, in object order.
    ///
    /// The condition Table 224's `/NeedAppearances` states, held by name rather than as a flag:
    /// the entry binds a writer that "has not provided appearance streams for all visible widget
    /// annotations present in the document", so the writer that sets it knows which widgets it
    /// means and a person is owed that list (ADR 1159).
    owed: Vec<ObjectId>,
    /// What a host says the time is, for Table 166's `/M`.
    ///
    /// `None` is a host that has said nothing, and nothing is then written: this crate has no
    /// clock (`CLAUDE.md` rule 3), and the entry is Optional. ADR 1160.
    modified: Option<pdf_syntax::Date>,
}

impl Update {
    /// An empty update beside a document, numbering its first new object where
    /// [`pdf_syntax::XrefTable::next_object_number`] says nothing in the file has stated one
    /// (ADR 2362).
    fn beside(document: &Document, modified: Option<pdf_syntax::Date>) -> Self {
        Self {
            replacements: BTreeMap::new(),
            next: document.xref().next_object_number(),
            owed: Vec::new(),
            modified,
        }
    }

    /// Writes Table 166's `/M` on one annotation dictionary, where a host has said what time it
    /// is.
    ///
    /// §12.5.2, the entry's own row:
    ///
    /// > The date and time when the annotation was most recently modified. The format should be a
    /// > date string as described in 7.9.4, "Dates" but interactive PDF processors shall accept
    /// > and display a string in any format.
    ///
    /// The `should` is met rather than taken advantage of: §7.9.4's string is what every reader
    /// can *order*, and the second half of the sentence is about what a reader accepts rather
    /// than about what a writer may invent. The date is written whole, in the fields the host
    /// stated it in.
    ///
    /// Nothing is written where no host has said anything, which is every caller of
    /// [`ViewState::of`] until one does. ADR 1160.
    pub(super) fn stamp(&self, dict: &mut Dictionary) {
        let Some(at) = self.modified else {
            return;
        };
        dict.insert(
            Name::new(&b"M"[..]),
            Object::String(crate::attachment::filing::pdf_date(at).into_bytes().into()),
        );
    }

    /// The same entry on an annotation this update is otherwise writing nothing of.
    ///
    /// A field's value lives on the *field*, which §12.7.4.1 lets be an ancestor of the widget,
    /// and a widget whose stored stream was replaced has its bytes rewritten in another object —
    /// so the annotation a person changed is not always one this update was going to write. Table
    /// 166 is the annotation dictionary's table, so the entry belongs on the widget in both
    /// cases, and the extra object is the price of saying so. It is paid only by a host that
    /// supplies an instant. ADR 1160.
    fn stamp_annotation(&mut self, document: &Document, annotation: ObjectId) {
        if self.modified.is_none() {
            return;
        }
        let Some(mut dict) = self.current(document, annotation) else {
            return;
        };
        self.stamp(&mut dict);
        self.put(annotation, Object::Dictionary(dict));
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
        let root = document
            .trailer()
            .get("Root")
            .and_then(Object::as_reference);
        // As the update has it: [`ViewState::write_filings`] may already have rewritten the
        // catalog, and a second rewrite composes only if it reads the first.
        let Some(catalog) = root
            .and_then(|id| self.current(document, id))
            .or_else(|| document.catalog().ok())
        else {
            return;
        };
        let held = |update: &Self, entry: Option<&Object>| match entry {
            Some(Object::Reference(id)) => (Some(*id), update.current(document, *id)),
            other => (None, other.and_then(Object::as_dict).cloned()),
        };
        let (form_id, form) = held(self, catalog.get("AcroForm"));
        let mut form = form.unwrap_or_else(|| {
            let mut form = Dictionary::default();
            // Table 224 makes the entry required: "(Required) An array of references to the
            // document's root fields (those with no ancestors in the field hierarchy)."
            // The parenthetical is what §12.7.4.2's row argues a recovery against.
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
        let Some(root) = root else {
            return;
        };
        let mut catalog = catalog;
        catalog.insert(Name::new(&b"AcroForm"[..]), Object::Dictionary(form));
        self.put(root, Object::Dictionary(catalog));
    }

    /// Sets Table 224's `/NeedAppearances` in the interactive form dictionary, wherever the catalog
    /// keeps it (ADR 1677).
    ///
    /// Table 29's `/AcroForm` row types the entry as a dictionary and states no indirectness, so a
    /// form held directly in the catalog is as much the document's form as one held by
    /// reference. An update replaces objects, and a direct dictionary has no number
    /// of its own, so it is rewritten inside the catalog that holds it — the distinction
    /// [`withdrawn_usage_rights`] draws for `/Perms`. Both levels are read through
    /// [`Self::current`], because [`Self::state_default_font`] and [`ViewState::write_filings`]
    /// may have rewritten either already, and two rewrites of one object compose only if the
    /// second reads the first. A document with no form dictionary is left without one: a flag
    /// about "all visible widget annotations" in a form that lists no field would describe a form
    /// the file does not have, and [`Written::unconstructed`] still names what is owed.
    fn owe_appearances(&mut self, document: &Document) {
        let Some(root) = document
            .trailer()
            .get("Root")
            .and_then(Object::as_reference)
        else {
            return;
        };
        let Some(mut catalog) = self.current(document, root) else {
            return;
        };
        let flag = Name::new(&b"NeedAppearances"[..]);
        match catalog.get("AcroForm").cloned() {
            Some(Object::Reference(id)) => {
                let Some(mut form) = self.current(document, id) else {
                    return;
                };
                form.insert(flag, Object::Boolean(true));
                self.put(id, Object::Dictionary(form));
            }
            Some(Object::Dictionary(mut form)) => {
                form.insert(flag, Object::Boolean(true));
                catalog.insert(Name::new(&b"AcroForm"[..]), Object::Dictionary(form));
                self.put(root, Object::Dictionary(catalog));
            }
            _ => {}
        }
    }

    /// Records what one object now says, replacing anything already recorded for it.
    pub(super) fn put(&mut self, id: ObjectId, object: Object) {
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
    pub(super) fn current(&self, document: &Document, id: ObjectId) -> Option<Dictionary> {
        match self.replacements.get(&id) {
            Some(object) => object.as_dict().cloned(),
            None => document.get(id).as_dict().cloned(),
        }
    }

    /// Writes a toggling button's `/AS`, the appearance state its new value selects.
    ///
    /// ISO 32000-2 §12.7.5.2.3, which §12.7.5.2.4 repeats for a radio button:
    ///
    /// > The value of the V key shall also be the value of the AS key. If they are not equal, then
    /// > the value of the AS key shall be used instead of the V key to determine which appearance
    /// > to use.
    ///
    /// So an update that wrote `/V` alone left a ticked box drawn unticked by every reader,
    /// this one included, the moment the file was opened again (ADR 1453). The state is the value
    /// where this widget's `/AP` `/N` holds an appearance of that name, and `Off` where it does not
    /// — which is how one widget of a radio set is on and its siblings, sharing the field's
    /// value, are off. A widget whose `/N` is not a dictionary of states has nothing to select
    /// among and is left as it is.
    fn write_state(&mut self, document: &Document, widget: ObjectId, value: Option<&Object>) {
        let Some(mut widget_dict) = self.current(document, widget) else {
            return;
        };
        let normal = document
            .get_key(&widget_dict, "AP")
            .as_dict()
            .map(|appearances| document.get_key(appearances, "N"));
        let Some(Object::Dictionary(states)) = normal else {
            return;
        };
        let on = value
            .map(state_name)
            .and_then(|name| name.as_name().cloned())
            .filter(|name| {
                name.as_bytes() != b"Off" && states.iter().any(|(state, _)| state == name)
            });
        let state = on.unwrap_or_else(|| Name::new(&b"Off"[..]));
        widget_dict.insert(Name::new(&b"AS"[..]), Object::Name(state));
        self.put(widget, Object::Dictionary(widget_dict));
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
    /// A widget this cannot produce a stream for — and one it can produce only part of — goes on
    /// [`Self::owed`], which is what Table 224's flag is for and what [`Written::unconstructed`]
    /// names. Writing a stream that is missing a glyph *and* setting the flag says both true
    /// things: here is what could be laid out, and it is not all of it.
    fn write_appearance(
        &mut self,
        document: &Document,
        widget: ObjectId,
        dict: &Dictionary,
        value: FieldValue<'_>,
        displayed: Option<&Displayed>,
        scripted: &[Property],
    ) {
        let built = match crate::appearance::for_saving(document, dict, value, displayed, scripted)
        {
            crate::appearance::ForSaving::Stream(built) => built,
            crate::appearance::ForSaving::States(states) => {
                self.write_states(document, widget, states);
                return;
            }
            crate::appearance::ForSaving::Selected => return,
            crate::appearance::ForSaving::Owed => {
                self.owed.push(widget);
                return;
            }
        };
        if built.report.is_some() {
            self.owed.push(widget);
        }

        let added = built.existing.is_none();
        let stream_id = match &built.existing {
            Some((id, _)) => *id,
            None => self.allocate(),
        };
        let stream = self.form_stream(built);
        self.put(stream_id, stream);

        if added {
            // A widget that had no `/AP` needs one pointing at the stream just written. Table
            // 170's `/N` is "the annotation's normal appearance"; a widget with one state has it
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

    /// Writes a toggling button's constructed states as its appearance dictionary's (ADR 1676).
    ///
    /// Table 170 gives `/N` "either a single appearance stream or an appearance subdictionary",
    /// and §12.7.5.2.3 puts a check box's states in the second, so each state is a new stream
    /// object under its own name. New rather than the producer's numbers, because a generated
    /// form shares one `Off` stream among many widgets, and replacing it would redraw them all.
    ///
    /// `/R` and `/D`, where the widget states them, name the same states: Table 170 makes each
    /// default to "the value of the N entry", and a constructed widget draws one appearance per
    /// state whatever the pointer does, so a stored rollover or down state would show the
    /// producer's glyph the moment the box is touched. The appearance dictionary is written onto
    /// the widget itself, because a dictionary held indirectly may be another widget's too.
    fn write_states(
        &mut self,
        document: &Document,
        widget: ObjectId,
        saved: crate::appearance::SavedStates,
    ) {
        if saved.states.iter().any(|(_, built)| built.report.is_some()) {
            self.owed.push(widget);
        }
        let mut states = Dictionary::new();
        for (name, built) in saved.states {
            let id = self.allocate();
            let stream = self.form_stream(built);
            self.put(id, stream);
            states.insert(name, Object::Reference(id));
        }
        let Some(mut widget_dict) = self.current(document, widget) else {
            return;
        };
        let mut appearances = document
            .get_key(&widget_dict, "AP")
            .as_dict()
            .cloned()
            .unwrap_or_default();
        for key in ["R", "D"] {
            if appearances.get(key).is_some() {
                appearances.insert(
                    Name::new(key.as_bytes()),
                    Object::Dictionary(states.clone()),
                );
            }
        }
        appearances.insert(Name::new(&b"N"[..]), Object::Dictionary(states));
        widget_dict.insert(Name::new(&b"AP"[..]), Object::Dictionary(appearances));
        self.put(widget, Object::Dictionary(widget_dict));
    }

    /// A built appearance as the form `XObject` §8.10.2 defines, its dictionary the one it replaces
    /// where there was one.
    fn form_stream(&mut self, built: crate::appearance::SavedStream) -> Object {
        let mut stream_dict = built.existing.map(|(_, dict)| dict).unwrap_or_default();
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
        let resources = promote_resource_streams(self, built.resources);
        stream_dict.insert(Name::new(&b"Resources"[..]), Object::Dictionary(resources));
        stream_dict.insert(
            Name::new(&b"Length"[..]),
            Object::Integer(i64::try_from(built.content.len()).unwrap_or(i64::MAX)),
        );
        // The bytes written are the decoded ones, so whatever the old stream said it was encoded
        // with is now a lie about it. §7.4's `/Filter` and `/DecodeParms` go with the data they
        // described — a stream keeping a `/FlateDecode` it no longer has would not decode at all.
        stream_dict.remove("Filter");
        stream_dict.remove("DecodeParms");
        Object::Stream(std::sync::Arc::new(pdf_syntax::Stream {
            dict: stream_dict,
            data: built.content.into(),
            decryption_failed: false,
        }))
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
/// replace. That is the same distinction `Update::owe_appearances` draws for `/AcroForm`.
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
fn withdrawn_usage_rights(document: &Document, update: &Update) -> Option<(ObjectId, Dictionary)> {
    use pdf_signature::signature::Right;

    let rights = pdf_signature::signature::permissions(document).usage_rights?;
    if rights.grants(Right::FillInForm) && rights.grants(Right::FullSave) {
        return None;
    }
    // As the update has it, where an earlier rewrite — §7.11.4's tree holder — replaced it: two
    // rewrites of one object compose only if the second reads the first. A catalog the trailer
    // states *directly* is nobody's replacement and has no number to be replaced under, so
    // `Document::catalog` still answers for that one and this function still withdraws from it.
    let root = document
        .trailer()
        .get("Root")
        .and_then(Object::as_reference);
    let catalog = root
        .and_then(|id| update.current(document, id))
        .or_else(|| document.catalog().ok())?;
    if let Some(id) = catalog.get("Perms").and_then(Object::as_reference) {
        let mut perms = update.current(document, id)?;
        perms.remove("UR3");
        return Some((id, perms));
    }
    let id = root?;
    let mut catalog = catalog;
    let mut perms = catalog.get("Perms")?.as_dict().cloned()?;
    perms.remove("UR3");
    catalog.insert(Name::new(&b"Perms"[..]), Object::Dictionary(perms));
    Some((id, catalog))
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
/// written.
///
/// **An annotation with no `/AP` is not legal from a writer, and Table 166 says so**: "\[a] PDF
/// writer shall include an appearance dictionary when writing or updating the PDF file except for
/// the two cases listed below", the two being a `/Rect` whose opposite corners coincide and a
/// `/Subtype` of `Popup`, `Projection` or `Link`. Errata Collection 3's Issue #22 moves the same
/// requirement into the entry's own column ("Required except for conditions listed below (PDF
/// 2.0)"). A markup this function is called for is none of the exceptions, which is why writing the
/// appearance is obligatory here rather than a courtesy. The one construction that is not written
/// is a value set in a face from this machine whose licence forbids embedding it (ADR 1425), and
/// `false` says so, for [`Written::unappeared`] to name; [`ViewState::write_retypings`] is where
/// the same rule is departed from for a layout that declined.
fn write_added_appearance(document: &Document, update: &mut Update, dict: &mut Dictionary) -> bool {
    // An annotation that arrived with an appearance of its own keeps it. §12.7.8.3.4's
    // annotations are the one kind that does — they carry the producer's `/AP`, copied whole —
    // and constructing over it would replace another producer's marks with this program's, which
    // is the line `CLAUDE.md`'s exclusion draws. The streams become objects here for
    // [`ViewState::write_imported_appearances`]'s reason: §7.3.8.1 requires it of a file.
    if let Some(stated) = dict.get("AP").cloned() {
        dict.insert(Name::new(&b"AP"[..]), promote_streams(update, stated));
        return true;
    }
    let Some(subtype) = document
        .get_key(dict, "Subtype")
        .as_name()
        .map(|name| name.as_bytes().to_vec())
    else {
        return true;
    };
    let Some(rect) = crate::annotation::rectangle(document, dict, "Rect") else {
        return true;
    };
    let built = crate::appearance::construct(
        document,
        dict,
        &subtype,
        AnnotationView::default(),
        rect,
        &crate::variable_text::LayoutFonts::new(document),
    );
    // A value set in a face from this machine is written with the face subset and embedded, and
    // not written at all where the face's licence forbids it (ADR 1425).
    let Ok(resources) = crate::variable_text::for_a_file(&built.resources) else {
        return false;
    };
    let Some(content) = built.content else {
        return true;
    };
    let resources = promote_resource_streams(update, resources);
    let id = update.allocate();
    update.put(
        id,
        crate::appearance::form_xobject(rect, resources, content),
    );
    let mut appearances = Dictionary::new();
    appearances.insert(Name::new(&b"N"[..]), Object::Reference(id));
    dict.insert(Name::new(&b"AP"[..]), Object::Dictionary(appearances));
    true
}

/// A form's resources with every stream inside them given a number in this update.
///
/// §7.3.8.1 for the fonts [`crate::variable_text::for_a_file`] writes around a face from this
/// machine: the program, its `/CIDToGIDMap` and its `/ToUnicode` are streams held directly in
/// their dictionaries until a writer numbers them (ADR 1425).
fn promote_resource_streams(update: &mut Update, resources: Dictionary) -> Dictionary {
    match promote_streams(update, Object::Dictionary(resources)) {
        Object::Dictionary(resources) => resources,
        // `promote_streams` of a dictionary answers a dictionary; an empty one is the only
        // harmless answer to a case that cannot arise.
        _ => Dictionary::new(),
    }
}

/// Gives every stream inside a carried object a number in this update, in place.
///
/// §7.3.8.1: "A stream shall consist of a dictionary followed by zero or more bytes bracketed
/// between the keywords stream … and endstream". A stream is written as an indirect object and
/// nothing else, so a value that crossed from an FDF file as a *direct* stream — which is what
/// `crate::forms_data`'s copy produces, deliberately, since a direct object names nothing —
/// becomes one here, and the reference takes its place.
///
/// Depth-first, so a stream inside another stream's `/Resources` is numbered before the stream
/// that names it. The recursion is bounded by the structure the copy produced, which
/// `crate::forms_data`'s own budget bounded when it was made. ADR 1223.
fn promote_streams(update: &mut Update, value: Object) -> Object {
    match value {
        Object::Stream(stream) => {
            let dict = match promote_streams(update, Object::Dictionary(stream.dict.clone())) {
                Object::Dictionary(dict) => dict,
                // `promote_streams` of a dictionary answers a dictionary; this arm cannot be
                // reached and keeping the stream unchanged is the only harmless thing to do.
                _ => stream.dict.clone(),
            };
            let id = update.allocate();
            update.put(
                id,
                Object::Stream(std::sync::Arc::new(pdf_syntax::Stream {
                    dict,
                    data: std::sync::Arc::clone(&stream.data),
                    decryption_failed: false,
                })),
            );
            Object::Reference(id)
        }
        Object::Dictionary(dict) => {
            let mut out = Dictionary::new();
            for (key, entry) in dict.iter() {
                out.insert(key.clone(), promote_streams(update, entry.clone()));
            }
            Object::Dictionary(out)
        }
        Object::Array(items) => Object::Array(
            items
                .into_iter()
                .map(|item| promote_streams(update, item))
                .collect(),
        ),
        other => other,
    }
}

/// Replaces the appearance of a free text annotation whose `/Contents` a person retyped.
///
/// [`ViewState::write_retypings`] carries the argument for replacing rather than splicing and for
/// building the stream dictionary afresh. What is here is the two places the new stream can go:
/// over the object Table 170's `/N` already names, where the file states one indirectly — so the
/// update replaces an object rather than adding one and orphaning another — and into a new object
/// otherwise.
///
/// `text` is passed beside the dictionary rather than read back out of it because an empty string
/// is a decision: [`crate::appearance`] falls back to Table 177's `/RC` where `/Contents` says
/// nothing, and a person who took the text out has not asked for the rich text underneath it.
///
/// Returns whether the annotation ends this update showing what it says. `false` is the one case
/// that is a shortfall — the text could not be laid out, so the annotation is written with no
/// appearance at all and the next reader constructs from Table 177's Required `/DA`. A note whose
/// text a person removed draws nothing and is `true`: there is nothing to show and nothing owed.
fn write_retyped_appearance(
    document: &Document,
    update: &mut Update,
    dict: &mut Dictionary,
    text: &str,
) -> bool {
    let Some(rect) = crate::annotation::rectangle(document, dict, "Rect") else {
        return false;
    };
    let view = AnnotationView {
        contents: Some(text),
        ..AnnotationView::default()
    };
    let built = crate::appearance::construct(
        document,
        dict,
        b"FreeText",
        view,
        rect,
        &crate::variable_text::LayoutFonts::new(document),
    );
    // A value set in a face from this machine is written with the face subset and embedded; where
    // the face's licence forbids that, the annotation is written as one owing its appearance, as
    // a layout that declined would be (ADR 1425).
    let resources = crate::variable_text::for_a_file(&built.resources).ok();
    let unwritten = resources.is_none();
    let (Some(content), Some(resources)) = (built.content, resources) else {
        // Table 177 makes `/AP` decisive over `/DA`, so a stream that draws the producer's text is
        // exactly what an annotation that no longer says it must not keep.
        dict.remove("AP");
        return !unwritten && built.report.is_none();
    };
    let resources = promote_resource_streams(update, resources);
    let existing = document
        .get_key(dict, "AP")
        .as_dict()
        .and_then(|appearances| appearances.get("N").cloned())
        .as_ref()
        .and_then(Object::as_reference);
    let id = existing.unwrap_or_else(|| update.allocate());
    update.put(
        id,
        crate::appearance::form_xobject(rect, resources, content),
    );
    let mut appearances = Dictionary::new();
    appearances.insert(Name::new(&b"N"[..]), Object::Reference(id));
    dict.insert(Name::new(&b"AP"[..]), Object::Dictionary(appearances));
    true
}

/// A field's `/V` as the file states it (`update` `None`) or as the update leaves it, inherited
/// up §12.7.4.1's `/Parent` chain from `widget`, references resolved; `Null` where no
/// dictionary of the chain states one.
fn inherited_value(document: &Document, update: Option<&Update>, widget: ObjectId) -> Object {
    let read = |id: ObjectId| match update {
        Some(update) => update.current_object(document, id),
        None => document.get(id),
    };
    let mut id = widget;
    for _ in 0..MAX_FIELD_DEPTH {
        let Some(dict) = read(id).as_dict().cloned() else {
            break;
        };
        match dict.get("V") {
            Some(Object::Reference(value)) => return read(*value),
            Some(Object::Null) | None => {}
            Some(value) => return value.clone(),
        }
        let Some(parent) = dict.get("Parent").and_then(Object::as_reference) else {
            break;
        };
        id = parent;
    }
    Object::Null
}

/// A `/V` in the terms an XFA data value needs; `None` for a value of a type no field states.
///
/// A button's name is turned into the export value of the widget it turns on: §12.7.5.2.3's
/// Table 230 `/Opt`, "an array of text strings representing the export value of each annotation
/// in the field", indexed by the widget whose `/AP` `/N` holds the state, and the state's own name
/// where the field states no `/Opt`.
fn xfa_change(
    document: &Document,
    update: &Update,
    widgets: &[ObjectId],
    value: &Object,
) -> Option<crate::xfa::Change> {
    use crate::xfa::Change;
    let text = |object: &Object| match document.resolve(object) {
        Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
        _ => None,
    };
    Some(match value {
        Object::Null => Change::Cleared,
        Object::String(bytes) => Change::Text(pdf_syntax::text_string(bytes)),
        Object::Array(items) => Change::Choices(items.iter().map(text).collect::<Option<_>>()?),
        Object::Name(state) if state.as_bytes() == crate::appearance::OFF => Change::Off,
        Object::Name(state) => {
            let has_state = |widget: &ObjectId| {
                update.current(document, *widget).is_some_and(|dict| {
                    let appearances = document.get_key(&dict, "AP");
                    appearances.as_dict().is_some_and(|appearances| {
                        document
                            .get_key(appearances, "N")
                            .as_dict()
                            .is_some_and(|states| {
                                states
                                    .iter()
                                    .any(|(name, _)| name.as_bytes() == state.as_bytes())
                            })
                    })
                })
            };
            let export = widgets.iter().position(has_state).and_then(|index| {
                let first = widgets.first()?;
                let options = inherited_entry(document, update, *first, "Opt");
                options
                    .as_array()
                    .and_then(|options| text(options.get(index)?))
            });
            Change::On(
                export.unwrap_or_else(|| String::from_utf8_lossy(state.as_bytes()).into_owned()),
            )
        }
        _ => return None,
    })
}

/// An inheritable entry up the `/Parent` chain, read through the update.
fn inherited_entry(document: &Document, update: &Update, widget: ObjectId, key: &str) -> Object {
    let mut id = widget;
    for _ in 0..MAX_FIELD_DEPTH {
        let Some(dict) = update.current(document, id) else {
            break;
        };
        let value = document.get_key(&dict, key);
        if !value.is_null() {
            return value;
        }
        let Some(parent) = dict.get("Parent").and_then(Object::as_reference) else {
            break;
        };
        id = parent;
    }
    Object::Null
}

/// The stream `id` with `data` as its decoded bytes: its dictionary kept, and §7.4's `/Filter`
/// and `/DecodeParms` removed with the encoding they described, as `Update::form_stream` does.
fn replaced_stream(document: &Document, id: ObjectId, data: Vec<u8>) -> Object {
    let mut dict = document
        .get(id)
        .as_stream()
        .map(|stream| stream.dict.clone())
        .unwrap_or_default();
    dict.remove("Filter");
    dict.remove("DecodeParms");
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(data.len()).unwrap_or(i64::MAX)),
    );
    Object::Stream(std::sync::Arc::new(pdf_syntax::Stream {
        dict,
        data: data.into(),
        decryption_failed: false,
    }))
}
