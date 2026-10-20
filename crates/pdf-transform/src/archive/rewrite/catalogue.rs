//! The catalogue and the interactive form: the output intents and the entries a
//! requirement edits, the signatures taken out, and the permissions restated.
use std::collections::{BTreeMap, BTreeSet};

use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::super::prepare::{intent_dictionary, needs_a_profile, output_intent_entries};
use super::super::signatures::Site;
use super::super::sites;
use super::{Rewrite, Rewriter, clear_append_only, count};

impl Rewriter<'_> {
    /// ISO 19005-2 section 6.7.3.4 at the object holding the role map: the structure tree root,
    /// whose `/RoleMap` entry is extended, or the map itself where the root names it by reference.
    pub(super) fn extend_the_role_map(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.wants(Rewrite::RoleMapSupplied) {
            return false;
        }
        let Some(roles) = self.remedies.role_entries.as_ref() else {
            return false;
        };
        if roles.at != id {
            return false;
        }
        if roles.in_the_root {
            let held = out
                .get("RoleMap")
                .and_then(Object::as_dict)
                .cloned()
                .unwrap_or_default();
            out.insert(
                Name::new(&b"RoleMap"[..]),
                Object::Dictionary(super::super::roles::extended(&held, &roles.entries)),
            );
        } else {
            *out = super::super::roles::extended(out, &roles.entries);
        }
        for _ in 0..roles.entries.len() {
            count(applied, Rewrite::RoleMapSupplied);
        }
        true
    }

    /// The catalog: §7.7.2's `/Requirements`, `/Version` and a direct `/Names`.
    /// §12.4.2's labels, where the catalog states the number tree inline rather than by reference.
    ///
    /// The extended tree is the preparation's whichever of the two shapes the document used, and
    /// [`Self::preserve`] writes the other.
    fn state_the_labels_inline(&self, catalog: &mut Dictionary) -> bool {
        if !self.wants(Rewrite::PreservedAsPage) {
            return false;
        }
        let Some(composed) = self.preserved else {
            return false;
        };
        let Some((None, labels)) = composed.labels.as_ref() else {
            return false;
        };
        catalog.insert(
            Name::new(&b"PageLabels"[..]),
            Object::Dictionary(labels.clone()),
        );
        true
    }

    /// ISO 19005 section 6.2.3's `OutputIntents` array: the entries the source holds corrected or
    /// completed, and a PDF/A entry appended where none is.
    fn write_output_intents(
        &self,
        catalog: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let mut changed = false;
        // ISO 19005 section 6.2.3's array, which two rewrites write: one corrects the entries
        // the source holds and one appends a PDF/A entry, and a document wanting both gets one
        // array with both done to it rather than two writers overwriting each other.
        let corrected = self
            .wants(Rewrite::SharedDestinationProfile)
            .then(|| {
                self.shared_profile
                    .and_then(|shared| shared.entries.clone())
            })
            .flatten();
        if let Some(entries) = &corrected
            && !self.wants(Rewrite::OutputIntent)
        {
            catalog.insert(
                Name::new(&b"OutputIntents"[..]),
                Object::Array(entries.clone()),
            );
            count(applied, Rewrite::SharedDestinationProfile);
            changed = true;
        }
        // A PDF/A entry stating no profile is given the one this conversion's own entry would
        // name, and is then that entry: one written in the array is completed here, and no second
        // entry is appended beside it (`doc/adr/2035`).
        let supplied = self
            .supplied_destination
            .filter(|_| self.wants(Rewrite::DestinationProfileSupplied));
        let mut completed = 0_usize;
        let entries = supplied
            .filter(|supplied| supplied.in_catalog)
            .map(|supplied| {
                corrected
                    .clone()
                    .unwrap_or_else(|| output_intent_entries(self.document, catalog))
                    .into_iter()
                    .map(|entry| match entry {
                        Object::Dictionary(mut dict) if needs_a_profile(self.document, &dict) => {
                            dict.insert(
                                Name::new(&b"DestOutputProfile"[..]),
                                Object::Reference(supplied.destination),
                            );
                            completed = completed.saturating_add(1);
                            Object::Dictionary(dict)
                        }
                        other => other,
                    })
                    .collect::<Vec<_>>()
            });
        for _ in 0..completed {
            count(applied, Rewrite::DestinationProfileSupplied);
        }
        if let Some(entries) = &entries
            && !self.wants(Rewrite::OutputIntent)
        {
            if corrected.is_some() {
                count(applied, Rewrite::SharedDestinationProfile);
            }
            catalog.insert(
                Name::new(&b"OutputIntents"[..]),
                Object::Array(entries.clone()),
            );
            changed = true;
        }
        if self.wants(Rewrite::OutputIntent)
            && let Some(intent) = self.intent
        {
            // Written as a direct array whatever the source stated it as: an array object the
            // source held indirectly is not carried, because nothing in the rewritten catalog
            // refers to it any more. Its entries are — a reference among them is renumbered like
            // any other reference this verb rewrites.
            let mut entries = entries
                .or_else(|| corrected.clone())
                .unwrap_or_else(|| output_intent_entries(self.document, catalog));
            if corrected.is_some() {
                count(applied, Rewrite::SharedDestinationProfile);
            }
            if supplied.is_none() {
                entries.push(Object::Dictionary(intent_dictionary(intent)));
            }
            catalog.insert(Name::new(&b"OutputIntents"[..]), Object::Array(entries));
            changed = true;
        }
        changed
    }

    pub(super) fn rewrite_catalog(
        &self,
        catalog: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let mut changed = false;
        if self.wants(Rewrite::RequirementsDictionary) && catalog.remove("Requirements").is_some() {
            count(applied, Rewrite::RequirementsDictionary);
            changed = true;
        }
        if self.wants(Rewrite::NeedsRendering) && catalog.remove("NeedsRendering").is_some() {
            count(applied, Rewrite::NeedsRendering);
            changed = true;
        }
        changed |= self.state_the_labels_inline(catalog);
        changed |= self.file_the_attachments(catalog, applied);
        if self.wants(Rewrite::CatalogVersion) && catalog.get("Version").is_some() {
            // ISO 19005-4 section 6.1.12 fixes the shape of this value; §7.7.2's Table 28 gives
            // the entry its meaning, "[t]he version of the PDF specification to which this
            // document conforms". Restating it as the header's own version is the smallest
            // value that satisfies both, and it is the one the file already asserts.
            let (major, minor) = match self.target.part() {
                pdf_archive::Part::Two => (1, 7),
                pdf_archive::Part::Four => (2, 0),
            };
            catalog.insert(
                Name::new(&b"Version"[..]),
                Object::Name(Name::new(format!("{major}.{minor}").as_bytes())),
            );
            count(applied, Rewrite::CatalogVersion);
            changed = true;
        }
        changed |= self.write_output_intents(catalog, applied);
        if self.wants_metadata()
            && let Some(metadata) = self.metadata
            && metadata.written.is_some()
        {
            catalog.insert(Name::new(&b"Metadata"[..]), Object::Reference(metadata.at));
            changed = true;
        }
        if self.wants(Rewrite::MarkInfo) {
            // ISO 19005-2 section 6.7.2.2 asks for one entry in one dictionary, and the entry
            // is written into whatever `/MarkInfo` the producer stated rather than over it: a
            // `/UserProperties` or `/Suspects` beside it is theirs and says nothing this
            // requirement is about.
            let mut mark_info = match self.document.get_key(catalog, "MarkInfo") {
                Object::Dictionary(stated) => stated,
                _ => Dictionary::new(),
            };
            mark_info.insert(Name::new(&b"Marked"[..]), Object::Boolean(true));
            catalog.insert(Name::new(&b"MarkInfo"[..]), Object::Dictionary(mark_info));
            count(applied, Rewrite::MarkInfo);
            changed = true;
        }
        if self.wants(Rewrite::OptionalContentOrder)
            && let Some(orders) = self.orders
            && let Some(properties) = &orders.in_catalog
        {
            // The catalog states `/OCProperties` directly, so the completed configurations go
            // back into the catalog rather than into an object of their own.
            catalog.insert(
                Name::new(&b"OCProperties"[..]),
                Object::Dictionary(properties.clone()),
            );
            count(applied, Rewrite::OptionalContentOrder);
            changed = true;
        }
        changed |= self.remove_the_catalogs_automatic_states(catalog, applied);
        changed |= self.name_the_catalogs_configurations(catalog, applied);
        if self.wants(Rewrite::AlternatePresentations)
            && let Some(Object::Dictionary(names)) = catalog.get("Names")
        {
            let mut names = names.clone();
            if names.remove("AlternatePresentations").is_some() {
                catalog.insert(Name::new(&b"Names"[..]), Object::Dictionary(names));
                count(applied, Rewrite::AlternatePresentations);
                changed = true;
            }
        }
        changed |= self.edit_catalog_dictionaries(catalog, applied);
        changed
    }

    /// §12.8.6's permissions dictionary and §12.7.3's interactive form dictionary, where the
    /// catalog writes either directly: the same edits their own objects would take.
    fn edit_catalog_dictionaries(
        &self,
        catalog: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let mut changed = false;
        if self.permissions_site() == Some(Site::InCatalog)
            && let Some(Object::Dictionary(permissions)) = catalog.get("Perms")
        {
            let mut permissions = permissions.clone();
            if self.edit_permissions(&mut permissions, applied) {
                catalog.insert(Name::new(&b"Perms"[..]), Object::Dictionary(permissions));
                changed = true;
            }
        }
        if self
            .signatures
            .filter(|_| self.wants(Rewrite::SignatureValueRemoved))
            .and_then(|signatures| signatures.form)
            == Some(Site::InCatalog)
            && let Some(Object::Dictionary(form)) = catalog.get("AcroForm")
        {
            let mut form = form.clone();
            if clear_append_only(&mut form, applied) {
                catalog.insert(Name::new(&b"AcroForm"[..]), Object::Dictionary(form));
                changed = true;
            }
        }
        changed |= self.forget_embedded_files(None, catalog, applied);
        // §12.7.3 puts the form dictionary where the catalog's `/AcroForm` names it, and a
        // producer may write it directly: the two section 6.4 removals edit whatever this catalog
        // now holds, after the append-only clear above, so a document needing both gets one
        // dictionary with both done to it.
        if let Some(Object::Dictionary(form)) = catalog.get("AcroForm") {
            let mut form = form.clone();
            if self.edit_interactive_form(Site::InCatalog, &mut form, applied) {
                catalog.insert(Name::new(&b"AcroForm"[..]), Object::Dictionary(form));
                changed = true;
            }
        }
        changed
    }

    /// ISO 19005's three clause 6.4 and 6.8 edits, at whichever object each one's site names.
    ///
    /// One function rather than three branches in [`Self::rewrite_dictionary`], on
    /// [`Self::owed_rewrites`]' reason: each is an edit at objects a preparation chose, and none
    /// of them decides anything about the object it is handed.
    pub(super) fn interactive_edits(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let mut changed = false;
        if self.wants(Rewrite::WidgetAppearance)
            && let Some(stream) = self.field_appearances.and_then(|built| built.at.get(&id))
        {
            // The population is the widgets whose appearance dictionary states no `/N`, so an
            // `/AP` the producer wrote is added to rather than replaced: §12.5.5's Table 170 makes
            // `/R` and `/D` the rollover and down appearances, and a widget that states one of
            // those and no normal one keeps it. (ISO 19005's own section 6.3.3 row is what then
            // reduces the dictionary to `/N`, at the population it reported.)
            let mut appearance = self
                .document
                .get_key(out, "AP")
                .as_dict()
                .cloned()
                .unwrap_or_default();
            appearance.insert(Name::new(&b"N"[..]), Object::Reference(*stream));
            out.insert(Name::new(&b"AP"[..]), Object::Dictionary(appearance));
            count(applied, Rewrite::WidgetAppearance);
            changed = true;
        }
        changed |= self.edit_interactive_form(Site::Object(id), out, applied);
        changed |= self.forget_embedded_files(Some(id), out, applied);
        changed
    }

    /// Takes every removed file out of the `/Names` arrays this dictionary reaches directly.
    ///
    /// §7.9.6 makes a leaf's `/Names` "an array of the form [ key1 value1 key2 value2 … keyn
    /// valuen ]", so a removed specification takes the name beside it: an odd-length remainder
    /// would be a tree no reader could read. The descent follows only entries written *directly*
    /// inside this dictionary — a node that is an object of its own is reached by the walk, and
    /// `super::super::sites::embedded_files` recorded it there.
    pub(super) fn prune_name_tree(
        out: &mut Dictionary,
        removed: &BTreeSet<ObjectId>,
        depth: usize,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if depth >= sites::MAX_NAME_TREE_DEPTH {
            return false;
        }
        let mut changed = false;
        // §7.7.4's name dictionary states `/Names` as a *dictionary* of name trees, and §7.9.6's
        // leaf states it as an array; the descent starts at the first and the pruning happens at
        // the second, so one key is read two ways and which it is decides which.
        if let Some(Object::Dictionary(mut names)) = out.get("Names").cloned()
            && Self::prune_name_tree(&mut names, removed, depth.saturating_add(1), applied)
        {
            out.insert(Name::new(&b"Names"[..]), Object::Dictionary(names));
            changed = true;
        }
        if let Some(Object::Array(names)) = out.get("Names") {
            let kept: Vec<Object> = names
                .chunks(2)
                .filter(|pair| {
                    pair.get(1)
                        .and_then(Object::as_reference)
                        .is_none_or(|id| !removed.contains(&id))
                })
                .flat_map(<[Object]>::to_vec)
                .collect();
            if kept.len() != names.len() {
                out.insert(Name::new(&b"Names"[..]), Object::Array(kept));
                count(applied, Rewrite::EmbeddedFileRemoved);
                changed = true;
            }
        }
        for key in ["EmbeddedFiles", "Kids"] {
            match out.get(key).cloned() {
                Some(Object::Dictionary(mut node)) => {
                    if Self::prune_name_tree(&mut node, removed, depth.saturating_add(1), applied) {
                        out.insert(Name::new(key.as_bytes()), Object::Dictionary(node));
                        changed = true;
                    }
                }
                Some(Object::Array(kids)) => {
                    let mut written = Vec::with_capacity(kids.len());
                    let mut any = false;
                    for kid in kids {
                        match kid {
                            Object::Dictionary(mut node) => {
                                any |= Self::prune_name_tree(
                                    &mut node,
                                    removed,
                                    depth.saturating_add(1),
                                    applied,
                                );
                                written.push(Object::Dictionary(node));
                            }
                            other => written.push(other),
                        }
                    }
                    if any {
                        out.insert(Name::new(key.as_bytes()), Object::Array(written));
                        changed = true;
                    }
                }
                _ => {}
            }
        }
        changed
    }

    /// ISO 19005-2 sections 6.4.1 and 6.4.2, in the form dictionary wherever §12.7.3 put it.
    ///
    /// One function for both keys because they are one dictionary and one preparation:
    /// `super::super::sites::interactive_form` reads where it is once, and each removal is wanted or not
    /// by its own requirement. The site is compared rather than the object hunted for, so a
    /// dictionary that merely happens to hold a `/NeedAppearances` is untouched.
    fn edit_interactive_form(
        &self,
        site: Site,
        form: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let mut changed = false;
        if self.wants(Rewrite::NeedAppearancesCleared)
            && self.form == Some(site)
            && form.remove("NeedAppearances").is_some()
        {
            count(applied, Rewrite::NeedAppearancesCleared);
            changed = true;
        }
        if self.wants(Rewrite::XfaRemoved) && self.xfa == Some(site) && form.remove("XFA").is_some()
        {
            count(applied, Rewrite::XfaRemoved);
            changed = true;
        }
        changed
    }

    /// `doc/pdf-a-conversion-limits.md` section 3.6, at one signature field or widget: the
    /// value goes, and nothing else in the dictionary is touched.
    pub(super) fn unsign(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let Some(signatures) = self
            .signatures
            .filter(|_| self.wants(Rewrite::SignatureValueRemoved))
        else {
            return false;
        };
        if !signatures.values_at.contains(&id) || out.remove("V").is_none() {
            return false;
        }
        count(applied, Rewrite::SignatureValueRemoved);
        true
    }

    /// Where the permissions dictionary is, if either rewrite that edits it is wanted.
    pub(super) fn permissions_site(&self) -> Option<Site> {
        let signed = self
            .signatures
            .filter(|_| self.wants(Rewrite::SignatureValueRemoved))
            .and_then(|signatures| signatures.permissions);
        let foreign = self
            .foreign_handlers
            .filter(|_| self.wants(Rewrite::ForeignPermissionHandlers))
            .map(|handlers| handlers.at);
        signed.or(foreign)
    }

    /// Both edits to §12.8.6's permissions dictionary, at whichever site holds it.
    ///
    /// The two are one function because they are one dictionary: a document wanting both gets
    /// one edited dictionary rather than two writers overwriting each other. The `DocMDP` and
    /// `UR3` entries go under [`Rewrite::SignatureValueRemoved`] — Table 263 makes each a
    /// signature — and every other key under [`Rewrite::ForeignPermissionHandlers`]. What is
    /// left may be empty, and stays: Table 263 makes every entry optional, so an empty
    /// permissions dictionary states nothing and is not a shape the conversion has to answer for.
    pub(super) fn edit_permissions(
        &self,
        permissions: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let mut changed = false;
        if self
            .signatures
            .filter(|_| self.wants(Rewrite::SignatureValueRemoved))
            .is_some_and(|signatures| signatures.permissions.is_some())
        {
            for key in ["DocMDP", "UR3"] {
                if permissions.remove(key).is_some() {
                    count(applied, Rewrite::SignatureValueRemoved);
                    changed = true;
                }
            }
        }
        if let Some(handlers) = self
            .foreign_handlers
            .filter(|_| self.wants(Rewrite::ForeignPermissionHandlers))
        {
            for key in &handlers.keys {
                if permissions.remove(key).is_some() {
                    count(applied, Rewrite::ForeignPermissionHandlers);
                    changed = true;
                }
            }
        }
        changed
    }
}
