//! ISO 19005-2 section 6.2.2 and ISO 19005-4 section 6.2.2, where the operator a content stream
//! uses and the base standard does not define is a run this reader draws as a number: the number
//! it draws, written in place of the run (`doc/adr/2638`).
//!
//! # What the run is, and what this reader draws for it
//!
//! ISO 32000-2 §7.2.3 makes a run of regular characters one token:
//!
//! > A sequence of consecutive regular characters comprises a single token.
//!
//! and §7.3.3 spells a real with one period, so `33600.272.58` is no number. To the checker it is
//! the keyword §7.8.2 makes an operator (`doc/adr/2619`); the window draws the number the lexer
//! reads off its front, `33600.272`, wherever the rest names no operator of §8.2 Table 50
//! (`doc/adr/1004`). The page a reader sees is the page with that number in that place.
//!
//! # Why this sits on the near side of the authoring line
//!
//! The bytes written are the operand this reader already draws, in §7.3.3's own spelling, at the
//! place the producer wrote the run; no operator is added, removed or moved, and nothing around the
//! run changes. It invents no mark: the page draws as the window drew it before, which is the
//! photograph's own test (`doc/scope-ratifications.md`, A570) without the photograph's cost. It is
//! still a loss and is carried as one, `content-as-drawn`, because the characters after the number
//! are the producer's and are gone, and this reader's salvage is ADR 1004's choice rather than a
//! value the standard states — the difference from §7.3.4.3's assumed hexadecimal digit, which
//! `super::hexadecimal` writes losing nothing. A run this reader does not draw as a number — one
//! whose tail is an operator, or that opens with no number at all — is not touched, and its page
//! keeps whatever answer the fence gives it.
//!
//! # A lone delimiter, which this reader reads as no operator
//!
//! §7.2.3 makes `>` and `)` delimiters, and one that closes nothing opens no object: this tree's
//! lexer hands it over as a keyword, the checker reads it as an operator the base standard does
//! not define, and the reader meets the error §7.8.2 states — "an error shall occur" — and clears
//! the operands before it (`pdf_model::content::run`). Where no operand stands before it, that
//! clearing clears nothing, and the page draws as though the byte were white space; so the byte is
//! written as one white-space character, which §7.2.3 makes the separator it already was, under
//! the same loss, the producer's byte gone (`doc/adr/2859`). One that stands after an operand, or
//! inside an array or a dictionary, does change what the reader draws by being there, and is left
//! where it is: its stream keeps the operator and the row its answer.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use pdf_archive::survey::{Survey, UnlistedOperator};
use pdf_model::Pages;
use pdf_model::content::reader::draws_the_salvage;
use pdf_syntax::object::{Object, ObjectId};
use pdf_syntax::{Document, Lexer, Token};

use super::decision::Because;
use super::hexadecimal;

/// Why a stream whose bytes cannot be written back keeps its runs.
const STREAM_WILL_NOT_RE_ENCODE: &str = "a content stream holding a run this reader draws as a \
     number cannot be written back: it will not re-encode, or its /DecodeParms names another \
     object that discarding the parameters would orphan. So the number cannot be written in the \
     run's place";

/// Why a document naming such a run nowhere this walk reaches keeps the refusal.
const NO_STREAM_HOLDS_ONE: &str = "no content stream this conversion reads holds a run this \
     reader draws as a number, so the operator the base standard does not define is something \
     else, and writing a number would answer nothing";

/// The most runs one conversion writes, over every content stream it reads.
///
/// `CLAUDE.md` principle 3's explicit budget, checked as each run is found and before its record
/// is kept: a record costs a few hundred bytes (its pages, its bytes and its number), and the
/// stream bounds the runs only by half its decoded length, so a stream of runs alone would
/// otherwise cost some sixty times its length. `509284`, the document this remedy was built for,
/// holds 58; a document past the bound keeps the operator row's refusal (`doc/adr/2638`).
const MOST_RUNS: usize = 1 << 16;

