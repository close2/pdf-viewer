//! A CID-keyed CFF program with a glyph appended for each CID asked for, under that CID.
//!
//! ISO 32000-2 §9.7.4.2 reaches a Type 0 `CIDFont`'s glyph through the program's charset:
//!
//! > The "CFF" font program has a Top DICT that uses CIDFont operators: The CIDs shall be used to
//! > determine the GID value for the glyph procedure using the charset table in the CFF program.
//!
//! So a CID the charset does not hold reaches no glyph, and §9.7.6.3 substitutes the glyph for
//! CID 0 — the `CIDFont`'s `.notdef` — which ISO 19005-2 section 6.2.11.8 and ISO 19005-4 section
//! 6.2.10.9 forbid a text-showing operator to reference. The construction here is the one
//! `doc/adr/2578` section 4 describes and `doc/adr/2598` decides: each such CID is given a glyph of
//! its own that draws what this reader drew for it, a copy of glyph 0's charstring at the advance
//! asked for, and the charset is extended with the CID itself, so no `/CIDToGIDMap` and no `CMap`
//! changes. A CID-keyed program's glyph is interpreted against the Font DICT its `FDSelect` names
//! (Adobe Technical Note #5176, sections 18 and 19), so every copy is placed under glyph 0's Font
//! DICT, whose Private DICT and local subroutines the copied charstring was written against; the
//! `FDSelect` is restated in the format the program stated it in, 0 or 3, and the `FDArray` is
//! rewritten with each Private DICT offset shifted by `Layout::with_glyph_tables`, which every
//! other offset of the Top DICT already goes through. `CIDCount` is raised where a CID asked for
//! passes it.
//!
//! **Nothing leaves this module that does not read back** (trap 301): the rebuilt program is
//! opened by `read-fonts` and its `CharStrings` INDEX, its charset, its `FDSelect` answer for every
//! glyph, every Font DICT's Private DICT and local subroutines, its global subroutines and its
//! `CIDCount` are compared with what was written, before any caller sees the bytes.

use std::collections::BTreeSet;

use skrifa::GlyphId;
use skrifa::raw::ps::cff::CffFontRef;
use skrifa::raw::ps::cff::charset::CharsetKind;
use skrifa::raw::ps::string::Sid;

use super::{
    CffError, ESCAPE, Layout, NOTDEF_NOT_RESTATED, OP_CHARSTRINGS, OP_FD_ARRAY, OP_FD_SELECT,
    Subroutines, charstring_with_advance, dict_ints, global_subroutines, index_item, index_items,
    int5, local_subroutines, nominal_widths, offsets_stated_once, open, private_dict,
};

/// One CID for [`with_cids_given`] to give a glyph: the glyph is a copy of glyph 0's outline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CidToGive {
    /// The CID the charset states for the new glyph.
    pub cid: u16,
    /// Its advance, in the program's units.
    pub advance: i64,
}

/// The most charstring bytes one call appends, checked before a charstring is built and again as
/// each is: 4 MiB holds a copy for every CID a 16-bit charset names of a `.notdef` of 64 bytes,
/// and it bounds what a hostile `.notdef` times a hostile count of CIDs can make this allocate.
pub const MOST_CID_BYTES_GIVEN: usize = 4 << 20;

/// The Top DICT's `CIDCount` operator (Adobe Technical Note #5176, Table 10).
const OP_CID_COUNT: u16 = ESCAPE | 0x22;

/// The `CIDCount` a Top DICT that states none has (Adobe Technical Note #5176, Table 10).
const DEFAULT_CID_COUNT: i64 = 8720;

/// The most glyphs a `CharStrings` INDEX holds: its count is a `Card16`.
const MOST_GLYPHS: usize = 0xFFFF;

/// What the rebuilt program must read back as, glyph by glyph and Font DICT by Font DICT.
struct Expected<'a> {
    /// Every charstring, by glyph.
    charstrings: &'a [Vec<u8>],
    /// Every glyph's CID, glyph 0's CID 0 first.
    cids: &'a [u16],
    /// Every glyph's Font DICT.
    font_dicts: &'a [u8],
    /// Each Font DICT's Private DICT and local subroutines, as the source states them.
    privates: &'a [(Vec<u8>, Vec<Vec<u8>>)],
    /// The source's global subroutines.
    global: &'a [Vec<u8>],
    /// The least `CIDCount` the rebuilt Top DICT may state.
    cid_count: i64,
}

