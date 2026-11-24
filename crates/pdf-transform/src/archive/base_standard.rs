//! ISO 19005-2 and ISO 19005-4 section 5.1 over the reader's log of repairs: what the rewrite
//! answers, what this conversion restates at a stated loss, and what it refuses by name (ADR 3521).
//!
//! # Three answers, by what the reader repaired
//!
//! `pdf_archive` fails section 5.1 at every repair `pdf_syntax` and the content survey logged on the
//! source (ADR 3506). Which answer a repair takes depends on where its bytes go:
//!
//! - **A repair of the file's structure or of an object outside content** — a table rebuilt, a
//!   key repeated, an `endobj` missing, a number spelled with an exponent in a dictionary — is
//!   answered by the whole-file rewrite: the output is the repaired reading written in the
//!   serializer's own syntax, so the departure is not in it.
//! - **A repair inside a content stream** is refused by name. The stream crosses this verb byte
//!   for byte, so the departure would be in the output too, and correcting it would edit the
//!   producer's page — ADR 0816's fence. The survey records these at [`pdf_syntax::Site::Content`],
//!   which is how they are told from the same class met in a dictionary.
//! - **Stream data the reader decoded only in part** (`syntax/stream-data-damaged`) is written as
//!   the bytes this reader decoded, re-encoded as one `FlateDecode`, at the loss
//!   [`super::Loss::DamagedDataAsDecoded`] — ADR 3473's construction for ASCII data cut before its
//!   marker, which this leaves to that row where it is the whole of a stream's damage. A
//!   `/DecodeParms` that does not pair with its `/Filter` is answered where a restatement
//!   re-encodes the stream anyway, and refused by name elsewhere.
//! - **Data no stage decodes at all** (`syntax/stream-data-undecodable`) has no bytes to write, and
//!   whether the output still carries the stream is another rewrite's to say — a metadata packet
//!   written afresh, a font program substituted, a page photographed. So it is left to the output's
//!   own check, whose refusal names each class the output's log still holds (ADR 3521).
//!
//! The log places stream damage at the whole file, because the decoder is handed data without its
//! object, so the streams are found here by decoding each one the output carries again.

use std::collections::{BTreeMap, BTreeSet};

use pdf_archive::Target;
use pdf_archive::repairs::BASE_STANDARD;
use pdf_syntax::object::{Object, ObjectId, Stream};
use pdf_syntax::{Damage, Document, RepairClass, Site, repair};

use super::decision::Because;
use super::rewrite::{Cut, reencoded_through_flate};

/// One stream written as the bytes this reader decoded, as the report names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DamagedDataRestated {
    /// The stream, by object number.
    pub object: u32,
    /// Its generation.
    pub generation: u16,
    /// The first stage that reported damage, by the name the stream states it under.
    pub stage: String,
    /// What that stage reported, in a few words.
    pub damage: &'static str,
    /// How many bytes of stream data the source held.
    pub encoded: usize,
}

impl DamagedDataRestated {
    /// The report's sentence for this stream.
    #[must_use]
    pub fn sentence(&self) -> String {
        format!(
            "the stream object {} {} ({} bytes of stream data, its {} stage {}) is written as \
             the bytes this viewer decoded from it (damaged-data-as-decoded)",
            self.object, self.generation, self.encoded, self.stage, self.damage
        )
    }
}

/// How section 5.1 is answered for one document: by the rewrite alone where `at` is empty, and
/// otherwise by writing these streams as decoded too.
#[derive(Debug, Default)]
pub(super) struct Answered {
    /// Each damaged stream's replacement, in the source's numbering.
    pub(super) at: BTreeMap<ObjectId, Object>,
    /// What the report names, one entry per stream.
    pub(super) streams: Vec<DamagedDataRestated>,
    /// Each class of repair made inside a content stream, with the page or form whose content it
    /// was; [`settled`] answers or refuses them once the content rewrites are known.
    content: Vec<(&'static RepairClass, Option<ObjectId>)>,
}

impl Answered {
    /// Whether the rewrite alone answers the row, so that nothing is lost.
    pub(super) fn costs_nothing(&self) -> bool {
        self.at.is_empty()
    }

