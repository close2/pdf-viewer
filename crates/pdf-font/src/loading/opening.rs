//! How a font dictionary becomes a [`LoadedFont`]: the simple route, the composite route and
//! the question of whether the machine could stand in for a face the document did not embed.
//!
//! `LoadedFont::load` in the parent module picks the route; the two readers are here.

use super::{
    CodeMapping, CodeSet, DEFAULT_CID_WIDTH, FontError, LoadedFont, both_symbol_flags, class_faces,
    composite_substitute, encoding_name, parsed_program, published_pairs, to_unicode,
    type1_code_to_glyph, with_readable_font_dicts,
};
use crate::cff::CodeToGlyph;
use crate::composite::{cid_to_glyph, collection_gap, collection_table, composite_cmap};
use crate::glyph_class::Decision;
use crate::metrics::{
    NO_STRETCH, SimpleMetrics, Vertical, composite_widths, missing_width, narrow, simple_advances,
    vertical_extent,
};
use crate::name_keyed::simple_code_table;
use crate::program::{Embedded, Program, embedded_program};
use crate::runs::Runs;
use crate::substitute;
use crate::substituted::{
    substitute_code_table, substitute_encoding_names, substitute_face, symbolic_set,
};
use crate::truetype::truetype_code_table;
use crate::vertical::Downward;
use pdf_syntax::{Dictionary, Document, Object};
use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

impl LoadedFont {
    /// Loads a simple font: one byte per code.
    pub(super) fn load_simple(
        document: &Document,
        dict: &Dictionary,
        name: &str,
    ) -> Result<Self, FontError> {
        let descriptor_object = document.get_key(dict, "FontDescriptor");
        // A descriptor is required of every font except the standard 14, which is exactly
        // the case that most often needs substituting, so its absence is not an error yet.
        let descriptor = descriptor_object.as_dict();
        let embedded = match descriptor {
            Some(descriptor) => embedded_program(document, descriptor, name),
            None => Err(FontError::NotEmbedded {
                name: name.to_owned(),
            }),
        };

        let (data, program, substituted) = match embedded {
            Ok(Embedded { data, program }) => (data, program, None),
            // Nothing usable is embedded. A substitute renders the text in the wrong
            // shapes; refusing renders it not at all, and the document's own `/Widths`
            // keep the layout right either way.
            Err(FontError::NotEmbedded { .. }) => {
                let request = substitute::Request::derive(document, dict, descriptor);
                // The encoding is read before the face is chosen, because *which* face is
                // usable is decided by the characters the encoding names: see
                // `substitute_face`. It is handed on to the code table below rather than read
                // twice.
                let names = substitute_encoding_names(document, dict, request, name)?;
                let (data, format) = substitute_face(document, dict, request, &names, name);
                (data, Program::from(format), Some((request, names)))
            }
            Err(other) => return Err(other),
        };
        // The request outlives the match below, which consumes the encoding beside it.
        let requested = substituted.as_ref().map(|(request, _)| *request);
        let (type1, units_per_em) = parsed_program(program, &data, name)?;

        // Kept for text extraction: a glyph name is what a code means when a font carries
        // no `/ToUnicode`, which is common in older documents.
        let names;
        let mut notdef = None; // §9.6.5.2's substitute; only a name-keyed program has one
        // Empty unless the route below is a simple sfnt one, which is the only route §9.6.5.4
        // governs.
        let mut reader_chosen = CodeSet::default();
        // Read once: the TrueType route below asks it which character a code stands for, and
        // text extraction keeps it.
        let stated = to_unicode(document, dict);
        let mapping = match (program, substituted) {
            // A substitute shares no glyph order with the font the document meant, so its
            // glyphs are reached by what each code *means* rather than by index.
            (_, Some((request, encoded))) => {
                let (table, resolved) =
                    substitute_code_table(document, dict, request, encoded, &data, program, name)?;
                names = Some(resolved);
                CodeMapping::Named(Box::new(table))
            }
            (Program::Sfnt, None) => {
                let (table, resolved, chosen) =
                    truetype_code_table(document, dict, descriptor, &data, &stated, name)?;
                names = Some(resolved);
                reader_chosen = chosen;
                CodeMapping::Named(Box::new(table))
            }
            (Program::BareCff, None) => {
                let cff = CodeToGlyph::read(&data).map_err(|e| FontError::Malformed {
                    name: name.to_owned(),
                    detail: e.to_string(),
                })?;
                // A CID-keyed program has no glyph names for `/Encoding` to address, and
                // §9.7.4.2 puts one in a *composite* font, so a simple font naming one is
                // malformed rather than unsupported.
                let CodeToGlyph::Named(keyed) = cff else {
                    return Err(FontError::UnsupportedEncoding {
                        name: name.to_owned(),
                        encoding: "CID-keyed CFF in a simple font".to_owned(),
                    });
                };
                let (table, resolved) = simple_code_table(document, dict, &keyed, name)?;
                (names, notdef) = (Some(resolved), keyed.by_name.get(".notdef").copied());
                CodeMapping::Named(Box::new(table))
            }
            (Program::Type1, None) => {
                let keyed = type1_code_to_glyph(type1.as_ref(), name)?;
                let (table, resolved) = simple_code_table(document, dict, &keyed, name)?;
                (names, notdef) = (Some(resolved), keyed.by_name.get(".notdef").copied());
                CodeMapping::Named(Box::new(table))
            }
        };

        let metrics = SimpleMetrics {
            substituted: requested,
            names: names.as_ref(),
            data: &data,
            program,
            mapping: &mapping,
            units_per_em,
        };
        let (widths, stretch) = simple_advances(document, dict, metrics);
        let default_width = missing_width(document, descriptor);

        Ok(Self {
            data,
            program,
            font_dicts: None, // name-keyed, so there are no Font DICTs to replace
            type1,
            mapping,
            widths: Runs::from_map(&widths),
            default_width,
            extent: vertical_extent(document, descriptor),
            // §9.2.4: a second set of metrics "is available only for composite fonts".
            vertical: None,
            units_per_em,
            stretch,
            substituted: requested.is_some(),
            published_pairs: requested.and_then(|_| published_pairs(document, dict)),
            to_unicode: stated,
            collection: None,
            symbolic_set: requested.and_then(|request| symbolic_set(request.family)),
            glyph_names: names,
            notdef,
            symbolic_flags_conflict: both_symbol_flags(document, descriptor),
            reader_chosen,
            // §9.8.3.3's `/FD` is Table 122's, an entry of a `CIDFont`'s descriptor, and §9.8.3
            // says so: a simple font's descriptor has no glyph classes to override anything for.
            class_of: Decision::default(),
            classes: Vec::new(),
            outlines: Mutex::new(BTreeMap::new()),
            codes_by_character: OnceLock::new(),
            agl_by_code: OnceLock::new(),
            hinting: OnceLock::new(),
            program_by_glyph: OnceLock::new(),
        })
    }

