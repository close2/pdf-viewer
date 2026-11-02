//! A `Type0` font over a CID-keyed CFF program whose shown CIDs the program's charset does not
//! hold, given a glyph for each such CID by writing the program.
//!
//! ISO 32000-2 §9.7.4.2 reaches a Type 0 `CIDFont`'s glyph through the program's charset, and a
//! CID the charset does not hold has no glyph; §9.7.6.3 then substitutes "the glyph for CID 0
//! (which shall be present)", the `CIDFont`'s `.notdef`, which ISO 19005-2 section 6.2.11.8 and
//! ISO 19005-4 section 6.2.10.9 forbid a text-showing operator to reference. **The charset is
//! extended with the CID itself**: `pdf_font::cff::with_cids_given` appends, for each such CID, a
//! copy of glyph 0's charstring — the outline this reader drew for it (`doc/questions/A694` item
//! 3) — at the width the font dictionary's `/W` or `/DW` states for it (ISO 19005-2 section
//! 6.2.11.5), under glyph 0's Font DICT. A Type 0 `CIDFont` has no `/CIDToGIDMap` (Table 115 makes
//! it a Type 2 entry), so neither the map nor the `CMap` nor any content stream changes; the
//! descendant names a new descriptor, which names the new program (`doc/adr/2598`).
//!
//! **One glyph per CID**, since a charset states one CID per glyph; a CID shown at two widths is
//! refused. **A code whose own CID is 0 is sent to a fresh CID** (`doc/adr/2557`, `doc/adr/2622`):
//! §9.7.4.2 makes CID 0 the `.notdef` character whatever glyph draws it, so the encoding
//! [`super::fresh_cid`] restates sends the code to a CID past every CID the charset holds, and
//! that CID is given the copy of glyph 0 at CID 0's width like any other. A code that
//! reached glyph 0 through §9.7.6.3's notdef mappings rather than its own CID is refused too,
//! because no glyph keyed by a CID the code does not have would be the one it reaches.
//!
//! **Nothing is written that was not proved**: the writer reads its own tables back (trap 301),
//! and the font is then loaded again from the descendant, descriptor and program held in memory,
//! and every code the pages show must draw the outline it drew, at the width it had, by the glyph
//! it reached before or by the copy given its CID with the copy's advance its width, and none
//! may be drawn as CID 0.

use std::collections::BTreeMap;

use pdf_font::cff::CidToGive;
use pdf_font::{Code, LoadedFont};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};

use super::super::prepare::Spare;
use super::super::sites::{dictionary_of, loaded_as_stated};
use super::cid_notdef_glyph::{FreshWritten, cid_set_restated, program_in_budget};
use super::fresh_cid::{self, FreshCid};

/// Why a `Type0` font over a CID-keyed CFF program named by the `.notdef` rows keeps the refusal.
pub(in crate::archive) const CID_CFF_NOT_GIVEN: &str = "this file shows a CID of a CIDFontType0 \
     font that the charset of its embedded CID-keyed CFF program does not hold, so the code \
     reaches the glyph for CID 0, the .notdef glyph; this conversion gives each such CID a glyph \
     of its own, a copy of the .notdef glyph's outline at the width the font states for the CID, \
     with the CID added to the charset. It does that only where the descendant CIDFont is an \
     object of its own, every string the pages show in the font is known, each code on .notdef \
     reached it by its own CID, each CID is shown at one width, the program can be written and \
     read back, and the font loads again with every shown code drawing what it drew: one of \
     those does not hold here";

/// Why a `Type0` font over a CID-keyed CFF program that shows CID 0 keeps the refusal.
pub(in crate::archive) const CID_CFF_0_SHOWN: &str = "this file shows CID 0 of a CIDFontType0 \
     font, and ISO 32000-2 section 9.7.4.2 makes CID 0 the CIDFont's own .notdef character, so \
     the code references .notdef — which ISO 19005-2 section 6.2.11.8 and ISO 19005-4 section \
     6.2.10.9 forbid a text-showing operator to do. This conversion sends such a code to a fresh \
     CID given a copy of the .notdef glyph, by restating the font's Encoding as an embedded CMap \
     and extending the program's charset; it does that only where the Encoding of every font \
     showing CID 0 is Identity-H, each has a ToUnicode CMap or a character collection ISO 32000-2 \
     section 9.10.2 does not read by CID, a 16-bit CID is left past every CID the font uses, the \
     program can be written and read back, and the font loads again with every shown code \
     drawing what it drew: one of those does not hold here";

