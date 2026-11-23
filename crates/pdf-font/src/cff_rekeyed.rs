//! A name-keyed CFF program written again as a CID-keyed one whose charset states the glyph a
//! `CIDFontType0`'s `/CIDToGIDMap` selected for each CID (`doc/adr/3446`, after `doc/adr/3428`).
//!
//! ISO 32000-2 §9.7.4.2 decides a `CIDFontType0`'s glyph by what its CFF's Top DICT holds:
//!
//! > The "CFF" font program has a Top DICT that does not use CIDFont operators: The CIDs shall
//! > be used directly as GID values, and the glyph procedure shall be retrieved using the
//! > CharStrings INDEX (refer to Adobe Technical Note #5177, The Type 2 Charstring Format ).
//!
//! and Table 115 gives `/CIDToGIDMap` to a Type 2 `CIDFont`. A producer that pairs a name-keyed
//! program with a map has written a font whose glyphs the clause and the map select differently,
//! and this reader draws by the map ([`crate::composite`] argues why). The program written here
//! makes the two one: glyph 0 is the source's glyph 0 at CID 0, and each CID the map sends to a
//! glyph other than 0 is given, at the next glyph index, a copy of that glyph's charstring and,
//! in a format 0 charset (Adobe Technical Note #5176 section 13), the CID itself — so the
//! clause's first case, "using the charset table in the CFF program", reaches through the
//! charset exactly the glyph the map reached. A glyph two CIDs reach is copied twice.
//!
//! The charstrings are the source's bytes, and so are the Global and Local Subr INDEXes, which
//! keep their counts and therefore their biases (Technical Note #5177 section 4.7); the one Font
//! DICT names the source's Private DICT, every entry restated in its own operand bytes but
//! `Subrs`, which is re-pointed at the Local Subr INDEX written after it. `ROS` is the
//! `CIDFont`'s own `CIDSystemInfo`, `FDSelect` puts every glyph in Font DICT 0, and the Top
//! DICT keeps the source's `FontMatrix` and `FontBBox`.
//!
//! **Nothing is returned that does not read back** (trap 301): the written program's charset
//! must send every CID to the glyph it was given, its units per em must be the source's, and
//! every CID must draw, command for command and coordinate bit for bit, the outline the map's
//! glyph drew, at the width that glyph's charstring stated.

use std::collections::BTreeMap;

use skrifa::outline::OutlinePen;

use super::{
    CffError, ESCAPE, OP_CHARSET, OP_CHARSTRINGS, OP_FD_ARRAY, OP_FD_SELECT, OP_PRIVATE, OP_SUBRS,
    dict_entries, global_subroutines, index_extent, index_item, index_items, int5, open,
    private_dict, push_operator,
};
use crate::embed::SystemInfo;
use crate::embed::cff::index;

/// `ROS` (Technical Note #5176, Table 10).
const OP_ROS: u16 = ESCAPE | 0x1e;
/// `CIDCount` (Table 10).
const OP_CID_COUNT: u16 = ESCAPE | 0x22;
/// `FontMatrix` (Table 9).
const OP_FONT_MATRIX: u16 = ESCAPE | 0x07;
/// `FontBBox` (Table 9).
const OP_FONT_BBOX: u16 = 5;
/// How many strings Technical Note #5176's Appendix A predefines before a program's own.
const STANDARD_STRINGS: i64 = 391;

