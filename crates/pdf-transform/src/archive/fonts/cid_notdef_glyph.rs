//! A `CIDFontType2` font whose shown CIDs reach glyph 0, given a glyph of its own for each width,
//! reached through a restated `/CIDToGIDMap`.
//!
//! ISO 19005-2 section 6.2.11.8 and ISO 19005-4 section 6.2.10.9 forbid a text-showing operator
//! to reference the `.notdef` glyph, and in a `CIDFontType2` font the glyph a CID selects is the
//! one ISO 32000-2 §9.7.4.2 and Table 115's `/CIDToGIDMap` name: the identity, or the 2-byte index
//! a map stream stores for the CID. A CID that lands on glyph 0 draws the program's own `.notdef`
//! outline, and that outline is what this reader drew for it (`doc/questions/A694` item 3). The
//! program gains copies of glyph 0 ([`super::notdef_glyph::with_notdef_copied`], the copy the
//! simple TrueType route makes), and the map is restated as a stream that sends each such CID to
//! its copy: the page draws the producer's marks, no content stream changes, and no CID shown
//! reaches `.notdef` (`doc/adr/2556`).
//!
//! **A map stream the file stated.** The stream keeps every entry and grows only to the highest
//! CID given (Table 115); the identity, stated or absent (§9.7.4.2), never arrives, because under
//! it the only CID on glyph 0 is CID 0 (below).
//!
//! **CID 0 by a fresh CID.** ISO 32000-2 §9.7.4.2 makes the glyph description for CID 0 the
//! `CIDFont`'s analogue of the `.notdef` character name in a simple font, so a shown CID 0
//! references `.notdef` whatever glyph a map sends it to (`doc/adr/2557`). A code whose own CID is
//! 0 is sent to a fresh CID by the encoding [`super::fresh_cid`] restates, and the map sends that
//! CID to the glyph CID 0 drew, its copy where that was glyph 0; under the identity the map is
//! restated as a stream that keeps every glyph's own index (`doc/adr/2622`).
//!
//! **One glyph per width.** A composite font's widths are by CID (§9.7.4.3's `/W` and `/DW`), and
//! ISO 19005-2 section 6.2.11.5 holds the program's advance for the glyph a CID reaches to that
//! width; CIDs the file gave two widths cannot share one copy.
//!
//! **Nothing is written that was not proved**: the font is loaded again from the new descendant,
//! descriptor, program and map held in memory, and every code the pages show must reach the
//! glyph it reached before, or the copy given its CID, at the width it had before and with the
//! copy's advance equal to it.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use pdf_font::{Code, LoadedFont};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};

use super::super::prepare::Spare;
use super::super::sites::{dictionary_of, loaded_as_stated};
use super::fresh_cid::{self, FreshCid};
use super::notdef_glyph;

/// The most entries a map stream holds: one for each CID a 16-bit index names.
const MOST_ENTRIES: usize = 1 << 16;

/// Glyph 0, which every TrueType program keeps for a character it has no glyph for.
const NOTDEF: u16 = 0;

/// Why a `CIDFontType2` font named by the `.notdef` rows keeps the refusal.
pub(in crate::archive) const CID_NOT_GIVEN: &str = "this file shows a CID of a CIDFontType2 font \
     whose CIDToGIDMap reaches the .notdef glyph of its embedded TrueType program, and this \
     conversion gives each such CID a glyph of its own whose outline and advance are copied from \
     .notdef's, one for each width, with the map restated as a stream. It does that only where \
     the descendant CIDFont is an object of its own, every string the pages show in the font is \
     known, each code on .notdef reached it by its own CID rather than by a fallback, the \
     program can be extended, and the font loads again with every shown code on the glyph it \
     reached before or on its copy: one of those does not hold here";