/// A CID-keyed program with one glyph appended for each CID in `cids`, each a copy of glyph 0's
/// charstring at its advance, under glyph 0's Font DICT, reached by that CID through the charset;
/// with the glyph index each was given, in `cids`' order.
///
/// Every glyph the program held keeps its index, its CID, its Font DICT and its charstring byte
/// for byte.
///
/// # Errors
///
/// [`CffError::Malformed`] where the program does not open, is name-keyed, states an offset or
/// `CIDCount` twice or with the wrong operands, states no `FDSelect` or one of a format other
/// than 0 and 3, has a predefined charset, is asked for CID 0, for a CID its charset holds or for
/// one CID twice, would hold more glyphs than an INDEX counts or append more than
/// [`MOST_CID_BYTES_GIVEN`], or does not read back as written; [`CffError::NoCharset`] where it
/// states no charset; and [`CffError::AdvanceNotRestatable`] where an advance does not fit a
/// charstring's width operand or `.notdef`'s charstring states its width where it cannot be
/// restated.
pub fn with_cids_given(data: &[u8], cids: &[CidToGive]) -> Result<(Vec<u8>, Vec<u16>), CffError> {
    given_with(data, cids, fd_select_written)
}

/// [`with_cids_given`] with the `FDSelect` written by `fd_select_of`, from the source's format and
/// every glyph's Font DICT: the seam a test plants a mis-written table through, to prove that
/// [`read_back`] refuses it.
fn given_with(
    data: &[u8],
    cids: &[CidToGive],
    fd_select_of: fn(u8, &[u8]) -> Option<Vec<u8>>,
) -> Result<(Vec<u8>, Vec<u16>), CffError> {
    let font = open(data)?;
    if !font.is_cid() {
        return Err(malformed(
            "the program is name-keyed, and a name-keyed program's glyphs are given by name",
        ));
    }
    let (mut layout, stated) = located(data)?;
    let charstrings: Vec<Vec<u8>> = index_items(data, stated.charstrings_at)
        .ok_or_else(|| malformed("the CharStrings INDEX could not be read"))?
        .into_iter()
        .map(<[u8]>::to_vec)
        .collect();
    if charstrings.len().saturating_add(cids.len()) > MOST_GLYPHS {
        return Err(malformed(
            "the CIDs asked for would take the program past the glyphs an INDEX counts",
        ));
    }
    let notdef_len = charstrings
        .first()
        .map(Vec::len)
        .ok_or_else(|| malformed("the CharStrings INDEX holds no glyph for CID 0"))?;
    if cids.len().saturating_mul(notdef_len) > MOST_CID_BYTES_GIVEN {
        return Err(malformed(
            "the copies of glyph 0 asked for would append more than this writer's bound",
        ));
    }
    let mut glyphs = glyphs_read(&font, charstrings, cids)?;

    // Glyph 0's Font DICT: its Private DICT's nominal width and its local subroutines are what
    // the copied charstring was written against.
    let notdefs_dict = *glyphs.font_dicts.first().unwrap_or(&0);
    let font_dict_bytes = index_items(data, stated.fd_array_at)
        .ok_or_else(|| malformed("the FDArray could not be read"))?;
    let privates: Vec<(Vec<u8>, Vec<Vec<u8>>)> = font_dict_bytes
        .iter()
        .map(|dict| {
            (
                private_dict(data, dict).unwrap_or_default().to_vec(),
                local_subroutines(data, dict),
            )
        })
        .collect();
    if glyphs
        .font_dicts
        .iter()
        .any(|font_dict| usize::from(*font_dict) >= privates.len())
    {
        return Err(malformed(
            "the FDSelect names a Font DICT the FDArray does not hold",
        ));
    }
    let nominal = nominal_widths(data, &layout, true)
        .get(usize::from(notdefs_dict))
        .copied()
        .unwrap_or(0);
    let global = global_subroutines(data);
    let local = index_item(data, stated.fd_array_at, usize::from(notdefs_dict))
        .map(|dict| local_subroutines(data, dict))
        .unwrap_or_default();
    let subroutines = Subroutines {
        global: &global,
        local: &local,
    };
    let added = copies_appended(&mut glyphs, cids, (notdefs_dict, nominal), &subroutines)?;

    let needed = cids
        .iter()
        .map(|asked| i64::from(asked.cid).saturating_add(1))
        .max()
        .unwrap_or(0);
    if needed > stated.cid_count {
        cid_count_raised(&mut layout, needed);
    }
    let expected = Expected {
        charstrings: &glyphs.charstrings,
        cids: &glyphs.cids,
        font_dicts: &glyphs.font_dicts,
        privates: &privates,
        global: &global,
        cid_count: needed.max(stated.cid_count),
    };
    let strings: Vec<Vec<u8>> = index_items(data, layout.top_index.end)
        .ok_or_else(|| malformed("the String INDEX could not be read"))?
        .into_iter()
        .map(<[u8]>::to_vec)
        .collect();
    let fd_select = fd_select_of(stated.fd_select_format, &glyphs.font_dicts)
        .ok_or_else(|| malformed("the FDSelect could not be restated"))?;
    let rebuilt = layout.with_glyph_tables(
        data,
        &strings,
        (&glyphs.charstrings, &charset_written(&glyphs.cids)),
        Some(&fd_select),
    )?;
    read_back(&rebuilt, &expected)?;
    Ok((rebuilt, added))
}

