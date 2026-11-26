//! A DICT real operand that states no digit, found, and restated as the zero this reader reads.
//!
//! Adobe Technical Note #5176, section 4, encodes a real DICT operand as the byte 30 followed by
//! nibbles: the digits 0 to 9, a decimal point, `E`, `E-`, a minus sign, and 0xf ending the
//! number. The grammar it describes is a decimal number, so a real whose nibbles reach an exponent
//! or the end without a single digit — `1e ff`, `1e fc`, `1e c5 77 02` — states no number at all.
//! ISO 32000-2 §9.9's Table 124 holds a `/FontFile3` `/Type1C` or `/CIDFontType0C` program to that
//! note, and ISO 19005-2 section 6.2.11.2 and ISO 19005-4 section 6.2.10.2 hold a conforming file
//! to it.
//!
//! **What this reader takes such an operand to be is zero.** `read-fonts`, which draws every CFF
//! glyph here, composes a real from its digits and returns zero where it composed none (its
//! `BcdComponents::value`, after `FreeType`'s `cff_parse_real`), and it ends the operand at the byte
//! holding the nibble it stops at — the 0xf, or the digit that takes an exponent past 1000, which
//! [`super::real_width`] mirrors. So restating each such operand as zero digits ended by 0xf, **in
//! the same bytes**, writes the value this reader drew with and leaves every byte it tokenises
//! after the operand where it was: no INDEX offset, Private DICT offset or `Subrs` offset moves, and
//! every other byte of the program is unchanged. A reader that refuses the digitless form — veraPDF
//! parses the nibbles as a decimal string and stops at the first 0xf — then reads the same zero
//! over the same bytes this reader does (ADR 3426).
//!
//! The DICTs read are the Top DICT, the Private DICT it locates, and for a CID-keyed program every
//! Font DICT its `FDArray` holds and the Private DICT each locates: the DICTs whose operands decide
//! a glyph's outline or metrics. A DICT is tokenised the way `read-fonts` tokenises it — an
//! operator byte that names no operator is one byte, as `FreeType` ignores it — and not by this
//! crate's stricter [`super::dict_entries`], which refuses such a DICT for the writers that restate
//! one whole; this module restates only an operand's own bytes.

use std::ops::Range;

use super::{CffError, ESCAPE, OP_FD_ARRAY, OP_PRIVATE, index_offset, real_width};

/// Which DICT of a CFF program an operand is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictKind {
    /// The program's Top DICT (Adobe Technical Note #5176, section 9).
    Top,
    /// A Private DICT, the Top DICT's or a Font DICT's (section 15).
    Private,
    /// A Font DICT of a CID-keyed program's `FDArray` (section 19).
    Font,
}

/// A real operand that states no digit before its exponent or its end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigitlessReal {
    /// The DICT it is in.
    pub dict: DictKind,
    /// The operator it is an operand of, as `0x0c00 + second byte` for the two-byte form; `None`
    /// where the DICT ends before an operator does.
    pub operator: Option<u16>,
    /// Where in the program its bytes are, the leading 30 included.
    pub bytes: Range<usize>,
}