/// Why a `CIDFontType2` font that shows CID 0 keeps the refusal, whatever glyph CID 0 is drawn by.
///
/// ISO 32000-2 §9.7.4.2 makes CID 0 the `CIDFont`'s own `.notdef` character, and a code the font
/// draws as CID 0 references `.notdef` whatever glyph a `/CIDToGIDMap` sends CID 0 to
/// (`doc/adr/2557`). A code whose own CID is 0 is sent to a fresh CID through an embedded `CMap`
/// ([`super::fresh_cid`], `doc/adr/2622`); this is the sentence for a font where that does not
/// hold, or where a code reaches CID 0 by §9.7.6.3's fallbacks rather than its own CID.
pub(in crate::archive) const CID_0_SHOWN: &str = "this file shows CID 0 of a CIDFontType2 font, \
     and ISO 32000-2 section 9.7.4.2 makes CID 0 the CIDFont's own .notdef character whatever \
     glyph its CIDToGIDMap sends it to, so the code references .notdef — which ISO 19005-2 \
     section 6.2.11.8 and ISO 19005-4 section 6.2.10.9 forbid a text-showing operator to do. This \
     conversion sends such a code to a fresh CID given the glyph CID 0 drew, by restating the \
     font's Encoding as an embedded CMap; it does that only where the code's own CID is 0 rather \
     than CID 0 being a fallback, the Encoding is Identity-H, the font has a ToUnicode CMap or a \
     character collection ISO 32000-2 section 9.10.2 does not read by CID, a 16-bit CID is left \
     past every CID the font uses, and the font loads again with every shown code drawing what \
     it drew: one of those does not hold here";

/// Whether any code in the strings `shown` is drawn as CID 0 by the `Type0` font `dict`, after
/// ISO 32000-2 §9.7.6.3's fallbacks, which end at CID 0 too ([`LoadedFont::drawn_cid`]).
pub(in crate::archive) fn shows_cid_0<'a>(
    document: &Document,
    dict: &Dictionary,
    mut shown: impl Iterator<Item = &'a [u8]>,
) -> bool {
    loaded_as_stated(document, dict).is_some_and(|font| {
        shown.any(|text| {
            font.decode(text)
                .into_iter()
                .any(|code| font.drawn_cid(code) == Some(0))
        })
    })
}

/// Whether a code in the strings `shown` is drawn as CID 0 by the `Type0` font `dict` through
/// §9.7.6.3's fallbacks, its own CID being another or none: such a code is no code the encoding
/// can send to a fresh CID, since the CID it would be sent from is not 0.
pub(in crate::archive) fn falls_back_to_cid_0<'a>(
    document: &Document,
    dict: &Dictionary,
    mut shown: impl Iterator<Item = &'a [u8]>,
) -> bool {
    loaded_as_stated(document, dict).is_some_and(|font| {
        shown.any(|text| {
            font.decode(text)
                .into_iter()
                .any(|code| font.drawn_cid(code) == Some(0) && font.cid(code) != Some(0))
        })
    })
}

/// The objects one `CIDFontType2` font's glyphs are given by: the descendant they are written
/// into, the new descriptor and map it names, and every new object to add.
#[derive(Debug)]
pub(in crate::archive) struct CidGlyphsGiven {
    /// The descendant `CIDFont` dictionary that names the new descriptor and map.
    pub(in crate::archive) descendant: ObjectId,
    /// The descriptor the descendant names from now on.
    pub(in crate::archive) descriptor: ObjectId,
    /// The `/CIDToGIDMap` stream the descendant names from now on.
    pub(in crate::archive) map: ObjectId,
    /// The encoding and widths that send a shown CID 0 to a fresh CID, where one was shown.
    pub(in crate::archive) fresh: Option<FreshWritten>,
    /// The new objects: the map, and where they changed the program, the descriptor and the
    /// `/CIDSet`, and the embedded `CMap` of [`Self::fresh`].
    pub(in crate::archive) written: Vec<(ObjectId, Object)>,
}

/// What a `Type0` font and its descendant name from now on where a code of CID 0 is sent to a
/// fresh CID (`doc/adr/2622`).
#[derive(Debug)]
pub(in crate::archive) struct FreshWritten {
    /// The fresh CID.
    pub(in crate::archive) cid: u16,
    /// The embedded `CMap` the `Type0` fonts that show CID 0 name as their `/Encoding`.
    pub(in crate::archive) cmap: ObjectId,
    /// The descendant's `/W` from now on, with the fresh CID at CID 0's width.
    pub(in crate::archive) widths: Object,
}

/// Whether a font dictionary is a `Type0` font whose one descendant is a `CIDFontType2` with an
/// embedded `/FontFile2`, written as an object of its own — the font [`given`] can carry.
pub(in crate::archive) fn is_cid_truetype(document: &Document, dict: &Dictionary) -> bool {
    descendant_of(document, dict).is_some()
}

