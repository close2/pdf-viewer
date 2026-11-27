//! An embedded `Encoding` `CMap` that maps with `beginbfchar`, restated with `begincidchar` as
//! this reader read it (ADR 3428).
//!
//! ISO 32000-2 §9.7.5.4 c) reserves `beginbfchar` and `endbfchar` for a `ToUnicode` `CMap`, and ISO
//! 19005-2 section 6.2.11.3.3 and ISO 19005-4 section 6.2.10.3.3 have an embedded `CMap` follow
//! §9.7.5 (`pdf_archive`'s `fonts/encoding-cmap-maps-codes-to-cids`). What the producer meant is
//! not open: §9.7.6.2's decoding algorithm looks a code up in "the mappings defined by
//! `beginbfchar` … and corresponding operators for ranges", and a mapping of a Type 0 font's
//! encoding selects a CID (§9.7.5.1), so `pdf_font::cmap` reads each destination hex string as a
//! CID, most significant byte first, and drew the page with those CIDs.
//!
//! **So each section is restated as the `begincidchar` section that says the same**: the source
//! code written as the same bytes, the destination as the integer this reader read it as, and
//! every other byte of the program — its header, its codespace, any other section — unchanged.
//! That is checked rather than assumed: every code the original program's codespace admits
//! selects the same CID under the restated program, or the `CMap` is refused by name and the
//! finding stands.
//!
//! A map this reader read past entries of is not this module's: it is written whole from its
//! reading (`super::font_maps`, ADR 3804), which states each of these mappings as `cidchar` too,
//! so one route answers both findings (ADR 3827).
//!
//! What stays refused, by name: a `CMap` written inside another object, one this reader does not
//! decode, a destination that is not a hex string of at most four bytes — a glyph name, which
//! states no CID — and a program with more codes than the check walks.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use pdf_font::cmap::CMap;
use pdf_syntax::{Document, Lexer, Object, ObjectId, Token};

use super::decision::Because;

/// The requirement this module answers, by the identifier the validator reports it under.
pub(super) const ROW: &str = "fonts/encoding-cmap-maps-codes-to-cids";

/// Why an `Encoding` `CMap` is not restated.
const NOT_RESTATED: &str = "a Type 0 font's embedded Encoding CMap maps codes with beginbfchar, \
     which ISO 32000-2 \u{a7}9.7.5.4 c) reserves for a ToUnicode CMap, and this conversion \
     restates such a section with begincidchar only where the CMap is an object of its own, \
     decodes, maps every code to a hex string of at most four bytes, and selects every code's CID \
     as it did; this CMap is not that";

/// How many codes the check walks before it refuses: a two-byte codespace is 65 536.
const CODES_WALKED: u64 = 1 << 20;

/// Adobe Technical Note #5014 caps one mapping section at a hundred entries.
const ENTRIES_PER_SECTION: usize = 100;

/// The restated `CMap` streams, each keyed by the object it goes into, in the source's numbering.
#[derive(Debug, Default)]
pub(super) struct Restated {
    /// Each `CMap` stream, whole: new bytes, and the `/Filter` they are written under.
    pub(super) streams: BTreeMap<ObjectId, Object>,
}

/// Every `CMap` [`ROW`] failed at, restated, or why one cannot be.
///
/// # Errors
///
/// [`NOT_RESTATED`] where any failing `CMap` is outside what the module comment lists; a
/// requirement half answered is a requirement refused.
pub(super) fn restated(
    document: &Document,
    input: &pdf_archive::Report,
) -> Result<Restated, Because> {
    let refused = Because::NotBuiltYet(NOT_RESTATED);
    let (named, elsewhere) = super::sites::objects_named(input, &[ROW])?;
    if elsewhere {
        return Err(refused);
    }
    let mut restated = Restated::default();
    for id in named {
        let Object::Stream(stream) = document.get(id) else {
            return Err(refused);
        };
        let original = document.decoded_stream_data(&stream).ok_or(refused)?;
        // A map this reader read past entries of is written whole from its reading at
        // `fonts/font-maps-read-as-written` (`super::font_maps`, ADR 3804), which states every
        // mapping a `bfchar` section made as the `cidchar` entry read; restating the sections here
        // as well would keep the damage (ADR 3827).
        if CMap::parse(&original, None).damaged() {
            continue;
        }
        let program = with_cid_sections(&original).ok_or(refused)?;
        if !selects_as_it_did(&original, &program) {
            return Err(refused);
        }
        let written = super::remedies::restated_stream(&stream.dict, &program).ok_or(refused)?;
        restated.streams.insert(id, written);
    }
    Ok(restated)
}

