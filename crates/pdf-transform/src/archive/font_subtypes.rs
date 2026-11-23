//! A font dictionary that states no `/Subtype`, given the one the program it embeds decides
//! (ADR 3409).
//!
//! ISO 32000-2 §9.6.2.1's Table 109, §9.6.4's Table 110, §9.7.4.1's Table 115 and §9.7.6.1's
//! Table 119 make `/Subtype` required, and ISO 19005-2 section 6.2.11.2 and ISO 19005-4 section
//! 6.2.10.2 hold every font to them (`pdf_archive`'s `fonts/font-dictionary-states-its-subtype`).
//! Unlike `/Type`, the entry admits several values, so no table and no position decides it alone.
//! **The program decides it**, where there is one: §9.9's Table 124 files each program format
//! under one descriptor key and, for `/FontFile3`, one stream `/Subtype`, and each of those rows
//! names the font kinds that program goes under. So the value written is a reading of the
//! program's own filing, never this converter's choice:
//!
//! - `/FontFile`, a Type 1 program: `Type1`;
//! - `/FontFile2`, a TrueType program: `TrueType`, or `CIDFontType2` for a dictionary a Type 0
//!   font's `/DescendantFonts` names;
//! - `/FontFile3 /Type1C`: `Type1`; `/FontFile3 /CIDFontType0C`: `CIDFontType0`;
//! - `/FontFile3 /OpenType`: by its outlines, a `CFF ` table making it `Type1` (`CIDFontType0`
//!   as a descendant) and `glyf` outlines `TrueType` (`CIDFontType2`).
//!
//! **A font that embeds no program** is the one `fonts/font-programs-embedded` fails where it is
//! rendered, and the face the substitution embeds for it decides its kind there: `fonts`'
//! `described` route writes the `/Subtype` of the face it embeds into the entries the font gains.
//! This module checks that every such font is one the substitution wrote, and writes nothing for
//! it.
//!
//! **A font no face was embedded for, that embeds no program and states no descriptor, and that
//! names one of §9.6.2.2's fourteen** is given `Type1`: the substitution embeds a face for every
//! such font a page renders, so this one draws nothing, the fourteen are Type 1 fonts, and their
//! metrics are the ones this viewer lays the font out by already, so no position moves either.
//!
//! **Where the substitution photographed a page**, a font this module cannot decide is left to
//! the output's own re-check rather than refused here: the page it stood on may no longer name it,
//! and if it still does the output fails the row and is not written.
//!
//! Refused by name otherwise: a font embedding no program that the substitution did not reach and
//! that names no standard font, a Type 1 program carrying multiple master axes (Table 124
//! files it under `/FontFile` with the plain Type 1 programs, so the filing does not decide
//! between `Type1` and `MMType1`), a Type 1 or `Type1C` program named as a descendant, which no
//! `CIDFont` kind takes, and a font written directly inside another object that the substitution
//! did not write out as an object of its own.

use std::collections::{BTreeMap, BTreeSet};

use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId};

use super::decision::Because;
use super::fonts::Substitutes;

/// The requirement this module answers, by the identifier the validator reports it under.
pub(super) const ROW: &str = "fonts/font-dictionary-states-its-subtype";

/// Why a font's `/Subtype` is not stated.
const NOT_DECIDED: &str = "a font dictionary states no Subtype ISO 32000-2's font dictionary \
     tables name, and this conversion writes one only where the font program it embeds decides \
     it, by the descriptor key and stream Subtype \u{a7}9.9's Table 124 files the program under, \
     or where it embeds a face for the font itself; this font embeds no program the filing \
     decides a kind for, and no face was embedded for it";

/// The `/Subtype` each font object is given, keyed by the object, in the source's numbering.
#[derive(Debug, Default)]
pub(super) struct Stated {
    /// Each font's kind, as its program's filing decides it.
    pub(super) subtypes: BTreeMap<ObjectId, Name>,
}

/// Every font [`ROW`] failed at, with the kind its program decides, or why none is decided.
///
/// `substitutes` is what the font substitution wrote, where it ran: a font it embedded a face for
/// is given its `/Subtype` there, so it is checked and left alone here.
///
/// # Errors
///
/// [`NOT_DECIDED`] where any failing font is outside what the module comment lists; a requirement
/// half answered is a requirement refused.
pub(super) fn stated(
    document: &Document,
    input: &pdf_archive::Report,
    substitutes: Option<&Substitutes>,
) -> Result<Stated, Because> {
    let (named, elsewhere) = super::sites::objects_named(input, &[ROW])?;
    let photographed = substitutes.is_some_and(|substitutes| !substitutes.photographs.is_empty());
    if elsewhere {
        let reported = super::sites::findings_at_no_object(input, ROW);
        let written_out = substitutes.map_or(0, |substitutes| {
            substitutes
                .direct
                .iter()
                .filter(|(source, at)| {
                    !states_a_kind(document, source)
                        && substitutes
                            .written
                            .get(at)
                            .and_then(Object::as_dict)
                            .is_some_and(|written| states_a_kind(document, written))
                })
                .count()
        });
        if written_out != reported && !photographed {
            return Err(Because::NotBuiltYet(NOT_DECIDED));
        }
    }
    let descendants = descendants(document, &named);
    let mut stated = Stated::default();
    for id in named {
        let resolved = document.get(id);
        let Object::Dictionary(font) = &resolved else {
            return Err(Because::NotBuiltYet(NOT_DECIDED));
        };
        if let Some(kind) = program_kind(document, font, descendants.contains(&id)) {
            stated.subtypes.insert(id, Name::new(kind.as_bytes()));
        } else if substitution_states_it(document, id, substitutes) {
            // The face embedded for it decides the kind, and the substitution writes it.
        } else if names_a_standard_font(document, font) {
            stated.subtypes.insert(id, Name::new(&b"Type1"[..]));
        } else if !photographed {
            return Err(Because::NotBuiltYet(NOT_DECIDED));
        }
    }
    Ok(stated)
}

