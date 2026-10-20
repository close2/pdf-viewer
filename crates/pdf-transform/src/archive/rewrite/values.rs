//! A value restated in place: an entry whose spelling, range or agreement a requirement fixes, and the
//! colour-space, encoding and font-descriptor entries that carry one.
use std::collections::BTreeMap;

use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::super::sites;
use super::super::sites::{RELATIVE_COLORIMETRIC, RENDERING_INTENTS};
use super::{Entry, Rewrite, Rewriter, count, resources_with_default, with_default};

impl Rewriter<'_> {
    /// A dictionary object, rewritten where its position asks for it.
    /// Writes the `/CIDToGIDMap` a descendant `CIDFont` owes once a program is embedded in it.
    ///
    /// §9.7.4.2 makes a Type 2 `CIDFont`'s `/CIDToGIDMap` the mapping to "the glyph indices for
    /// the appropriate glyph descriptions in that font program", and Table 115 makes the entry
    /// required of such a `CIDFont` once it has one — so a descendant whose program this
    /// conversion has just written in owes an entry it did not owe before. `Identity` is the only
    /// value written, and it is what the glyphs the preparation checked were chosen under
    /// (`doc/adr/1222`).
    pub(super) fn identity_cid_to_gid(&self, id: ObjectId, out: &mut Dictionary) -> bool {
        if !self.wants(Rewrite::SubstituteFontProgram)
            || !self
                .substitutes
                .is_some_and(|substitutes| substitutes.identity_cid_to_gid.contains(&id))
        {
            return false;
        }
        out.insert(
            Name::new(&b"CIDToGIDMap"[..]),
            Object::Name(Name::new(&b"Identity"[..])),
        );
        true
    }

    /// A graphics state's halftone entries ISO 19005 section 6.2.5 forbids, and a PDF/A output
    /// intent's missing profile (`doc/adr/2034`, `doc/adr/2035`).
    pub(super) fn halftone_and_intent_entries(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let mut changed = false;
        // Table 57's name for the device's own halftone, where the state named a threshold one
        // (`doc/adr/2034`).
        if self.wants(Rewrite::HalftoneWrittenDefault)
            && self
                .halftone_defaults
                .is_some_and(|sites| sites.at.contains(&id))
        {
            out.insert(
                Name::new(&b"HT"[..]),
                Object::Name(Name::new(&b"Default"[..])),
            );
            count(applied, Rewrite::HalftoneWrittenDefault);
            changed = true;
        }
        for (rewrite, sites, key) in [
            (Rewrite::HalftonePhaseRemoved, self.halftone_phases, "HTP"),
            (Rewrite::HalftoneOriginRemoved, self.halftone_origins, "HTO"),
            (
                Rewrite::DestinationProfileReferenceRemoved,
                self.profile_references,
                "DestOutputProfileRef",
            ),
        ] {
            if self.wants(rewrite)
                && sites.is_some_and(|sites| sites.at.contains(&id))
                && out.remove(key).is_some()
            {
                count(applied, rewrite);
                changed = true;
            }
        }
        // A PDF/A entry written as its own object, given the profile its construction names
        // (`doc/adr/2035`).
        if self.wants(Rewrite::DestinationProfileSupplied)
            && let Some(supplied) = self
                .supplied_destination
                .filter(|supplied| supplied.at.contains(&id))
        {
            out.insert(
                Name::new(&b"DestOutputProfile"[..]),
                Object::Reference(supplied.destination),
            );
            count(applied, Rewrite::DestinationProfileSupplied);
            changed = true;
        }
        changed
    }

    /// ISO 19005-2 section 6.2.4.2 by ISO 32000-2 §8.6.5.5's Table 65, inside whatever object
    /// holds the `ICCBased` array: every object is descended, as [`Self::agree_separations`]
    /// does, because a colour space in a page's resource dictionary is reported at the page.
    pub(super) fn write_profile_alternates(
        &self,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.wants(Rewrite::ProfileAlternateWritten) {
            return false;
        }
        let Some(alternates) = self.profile_alternates else {
            return false;
        };
        let written = super::super::icc::write_alternates_in(self.document, out, alternates, 0);
        for _ in 0..written {
            count(applied, Rewrite::ProfileAlternateWritten);
        }
        written > 0
    }

    /// ISO 19005-2 section 6.2.4.4 and ISO 19005-4 section 6.2.4.4, inside whatever object holds
    /// the `Separation` array.
    ///
    /// Descended rather than edited at the top level, for [`Self::write_colorants`]' reason: a
    /// colour space written in a page's resource dictionary is reported at the page. Unlike that
    /// one this reaches **every** object rather than the ones a finding named, and it has to: the
    /// arrays that need rewriting are the ones that *disagree* with the chosen definition, and a
    /// finding names only the later array of each disagreeing pair.
    pub(super) fn agree_separations(
        &self,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.wants(Rewrite::SeparationAgreed) || self.remedies.separations.is_empty() {
            return false;
        }
        let reached = sites::agree_separations(self.document, out, &self.remedies.separations, 0);
        for _ in 0..reached.len() {
            count(applied, Rewrite::SeparationAgreed);
        }
        !reached.is_empty()
    }

    /// ISO 19005-2 section 6.2.4.4 and ISO 19005-4 section 6.2.4.4, inside the object the
    /// validator named.
    ///
    /// `doc/pdf-a-mitigations.md` section 13.3.1's lesson: a colour space is reported at the
    /// object it is *written in*, which for a space defined in a page's resource dictionary is
    /// the page. So the object is descended rather than edited at its top level, and
    /// [`super::super::sites::place_colorants`] is the same placement the preparation already proved
    /// reaches every colourant it promised.
    pub(super) fn write_colorants(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.wants(Rewrite::SpotColorantEntry) {
            return false;
        }
        let Some(entries) = self.colorants.and_then(|written| written.at.get(&id)) else {
            return false;
        };
        let placed = sites::place_colorants(self.document, out, entries, 0);
        for _ in 0..placed.len() {
            count(applied, Rewrite::SpotColorantEntry);
        }
        !placed.is_empty()
    }

    /// ISO 19005-2 section 6.2.3 and ISO 19005-4 section 6.2.3, at one output intent object.
    ///
    /// The entries written directly inside the catalog's array are corrected by
    /// [`Self::rewrite_catalog`] instead, because there is no object of their own to reach.
    pub(super) fn share_destination_profile(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.wants(Rewrite::SharedDestinationProfile) {
            return false;
        }
        let Some(shared) = self.shared_profile else {
            return false;
        };
        if !shared.at.contains(&id) {
            return false;
        }
        out.insert(
            Name::new(&b"DestOutputProfile"[..]),
            Object::Reference(shared.profile),
        );
        count(applied, Rewrite::SharedDestinationProfile);
        true
    }

    /// ISO 19005-2 section 6.2.11.6 and ISO 19005-4 section 6.2.10.6's two font-dictionary rows,
    /// written at the fonts [`super::super::sites`] proved the change safe for.
    ///
    /// **Nothing is decided here.** Which fonts are symbolic is the validator's reading of the
    /// descriptor's flags, and whether the new value leaves every code on the glyph it already
    /// reached was settled by the preparation — a font that failed that comparison never reaches
    /// this walk, because the requirement is refused and no file is written.
    ///
    /// The value written for a non-symbolic font is the producer's own encoding dictionary where
    /// they wrote one, with its `/BaseEncoding` restated. An encoding dictionary the file held
    /// indirectly is written directly into this font, which changes the object graph and not one
    /// name a code resolves through: another font sharing that object goes on sharing it, and
    /// this one now states the same entries with one name corrected.
    pub(super) fn restate_truetype_encoding(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let mut changed = false;
        if self.wants(Rewrite::SymbolicTrueTypeEncodingRemoved)
            && self
                .symbolic_encodings
                .is_some_and(|sites| sites.at.contains(&id))
            && out.remove("Encoding").is_some()
        {
            count(applied, Rewrite::SymbolicTrueTypeEncodingRemoved);
            changed = true;
        }
        if self.wants(Rewrite::StandardTrueTypeEncoding)
            && let Some(value) = self
                .standard_encodings
                .and_then(|encodings| encodings.at.get(&id))
        {
            out.insert(Name::new(&b"Encoding"[..]), value.clone());
            count(applied, Rewrite::StandardTrueTypeEncoding);
            changed = true;
        }
        if self.wants(Rewrite::StandardRouteNames)
            && let Some(value) = self.standard_route.and_then(|names| names.at.get(&id))
        {
            out.insert(Name::new(&b"Encoding"[..]), value.clone());
            count(applied, Rewrite::StandardRouteNames);
            changed = true;
        }
        if self.wants(Rewrite::Type3NotdefRenamed)
            && let Some((encoding, procedures)) =
                self.type3_notdef.and_then(|renames| renames.at.get(&id))
        {
            out.insert(Name::new(&b"Encoding"[..]), encoding.clone());
            out.insert(Name::new(&b"CharProcs"[..]), procedures.clone());
            count(applied, Rewrite::Type3NotdefRenamed);
            changed = true;
        }
        changed
    }

    /// §8.6.5.8's answer, written into every entry inside this object that stated a name it does
    /// not define.
    ///
    /// **Inside the object rather than on its top-level dictionary**, and that is not a detail:
    /// `pdf_archive` reports a dictionary at the object it is written in, so a resource
    /// dictionary a page states directly carries its graphics states at the page's own object
    /// number. An edit that looked only at the dictionary the number names would fix nothing
    /// there and leave the requirement failed.
    ///
    /// The two keys are guarded differently, because only one of them is unambiguous. `/RI` is a
    /// graphics state parameter and §8.4.5's Table 57 gives it no other home, so any value
    /// outside §8.6.5.8's four names is the one this requirement is about. `/Intent` is an image
    /// `XObject`'s in §8.9.5.1's Table 87 **and** an optional content group's in §8.11.2.3,
    /// where its values are `View` and `Design` — so it is restated only where the dictionary
    /// states `/Subtype /Image`, which is the validator's own test.
    pub(super) fn restate_rendering_intent(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.wants(Rewrite::RenderingIntent)
            || !self
                .rendering_intents
                .is_some_and(|sites| sites.at.contains(&id))
        {
            return false;
        }
        let mut places = 0usize;
        self.restate_inside(out, Entry::RenderingIntent, 0, &mut places);
        for _ in 0..places {
            count(applied, Rewrite::RenderingIntent);
        }
        places > 0
    }

    /// §8.4.1's Table 57 answer, written wherever inside this object the entry was an array of
    /// names it does not define.
    ///
    /// [`Self::restate_rendering_intent`]'s reason for descending, and `/BM` needs no guard of
    /// its own: §8.4.1's Table 57 is the only place the base standard gives the key, and an array
    /// with no recognised name in it means `Normal` to a reader wherever it is written.
    pub(super) fn restate_blend_mode(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.wants(Rewrite::BlendModeNormal)
            || !self.blend_modes.is_some_and(|sites| sites.at.contains(&id))
        {
            return false;
        }
        let mut places = 0usize;
        self.restate_inside(out, Entry::BlendMode, 0, &mut places);
        for _ in 0..places {
            count(applied, Rewrite::BlendModeNormal);
        }
        places > 0
    }

    /// One dictionary and everything written inside it, with `entry` restated where it is wrong.
    ///
    /// References are **not** followed, for the reason `pdf_archive`'s own walk does not follow
    /// them: a dictionary written as its own object is reported at that object and is reached by
    /// this rewrite there.
    fn restate_inside(
        &self,
        dict: &mut Dictionary,
        entry: Entry,
        depth: usize,
        places: &mut usize,
    ) {
        if depth >= sites::MAX_ENTRY_DEPTH {
            return;
        }
        if let Some((key, value)) = self.restated_entry(dict, entry) {
            dict.insert(Name::new(key.as_bytes()), value);
            *places = places.saturating_add(1);
        }
        let deeper = depth.saturating_add(1);
        let inside: Vec<(Name, Object)> = dict
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        for (key, value) in inside {
            let mut held = value;
            if self.restate_value(&mut held, entry, deeper, places) {
                dict.insert(key, held);
            }
        }
    }

    /// One value inside a dictionary, and whether the restating changed it.
    fn restate_value(
        &self,
        value: &mut Object,
        entry: Entry,
        depth: usize,
        places: &mut usize,
    ) -> bool {
        let before = *places;
        match value {
            Object::Dictionary(dict) => self.restate_inside(dict, entry, depth, places),
            Object::Array(items) => {
                for item in items {
                    self.restate_value(item, entry, depth.saturating_add(1), places);
                }
            }
            _ => {}
        }
        *places > before
    }

    /// The entry this dictionary states wrongly, and what it is to state instead.
    fn restated_entry(&self, dict: &Dictionary, entry: Entry) -> Option<(&'static str, Object)> {
        match entry {
            Entry::RenderingIntent => {
                let key = if dict.get("RI").is_some() {
                    "RI"
                } else if dict.get("Intent").is_some() && self.subtype_is(dict, b"Image") {
                    "Intent"
                } else {
                    return None;
                };
                let stated = self.document.get_key(dict, key);
                let defined = stated
                    .as_name()
                    .is_some_and(|name| RENDERING_INTENTS.contains(&name.as_bytes()));
                (!defined).then(|| (key, Object::Name(Name::new(RELATIVE_COLORIMETRIC))))
            }
            Entry::BlendMode => {
                dict.get("BM")?;
                let Object::Array(names) = self.document.get_key(dict, "BM") else {
                    return None;
                };
                names
                    .iter()
                    .all(|name| !sites::is_blend_mode(&self.document.resolve(name)))
                    .then(|| ("BM", Object::Name(Name::new(&b"Normal"[..]))))
            }
        }
    }

    /// A font descriptor's incomplete `/CharSet` or `/CIDSet`, removed.
    pub(super) fn remove_descriptor_sets(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let Some(keys) = self
            .descriptor_sets
            .filter(|_| self.wants(Rewrite::DescriptorSetRemoved))
            .and_then(|sets| sets.at.get(&id))
        else {
            return false;
        };
        let mut changed = false;
        for key in keys {
            if out.remove(key).is_some() {
                count(applied, Rewrite::DescriptorSetRemoved);
                changed = true;
            }
        }
        changed
    }

    /// ISO 32000-1's default for `/CIDToGIDMap`, written where a part 2 target asks for the
    /// entry.
    pub(super) fn write_cid_to_gid_map(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.wants(Rewrite::CidToGidIdentity)
            || !self.cid_to_gid.is_some_and(|sites| sites.at.contains(&id))
            || !self.document.get_key(out, "CIDToGIDMap").is_null()
        {
            return false;
        }
        out.insert(
            Name::new(&b"CIDToGIDMap"[..]),
            Object::Name(Name::new(&b"Identity"[..])),
        );
        count(applied, Rewrite::CidToGidIdentity);
        true
    }

    /// §8.6.5.6's `/DefaultCMYK` and `/DefaultRGB`, each written wherever [`DefaultSites`] found
    /// a place for it, where this conversion writes it.
    ///
    /// Four places, and they are one rule read through four shapes a document can take: the
    /// `/ColorSpace` subdictionary itself where it is an object, the resource dictionary where
    /// that subdictionary is direct or absent, the holder where the resource dictionary itself is
    /// direct, and a page that has no resource dictionary of its own at all.
    ///
    /// **An entry the producer already wrote is never replaced.** A default in the file is the
    /// producer saying how their device colour is to be read, and overwriting it would be
    /// changing what the file says rather than adding what ISO 19005 requires it to say. Where
    /// such an entry is one the clause does not license, the requirement stays failed and
    /// `doc/adr/0947`'s third stage refuses to write the file.
    ///
    /// [`DefaultSites`]: super::DefaultSites
    pub(super) fn write_defaults(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let cmyk = self
            .default_cmyk
            .filter(|_| self.wants(Rewrite::DefaultCmyk))
            .map(|default| (Rewrite::DefaultCmyk, "DefaultCMYK", default.space));
        let rgb = self
            .default_rgb
            .filter(|_| self.wants(Rewrite::DefaultRgb))
            .map(|default| (Rewrite::DefaultRgb, "DefaultRGB", default.space));
        // The preparation builds at most one of the two `/DefaultCMYK` constructions, so the key
        // is never asked for twice (`doc/adr/2130`).
        let supplied = self
            .supplied_cmyk
            .filter(|_| self.wants(Rewrite::SuppliedDefaultCmyk))
            .map(|default| (Rewrite::SuppliedDefaultCmyk, "DefaultCMYK", default.space));
        let mut changed = false;
        for (rewrite, key, space) in [cmyk, supplied, rgb].into_iter().flatten() {
            if self.write_default(id, out, key, space) {
                count(applied, rewrite);
                changed = true;
            }
        }
        // A page given an inherited dictionary whose own `/ColorSpace` is another object has
        // still been changed: the entry itself goes into that object.
        changed
            || (cmyk.is_some() || supplied.is_some() || rgb.is_some())
                && self.sites.defaults.pages.contains_key(&id)
    }

    /// One default, `key` naming `space`, written at `id` where the sites put one; whether it
    /// was written here.
    fn write_default(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        key: &str,
        space: ObjectId,
    ) -> bool {
        let sites = &self.sites.defaults;
        let written = if sites.spaces.contains(&id) {
            with_default(out, key, space).map(|spaces| {
                *out = spaces;
            })
        } else if sites.resources.contains(&id) {
            resources_with_default(out, key, space).map(|resources| {
                *out = resources;
            })
        } else {
            // A page that states none of its own is given one first — once, so that a second
            // default goes into the dictionary the first was written into — and the entry then
            // goes into that dictionary by the same rule as any other direct one.
            if let Some(resources) = sites.pages.get(&id)
                && out.get("Resources").is_none()
            {
                out.insert(Name::new(&b"Resources"[..]), resources.clone());
            }
            sites
                .direct
                .contains(&id)
                .then(|| out.get("Resources").and_then(Object::as_dict).cloned())
                .flatten()
                .and_then(|resources| resources_with_default(&resources, key, space))
                .map(|resources| {
                    out.insert(Name::new(&b"Resources"[..]), Object::Dictionary(resources));
                })
        };
        written.is_some()
    }

    /// ISO 19005-2 section 6.7.4 at a dictionary: the catalog's `/Lang`, a structure element's,
    /// or a property list's — the object's own or one it files under `/Properties`.
    ///
    /// Counted from the preparation by [`count_what_has_no_place`], since one object may hold a
    /// restated entry of each class and a stream may hold several (`doc/adr/2066`).
    pub(super) fn restate_languages(&self, id: ObjectId, out: &mut Dictionary) -> bool {
        let Some(restated) = self.languages else {
            return false;
        };
        let mut changed = false;
        if self.wants(Rewrite::CatalogLanguageRestated) && restated.catalog == Some(id) {
            out.insert(Name::new(&b"Lang"[..]), restated.catalog_value.clone());
            changed = true;
        }
        if self.wants(Rewrite::MarkedLanguagesRestated) {
            changed |= super::super::languages::apply(restated, id, out);
        }
        changed
    }
}
