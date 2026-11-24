//! ISO 19005-2 section 6.1.13, for the integers written inside a content stream: an operand past
//! the part's integer bound written as the real of the same value, its digits followed by a
//! period (ADR 3533).
//!
//! # Why this is not editing the producer's page
//!
//! ISO 32000-2 §7.3.3 makes both of its numeric forms the same kind of thing — "Integer objects
//! represent mathematical integers. Real objects represent mathematical real numbers" — and spells
//! the real form as digits with "a leading, trailing, or embedded PERIOD". So
//! `2366454844678207931159090160467968.` is the number `2366454844678207931159090160467968` names,
//! and a period written after the last digit is a change of form and not of value. The same
//! paragraph admits the change where the operand is a number:
//!
//! > Wherever a real number is expected, an integer may be used instead.
//!
//! Section 6.1.13 bounds the two forms differently, and a value between the integer bound and the
//! real bound is one the part admits in the second form only. Nothing a reader computes changes:
//! no operator is added, removed or respelled, no operand moves, and every byte of the stream that
//! is not one of those periods is the producer's. `doc/pdf-a-conversion-limits.md` section 5.2 is
//! where the fence names it beside `A50`'s spellings, `A978`'s `/I` and ADR 1176's digit.
//!
//! # Where it is written, and where it is not
//!
//! Only as a **direct operand of an operator whose operands are numbers** ([`NUMERIC_OPERATORS`]):
//! the path, matrix, text-position, colour and line-width operators of §8.2's Table 50. An
//! operator that expects an *integer* — `J`, `j`, `Tr` — would be handed a real where §7.3.3 says
//! "\[a] real number shall not be present when an integer is expected", and an integer inside an
//! array or a dictionary is a value whose type this walk does not know. Either keeps the row's
//! refusal for the whole document, as does a value whose magnitude is past the real bound too,
//! and any other value of the row (a real, a string or a name past its bound), which no period
//! answers. A requirement half answered is a requirement refused.
//!
//! **A stream another content repair writes is refused rather than written twice**, as
//! `super::inline_smoothing` refuses it: the repairs are not composed.

use std::collections::{BTreeMap, BTreeSet};

use pdf_archive::survey::Survey;
use pdf_model::Pages;
use pdf_syntax::object::{Dictionary, Object, ObjectId};
use pdf_syntax::{Document, Lexer, Token};

use super::decision::Because;
use super::hexadecimal;

/// The requirement this answers.
pub(super) const ROW: &str = "implementation-limits/values-written-in-content-streams";

/// The same subclause's row for the integers of the file's objects, which the reader's log also
/// fails where a content stream states an integer too wide for 64 bits (`pdf_archive::repairs`).
pub(super) const INTEGER_VALUES: &str = "implementation-limits/integer-values";

/// The class of the reader's log that names such an integer (`pdf_syntax::repair`).
const LOGGED_WIDE_INTEGER: &str = "syntax/integer-past-i64";

/// The magnitude past which an integer the reader lexed is a real: `i64`'s, so that a page
/// dictionary stating one is told from a page whose content stream did.
const PAST_I64: f64 = 9_223_372_036_854_775_807.0;

/// ISO 19005-2 section 6.1.13's largest integer.
const LARGEST_INTEGER: i128 = 2_147_483_647;
/// ISO 19005-2 section 6.1.13's smallest integer.
const SMALLEST_INTEGER: i128 = -2_147_483_648;
/// ISO 19005-2 section 6.1.13's largest real magnitude.
const LARGEST_REAL: f64 = 3.403e38;
/// ISO 19005-2 section 6.1.13's smallest non-zero real magnitude.
const SMALLEST_REAL: f64 = 1.175e-38;
/// ISO 19005-2 section 6.1.13's longest string, in bytes.
const LONGEST_STRING: usize = 32_767;
/// ISO 19005-2 section 6.1.13's longest name, in bytes.
const LONGEST_NAME: usize = 127;

/// The operators of ISO 32000-2 §8.2's Table 50 every one of whose direct operands is a number,
/// so that a real stands wherever an integer does (§7.3.3). `d`'s array is not a direct operand,
/// and `Tf`'s and `scn`'s names are not integers, so neither keeps an operand from this list.
const NUMERIC_OPERATORS: &[&[u8]] = &[
    b"m", b"l", b"c", b"v", b"y", b"re", b"cm", b"w", b"M", b"d", b"i", b"Td", b"TD", b"Tm", b"Tc",
    b"Tw", b"Tz", b"TL", b"Ts", b"Tf", b"d0", b"d1", b"g", b"G", b"rg", b"RG", b"k", b"K", b"sc",
    b"SC", b"scn", b"SCN",
];

