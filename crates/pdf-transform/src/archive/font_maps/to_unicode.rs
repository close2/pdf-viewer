//! A damaged `/ToUnicode` `CMap` written again as the mappings this reader read (ADR 3781).
//!
//! ISO 32000-2 §9.10.3 gives the map's grammar: codespace ranges "consistent with the encoding
//! that the font uses", and `beginbfchar` and `beginbfrange` sections mapping codes "to Unicode
//! character sequences expressed in UTF-16BE encoding". `pdf_font` reads an entry outside that
//! grammar — a destination with an unpaired surrogate or an odd byte, a high code below its low, a
//! section the stream ends inside — as no mapping, and logs the map `font/to-unicode-damaged`
//! (ADR 3565, ADR 3777). What it read is every other entry, exactly.
//!
//! So the map is written again **from the reading, not from the producer's text**: one
//! `beginbfchar` entry per code the reader holds singly and one `beginbfrange` entry per span, each
//! code in the byte width the producer wrote it in (§9.7.6.2 makes `<01>` and `<0001>` two codes),
//! in the order the reader consults them; the codespace ranges the producer wrote that read are
//! carried, and where none do, each width the map's codes use is given its whole range. A map built
//! on another through `/UseCMap` is written as one map holding both, the reader's own statement
//! of the pair (Table 118), and the entry is not carried.
//!
//! **Proved before written.** The map written is read back by the same reader, and must be read
//! whole — not damaged, not cut by a bound — and must answer every code exactly as the producer's
//! map answered it, asked by value alone and at each of the four widths: the same text for every
//! code the old one read, and nothing for every code it did not. The codes asked are every code
//! either map states, so the proof covers the whole code space, under [`PROOF_BUDGET`].

use core::fmt::Write as _;
use std::collections::BTreeSet;

use pdf_font::tounicode::{Mapping, SourceCode, ToUnicode};
use pdf_syntax::object::{Dictionary, Name, Object, Stream};
use pdf_syntax::serialize::flate_encode;
use pdf_syntax::{Lexer, Token};

use super::super::COMPRESSION_LEVEL;

/// The most codes the proof asks the two maps about, each at five widths.
///
/// Two two-byte identity spans are 131 072 codes, the largest map the census meets; a span of
/// three- or four-byte codes can state millions, and a map past this is refused rather than proved
/// by a sample (`CLAUDE.md` principle 3's budgets).
const PROOF_BUDGET: usize = 1 << 20;

/// How many entries go between one `beginbfchar` or `beginbfrange` and its closing operator.
///
/// Adobe Technical Note #5014, to which §9.10.3 sends the syntax, limits each section to a hundred
/// entries; the converter's other `ToUnicode` writer chunks by the same number for the same reason.
const SECTION_LIMIT: usize = 100;

/// Why a damaged `/ToUnicode` is not written again.
pub(super) const NOT_WRITTEN: &str = "a font's ToUnicode CMap holds entries this reader read past, \
     and this conversion writes such a map again from the mappings it read only where the map was \
     not cut by one of this reader's bounds and the map written answers every code as the \
     producer's answered it; this map is not that";

/// A damaged map written again, and what it dropped.
#[derive(Debug)]
pub(super) struct Written {
    /// The `/ToUnicode` stream, to be written at the producer's object.
    pub(super) stream: Object,
    /// The entries the reader read past, as the producer wrote them, for the report.
    pub(super) dropped: Vec<Box<str>>,
    /// How many entries were dropped, those `dropped` names and those past its bound.
    pub(super) dropped_count: usize,
}

/// `read` — the reader's reading of the `/ToUnicode` stream `stream`, whose decoded data is
/// `data` — written again as a map of what it read, proved to read alike.
///
/// # Errors
///
/// [`NOT_WRITTEN`] where the reading was cut by a bound, a span's first value is no scalar, the
/// proof outruns its budget, or the map written does not read alike.
pub(super) fn written(
    stream: &Stream,
    data: &[u8],
    read: &ToUnicode,
) -> Result<Written, &'static str> {
    let program = program_as_read(data, read)?;
    let mut dict = stream.dict.clone();
    // The entries that describe the old bytes' encoding, and the base the new map now holds.
    for entry in ["Filter", "DecodeParms", "Length", "DL", "UseCMap"] {
        dict.remove(entry);
    }
    Ok(Written {
        stream: stream_object(dict, program),
        dropped: read.unread().to_vec(),
        dropped_count: read.unread_count(),
    })
}

/// The `CMap` program [`written`] puts in its stream, before compression: `data` read as `read`,
/// written again and proved.
///
/// # Errors
///
/// [`NOT_WRITTEN`] where a bound cut the reading, or the rewrite is not proved.
pub(super) fn program_as_read(data: &[u8], read: &ToUnicode) -> Result<Vec<u8>, &'static str> {
    if read.truncated().is_some() {
        return Err(NOT_WRITTEN);
    }
    let program = program(&codespace(data, read), read).ok_or(NOT_WRITTEN)?;
    prove(read, &program)?;
    Ok(program)
}