/// Why a document holding more runs than [`MOST_RUNS`] keeps the refusal.
const TOO_MANY_RUNS: &str = "the content streams hold more than 65536 runs this reader draws as \
     numbers, more than this conversion writes, so the operator the base standard does not define \
     stays where the producer wrote it";

/// Why a stream that also omits a hexadecimal digit keeps its runs.
const ALSO_A_HEXADECIMAL_REPAIR: &str = "a content stream holding a run this reader draws as a \
     number also states a hexadecimal string with an odd number of digits, and the two rewrites \
     of one stream are not composed";

/// Why a stream whose `/Lang` is also respelled keeps its runs.
const ALSO_A_LANGUAGE_RESPELLED: &str = "a content stream holding a run this reader draws as a \
     number also states a /Lang that super::languages respells, and the two rewrites of one \
     stream are not composed";

/// One run a conversion wrote as the number this reader draws for it, under the loss
/// `content-as-drawn` (`doc/adr/2638`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrittenRun {
    /// The pages, counting from one, whose content the checker read the run on.
    pub pages: Vec<usize>,
    /// The content stream object it stood in, in the source's numbering.
    pub object: ObjectId,
    /// The run as the producer wrote it.
    pub run: Vec<u8>,
    /// The number written in its place, in ISO 32000-2 §7.3.3's spelling; empty where the run is
    /// a lone delimiter, written as one white-space character (`doc/adr/2859`).
    pub number: Vec<u8>,
}

/// The most runs `xmpMM:History` names one by one; the rest are counted.
const MOST_NAMED: usize = 16;

/// The content streams this conversion rewrites, and every run it wrote a number for.
#[derive(Debug, Default)]
pub(super) struct Drawn {
    /// The stream each object is to carry.
    pub(super) at: BTreeMap<ObjectId, Object>,
    /// Every run written, in stream order.
    pub(super) runs: Vec<WrittenRun>,
    /// Whether every operator the survey found that the base standard does not define is a run
    /// this reader draws as a number, so that writing the numbers answers the operator row.
    pub(super) answers_every_operator: bool,
    /// The content streams the survey read whose filters do not decode whole, so that no run in
    /// them is written (ADR 2803).
    pub(super) unread: BTreeSet<ObjectId>,
    /// Whether a lone delimiter stood where removing it would change what this reader draws —
    /// after an operand, or inside an array or a dictionary — and so was left (`doc/adr/2859`).
    pub(super) delimiter_left: bool,
}

impl Drawn {
    /// Whether every operator the survey reports under `spelling` is one this write answers: a run
    /// this reader draws as a number, or a lone delimiter where none was left.
    pub(super) fn answers(&self, spelling: &[u8]) -> bool {
        is_drawn_as_a_number(spelling) || (is_a_lone_delimiter(spelling) && !self.delimiter_left)
    }

    /// Whether the stream an operator the survey found stood in is one this write could not read,
    /// so that the run in it is still in the output: the object the survey names, or for a page's
    /// own content, any stream its `/Contents` names.
    pub(super) fn left(&self, document: &Document, unlisted: &UnlistedOperator) -> bool {
        if self.unread.is_empty() {
            return false;
        }
        match unlisted.place.object {
            Some(object) => self.unread.contains(&object),
            None => Pages::new(document).get(unlisted.page).is_none_or(|page| {
                hexadecimal::contents_of(document, &page.dict)
                    .iter()
                    .any(|id| self.unread.contains(id))
            }),
        }
    }
}

/// Writes the number this reader draws in place of every run it salvages, in every content
/// stream the page tree and the survey reach.
///
/// `hexadecimal` is the other content-stream repair, where it was asked for: a stream both would
/// rewrite is refused rather than written twice.
pub(super) fn write(
    document: &Document,
    survey: Option<&Survey>,
    hexadecimal: Option<&hexadecimal::Completed>,
) -> Result<Drawn, Because> {
    // A lone delimiter is written only where the write then answers every operator: where some
    // other operator stays, the page keeps the answer it had without this write, the photograph's
    // included, and the delimiter stays with it (`doc/adr/2859`).
    match write_places(document, survey, hexadecimal, true) {
        Ok(drawn)
            if drawn.answers_every_operator
                || drawn.runs.iter().all(|run| !run.number.is_empty()) =>
        {
            Ok(drawn)
        }
        _ => write_places(document, survey, hexadecimal, false),
    }
}