/// A [`CffError::Malformed`] stating `detail`.
fn malformed(detail: &str) -> CffError {
    CffError::Malformed {
        detail: detail.to_owned(),
    }
}

/// What the Top DICT of a CID-keyed program states that the writer restates around.
struct Stated {
    /// `CIDCount`, or its default where the Top DICT states none.
    cid_count: i64,
    /// Where the `CharStrings` INDEX starts.
    charstrings_at: usize,
    /// The `FDSelect`'s format byte, 0 or 3.
    fd_select_format: u8,
    /// Where the `FDArray` INDEX starts.
    fd_array_at: usize,
}

/// The Top DICT of `data` and the offsets it states once each, refused where it states one twice,
/// misstates one, or names an `FDSelect` of a format other than 0 and 3.
fn located(data: &[u8]) -> Result<(Layout, Stated), CffError> {
    let layout = Layout::read(data)
        .filter(offsets_stated_once)
        .ok_or_else(|| {
            malformed("the Top DICT could not be located, or states an offset twice or misstated")
        })?;
    let stated = |wanted: u16| -> Option<Vec<i64>> {
        let mut found = layout.entries.iter().filter(|(_, op)| *op == wanted);
        let first = found.next().map(|(operands, _)| dict_ints(operands));
        if found.next().is_some() {
            return Some(Vec::new());
        }
        first
    };
    let offset = |wanted: u16| -> Option<usize> {
        match stated(wanted)?.as_slice() {
            [only] => usize::try_from(*only).ok(),
            _ => None,
        }
    };
    let cid_count = match stated(OP_CID_COUNT) {
        None => DEFAULT_CID_COUNT,
        Some(values) => match values.as_slice() {
            [only] => *only,
            _ => return Err(malformed("the Top DICT states CIDCount twice or misstated")),
        },
    };
    let charstrings_at = offset(OP_CHARSTRINGS)
        .ok_or_else(|| malformed("the Top DICT states no CharStrings offset"))?;
    let fd_select_at =
        offset(OP_FD_SELECT).ok_or_else(|| malformed("the Top DICT states no FDSelect offset"))?;
    let fd_array_at =
        offset(OP_FD_ARRAY).ok_or_else(|| malformed("the Top DICT states no FDArray offset"))?;
    let fd_select_format = *data
        .get(fd_select_at)
        .ok_or_else(|| malformed("the FDSelect could not be read"))?;
    if fd_select_format != 0 && fd_select_format != 3 {
        return Err(malformed(
            "the FDSelect is of a format other than 0 and 3, which this writer does not restate",
        ));
    }
    let stated = Stated {
        cid_count,
        charstrings_at,
        fd_select_format,
        fd_array_at,
    };
    Ok((layout, stated))
}

/// Every glyph of a program, one entry each in three parallel lists.
struct Glyphs {
    /// Each glyph's charstring.
    charstrings: Vec<Vec<u8>>,
    /// The CID the charset states for each glyph.
    cids: Vec<u16>,
    /// The Font DICT the `FDSelect` names for each glyph.
    font_dicts: Vec<u8>,
}

