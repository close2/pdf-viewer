//! The glyph a face draws for a character *written downwards*.
//!
//! ISO 32000-2 §9.7.5.1 says why this module exists, in a NOTE under the sentence that makes a
//! `CMap` carry a writing mode:
//!
//! > Writing mode is specified as part of the CMap because, in some cases, different shapes are
//! > used when writing horizontally and vertically. In such cases, the horizontal and vertical
//! > variants of a CMap specify different CIDs for a given character code.
//!
//! So a vertical `CMap` selects *different glyphs*, not only different metrics — §9.2.4's `/W2`
//! and `/DW2` are the metrics half and [`crate::metrics::Vertical`] has had them since the
//! thirty-sixth session. A document that has chosen those CIDs and embedded its font is drawn
//! correctly by that alone, because the CID reaches the producer's own glyph.
//!
//! # Why a *substituted* font loses the shapes and this is what puts them back
//!
//! §9.7.4.2 makes a substituted composite font reachable only by character, so this tree takes a
//! CID to a Unicode value through the collection's own `registry-ordering-UCS2` table (§9.10.2's
//! third method) and asks the chosen face's `cmap` for it. **That table is keyed to the character
//! and not to the form**: Adobe-Japan1's CID 7911 is the vertical LEFT CORNER BRACKET and CID 686
//! is the horizontal one, and `Adobe-Japan1-UCS2` maps both to U+300C, because Unicode has one
//! code point for the character. The producer's choice of form is thrown away at that step, and
//! the page comes out with its brackets lying on their sides and its full stops in the middle of
//! the column instead of at the top right.
//!
//! Nothing in ISO 32000-2 says which glyph a *substitute* should draw — §9.5's NOTE 5 puts the
//! choice of face outside the standard altogether — so both halves of what follows are documented
//! choices in a place the standard leaves open. What they are *not* is guesses: each half is a
//! published table read for what it says.
//!
//! # The two halves
//!
//! **Which CIDs are vertical forms is the character collection's own statement**, and Table 116
//! publishes it twice over: of the `CMap`s it names, "those ending in V specify vertical writing
//! mode" and are otherwise their horizontal twins. So for the collection's Unicode pair — `UniJIS-UCS2-H`
//! and `UniJIS-UCS2-V` for Adobe-Japan1 — a character whose vertical `CMap` sends it to a
//! different CID than its horizontal one *has* a vertical form, and that CID *is* it. Both files
//! are already compiled in ([`crate::predefined`]), and [`crate::predefined::is_vertical_form`]
//! is that comparison. Nothing is hard-coded about which characters rotate.
//!
//! **Which glyph of the face is that form is the face's own statement**, and OpenType's registered
//! `vert` and `vrt2` features are where a face makes it: both are defined as substituting a
//! glyph's vertical form for its horizontal one, and a CJK face that supports vertical writing at
//! all carries one. This module reads them straight out of `GSUB` — it does not shape, which
//! `doc/stack.md` and this crate's own module comment rule out and which is not needed: the
//! question here is about one glyph and has one answer.
//!
//! # Two things this deliberately does not do
//!
//! **It does not select by script or language.** A `vert` feature is looked up in the feature
//! list by tag alone rather than through a `ScriptList` and a `LangSys`. Choosing a script means
//! deciding what language the run is in, which a PDF content stream does not say and which
//! shaping is for; and the feature means the same thing under every script that registers it, so
//! the choice can only change *whether* an answer is found, never which one.
//!
//! **It does not report a face with no vertical form.** That is the same shortfall as a face with
//! no glyph for a character at all, which ADR 0152 priced and this tree deliberately counts
//! rather than reports: the page draws the producer's character in the producer's place, in a
//! shape the substitute had. Calling that a drawing fault would take a page off the oracle's
//! judged set to say something about a face rather than about the file.

use std::collections::BTreeMap;

use pdf_syntax::{Dictionary, Document};
use skrifa::raw::TableProvider;
use skrifa::raw::tables::gsub::{SingleSubst, SubstitutionSubtables};
use skrifa::{FontRef, Tag};