/// Why a stream whose bytes cannot be written back is not restated.
const STREAM_WILL_NOT_RE_ENCODE: &str = "a content stream stating an integer past ISO 19005-2 \
     section 6.1.13's bound cannot be written back: it will not re-encode, or its /DecodeParms \
     names another object that discarding the parameters would orphan";

/// Why a stream another content repair also writes is not restated.
const ALSO_ANOTHER_REPAIR: &str = "a content stream whose integers past ISO 19005-2 section \
     6.1.13's bound are to be written as reals is also written by another repair of its content, \
     and the two are not composed";

/// What the walk of every content stream found, before the photographs are known.
#[derive(Debug, Default)]
pub(super) struct Walked {
    /// The offsets just past each integer to be followed by a period, by stream.
    periods: BTreeMap<ObjectId, Vec<usize>>,
    /// The streams stating a value past one of the row's bounds that no period answers.
    unanswered: BTreeSet<ObjectId>,
}

/// The content streams written with each integer past the bound as the real of its value.
#[derive(Debug, Default)]
pub(super) struct Restated {
    /// The stream each object is to carry.
    pub(super) at: BTreeMap<ObjectId, Object>,
    /// How many integers each stream of `at` had restated.
    written_at: BTreeMap<ObjectId, usize>,
    /// The pages whose content this answers — one of their streams is in `at`, or a photograph
    /// took their content off — and whose own dictionary states no value the reader could have
    /// lexed from an integer too wide for 64 bits.
    pages: BTreeSet<ObjectId>,
}

impl Restated {
    /// How many integers were written as reals, over every stream.
    pub(super) fn integers(&self) -> usize {
        self.written_at.values().sum()
    }

    /// Whether the photographs answer the row alone: every stream stating a value past a bound
    /// was on a photographed page, and none is left to write.
    pub(super) fn only_on_photographed_pages(&self) -> bool {
        self.at.is_empty()
    }

    /// Whether every place [`INTEGER_VALUES`] failed at is the reader's log of an integer too
    /// wide for 64 bits, read at a page whose content this answers.
    ///
    /// The log names the page whose content the lexer was reading, so the row that bounds the
    /// file's *objects* fails where only the page's content stream stated the integer. Where the
    /// page's own dictionary states nothing the lexer could have read from such an integer, the
    /// place is the content stream's and the content's answer is the row's; anywhere else the
    /// row keeps its refusal (`doc/adr/3533`).
    pub(super) fn answers_the_log(&self, judgement: &pdf_archive::Judgement) -> bool {
        let pdf_archive::Outcome::Failed { places, total } = &judgement.outcome else {
            return false;
        };
        judgement.id == INTEGER_VALUES
            && *total == places.len()
            && !places.is_empty()
            && places.iter().all(|finding| {
                finding.what.starts_with(LOGGED_WIDE_INTEGER)
                    && finding
                        .place
                        .object
                        .is_some_and(|id| self.pages.contains(&id))
            })
    }
}

/// What one stream's walk found.
#[derive(Debug, PartialEq, Eq)]
enum Found {
    /// The offset just past each integer to be followed by a period, in stream order.
    Periods(Vec<usize>),
    /// A value past one of the row's bounds that no period answers: a real, a string or a name
    /// past its own, a string the stream ends inside, or an integer inside an array or a
    /// dictionary, an operand of an operator outside [`NUMERIC_OPERATORS`] or of none, or past
    /// the real bound as well.
    Unanswered,
}

/// Walks every content stream the page tree and the survey reach for the values the row bounds.
///
/// # Errors
///
/// `refused`, the row's sentence, where no survey was made.
pub(super) fn walk(
    document: &Document,
    survey: Option<&Survey>,
    refused: Because,
) -> Result<Walked, Because> {
    let survey = survey.ok_or(refused)?;
    let pages = Pages::new(document);
    // The streams some page's `/Contents` array continues past: §7.7.3.3 reads the array as one
    // stream, so a string one of these ends inside may run on into the next.
    let mut continued = BTreeSet::new();
    for page in pages.every().into_iter().flatten() {
        let contents = hexadecimal::contents_of(document, &page.dict);
        if let Some((_, before)) = contents.split_last() {
            continued.extend(before.iter().copied());
        }
    }
    let mut out = Walked::default();
    for (id, resources) in hexadecimal::streams(document, &pages, Some(survey)) {
        let object = document.get(id);
        let Some(stream) = object.as_stream() else {
            continue;
        };
        let Some(data) = document.decoded_stream_data(stream) else {
            continue;
        };
        match values_past_a_bound(document, &data, &resources, continued.contains(&id)) {
            Found::Unanswered => {
                out.unanswered.insert(id);
            }
            Found::Periods(periods) if periods.is_empty() => {}
            Found::Periods(periods) => {
                out.periods.insert(id, periods);
            }
        }
    }
    Ok(out)
}

