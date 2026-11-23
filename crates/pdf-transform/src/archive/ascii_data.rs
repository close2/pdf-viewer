//! A stream whose ASCII-armoured data stops before its end-of-data marker, written as the bytes
//! this reader decoded.
//!
//! # What the clauses allow
//!
//! ISO 19005-2 section 5.1 and ISO 19005-4 section 5.1 require a conforming file to conform to the
//! base standard, and ISO 32000-2 §7.4.2 and §7.4.3 give each ASCII encoding its end-of-data
//! marker. §7.4.3 makes the last, partial group of characters the encoding of the last bytes only
//! where the marker follows it: those characters are to be followed immediately by `~>`. So data
//! that stops before the marker states no bytes for the group the cut ended. This reader decodes
//! the complete groups and draws them (`doc/adr/3281`); another reader may pad the partial group
//! into bytes of its own choosing, and the two readings are both readings of damage. The stream is
//! written as what the window drew — the bytes decoded through every stage in front of any image
//! codec, re-encoded as one `FlateDecode` — which restates the drawing rather than repairing the
//! data. What is lost is the cut group's characters, the loss [`super::Loss::AsciiDataAsDecoded`],
//! and the report names each stream (`doc/adr/3462`).
//!
//! # Where it is answered here, and where not
//!
//! Only where every stream the validator named can be re-encoded: one whose data lives outside
//! the file, did not decrypt, names its parameters through another object, or whose later stage
//! reports damage of its own over the bytes the cut handed on keeps the refusal, by name.

use std::collections::BTreeMap;

use pdf_syntax::Document;
use pdf_syntax::object::{Object, ObjectId};

use super::decision::Because;

/// The requirement this module answers, by the identifier the validator reports it under.
pub(super) const SITE: &str = "file-structure/ascii-data-ends-at-its-end-of-data-marker";

/// One stream written as the bytes this reader decoded, as the report names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsciiDataRestated {
    /// The stream, by object number.
    pub object: u32,
    /// Its generation.
    pub generation: u16,
    /// The ASCII stage whose data stopped before its marker, by the name the stream states it
    /// under.
    pub stage: String,
    /// How many bytes of stream data the source held.
    pub encoded: usize,
}

impl AsciiDataRestated {
    /// The report's sentence for this stream.
    #[must_use]
    pub fn sentence(&self) -> String {
        format!(
            "the stream object {} {} ({} bytes of stream data, its {} stage ending before its \
             end-of-data marker) is written as the bytes this viewer decoded from its complete \
             groups (ascii-data-as-decoded)",
            self.object, self.generation, self.encoded, self.stage
        )
    }
}

/// The streams this conversion restates.
#[derive(Debug, Default)]
pub(super) struct Restated {
    /// Each stream's replacement, in the source's numbering.
    pub(super) at: BTreeMap<ObjectId, Object>,
    /// What the report names, one entry per stream.
    pub(super) streams: Vec<AsciiDataRestated>,
}

/// Every stream the requirement was failed at, re-encoded from what this reader decoded, or why
/// the restatement does not answer the document.
pub(super) fn restate(
    document: &Document,
    report: &pdf_archive::Report,
) -> Result<Restated, Because> {
    let Some(judgement) = report.failures().find(|judgement| judgement.id == SITE) else {
        return Err(Because::NotThisTarget(NOTHING_FAILED));
    };
    let places = super::sites::places_failed(judgement)?;
    let mut out = Restated::default();
    for place in places {
        let id = place.object.ok_or(Because::NotBuiltYet(NOT_A_STREAM))?;
        let Object::Stream(stream) = document.get(id) else {
            return Err(Because::NotBuiltYet(NOT_A_STREAM));
        };
        // The validator's own question, asked again of the stream about to be written, so that
        // the stage the report names is the one this reader found cut.
        let stage = pdf_archive::table::ascii_stage_cut_short(document, &stream)
            .ok_or(Because::NotBuiltYet(NOT_A_STREAM))?;
        let written = super::rewrite::reencoded_through_flate(
            document,
            &stream,
            &stream.dict,
            super::rewrite::Cut::AsciiAdmitted,
        )
        .ok_or(Because::NotBuiltYet(NOT_REENCODED))?;
        out.streams.push(AsciiDataRestated {
            object: id.number,
            generation: id.generation,
            stage,
            encoded: stream.data.len(),
        });
        out.at.insert(id, written);
    }
    if out.at.is_empty() {
        return Err(Because::NotThisTarget(NOTHING_FAILED));
    }
    Ok(out)
}

/// Why nothing is restated though the target binds the requirement.
const NOTHING_FAILED: &str = "no stream in this document has ASCIIHexDecode or ASCII85Decode \
     data that stops before its end-of-data marker, so there is nothing to restate";

/// Why a place that names no stream this reader finds cut stops the restatement.
const NOT_A_STREAM: &str = "the validator named a place for ASCII data cut before its \
     end-of-data marker that this reader does not read as such a stream, so there are no decoded \
     bytes to write";

/// Why a stream that cannot be re-encoded stops the restatement.
const NOT_REENCODED: &str = "a stream whose ASCII data stops before its end-of-data marker \
     cannot be written as the bytes this reader decoded: its data lives outside the file or did \
     not decrypt, its decode parameters name another object, a stage this reader does not decode \
     stands in front of its image codec, or a stage after the cut reports damage of its own, so \
     the bytes handed on are not the file's (ADR 3462)";