/// The descendant's object number, its dictionary and its descriptor, for a font
/// [`is_cid_truetype`] admits.
///
/// The descendant is required to be an indirect object because that is where the new entries are
/// written: §9.7.1's `/DescendantFonts` is a one-element array, and a dictionary written inside
/// it would have to be rewritten inside the `Type0` font instead.
fn descendant_of(
    document: &Document,
    dict: &Dictionary,
) -> Option<(ObjectId, Dictionary, Dictionary)> {
    if document.get_key(dict, "Subtype").as_name()?.as_bytes() != b"Type0" {
        return None;
    }
    let descendants = document.resolve(dict.get("DescendantFonts")?);
    let array = descendants.as_array()?;
    let [only] = array else {
        return None;
    };
    let at = only.as_reference()?;
    let resolved = document.get(at);
    let descendant = dictionary_of(&resolved)?.clone();
    if document
        .get_key(&descendant, "Subtype")
        .as_name()?
        .as_bytes()
        != b"CIDFontType2"
    {
        return None;
    }
    let resolved = document.resolve(descendant.get("FontDescriptor")?);
    let descriptor = dictionary_of(&resolved)?.clone();
    descriptor.get("FontFile2")?.as_reference()?;
    Some((at, descendant, descriptor))
}

/// The glyphs a `CIDFontType2` font's CIDs on glyph 0 are given, with `shown` the strings the
/// pages show in it, proved; `None` where the font is not one [`is_cid_truetype`] admits, a code
/// on glyph 0 reached it by §9.7.6.3's fallbacks rather than its own CID, the program cannot be
/// extended, or the font fails the proof.
///
/// `copied` is the running count of program bytes the conversion copies, which the caller holds
/// to its budget; it grows by this program's length before anything is copied.
pub(in crate::archive) fn given<'a>(
    document: &Document,
    dict: &Dictionary,
    shown: impl Iterator<Item = &'a [u8]>,
    (spare, programs, copied): (&mut Spare, &BTreeMap<ObjectId, Object>, &mut usize),
    budget: usize,
) -> Option<CidGlyphsGiven> {
    let (descendant_id, descendant, descriptor) = descendant_of(document, dict)?;
    let program_at = descriptor.get("FontFile2")?.as_reference()?;
    let before = loaded_as_stated(document, dict)?;

    // Every distinct code shown, keyed by its value and length, which together are its identity.
    let mut codes: BTreeMap<(u32, u8), Code> = BTreeMap::new();
    for text in shown {
        for code in before.decode(text) {
            codes.entry((code.value(), code.length())).or_insert(code);
        }
    }

    let (stream, program) = program_in_budget(document, programs, program_at, (copied, budget))?;
    let units = f32::from(notdef_glyph::units_per_em(&program)?);

    let by_advance = cids_on_notdef(&before, codes.values(), units)?;
    // A code whose own CID is 0 selects the `CIDFont`'s `.notdef` character (§9.7.4.2), which no
    // map can give a glyph: the encoding sends it to a fresh CID instead (`doc/adr/2622`).
    let zero_codes: Vec<Code> = codes
        .values()
        .copied()
        .filter(|code| before.cid(*code) == Some(0))
        .collect();
    if by_advance.is_empty() && zero_codes.is_empty() {
        return None;
    }
    let advances: Vec<u16> = by_advance.keys().copied().collect();
    let (restated, notdef_copies) = if advances.is_empty() {
        (None, Vec::new())
    } else {
        let glyphs = notdef_glyph::with_notdef_copied(&program, &advances)?;
        let restated = pdf_font::embedding::with_tables(&program, &glyphs.tables)?;
        (Some(restated), glyphs.glyphs)
    };
    // Each CID on glyph 0, as the file states it, to the copy given for its width.
    let mut given: BTreeMap<u16, u16> = BTreeMap::new();
    for (cids, glyph) in by_advance.values().zip(&notdef_copies) {
        for cid in cids {
            given.insert(*cid, *glyph);
        }
    }
    if let Some(restated) = &restated {
        let notdef_outline = notdef_glyph::outline(&program, NOTDEF);
        if given
            .values()
            .any(|glyph| notdef_glyph::outline(restated, *glyph) != notdef_outline)
        {
            return None;
        }
    }
    let identity = !matches!(
        descendant
            .get("CIDToGIDMap")
            .map(|value| document.resolve(value)),
        Some(Object::Stream(_))
    );
    let glyph_count = notdef_glyph::glyph_count(&program)?;
    let fresh = match zero_codes.first() {
        Some(zero) => Some(fresh_planned(
            document,
            (dict, descendant_id, &descendant),
            (&before, &codes, zero),
            (identity, glyph_count),
            &given,
        )?),
        None => None,
    };
    // The map as written: CID 0 keeps its entry and the fresh CID takes the glyph it drew.
    let mut mapped: BTreeMap<u16, u16> = given
        .iter()
        .filter(|(cid, _)| **cid != 0)
        .map(|(cid, glyph)| (*cid, *glyph))
        .collect();
    if let Some((plan, glyph)) = &fresh {
        mapped.insert(plan.cid, *glyph);
    }
    let identity_count = (identity && fresh.is_some()).then_some(glyph_count);
    let map = super::program_stream(
        None,
        &map_restated(document, &descendant, &mapped, identity_count)?,
        false,
    )?;
    let new_program = match &restated {
        Some(restated) => Some(super::program_stream(Some(&stream.dict), restated, true)?),
        None => None,
    };
    let cid_set = match descriptor.get("CIDSet") {
        Some(value) => Some(cid_set_restated(document, value, mapped.keys())?),
        None => None,
    };

    // The proof reads the font as the conversion will write it, from objects held in memory.
    let candidate = written_font(
        dict,
        (&descendant, &descriptor),
        (new_program.as_ref(), &map, cid_set.as_ref()),
        fresh.as_ref().map(|(plan, _)| plan),
    );
    let after = LoadedFont::load(document, &candidate, "").ok()?;
    let fresh_cid = fresh.as_ref().map(|(plan, _)| plan.cid);
    if after.is_substituted() || !proved(&before, &after, codes.values(), (&given, fresh_cid)) {
        return None;
    }

    objects_written(
        document,
        spare,
        (descendant_id, &descendant, descriptor),
        (new_program, map, cid_set),
        fresh.map(|(plan, _)| plan),
    )
}