    /// The `xmpMM:History` parameters naming every stream restated, the report's sentences
    /// joined; `None` where none is (ADR 3494's rule, for this loss).
    pub(super) fn history(&self) -> Option<String> {
        (!self.streams.is_empty()).then(|| {
            let sentences: Vec<String> = self
                .streams
                .iter()
                .map(DamagedDataRestated::sentence)
                .collect();
            sentences.join("; ")
        })
    }
}

/// Why no stream is restated where the file's own history cannot name it.
pub(super) const NO_PLACE_TO_RECORD: &str = "writing a stream whose data decodes only in part as \
     the bytes this viewer decoded names it in this file's own xmpMM:History, and this document's \
     XMP packet will not take that entry, so the stream is not restated and no file is written \
     (ADR 3522)";

/// Section 5.1's answer for this document, or the refusal that names why there is none.
///
/// Read after `pdf_archive::check`, which reads the whole file first, so the log is the whole
/// file's (ADR 3506).
pub(super) fn answer(
    document: &Document,
    report: &pdf_archive::Report,
    target: Target,
) -> Result<Answered, Because> {
    if !report
        .failures()
        .any(|judgement| judgement.id == BASE_STANDARD)
    {
        return Err(Because::NotThisTarget(NOTHING_FAILED));
    }
    let mut damaged = false;
    let mut unpaired = false;
    let mut content = Vec::new();
    for logged in document.repairs() {
        if !pdf_archive::repairs::breaches_row(logged.class.name, BASE_STANDARD) {
            continue;
        }
        if let Site::Content(holder) = logged.at {
            content.push((logged.class, holder));
            continue;
        }
        damaged |= logged.class.name == repair::STREAM_DATA_DAMAGED.name;
        unpaired |= logged.class.name == repair::DECODE_PARMS_MISMATCHED.name;
    }
    let mut answered = if damaged || unpaired {
        restated(document, target, (damaged, unpaired))?
    } else {
        Answered::default()
    };
    answered.content = content;
    Ok(answered)
}

/// The answer once the conversion's content rewrites are known: a repair inside a content stream
/// is answered where the bytes it was made in do not cross — the page is photographed, or every
/// content stream of the page or form is written again by the run rewrite and the class is one
/// that rewrite writes out (`super::as_drawn`), or by the integer rewrite and the class is an
/// integer too wide for 64 bits (`super::integer_reals`, ADR 3533) — and refused by name, under the fence, everywhere
/// else (ADR 3521). The output's own check is what holds a rewrite to having written every one.
///
/// A damaged stream only photographed pages named (`taken_off`) is not written, so it is not
/// restated either, and nothing is lost on its account.
pub(super) fn settled(
    answered: Result<Answered, Because>,
    document: &Document,
    (drawn, reals): (
        Option<&super::as_drawn::Drawn>,
        Option<&super::integer_reals::Restated>,
    ),
    (photographed, taken_off): (&BTreeSet<ObjectId>, &BTreeSet<ObjectId>),
) -> Result<Answered, Because> {
    let mut answered = answered?;
    answered.at.retain(|id, _| !taken_off.contains(id));
    answered
        .streams
        .retain(|stream| !taken_off.contains(&ObjectId::new(stream.object, stream.generation)));
    for (class, holder) in &answered.content {
        let rewritten = holder.is_some_and(|id| {
            let written_by = |at: &BTreeMap<ObjectId, Object>| {
                let streams = content_streams_of(document, id);
                !streams.is_empty() && streams.iter().all(|stream| at.contains_key(stream))
            };
            photographed.contains(&id)
                || (WRITTEN_OUT_BY_THE_RUN_REWRITE.contains(&class.name)
                    && drawn.is_some_and(|drawn| written_by(&drawn.at)))
                || (class.name == WRITTEN_OUT_AS_REALS
                    && reals.is_some_and(|reals| written_by(&reals.at)))
        });
        if !rewritten {
            return Err(Because::TheFence(in_content(class)));
        }
    }
    Ok(answered)
}

/// The classes `super::as_drawn` writes out of a stream it rewrites: a run it reads a number off
/// (ADR 2638) and a lone delimiter it writes as the white space it draws as (ADR 2859).
const WRITTEN_OUT_BY_THE_RUN_REWRITE: [&str; 2] =
    ["syntax/number-salvaged", "syntax/delimiter-unmatched"];

/// The class `super::integer_reals` writes out of a stream it rewrites: an integer too wide for 64
/// bits, written as the real of the same value (ADR 3533).
const WRITTEN_OUT_AS_REALS: &str = "syntax/integer-past-i64";

/// The content streams of the page or form `id`: a page's `/Contents`, or the form itself.
fn content_streams_of(document: &Document, id: ObjectId) -> Vec<ObjectId> {
    match document.get(id) {
        Object::Stream(_) => vec![id],
        Object::Dictionary(page) => super::hexadecimal::contents_of(document, &page),
        _ => Vec::new(),
    }
}

/// Every damaged stream the output carries, re-encoded from what this reader decoded.
///
/// A stream whose `/DecodeParms` does not pair with its `/Filter` is answered only where a
/// restatement re-encodes it anyway — this one, for its damage, or the ASCII row's — because the
/// re-encoding writes one `/DecodeParms` entry per stage it keeps; anywhere else it is refused by
/// name, since restating it would write this reader's pairing as the producer's.
fn restated(
    document: &Document,
    target: Target,
    (damaged, unpaired): (bool, bool),
) -> Result<Answered, Because> {
    let examination = pdf_archive::Examination::new(document, target);
    let reach = pdf_archive::Reach::of(&examination);
    let mut out = Answered::default();
    let (mut found_damage, mut found_unpaired) = (false, false);
    let programs = font_programs(examination.objects());
    for (id, object) in examination.objects() {
        let Object::Stream(stream) = object else {
            continue;
        };
        let damage = damaged.then(|| first_damage(document, stream)).flatten();
        let parameters_off = unpaired && super::rewrite::parameters_unpaired(document, stream);
        found_damage |= damage.is_some();
        found_unpaired |= parameters_off;
        // A stream nothing reaches is not written, so there is nothing of it to restate; a font
        // program's cut is the font routes' to answer — written whole, dropped for the face the
        // window drew, or substituted (ADR 3256) — and the output's own check holds them to it.
        if !reach.reaches(*id) || programs.contains(id) || cut_ascii_data_alone(document, stream) {
            continue;
        }
        let Some((stage, damage)) = damage else {
            if parameters_off {
                return Err(Because::NotBuiltYet(PARAMETERS_UNPAIRED));
            }
            continue;
        };
        let written = reencoded_through_flate(document, stream, &stream.dict, Cut::DamageAdmitted)
            .ok_or(Because::NotBuiltYet(NOT_REENCODED))?;
        out.streams.push(DamagedDataRestated {
            object: id.number,
            generation: id.generation,
            stage,
            damage: described(damage),
            encoded: stream.data.len(),
        });
        out.at.insert(*id, written);
    }
    if unpaired && !found_unpaired {
        return Err(Because::NotBuiltYet(PARAMETERS_UNPAIRED));
    }
    if damaged && !found_damage {
        return Err(Because::NotBuiltYet(NOT_FOUND));
    }
    Ok(out)
}

/// Every stream a font descriptor names as its program (ISO 32000-2 §9.9's Table 124 keys).
fn font_programs(objects: &[(ObjectId, Object)]) -> BTreeSet<ObjectId> {
    objects
        .iter()
        .filter_map(|(_, object)| object.as_dict())
        .flat_map(|dict| {
            ["FontFile", "FontFile2", "FontFile3"]
                .into_iter()
                .filter_map(|key| match dict.get(key) {
                    Some(Object::Reference(id)) => Some(*id),
                    _ => None,
                })
        })
        .collect()
}

/// Whether the stream's one fault is ASCII data cut before its marker, which the row
/// `file-structure/ascii-data-ends-at-its-end-of-data-marker` restates at its own loss (ADR
/// 3473): asked as that row asks it, so the two never restate one stream twice.
fn cut_ascii_data_alone(document: &Document, stream: &Stream) -> bool {
    pdf_archive::table::ascii_stage_cut_short(document, stream).is_some()
        && reencoded_through_flate(document, stream, &stream.dict, Cut::AsciiAdmitted).is_some()
}

/// The first stage in front of the stream's image codec, if any, that reported damage, and the
/// damage: the stream `pdf_syntax` logged as `syntax/stream-data-damaged`, asked again of the one
/// stream (`pdf_syntax::Document::decoded_stream_data_reported` keeps the first damage in a chain
/// for the same reason, and a framing missing gives way to a later stage's damage).
fn first_damage(document: &Document, stream: &Stream) -> Option<(String, Damage)> {
    if document.is_external(stream) || stream.decryption_failed {
        return None;
    }
    let mut first = None;
    let walked =
        super::rewrite::each_stage_in_front_of_the_codec(document, stream, |filter, damage| {
            if let Some(damage) = damage
                && first
                    .as_ref()
                    .is_none_or(|(_, kept): &(String, Damage)| !kept.questions_the_bytes())
            {
                first = Some((String::from_utf8_lossy(filter).into_owned(), damage));
            }
        });
    // A stage that refuses after one that reported damage leaves that damage in the log; one that
    // refuses before any did is `refused_as_the_files`'s, not this.
    if walked.is_err() && first.is_none() {
        return None;
    }
    first
}

/// What a stage's damage was, worded for the report's sentence.
const fn described(damage: Damage) -> &'static str {
    match damage {
        Damage::Truncated => "running out before its end-of-data",
        Damage::Corrupt => "broken part way through",
        Damage::CheckValue => "decoding to bytes its check value disagrees with",
        Damage::ForeignCharacter => "holding a character outside its alphabet",
        Damage::Unframed => "missing its framing",
    }
}

