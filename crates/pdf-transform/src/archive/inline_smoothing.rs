//! ISO 19005-2 section 6.2.8.1 and ISO 19005-4 section 6.2.7.1, the inline image half: an inline
//! image's `/I` (or `/Interpolate`) written false inside its `BI` dictionary, under the loss
//! `image-smoothing` (`doc/questions/A978`, `doc/adr/3373`).
//!
//! # Why this is the image `XObject`'s rewrite, written where the key is
//!
//! ISO 32000-2 §8.9.5.3 says of the entry the two parts require to be false:
//!
//! > However, this is only a hint, and a PDF processor may ignore it.
//!
//! and §8.9.7's Table 91 abbreviates the same key `I` for an inline image. So the edit is
//! `super::smoothing`'s, R02's, made in the one place this key can be: the dictionary between
//! `BI` and `ID`, inside the producer's content stream. The owner's `A978` admits it as the second
//! closed exception beside `A50`'s operator spellings, on the condition that nothing else in the
//! stream moves. This writes the value token `false` over whatever value the key states and
//! touches no other byte: not another key, not the `ID`, not one sample of the image's data, and
//! nothing outside the `BI`…`ID` span. The tests below hold it to that byte for byte.
//!
//! **A stream another content repair writes is refused rather than written twice**, as
//! `super::as_drawn` and `super::type3_widths` refuse it: the repairs are not composed.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use pdf_archive::survey::Survey;
use pdf_model::Pages;
use pdf_syntax::object::{Object, ObjectId};
use pdf_syntax::{Document, Lexer, Parser, Token};

use super::decision::{Because, Loss};
use super::hexadecimal;

/// The requirement this answers.
pub(super) const SITE: &str = "graphics/inline-image-interpolation-is-off";

/// The value written over the producer's.
const FALSE: &[u8] = b"false";

/// How many streams the history entry names before it counts the rest.
const MOST_NAMED: usize = 20;

/// Why no stream was written although the requirement failed.
const NO_STREAM_HOLDS_ONE: &str = "no content stream this conversion reads states an inline \
     image whose I entry is other than false, so the image that fails ISO 19005's interpolation \
     rule is in bytes reached some other way, and writing false into a stream that does not hold \
     it would fix nothing";

/// Why a stream whose bytes cannot be written back is not edited.
const STREAM_WILL_NOT_RE_ENCODE: &str = "a content stream stating an inline image's I entry \
     cannot be written back: it will not re-encode, or its /DecodeParms names another object \
     that discarding the parameters would orphan";

/// Why a stream another content repair also writes is not edited.
const ALSO_ANOTHER_REPAIR: &str = "a content stream whose inline image's I entry is to be written \
     false is also written by another repair of its content, and the two are not composed";

/// The content streams written with each inline image's `/I` stated false.
#[derive(Debug, Default)]
pub(super) struct Unsmoothed {
    /// The stream each object is to carry.
    pub(super) at: BTreeMap<ObjectId, Object>,
    /// How many entries each stream of `at` had written false.
    written_at: BTreeMap<ObjectId, usize>,
}

impl Unsmoothed {
    /// How many entries were written false, over every stream.
    pub(super) fn entries(&self) -> usize {
        self.written_at.values().sum()
    }
}

/// One edit: the span of a value token inside a `BI` dictionary, which `false` replaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Edit {
    start: usize,
    end: usize,
}

/// Writes `false` as the value of every inline image's interpolation entry the content streams
/// state other than false, in every stream the page tree and the survey reach.
pub(super) fn write(
    document: &Document,
    survey: Option<&Survey>,
    others: &[&BTreeMap<ObjectId, Object>],
) -> Result<Unsmoothed, Because> {
    let pages = Pages::new(document);
    let mut out = Unsmoothed::default();
    for (id, resources) in hexadecimal::streams(document, &pages, survey) {
        let object = document.get(id);
        let Some(stream) = object.as_stream() else {
            continue;
        };
        let Some(data) = document.decoded_stream_data(stream) else {
            continue;
        };
        let edits = interpolating_entries(document, &data, &resources);
        if edits.is_empty() {
            continue;
        }
        if others.iter().any(|other| other.contains_key(&id)) {
            return Err(Because::NotBuiltYet(ALSO_ANOTHER_REPAIR));
        }
        let written = applied(&data, &edits);
        let replacement = hexadecimal::repaired(stream, &written)
            .ok_or(Because::NotBuiltYet(STREAM_WILL_NOT_RE_ENCODE))?;
        out.written_at.insert(id, edits.len());
        out.at.insert(id, replacement);
    }
    if out.at.is_empty() {
        return Err(Because::NotBuiltYet(NO_STREAM_HOLDS_ONE));
    }
    Ok(out)
}

/// What `xmpMM:History` says of the entries written: the requirement, the count and the loss, then
/// the streams that held them, the first [`MOST_NAMED`] by reference.
pub(super) fn history(unsmoothed: &Unsmoothed, citation: &str) -> String {
    let mut out = format!(
        "{citation}: {} inline image(s) had /I set to false inside their content stream; loss {}",
        unsmoothed.entries(),
        Loss::ImageSmoothing.word()
    );
    for (index, (id, count)) in unsmoothed.written_at.iter().take(MOST_NAMED).enumerate() {
        let lead = if index == 0 { ": stream" } else { ", stream" };
        // Writing into a `String` cannot fail, so the `write!` result is discarded deliberately.
        let _ = write!(out, "{lead} {} {} R ({count})", id.number, id.generation);
    }
    if unsmoothed.written_at.len() > MOST_NAMED {
        let _ = write!(
            out,
            ", and {} more stream(s)",
            unsmoothed.written_at.len().saturating_sub(MOST_NAMED)
        );
    }
    out
}