/// [`write()`], with lone delimiters written where `delimiters` is set and every one of them left
/// where it is not.
fn write_places(
    document: &Document,
    survey: Option<&Survey>,
    hexadecimal: Option<&hexadecimal::Completed>,
    delimiters: bool,
) -> Result<Drawn, Because> {
    let pages = Pages::new(document);
    let mut read_on: BTreeMap<&[u8], BTreeSet<usize>> = BTreeMap::new();
    for unlisted in survey.map_or(&[][..], Survey::unlisted_operators) {
        read_on
            .entry(unlisted.spelling.as_slice())
            .or_default()
            .insert(unlisted.page.saturating_add(1));
    }
    let mut out = Drawn {
        answers_every_operator: survey.is_some_and(|survey| {
            survey.unlisted_operators().iter().all(|unlisted| {
                is_drawn_as_a_number(&unlisted.spelling) || is_a_lone_delimiter(&unlisted.spelling)
            })
        }),
        ..Drawn::default()
    };
    for (id, resources) in hexadecimal::streams(document, &pages, survey) {
        let object = document.get(id);
        let Some(stream) = object.as_stream() else {
            continue;
        };
        let Some(data) = document.decoded_stream_data(stream) else {
            // The survey read this stream through the content reader, which draws what a
            // damaged filter yields before the damage; the whole decode the bytes would be
            // written back from does not exist, so its runs stay and its page is photographed
            // where the profile allows (ADR 2803).
            out.unread.insert(id);
            continue;
        };
        let room = MOST_RUNS.saturating_sub(out.runs.len());
        let (places, delimiter_left) = salvaged_runs(document, &data, &resources, room, delimiters)
            .ok_or(Because::NotBuiltYet(TOO_MANY_RUNS))?;
        out.delimiter_left |= delimiter_left;
        if places.is_empty() {
            continue;
        }
        if hexadecimal.is_some_and(|completed| completed.at.contains_key(&id)) {
            return Err(Because::NotBuiltYet(ALSO_A_HEXADECIMAL_REPAIR));
        }
        let mut written = Vec::with_capacity(data.len());
        let mut from = 0;
        for place in places {
            written.extend_from_slice(data.get(from..place.start).unwrap_or_default());
            // A lone delimiter's place is written as the white space §7.2.3 already read it as,
            // so the tokens either side of it stay apart.
            if place.number.is_empty() {
                written.push(b' ');
            } else {
                written.extend_from_slice(&place.number);
            }
            from = place.end;
            let run = data
                .get(place.start..place.end)
                .unwrap_or_default()
                .to_vec();
            out.runs.push(WrittenRun {
                pages: read_on
                    .get(run.as_slice())
                    .map(|pages| pages.iter().copied().collect())
                    .unwrap_or_default(),
                object: id,
                run,
                number: place.number,
            });
        }
        written.extend_from_slice(data.get(from..).unwrap_or_default());
        let replacement = hexadecimal::repaired(stream, &written)
            .ok_or(Because::NotBuiltYet(STREAM_WILL_NOT_RE_ENCODE))?;
        out.at.insert(id, replacement);
    }
    if out.at.is_empty() {
        return Err(Because::NotBuiltYet(NO_STREAM_HOLDS_ONE));
    }
    // A run left in a stream this write could not read is an operator the numbers do not answer,
    // and so is a delimiter left where it changes the drawing.
    out.answers_every_operator &= out.unread.is_empty() && !out.delimiter_left;
    Ok(out)
}