/// The name-keyed CFF `cff` written as a CID-keyed one whose CID *c* selects the glyph
/// `cid_to_glyph[c]` selected, under `system`'s `ROS`, proved as the module documentation says.
///
/// `cid_to_glyph` is the `/CIDToGIDMap`, read as §9.7.4.2's Table 115 entry states it: the glyph
/// index for CID *c* in its two bytes at *2c*, high byte first.
///
/// # Errors
///
/// [`CffError::Malformed`] where the program does not read, already uses `CIDFont` operators, the
/// map sends a CID to a glyph the program does not hold or to more glyphs than a CFF counts, or
/// the written program does not read back as the source drew; the detail says which.
pub fn cid_keyed_by_map(
    cff: &[u8],
    cid_to_glyph: &[u16],
    system: &SystemInfo,
) -> Result<Vec<u8>, CffError> {
    open(cff)?;
    if super::uses_cid_operators(cff)? {
        return Err(refused("its Top DICT already uses CIDFont operators"));
    }
    let header = usize::from(*cff.get(2).ok_or_else(|| refused("it has no header"))?);
    let name_extent = index_extent(cff, header).ok_or_else(|| refused("no Name INDEX"))?;
    let name = index_item(cff, header, 0).ok_or_else(|| refused("no font name"))?;
    let top = index_item(cff, name_extent.end, 0).ok_or_else(|| refused("no Top DICT"))?;
    let top_entries = dict_entries(top).ok_or_else(|| refused("its Top DICT does not read"))?;
    let charstrings_at = top_entries
        .iter()
        .find(|(_, op)| *op == OP_CHARSTRINGS)
        .and_then(|(operands, _)| super::dict_int(operands))
        .and_then(|at| usize::try_from(at).ok())
        .ok_or_else(|| refused("its Top DICT names no CharStrings"))?;
    let charstrings = index_items(cff, charstrings_at)
        .ok_or_else(|| refused("its CharStrings INDEX does not read"))?;
    let private =
        private_dict(cff, top).ok_or_else(|| refused("its Private DICT does not read"))?;
    let local = super::local_subroutines(cff, top);
    let global = global_subroutines(cff);

    // Glyph 0 stays glyph 0 at CID 0; every other CID the map sends somewhere gets a glyph.
    let mut given: Vec<(u16, u16)> = Vec::new();
    for (cid, &glyph) in cid_to_glyph.iter().enumerate().skip(1) {
        if glyph == 0 {
            continue;
        }
        if usize::from(glyph) >= charstrings.len() {
            return Err(refused(&format!(
                "CID {cid} maps to glyph {glyph}, past the program's {} glyphs",
                charstrings.len()
            )));
        }
        let cid = u16::try_from(cid).map_err(|_| refused("the map passes CID 65 535"))?;
        given.push((cid, glyph));
    }
    let glyph_zero = charstrings
        .first()
        .ok_or_else(|| refused("it holds no glyph"))?;
    let mut ordered: Vec<&[u8]> = vec![glyph_zero];
    for &(_, glyph) in &given {
        ordered.push(charstrings[usize::from(glyph)]);
    }
    let cid_count = given
        .last()
        .map_or(1, |&(cid, _)| i64::from(cid).saturating_add(1));
    let carried: Vec<(Vec<u8>, u16)> = top_entries
        .into_iter()
        .filter(|(_, op)| matches!(*op, OP_FONT_MATRIX | OP_FONT_BBOX))
        .collect();
    let written = written(
        name,
        system,
        &Parts {
            carried: &carried,
            cid_count,
            cids: &given.iter().map(|&(cid, _)| cid).collect::<Vec<_>>(),
            charstrings: &ordered,
            private,
            local: &local,
            global: &global,
        },
    )?;
    proved(cff, &written, &given)?;
    Ok(written)
}

/// The name-keyed CFF a `/FontFile3` holds, bare or as an `OpenType` program's `CFF ` table, or
/// `None` where it holds another program or a CFF whose Top DICT uses `CIDFont` operators — the
/// program §9.7.4.2's second CFF case selects glyphs from by CID alone.
#[must_use]
pub fn name_keyed_cff(program: &[u8]) -> Option<std::borrow::Cow<'_, [u8]>> {
    let cff = if program.first() == Some(&1) {
        std::borrow::Cow::Borrowed(program)
    } else {
        std::borrow::Cow::Owned(crate::sfnt::table(program, *b"CFF ")?.to_vec())
    };
    (super::uses_cid_operators(&cff) == Ok(false)).then_some(cff)
}

/// What [`written`] lays out.
struct Parts<'a> {
    /// The source Top DICT's entries carried over: `FontMatrix` and `FontBBox`.
    carried: &'a [(Vec<u8>, u16)],
    /// `CIDCount`.
    cid_count: i64,
    /// The CID of each glyph after glyph 0, in glyph order.
    cids: &'a [u16],
    /// Every charstring, glyph 0 first.
    charstrings: &'a [&'a [u8]],
    /// The source's Private DICT.
    private: &'a [u8],
    /// The source's Local Subr INDEX entries.
    local: &'a [Vec<u8>],
    /// The source's Global Subr INDEX entries.
    global: &'a [Vec<u8>],
}

