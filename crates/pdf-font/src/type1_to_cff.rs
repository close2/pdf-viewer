//! A bare Type 1 program written again as the CID-keyed CFF ISO 32000-2 §9.9's Table 124 files
//! under a `CIDFontType0` (ADR 3155, built by ADR 3173).
//!
//! Table 124 gives a `CIDFontType0` dictionary two programs, a CFF under `/FontFile3` `/Subtype
//! /CIDFontType0C` and an OpenType one, and a Type 1 program under `/FontFile` is neither. This
//! reader draws such a font by §9.7.4.2's rule for a CFF without `CIDFont` operators, read through
//! §9.6.2.1's NOTE 1 (`crate::composite`): a CID is the index of a charstring in the program's
//! order. So the program this writes holds, at glyph index *n*, the charstring that draws what the
//! Type 1 program's glyph *n* draws, and its charset gives glyph *n* the CID *n* — the glyph every
//! CID selected before is the glyph it selects after.
//!
//! # How a charstring is translated
//!
//! **Each Type 1 charstring is run through the tree's own interpreter**, the one the window draws
//! with (`read_fonts::ps::cs`, reached through [`Program`]), and what it draws is written down as
//! a Type 2 charstring (Adobe Technical Note #5177) of `rmoveto`, `rlineto` and `rrcurveto`, each
//! point relative to the one before in the note's 16.16 operand form, after the width the
//! charstring's `hsbw` or `sbw` stated and before `endchar`. So everything the Type 1 Font Format
//! gives a charstring to compute its path with is carried as the path it computes: `callsubr`
//! inlined, `div` evaluated, the side bearing in the first point, the flex of `OtherSubrs` 0 and 1
//! as the two curves it draws, `closepath` as the Type 2 rule that a subpath closes at the next
//! `rmoveto` or `endchar`. `seac` is carried as its two components' paths, the accent moved by its
//! offset, because the note's `endchar` form of it reaches the components through
//! `StandardEncoding` and a charset of glyph *names*, and a CID-keyed charset holds CIDs (ADR
//! 1486 makes the same move for a subset).
//!
//! **The translated charstrings carry no stem hints**, which is the cost (ADR 3173): hints decide
//! how an outline is fitted to a coarse pixel grid and never the outline itself, and the
//! interpreter this reads through reports Type 1's `hstem3`, `vstem3` and the hint replacement of
//! `OtherSubrs` 3 to no sink, so stems read through it would be a subset of the program's that the
//! Private DICT's blue zones would then be applied over. The Private DICT is written with no
//! hinting values for the same reason.
//!
//! # The proof before a program is returned
//!
//! The written program is read back by the readers the window draws a `CIDFontType0C` with
//! ([`crate::cff`]): its charset must send every CID *n* to glyph *n*, its units per em must be
//! the Type 1 program's, and every glyph must draw, command for command and coordinate bit for
//! bit, the outline the Type 1 program's glyph drew, at the width it stated. A glyph that does
//! not is refused by its index and name, and nothing is returned.

use skrifa::GlyphId;
use skrifa::outline::OutlinePen;
use skrifa::raw::ps::transform::FontMatrix;

use super::{Program, Type1Error};
use crate::cff::{ESCAPE, OP_FD_ARRAY, OP_FD_SELECT, int5, push_operator};
use crate::embed::SystemInfo;
use crate::embed::accented::{Path, push_operand};
use crate::embed::cff::index;

/// The most glyphs the written program holds: a CFF INDEX counts its entries in 16 bits, and a
/// charset and `FDSelect` number glyphs in 16 (Adobe Technical Note #5176, sections 5, 13, 19).
pub const MOST_GLYPHS: u32 = 65_535;

