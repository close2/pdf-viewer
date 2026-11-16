//! An `/ActualText` entry holding a Private Use character, removed from the object that states it.
//!
//! # What the clauses allow
//!
//! ISO 19005-4 section 6.2.10.8 recommends an `/ActualText` for a character mapped into the Unicode
//! Private Use Area, and then forbids the entry itself to contain any Private Use value. ISO
//! 32000-2 §14.9.4 makes the entry optional on a structure element and on a marked-content
//! property list, and says what its value is:
//!
//! > The ActualText value shall be used as a replacement, not a description, for the content,
//! > providing text that is equivalent to what a person would see when viewing the content.
//!
//! A Private Use value is equivalent to nothing a person sees unless the reader shares the
//! producer's private agreement, so the entry carries no replacement any reader of the standard
//! can use. Two rewrites meet part 4: the private characters taken out of the value, or the entry
//! taken off. The first writes a value the producer never wrote — for a bullet stated as U+F0B7,
//! an empty string, which claims the content reads as nothing — so it is a substitution this
//! converter would be inventing. The second leaves the content to be read as its own text, the
//! state §14.9.4 describes for content with no entry, and claims nothing. It is the one taken,
//! at the loss [`super::Loss::ActualTextPrivateUse`]: the value is gone, and the report names each
//! object and the value it held (`doc/adr/2889`).
//!
//! # Where it is answered here, and where not
//!
//! Only where every place the validator kept names an **object** — a structure element's
//! dictionary, or a property list held as a resource. An entry written inline in a `BDC`
//! operator's operand is inside a producer's content stream, ADR 0816's fence; its page is the
//! photograph's (`super::fenced`), and a document holding both kinds keeps the refusal.

use std::collections::BTreeMap;

use pdf_syntax::Document;
use pdf_syntax::object::{Object, ObjectId};

use super::decision::Because;

/// The requirement this module answers, by the identifier the validator reports it under.
pub(super) const SITE: &str = "fonts/actual-text-states-no-private-use";

/// One entry removed, as the report names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedActualText {
    /// The object that stated it, by number.
    pub object: u32,
    /// Its generation.
    pub generation: u16,
    /// The value it held, each character as Unicode writes it (`U+F0B7`), so that a Private Use
    /// value survives a report read anywhere.
    pub value: String,
}

/// The entries this conversion removes.
#[derive(Debug, Default)]
pub(super) struct Removed {
    /// The objects that lose the entry, in the source's numbering.
    pub(super) at: BTreeMap<ObjectId, RemovedActualText>,
}

/// Every object the requirement was failed at, with the value each one holds, or why the removal
/// does not answer the document.
pub(super) fn removal(
    document: &Document,
    report: &pdf_archive::Report,
) -> Result<Removed, Because> {
    let Some(judgement) = report.failures().find(|judgement| judgement.id == SITE) else {
        return Err(Because::NotThisTarget(NOTHING_FAILED));
    };
    let places = super::sites::places_failed(judgement)?;
    let mut out = Removed::default();
    for place in places {
        let Some(id) = place.object else {
            return Err(Because::TheFence(INLINE_IN_A_CONTENT_STREAM));
        };
        let dict = match document.get(id) {
            Object::Dictionary(dict) => dict,
            Object::Stream(stream) => stream.dict.clone(),
            _ => return Err(Because::NotBuiltYet(NOT_A_DICTIONARY)),
        };
        let value = document
            .get_key(&dict, "ActualText")
            .as_string()
            .map(<[u8]>::to_vec)
            .ok_or(Because::NotBuiltYet(NOT_A_DICTIONARY))?;
        out.at.insert(
            id,
            RemovedActualText {
                object: id.number,
                generation: id.generation,
                value: code_points(&value),
            },
        );
    }
    if out.at.is_empty() {
        return Err(Because::NotThisTarget(NOTHING_FAILED));
    }
    Ok(out)
}

/// A text string (ISO 32000-2 §7.9.2.2) as its characters' Unicode names, space-separated.
fn code_points(bytes: &[u8]) -> String {
    pdf_syntax::text_string::text_string(bytes)
        .chars()
        .map(|character| format!("U+{:04X}", u32::from(character)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Why nothing is removed though the target binds the requirement.
const NOTHING_FAILED: &str = "no ActualText entry in this document holds a character of the \
     Unicode Private Use Area, so there is nothing to remove";

/// Why an entry inside a content stream is not removed here.
const INLINE_IN_A_CONTENT_STREAM: &str = "an ActualText entry holding a Private Use character is \
     written inline in a marked-content operator's property list, inside a content stream this \
     verb carries byte for byte (ADR 0816's fence); the page holding it is photographed under a \
     profile that admits text-as-image, and an entry an object states beside it is not removed \
     alone";

/// Why a place that names no dictionary stops the removal.
const NOT_A_DICTIONARY: &str = "the validator named an object for an ActualText entry holding a \
     Private Use character that this reader does not read as a dictionary stating one, so there \
     is no entry to remove";