/// Every real operand of the program's Top, Font and Private DICTs that states no digit.
///
/// # Errors
///
/// [`CffError::Malformed`] where the header, the Name INDEX or the Top DICT INDEX cannot be
/// located. A Private DICT or an `FDArray` that cannot be located is not read, which leaves its
/// operands unreported rather than guessed at.
pub fn digitless_reals(data: &[u8]) -> Result<Vec<DigitlessReal>, CffError> {
    let unread = || CffError::Malformed {
        detail: "the Top DICT INDEX could not be located".to_owned(),
    };
    let header = usize::from(*data.get(2).ok_or_else(unread)?);
    let names = index_ranges(data, header).ok_or_else(unread)?;
    let after_names = names.end;
    let tops = index_ranges(data, after_names).ok_or_else(unread)?;
    let top = tops.items.first().cloned().ok_or_else(unread)?;
    let mut found = Vec::new();
    let top_tokens = tokens(data, top.clone());
    read_reals(&top_tokens, DictKind::Top, &mut found);
    if let Some(private) = private_of(&top_tokens) {
        read_reals(&tokens(data, private), DictKind::Private, &mut found);
    }
    // not a repair: a decline of this finder's: an `FDArray` offset that is negative locates no
    // Font DICT, so none is searched there, and no value stands in for the file's.
    let fd_array = top_tokens
        .iter()
        .position(|token| matches!(token, Token::Operator(op) if *op == OP_FD_ARRAY))
        .and_then(|at| last_int(&top_tokens[..at]))
        .and_then(|at| usize::try_from(at).ok())
        .and_then(|at| index_ranges(data, at));
    for font in fd_array.map_or_else(Vec::new, |index| index.items) {
        let font_tokens = tokens(data, font);
        read_reals(&font_tokens, DictKind::Font, &mut found);
        if let Some(private) = private_of(&font_tokens) {
            read_reals(&tokens(data, private), DictKind::Private, &mut found);
        }
    }
    Ok(found)
}

/// The program with every [`digitless_reals`] operand restated as zero in the same bytes, or
/// `None` where it holds none.
///
/// # Errors
///
/// As [`digitless_reals`].
pub fn with_digitless_reals_restated(data: &[u8]) -> Result<Option<Vec<u8>>, CffError> {
    let found = digitless_reals(data)?;
    if found.is_empty() {
        return Ok(None);
    }
    let mut out = data.to_vec();
    for real in &found {
        // The 30 stays; the nibbles after it become zero digits ended by 0xf in the last byte,
        // which `read-fonts` and a decimal-string reader both read as 0 over the same width.
        let nibbles = real.bytes.start.saturating_add(1)..real.bytes.end;
        let last = real.bytes.end.saturating_sub(1);
        for at in nibbles {
            if let Some(byte) = out.get_mut(at) {
                *byte = if at == last { 0x0f } else { 0x00 };
            }
        }
    }
    Ok(Some(out))
}

/// One token of a DICT, as `read-fonts` tokenises it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    /// An integer operand and its value.
    Integer(i64),
    /// A real operand: its bytes in the program, and whether a digit precedes its exponent or end.
    Real(Range<usize>, bool),
    /// An operator, the two-byte form as `ESCAPE | second byte`, a byte naming none included.
    Operator(u16),
}

/// The tokens of the DICT at `range`, stopping where an operand runs past it.
fn tokens(data: &[u8], range: Range<usize>) -> Vec<Token> {
    let mut out = Vec::new();
    let Some(dict) = data.get(range.clone()) else {
        return out;
    };
    let mut at = 0usize;
    while let Some(&b0) = dict.get(at) {
        let next = at.saturating_add(1);
        let (token, width) = match b0 {
            12 => match dict.get(next) {
                Some(&b1) => (Token::Operator(ESCAPE | u16::from(b1)), 2),
                None => break,
            },
            // not a repair: an operand the DICT's end cuts short ends the tokens, a decline.
            28 => match dict
                .get(next..at.saturating_add(3))
                .and_then(|bytes| <[u8; 2]>::try_from(bytes).ok())
            {
                Some(bytes) => (Token::Integer(i64::from(i16::from_be_bytes(bytes))), 3),
                None => break,
            },
            // not a repair: as above, a cut operand ends the tokens.
            29 => match dict
                .get(next..at.saturating_add(5))
                .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
            {
                Some(bytes) => (Token::Integer(i64::from(i32::from_be_bytes(bytes))), 5),
                None => break,
            },
            30 => {
                let Some(width) = dict
                    .get(next..)
                    .and_then(real_width)
                    .and_then(|width| width.checked_add(1))
                else {
                    break;
                };
                let start = range.start.saturating_add(at);
                let nibbles = dict.get(next..at.saturating_add(width));
                (
                    Token::Real(
                        start..start.saturating_add(width),
                        nibbles.is_some_and(states_a_digit),
                    ),
                    width,
                )
            }
            32..=254 => {
                // not a repair: an operand whose second byte is past the DICT's end is refused by
                // the width test below, so the 0 is never read as a value.
                let b1 = dict.get(next).copied().unwrap_or(0);
                let (value, width) = super::small_operand(b0, b1);
                if at.saturating_add(width) > dict.len() {
                    break;
                }
                (Token::Integer(value), width)
            }
            _ => (Token::Operator(u16::from(b0)), 1),
        };
        out.push(token);
        at = at.saturating_add(width);
    }
    out
}