/// The program with every `beginbfchar` section written as `begincidchar` sections, or `None`
/// where a destination states no CID.
fn with_cid_sections(program: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(program.len());
    let mut copied = 0usize;
    let mut lexer = Lexer::new(program);
    let mut section: Option<Section> = None;
    let mut count_at = None;
    while let Some(token) = lexer.next_token() {
        let end = lexer.position();
        match (&mut section, token) {
            (None, Token::Integer(_)) => {
                // The count before a section's opening operator is restated with the section.
                count_at = Some(start_of_token(program, end));
            }
            (None, Token::Keyword(b"beginbfchar")) => {
                let start = count_at.unwrap_or_else(|| end.saturating_sub(b"beginbfchar".len()));
                section = Some((start, Vec::new(), Vec::new()));
            }
            (Some((_, _, pending)), Token::String(bytes)) => pending.push(bytes),
            (Some((start, entries, pending)), Token::Keyword(b"endbfchar")) => {
                if !pending.len().is_multiple_of(2) {
                    return None;
                }
                for pair in pending.chunks(2) {
                    let [source, target] = pair else {
                        return None;
                    };
                    entries.push((source.clone(), cid_of(target)?));
                }
                out.extend_from_slice(program.get(copied..*start)?);
                write_cid_sections(&mut out, entries);
                copied = end;
                section = None;
                count_at = None;
            }
            // Anything else inside a section is a destination that is no hex string: a glyph
            // name or an array, neither of which states a CID.
            (Some(_), _) => return None,
            (None, _) => count_at = None,
        }
    }
    if section.is_some() {
        return None;
    }
    out.extend_from_slice(program.get(copied..)?);
    Some(out)
}

/// Where the token ending at `end` begins: the integer is a run of digits and an optional sign.
fn start_of_token(program: &[u8], end: usize) -> usize {
    let mut at = end;
    while at > 0
        && program
            .get(at.saturating_sub(1))
            .is_some_and(|byte| byte.is_ascii_digit() || matches!(byte, b'+' | b'-'))
    {
        at = at.saturating_sub(1);
    }
    at
}

/// A `beginbfchar` section being read: where its count begins, the entries read, and the strings
/// not yet paired.
type Section = (usize, Vec<(Vec<u8>, u32)>, Vec<Vec<u8>>);

/// A destination hex string read as `pdf_font::cmap` reads it: a CID, most significant byte first.
fn cid_of(bytes: &[u8]) -> Option<u32> {
    if bytes.is_empty() || bytes.len() > 4 {
        return None;
    }
    Some(
        bytes
            .iter()
            .fold(0u32, |value, &byte| (value << 8_u32) | u32::from(byte)),
    )
}

/// Writes `entries` as `begincidchar` sections of at most [`ENTRIES_PER_SECTION`] each.
fn write_cid_sections(out: &mut Vec<u8>, entries: &[(Vec<u8>, u32)]) {
    let mut text = String::new();
    for (at, chunk) in entries.chunks(ENTRIES_PER_SECTION).enumerate() {
        if at > 0 {
            text.push('\n');
        }
        let _ = writeln!(text, "{} begincidchar", chunk.len());
        for (source, cid) in chunk {
            text.push('<');
            for byte in source {
                let _ = write!(text, "{byte:02x}");
            }
            let _ = writeln!(text, "> {cid}");
        }
        text.push_str("endcidchar");
    }
    out.extend_from_slice(text.as_bytes());
}

/// Whether every code the original's codespace admits selects the same CID under both programs.
fn selects_as_it_did(original: &[u8], restated: &[u8]) -> bool {
    let before = CMap::parse(original, None);
    let after = CMap::parse(restated, None);
    if !before.has_mappings() || !after.has_mappings() {
        return false;
    }
    let mut agree = true;
    let walked = before.each_addressable_code(CODES_WALKED, |code| {
        agree &= before.cid(code) == after.cid(code);
    });
    walked && agree
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::{selects_as_it_did, with_cid_sections};

    const HEADER: &str = "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
                          1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n";

    #[test]
    fn a_bfchar_section_is_restated_as_the_cids_this_reader_read() {
        let program =
            format!("{HEADER}2 beginbfchar\n<0020> <0003>\n<fffc> <035f>\nendbfchar\nendcmap\n");
        let restated = with_cid_sections(program.as_bytes()).expect("every destination is a CID");
        let text = String::from_utf8(restated.clone()).expect("ASCII");
        assert_eq!(
            text,
            format!("{HEADER}2 begincidchar\n<0020> 3\n<fffc> 863\nendcidchar\nendcmap\n"),
            "the count, the operators and the destinations change, and nothing else"
        );
        assert!(selects_as_it_did(program.as_bytes(), &restated));
    }

    #[test]
    fn a_glyph_name_destination_states_no_cid_and_is_refused() {
        let program = format!("{HEADER}1 beginbfchar\n<0020> /space\nendbfchar\nendcmap\n");
        assert_eq!(with_cid_sections(program.as_bytes()), None);
    }

    #[test]
    fn more_than_a_hundred_entries_are_written_as_several_sections() {
        let entries = (0..150u16).fold(String::new(), |mut entries, code| {
            let _ = writeln!(entries, "<{code:04x}> <{:04x}>", code.saturating_add(1));
            entries
        });
        let program = format!("{HEADER}150 beginbfchar\n{entries}endbfchar\nendcmap\n");
        let restated = with_cid_sections(program.as_bytes()).expect("CIDs");
        let text = String::from_utf8(restated.clone()).expect("ASCII");
        assert!(text.contains("100 begincidchar") && text.contains("50 begincidchar"));
        assert!(selects_as_it_did(program.as_bytes(), &restated));
    }
}