/// The codespace ranges to write: the producer's that read, or each width the map uses whole.
///
/// §9.10.3 asks for ranges "consistent with the encoding that the font uses", which the producer's
/// own are the statement of; this reader does not read them, so a range is carried where its two
/// codes are strings of one width, one to four bytes, low not above high.
fn codespace(data: &[u8], read: &ToUnicode) -> Vec<(Vec<u8>, Vec<u8>)> {
    let mut ranges = Vec::new();
    let mut lexer = Lexer::new(data);
    let mut inside = false;
    let mut operands: Vec<Vec<u8>> = Vec::new();
    while let Some(token) = lexer.next_token() {
        match token {
            Token::Keyword(b"begincodespacerange") => {
                inside = true;
                operands.clear();
            }
            Token::Keyword(b"endcodespacerange") => {
                if inside {
                    for [low, high] in operands.as_chunks::<2>().0 {
                        if low.len() == high.len() && (1..=4).contains(&low.len()) && low <= high {
                            ranges.push((low.clone(), high.clone()));
                        }
                    }
                }
                inside = false;
            }
            Token::Keyword(_) => inside = false,
            Token::String(bytes) if inside => operands.push(bytes),
            _ => {}
        }
    }
    if ranges.is_empty() {
        let widths: BTreeSet<u8> = read
            .mappings()
            .map(|mapping| match mapping {
                Mapping::Single { length, .. } | Mapping::Span { length, .. } => length,
            })
            .collect();
        ranges = widths
            .into_iter()
            .map(|width| {
                let width = usize::from(width);
                (vec![0; width], vec![0xFF; width])
            })
            .collect();
    }
    ranges
}

/// The `CMap` program: §9.10.3's EXAMPLE 2's frame around `codespace` and the reading's entries,
/// or `None` where a span starts at no scalar value.
fn program(codespace: &[(Vec<u8>, Vec<u8>)], read: &ToUnicode) -> Option<Vec<u8>> {
    let mut singles: Vec<String> = Vec::new();
    let mut spans: Vec<String> = Vec::new();
    // A code the map and its base both state is answered by the map's own (Table 118), which
    // `mappings` gives first; the reader keeps the last of two singles, so only the first is kept.
    let mut seen: BTreeSet<(u32, u8)> = BTreeSet::new();
    for mapping in read.mappings() {
        match mapping {
            Mapping::Single { code, length, text } => {
                if seen.insert((code, length)) {
                    singles.push(format!("{} <{}>", code_hex(code, length), utf16be(text)));
                }
            }
            Mapping::Span {
                low,
                high,
                first,
                length,
            } => {
                let first = char::from_u32(first)?;
                spans.push(format!(
                    "{} {} <{}>",
                    code_hex(low, length),
                    code_hex(high, length),
                    utf16be(first.encode_utf8(&mut [0; 4]))
                ));
            }
        }
    }
    let mut out = String::from(
        "/CIDInit /ProcSet findresource begin\n\
         12 dict begin\n\
         begincmap\n\
         /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
         /CMapName /Adobe-Identity-UCS def\n\
         /CMapType 2 def\n",
    );
    for chunk in codespace.chunks(SECTION_LIMIT) {
        appended(
            &mut out,
            format_args!("{} begincodespacerange\n", chunk.len()),
        );
        for (low, high) in chunk {
            appended(&mut out, format_args!("<{}> <{}>\n", hex(low), hex(high)));
        }
        out.push_str("endcodespacerange\n");
    }
    sections(&mut out, &singles, ("beginbfchar", "endbfchar"));
    sections(&mut out, &spans, ("beginbfrange", "endbfrange"));
    out.push_str(
        "endcmap\n\
         CMapName currentdict /CMap defineresource pop\n\
         end\nend\n",
    );
    Some(out.into_bytes())
}

/// Appends `entries` in sections of [`SECTION_LIMIT`], between the operator pair `keywords`.
fn sections(out: &mut String, entries: &[String], keywords: (&str, &str)) {
    for chunk in entries.chunks(SECTION_LIMIT) {
        appended(out, format_args!("{} {}\n", chunk.len(), keywords.0));
        for entry in chunk {
            out.push_str(entry);
            out.push('\n');
        }
        out.push_str(keywords.1);
        out.push('\n');
    }
}

/// Proves `program` reads alike with `old`: whole, and every code answered as `old` answered it.
fn prove(old: &ToUnicode, program: &[u8]) -> Result<(), &'static str> {
    let new = ToUnicode::parse(program);
    if new.damaged() || new.truncated().is_some() {
        return Err(NOT_WRITTEN);
    }
    let mut asked = 0usize;
    for (low, high) in old
        .mappings()
        .chain(new.mappings())
        .map(|mapping| match mapping {
            Mapping::Single { code, .. } => (code, code),
            Mapping::Span { low, high, .. } => (low, high),
        })
    {
        for value in low..=high {
            asked = asked.saturating_add(1);
            if asked > PROOF_BUDGET {
                return Err(NOT_WRITTEN);
            }
            for length in [None, Some(1), Some(2), Some(3), Some(4)] {
                let code = SourceCode { value, length };
                let (mut was, mut now) = (String::new(), String::new());
                let answered = (old.append(code, &mut was), new.append(code, &mut now));
                if answered.0 != answered.1 || was != now {
                    return Err(NOT_WRITTEN);
                }
            }
        }
    }
    Ok(())
}