/// What `xmpMM:History` says of the runs written: the requirement, the count and the loss, then
/// each run by its pages, its bytes and the number written, the first [`MOST_NAMED`] by name.
///
/// A run's bytes are the producer's and need not be text, so each byte outside printable ASCII is
/// written as `\xHH`.
pub(super) fn history(drawn: &Drawn, citation: &str) -> String {
    let mut out = format!(
        "{citation}: {} run(s) the base standard spells as no number, or lone delimiters it reads \
         as no operator, were written as this reader draws them, a run as its number and a \
         delimiter as white space; loss {}",
        drawn.runs.len(),
        super::Loss::ContentAsDrawn.word()
    );
    for (index, written) in drawn.runs.iter().take(MOST_NAMED).enumerate() {
        let pages: Vec<String> = written.pages.iter().map(usize::to_string).collect();
        // Writing into a `String` cannot fail, so the `write!` result is discarded deliberately.
        let _ = write!(
            out,
            "{} page {} object {} {}: {} as {}",
            if index == 0 { ": " } else { "; " },
            if pages.is_empty() {
                "?".to_owned()
            } else {
                pages.join(",")
            },
            written.object.number,
            written.object.generation,
            spelled(&written.run),
            written_as(&written.number)
        );
    }
    if drawn.runs.len() > MOST_NAMED {
        let _ = write!(
            out,
            ", and {} more",
            drawn.runs.len().saturating_sub(MOST_NAMED)
        );
    }
    out
}

/// What was written in a run's place, for a person: the number, or that a lone delimiter was
/// written as white space.
pub(super) fn written_as(number: &[u8]) -> String {
    if number.is_empty() {
        "white space".to_owned()
    } else {
        spelled(number)
    }
}

/// A run's bytes as text, each byte outside printable ASCII written as `\xHH`.
pub(super) fn spelled(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for byte in bytes {
        if byte.is_ascii_graphic() {
            out.push(char::from(*byte));
        } else {
            let _ = write!(out, "\\x{byte:02X}");
        }
    }
    out
}

/// The runs written, unless a stream they are in is one `super::languages` also respells: that
/// rewrite writes the stream from the producer's bytes, so carrying both would drop one, and the
/// stream is refused rather than written twice.
pub(super) fn apart_from(
    drawn: Drawn,
    languages: Option<&super::languages::Restated>,
) -> Result<Drawn, Because> {
    match languages {
        Some(restated) if drawn.at.keys().any(|id| restated.streams.contains_key(id)) => {
            Err(Because::NotBuiltYet(ALSO_A_LANGUAGE_RESPELLED))
        }
        _ => Ok(drawn),
    }
}

/// Whether `because` is [`write()`]'s refusal of a document in which no content stream holds a
/// run this reader draws as a number, so that the operator row has nothing this remedy answers.
pub(super) fn reached_no_run(because: Because) -> bool {
    because == Because::NotBuiltYet(NO_STREAM_HOLDS_ONE)
}

/// Whether a keyword the checker reports is a run this reader draws as a number, read alone.
///
/// The survey reports each operator by its spelling, and a run lexes the same wherever it
/// stands, because §7.2.3 ends a token only at white space or a delimiter.
pub(super) fn is_drawn_as_a_number(spelling: &[u8]) -> bool {
    let mut lexer = Lexer::new(spelling);
    lexer.next_token().is_some()
        && lexer.position() == spelling.len()
        && lexer.salvaged().is_some_and(draws_the_salvage)
}

/// Whether a keyword the checker reports is a delimiter that closes nothing, which this tree's
/// lexer hands over as a keyword of its own (§7.2.3's `>` and `)`).
pub(super) fn is_a_lone_delimiter(spelling: &[u8]) -> bool {
    matches!(spelling, b">" | b")")
}

/// A run's span in one decoded stream, and the number written for it.
struct Place {
    /// The run's first byte.
    start: usize,
    /// One past its last byte.
    end: usize,
    /// The number the reader draws, in §7.3.3's spelling; empty for a lone delimiter.
    number: Vec<u8>,
}

