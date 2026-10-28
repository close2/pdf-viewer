//! A value restated in place: an entry whose spelling, range or agreement a requirement fixes, and the
//! colour-space, encoding and font-descriptor entries that carry one.
use std::collections::BTreeMap;

use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::super::sites;
use super::super::sites::{RELATIVE_COLORIMETRIC, RENDERING_INTENTS};
use super::{Entry, Rewrite, Rewriter, count, resources_with_default, with_default};

/// What a dictionary being restated is known to be by where it stands in the object the rewrite
/// was asked to reach.
///
/// `/RI` is a graphics state parameter (ISO 32000-2 §8.4.5, Table 57), but the key is also an
/// annotation's `/MK` entry in Table 192, the rollover icon of a push button, where a stream
/// stands in the place of a name. A dictionary is therefore restated for its `/RI` only where
/// `pdf_archive`'s own rule makes it a graphics state: one entry of an `ExtGState` table, a
/// dictionary typed `ExtGState`, or the object the rewrite was asked to reach.
#[derive(Clone, Copy)]
enum Held {
    /// The object itself, which is a site because a graphics state or an image is written there.
    TopLevel,
    /// An `ExtGState` table, itself or as an object of its own.
    StateTable,
    /// One entry of such a table.
    State,
    /// Any other dictionary written inside the object.
    Elsewhere,
}

