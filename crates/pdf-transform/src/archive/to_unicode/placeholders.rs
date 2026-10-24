//! A producer's `/ToUnicode` `CMap` restated without the values ISO 19005 calls placeholders.
//!
//! # When this is the answer, and when it is not
//!
//! ISO 19005-2 section 6.2.11.7.2 and ISO 19005-4 section 6.2.10.7 end on the same sentence: every
//! Unicode value a `/ToUnicode` `CMap` states is greater than zero and is neither U+FEFF nor U+FFFE.
//! The parent module answers it by deriving a fresh `CMap` from the font's glyph names, which
//! §9.10.2's second method makes possible for a simple font and impossible for a composite one —
//! a CID names a glyph of the font that defined it and no character (§9.7.4.2). For such a font
//! there is nothing to put where a placeholder was. What remains is to **stop stating the
//! placeholder**: a code mapped to U+0000, or a span whose run passes through U+FEFF, says nothing
//! about any character (part 2's NOTE 2 calls such values placeholders), and removing the
//! statement leaves the code where the clause's other methods can still be tried.
//!
//! That removal leaves a code unmapped, and whether a target admits that is the clause's first
//! paragraph, which the two parts state differently. **Part 4 states it as a recommendation** —
//! a font should carry a `CMap` mapping the codes of at least every referenced glyph — so a code a
//! part 4 file leaves unmapped breaks nothing the part requires. **Part 2
//! at Level U and A states it as a requirement**, so a removal is admitted there only where no
//! content stream showed the code — the clause asks about *referenced* glyphs — and the font's
//! shown strings were read whole. ADR 2294 is the argument.
//!
//! # The producer's bytes, kept
//!
//! The `CMap` is rewritten statement by statement and section by section rather than composed
//! again: everything outside a `beginbfchar` or `beginbfrange` section — the codespace ranges,
//! the `CIDSystemInfo`, a `usecmap` — is carried byte for byte, and so is every section none of
//! whose statements states a placeholder. A section that does is written again from its own
//! statements, the codes in the byte width the producer wrote them in, with the placeholder's
//! statement left out and a span split around the value it passed through.
//!
//! The rewrite is then **proved rather than trusted**: both maps are parsed by `pdf_font`'s own
//! reader, the one the validator judges with, and every code the producer's map answered with a
//! usable value is asked of the new one, which must answer the same — while the new one states no
//! placeholder at all.

use std::collections::BTreeSet;

use pdf_font::tounicode::{Mapping, ToUnicode};
use pdf_syntax::{Lexer, Token};

use super::UNUSABLE;

/// One `beginbfchar` or `beginbfrange` statement, with its codes in the bytes the producer wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Statement {
    /// `<code> <text>`.
    Char { code: Vec<u8>, text: Vec<u8> },
    /// `<low> <high> <first>`: consecutive scalars from `first`.
    Span {
        low: Vec<u8>,
        high: Vec<u8>,
        first: Vec<u8>,
    },
    /// `<low> <high> [<text> …]`: one destination per code.
    Each {
        low: Vec<u8>,
        high: Vec<u8>,
        texts: Vec<Vec<u8>>,
    },
}

/// Which of the two sections a statement was read from, and is written back into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    Chars,
    Ranges,
}

impl Section {
    /// The operator pair that brackets the section.
    const fn keywords(self) -> (&'static str, &'static str) {
        match self {
            Self::Chars => ("beginbfchar", "endbfchar"),
            Self::Ranges => ("beginbfrange", "endbfrange"),
        }
    }
}

/// The most statements one section is written with.
///
/// The parent module's [`super::BFCHAR_LIMIT`], for the same reason and on the same terms.
const SECTION_LIMIT: usize = super::BFCHAR_LIMIT;

/// The most codes the proof asks the two maps about.
///
/// Two two-byte identity spans are 131 072 codes and the largest map the census meets; a span of
/// three- or four-byte codes can state millions, and a map past this is refused rather than proved
/// by a sample (`CLAUDE.md` principle 3's budgets).
const PROOF_BUDGET: usize = 1 << 20;