/// The program stream at `at`, as `programs` holds it where an earlier step rewrote it, and its
/// decoded bytes; `None` where it is not a stream that decodes or where `copied`, grown by its
/// length before anything is copied, passes `budget`.
pub(super) fn program_in_budget(
    document: &Document,
    programs: &BTreeMap<ObjectId, Object>,
    at: ObjectId,
    (copied, budget): (&mut usize, usize),
) -> Option<(Arc<Stream>, Arc<[u8]>)> {
    let Object::Stream(stream) = programs
        .get(&at)
        .cloned()
        .unwrap_or_else(|| document.get(at))
    else {
        return None;
    };
    let program = document.decoded_stream_data(&stream)?;
    *copied = copied.saturating_add(program.len());
    if *copied > budget {
        return None;
    }
    Some((stream, program))
}

/// The fresh CID [`given`] sends `zero`, a code whose own CID is 0, to, with the glyph it draws
/// (`doc/adr/2622`); `None` where no fresh CID can be planned.
///
/// The fresh CID is placed past every CID shown and every CID the map reaches, so no CID the
/// file already draws changes its glyph.
fn fresh_planned(
    document: &Document,
    (dict, descendant_id, descendant): (&Dictionary, ObjectId, &Dictionary),
    (before, codes, zero): (&LoadedFont, &BTreeMap<(u32, u8), Code>, &Code),
    (identity, glyph_count): (bool, u16),
    given: &BTreeMap<u16, u16>,
) -> Option<(FreshCid, u16)> {
    let highest_shown = codes.values().filter_map(|code| before.cid(*code)).max()?;
    let highest_mapped = if identity {
        u32::from(glyph_count.checked_sub(1)?)
    } else {
        u32::try_from(map_entries(document, descendant)?.len().checked_sub(1)?).ok()?
    };
    let plan = fresh_cid::planned(
        document,
        &[dict],
        (descendant_id, descendant),
        highest_shown.max(highest_mapped),
    )?;
    // The fresh CID draws what CID 0 drew: the copy where that was glyph 0, the program's own
    // glyph where the map sent CID 0 elsewhere.
    let glyph = match given.get(&0) {
        Some(copy) => *copy,
        None => before.glyph_index(*zero)?,
    };
    Some((plan, glyph))
}

/// The `/CIDSet` a descriptor states as `value`, with every CID of `cids` set; `None` where it is
/// not a stream that decodes.
pub(super) fn cid_set_restated<'a>(
    document: &Document,
    value: &Object,
    cids: impl Iterator<Item = &'a u16>,
) -> Option<Object> {
    let Object::Stream(set) = document.resolve(value) else {
        return None;
    };
    let bits = document.decoded_stream_data(&set)?;
    super::program_stream(None, &cid_set_with(&bits, cids), false)
}