/// The refusal naming a class of repair made inside a content stream.
///
/// One sentence per class the content reader's lexer records, so that the refusal names the class,
/// its clause and section 5.1 as a constant the report can carry; a class not listed here takes
/// [`IN_CONTENT`]. The test below holds each entry to `pdf_syntax`'s own name and clause.
fn in_content(class: &RepairClass) -> &'static str {
    IN_CONTENT_BY_CLASS
        .iter()
        .find(|(name, _, _)| *name == class.name)
        .map_or(IN_CONTENT, |(_, _, sentence)| *sentence)
}

/// Builds one [`IN_CONTENT_BY_CLASS`] entry from the class's name and clause.
macro_rules! in_content {
    ($name:literal, $clause:literal) => {
        (
            $name,
            $clause,
            concat!(
                "a content stream of this file needed the repair ",
                $name,
                " (",
                $clause,
                "), so it does not adhere to the base standard, which ISO 19005 section 5.1 \
                 requires; the stream crosses this verb byte for byte, and correcting it would \
                 edit the producer's page, where ADR 0816's fence stops this converter — \
                 doc/pdf-a-conversion-limits.md section 5.2"
            ),
        )
    };
}

/// The sentence for each class the content reader's lexer records, with the class's clause.
const IN_CONTENT_BY_CLASS: &[(&str, &str, &str)] = &[
    in_content!("syntax/delimiter-unmatched", "ISO 32000-2 §7.2.3"),
    in_content!("syntax/name-escape-incomplete", "ISO 32000-2 §7.3.5"),
    in_content!("syntax/string-unterminated", "ISO 32000-2 §7.3.4.2"),
    in_content!(
        "syntax/hexadecimal-string-unterminated",
        "ISO 32000-2 §7.3.4.3"
    ),
    in_content!(
        "syntax/hexadecimal-string-stray-byte",
        "ISO 32000-2 §7.3.4.3"
    ),
    in_content!("syntax/integer-past-i64", "ISO 32000-2 §7.3.3"),
    in_content!("syntax/number-with-an-exponent", "ISO 32000-2 §7.3.3"),
    in_content!("syntax/number-salvaged", "ISO 32000-2 §7.3.3"),
];