/// `endchar` (Technical Note #5177, Appendix A).
const ENDCHAR: u8 = 14;
/// How many strings Technical Note #5176's Appendix A predefines before a program's own.
const STANDARD_STRINGS: i64 = 391;
/// `ROS` (Technical Note #5176, Table 10).
const OP_ROS: u16 = ESCAPE | 0x1e;
/// `CIDCount` (Table 10).
const OP_CID_COUNT: u16 = ESCAPE | 0x22;
/// `charset` (Table 9).
const OP_CHARSET: u16 = 15;
/// `CharStrings` (Table 9).
const OP_CHARSTRINGS: u16 = 17;
/// `Private` (Table 9).
const OP_PRIVATE: u16 = 18;
/// The Private DICT: `0 nominalWidthX` (Table 23), so that a charstring's width operand is its
/// width. It is also the operator's default, and is written so that the DICT is not empty.
const PRIVATE: [u8; 2] = [139, 21];

/// The Type 1 program `data` written as a CID-keyed CFF whose `ROS` is `system`, glyph *n* at CID
/// *n*, proved as the module documentation describes.
///
/// # Errors
///
/// [`Type1Error::Malformed`] where the program does not read; [`Type1Error::NotWritten`] naming
/// why the program cannot be written this way — a `FontMatrix` other than the 1000-unit one, more
/// glyphs than [`MOST_GLYPHS`], a charstring the interpreter refuses or whose points leave the
/// 16.16 range, or a glyph the read-back does not draw as the source did, by its index and name.
pub fn as_cid_keyed_cff(data: &[u8], system: &SystemInfo) -> Result<Vec<u8>, Type1Error> {
    let program = Program::parse(data)?;
    // The written Top DICT states no `FontMatrix`, so it takes the format's default, the
    // 1000-unit one; a program stating another is refused rather than written in other units.
    if program.font.upem() != 1000 || program.font.matrix() != FontMatrix::IDENTITY {
        return Err(not_written(
            "its FontMatrix is not the 1000-unit one a CFF defaults to",
        ));
    }
    let count = program.font.num_glyphs();
    if count == 0 || count > MOST_GLYPHS {
        return Err(not_written(&format!(
            "it holds {count} glyphs, and a CID-keyed CFF holds 1 to {MOST_GLYPHS}"
        )));
    }
    let glyphs = 0..u16::try_from(count).map_err(|_| not_written("too many glyphs"))?;
    let charstrings = glyphs
        .clone()
        .map(|glyph| translated(&program, glyph))
        .collect::<Result<Vec<_>, _>>()?;
    let name = program
        .font
        .name()
        .ok_or_else(|| not_written("it states no FontName for the Name INDEX"))?;
    let cff = written(name.as_bytes(), system, &charstrings)?;
    proved(&program, &cff, glyphs)?;
    Ok(cff)
}

/// Glyph `glyph`'s charstring in the Type 2 form: its width, the path it draws, `endchar`; refused
/// where it states no width.
fn translated(program: &Program, glyph: u16) -> Result<Vec<u8>, Type1Error> {
    let mut path = Path::default();
    let width = program
        .font
        .evaluate_charstring(GlyphId::from(glyph), &mut path)
        .map_err(|e| refused(program, glyph, &e.to_string()))?;
    if path.overflowed {
        return Err(refused(
            program,
            glyph,
            "a point is farther from the one before than a 16.16 operand states",
        ));
    }
    // Section 6.4 opens every charstring with `hsbw` or `sbw`. A Type 2 charstring that omits
    // its width has the Private DICT's `defaultWidthX` (Technical Note #5177 section 3.1), so one
    // written without would state a width the source never stated.
    let width =
        width.ok_or_else(|| refused(program, glyph, "it states no width with hsbw or sbw"))?;
    let mut out = Vec::with_capacity(path.code.len().saturating_add(6));
    push_operand(&mut out, width.to_bits());
    out.extend_from_slice(&path.code);
    out.push(ENDCHAR);
    Ok(out)
}