/// The objects [`given`] writes once its proof holds, each at a spare object number: the
/// program and the `CIDSet` where they changed, the descriptor where it changed or was direct,
/// the map, and the embedded `CMap` of the fresh CID.
fn objects_written(
    document: &Document,
    spare: &mut Spare,
    (descendant_id, descendant, descriptor): (ObjectId, &Dictionary, Dictionary),
    (new_program, map, cid_set): (Option<Object>, Object, Option<Object>),
    fresh: Option<FreshCid>,
) -> Option<CidGlyphsGiven> {
    let mut written = Vec::new();
    let mut new_descriptor = descriptor;
    let mut descriptor_changed = false;
    if let Some(program) = new_program {
        let program_id = spare.take(document)?;
        new_descriptor.insert(Name::new(&b"FontFile2"[..]), Object::Reference(program_id));
        written.push((program_id, program));
        descriptor_changed = true;
    }
    let map_id = spare.take(document)?;
    written.push((map_id, map));
    if let Some(set) = cid_set {
        let set_id = spare.take(document)?;
        new_descriptor.insert(Name::new(&b"CIDSet"[..]), Object::Reference(set_id));
        written.push((set_id, set));
        descriptor_changed = true;
    }
    // A descriptor nothing changed is named as the file named it, where it is an object of its own.
    let descriptor_id = match descendant
        .get("FontDescriptor")
        .and_then(Object::as_reference)
    {
        Some(stated) if !descriptor_changed => stated,
        _ => {
            let at = spare.take(document)?;
            written.push((at, Object::Dictionary(new_descriptor)));
            at
        }
    };
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
    Some(CidGlyphsGiven {
        descendant: descendant_id,
        descriptor: descriptor_id,
        map: map_id,
        fresh,
        written,
    })
}

/// The CIDs of the codes shown that reach glyph 0, grouped by the width the font gives them in
/// the program's `units` per em.
///
/// Empty where none does; `None` where one reached glyph 0 through §9.7.6.3's fallbacks rather
/// than by its own CID, where a CID passes 16 bits, or where one CID is shown at two widths.
fn cids_on_notdef<'a>(
    font: &LoadedFont,
    codes: impl Iterator<Item = &'a Code>,
    units: f32,
) -> Option<BTreeMap<u16, BTreeSet<u16>>> {
    let mut by_advance: BTreeMap<u16, BTreeSet<u16>> = BTreeMap::new();
    for code in codes {
        if font.glyph_index(*code) != Some(NOTDEF) {
            continue;
        }
        // A code with no CID of its own reached glyph 0 through §9.7.6.3's fallbacks, which a map
        // keyed by CID cannot restate without moving every other code that falls back.
        let cid = u16::try_from(font.cid(*code)?).ok()?;
        by_advance
            .entry(advance_in_units(font, *code, units)?)
            .or_default()
            .insert(cid);
    }
    // One CID at two widths would need two glyphs for one map entry.
    let mut seen = BTreeSet::new();
    if !by_advance.values().flatten().all(|cid| seen.insert(*cid)) {
        return None;
    }
    Some(by_advance)
}

/// The `Type0` font as the conversion writes it, every new object held inline, for the proof to
/// load.
fn written_font(
    dict: &Dictionary,
    (descendant, descriptor): (&Dictionary, &Dictionary),
    (program, map, cid_set): (Option<&Object>, &Object, Option<&Object>),
    fresh: Option<&FreshCid>,
) -> Dictionary {
    let mut proof_descriptor = descriptor.clone();
    if let Some(program) = program {
        proof_descriptor.insert(Name::new(&b"FontFile2"[..]), program.clone());
    }
    if let Some(set) = cid_set {
        proof_descriptor.insert(Name::new(&b"CIDSet"[..]), set.clone());
    }
    let mut proof_descendant = descendant.clone();
    proof_descendant.insert(
        Name::new(&b"FontDescriptor"[..]),
        Object::Dictionary(proof_descriptor),
    );
    proof_descendant.insert(Name::new(&b"CIDToGIDMap"[..]), map.clone());
    let mut candidate = dict.clone();
    if let Some(fresh) = fresh {
        proof_descendant.insert(Name::new(&b"W"[..]), fresh.widths.clone());
        candidate.insert(Name::new(&b"Encoding"[..]), fresh.cmap.clone());
    }
    candidate.insert(
        Name::new(&b"DescendantFonts"[..]),
        Object::Array(vec![Object::Dictionary(proof_descendant)]),
    );
    candidate
}