impl Walked {
    /// Writes every integer the walk found as the real of its value, leaving out every stream of
    /// `taken_off` — the streams a photograph of each page of `photographed` takes off its page
    /// (`super::hexadecimal::taken_off_by`), whose values the output no longer states.
    ///
    /// # Errors
    ///
    /// `refused`, the row's sentence, where a stream left on its page states a value
    /// [`Found::Unanswered`] names, or where no stream states a value past a bound at all; one of
    /// the two sentences above where a stream cannot be written.
    pub(super) fn settle(
        self,
        document: &Document,
        (taken_off, photographed): (&BTreeSet<ObjectId>, &BTreeSet<ObjectId>),
        others: &[&BTreeMap<ObjectId, Object>],
        refused: Because,
    ) -> Result<Restated, Because> {
        if self.unanswered.iter().any(|id| !taken_off.contains(id)) {
            return Err(refused);
        }
        let answered_by_photographs = self
            .unanswered
            .iter()
            .chain(self.periods.keys())
            .any(|id| taken_off.contains(id));
        let mut out = Restated::default();
        for (id, periods) in self.periods {
            if taken_off.contains(&id) {
                continue;
            }
            if others.iter().any(|other| other.contains_key(&id)) {
                return Err(Because::NotBuiltYet(ALSO_ANOTHER_REPAIR));
            }
            let object = document.get(id);
            let data = object.as_stream().and_then(|stream| {
                document
                    .decoded_stream_data(stream)
                    .map(|data| (stream, data))
            });
            let Some((stream, data)) = data else {
                return Err(refused);
            };
            let written = with_periods(&data, &periods);
            let replacement = hexadecimal::repaired(stream, &written)
                .ok_or(Because::NotBuiltYet(STREAM_WILL_NOT_RE_ENCODE))?;
            out.written_at.insert(id, periods.len());
            out.at.insert(id, replacement);
        }
        if out.at.is_empty() && !answered_by_photographs {
            return Err(refused);
        }
        for page in Pages::new(document).every().into_iter().flatten() {
            let Some(id) = page.id else {
                continue;
            };
            let answered = photographed.contains(&id)
                || hexadecimal::contents_of(document, &page.dict)
                    .iter()
                    .any(|stream| out.at.contains_key(stream));
            if answered
                && !page
                    .dict
                    .iter()
                    .any(|(_, value)| states_a_wide_value(value, 1))
            {
                out.pages.insert(id);
            }
        }
        Ok(out)
    }
}

/// How deep [`states_a_wide_value`] reads a direct value before it says yes.
const MOST_DEPTH: usize = 16;

/// Whether `value`, read through its direct arrays and dictionaries, holds a real as large as an
/// integer too wide for 64 bits — the value the lexer reads such an integer as.
fn states_a_wide_value(value: &Object, depth: usize) -> bool {
    if depth > MOST_DEPTH {
        return true;
    }
    match value {
        Object::Real(number) => number.abs() >= PAST_I64,
        Object::Array(items) => items
            .iter()
            .any(|item| states_a_wide_value(item, depth.saturating_add(1))),
        Object::Dictionary(dict) => dict
            .iter()
            .any(|(_, item)| states_a_wide_value(item, depth.saturating_add(1))),
        _ => false,
    }
}

/// `data` with a period after each offset of `periods`, every other byte as it was.
fn with_periods(data: &[u8], periods: &[usize]) -> Vec<u8> {
    let mut written = Vec::with_capacity(data.len().saturating_add(periods.len()));
    let mut from = 0;
    for &at in periods {
        written.extend_from_slice(data.get(from..at).unwrap_or_default());
        written.push(b'.');
        from = at;
    }
    written.extend_from_slice(data.get(from..).unwrap_or_default());
    written
}