/// The producer's `CMap` with every placeholder statement taken out, and the codes that lost one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Restated {
    /// The `CMap` program, as a `/ToUnicode` stream's decoded data.
    pub(super) data: Vec<u8>,
    /// Every code a statement taken out had mapped, which is what part 2's referenced-glyph
    /// condition is asked about.
    pub(super) unmapped: BTreeSet<u32>,
}

/// Restates `bytes`, a `/ToUnicode` `CMap` program, without its placeholder values.
///
/// # Errors
///
/// The reason it cannot: a statement that maps a code to a placeholder *and* a character, a span
/// whose run would continue into the surrogate range once split, a map past the proof's budget,
/// a rewrite the proof does not confirm, or a map that states no placeholder after all.
pub(super) fn restate(bytes: &[u8]) -> Result<Restated, &'static str> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut unmapped = BTreeSet::new();
    let mut copied_to = 0usize;
    let mut lexer = Lexer::new(bytes);
    // Where the last token began, and whether it was the count a section's keyword follows.
    let mut count_at: Option<usize> = None;
    loop {
        lexer.skip_whitespace();
        let start = lexer.position();
        let Some(token) = lexer.token_at_cursor() else {
            break;
        };
        let section = match &token {
            Token::Keyword(b"beginbfchar") => Section::Chars,
            Token::Keyword(b"beginbfrange") => Section::Ranges,
            Token::Integer(_) => {
                count_at = Some(start);
                continue;
            }
            _ => {
                count_at = None;
                continue;
            }
        };
        let begins = count_at.take().unwrap_or(start);
        let Some((statements, ends)) = read_section(&mut lexer, section) else {
            // A section the stream ends inside is one `pdf_font` reads up to the damage; a
            // rewrite would have to decide where the producer meant it to end.
            return Err(UNFINISHED_SECTION);
        };
        let mut kept = Vec::with_capacity(statements.len());
        let mut changed = false;
        for statement in statements {
            let pieces = without_placeholders(&statement, &mut unmapped)?;
            changed |= pieces.len() != 1 || pieces.first() != Some(&statement);
            kept.extend(pieces);
        }
        if changed {
            out.extend_from_slice(bytes.get(copied_to..begins).unwrap_or_default());
            write_sections(&mut out, section, &kept);
            copied_to = ends;
        }
    }
    out.extend_from_slice(bytes.get(copied_to..).unwrap_or_default());
    if unmapped.is_empty() {
        return Err(NOTHING_TO_TAKE_OUT);
    }
    prove(bytes, &out)?;
    Ok(Restated {
        data: out,
        unmapped,
    })
}

/// Reads one section's statements, up to and past its closing keyword.
///
/// Returns where the section ends, or `None` where the stream ends first. Operands that make no
/// statement are skipped as `pdf_font`'s reader skips them, so the two read the same statements.
fn read_section(lexer: &mut Lexer<'_>, section: Section) -> Option<(Vec<Statement>, usize)> {
    let mut operands: Vec<Token<'_>> = Vec::new();
    let closing = section.keywords().1.as_bytes();
    while let Some(token) = lexer.next_token() {
        match token {
            Token::Keyword(word) if word == closing => {
                return Some((statements_of(section, &operands), lexer.position()));
            }
            // Any other keyword ends the section for `pdf_font` too, without its statements.
            Token::Keyword(_) => return Some((Vec::new(), lexer.position())),
            other => operands.push(other),
        }
    }
    None
}