/// A code's width in the program's units, rounded to the whole unit an advance is stored in.
///
/// `None` where it falls outside what a 16-bit advance holds.
fn advance_in_units(font: &LoadedFont, code: Code, units: f32) -> Option<u16> {
    let advance = (font.advance(code) * units).round();
    if !(0.0..=f32::from(u16::MAX)).contains(&advance) {
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the advance was checked to lie in 0..=u16::MAX and is a whole number"
    )]
    Some(advance as u16)
}

/// The `/CIDToGIDMap` stream's bytes: the map the descendant stated, with each CID of `given`
/// sent to its glyph (Table 115's 2 × c and 2 × c + 1, high byte first).
///
/// A stated stream keeps every entry it had and grows only as far as the highest CID given, at
/// most 65 536 entries, the most a 16-bit CID indexes. The identity, stated or absent (§9.7.4.2),
/// is restated only where `identity_count` is the program's glyph count, as an entry for each of
/// those glyphs sending its CID to itself: that is where a shown CID 0 is sent to a fresh CID,
/// past every glyph (`doc/adr/2622`); otherwise the only CID the identity sends to glyph 0 is CID
/// 0, and there is nothing to restate.
fn map_restated(
    document: &Document,
    descendant: &Dictionary,
    given: &BTreeMap<u16, u16>,
    identity_count: Option<u16>,
) -> Option<Vec<u8>> {
    let mut entries: Vec<u16> = match identity_count {
        Some(count) => (0..count).collect(),
        None => map_entries(document, descendant)?,
    };
    let wanted = usize::from(*given.keys().last()?).checked_add(1)?;
    if entries.len() < wanted {
        entries.resize(wanted, NOTDEF);
    }
    for (cid, glyph) in given {
        *entries.get_mut(usize::from(*cid))? = *glyph;
    }
    Some(
        entries
            .iter()
            .flat_map(|glyph| glyph.to_be_bytes())
            .collect(),
    )
}

/// The glyph index each CID of a stated `/CIDToGIDMap` stream sends it to, at most
/// [`MOST_ENTRIES`]; `None` where the map is the identity, stated or absent, or does not decode.
fn map_entries(document: &Document, descendant: &Dictionary) -> Option<Vec<u16>> {
    let Object::Stream(stream) = document.resolve(descendant.get("CIDToGIDMap")?) else {
        return None;
    };
    let bytes = document.decoded_stream_data(&stream)?;
    Some(
        bytes
            .chunks_exact(2)
            .take(MOST_ENTRIES)
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect(),
    )
}

/// A `/CIDSet`'s bits with each of `cids` set, the table grown as far as the highest of them.
///
/// Table 124: one bit a CID, high-order bit first, CID 0 the first byte's most significant bit.
/// A CID given a glyph is present in the program from now on, which is what the set states.
pub(super) fn cid_set_with<'a>(bits: &[u8], cids: impl Iterator<Item = &'a u16>) -> Vec<u8> {
    let mut out = bits.to_vec();
    for cid in cids {
        let byte = usize::from(*cid / 8);
        if out.len() <= byte {
            out.resize(byte.saturating_add(1), 0);
        }
        if let Some(slot) = out.get_mut(byte) {
            *slot |= 0x80 >> (cid % 8);
        }
    }
    out
}

/// Whether every code shown reaches, in the font as written, the glyph it reached before — or
/// the copy given its CID, with the copy's advance its width — at the width it had before, and by
/// the program's own glyphs — and none selects CID 0, which §9.7.4.2 makes the `CIDFont`'s
/// `.notdef` whatever glyph it reaches, by its own CID or by §9.7.6.3's fallbacks (`doc/adr/2582`).
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
        {
            return false;
        }
        let copy = before
            .cid(code)
            .and_then(|cid| u16::try_from(cid).ok())
            .and_then(|cid| given.get(&cid))
            .filter(|_| before.glyph_index(code) == Some(NOTDEF));
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

#[cfg(test)]
mod tests {
    use super::cid_set_with;

    #[test]
    fn a_cid_given_a_glyph_is_set_in_the_cid_set_with_the_high_order_bit_first() {
        // CID 0 is the first byte's most significant bit, CID 9 the second byte's second.
        assert_eq!(cid_set_with(&[0x00], [0_u16, 9].iter()), vec![0x80, 0x40]);
        assert_eq!(
            cid_set_with(&[0x01, 0xFF], [1_u16].iter()),
            vec![0x41, 0xFF]
        );
    }
}