    /// Whether this machine offers a face the composite font `dict` names could be drawn from.
    ///
    /// §9.7.4.2 leaves a composite font whose program the document did not embed reachable only
    /// by *character* — "CIDs shall not participate in glyph selection" — so its stand-in has to
    /// answer a `cmap` lookup, which an `sfnt` face on this machine does and the compiled-in
    /// name-keyed CFF faces cannot. A machine offering none therefore draws no such font at all,
    /// and that is a fact about the machine rather than about the document.
    ///
    /// **Which is why this is public: it is the question a test has to ask before it measures
    /// anything through such a font.** A fixture that is a composite font with no `/FontFile`
    /// measures this machine's font collection on every assertion it makes, and a suite that
    /// cannot tell "no face is installed" from "the code under test broke" is measuring the
    /// instrument (`doc/habits/tests-gates-and-reports.md`). The answer is the *machine's*, so a
    /// test that gets `false` says so and skips, and one that gets `true` fails loudly like any
    /// other. It asks the same search [`LoadedFont::load`] runs — [`composite_substitute`] — so
    /// there is no second matcher to disagree with the first (ADR 1154).
    ///
    /// `true` for anything that is not a composite font with a descendant, and for one whose
    /// program the document embedded: neither has a substitute to look for, so neither is a
    /// reason to skip.
    #[must_use]
    pub fn machine_offers_a_substitute(document: &Document, dict: &Dictionary) -> bool {
        let descendants = document.get_key(dict, "DescendantFonts");
        let Some(descendant) = descendants
            .as_array()
            .and_then(<[Object]>::first)
            .map(|item| document.resolve(item))
            .and_then(|item| item.as_dict().cloned())
        else {
            return true;
        };
        let descriptor_object = document.get_key(&descendant, "FontDescriptor");
        let descriptor = descriptor_object.as_dict();
        // The resource name only ever reaches an error this discards, so the empty one is the
        // honest argument rather than a borrowed name that would read as a claim about a font.
        if descriptor.is_some_and(|descriptor| embedded_program(document, descriptor, "").is_ok()) {
            return true;
        }
        composite_substitute(document, &descendant, descriptor)
            .1
            .is_some()
    }