/// The CID and Font DICT of every glyph `font` holds, as `read-fonts` reads them, refused where
/// the charset is a predefined one, where glyph 0 is not CID 0, and where a CID in `cids` is one
/// the charset holds or is asked for twice.
fn glyphs_read(
    font: &CffFontRef<'_>,
    charstrings: Vec<Vec<u8>>,
    cids: &[CidToGive],
) -> Result<Glyphs, CffError> {
    let charset = font.charset().ok_or(CffError::NoCharset)?;
    if matches!(
        charset.kind(),
        CharsetKind::IsoAdobe | CharsetKind::Expert | CharsetKind::ExpertSubset
    ) {
        return Err(malformed(
            "the charset is a predefined one, which names SIDs rather than CIDs",
        ));
    }
    let capacity = charstrings.len().saturating_add(cids.len());
    let mut glyphs = Glyphs {
        cids: Vec::with_capacity(capacity),
        font_dicts: Vec::with_capacity(capacity),
        charstrings,
    };
    for glyph in 0..glyphs.charstrings.len() {
        let id = GlyphId::from(u32::try_from(glyph).unwrap_or(u32::MAX));
        let cid = charset
            .string_id(id)
            .ok_or_else(|| malformed("the charset names no CID for a glyph the program holds"))?;
        let font_dict = font
            .subfont_index(id)
            .and_then(|index| u8::try_from(index).ok())
            .ok_or_else(|| malformed("the FDSelect names no Font DICT for a glyph"))?;
        glyphs.cids.push(cid.to_u16());
        glyphs.font_dicts.push(font_dict);
    }
    if glyphs.cids.first() != Some(&0) {
        return Err(malformed("glyph 0 is not CID 0"));
    }
    let mut held: BTreeSet<u16> = glyphs.cids.iter().copied().collect();
    for asked in cids {
        if !held.insert(asked.cid) {
            return Err(malformed(
                "a CID asked for is CID 0, one the charset holds, or asked for twice",
            ));
        }
    }
    Ok(glyphs)
}

/// The highest CID a CID-keyed program's charset holds: a CID past it is one
/// [`with_cids_given`] can give a glyph.
///
/// # Errors
///
/// [`CffError::Malformed`] where the program does not open, is name-keyed, or names no CID for a
/// glyph it holds; [`CffError::NoCharset`] where it states no charset.
pub fn highest_cid(data: &[u8]) -> Result<u16, CffError> {
    let font = open(data)?;
    if !font.is_cid() {
        return Err(malformed(
            "the program is name-keyed, and a name-keyed program's glyphs are keyed by name",
        ));
    }
    let charset = font.charset().ok_or(CffError::NoCharset)?;
    let mut highest = 0_u16;
    for glyph in 0..font.num_glyphs() {
        let cid = charset
            .string_id(GlyphId::from(glyph))
            .ok_or_else(|| malformed("the charset names no CID for a glyph the program holds"))?;
        highest = highest.max(cid.to_u16());
    }
    Ok(highest)
}

/// A copy of glyph 0 appended to `glyphs` for each CID in `cids`, at its advance, under glyph 0's
/// Font DICT (`notdef.0`) whose nominal width is `notdef.1`; the glyph index of each, in order.
/// Refused where an advance cannot be restated or the copies pass [`MOST_CID_BYTES_GIVEN`].
fn copies_appended(
    glyphs: &mut Glyphs,
    cids: &[CidToGive],
    notdef: (u8, i64),
    subroutines: &Subroutines<'_>,
) -> Result<Vec<u16>, CffError> {
    let (notdefs_dict, nominal) = notdef;
    let outline = glyphs.charstrings.first().cloned().unwrap_or_default();
    let mut added = Vec::with_capacity(cids.len());
    let mut appended = 0usize;
    for asked in cids {
        let glyph = u16::try_from(glyphs.charstrings.len()).map_err(|_| {
            malformed("the program already holds as many glyphs as an INDEX counts")
        })?;
        let charstring = charstring_with_advance(&outline, asked.advance, nominal, subroutines)
            .map_err(|why| why.refusal(glyph, NOTDEF_NOT_RESTATED))?;
        appended = appended.saturating_add(charstring.len());
        if appended > MOST_CID_BYTES_GIVEN {
            return Err(malformed(
                "the copies of glyph 0 asked for would append more than this writer's bound",
            ));
        }
        glyphs.charstrings.push(charstring);
        glyphs.cids.push(asked.cid);
        glyphs.font_dicts.push(notdefs_dict);
        added.push(glyph);
    }
    Ok(added)
}