impl Rewriter<'_> {
    /// Moves a descriptor's program to `/FontFile3`, the same stream under the key §9.9's Table
    /// 124 files its format under (ADR 2307), writes a `CIDFont` the move retypes as the subtype
    /// that row is under (ADR 2326), and gives a `Type0` font over a re-keyed one the `CMap` that
    /// re-keys its codes (ADR 2342).
    ///
    /// Counted once, at the stream, which every refiled program has.
    pub(super) fn refile_font_program(&self, id: ObjectId, out: &mut Dictionary) -> bool {
        if self.retype_cid_font(id, out) || self.reencode_type0(id, out) {
            return true;
        }
        let Some(from) = self
            .font_files
            .filter(|_| self.wants(Rewrite::FontProgramRefiled))
            .and_then(|refiled| refiled.moved.get(&id))
        else {
            return false;
        };
        let Some(program) = out.remove(from) else {
            return false;
        };
        out.insert(Name::new(&b"FontFile3"[..]), program);
        true
    }

    /// Writes a `CIDFontType2` dictionary whose `Identity` map makes each CID its glyph's index
    /// over a name-keyed CFF program as the `CIDFontType0` that selects the same glyphs, and
    /// removes the `/CIDToGIDMap` Table 115 gives a Type 2 `CIDFont` alone (ADR 2326).
    ///
    /// A font dictionary is never a descriptor, so an object is one or the other of the two
    /// edits [`Self::refile_font_program`] makes.
    fn retype_cid_font(&self, id: ObjectId, out: &mut Dictionary) -> bool {
        if !self.wants(Rewrite::FontProgramRefiled)
            || !self
                .font_files
                .is_some_and(|refiled| refiled.retyped.contains(&id))
        {
            return false;
        }
        out.insert(
            Name::new(&b"Subtype"[..]),
            Object::Name(Name::new(&b"CIDFontType0"[..])),
        );
        out.remove("CIDToGIDMap");
        // §9.7.4.3 keys `/W` by CID, and the CIDs of a font whose map was a stream are now the
        // glyph indices they selected (ADR 2342).
        if let Some(rekeyed) = self.font_files.and_then(|refiled| refiled.rekeyed.get(&id)) {
            if rekeyed.widths.is_empty() {
                out.remove("W");
            } else {
                out.insert(Name::new(&b"W"[..]), Object::Array(rekeyed.widths.clone()));
            }
        }
        true
    }

    /// Writes a `Type0` font over a re-keyed `CIDFont` with the embedded `CMap` that sends each
    /// code to the glyph index it drew, where the font named an identity `CMap` (ADR 2342).
    ///
    /// A `Type0` font is neither a `CIDFont` nor a descriptor, so an object is at most one of the
    /// three edits [`Self::refile_font_program`] makes.
    fn reencode_type0(&self, id: ObjectId, out: &mut Dictionary) -> bool {
        let Some(at) = self
            .font_files
            .filter(|_| self.wants(Rewrite::FontProgramRefiled))
            .and_then(|refiled| refiled.encodings.get(&id))
        else {
            return false;
        };
        out.insert(Name::new(&b"Encoding"[..]), Object::Reference(*at));
        true
    }

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

    /// Writes the entries a standard font that stated no descriptor gains once its face is embedded.
    ///
    /// `/FontDescriptor`, and the widths the page is laid out by where the file stated none; for a
    /// face the dictionary must be a `TrueType` one to hold, its `/Subtype` and, where the file's
    /// encoding is not of the kind ISO 19005-2 section 6.2.11.6 admits for one, the restated
    /// encoding. Every value was prepared and proved against the source font before it got here
    /// (`doc/adr/2146`).
    pub(super) fn describe_font(&self, id: ObjectId, out: &mut Dictionary) -> bool {
        if !self.wants(Rewrite::SubstituteFontProgram) {
            return false;
        }
        let Some(entries) = self
            .substitutes
            .and_then(|substitutes| substitutes.described.get(&id))
        else {
            return false;
        };
        for (key, value) in entries.iter() {
            out.insert(key.clone(), value.clone());
        }
        true
    }

    /// Puts a reference to its own object where each font written directly inside a resource
    /// dictionary stood, the object being the dictionary with the entries it gained
    /// ([`super::super::fonts::gains`], `doc/adr/2162`).
    ///
    /// **Descended, and matched by value.** The survey met the font as a value inside whatever
    /// dictionary holds the resources — a page, a form's stream dictionary, a resource dictionary
    /// of its own, an inherited one a page is given — so every dictionary and array inside this
    /// object is searched, and a dictionary equal to one written out is the font. Equal
    /// dictionaries are one font by §7.3.10's equivalence, so they share the one object.
    pub(super) fn reference_direct_fonts(
        &self,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.wants(Rewrite::SubstituteFontProgram) {
            return false;
        }
        let Some(direct) = self
            .substitutes
            .map(|substitutes| substitutes.direct.as_slice())
            .filter(|direct| !direct.is_empty())
        else {
            return false;
        };
        let written = referenced_in_dictionary(out, direct, 0);
        for _ in 0..written {
            count(applied, Rewrite::SubstituteFontProgram);
        }
        written > 0
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
    /// written at the fonts [`mod@super::super::sites`] proved the change safe for.
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
        if self.wants(Rewrite::TrueTypeCmapRestated)
            && let Some(descriptor) = self
                .cmap_restated
                .and_then(|restated| restated.descriptors.get(&id))
        {
            out.remove("Encoding");
            out.insert(
                Name::new(&b"FontDescriptor"[..]),
                Object::Reference(*descriptor),
            );
            count(applied, Rewrite::TrueTypeCmapRestated);
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
    /// The two keys are guarded differently, because neither is unambiguous. `/RI` is a graphics
    /// state parameter in §8.4.5's Table 57 and a push button's rollover icon in Table 192, so it
    /// is restated only in a dictionary that [`Held`] makes a graphics state, and there any value
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
        let held = if self
            .rendering_intents
            .is_some_and(|sites| sites.tables.contains(&id))
        {
            Held::StateTable
        } else {
            Held::TopLevel
        };
        self.restate_inside(out, Entry::RenderingIntent, 0, &mut places, held);
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
        self.restate_inside(out, Entry::BlendMode, 0, &mut places, Held::TopLevel);
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
        held: Held,
    ) {
        if depth >= sites::MAX_ENTRY_DEPTH {
            return;
        }
        if let Some((key, value)) = self.restated_entry(dict, entry, held) {
            dict.insert(Name::new(key.as_bytes()), value);
            *places = places.saturating_add(1);
        }
        let deeper = depth.saturating_add(1);
        let inside: Vec<(Name, Object)> = dict
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        for (key, value) in inside {
            let child = match (held, key.as_bytes()) {
                (Held::StateTable, _) => Held::State,
                (_, b"ExtGState") => Held::StateTable,
                _ => Held::Elsewhere,
            };
            let mut value = value;
            if self.restate_value(&mut value, entry, deeper, places, child) {
                dict.insert(key, value);
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
        held: Held,
    ) -> bool {
        let before = *places;
        match value {
            Object::Dictionary(dict) => self.restate_inside(dict, entry, depth, places, held),
            Object::Array(items) => {
                for item in items {
                    self.restate_value(
                        item,
                        entry,
                        depth.saturating_add(1),
                        places,
                        Held::Elsewhere,
                    );
                }
            }
            _ => {}
        }
        *places > before
    }

    /// The entry this dictionary states wrongly, and what it is to state instead.
    fn restated_entry(
        &self,
        dict: &Dictionary,
        entry: Entry,
        held: Held,
    ) -> Option<(&'static str, Object)> {
        match entry {
            Entry::RenderingIntent => {
                let key = if dict.get("RI").is_some()
                    && (matches!(held, Held::TopLevel | Held::State)
                        || self.subtype_is_named(dict, "Type", b"ExtGState"))
                {
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

    /// ISO 32000-1's default for `/CIDToGIDMap`, written where `sites::cid_to_gid_maps` found
    /// the entry absent and that edition describing the file.
    ///
    /// Into the `CIDFont` where it is an object of its own, and into the first element of the
    /// Type 0 font's `/DescendantFonts` array where the `CIDFont` is written there (ADR 2432).
    pub(super) fn write_cid_to_gid_map(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let identity = || Object::Name(Name::new(&b"Identity"[..]));
        if !self.wants(Rewrite::CidToGidIdentity) {
            return false;
        }
        let Some(sites) = self.cid_to_gid else {
            return false;
        };
        if sites.at.contains(&id) {
            if !self.document.get_key(out, "CIDToGIDMap").is_null() {
                return false;
            }
            out.insert(Name::new(&b"CIDToGIDMap"[..]), identity());
        } else if sites.descendant_in.contains(&id) {
            let Some(Object::Array(items)) = out.get("DescendantFonts") else {
                return false;
            };
            let Some(Object::Dictionary(held)) = items.first() else {
                return false;
            };
            if !self.document.get_key(held, "CIDToGIDMap").is_null() {
                return false;
            }
            let mut held = held.clone();
            held.insert(Name::new(&b"CIDToGIDMap"[..]), identity());
            let mut items = items.clone();
            if let Some(first) = items.first_mut() {
                *first = Object::Dictionary(held);
            }
            out.insert(Name::new(&b"DescendantFonts"[..]), Object::Array(items));
        } else {
            return false;
        }
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
    /// Counted from the preparation by [`super::count_what_has_no_place`], since one object may hold a
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

/// How deep [`referenced_in`] descends looking for a font written directly inside a resource
/// dictionary: past a page's or a form's `/Resources` and its `/Font` with room to spare, and a
/// bound on a hostile nesting of arrays.
const DIRECT_FONT_DEPTH: usize = 32;

/// Every font of `direct` found inside `dict`, replaced by a reference to its object; the count.
fn referenced_in_dictionary(
    dict: &mut Dictionary,
    direct: &[(Dictionary, ObjectId)],
    depth: usize,
) -> usize {
    let entries: Vec<(Name, Object)> = dict
        .iter()
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    let mut written: usize = 0;
    for (name, mut value) in entries {
        let here = referenced_in(&mut value, direct, depth);
        if here > 0 {
            dict.insert(name, value);
            written = written.saturating_add(here);
        }
    }
    written
}

/// [`referenced_in_dictionary`] for one value: the value itself where it is one of the fonts, and
/// what it holds otherwise.
fn referenced_in(value: &mut Object, direct: &[(Dictionary, ObjectId)], depth: usize) -> usize {
    if depth > DIRECT_FONT_DEPTH {
        return 0;
    }
    if let Object::Dictionary(dict) = &*value
        && let Some((_, at)) = direct.iter().find(|(font, _)| font == dict)
    {
        *value = Object::Reference(*at);
        return 1;
    }
    match value {
        Object::Dictionary(dict) => referenced_in_dictionary(dict, direct, depth.saturating_add(1)),
        Object::Array(items) => items
            .iter_mut()
            .map(|item| referenced_in(item, direct, depth.saturating_add(1)))
            .fold(0, usize::saturating_add),
        _ => 0,
    }
}