    /// Loads a composite (Type0) font.
    pub(super) fn load_composite(
        document: &Document,
        dict: &Dictionary,
        name: &str,
    ) -> Result<Self, FontError> {
        let cmap = composite_cmap(document, dict, name)?;
        // §9.7.5.1: "A CMap shall specify the writing mode … for any CIDFont with which the
        // CMap is combined", and §9.2.4 makes the writing mode the choice between two sets of
        // metrics rather than anything about the glyphs.
        let vertical = cmap.wmode() == 1;

        let descendants = document.get_key(dict, "DescendantFonts");
        let descendant = descendants
            .as_array()
            .and_then(<[Object]>::first)
            .map(|item| document.resolve(item))
            .and_then(|item| item.as_dict().cloned())
            // §9.7.6.1's Table 119 requires a one-element array naming the CIDFont dictionary,
            // and §9.7.6.2 indexes that array to select one. A file may state the entry and
            // still select nothing — `issue12823.pdf` writes `/DescendantFonts [ null ]`, and
            // §7.3.9's null is not a dictionary — so the fault is the dictionary's and not a
            // program's. [`FontError::MalformedDictionary`] says which.
            .ok_or_else(|| FontError::MalformedDictionary {
                name: name.to_owned(),
                detail: "/DescendantFonts selects no CIDFont dictionary, which Table 119 requires \
                         it to (§9.7.6.1)"
                    .to_owned(),
            })?;

        let descriptor_object = document.get_key(&descendant, "FontDescriptor");
        let descriptor = descriptor_object.as_dict();

        let embedded = match descriptor {
            Some(descriptor) => embedded_program(document, descriptor, name),
            None => Err(FontError::NotEmbedded {
                name: name.to_owned(),
            }),
        };

        // §9.10.2's first and third methods, read once each: the producer's `/ToUnicode`, keyed
        // by code, and the collection's `registry-ordering-UCS2` table, keyed by CID. Both serve
        // the readback of every composite font and, for a substituted one, glyph selection too.
        let to_unicode = to_unicode(document, dict);
        let collection = collection_table(document, &descendant);

        let (data, program, substituted) = match embedded {
            Err(FontError::NotEmbedded { .. }) => {
                // A CID is meaningless outside the font that defined it — it is an index into
                // that font's glyphs, not a character — so a substitute can only be reached
                // through what the codes *mean*. `/ToUnicode` is the only thing that says so,
                // and a composite font without one cannot be substituted at all. §9.7.4.2 says
                // the same thing from the other side: with the program absent, "CIDs shall not
                // participate in glyph selection", and a `/CIDToGIDMap` "shall be ignored, since
                // it is not meaningful to refer to glyph indices in an external font program".
                // §9.10.2's first method, then its third. The third is reachable because this
                // binary carries the collections' own tables (ADR 0140), so a CJK font without a
                // `/ToUnicode` is substituted through its `/CIDSystemInfo`.
                //
                // **And where the CMap is one of the two identity ones, §9.7.5.2 has already
                // forbidden the file rather than left the reader a choice**: "The Identity-H and
                // Identity-V CMaps shall not be used with a non-embedded font. Only standardized
                // character sets may be used." A document that states `/Identity-H` over a
                // descendant with no font program has broken that sentence, so the refusal below
                // is not a gap in this reader — it is the one honest answer to a combination the
                // standard says shall not exist. ADR 0433 measures what the other renderers do
                // with it instead, and they do four different things.
                //
                // **Both tables are kept, and the choice between them is made per code.** The
                // clause ranks its methods and a method that "fail[s] to produce a Unicode value"
                // for one code has failed for that code only, so a `/ToUnicode` that omits a code
                // the collection names has not settled the font's route — `substituted_character`
                // asks the second table where the first says nothing (ADR 1002). What is refused
                // here is the font with *neither*, which is the one with no question left to ask.
                //
                // **This question is asked before the machine's is, and the order is the whole
                // point of it (ADR 1154).** Both refusals are [`FontError::NoSubstitute`], and
                // only one of them is about the file: whether a code has a character to be
                // reached by is decided by the bytes, and whether a face covering that script is
                // installed is decided by the machine. Asking the machine first made the sentence
                // a reader is shown about a file that broke §9.7.5.2 depend on which fonts that
                // reader happened to have, and named the machine for a fault the producer
                // committed.
                if to_unicode.is_empty() && collection.is_none() {
                    // **[`FontError::NoSubstitute`] rather than
                    // [`FontError::UnsupportedEncoding`], and the encoding is why**: `Identity-H`
                    // is read perfectly well here — [`composite_cmap`] built `cmap` out of it
                    // above — so nothing about the *encoding* is unsupported. What failed is
                    // reaching a substitute through it, which is the case that variant's own doc
                    // comment describes and which the sibling refusal below already uses.
                    // [`collection_gap`] says which of its four facts this file is.
                    return Err(FontError::NoSubstitute {
                        name: name.to_owned(),
                        reason: collection_gap(
                            document,
                            dict,
                            &descendant,
                            encoding_name(document, dict).as_deref(),
                        ),
                    });
                }
                let (request, found) = composite_substitute(document, &descendant, descriptor);
                let data = found.ok_or_else(|| FontError::NoSubstitute {
                    name: name.to_owned(),
                    reason: format!(
                        "no {:?} face this machine offers can be addressed by character, which \
                         is the only way §9.7.4.2 leaves to reach a substitute for a composite \
                         font",
                        request.family
                    ),
                })?;
                (data, Program::Sfnt, true)
            }
            Ok(Embedded { data, program }) => (data, program, false),
            Err(other) => return Err(other),
        };
        let (data, font_dicts) = with_readable_font_dicts(data, program);
        // Kept parsed for the same measured reason a simple font keeps it (`type1::Program`),
        // and read for its scale the same way: a Type 1 program states it in a `/FontMatrix`
        // rather than in an `sfnt` header, so `FontRef` cannot be asked.
        let (type1, units_per_em) = parsed_program(program, &data, name)?;

        let (class_of, classes) = class_faces(document, &descendant, substituted, vertical, name);

        let mapping = if substituted {
            // What reaches a substituted composite font's glyphs is §9.10.2's first and third
            // methods, and the font with neither is refused above — beside the machine's own
            // search rather than after it, so that the file's fault is named by the file's
            // sentence (ADR 1154).
            CodeMapping::Substituted {
                cmap: Box::new(cmap),
                // §9.7.5.1's NOTE: a vertical `CMap` names *different CIDs*, so a substituted
                // face has to be asked for a different glyph too.
                downward: Downward::read(document, &descendant, &data, vertical),
            }
        } else {
            CodeMapping::Composite {
                cmap: Box::new(cmap),
                glyphs: cid_to_glyph(document, &descendant, &data, program, name)?,
            }
        };

        let default_width = document
            .get_key(&descendant, "DW")
            .as_number()
            .map_or(DEFAULT_CID_WIDTH, narrow);

        Ok(Self {
            data,
            program,
            font_dicts,
            // §9.9's Table 124 gives a CIDFont `/FontFile2` and `/FontFile3` and never
            // `/FontFile` — but a descriptor writing one is read rather than refused; see
            // the `mapping` above for the clause's own analogy that decides how.
            type1,
            mapping,
            substituted,
            // §9.6.2.2's fourteen are simple Type 1 fonts.
            published_pairs: None,
            to_unicode,
            collection,
            // §9.6.5.1 gives the two symbolic standard-14 fonts *simple* built-in encodings, and
            // a composite font has none of them.
            symbolic_set: None,
            glyph_names: None,
            // A composite font's substitute is §9.7.6.3's CID 0, applied in `glyph_for`.
            notdef: None,
            // §9.6.5.4 is a *simple* font's subclause: a composite font's codes reach glyphs
            // through §9.7.6.2 and §9.7.4.2, neither of which offers the processor a choice.
            symbolic_flags_conflict: false,
            reader_chosen: CodeSet::default(),
            widths: composite_widths(document, &descendant),
            default_width,
            extent: vertical_extent(document, descriptor),
            vertical: vertical.then(|| Vertical::read(document, &descendant)),
            units_per_em,
            // A composite font's substitute is addressed by *character* through `/ToUnicode`
            // (§9.7.4.2), so there is no code whose `/W` entry and whose face advance are two
            // statements about one glyph — which is the comparison `substitute_stretch` is.
            // ADR 0358 states the restriction and what would lift it.
            stretch: NO_STRETCH,
            class_of,
            classes,
            outlines: Mutex::new(BTreeMap::new()),
            codes_by_character: OnceLock::new(),
            agl_by_code: OnceLock::new(),
            hinting: OnceLock::new(),
            program_by_glyph: OnceLock::new(),
        })
    }
}