use crate::composite::collection_names;
use crate::predefined;

/// What a substituted composite font in writing mode 1 needs beyond the horizontal route.
///
/// The two halves of this module's rule, resolved when the font is loaded: which collection says
/// a CID is a vertical form, and which glyph of the chosen face is that form.
#[derive(Debug)]
pub(crate) struct Downward {
    /// The descendant's `/CIDSystemInfo` registry (§9.7.3).
    registry: String,
    /// Its ordering.
    ordering: String,
    /// The face's own `vert` and `vrt2` substitutions.
    forms: VerticalForms,
}

impl Downward {
    /// The route for one substituted vertical font, or `None` where there is nothing to do.
    ///
    /// `None` for four reasons, each of which is a fact and not a failure: the `CMap` is in
    /// writing mode 0, so §9.7.5.1's NOTE is not about it; the descendant states no readable
    /// `/CIDSystemInfo` (Table 115 makes it required, so §9.10.2 step (b) has nothing to obtain
    /// either); Table 116 publishes no vertical `CMap` for the collection it does state, which
    /// includes every `Identity` ordering; or the face this machine offered states no vertical
    /// form of any glyph, which is every Latin face.
    ///
    /// The writing mode is a parameter rather than the caller's `if` so that every one of the
    /// four sits in one place, which is what makes the list above checkable.
    pub(crate) fn read(
        document: &Document,
        descendant: &Dictionary,
        data: &[u8],
        vertical: bool,
    ) -> Option<Box<Self>> {
        if !vertical {
            return None;
        }
        let (registry, ordering) = collection_names(document, descendant)?;
        if !predefined::has_vertical_forms(&registry, &ordering) {
            return None;
        }
        let font = FontRef::new(data).ok()?;
        let forms = VerticalForms::read(&font);
        (!forms.is_empty()).then(|| {
            Box::new(Self {
                registry,
                ordering,
                forms,
            })
        })
    }

    /// The glyph to draw where the producer's CID is the collection's vertical form.
    ///
    /// `character` and `glyph` are what the horizontal route already produced — §9.10.2's third
    /// method and the face's `cmap` — and `cid` is what the file wrote. Both halves have to
    /// agree: the collection has to call this CID that character's vertical form, and the face
    /// has to state one for the glyph. `None` where either does not, and then the caller draws
    /// what it had.
    pub(crate) fn form_of(&self, character: char, cid: u32, glyph: u16) -> Option<u16> {
        predefined::is_vertical_form(&self.registry, &self.ordering, character, cid)
            .then(|| self.forms.of(glyph))
            .flatten()
    }
}

/// The vertical form of each glyph the face states one for.
///
/// Empty for a face with no `GSUB`, no `vert` and no `vrt2` — which is every Latin face and most
/// of the CJK ones that are not meant for vertical setting.
#[derive(Debug, Default)]
pub(crate) struct VerticalForms {
    by_glyph: BTreeMap<u16, u16>,
}