/// The values past one of the row's bounds in one decoded content stream, as [`Found`] says.
///
/// **The lexer decides where a token ends**, as in `super::hexadecimal`: a run is an integer by
/// §7.3.3 where its bytes are a sign and digits, whichever of the two tokens the lexer returned it
/// as — one past `i64` comes back as the real nearest it (ADR 3393), and its spelling is still
/// §7.3.3's integer form.
fn values_past_a_bound(
    document: &Document,
    data: &[u8],
    resources: &Dictionary,
    continued: bool,
) -> Found {
    let mut lexer = Lexer::new(data);
    let mut periods = Vec::new();
    let mut pending: Vec<usize> = Vec::new();
    let mut depth = 0_usize;
    loop {
        lexer.skip_whitespace();
        let start = lexer.position();
        let Some(token) = lexer.next_token() else {
            break;
        };
        let end = lexer.position();
        match token {
            Token::Keyword(b"BI") => {
                // §8.9.7's inline image data is not a program, and its dictionary's values are
                // integers where §8.9.7's Table 91 expects them: neither is an operand here, and
                // a value past a bound there is not one this writes.
                if !pending.is_empty() {
                    return Found::Unanswered;
                }
                let scan = pdf_model::inline_image::scan(
                    document,
                    data,
                    lexer.position(),
                    resources,
                    true,
                );
                lexer.seek(scan.resume);
            }
            Token::Keyword(operator) => {
                if !pending.is_empty() {
                    if depth > 0 || !NUMERIC_OPERATORS.contains(&operator) {
                        return Found::Unanswered;
                    }
                    periods.append(&mut pending);
                }
            }
            Token::ArrayOpen | Token::DictOpen => depth = depth.saturating_add(1),
            Token::ArrayClose | Token::DictClose => depth = depth.saturating_sub(1),
            Token::Integer(_) | Token::Real(_) => {
                let spelling = data.get(start..end).unwrap_or_default();
                match (spelled(spelling), token) {
                    (Some(Spelled::PastEither), _) => return Found::Unanswered,
                    (Some(Spelled::Restatable), _) if depth > 0 => return Found::Unanswered,
                    (Some(Spelled::Restatable), _) => pending.push(end),
                    // A real is past its bound as the lexer read it, `±f64::MAX` included
                    // (ADR 0733), and zero is no magnitude the lower bound speaks of.
                    (None, Token::Real(value))
                        if value.abs() > LARGEST_REAL
                            || (value != 0.0 && value.abs() < SMALLEST_REAL) =>
                    {
                        return Found::Unanswered;
                    }
                    (Some(Spelled::Within) | None, _) => {}
                }
            }
            Token::String(bytes) => {
                // A string the stream ends inside runs to the stream's last byte, and where the
                // page's `/Contents` goes on, §7.7.3.3's concatenation may continue it into the
                // next stream, past what this walk reads.
                if bytes.len() > LONGEST_STRING || (continued && end >= data.len()) {
                    return Found::Unanswered;
                }
            }
            Token::Name(bytes) if bytes.len() > LONGEST_NAME => return Found::Unanswered,
            Token::Name(_) => {}
        }
    }
    if pending.is_empty() {
        Found::Periods(periods)
    } else {
        Found::Unanswered
    }
}

/// What one numeric token's spelling states against ISO 19005-2 section 6.1.13's bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Spelled {
    /// §7.3.3's integer form, within the integer bound.
    Within,
    /// §7.3.3's integer form past the integer bound and within the real bound, which a period
    /// after the last digit answers.
    Restatable,
    /// §7.3.3's integer form past the real bound as well, which no period answers.
    PastEither,
}