/// The sentence for a class inside a content stream that [`IN_CONTENT_BY_CLASS`] does not list.
const IN_CONTENT: &str = "a content stream of this file needed a repair this reader logged \
     (the report's section 5.1 places name it), so it does not adhere to the base standard, which \
     ISO 19005 section 5.1 requires; the stream crosses this verb byte for byte, and correcting it \
     would edit the producer's page, where ADR 0816's fence stops this converter — \
     doc/pdf-a-conversion-limits.md section 5.2";

/// Why nothing is answered though the target binds section 5.1.
const NOTHING_FAILED: &str = "this reader repaired nothing in this document, so ISO 19005 \
     section 5.1 has nothing for the conversion to answer";

/// Why a chain whose parameters do not pair with its filters stops the conversion.
const PARAMETERS_UNPAIRED: &str = "a stream of this file states a /DecodeParms array that does \
     not pair with its /Filter entry for entry (syntax/decode-parms-mismatched, ISO 32000-2 \
     §7.3.8.2's Table 5), so it does not adhere to the base standard, which ISO 19005 section 5.1 \
     requires; which parameters belong to which filter is this reader's guess, and restating the \
     stream on that guess is not built (ADR 3521)";

/// Why a damaged stream that cannot be re-encoded stops the conversion.
const NOT_REENCODED: &str = "a stream of this file decodes only in part \
     (syntax/stream-data-damaged, ISO 32000-2 §7.4), which ISO 19005 section 5.1 does not admit, \
     and it cannot be written as the bytes this reader decoded: its decode parameters name another \
     object, or a stage this reader does not decode stands in front of its image codec (ADR 3521)";