/// The Top DICT with `CIDCount` stated as `needed`: the entry restated where there is one, added
/// where the default was in force.
fn cid_count_raised(layout: &mut Layout, needed: i64) {
    let operands = int5(needed).to_vec();
    match layout
        .entries
        .iter_mut()
        .find(|(_, op)| *op == OP_CID_COUNT)
    {
        Some(entry) => entry.0 = operands,
        None => layout.entries.push((operands, OP_CID_COUNT)),
    }
}

/// A charset in format 0 (Adobe Technical Note #5176, section 13): a format byte, then one CID
/// for every glyph after glyph 0, which has none.
fn charset_written(cids: &[u16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(cids.len().saturating_mul(2).saturating_add(1));
    out.push(0);
    for cid in cids.iter().skip(1) {
        out.extend_from_slice(&cid.to_be_bytes());
    }
    out
}

/// An `FDSelect` naming each glyph's Font DICT (Adobe Technical Note #5176, section 19): format 0
/// a byte per glyph, format 3 a range per run of one Font DICT and a sentinel one past the last
/// glyph. `None` for another format, or where a glyph index passes 16 bits.
fn fd_select_written(format: u8, font_dicts: &[u8]) -> Option<Vec<u8>> {
    let mut out = vec![format];
    match format {
        0 => out.extend_from_slice(font_dicts),
        3 => {
            let mut ranges: Vec<(u16, u8)> = Vec::new();
            for (glyph, font_dict) in font_dicts.iter().enumerate() {
                if ranges.last().map(|(_, last)| last) != Some(font_dict) {
                    ranges.push((u16::try_from(glyph).ok()?, *font_dict));
                }
            }
            out.extend_from_slice(&u16::try_from(ranges.len()).ok()?.to_be_bytes());
            for (first, font_dict) in ranges {
                out.extend_from_slice(&first.to_be_bytes());
                out.push(font_dict);
            }
            out.extend_from_slice(&u16::try_from(font_dicts.len()).ok()?.to_be_bytes());
        }
        _ => return None,
    }
    Some(out)
}

/// Whether `rebuilt` reads, by `read-fonts`, as `expected`: the `CharStrings` INDEX byte for
/// byte, the charset's CID and the `FDSelect`'s Font DICT for every glyph, the `FDArray`'s count
/// and each Font DICT's Private DICT and local subroutines byte for byte, the global subroutines,
/// and a `CIDCount` that covers every CID.
fn read_back(rebuilt: &[u8], expected: &Expected<'_>) -> Result<(), CffError> {
    let differs = || CffError::Malformed {
        detail: "the rebuilt program does not read back as it was written".to_owned(),
    };
    let font = open(rebuilt)?;
    let read = font.charstrings();
    if read.count() as usize != expected.charstrings.len() {
        return Err(differs());
    }
    for (at, written) in expected.charstrings.iter().enumerate() {
        if read.get(at) != Some(written.as_slice()) {
            return Err(differs());
        }
    }
    let charset = font.charset().ok_or(CffError::NoCharset)?;
    for (glyph, (cid, font_dict)) in expected.cids.iter().zip(expected.font_dicts).enumerate() {
        let id = GlyphId::from(u32::try_from(glyph).map_err(|_| differs())?);
        if charset.string_id(id).map(Sid::to_u16) != Some(*cid)
            || font.subfont_index(id) != Some(u16::from(*font_dict))
        {
            return Err(differs());
        }
    }
    let layout = Layout::read(rebuilt).ok_or_else(differs)?;
    let dicts =
        index_items(rebuilt, layout.fd_array_at.ok_or_else(differs)?).ok_or_else(differs)?;
    if dicts.len() != expected.privates.len() || font.num_subfonts() as usize != dicts.len() {
        return Err(differs());
    }
    for (dict, (private, local)) in dicts.iter().zip(expected.privates) {
        if private_dict(rebuilt, dict).unwrap_or_default() != private.as_slice()
            || local_subroutines(rebuilt, dict) != *local
        {
            return Err(differs());
        }
    }
    if global_subroutines(rebuilt) != expected.global {
        return Err(differs());
    }
    let cid_count = layout
        .entries
        .iter()
        .find(|(_, op)| *op == OP_CID_COUNT)
        .map_or(Some(DEFAULT_CID_COUNT), |(operands, _)| {
            dict_ints(operands).first().copied()
        });
    if cid_count.is_none_or(|count| count < expected.cid_count) {
        return Err(differs());
    }
    Ok(())
}

#[cfg(test)]
#[path = "cff_cids_given_tests.rs"]
mod tests;