/// The objects one CID-keyed CFF font's glyphs are given by.
#[derive(Debug)]
pub(in crate::archive) struct CidCffGlyphsGiven {
    /// The descendant `CIDFont` dictionary that names the new descriptor.
    pub(in crate::archive) descendant: ObjectId,
    /// The descriptor the descendant names from now on.
    pub(in crate::archive) descriptor: ObjectId,
    /// The encoding and widths that send a shown CID 0 to a fresh CID, where one was shown.
    pub(in crate::archive) fresh: Option<FreshWritten>,
    /// The new objects: the program, the descriptor, where the descriptor stated one the
    /// `/CIDSet`, and the embedded `CMap` of [`Self::fresh`].
    pub(in crate::archive) written: Vec<(ObjectId, Object)>,
}

/// Whether a font dictionary is a `Type0` font whose one descendant is a `CIDFontType0`, written
/// as an object of its own, whose descriptor embeds a bare CID-keyed CFF as `/FontFile3` of
/// `/Subtype /CIDFontType0C` — the font [`given`] can carry.
pub(in crate::archive) fn is_cid_cff(document: &Document, dict: &Dictionary) -> bool {
    descendant_of(document, dict).is_some()
}

/// The descendant a font [`is_cid_cff`] admits, by object number: the `Type0` fonts that share
/// one are given their glyphs together, since the program and its charset are the descendant's.
pub(in crate::archive) fn descendant_id(
    document: &Document,
    dict: &Dictionary,
) -> Option<ObjectId> {
    descendant_of(document, dict).map(|(at, ..)| at)
}

/// Whether any code in the strings `shown` is given CID 0 by the `Type0` font's own `CMap`.
///
/// Not [`super::cid_notdef_glyph::shows_cid_0`]'s question: in a `CIDFontType0` a CID the charset
/// does not hold is *drawn* as CID 0 by §9.7.6.3's fallback, which is the code this module gives a
/// glyph; what keeps the refusal is a code whose own CID is 0.
pub(in crate::archive) fn shows_own_cid_0<'a>(
    document: &Document,
    dict: &Dictionary,
    mut shown: impl Iterator<Item = &'a [u8]>,
) -> bool {
    loaded_as_stated(document, dict).is_some_and(|font| {
        shown.any(|text| {
            font.decode(text)
                .into_iter()
                .any(|code| font.cid(code) == Some(0))
        })
    })
}

/// The descendant's object number, its dictionary, its descriptor and the program's reference,
/// for a font [`is_cid_cff`] admits.
///
/// The descendant is required to be an indirect object because that is where the new descriptor
/// is named: §9.7.1's `/DescendantFonts` is a one-element array, and a dictionary written inside
/// it would have to be rewritten inside the `Type0` font instead.
fn descendant_of(
    document: &Document,
    dict: &Dictionary,
) -> Option<(ObjectId, Dictionary, Dictionary, ObjectId)> {
    if document.get_key(dict, "Subtype").as_name()?.as_bytes() != b"Type0" {
        return None;
    }
    let descendants = document.resolve(dict.get("DescendantFonts")?);
    let [only] = descendants.as_array()? else {
        return None;
    };
    let at = only.as_reference()?;
    let resolved = document.get(at);
    let descendant = dictionary_of(&resolved)?.clone();
    if document
        .get_key(&descendant, "Subtype")
        .as_name()?
        .as_bytes()
        != b"CIDFontType0"
    {
        return None;
    }
    let resolved = document.resolve(descendant.get("FontDescriptor")?);
    let descriptor = dictionary_of(&resolved)?.clone();
    let program = descriptor.get("FontFile3")?.as_reference()?;
    let Object::Stream(stream) = document.get(program) else {
        return None;
    };
    (document
        .get_key(&stream.dict, "Subtype")
        .as_name()?
        .as_bytes()
        == b"CIDFontType0C")
        .then_some((at, descendant, descriptor, program))
}

/// A `Type0` font as read, with every distinct code it shows, keyed by its value and length,
/// which together are its identity.
type FontRead<'a> = (&'a Dictionary, LoadedFont, BTreeMap<(u32, u8), Code>);

/// One glyph per CID on glyph 0 that `read` shows, CID 0 among them, at the width the font gives
/// the CID's codes, in the program's `units` per em; `None` where such a code reached glyph 0
/// by §9.7.6.3's notdef mappings rather than its own CID, where a CID is shown at two widths, or
/// at one a 16-bit advance does not hold.
fn cids_asked(read: &[FontRead<'_>], units: f32) -> Option<BTreeMap<u16, i64>> {
    let mut asked: BTreeMap<u16, i64> = BTreeMap::new();
    for (before, code) in read
        .iter()
        .flat_map(|(_, before, codes)| codes.values().map(move |code| (before, code)))
    {
        if before.glyph_index(*code) != Some(0) {
            continue;
        }
        let cid = u16::try_from(before.cid(*code)?).ok()?;
        let advance = (before.advance(*code) * units).round();
        if !(0.0..=f32::from(u16::MAX)).contains(&advance) {
            return None;
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the advance was checked to lie in 0..=u16::MAX and is a whole number"
        )]
        let advance = advance as i64;
        if *asked.entry(cid).or_insert(advance) != advance {
            return None;
        }
    }
    Some(asked)
}

