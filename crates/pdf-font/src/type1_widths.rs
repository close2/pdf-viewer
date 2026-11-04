//! A bare Type 1 program with the advances of named glyphs restated — the Type 1 counterpart of
//! [`crate::cff::with_advances`] (`doc/adr/2685`).
//!
//! Adobe Type 1 Font Format section 6.4 states a glyph's advance as the `wx` operand of the
//! `hsbw` command (or of `sbw`) that opens its charstring, and the charstring is encrypted under
//! section 7.3's key 4330 inside a section encrypted under section 7.2's key 55665. So an advance
//! is restated by decrypting the section, decrypting the glyph's charstring, writing the one
//! operand as the number asked for with every other byte of the charstring kept — the `lenIV`
//! leading bytes as they were — encrypting the charstring again, writing its new byte count
//! before its reader token, and encrypting the section again. Nothing else in the program
//! changes: every other entry is copied byte for byte, and the header and trailer are written as
//! [`super::with_glyphs_given`] writes them, so `/Length1` and `/Length3` keep their values.
//!
//! The width is the only number that moves: section 6.4 makes `sbx` (and `sby`) the side bearing
//! the outline starts from and `wx` (and `wy`) where the next glyph starts, so an outline drawn
//! before and after the rewrite is the same, and [`read_back`] draws both to check it.

use std::collections::BTreeMap;

use super::{
    CHARSTRING_KEY, EEXEC_KEY, MOST_BYTES_WRITTEN, MOST_CHARSTRING_BYTES, Scan, Sections,
    agrees_with_reader, decrypt, encrypt, encrypt_in_place, not_written, outline, with_width,
};
use crate::type1::{Program, Type1Error};

/// A program [`with_advances`] rewrote, with the three lengths §9.9.1's Table 125 states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvancesRestated {
    /// The rewritten program: the header, the re-encrypted section and the trailer.
    pub program: Vec<u8>,
    /// `/Length1`, `/Length2` and `/Length3` of [`Self::program`].
    pub lengths: [usize; 3],
}

/// A bare Type 1 program with each named glyph's advance restated to the whole number of
/// character space units `advances` gives it, where `length1` and `length2` are the stream's
/// `/Length1` and `/Length2`.
///
/// `advances` is keyed by the glyph index `read-fonts` gives a glyph — the `CharStrings`
/// dictionary's order with `.notdef` moved to index 0 — which is the index
/// [`Program::advance`] and a loaded font's `glyph_index` answer in.
///
/// # Errors
///
/// [`Type1Error::NotWritten`] where the lengths do not divide the program into a header ending in
/// `eexec` and a binary section, the section's `CharStrings` dictionary does not read as
/// `read-fonts` reads it, a glyph index passes the program's glyphs, a charstring does not open
/// with `hsbw` or `sbw` over plain numbers (section 6.4 makes every charstring open with one of
/// them, so one that does not states its width where this does not rewrite), a charstring would
/// pass the 65 535 bytes chapter 10 allows one, or the rewritten program does not read back as
/// restated; [`Type1Error::Malformed`] where `read-fonts` does not read the program.
pub fn with_advances(
    data: &[u8],
    (length1, length2): (usize, usize),
    advances: &BTreeMap<u16, i32>,
) -> Result<AdvancesRestated, Type1Error> {
    if data.len() > MOST_BYTES_WRITTEN {
        return Err(not_written("the program passes the writer's bound"));
    }
    let sections = Sections::of(data, length1, length2)?;
    let plain = decrypt(sections.encrypted, EEXEC_KEY);
    let scan =
        Scan::of(&plain).ok_or_else(|| not_written("the CharStrings dictionary does not read"))?;
    let before = Program::parse(data)?;
    agrees_with_reader(&scan, &plain, &before)?;
    let notdef = scan
        .entries
        .iter()
        .position(|entry| entry.name(&plain) == b".notdef");

    // Each restated charstring by the position of its entry in the section, so the section is
    // written in one pass in the order it holds them.
    let mut restated: BTreeMap<usize, Vec<u8>> = BTreeMap::new();
    for (glyph, advance) in advances {
        let at = entry_of(usize::from(*glyph), notdef);
        let entry = scan
            .entries
            .get(at)
            .ok_or_else(|| not_written("a glyph index passes the program's glyphs"))?;
        let stored = &plain[entry.data.clone()];
        restated.insert(at, charstring_restated(stored, scan.len_iv, *advance)?);
    }

    let mut program =
        Vec::with_capacity(data.len().saturating_add(restated.len().saturating_mul(8)));
    program.extend(sections.header_written());
    let section_at = program.len();
    let mut from = 0;
    for (at, charstring) in &restated {
        let entry = &scan.entries[*at];
        program.extend_from_slice(&plain[from..entry.count.start]);
        program.extend_from_slice(charstring.len().to_string().as_bytes());
        program.extend_from_slice(&plain[entry.count.end..entry.data.start]);
        program.extend_from_slice(charstring);
        from = entry.data.end;
    }
    program.extend_from_slice(&plain[from..]);
    encrypt_in_place(&mut program[section_at..], EEXEC_KEY);
    let section_length = program.len().saturating_sub(section_at);
    program.extend_from_slice(sections.trailer);
    let restated_program = AdvancesRestated {
        lengths: [section_at, section_length, sections.trailer.len()],
        program,
    };
    read_back(
        &restated_program,
        (&plain, &scan),
        &before,
        advances,
        notdef,
    )?;
    Ok(restated_program)
}