/// The program, laid out as Technical Note #5176 section 18 lays a CID-keyed font out.
fn written(name: &[u8], system: &SystemInfo, parts: &Parts<'_>) -> Result<Vec<u8>, CffError> {
    let too_large = || refused("the program is larger than a CFF's offsets state");
    let count = u16::try_from(parts.charstrings.len()).map_err(|_| too_large())?;
    let name_index = index(&[name]).ok_or_else(too_large)?;
    let string_index =
        index(&[system.registry.as_slice(), system.ordering.as_slice()]).ok_or_else(too_large)?;
    let global_index = index(parts.global).ok_or_else(too_large)?;
    // Section 13's format 0: a CID per glyph after glyph 0.
    let mut charset = vec![0u8];
    for cid in parts.cids {
        charset.extend_from_slice(&cid.to_be_bytes());
    }
    // Section 19's format 3: one range, every glyph in Font DICT 0.
    let mut fd_select = vec![3, 0, 1, 0, 0, 0];
    fd_select.extend_from_slice(&count.to_be_bytes());
    let charstrings_index = index(parts.charstrings).ok_or_else(too_large)?;
    let local_index = index(parts.local).ok_or_else(too_large)?;
    let private = private_written(parts.private, !parts.local.is_empty())?;

    let top_length = top_dict(system, parts, [0; 4]).len();
    let top_index_length = index(&[vec![0u8; top_length]]).ok_or_else(too_large)?.len();
    let header = [1u8, 0, 4, 4];
    let charset_at = [
        header.len(),
        name_index.len(),
        top_index_length,
        string_index.len(),
        global_index.len(),
    ]
    .iter()
    .sum::<usize>();
    let fd_select_at = charset_at.saturating_add(charset.len());
    let charstrings_at = fd_select_at.saturating_add(fd_select.len());
    let fd_array_at = charstrings_at.saturating_add(charstrings_index.len());
    let fd_array_length = index(&[font_dict(0, 0)]).ok_or_else(too_large)?.len();
    let private_at = fd_array_at.saturating_add(fd_array_length);
    let offset = |at: usize| {
        i64::try_from(at)
            .ok()
            .filter(|at| *at <= i64::from(i32::MAX))
            .ok_or_else(too_large)
    };
    let placed = [
        offset(charset_at)?,
        offset(fd_select_at)?,
        offset(charstrings_at)?,
        offset(fd_array_at)?,
    ];
    let top_index = index(&[top_dict(system, parts, placed)]).ok_or_else(too_large)?;
    let fd_array =
        index(&[font_dict(offset(private.len())?, offset(private_at)?)]).ok_or_else(too_large)?;
    let mut out = Vec::new();
    for part in [
        &header[..],
        &name_index,
        &top_index,
        &string_index,
        &global_index,
        &charset,
        &fd_select,
        &charstrings_index,
        &fd_array,
        &private,
    ] {
        out.extend_from_slice(part);
    }
    if !parts.local.is_empty() {
        out.extend_from_slice(&local_index);
    }
    Ok(out)
}

/// The source's Private DICT, each entry in its own operand bytes, with `Subrs` dropped and —
/// where there are local subroutines — stated last as the DICT's own length, so the Local Subr
/// INDEX follows it directly.
fn private_written(private: &[u8], with_subrs: bool) -> Result<Vec<u8>, CffError> {
    let mut out = Vec::new();
    for (operands, op) in
        dict_entries(private).ok_or_else(|| refused("its Private DICT does not read"))?
    {
        if op == OP_SUBRS {
            continue;
        }
        out.extend_from_slice(&operands);
        push_operator(&mut out, op);
    }
    if with_subrs {
        // `int5` is five bytes and `Subrs` one, so the DICT ends six bytes on.
        let length =
            i64::try_from(out.len().saturating_add(6)).map_err(|_| refused("its Private DICT"))?;
        out.extend_from_slice(&int5(length));
        push_operator(&mut out, OP_SUBRS);
    }
    Ok(out)
}