/// Why damage the log names in no stream object stops the conversion.
const NOT_FOUND: &str = "this reader decoded some stream data of this file only in part \
     (syntax/stream-data-damaged, ISO 32000-2 §7.4), which ISO 19005 section 5.1 does not admit, \
     and no stream object of the file decodes that way when asked again, so the damaged data is \
     somewhere this converter does not rewrite, such as an inline image inside a content stream, \
     which crosses this verb byte for byte (ADR 3521)";

#[cfg(test)]
mod tests {
    use super::*;

    /// Each sentence names a class `pdf_syntax` declares, under that class's own clause.
    #[test]
    fn every_content_sentence_names_a_declared_class_under_its_clause() {
        for (name, clause, sentence) in IN_CONTENT_BY_CLASS {
            let declared = repair::CLASSES
                .iter()
                .find(|class| class.name == *name)
                .unwrap_or_else(|| panic!("{name} is a class pdf_syntax declares"));
            assert_eq!(declared.clause, *clause, "{name}");
            assert!(sentence.contains(name) && sentence.contains("section 5.1"));
        }
    }

    #[test]
    fn a_class_not_listed_is_refused_by_the_general_sentence() {
        assert_eq!(in_content(&repair::DUPLICATE_KEY), IN_CONTENT);
        assert!(in_content(&repair::STRING_UNTERMINATED).contains("syntax/string-unterminated"));
    }
}