/// `data` with `false` written over each edit's span, every other byte as it was.
fn applied(data: &[u8], edits: &[Edit]) -> Vec<u8> {
    let mut written = Vec::with_capacity(data.len().saturating_add(edits.len()));
    let mut from = 0;
    for edit in edits {
        written.extend_from_slice(data.get(from..edit.start).unwrap_or_default());
        written.extend_from_slice(FALSE);
        from = edit.end;
    }
    written.extend_from_slice(data.get(from..).unwrap_or_default());
    written
}

/// The value spans of every `/I` or `/Interpolate` entry, in one decoded content stream, whose
/// value is other than `false`.
///
/// The lexer finds each `BI`, and the pairs up to `ID` are read as `pdf_model::inline_image`
/// reads them — a name, then one object — so that a name in a value's place (`/CS /I`, Table 91's
/// abbreviation of `Indexed`) is never taken for the key. Where the image data begins and ends is
/// `pdf_model::inline_image::scan`'s answer, so no sample is read as syntax.
fn interpolating_entries(
    document: &Document,
    data: &[u8],
    resources: &pdf_syntax::Dictionary,
) -> Vec<Edit> {
    let mut lexer = Lexer::new(data);
    let mut out = Vec::new();
    loop {
        lexer.skip_whitespace();
        let Some(token) = lexer.next_token() else {
            break;
        };
        if !matches!(token, Token::Keyword(b"BI")) {
            continue;
        }
        let after_bi = lexer.position();
        out.extend(dictionary_edits(document, data, after_bi));
        let scan = pdf_model::inline_image::scan(document, data, after_bi, resources, true);
        lexer.seek(scan.resume);
    }
    out
}

/// The edits one `BI` dictionary asks for, read from just after its `BI`.
///
/// A dictionary this cannot read to its `ID` yields nothing: the image is then one the reader
/// draws nothing for, the requirement stays failed, and the conversion refuses at its last stage
/// rather than writing into bytes whose shape is not known.
fn dictionary_edits(document: &Document, data: &[u8], at: usize) -> Vec<Edit> {
    let mut lexer = Lexer::at(data, at);
    let mut out = Vec::new();
    loop {
        let Some(token) = lexer.next_token() else {
            return Vec::new();
        };
        let key = match token {
            Token::Keyword(b"ID") => return out,
            Token::Name(key) => key,
            _ => return Vec::new(),
        };
        lexer.skip_whitespace();
        let start = lexer.position();
        let mut parser = Parser::at(data, start, document.limits());
        let Ok(value) = parser.parse_object() else {
            return Vec::new();
        };
        let end = parser.position();
        lexer.seek(end);
        let interpolation = key.as_slice() == b"I" || key.as_slice() == b"Interpolate";
        if interpolation && value != Object::Boolean(false) {
            out.push(Edit { start, end });
        }
    }
}

#[cfg(test)]
mod tests {
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

    /// The span from `BI` to the end of `ID`, which is the only place an edit may fall.
    fn bi_to_id(data: &[u8]) -> (usize, usize) {
        let bi = data.windows(2).position(|w| w == b"BI").expect("a BI");
        let id = data.windows(2).position(|w| w == b"ID").expect("an ID");
        (bi, id.saturating_add(2))
    }

    #[test]
    fn i_true_is_written_false_and_no_byte_outside_bi_to_id_moves() {
        // The samples hold `/I true` themselves, so a reader that took them for syntax would edit
        // the image: §8.9.7's data is not a program.
        let data: &[u8] = b"q 1 0 0 1 0 0 cm\nBI /W 8 /H 1 /BPC 8 /CS /G /I true ID \
                            /I true EI Q\n0 0 m (I true) Tj";
        let document = document();
        let edits = interpolating_entries(&document, data, &pdf_syntax::Dictionary::new());
        assert_eq!(
            edits.len(),
            1,
            "one entry, not the samples' or the string's"
        );
        let written = applied(data, &edits);
        let (start, end) = bi_to_id(data);
        let (_, written_end) = bi_to_id(&written);
        assert_eq!(written.get(..start), data.get(..start), "before BI");
        assert_eq!(
            written.get(written_end..),
            data.get(end..),
            "from ID on, the samples included"
        );
        assert_eq!(
            written.get(start..written_end),
            Some(&b"BI /W 8 /H 1 /BPC 8 /CS /G /I false ID"[..])
        );
    }

    #[test]
    fn the_full_spelling_is_written_and_a_value_named_i_is_not() {
        let data: &[u8] = b"BI /W 1 /H 1 /BPC 8 /CS /I /Interpolate true /D [0 1] ID \x00 EI";
        let edits = interpolating_entries(&document(), data, &pdf_syntax::Dictionary::new());
        let written = applied(data, &edits);
        assert_eq!(
            written,
            b"BI /W 1 /H 1 /BPC 8 /CS /I /Interpolate false /D [0 1] ID \x00 EI"
        );
    }

    #[test]
    fn an_entry_already_false_is_left_alone() {
        let data: &[u8] = b"BI /W 1 /H 1 /BPC 8 /CS /G /I false ID \x00 EI";
        assert_eq!(
            interpolating_entries(&document(), data, &pdf_syntax::Dictionary::new()),
            Vec::new()
        );
    }

    #[test]
    fn a_dictionary_that_does_not_read_to_its_id_is_not_written() {
        let data: &[u8] = b"BI /W 1 /I true 7 ID \x00 EI";
        assert_eq!(
            interpolating_entries(&document(), data, &pdf_syntax::Dictionary::new()),
            Vec::new()
        );
    }
}
