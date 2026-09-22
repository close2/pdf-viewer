//! ISO 19005-2 section 6.1.6 and ISO 19005-4 section 6.1.5, inside a content stream: the final
//! digit the base standard already assumed, written down.
//!
//! # Why this is not editing the producer's page
//!
//! §7.3.4.3 states the value of an odd-digit hexadecimal string outright:
//!
//! > If the final digit of a hexadecimal string is missing -that is, if there is an odd number of
//! > digits -the final digit shall be assumed to be 0.
//!
//! and its EXAMPLE 2 spells the consequence: `<901FA>` *is* the three bytes 90 1F A0. So the byte
//! this writes is one every conforming reader already supplies, the operand's value does not
//! change, and no operator is added, removed or respelled. `doc/pdf-a-conversion-limits.md`
//! section 5.2 is where that line was already drawn — the owner's `A50` admits the `F`-to-`f`
//! respelling because "[i]t changes a byte in a content stream and cannot change a mark" — and
//! `CLAUDE.md`'s fourth amendment makes mark *provenance* the test the fence applies. ADR 1176 is
//! the argument, and ADR 0947 decision 3 is what it supersedes.
//!
//! **The sibling row stays refused, and the difference is the point.** A hexadecimal string
//! holding a byte that is neither a digit nor white space
//! (`file-structure/hexadecimal-string-holds-only-digits`) has no value the standard states:
//! §7.3.4.3 gives that byte no meaning at all, so deleting it is a guess about what the producer
//! meant rather than a transcription of what the standard already decided.
//!
//! # One stream at a time, which is the standard's own division
//!
//! §7.7.3.3's Table 31 makes a page's `/Contents` array behave as one concatenated stream and
//! then bounds where the seams may fall:
//!
//! > The division between streams may occur only at the boundaries between lexical tokens (see
//! > 7.2, "Lexical conventions" ) but shall be unrelated to the page's logical content or
//! > organisation.
//!
//! So a hexadecimal string never straddles two elements of a conforming array, and this reads each
//! stream object alone. A file that straddles one anyway leaves a token this walk does not
//! complete — the `<` has no `>` in the same object — and nothing is written for it: the
//! requirement stays failed and ADR 0947's third stage refuses the file rather than a wrong byte
//! being put in it.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use pdf_archive::survey::Survey;
use pdf_model::Pages;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::serialize::flate_encode;
use pdf_syntax::{Document, Lexer, Token};

use super::COMPRESSION_LEVEL;
use super::decision::Because;

/// Why a stream whose bytes cannot be written back is not repaired.
const STREAM_WILL_NOT_RE_ENCODE: &str = "a content stream stating a hexadecimal string with an \
     odd number of digits cannot be written back: it will not re-encode, or its /DecodeParms \
     names another object that discarding the parameters would orphan. So the digit ISO 32000-2 \
     \u{a7}7.3.4.3 already assumes cannot be written into it";

/// Why a document stating one nowhere this walk reaches is refused rather than passed.
const NO_STREAM_TO_REPAIR: &str = "this document states a hexadecimal string with an odd number \
     of digits in a content stream, and no content stream this conversion reads states one \
     \u{2014} so the string is in bytes reached some other way, and writing the digit into a \
     stream that does not hold it would fix nothing";

/// The content streams this conversion rewrites, and how many digits it writes.
#[derive(Debug, Default)]
pub(super) struct Completed {
    /// The stream each object is to carry.
    pub(super) at: BTreeMap<ObjectId, Object>,
    /// How many final digits were written, over every stream.
    pub(super) digits: usize,
}

/// Writes §7.3.4.3's assumed final digit into every content stream that omitted one.
///
/// `survey` is the walk that already found the content streams a resource dictionary reaches; a
/// page's own `/Contents` is not one of its objects — §7.7.3.3 lets it be an array — so the page
/// tree supplies those.
pub(super) fn complete(document: &Document, survey: Option<&Survey>) -> Result<Completed, Because> {
    let pages = Pages::new(document);
    let mut out = Completed::default();
    for (id, resources) in streams(document, &pages, survey) {
        let object = document.get(id);
        let Some(stream) = object.as_stream() else {
            continue;
        };
        let Some(data) = document.decoded_stream_data(stream) else {
            continue;
        };
        let places = missing_digits(document, &data, &resources);
        if places.is_empty() {
            continue;
        }
        let mut written = Vec::with_capacity(data.len().saturating_add(places.len()));
        let mut from = 0;
        for place in &places {
            written.extend_from_slice(data.get(from..*place).unwrap_or_default());
            written.push(b'0');
            from = *place;
        }
        written.extend_from_slice(data.get(from..).unwrap_or_default());
        let replacement =
            repaired(stream, &written).ok_or(Because::NotBuiltYet(STREAM_WILL_NOT_RE_ENCODE))?;
        out.digits = out.digits.saturating_add(places.len());
        out.at.insert(id, replacement);
    }
    if out.at.is_empty() {
        return Err(Because::NotBuiltYet(NO_STREAM_TO_REPAIR));
    }
    Ok(out)
}