/// The program's bytes, in Technical Note #5176 section 2's order: header, Name INDEX, Top DICT
/// INDEX, String INDEX, an empty Global Subr INDEX, then the charset, `FDSelect`, `CharStrings`,
/// the one Font DICT and its Private DICT.
///
/// Every offset is a five-byte integer, so each DICT's length is known before any offset is and
/// the layout is computed once (the note's own way out of the circularity, as
/// `crate::embed::cff`'s subsets take it).
fn written(
    name: &[u8],
    system: &SystemInfo,
    charstrings: &[Vec<u8>],
) -> Result<Vec<u8>, Type1Error> {
    let too_large = || not_written("the program is larger than a CFF's offsets state");
    let count = charstrings.len();
    let count16 = u16::try_from(count).map_err(|_| too_large())?;
    let name_index = index(&[name]).ok_or_else(too_large)?;
    let string_index =
        index(&[system.registry.as_slice(), system.ordering.as_slice()]).ok_or_else(too_large)?;
    let global_index = index::<&[u8]>(&[]).ok_or_else(too_large)?;
    // Section 13's format 2: one range, CIDs 1 to `count - 1` at glyphs 1 to `count - 1` (glyph 0
    // is CID 0 and is not listed); format 0 with no entries where there is glyph 0 alone.
    let charset = match count16.checked_sub(2) {
        Some(left) => {
            let mut out = vec![2, 0, 1];
            out.extend_from_slice(&left.to_be_bytes());
            out
        }
        None => vec![0],
    };
    // Section 19's format 3: one range, every glyph in Font DICT 0.
    let mut fd_select = vec![3, 0, 1, 0, 0, 0];
    fd_select.extend_from_slice(&count16.to_be_bytes());
    let charstrings_index = index(charstrings).ok_or_else(too_large)?;

    let top_length = top_dict(system, count, [0; 4]).len();
    let top_index_length = index(&[vec![0u8; top_length]]).ok_or_else(too_large)?.len();
    let header = [1u8, 0, 4, 4];
    let lengths = [
        header.len(),
        name_index.len(),
        top_index_length,
        string_index.len(),
        global_index.len(),
    ];
    let charset_at = lengths.iter().sum::<usize>();
    let fd_select_at = charset_at.saturating_add(charset.len());
    let charstrings_at = fd_select_at.saturating_add(fd_select.len());
    let fd_array_at = charstrings_at.saturating_add(charstrings_index.len());
    let font_dict_length = font_dict(0).len();
    let fd_array_length = index(&[vec![0u8; font_dict_length]])
        .ok_or_else(too_large)?
        .len();
    let private_at = fd_array_at.saturating_add(fd_array_length);
    // not a repair: an offset of this writer's that a DICT cannot state refuses the program
    // (`too_large`), a decline.
    let offset = |at: usize| {
        i64::try_from(at)
            .ok()
            .filter(|at| *at <= i64::from(i32::MAX))
    };
    let [
        Some(charset_at),
        Some(fd_select_at),
        Some(charstrings_at),
        Some(fd_array_at),
    ] = [charset_at, fd_select_at, charstrings_at, fd_array_at].map(offset)
    else {
        return Err(too_large());
    };
    let placed = [charset_at, fd_select_at, charstrings_at, fd_array_at];
    let private_at = offset(private_at).ok_or_else(too_large)?;

    let top_index = index(&[top_dict(system, count, placed)]).ok_or_else(too_large)?;
    let fd_array = index(&[font_dict(private_at)]).ok_or_else(too_large)?;
    // not a repair: a capacity hint, of an offset `offset` has already held to `i32`'s range.
    let mut out = Vec::with_capacity(
        usize::try_from(private_at)
            .unwrap_or_default()
            .saturating_add(PRIVATE.len()),
    );
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
        &PRIVATE,
    ] {
        out.extend_from_slice(part);
    }
    Ok(out)
}