/// Whether a font stating no descriptor names one of §9.6.2.2's fourteen by its `/BaseFont`.
fn names_a_standard_font(document: &Document, font: &Dictionary) -> bool {
    document.get_key(font, "FontDescriptor").is_null()
        && document
            .get_key(font, "BaseFont")
            .as_name()
            .and_then(|name| std::str::from_utf8(name.as_bytes()).ok())
            .is_some_and(pdf_font::standard::is_standard_name)
}

/// Whether a dictionary states a `/Subtype` one of the font dictionary tables names.
fn states_a_kind(document: &Document, font: &Dictionary) -> bool {
    document
        .get_key(font, "Subtype")
        .as_name()
        .is_some_and(|name| {
            matches!(
                name.as_bytes(),
                b"Type1"
                    | b"MMType1"
                    | b"Type3"
                    | b"TrueType"
                    | b"CIDFontType0"
                    | b"CIDFontType2"
                    | b"Type0"
            )
        })
}

/// Whether the substitution embedded a face for the font object `id` and wrote a kind with it.
fn substitution_states_it(
    document: &Document,
    id: ObjectId,
    substitutes: Option<&Substitutes>,
) -> bool {
    substitutes.is_some_and(|substitutes| {
        substitutes
            .described
            .get(&id)
            .is_some_and(|entries| states_a_kind(document, entries))
            || substitutes
                .written
                .get(&id)
                .and_then(Object::as_dict)
                .is_some_and(|written| states_a_kind(document, written))
    })
}

/// Which of `fonts` a Type 0 font's `/DescendantFonts` names, so that a program's filing is read
/// as Table 115's `CIDFont` kinds for them (§9.7.6.1's Table 119).
fn descendants(document: &Document, fonts: &BTreeSet<ObjectId>) -> BTreeSet<ObjectId> {
    let mut named = BTreeSet::new();
    if fonts.is_empty() {
        return named;
    }
    for number in document.xref().object_numbers() {
        let object = document.get(document.defined_id(number));
        let Object::Dictionary(dict) = &object else {
            continue;
        };
        if let Object::Array(elements) = document.get_key(dict, "DescendantFonts") {
            named.extend(elements.iter().filter_map(|element| match element {
                Object::Reference(id) if fonts.contains(id) => Some(*id),
                _ => None,
            }));
        }
    }
    named
}

/// The kind Table 124's filing of the descriptor's program decides, or `None` where it embeds no
/// program or the filing decides no single kind.
fn program_kind(document: &Document, font: &Dictionary, descendant: bool) -> Option<&'static str> {
    let descriptor = document.get_key(font, "FontDescriptor");
    let descriptor = descriptor.as_dict()?;
    if let Object::Stream(program) = document.get_key(descriptor, "FontFile") {
        let bytes = document.decoded_stream_data(&program)?;
        let multiple_master = bytes
            .windows(b"/BlendAxisTypes".len())
            .any(|window| window == b"/BlendAxisTypes");
        return (!descendant && !multiple_master).then_some("Type1");
    }
    if let Object::Stream(_) = document.get_key(descriptor, "FontFile2") {
        return Some(if descendant {
            "CIDFontType2"
        } else {
            "TrueType"
        });
    }
    let Object::Stream(program) = document.get_key(descriptor, "FontFile3") else {
        return None;
    };
    let filed = document.get_key(&program.dict, "Subtype");
    match filed.as_name().map(Name::as_bytes)? {
        b"Type1C" => (!descendant).then_some("Type1"),
        b"CIDFontType0C" => descendant.then_some("CIDFontType0"),
        b"OpenType" => {
            let bytes = document.decoded_stream_data(&program)?;
            let compact = pdf_font::embed::has_cff_outlines(&bytes);
            Some(match (compact, descendant) {
                (true, false) => "Type1",
                (true, true) => "CIDFontType0",
                (false, false) => "TrueType",
                (false, true) => "CIDFontType2",
            })
        }
        _ => None,
    }
}
