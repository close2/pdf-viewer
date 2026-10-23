//! ISO 32000-2 §9.6.2.1's Table 109 `/Widths` made as long as the simple font's stated character
//! range, which ISO 19005-2 section 6.2.11.2 and ISO 19005-4 section 6.2.10.2 require by requiring
//! every font to conform to the base standard's font clauses (ADR 2178). §9.6.3 gives a TrueType
//! font dictionary the same entries, which is why the validator's rule reads `Type1`, `MMType1` and
//! `TrueType` alike. §9.6.2.1's Table 109 states both the length and what a code outside it
//! advances by:
//!
//! > An array of ( LastChar - FirstChar + 1) numbers, each element being the glyph width for the
//! > character code that equals FirstChar plus the array index. For character codes outside the
//! > range FirstChar to LastChar , the value of MissingWidth from the FontDescriptor entry for
//! > this font shall be used.
//!
//! A short array narrows `/LastChar`; a long array loses entries past `/LastChar`. Both
//! state the same advances this reader uses: codes without a stated width use `/MissingWidth`.
//! An empty array is refused because the font loader may then use substitute metrics.

use std::collections::BTreeMap;

use pdf_archive::{Outcome, Report};
use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId};

use super::decision::Because;

/// The dictionary entries to replace, by the font object the validator named.
#[derive(Debug, Default)]
pub(super) struct WidthArrays {
    /// Only `/LastChar` or `/Widths`; every retained width is the producer's original value.
    pub(super) at: BTreeMap<ObjectId, Dictionary>,
}

/// Why an array that cannot be narrowed without choosing metrics stays refused.
const NOT_RESTATABLE: &str = "a simple font's Widths array does not cover its stated character \
    range, but the array is empty or its range cannot be read. This reader may then use a \
    substitute's metrics; inventing an entry would not state the widths the page already uses";

/// Restates only the font objects the widths-length requirement found.
pub(super) fn restated(document: &Document, input: &Report) -> Result<WidthArrays, Because> {
    let mut out = WidthArrays::default();
    for judgement in input.failures() {
        if judgement.id != "fonts/widths-array-covers-character-range" {
            continue;
        }
        let Outcome::Failed { places, .. } = &judgement.outcome else {
            continue;
        };
        for finding in places {
            let id = finding
                .place
                .object
                .ok_or(Because::NotBuiltYet(NOT_RESTATABLE))?;
            let object = document.get(id);
            let dict = object
                .as_dict()
                .ok_or(Because::NotBuiltYet(NOT_RESTATABLE))?;
            let first = document
                .get_key(dict, "FirstChar")
                .as_integer()
                .ok_or(Because::NotBuiltYet(NOT_RESTATABLE))?;
            let last = document
                .get_key(dict, "LastChar")
                .as_integer()
                .ok_or(Because::NotBuiltYet(NOT_RESTATABLE))?;
            let widths = document.get_key(dict, "Widths");
            let widths = widths
                .as_array()
                .ok_or(Because::NotBuiltYet(NOT_RESTATABLE))?;
            let wanted = last
                .checked_sub(first)
                .and_then(|n| n.checked_add(1))
                .and_then(|n| usize::try_from(n).ok())
                .filter(|n| *n > 0)
                .ok_or(Because::NotBuiltYet(NOT_RESTATABLE))?;
            if widths.is_empty() {
                return Err(Because::NotBuiltYet(NOT_RESTATABLE));
            }
            let mut entries = Dictionary::new();
            if widths.len() < wanted {
                let count = i64::try_from(widths.len())
                    .map_err(|_| Because::NotBuiltYet(NOT_RESTATABLE))?;
                let covered = first
                    .checked_add(count)
                    .and_then(|n| n.checked_sub(1))
                    .ok_or(Because::NotBuiltYet(NOT_RESTATABLE))?;
                entries.insert(Name::new(&b"LastChar"[..]), Object::Integer(covered));
            } else if widths.len() > wanted {
                entries.insert(
                    Name::new(&b"Widths"[..]),
                    Object::Array(widths[..wanted].to_vec()),
                );
            }
            if !entries.is_empty() {
                out.at.insert(id, entries);
            }
        }
    }
    Ok(out)
}