/// The statements one section's operands make, in `pdf_font`'s own reading of them.
fn statements_of(section: Section, operands: &[Token<'_>]) -> Vec<Statement> {
    let mut out = Vec::new();
    let mut at = 0usize;
    let string = |index: usize| match operands.get(index) {
        Some(Token::String(bytes)) => Some(bytes.clone()),
        _ => None,
    };
    while at < operands.len() {
        let next = at.saturating_add(1);
        match section {
            Section::Chars => {
                let (Some(code), Some(text)) = (string(at), string(next)) else {
                    at = next;
                    continue;
                };
                out.push(Statement::Char { code, text });
                at = at.saturating_add(2);
            }
            Section::Ranges => {
                let (Some(low), Some(high)) = (string(at), string(next)) else {
                    at = next;
                    continue;
                };
                let third = at.saturating_add(2);
                match operands.get(third) {
                    Some(Token::String(first)) => {
                        out.push(Statement::Span {
                            low,
                            high,
                            first: first.clone(),
                        });
                        at = third.saturating_add(1);
                    }
                    Some(Token::ArrayOpen) => {
                        let mut texts = Vec::new();
                        at = third.saturating_add(1);
                        while let Some(Token::String(text)) = operands.get(at) {
                            texts.push(text.clone());
                            at = at.saturating_add(1);
                        }
                        if matches!(operands.get(at), Some(Token::ArrayClose)) {
                            at = at.saturating_add(1);
                        }
                        out.push(Statement::Each { low, high, texts });
                    }
                    _ => at = third.saturating_add(1),
                }
            }
        }
    }
    out
}

/// One statement as it is written back: itself, nothing, or the pieces of a split span.
fn without_placeholders(
    statement: &Statement,
    unmapped: &mut BTreeSet<u32>,
) -> Result<Vec<Statement>, &'static str> {
    match statement {
        Statement::Char { code, text } => {
            let Some(value) = code_of(code) else {
                return Ok(vec![statement.clone()]);
            };
            if placeholder_only(text)? {
                unmapped.insert(value);
                return Ok(Vec::new());
            }
            Ok(vec![statement.clone()])
        }
        Statement::Span { low, high, first } => {
            let (Some(from), Some(to)) = (code_of(low), code_of(high)) else {
                return Ok(vec![statement.clone()]);
            };
            if from > to {
                return Ok(vec![statement.clone()]);
            }
            let Some(start) = scalar_of(first) else {
                // A destination of more than one character applies to the low code alone
                // (`pdf_font`'s reading), which makes it the same statement as a `bfchar`.
                if text_of(first).is_some() && placeholder_only(first)? {
                    unmapped.insert(from);
                    return Ok(Vec::new());
                }
                return Ok(vec![statement.clone()]);
            };
            Ok(split_span(low.len(), from, to, start, unmapped)?
                .unwrap_or_else(|| vec![statement.clone()]))
        }
        Statement::Each { low, high, texts } => {
            let (Some(from), Some(to)) = (code_of(low), code_of(high)) else {
                return Ok(vec![statement.clone()]);
            };
            let mut pieces = Vec::new();
            let mut run: Vec<Vec<u8>> = Vec::new();
            let mut run_from = from;
            let mut dropped = false;
            let width = low.len();
            for (offset, text) in (0u32..).zip(texts) {
                let Some(code) = from.checked_add(offset).filter(|code| *code <= to) else {
                    break;
                };
                if text_of(text).is_some() && placeholder_only(text)? {
                    unmapped.insert(code);
                    dropped = true;
                    flush_run(&mut pieces, &mut run, width, run_from);
                    run_from = code.saturating_add(1);
                } else {
                    run.push(text.clone());
                }
            }
            if !dropped {
                return Ok(vec![statement.clone()]);
            }
            flush_run(&mut pieces, &mut run, width, run_from);
            Ok(pieces)
        }
    }
}