/// The glyphs the CIDs on glyph 0 of the `Type0` fonts `fonts` over one CID-keyed CFF descendant
/// are given, each font with the strings the pages show in it, proved for every one of them;
/// `None` where `fonts` is empty, a font is not one [`is_cid_cff`] admits or names another
/// descendant, a code on glyph 0 reached it by §9.7.6.3's notdef mappings rather than its own
/// CID or by CID 0, a CID is shown at two widths or at one a 16-bit advance does not hold, the
/// program cannot be written, or the font fails the proof.
///
/// `copied` is the running count of program bytes the conversion copies, which the caller holds
/// to `budget`; it grows by this program's length before anything is copied.
pub(in crate::archive) fn given(
    document: &Document,
    fonts: &[(&Dictionary, Vec<&[u8]>)],
    (spare, programs, copied): (&mut Spare, &BTreeMap<ObjectId, Object>, &mut usize),
    budget: usize,
) -> Option<CidCffGlyphsGiven> {
    let (first, _) = fonts.first()?;
    let (descendant_at, descendant, descriptor, program_at) = descendant_of(document, first)?;
    // Each font as read, with every distinct code it shows, keyed by its value and length, which
    // together are its identity.
    let mut read: Vec<FontRead<'_>> = Vec::new();
    for (dict, shown) in fonts {
        if descendant_id(document, dict) != Some(descendant_at) {
            return None;
        }
        let before = loaded_as_stated(document, dict)?;
        if before.is_substituted() {
            return None;
        }
        let mut codes: BTreeMap<(u32, u8), Code> = BTreeMap::new();
        for text in shown {
            for code in before.decode(text) {
                codes.entry((code.value(), code.length())).or_insert(code);
            }
        }
        read.push((dict, before, codes));
    }
    let (stream, program) = program_in_budget(document, programs, program_at, (copied, budget))?;
    let units = pdf_font::cff::units_per_em(&program).ok()?;

    let asked = cids_asked(&read, units)?;
    if asked.is_empty() {
        return None;
    }
    // A code whose own CID is 0 goes to a fresh CID (`doc/adr/2622`).
    let fresh = if asked.contains_key(&0) {
        Some(fresh_planned(
            document,
            &read,
            (descendant_at, &descendant),
            &program,
        )?)
    } else {
        None
    };
    let fresh_cid = fresh.as_ref().map(|plan| plan.cid);
    let cids: Vec<CidToGive> = asked
        .iter()
        .map(|(cid, advance)| CidToGive {
            cid: match (*cid, fresh_cid) {
                (0, Some(fresh)) => fresh,
                (cid, _) => cid,
            },
            advance: *advance,
        })
        .collect();
    let (rebuilt, glyphs) = pdf_font::cff::with_cids_given(&program, &cids).ok()?;
    // Keyed by the CID the file gave each code, CID 0 to the glyph its fresh CID reaches.
    let given: BTreeMap<u16, u16> = asked.keys().copied().zip(glyphs).collect();
    let given_cids: Vec<u16> = cids.iter().map(|asked| asked.cid).collect();

    let new_program = super::program_stream(Some(&stream.dict), &rebuilt, false)?;
    let cid_set = match descriptor.get("CIDSet") {
        Some(value) => Some(cid_set_restated(document, value, given_cids.iter())?),
        None => None,
    };

    if !proved_for_every_font(
        document,
        &read,
        (descendant, &descriptor),
        (&new_program, cid_set.as_ref()),
        (fresh.as_ref(), &given),
    ) {
        return None;
    }

    let program_id = spare.take(document)?;
    let descriptor_id = spare.take(document)?;
    let mut new_descriptor = descriptor;
    new_descriptor.insert(Name::new(&b"FontFile3"[..]), Object::Reference(program_id));
    let mut written = vec![(program_id, new_program)];
    if let Some(set) = cid_set {
        let set_id = spare.take(document)?;
        new_descriptor.insert(Name::new(&b"CIDSet"[..]), Object::Reference(set_id));
        written.push((set_id, set));
    }
    written.push((descriptor_id, Object::Dictionary(new_descriptor)));
    let fresh = match fresh {
        Some(plan) => {
            let cmap_id = spare.take(document)?;
            written.push((cmap_id, plan.cmap));
            Some(FreshWritten {
                cid: plan.cid,
                cmap: cmap_id,
                widths: plan.widths,
            })
        }
        None => None,
    };
    Some(CidCffGlyphsGiven {
        descendant: descendant_at,
        descriptor: descriptor_id,
        fresh,
        written,
    })
}