/// The position in the section of the entry `read-fonts` numbers `glyph`: it moves `.notdef` to
/// index 0 by swapping it with the entry there, which [`agrees_with_reader`] has checked.
fn entry_of(glyph: usize, notdef: Option<usize>) -> usize {
    match notdef {
        Some(notdef) if glyph == 0 => notdef,
        Some(notdef) if glyph == notdef => 0,
        _ => glyph,
    }
}

/// One stored charstring with its width operand written as `advance`: decrypted whole, its
/// `len_iv` leading bytes kept as the producer wrote them, the body restated by section 6.4's
/// rule, and encrypted again — or, where `len_iv` is negative and the charstring is stored in
/// the clear, restated as it is.
fn charstring_restated(stored: &[u8], len_iv: i64, advance: i32) -> Result<Vec<u8>, Type1Error> {
    let unstated = || {
        not_written(
            "a charstring does not open with hsbw or sbw over plain numbers, so its width is \
             stated where this does not rewrite",
        )
    };
    let charstring = match usize::try_from(len_iv) {
        Ok(leading) => {
            let whole = decrypt(stored, CHARSTRING_KEY);
            let (front, body) = whole
                .split_at_checked(leading)
                .ok_or_else(|| not_written("a charstring is shorter than its lenIV"))?;
            let mut restated = front.to_vec();
            restated.extend(with_width(body, advance).ok_or_else(unstated)?);
            encrypt(&restated, CHARSTRING_KEY)
        }
        Err(_) => with_width(stored, advance).ok_or_else(unstated)?,
    };
    if charstring.len() > MOST_CHARSTRING_BYTES {
        return Err(not_written(
            "a restated charstring would pass the 65 535 bytes a charstring may hold",
        ));
    }
    Ok(charstring)
}

/// Whether `restated` reads as it was written, by this module's scan and by `read-fonts`: every
/// entry it did not restate byte for byte as it was, and every glyph it did stating the advance
/// asked for and drawing the outline it drew before.
fn read_back(
    restated: &AdvancesRestated,
    (plain, scan): (&[u8], &Scan),
    before: &Program,
    advances: &BTreeMap<u16, i32>,
    notdef: Option<usize>,
) -> Result<(), Type1Error> {
    let differs = || not_written("the rewritten program does not read back as restated");
    let [header, section, _] = restated.lengths;
    let stored = restated
        .program
        .get(header..header.saturating_add(section))
        .ok_or_else(differs)?;
    let section = decrypt(stored, EEXEC_KEY);
    let again = Scan::of(&section).ok_or_else(differs)?;
    if again.len_iv != scan.len_iv
        || again.stated != scan.stated
        || again.entries.len() != scan.entries.len()
    {
        return Err(differs());
    }
    for (at, (was, is)) in scan.entries.iter().zip(&again.entries).enumerate() {
        let asked = advances
            .keys()
            .any(|glyph| entry_of(usize::from(*glyph), notdef) == at);
        if was.name(plain) != is.name(&section)
            || (!asked && plain[was.data.clone()] != section[is.data.clone()])
        {
            return Err(differs());
        }
    }
    drop(section);

    let after = Program::parse(&restated.program)?;
    if after.glyph_names() != before.glyph_names() {
        return Err(differs());
    }
    for (glyph, advance) in advances {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a charstring's width is read exactly as a whole number below 2^24, and one \
                      past it reads back as some other number and is refused"
        )]
        let wanted = *advance as f32;
        if after.advance(*glyph)? != Some(wanted)
            || outline(&after, *glyph)? != outline(before, *glyph)?
        {
            return Err(differs());
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "type1_widths_tests.rs"]
mod tests;