/// A span's run with every code mapping to a placeholder taken out, as the spans that remain.
///
/// `None` where the run passes through no placeholder, which the caller reads as "keep the span as
/// the producer wrote it"; an empty list where every code of it stood for one.
fn split_span(
    width: usize,
    from: u32,
    to: u32,
    start: u32,
    unmapped: &mut BTreeSet<u32>,
) -> Result<Option<Vec<Statement>>, &'static str> {
    // Every code of the span stands for `start` plus its distance from `from`, so a value lies in
    // the run exactly when it is no further above `start` than `to` is above `from`.
    let width_of_run = to.saturating_sub(from);
    let mut holes: Vec<u32> = UNUSABLE
        .iter()
        .filter_map(|character| {
            let offset = u32::from(*character).checked_sub(start)?;
            (offset <= width_of_run).then(|| from.saturating_add(offset))
        })
        .collect();
    if holes.is_empty() {
        return Ok(None);
    }
    holes.sort_unstable();
    let first_of = |code: u32| start.saturating_add(code.saturating_sub(from));
    let mut pieces = Vec::new();
    let mut piece_from = Some(from);
    for hole in holes {
        unmapped.insert(hole);
        if let Some(begins) = piece_from
            && hole > begins
        {
            pieces.push(span(
                width,
                begins,
                hole.saturating_sub(1),
                first_of(begins),
            )?);
        }
        piece_from = hole.checked_add(1).filter(|next| *next <= to);
    }
    if let Some(begins) = piece_from {
        pieces.push(span(width, begins, to, first_of(begins))?);
    }
    Ok(Some(pieces))
}

/// One span statement, its codes in `width` bytes.
fn span(width: usize, from: u32, to: u32, first: u32) -> Result<Statement, &'static str> {
    let character = char::from_u32(first).ok_or(SPAN_INTO_SURROGATES)?;
    Ok(Statement::Span {
        low: code_bytes(from, width),
        high: code_bytes(to, width),
        first: utf16be(character),
    })
}

/// Ends one run of an array span's kept destinations as a statement of its own.
fn flush_run(pieces: &mut Vec<Statement>, run: &mut Vec<Vec<u8>>, width: usize, from: u32) {
    if run.is_empty() {
        return;
    }
    let count = u32::try_from(run.len()).unwrap_or(u32::MAX);
    let to = from.saturating_add(count.saturating_sub(1));
    pieces.push(Statement::Each {
        low: code_bytes(from, width),
        high: code_bytes(to, width),
        texts: core::mem::take(run),
    });
}

/// Whether a destination states only placeholders, or an error where it mixes one with text.
///
/// A destination such as U+FEFF U+0041 is a producer having written the byte-order mark where
/// the character belonged — and taking the mark out would be this program deciding that, which is
/// a reading of the producer's intent rather than a removal of a statement.
fn placeholder_only(text: &[u8]) -> Result<bool, &'static str> {
    let Some(decoded) = text_of(text) else {
        return Ok(false);
    };
    let placeholders = decoded.chars().filter(|c| UNUSABLE.contains(c)).count();
    match placeholders {
        0 => Ok(false),
        all if all == decoded.chars().count() => Ok(true),
        _ => Err(PLACEHOLDER_BESIDE_TEXT),
    }
}

/// Writes `statements` as sections of [`SECTION_LIMIT`] at most, each with its count.
fn write_sections(out: &mut Vec<u8>, section: Section, statements: &[Statement]) {
    let (begin, end) = section.keywords();
    // The white space after the producer's closing keyword is outside the section and is carried,
    // so only a section this writes after another needs a line of its own.
    for (index, chunk) in statements.chunks(SECTION_LIMIT).enumerate() {
        if index > 0 {
            out.push(b'\n');
        }
        out.extend_from_slice(format!("{} {begin}\n", chunk.len()).as_bytes());
        for statement in chunk {
            match statement {
                Statement::Char { code, text } => {
                    out.extend_from_slice(format!("<{}> <{}>\n", hex(code), hex(text)).as_bytes());
                }
                Statement::Span { low, high, first } => out.extend_from_slice(
                    format!("<{}> <{}> <{}>\n", hex(low), hex(high), hex(first)).as_bytes(),
                ),
                Statement::Each { low, high, texts } => {
                    let each: Vec<String> = texts
                        .iter()
                        .map(|text| format!("<{}>", hex(text)))
                        .collect();
                    out.extend_from_slice(
                        format!("<{}> <{}> [{}]\n", hex(low), hex(high), each.join(" ")).as_bytes(),
                    );
                }
            }
        }
        out.extend_from_slice(end.as_bytes());
    }
}