/// Every run in one decoded content stream that this reader draws as a number.
///
/// Read by the same lexer the reader reads with, the salvage judged by the reader's own rule
/// ([`draws_the_salvage`]), and §8.9.7's inline image data passed over as `super::hexadecimal`
/// passes it. Each run is at least two bytes — a digit and what the number leaves out — so the
/// list is bounded by half the stream's length, and by `most`: `None` where the stream holds more
/// than `most` runs.
///
/// A lone delimiter is a place too, with no number, where `delimiters` is set and nothing stands
/// before it since the last operator and it is in no array or dictionary — the one place the
/// reader's clearing of the operands clears nothing. The flag beside the list says whether one was
/// left.
fn salvaged_runs(
    document: &Document,
    data: &[u8],
    resources: &pdf_syntax::object::Dictionary,
    most: usize,
    delimiters: bool,
) -> Option<(Vec<Place>, bool)> {
    let mut lexer = Lexer::new(data);
    let mut out = Vec::new();
    let mut left = false;
    // The operands since the last operator, and how deep in arrays and dictionaries the lexer is.
    let mut pending = 0_usize;
    let mut depth = 0_usize;
    loop {
        lexer.skip_whitespace();
        let start = lexer.position();
        let Some(token) = lexer.next_token() else {
            break;
        };
        if matches!(token, Token::Keyword(b"BI")) {
            let scan =
                pdf_model::inline_image::scan(document, data, lexer.position(), resources, true);
            lexer.seek(scan.resume);
            pending = 0;
            continue;
        }
        match token {
            Token::ArrayOpen | Token::DictOpen => depth = depth.saturating_add(1),
            Token::ArrayClose | Token::DictClose => {
                depth = depth.saturating_sub(1);
                pending = pending.saturating_add(1);
            }
            Token::Keyword(word) if is_a_lone_delimiter(word) => {
                if delimiters && depth == 0 && pending == 0 {
                    if out.len() == most {
                        return None;
                    }
                    out.push(Place {
                        start,
                        end: lexer.position(),
                        number: Vec::new(),
                    });
                } else {
                    left = true;
                    pending = 0;
                }
                continue;
            }
            Token::Keyword(b"true" | b"false" | b"null") => pending = pending.saturating_add(1),
            Token::Keyword(_) if depth == 0 && lexer.salvaged().is_none() => pending = 0,
            _ => pending = pending.saturating_add(1),
        }
        let Some(salvage) = lexer.salvaged() else {
            continue;
        };
        if !draws_the_salvage(salvage) {
            continue;
        }
        let value = match token {
            Token::Integer(value) => Object::Integer(value),
            Token::Real(value) => Object::Real(value),
            _ => continue,
        };
        let mut number = Vec::new();
        // §7.3.3: "A PDF writer shall not use the PostScript language syntax for numbers with
        // non-decimal radices (such as 16#FFFE) or in exponential format (such as 6.02E23)."
        // `write::object` writes the shortest decimal that reads back as the same value, which
        // never uses an exponent.
        if out.len() == most {
            return None;
        }
        pdf_syntax::write::object(&value, &mut number);
        out.push(Place {
            start,
            end: lexer.position(),
            number,
        });
    }
    Some((out, left))
}

#[cfg(test)]
mod tests {
    use super::is_drawn_as_a_number;

    #[test]
    fn a_run_whose_tail_names_no_operator_is_drawn_as_a_number_and_one_that_swallowed_a_fill_is_not()
     {
        assert!(is_drawn_as_a_number(b"33600.272.58"));
        assert!(is_drawn_as_a_number(b"3NT-DE290.58"));
        assert!(is_drawn_as_a_number(b"454\x85"));
        assert!(is_drawn_as_a_number(b"12pt"));
        // `f` is §8.2 Table 50's fill: the reader hands the run over whole and draws no number.
        assert!(!is_drawn_as_a_number(b"5f"));
        // §7.3.3 spells these whole, and a run the grammar reads nothing of is a keyword.
        assert!(!is_drawn_as_a_number(b"33600.272"));
        assert!(!is_drawn_as_a_number(b"--5"));
        assert!(!is_drawn_as_a_number(b"Frobnicate"));
    }
}