/// The fresh CID [`given`] sends the code of CID 0 to, placed past every CID the charset holds
/// or the pages show (`doc/adr/2622`); `None` where none can be planned.
fn fresh_planned(
    document: &Document,
    read: &[FontRead<'_>],
    descendant: (ObjectId, &Dictionary),
    program: &[u8],
) -> Option<FreshCid> {
    // The fonts that show a code whose own CID is 0, which the encoding sends to the fresh CID.
    let showing: Vec<&Dictionary> = read
        .iter()
        .filter(|(_, before, codes)| codes.values().any(|code| before.cid(*code) == Some(0)))
        .map(|(dict, ..)| *dict)
        .collect();
    let highest_shown = read
        .iter()
        .flat_map(|(_, before, codes)| codes.values().filter_map(|code| before.cid(*code)))
        .max()?;
    let highest_held = pdf_font::cff::highest_cid(program).ok()?;
    fresh_cid::planned(
        document,
        &showing,
        descendant,
        highest_shown.max(u32::from(highest_held)),
    )
}

/// Whether every font of `read` loads, from objects held in memory, as the conversion will write
/// it and draws every code it shows as [`proved`] requires.
fn proved_for_every_font(
    document: &Document,
    read: &[FontRead<'_>],
    (descendant, descriptor): (Dictionary, &Dictionary),
    (new_program, cid_set): (&Object, Option<&Object>),
    (fresh, given): (Option<&FreshCid>, &BTreeMap<u16, u16>),
) -> bool {
    let mut proof_descriptor = descriptor.clone();
    proof_descriptor.insert(Name::new(&b"FontFile3"[..]), new_program.clone());
    if let Some(set) = cid_set {
        proof_descriptor.insert(Name::new(&b"CIDSet"[..]), set.clone());
    }
    let mut proof_descendant = descendant;
    proof_descendant.insert(
        Name::new(&b"FontDescriptor"[..]),
        Object::Dictionary(proof_descriptor),
    );
    if let Some(plan) = fresh {
        proof_descendant.insert(Name::new(&b"W"[..]), plan.widths.clone());
    }
    let fresh_cid = fresh.map(|plan| plan.cid);
    read.iter().all(|(dict, before, codes)| {
        let mut candidate = (*dict).clone();
        let shows_zero = codes.values().any(|code| before.cid(*code) == Some(0));
        if let Some(plan) = fresh.filter(|_| shows_zero) {
            candidate.insert(Name::new(&b"Encoding"[..]), plan.cmap.clone());
        }
        candidate.insert(
            Name::new(&b"DescendantFonts"[..]),
            Object::Array(vec![Object::Dictionary(proof_descendant.clone())]),
        );
        LoadedFont::load(document, &candidate, "").is_ok_and(|after| {
            !after.is_substituted() && proved(before, &after, codes.values(), (given, fresh_cid))
        })
    })
}

/// The outline this reader draws for `code`, as its path commands; empty where it draws none.
fn drawn(font: &LoadedFont, code: Code) -> Vec<pdf_render::PathCommand> {
    font.outline(code)
        .map(|outline| outline.commands().to_vec())
        .unwrap_or_default()
}

/// Whether every code shown draws, in the font as written, the outline it drew at the width it
/// had; reaches the copy given its CID, with the copy's advance its width, where it reached glyph
/// 0, and the glyph it reached otherwise; and is not drawn as CID 0.
///
/// `given` is keyed by the CID the file gave each code. Where `fresh` is the CID a code of CID 0
/// is sent to, every such code must select it, and every other code the CID it selected before.
fn proved<'a>(
    before: &LoadedFont,
    after: &LoadedFont,
    codes: impl Iterator<Item = &'a Code>,
    (given, fresh): (&BTreeMap<u16, u16>, Option<u16>),
) -> bool {
    codes.into_iter().all(|code| {
        let code = *code;
        let cid_wanted = match before.cid(code) {
            Some(0) => fresh.map(u32::from),
            stated => stated,
        };
        if (after.advance(code) - before.advance(code)).abs() > super::CONSISTENT
            || after.drawn_cid(code) == Some(0)
            || after.cid(code) != cid_wanted
            || drawn(after, code) != drawn(before, code)
        {
            return false;
        }
        let copy = before
            .cid(code)
            .and_then(|cid| u16::try_from(cid).ok())
            .and_then(|cid| given.get(&cid))
            .filter(|_| before.glyph_index(code) == Some(0));
        match copy {
            Some(glyph) => {
                after.glyph_index(code) == Some(*glyph)
                    && after.program_advance(code).is_some_and(|program| {
                        (program - after.advance(code)).abs() <= super::CONSISTENT
                    })
            }
            None => after.glyph_index(code) == before.glyph_index(code),
        }
    })
}