/// Proves the rewrite: no placeholder stated, and every usable answer the producer's map gave
/// given again.
fn prove(before: &[u8], after: &[u8]) -> Result<(), &'static str> {
    let old = ToUnicode::parse(before);
    let new = ToUnicode::parse(after);
    if new.truncated().is_some() || old.truncated().is_some() {
        return Err(NOT_PROVED);
    }
    let mut asked = 0usize;
    for mapping in new.mappings() {
        let states_one = match mapping {
            Mapping::Single { text, .. } => text.chars().any(|c| UNUSABLE.contains(&c)),
            Mapping::Span { low, high, first } => UNUSABLE.iter().any(|c| {
                let value = u32::from(*c);
                value >= first && value <= first.saturating_add(high.saturating_sub(low))
            }),
        };
        if states_one {
            return Err(NOT_PROVED);
        }
    }
    for mapping in old.mappings() {
        let (low, high) = match mapping {
            Mapping::Single { code, .. } => (code, code),
            Mapping::Span { low, high, .. } => (low, high),
        };
        for code in low..=high {
            asked = asked.saturating_add(1);
            if asked > PROOF_BUDGET {
                return Err(PAST_THE_PROOF_BUDGET);
            }
            let mut was = String::new();
            if !old.append(code, &mut was) || was.chars().any(|c| UNUSABLE.contains(&c)) {
                continue;
            }
            let mut now = String::new();
            if !new.append(code, &mut now) || now != was {
                return Err(NOT_PROVED);
            }
        }
    }
    Ok(())
}

/// A code's value, its bytes read most significant first — `pdf_font`'s reading, one to four
/// bytes.
fn code_of(bytes: &[u8]) -> Option<u32> {
    if bytes.is_empty() || bytes.len() > 4 {
        return None;
    }
    bytes.iter().try_fold(0u32, |code, byte| {
        code.checked_mul(256)?.checked_add(u32::from(*byte))
    })
}

/// A code as `width` bytes, most significant first.
fn code_bytes(code: u32, width: usize) -> Vec<u8> {
    let all = code.to_be_bytes();
    all.get(all.len().saturating_sub(width)..)
        .unwrap_or(&all)
        .to_vec()
}

/// A destination's text, UTF-16BE — `pdf_font`'s reading, with a lone surrogate as U+FFFD.
fn text_of(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() || !bytes.len().is_multiple_of(2) {
        return None;
    }
    let units = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_be_bytes([pair[0], pair[1]]));
    let text: String = char::decode_utf16(units)
        .map(|unit| unit.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect();
    (!text.is_empty()).then_some(text)
}

/// A destination that is exactly one character, as its scalar value.
fn scalar_of(bytes: &[u8]) -> Option<u32> {
    let text = text_of(bytes)?;
    let mut chars = text.chars();
    let first = chars.next()?;
    chars.next().is_none().then(|| u32::from(first))
}

/// One character as §9.10.3's destination bytes, UTF-16BE.
fn utf16be(character: char) -> Vec<u8> {
    let mut units = [0u16; 2];
    character
        .encode_utf16(&mut units)
        .iter()
        .flat_map(|unit| unit.to_be_bytes())
        .collect()
}

/// Bytes as upper-case hexadecimal digits.
fn hex(bytes: &[u8]) -> String {
    use core::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, byte| {
        // `fmt::Write for String` cannot fail; the parent module's `appended` says why.
        let _ = write!(out, "{byte:02X}");
        out
    })
}

/// Why a section the stream ends inside is not restated.
const UNFINISHED_SECTION: &str = "this font's ToUnicode CMap ends inside a beginbfchar or \
     beginbfrange section, so where the producer meant that section to stop is not in the file";

/// Why a map that turned out to state no placeholder is not restated.
const NOTHING_TO_TAKE_OUT: &str = "this font's ToUnicode CMap states no value this conversion \
     could take out, though the validator found one — a reading the two disagree on, refused \
     rather than guessed past";

/// Why a destination mixing a placeholder with text is not restated.
const PLACEHOLDER_BESIDE_TEXT: &str = "this font's ToUnicode CMap maps a code to a placeholder \
     value written beside a character, and taking the placeholder out would be this program \
     deciding what the producer meant the code to stand for";