/// Every content stream object, with the resource dictionary in force inside it.
///
/// The resources are wanted for one thing only: §8.9.7 lets an inline image's `/CS` name a colour
/// space the resources define, and the component count is what says where its data ends.
fn streams(
    document: &Document,
    pages: &Pages<'_>,
    survey: Option<&Survey>,
) -> Vec<(ObjectId, Dictionary)> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        let resources = document
            .get_key(&page.dict, "Resources")
            .as_dict()
            .cloned()
            .unwrap_or_default();
        for entry in contents_of(document, &page.dict) {
            if seen.insert(entry) {
                out.push((entry, resources.clone()));
            }
        }
    }
    for opened in survey.map(Survey::opened_streams).unwrap_or_default() {
        let Some(id) = opened.object else {
            continue;
        };
        if !seen.insert(id) {
            continue;
        }
        let object = document.get(id);
        let resources = object
            .as_stream()
            .map(|stream| document.get_key(&stream.dict, "Resources"))
            .and_then(|value| value.as_dict().cloned())
            .or_else(|| {
                pages
                    .get(opened.page)
                    .and_then(|page| document.get_key(&page.dict, "Resources").as_dict().cloned())
            })
            .unwrap_or_default();
        out.push((id, resources));
    }
    out
}

/// The objects one page's `/Contents` names, which §7.7.3.3 lets be one stream or an array.
fn contents_of(document: &Document, page: &Dictionary) -> Vec<ObjectId> {
    match page.get("Contents") {
        Some(Object::Reference(id)) => match document.get(*id) {
            Object::Array(items) => items.iter().filter_map(Object::as_reference).collect(),
            _ => vec![*id],
        },
        Some(Object::Array(items)) => items.iter().filter_map(Object::as_reference).collect(),
        _ => Vec::new(),
    }
}

/// Where a final digit is missing in one decoded content stream: the offset of each `>`.
///
/// **The lexer decides which `<` opens a string**, which is why this reads tokens rather than
/// bytes: a second reader would have to re-derive that, and `pdf_syntax::HexadecimalStrings` says
/// what happens when it does. What this adds is the token's span, which the lexer's own position
/// gives on either side of the read.
fn missing_digits(document: &Document, data: &[u8], resources: &Dictionary) -> Vec<usize> {
    let mut lexer = Lexer::new(data);
    let mut out = Vec::new();
    loop {
        lexer.skip_whitespace();
        let start = lexer.position();
        let Some(token) = lexer.next_token() else {
            break;
        };
        if matches!(token, Token::Keyword(b"BI")) {
            // §8.9.7's inline image data is not a program: compressed samples hold angle brackets
            // like any other byte, and reading them as syntax would invent a string out of an
            // image.
            let scan =
                pdf_model::inline_image::scan(document, data, lexer.position(), resources, true);
            lexer.seek(scan.resume);
            continue;
        }
        if !matches!(token, Token::String(_)) || data.get(start) != Some(&b'<') {
            continue;
        }
        let Some(close) = lexer.position().checked_sub(1) else {
            continue;
        };
        // A `<` the object never closes is a token this walk does not complete, so nothing is
        // written for it.
        if data.get(close) != Some(&b'>') {
            continue;
        }
        let digits = data
            .get(start.saturating_add(1)..close)
            .unwrap_or_default()
            .iter()
            .filter(|byte| byte.is_ascii_hexdigit())
            .count();
        if !digits.is_multiple_of(2) {
            out.push(close);
        }
    }
    out
}

/// One content stream's replacement, written as §7.4.4's single `FlateDecode`.
///
/// The repaired bytes are the whole decoded stream, so the filter chain that produced them is
/// spent and one that produces them again takes its place. §7.4.4's NOTE 1 is why that is
/// not a cost: Flate-encoded output "is usually much more compact" than what most producers
/// wrote. `None` where the encoding fails, or where the parameters being discarded name another
/// object — discarding those would leave it in the file with nothing referring to it.
fn repaired(stream: &Stream, data: &[u8]) -> Option<Object> {
    if stream
        .dict
        .get("DecodeParms")
        .is_some_and(|value| holds_reference(value, 0))
    {
        return None;
    }
    let encoded = flate_encode(data, COMPRESSION_LEVEL)?;
    let mut dict = stream.dict.clone();
    dict.insert(
        Name::new(&b"Filter"[..]),
        Object::Name(Name::new(&b"FlateDecode"[..])),
    );
    dict.remove("DecodeParms");
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(encoded.len()).unwrap_or(i64::MAX)),
    );
    Some(Object::Stream(Arc::new(Stream {
        dict,
        data: encoded.into(),
        decryption_failed: false,
    })))
}

/// How deep a `/DecodeParms` value is searched for a reference before it is treated as holding
/// one.
const MAX_PARMS_DEPTH: usize = 16;

/// Whether a value names another object anywhere inside it.
fn holds_reference(value: &Object, depth: usize) -> bool {
    if depth >= MAX_PARMS_DEPTH {
        return true;
    }
    let deeper = depth.saturating_add(1);
    match value {
        Object::Reference(_) => true,
        Object::Array(items) => items.iter().any(|item| holds_reference(item, deeper)),
        Object::Dictionary(dict) => dict.iter().any(|(_, item)| holds_reference(item, deeper)),
        _ => false,
    }
}