impl VerticalForms {
    /// Reads a face's `vert` and `vrt2` single substitutions.
    ///
    /// **`vrt2` is read second and therefore wins.** OpenType defines `vert` as the subset that
    /// rotates only what has to be rotated and `vrt2` as the full set for a face laid out
    /// vertically throughout; where a face states both for one glyph, the second is the one it
    /// drew for vertical setting. This is a choice — the standard says nothing about either
    /// feature — and it costs nothing to state, because a face with only one of the two is
    /// unaffected either way.
    ///
    /// Only *single* substitutions are read (`GSUB` lookup type 1). A `vert` feature is a
    /// one-glyph-for-one-glyph mapping by construction, and a lookup of any other type would be
    /// a rule about a sequence, which is shaping and which nothing here has a sequence for.
    ///
    /// # What it costs, and where
    ///
    /// One pass over the feature's coverage tables, built once per loaded font and only for a
    /// composite font that (a) has no embedded program, (b) is drawn in writing mode 1, and (c)
    /// names a registered character collection. `CLAUDE.md`'s startup rule is met by the
    /// condition rather than by laziness: a document that opens no such font never reads a byte
    /// of `GSUB`. The map is bounded by the coverage tables, whose entries are glyph indices and
    /// therefore at most 65 536 pairs however malformed the face.
    pub(crate) fn read(font: &FontRef) -> Self {
        let mut by_glyph = BTreeMap::new();
        let Ok(gsub) = font.gsub() else {
            return Self { by_glyph };
        };
        let (Ok(features), Ok(lookups)) = (gsub.feature_list(), gsub.lookup_list()) else {
            return Self { by_glyph };
        };
        for tag in [Tag::new(b"vert"), Tag::new(b"vrt2")] {
            for record in features
                .feature_records()
                .iter()
                .filter(|record| record.feature_tag() == tag)
            {
                let Ok(feature) = record.feature(features.offset_data()) else {
                    continue;
                };
                for index in feature.lookup_list_indices() {
                    let Ok(lookup) = lookups.lookups().get(usize::from(index.get())) else {
                        continue;
                    };
                    let Ok(SubstitutionSubtables::Single(subtables)) = lookup.subtables() else {
                        continue;
                    };
                    for subtable in subtables.iter().flatten() {
                        collect_single(&subtable, &mut by_glyph);
                    }
                }
            }
        }
        Self { by_glyph }
    }

    /// The glyph to draw in place of `glyph` when the text runs downwards.
    pub(crate) fn of(&self, glyph: u16) -> Option<u16> {
        self.by_glyph.get(&glyph).copied()
    }

    /// Whether the face stated any vertical form at all.
    pub(crate) fn is_empty(&self) -> bool {
        self.by_glyph.is_empty()
    }
}

/// One single-substitution subtable, in either of its two formats.
///
/// Format 1 states a delta added to every covered glyph index and format 2 an array parallel to
/// the coverage table; both are one glyph for one glyph, which is why only these two are read.
fn collect_single(subtable: &SingleSubst<'_>, into: &mut BTreeMap<u16, u16>) {
    match subtable {
        SingleSubst::Format1(table) => {
            let Ok(coverage) = table.coverage() else {
                return;
            };
            let delta = table.delta_glyph_id();
            for glyph in coverage.iter() {
                // OpenType states the sum modulo 65 536, which is what `wrapping_add` is.
                let substitute = glyph.to_u16().wrapping_add(delta.cast_unsigned());
                let _ = into.insert(glyph.to_u16(), substitute);
            }
        }
        SingleSubst::Format2(table) => {
            let Ok(coverage) = table.coverage() else {
                return;
            };
            let substitutes = table.substitute_glyph_ids();
            for (at, glyph) in coverage.iter().enumerate() {
                let Some(substitute) = substitutes.get(at) else {
                    // A coverage table longer than the array it indexes is a malformed face;
                    // the pairs that do line up are still the face's own statement.
                    break;
                };
                let _ = into.insert(glyph.to_u16(), substitute.get().to_u16());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::VerticalForms;

    /// A face with no `GSUB` at all states no vertical form, and says so rather than erroring.
    ///
    /// `crate::standard`'s compiled-in fourteen are the faces this crate always has: none of
    /// them is a CJK face and none carries `vert`, so this is the answer for every one of them.
    #[test]
    fn a_face_with_no_vertical_feature_states_no_form() {
        let request = crate::substitute::Request {
            family: crate::substitute::Family::SansSerif,
            bold: false,
            italic: false,
            standard: true,
        };
        let (data, format) = crate::substitute::find(request);
        assert_eq!(
            format,
            crate::substitute::Format::Sfnt,
            "the compiled-in Helvetica is an sfnt, which is what has a GSUB to look in"
        );
        let font = skrifa::FontRef::new(&data).expect("the compiled-in face parses");
        assert!(
            VerticalForms::read(&font).is_empty(),
            "a Latin face has no vert or vrt2 feature to read"
        );
    }
}