/// Why a span that would continue into the surrogate code points is not restated.
const SPAN_INTO_SURROGATES: &str = "splitting one of this font's ToUnicode spans around a \
     placeholder would begin a span at a surrogate code point, which is no character";

/// Why a map whose proof would ask too many codes is not restated.
const PAST_THE_PROOF_BUDGET: &str = "this font's ToUnicode CMap states more codes than this \
     conversion proves a rewrite over, and a rewrite that is not proved is not written";

/// Why a rewrite the proof does not confirm is not written.
const NOT_PROVED: &str = "this font's ToUnicode CMap, rewritten without its placeholder values, \
     did not answer every code the producer's map answered with a usable value the same way, so \
     it is not written";

#[cfg(test)]
mod tests {
    use super::{PLACEHOLDER_BESIDE_TEXT, restate};
    use pdf_font::tounicode::ToUnicode;

    fn answer(map: &ToUnicode, code: u32) -> Option<String> {
        let mut out = String::new();
        map.append(code, &mut out).then_some(out)
    }

    /// An identity span over two-byte codes loses exactly its three placeholder codes, and every
    /// other code answers as it did; the codespace is carried byte for byte.
    #[test]
    fn an_identity_span_is_split_around_its_three_placeholders() {
        let source = b"1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
                       1 beginbfrange\n<0000> <FFFF> <0000>\nendbfrange\nendcmap\n";
        let restated = restate(source).expect("restated");
        assert_eq!(
            restated.unmapped.into_iter().collect::<Vec<_>>(),
            vec![0x0000, 0xFEFF, 0xFFFE]
        );
        let text = String::from_utf8(restated.data.clone()).expect("ascii");
        assert!(text.starts_with("1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n"));
        assert!(text.contains("<0001> <FEFE> <0001>"), "{text}");
        assert!(text.contains("<FF00> <FFFD> <FF00>"), "{text}");
        assert!(text.contains("<FFFF> <FFFF> <FFFF>"), "{text}");
        let map = ToUnicode::parse(&restated.data);
        assert_eq!(answer(&map, 0x0041).as_deref(), Some("A"));
        assert_eq!(answer(&map, 0x0000), None);
        assert_eq!(answer(&map, 0xFEFF), None);
    }

    /// A `bfchar` to U+0000 is taken out and its neighbours kept, in the producer's code width.
    #[test]
    fn a_code_mapped_to_zero_is_taken_out_and_its_neighbours_kept() {
        let source = b"3 beginbfchar\n<01> <0041>\n<02> <0000>\n<03> <0042>\nendbfchar\n";
        let restated = restate(source).expect("restated");
        assert_eq!(restated.unmapped.into_iter().collect::<Vec<_>>(), vec![2]);
        assert_eq!(
            String::from_utf8(restated.data).expect("ascii"),
            "2 beginbfchar\n<01> <0041>\n<03> <0042>\nendbfchar\n"
        );
    }

    /// An array span drops only the destination that was a placeholder, as two spans.
    #[test]
    fn an_array_span_drops_only_its_placeholder() {
        let source = b"1 beginbfrange\n<10> <12> [<0041> <FEFF> <0043>]\nendbfrange\n";
        let restated = restate(source).expect("restated");
        let map = ToUnicode::parse(&restated.data);
        assert_eq!(answer(&map, 0x10).as_deref(), Some("A"));
        assert_eq!(answer(&map, 0x11), None);
        assert_eq!(answer(&map, 0x12).as_deref(), Some("C"));
    }

    /// A placeholder written beside a character is refused rather than trimmed (ADR 2294).
    #[test]
    fn a_placeholder_beside_a_character_is_refused() {
        let source = b"1 beginbfchar\n<01> <FEFF0041>\nendbfchar\n";
        assert_eq!(restate(source), Err(PLACEHOLDER_BESIDE_TEXT));
    }

    /// A map with nothing to take out is no restatement.
    #[test]
    fn a_map_with_no_placeholder_is_not_restated() {
        assert!(restate(b"1 beginbfchar\n<01> <0041>\nendbfchar\n").is_err());
    }
}
