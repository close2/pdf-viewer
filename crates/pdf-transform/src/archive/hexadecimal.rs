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
//! respelling because "\[i]t changes a byte in a content stream and cannot change a mark" — and
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
    /// How many of `digits` each stream of `at` holds.
    written_at: BTreeMap<ObjectId, usize>,
    /// How many digits [`Self::leaving_out`] took back, with the streams that held them.
    left_out: usize,
}

impl Completed {
    /// Leaves out every stream of `taken_off`, the streams [`taken_off_by`] a photograph.
    ///
    /// A photographed page's `/Contents` is the photograph's own stream (`super::fonts`), so the
    /// stream it named is no longer a content stream of the output, and any other object that
    /// names it reads it as whatever else it is. A producer that reused an object number for its
    /// catalog's metadata stream while an older page still names it as its contents is the case
    /// in hand: writing §7.3.4.3's digit there would write `0` before every `>` of an XMP packet,
    /// and the packet writers' edits, which reach the same object later, would be lost with it.
    pub(super) fn leaving_out(mut self, taken_off: &BTreeSet<ObjectId>) -> Self {
        for id in taken_off {
            if self.at.remove(id).is_some() {
                let written = self.written_at.remove(id).unwrap_or(0);
                self.digits = self.digits.saturating_sub(written);
                self.left_out = self.left_out.saturating_add(written);
            }
        }
        self
    }

    /// Whether every place the repair found was on a page a photograph took off, so that the
    /// photograph rather than the repair is what answers ISO 19005's row.
    pub(super) fn only_on_photographed_pages(&self) -> bool {
        self.at.is_empty() && self.left_out > 0
    }
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
        out.written_at.insert(id, places.len());
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
pub(super) fn streams(
    document: &Document,
    pages: &Pages<'_>,
    survey: Option<&Survey>,
) -> Vec<(ObjectId, Dictionary)> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for page in pages.every().into_iter().flatten() {
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

/// The streams a photograph of each page of `photographed` takes off the page: those that are
/// some page's `/Contents`, are named so by photographed pages only, and are nothing the survey
/// opened as a resource.
///
/// What is left is a stream the output reaches, if at all, as something other than a content
/// stream, which is why a content stream's rewrite has nothing to write there.
pub(super) fn taken_off_by(
    document: &Document,
    survey: Option<&Survey>,
    photographed: &BTreeSet<ObjectId>,
) -> BTreeSet<ObjectId> {
    if photographed.is_empty() {
        return BTreeSet::new();
    }
    let mut kept: BTreeSet<ObjectId> = survey
        .map(Survey::opened_streams)
        .unwrap_or_default()
        .iter()
        .filter_map(|opened| opened.object)
        .collect();
    let mut taken = BTreeSet::new();
    let pages = Pages::new(document);
    for page in pages.every().into_iter().flatten() {
        // A page this walk cannot name by its object is one no photograph replaced.
        let into = if page.id.is_some_and(|id| photographed.contains(&id)) {
            &mut taken
        } else {
            &mut kept
        };
        into.extend(contents_of(document, &page.dict));
    }
    taken.difference(&kept).copied().collect()
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
pub(super) fn repaired(stream: &Stream, data: &[u8]) -> Option<Object> {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Two pages, objects 3 and 4; object 6 is the catalog's `/Metadata` and the `/Contents` of
    /// page 4, and of page 3 too where `shared`. Both content streams omit §7.3.4.3's final digit.
    fn fixture(shared: bool) -> Document {
        let page_three = if shared { "[5 0 R 6 0 R]" } else { "5 0 R" };
        let text = format!(
            "%PDF-1.7\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R /Metadata 6 0 R >>\nendobj\n\
             2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>\nendobj\n\
             3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents {page_three} >>\nendobj\n\
             4 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 6 0 R >>\nendobj\n\
             5 0 obj\n<< /Length 9 >>\nstream\n<414> Tj\nendstream\nendobj\n\
             6 0 obj\n<< /Length 11 >>\nstream\n<41424> Tj\nendstream\nendobj\n\
             trailer\n<< /Root 1 0 R /Size 7 >>\n%%EOF\n"
        );
        Document::open(text.into_bytes()).expect("the fixture is a well-formed file")
    }

    fn page(number: u32) -> ObjectId {
        ObjectId {
            number,
            generation: 0,
        }
    }

    /// A stream only a photographed page names as its contents is left out with its digits, and
    /// the repair of the stream an unphotographed page names stays.
    #[test]
    fn a_stream_only_photographed_pages_name_is_left_out() {
        let document = fixture(false);
        let completed = complete(&document, None).expect("both streams omit a digit");
        assert_eq!(completed.digits, 2);
        let taken_off = taken_off_by(&document, None, &BTreeSet::from([page(4)]));
        assert_eq!(taken_off, BTreeSet::from([page(6)]));
        let left = completed.leaving_out(&taken_off);
        assert_eq!(left.at.keys().copied().collect::<Vec<_>>(), vec![page(5)]);
        assert_eq!(left.digits, 1);
        assert!(!left.only_on_photographed_pages());
    }

    /// Where every page is photographed, nothing is left for the repair to write, and ISO
    /// 19005's row is the photographs' to answer.
    #[test]
    fn every_page_photographed_leaves_nothing_to_repair() {
        let document = fixture(false);
        let completed = complete(&document, None).expect("both streams omit a digit");
        let taken_off = taken_off_by(&document, None, &BTreeSet::from([page(3), page(4)]));
        let left = completed.leaving_out(&taken_off);
        assert!(left.at.is_empty());
        assert_eq!(left.digits, 0);
        assert!(left.only_on_photographed_pages());
    }

    /// A stream an unphotographed page still draws is a content stream of the output, so its
    /// repair is kept whatever else names it.
    #[test]
    fn a_stream_an_unphotographed_page_names_is_kept() {
        let document = fixture(true);
        let taken_off = taken_off_by(&document, None, &BTreeSet::from([page(4)]));
        assert!(taken_off.is_empty());
        let completed = complete(&document, None).expect("both streams omit a digit");
        let left = completed.leaving_out(&taken_off);
        assert_eq!(left.digits, 2);
        assert!(left.at.contains_key(&page(6)));
    }

    /// No page photographed takes nothing off, without walking the page tree.
    #[test]
    fn no_photograph_takes_nothing_off() {
        assert!(taken_off_by(&fixture(false), None, &BTreeSet::new()).is_empty());
    }
}
