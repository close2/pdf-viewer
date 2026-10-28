//! Annotations: those removed, those hidden, and the appearance written into the page in their place.
use std::collections::BTreeMap;

use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::{PRINT, Rewrite, Rewriter, count};

impl Rewriter<'_> {
    /// Puts a forbidden annotation's marks back onto the producer's own page, where §12.5.5 had them.
    ///
    /// `doc/adr/1123`, the construction `doc/adr/1120`'s amendment put in scope. The two edits are
    /// composed in [`super::super::preserve::relocate_a_page`]: the page's `/Contents` becomes the array
    /// with the `q`-prepend and the closing stream around the producer's own streams, and its
    /// `/Resources` gains the appearance under `/XObject`. Neither is a mark — what draws is
    /// §12.5.5's own placement of the producer's own stream. The producer's `/Contents` streams are
    /// referenced, never rewritten, so not a byte of the marks changes.
    pub(super) fn relocate_onto_page(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let Some(composed) = self.preserved else {
            return false;
        };
        let Some(relocated) = composed.relocations.get(&id) else {
            return false;
        };
        out.insert(Name::new(&b"Contents"[..]), relocated.contents.clone());
        out.insert(Name::new(&b"Resources"[..]), relocated.resources.clone());
        count(applied, Rewrite::RelocatedOnPage);
        true
    }

    /// Restates the entries ISO 19005's action clauses ask of this object.
    ///
    /// [`super::super::actions`] worked out what each holder, each `/AA` and each surviving action is to
    /// state; this puts it in the object. **Each edit names the rewrite that asked for it**, so a
    /// conversion whose caller departed from one of the three rows carries out the other two and
    /// leaves that one's entries as the producer wrote them.
    pub(super) fn remove_the_actions(&self, id: ObjectId, out: &mut Dictionary) -> bool {
        let Some(edits) = self.actions.and_then(|removals| removals.edits.get(&id)) else {
            return false;
        };
        let mut changed = false;
        for edit in edits {
            if !self.wants(edit.by()) {
                continue;
            }
            match edit {
                super::super::actions::Edit::Entry { key, value, .. } => match value {
                    None => changed |= out.remove(key).is_some(),
                    Some(value) => {
                        if out.get(key) != Some(value) {
                            out.insert(Name::new(key.as_bytes()), value.clone());
                            changed = true;
                        }
                    }
                },
                super::super::actions::Edit::Whole { dict, .. } => {
                    if out != dict {
                        out.clone_from(dict);
                        changed = true;
                    }
                }
            }
        }
        changed
    }

    /// The two removals ISO 19005's annotation clauses ask of one page's `/Annots` array.
    ///
    /// Section 6.3.1's subtype rule and section 6.3.2's flag rule are two authorisations and two
    /// counts, and they are one pass because each reads the array the other left: a page that
    /// loses its last annotation to either loses the array itself, and a page whose annotations
    /// both rules name loses them together.
    pub(super) fn remove_the_annotations(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        if !self.sites.pages.contains(&id) {
            return false;
        }
        let mut changed = self.remove_the_forbidden_annotations(out, applied);
        changed |= self.remove_the_hidden_annotations(out, applied);
        changed
    }

    /// Takes every annotation the target's part does not admit out of one page's `/Annots`.
    ///
    /// ISO 19005-2 section 6.3.1 and ISO 19005-4 section 6.3.1 forbid the subtype and offer
    /// nothing to put in its place, so the reference is what goes. Nothing hunts for the
    /// annotation object: the walk copies what the converted document reaches, so an annotation
    /// no array names is an object the output does not hold — and its media stream, its artwork
    /// and its popup go with it without this rewrite naming any of them.
    ///
    /// **An array left empty is removed rather than written empty.** §7.7.3.3's Table 31 makes
    /// `/Annots` optional, so a page whose every annotation the target forbids ends up saying
    /// what it now means — this page has no annotations — rather than stating an empty array.
    fn remove_the_forbidden_annotations(
        &self,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let Some(forbidden) = self
            .forbidden_annotations
            .filter(|_| self.wants(Rewrite::ForbiddenAnnotationRemoved))
        else {
            return false;
        };
        self.take_out_of_the_page(
            out,
            applied,
            &forbidden.at,
            Rewrite::ForbiddenAnnotationRemoved,
        )
    }

    /// Gives each annotation dictionary written directly in an `/Annots` array the flags ISO 19005
    /// section 6.3.2 requires, where it states none, and says whether any was given.
    pub(super) fn flag_direct_annotations(
        &self,
        items: &mut [Object],
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let mut changed = false;
        for item in items {
            if let Object::Dictionary(dict) = item
                && super::needs_flags(self.document, dict)
            {
                dict.insert(Name::new(&b"F"[..]), Object::Integer(PRINT));
                count(applied, Rewrite::AnnotationFlags);
                changed = true;
            }
        }
        changed
    }

    /// The rewrites that write one entry of a dictionary and move nothing.
    ///
    /// The annotation `/F` an absent entry is given, and the `preserve` and `supply` answers
    /// `doc/adr/1285` and `doc/adr/1286` carry out where the object already is — each over the
    /// population its own preparation or remedy worked out, so no dictionary a requirement passed
    /// is touched.
    pub(super) fn write_entries_in_place(
        &self,
        id: ObjectId,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let mut changed = false;
        if self.wants(Rewrite::AnnotationFlags)
            && self.sites.annotations.contains(&id)
            // §12.5.3 makes the entry "an integer interpreted as one-bit flags", so a value that
            // is not one states no flags at all and is written over by the same rule as an absent
            // entry. Where the annotation does state flags, they are its producer's and stay:
            // the second sentence of section 6.3.2 is a row of its own and not this one.
            && self.document.get_key(out, "F").as_integer().is_none()
        {
            out.insert(Name::new(&b"F"[..]), Object::Integer(PRINT));
            count(applied, Rewrite::AnnotationFlags);
            changed = true;
        }
        // The same entry for annotations written directly in a page's own `/Annots` array.
        if self.wants(Rewrite::AnnotationFlags)
            && self.sites.direct_annotations.contains(&id)
            && let Some(Object::Array(items)) = out.get("Annots").cloned()
        {
            let mut items = items;
            if self.flag_direct_annotations(&mut items, applied) {
                out.insert(Name::new(&b"Annots"[..]), Object::Array(items));
                changed = true;
            }
        }
        // `doc/adr/1285`: the annotation the removal would have taken stays, with the flags ISO
        // 19005-2 section 6.3.2 asks for and every other bit its producer's. The population is the
        // removal's, so an annotation the requirement passed is never touched.
        if (self.wants(Rewrite::HiddenAnnotationShown)
            || self.wants(Rewrite::AnnotationFlagsByShape))
            && let Some(hidden) = self.hidden_annotations
            && let Some(stated) = hidden
                .kept
                .as_ref()
                .unwrap_or(&hidden.removed)
                .iter()
                .find(|row| row.at == id)
                .and_then(|row| row.flags)
        {
            out.insert(
                Name::new(&b"F"[..]),
                Object::Integer(pdf_archive::flags_permitting(stated)),
            );
            count(
                applied,
                if self.wants(Rewrite::AnnotationFlagsByShape) {
                    Rewrite::AnnotationFlagsByShape
                } else {
                    Rewrite::HiddenAnnotationShown
                },
            );
            changed = true;
        }
        // `doc/adr/1286`: the operator said the program's writing mode stands, so the dictionary
        // states it.
        if self.wants(Rewrite::WriteModeAgreed)
            && let Some(mode) = self.remedies.stated_write_modes.get(&id)
        {
            out.insert(Name::new(&b"WMode"[..]), Object::Integer(*mode));
            count(applied, Rewrite::WriteModeAgreed);
            changed = true;
        }
        // `doc/adr/1286`: the font names the embedded program where it named the program by name.
        if self.wants(Rewrite::ShippedCMapEmbedded)
            && let Some(at) = self
                .shipped_cmaps
                .and_then(|shipped| shipped.encodings.get(&id))
        {
            out.insert(Name::new(&b"Encoding"[..]), Object::Reference(*at));
            count(applied, Rewrite::ShippedCMapEmbedded);
            changed = true;
        }
        // `doc/adr/1285`: §8.10.4.1 makes the form carrying `Ref` the proxy a processor draws when
        // the imported page is not available, so the entry goes and the form stays.
        if self.wants(Rewrite::ReferenceXObjectProxied)
            && self
                .reference_xobjects
                .is_some_and(|references| references.at.contains(&id))
            && out.remove("Ref").is_some()
        {
            count(applied, Rewrite::ReferenceXObjectProxied);
            changed = true;
        }
        changed
    }

    /// Takes every annotation whose stated `/F` ISO 19005 forbids out of one page's `/Annots`.
    ///
    /// ISO 19005-2 section 6.3.2 and ISO 19005-4 section 6.3.2's second sentence, by the same act
    /// the subtype rows are answered with and for the reason `doc/adr/1234` states: the clause
    /// offers nothing to write in place of the flags, and the other future — writing the ones it
    /// asks for — shows a mark on a page its producer kept it off.
    fn remove_the_hidden_annotations(
        &self,
        out: &mut Dictionary,
        applied: &mut BTreeMap<Rewrite, usize>,
    ) -> bool {
        let Some(hidden) = self.hidden_annotations.filter(|_| {
            self.wants(Rewrite::HiddenAnnotationRemoved)
                || self.wants(Rewrite::AnnotationFlagsByShape)
        }) else {
            return false;
        };
        self.take_out_of_the_page(
            out,
            applied,
            &hidden.at,
            if self.wants(Rewrite::AnnotationFlagsByShape) {
                Rewrite::AnnotationFlagsByShape
            } else {
                Rewrite::HiddenAnnotationRemoved
            },
        )
    }
}