/// The Top DICT: `ROS` first, as Technical Note #5176 section 18 requires of a CID-keyed font,
/// then `CIDCount` and the four offsets `placed` holds — `charset`, `FDSelect`, `CharStrings`,
/// `FDArray`.
fn top_dict(system: &SystemInfo, count: usize, placed: [i64; 4]) -> Vec<u8> {
    let [charset, fd_select, charstrings, fd_array] = placed;
    let mut out = Vec::new();
    // The String INDEX holds the registry, then the ordering.
    for operand in [STANDARD_STRINGS, STANDARD_STRINGS + 1, system.supplement] {
        out.extend_from_slice(&int5(operand));
    }
    push_operator(&mut out, OP_ROS);
    for (operand, operator) in [
        // not a repair: a glyph count, which a `u16` glyph index bounds, always fits.
        (i64::try_from(count).unwrap_or(i64::MAX), OP_CID_COUNT),
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
fn font_dict(private_at: i64) -> Vec<u8> {
    let mut out = Vec::new();
    // not a repair: the length of this writer's own fixed Private DICT, which always fits.
    out.extend_from_slice(&int5(i64::try_from(PRIVATE.len()).unwrap_or_default()));
    out.extend_from_slice(&int5(private_at));
    push_operator(&mut out, OP_PRIVATE);
    out
}

/// Whether the written program `cff` selects and draws every glyph as `program` does.
fn proved(program: &Program, cff: &[u8], glyphs: std::ops::Range<u16>) -> Result<(), Type1Error> {
    let unread =
        |e: &dyn std::fmt::Display| not_written(&format!("its CFF does not read back: {e}"));
    match crate::cff::CodeToGlyph::read(cff).map_err(|e| unread(&e))? {
        crate::cff::CodeToGlyph::Keyed { by_cid }
            if by_cid.len() == usize::from(glyphs.end)
                && by_cid.iter().all(|(cid, glyph)| cid == glyph) => {}
        _ => {
            return Err(not_written(
                "its CFF's charset does not send each CID to its glyph",
            ));
        }
    }
    let units = crate::cff::units_per_em(cff).map_err(|e| unread(&e))?;
    if units.to_bits() != program.units_per_em()?.to_bits() {
        return Err(not_written("its CFF's units per em are not the program's"));
    }
    let indices: Vec<u16> = glyphs.clone().collect();
    let widths = crate::cff::charstring_widths(cff, &indices).map_err(|e| unread(&e))?;
    for (glyph, width) in glyphs.zip(widths) {
        let mut source = Recorded::default();
        program.draw(glyph, &mut source)?;
        let mut drawn = Recorded::default();
        crate::cff::draw(cff, glyph, &mut drawn).map_err(|e| unread(&e))?;
        if source.0 != drawn.0 {
            return Err(refused(
                program,
                glyph,
                "its CFF charstring draws another outline",
            ));
        }
        let stated = program.advance(glyph)?;
        if stated.map(f32::to_bits) != width.map(f32::to_bits) {
            return Err(refused(
                program,
                glyph,
                "its CFF charstring states another width",
            ));
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
        let points = [cx, cy, x, y, 0.0, 0.0].map(f32::to_bits);
        self.0.push((2, points));
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0
            .push((3, [cx0, cy0, cx1, cy1, x, y].map(f32::to_bits)));
    }
    fn close(&mut self) {
        self.0.push((4, [0; 6]));
    }
}

/// A glyph refused by its index and name.
fn refused(program: &Program, glyph: u16, why: &str) -> Type1Error {
    // not a repair: the words of a refusal's message, not a value read in the file's place.
    let name = program
        .font
        .glyph_name(GlyphId::from(glyph))
        .unwrap_or("(no name)");
    not_written(&format!("glyph {glyph} ({name}): {why}"))
}

/// The program as a whole refused, saying why.
fn not_written(detail: &str) -> Type1Error {
    Type1Error::NotWritten {
        detail: format!("as a CID-keyed CFF: {detail}"),
    }
}

#[cfg(test)]
#[path = "type1_to_cff_tests.rs"]
mod tests;