/// The Top DICT: `ROS` first, as section 18 requires, then `CIDCount`, the source's `FontMatrix`
/// and `FontBBox`, and the four offsets `placed` holds — `charset`, `FDSelect`, `CharStrings`,
/// `FDArray`.
fn top_dict(system: &SystemInfo, parts: &Parts<'_>, placed: [i64; 4]) -> Vec<u8> {
    let [charset, fd_select, charstrings, fd_array] = placed;
    let mut out = Vec::new();
    for operand in [STANDARD_STRINGS, STANDARD_STRINGS + 1, system.supplement] {
        out.extend_from_slice(&int5(operand));
    }
    push_operator(&mut out, OP_ROS);
    out.extend_from_slice(&int5(parts.cid_count));
    push_operator(&mut out, OP_CID_COUNT);
    for (operands, op) in parts.carried {
        out.extend_from_slice(operands);
        push_operator(&mut out, *op);
    }
    for (operand, operator) in [
        (charset, OP_CHARSET),
        (fd_select, OP_FD_SELECT),
        (charstrings, OP_CHARSTRINGS),
        (fd_array, OP_FD_ARRAY),
    ] {
        out.extend_from_slice(&int5(operand));
        push_operator(&mut out, operator);
    }
    out
}

/// The one Font DICT: its Private DICT's size and offset.
fn font_dict(size: i64, at: i64) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&int5(size));
    out.extend_from_slice(&int5(at));
    push_operator(&mut out, OP_PRIVATE);
    out
}

/// Whether `written` selects, through its charset, and draws for every CID of `given` the glyph
/// the source's map selected, at the same width.
fn proved(source: &[u8], written: &[u8], given: &[(u16, u16)]) -> Result<(), CffError> {
    let unread = |e: &CffError| refused(&format!("the written program does not read back: {e}"));
    let by_cid = match super::CodeToGlyph::read(written).map_err(|e| unread(&e))? {
        super::CodeToGlyph::Keyed { by_cid } => by_cid,
        super::CodeToGlyph::Named(_) => {
            return Err(refused("the written program is not CID-keyed"));
        }
    };
    let expected: BTreeMap<u16, u16> = given
        .iter()
        .zip(1u16..)
        .map(|(&(cid, _), glyph)| (cid, glyph))
        .chain([(0, 0)])
        .collect();
    if by_cid != expected {
        return Err(refused(
            "the written charset does not send each CID to its glyph",
        ));
    }
    let units = super::units_per_em(written).map_err(|e| unread(&e))?;
    if units.to_bits() != super::units_per_em(source)?.to_bits() {
        return Err(refused(
            "the written program's units per em are not the source's",
        ));
    }
    let mut pairs: Vec<(u16, u16, u16)> = vec![(0, 0, 0)];
    pairs.extend(
        given
            .iter()
            .zip(1u16..)
            .map(|(&(cid, from), to)| (cid, from, to)),
    );
    let from: Vec<u16> = pairs.iter().map(|&(_, glyph, _)| glyph).collect();
    let to: Vec<u16> = pairs.iter().map(|&(_, _, glyph)| glyph).collect();
    let widths_from = super::charstring_widths(source, &from)?;
    let widths_to = super::charstring_widths(written, &to).map_err(|e| unread(&e))?;
    for (index, &(cid, from, to)) in pairs.iter().enumerate() {
        let mut before = Recorded::default();
        super::draw(source, from, &mut before)?;
        let mut after = Recorded::default();
        super::draw(written, to, &mut after).map_err(|e| unread(&e))?;
        if before.0 != after.0 {
            return Err(refused(&format!("CID {cid} draws another outline")));
        }
        let width = |widths: &[Option<f32>]| widths.get(index).copied().flatten().map(f32::to_bits);
        if width(&widths_from) != width(&widths_to) {
            return Err(refused(&format!("CID {cid} states another width")));
        }
    }
    Ok(())
}

/// One outline as its pen received it, each coordinate as the bits of its `f32`.
#[derive(Default)]
struct Recorded(Vec<(u8, [u32; 6])>);

impl OutlinePen for Recorded {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push((0, [x.to_bits(), y.to_bits(), 0, 0, 0, 0]));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push((1, [x.to_bits(), y.to_bits(), 0, 0, 0, 0]));
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.0.push((2, [cx, cy, x, y, 0.0, 0.0].map(f32::to_bits)));
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0
            .push((3, [cx0, cy0, cx1, cy1, x, y].map(f32::to_bits)));
    }
    fn close(&mut self) {
        self.0.push((4, [0; 6]));
    }
}

/// The program refused, saying why.
fn refused(detail: &str) -> CffError {
    CffError::Malformed {
        detail: format!("not written CID-keyed: {detail}"),
    }
}

#[cfg(test)]
#[path = "cff_rekeyed_tests.rs"]
mod tests;