/// The stream a program goes in, compressed where that is shorter.
pub(super) fn stream_object(mut dict: Dictionary, data: Vec<u8>) -> Object {
    let data = match flate_encode(&data, COMPRESSION_LEVEL) {
        Some(encoded) if encoded.len() < data.len() => {
            dict.insert(
                Name::new(&b"Filter"[..]),
                Object::Name(Name::new(&b"FlateDecode"[..])),
            );
            encoded
        }
        _ => data,
    };
    Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: data.into(),
        decryption_failed: false,
    }))
}

/// A code as a `CMap` hex string of `length` bytes.
fn code_hex(code: u32, length: u8) -> String {
    let all = code.to_be_bytes();
    let bytes = all
        .get(all.len().saturating_sub(usize::from(length))..)
        .unwrap_or(&all);
    format!("<{}>", hex(bytes))
}

/// Text as §9.10.3's destination: UTF-16BE, in hexadecimal digits.
fn utf16be(text: &str) -> String {
    let bytes: Vec<u8> = text.encode_utf16().flat_map(u16::to_be_bytes).collect();
    hex(&bytes)
}

/// Bytes as upper-case hexadecimal digits.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, byte| {
        appended(&mut out, format_args!("{byte:02X}"));
        out
    })
}

/// Appends one formatted piece to a string, discarding a `Result` that cannot be an error:
/// `fmt::Write for String` returns `Ok` on every call, growing the buffer or aborting the process
/// (`CLAUDE.md` principle 1's provably-infallible case, as `super::super::to_unicode` names it).
fn appended(out: &mut String, piece: core::fmt::Arguments<'_>) {
    let _ = out.write_fmt(piece);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stream() -> Stream {
        Stream {
            dict: Dictionary::new(),
            data: Vec::new().into(),
            decryption_failed: false,
        }
    }

    /// The program [`written`] puts in its stream, before compression.
    fn program_of(source: &[u8], read: &ToUnicode) -> Vec<u8> {
        program(&codespace(source, read), read).expect("every span starts at a scalar")
    }

    /// An unpaired surrogate's entry is dropped and named, and every other code — single, span
    /// and one-byte beside two-byte — reads as it did.
    #[test]
    fn a_damaged_map_is_written_as_read_and_names_what_it_drops() {
        let source = b"1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
            3 beginbfchar\n<0001> <0041>\n<0002> <D800>\n<0003> <D83DDE00>\nendbfchar\n\
            2 beginbfrange\n<0010> <0012> <0061>\n<0020> <0021> [<0062> <00>]\nendbfrange\n";
        let read = ToUnicode::parse(source);
        assert!(read.damaged());
        let written = written(&stream(), source, &read).expect("the reading is written");
        assert_eq!(written.dropped_count, 2);
        let program = program_of(source, &read);
        let text = String::from_utf8(program.clone()).expect("the program is text");
        assert!(text.contains("1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n"));
        let new = ToUnicode::parse(&program);
        assert!(!new.damaged());
        let answer = |map: &ToUnicode, code: u32| {
            let mut out = String::new();
            map.append(code, &mut out).then_some(out)
        };
        for code in [1, 2, 3, 0x10, 0x11, 0x12, 0x20, 0x21, 0x22] {
            assert_eq!(answer(&new, code), answer(&read, code), "code {code:X}");
        }
        assert_eq!(answer(&new, 2), None);
        assert_eq!(answer(&new, 3).as_deref(), Some("\u{1F600}"));
    }

    /// A map whose section the stream ends inside is written closed, with the codespace derived
    /// from the one width its codes use.
    #[test]
    fn an_unfinished_section_is_written_closed() {
        let source = b"2 beginbfchar\n<01> <0041>\n<02>";
        let read = ToUnicode::parse(source);
        assert!(read.damaged());
        let written = written(&stream(), source, &read).expect("the reading is written");
        let text = String::from_utf8(program_of(source, &read)).expect("the program is text");
        assert!(text.contains("1 begincodespacerange\n<00> <FF>\nendcodespacerange\n"));
        assert!(text.contains("1 beginbfchar\n<01> <0041>\nendbfchar\n"));
        assert_eq!(&*written.dropped[0], "a section the stream ends inside");
    }

    /// A map cut by one of this reader's bounds is not written: what was cut is not in the
    /// reading, and writing the reading would state that the producer's codes mean nothing.
    #[test]
    fn a_map_cut_by_a_bound_is_not_written() {
        let mut source = b"1 beginbfchar\n<01> <D800>\nendbfchar\n".to_vec();
        for _ in 0..=(1 << 14) {
            source.extend_from_slice(b"1 beginbfrange\n<0001> <0002> <0041>\nendbfrange\n");
        }
        let read = ToUnicode::parse(&source);
        assert!(read.truncated().is_some());
        assert_eq!(written(&stream(), &source, &read).err(), Some(NOT_WRITTEN));
    }
}