/// What `spelling` states, where it is §7.3.3's integer form: a sign and digits. `None` for any
/// other spelling, which this module leaves alone.
fn spelled(spelling: &[u8]) -> Option<Spelled> {
    let digits = match spelling.first() {
        Some(b'+' | b'-') => spelling.get(1..)?,
        _ => spelling,
    };
    if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let text = std::str::from_utf8(spelling).ok()?;
    if text
        .parse::<i128>()
        .is_ok_and(|value| (SMALLEST_INTEGER..=LARGEST_INTEGER).contains(&value))
    {
        return Some(Spelled::Within);
    }
    // Past `i128` the parse fails, and the value is past the integer bound all the same; the
    // real nearest it is what the real bound is read against.
    let magnitude = text.parse::<f64>().ok()?.abs();
    Some(if magnitude <= LARGEST_REAL {
        Spelled::Restatable
    } else {
        Spelled::PastEither
    })
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;

    fn document() -> Document {
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [] /Count 0 >>",
        ];
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let at = out.len();
        let _ = write!(out, "xref\n0 3\n0000000000 65535 f \n");
        for offset in offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{at}\n%%EOF\n"
        );
        Document::open(out.into_bytes()).expect("the fixture opens")
    }

    fn found(data: &[u8]) -> Found {
        values_past_a_bound(&document(), data, &Dictionary::new(), true)
    }

    /// `475121`'s own shape: thirty-four digits as `m` and `l` operands, which §7.3.3 makes the
    /// integer form and section 6.1.13 bounds; the period after each is the only byte added.
    #[test]
    fn a_wide_path_operand_gains_a_period_and_no_other_byte_moves() {
        let data: &[u8] =
            b"q -2366213136885537460660416106463232 2366454844678207931159090160467968\n\
                            m 2147483647 -2147483648 l 0.5 1 l\nS Q";
        let Found::Periods(periods) = found(data) else {
            panic!("both wide operands are answered");
        };
        assert_eq!(periods.len(), 2, "the bound itself is within the bound");
        let written = with_periods(data, &periods);
        assert_eq!(written.len(), data.len().saturating_add(2));
        let without: Vec<u8> = written
            .iter()
            .enumerate()
            .filter(|(at, _)| {
                !periods
                    .iter()
                    .enumerate()
                    .any(|(n, p)| p.saturating_add(n) == *at)
            })
            .map(|(_, byte)| *byte)
            .collect();
        assert_eq!(
            without, data,
            "every byte but the periods is the producer's"
        );
        let text = String::from_utf8(written).expect("ASCII");
        assert!(text.contains(
            "-2366213136885537460660416106463232. 2366454844678207931159090160467968.\nm"
        ));
    }

    /// §7.3.3: "A real number shall not be present when an integer is expected" — so `Tr`'s
    /// operand, an array element and a dictionary value are left to the refusal.
    #[test]
    fn an_integer_operand_an_array_element_and_a_dictionary_value_are_not_written() {
        assert_eq!(found(b"3000000000 Tr"), Found::Unanswered);
        assert_eq!(found(b"[3000000000 1] 0 d"), Found::Unanswered);
        assert_eq!(
            found(b"/Span << /MCID 3000000000 >> BDC EMC"),
            Found::Unanswered
        );
        assert_eq!(
            found(b"0 0 m 3000000000"),
            Found::Unanswered,
            "an operand of no operator"
        );
    }

    /// A value past the real bound as well stays refused: the period would leave it past the bound.
    #[test]
    fn a_value_past_the_real_bound_too_is_not_written() {
        let forty = [b'9'; 40];
        let mut data = forty.to_vec();
        data.extend_from_slice(b" 0 m");
        assert_eq!(found(&data), Found::Unanswered);
        assert_eq!(
            spelled(b"340282366920938463463374607431768211456"),
            Some(Spelled::Restatable)
        );
    }

    #[test]
    fn only_the_integer_form_is_read() {
        assert_eq!(spelled(b"3000000000.5"), None);
        assert_eq!(spelled(b"1e40"), None);
        assert_eq!(spelled(b"+2147483648"), Some(Spelled::Restatable));
        assert_eq!(spelled(b"-2147483648"), Some(Spelled::Within));
        assert_eq!(found(b"1 0 0 1 0 0 cm 0 0 m"), Found::Periods(Vec::new()));
    }

    /// The other values the row bounds, which no period answers: each keeps the refusal for its
    /// stream, and a string the stream ends inside is read as possibly longer than it looks.
    #[test]
    fn a_real_a_string_and_a_name_past_their_bounds_are_unanswered() {
        assert_eq!(found(b"1e40 0 m"), Found::Unanswered);
        assert_eq!(
            found(b"0.00000000000000000000000000000000000000001 w"),
            Found::Unanswered
        );
        assert_eq!(
            found(b"0 w 0.0 0 m"),
            Found::Periods(Vec::new()),
            "zero is within"
        );
        let mut long = b"/".to_vec();
        long.extend_from_slice(&[b'a'; 128]);
        long.extend_from_slice(b" gs");
        assert_eq!(found(&long), Found::Unanswered);
        assert_eq!(found(b"BT (abc) Tj ET (unclosed"), Found::Unanswered);
        assert_eq!(found(b"BT (abc) Tj ET"), Found::Periods(Vec::new()));
        // The last stream of its page has nothing to continue a cut string into.
        assert_eq!(
            values_past_a_bound(&document(), b"BT (unclosed", &Dictionary::new(), false),
            Found::Periods(Vec::new())
        );
    }
}