/// Whether a real's nibbles state a digit before its exponent or its end.
fn states_a_digit(nibbles: &[u8]) -> bool {
    for byte in nibbles {
        for nibble in [byte >> 4, byte & 0x0f] {
            match nibble {
                0..=9 => return true,
                0x0b | 0x0c | 0x0d | 0x0f => return false,
                _ => {}
            }
        }
    }
    false
}

/// Records every digitless real among `tokens` with the operator it is an operand of.
fn read_reals(tokens: &[Token], dict: DictKind, found: &mut Vec<DigitlessReal>) {
    for (at, token) in tokens.iter().enumerate() {
        if let Token::Real(bytes, false) = token {
            let operator = tokens.get(at..).and_then(|rest| {
                rest.iter().find_map(|token| match token {
                    Token::Operator(op) => Some(*op),
                    _ => None,
                })
            });
            found.push(DigitlessReal {
                dict,
                operator,
                bytes: bytes.clone(),
            });
        }
    }
}

/// The byte range of the Private DICT a Top or Font DICT's `Private` entry locates.
fn private_of(tokens: &[Token]) -> Option<Range<usize>> {
    let at = tokens
        .iter()
        .position(|token| matches!(token, Token::Operator(op) if *op == OP_PRIVATE))?;
    let operands = tokens.get(..at)?;
    let [.., Token::Integer(size), Token::Integer(offset)] = operands else {
        return None;
    };
    // not a repair: a negative size or offset locates no Private DICT, a decline.
    let size = usize::try_from(*size).ok()?;
    let offset = usize::try_from(*offset).ok()?;
    Some(offset..offset.checked_add(size)?)
}

/// The last integer operand among `tokens`.
fn last_int(tokens: &[Token]) -> Option<i64> {
    match tokens.last()? {
        Token::Integer(value) => Some(*value),
        _ => None,
    }
}

/// An INDEX's items as byte ranges of the program, and where the INDEX ends.
struct IndexRanges {
    items: Vec<Range<usize>>,
    end: usize,
}

/// Reads the INDEX at `at` (Adobe Technical Note #5176, section 5) as ranges.
fn index_ranges(data: &[u8], at: usize) -> Option<IndexRanges> {
    let count = usize::from(u16::from_be_bytes([
        *data.get(at)?,
        *data.get(at.checked_add(1)?)?,
    ]));
    if count == 0 {
        return Some(IndexRanges {
            items: Vec::new(),
            end: at.checked_add(2)?,
        });
    }
    let off_size = usize::from(*data.get(at.checked_add(2)?)?);
    if !(1..=4).contains(&off_size) {
        return None;
    }
    let offsets = at.checked_add(3)?;
    let base = offsets
        .checked_add(count.checked_add(1)?.checked_mul(off_size)?)?
        .checked_sub(1)?;
    let mut items = Vec::with_capacity(count);
    for n in 0..count {
        let start = base.checked_add(index_offset(data, offsets, off_size, n)?)?;
        let end = base.checked_add(index_offset(data, offsets, off_size, n.checked_add(1)?)?)?;
        if start > end || end > data.len() {
            return None;
        }
        items.push(start..end);
    }
    let end = items.last().map_or(base, |last| last.end);
    Some(IndexRanges { items, end })
}

#[cfg(test)]
#[path = "cff_reals_tests.rs"]
mod tests;
