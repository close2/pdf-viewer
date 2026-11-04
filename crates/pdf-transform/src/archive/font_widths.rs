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
//!
//! **A dictionary that states a descriptor and leaves out any of `/FirstChar`, `/LastChar` and
//! `/Widths`** is given all three, as the advances this reader lays its glyphs by
//! ([`super::fonts::state_laid_out_widths`]), which `fonts/simple-font-states-its-widths` asks
//! for: ISO 32000-1 Table 111 lets only a standard 14 font leave them out, and then with
//! `/FontDescriptor` too, and ISO 32000-2's Table 109 lets none. For one of the fourteen names
//! those advances are the standard's published metrics, which §9.6.2.1 makes a processor provide
//! when the entries are absent, and otherwise `/MissingWidth`, which Table 109 sends every code
//! outside a stated range to; either way the numbers written are the ones the page is already
//! drawn by (`doc/adr/2686`). A dictionary with no descriptor is the embedding route's where the
//! font is drawn, which writes the four entries with the program it embeds; one drawn only in
//! rendering mode 3, which no route embeds, is refused by name.

use std::collections::{BTreeMap, BTreeSet};

use pdf_archive::{Outcome, Report};
use pdf_font::LoadedFont;
use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId};

use super::decision::Because;
use super::rewrite::Rewrite;

/// The dictionary entries to replace, by the font object the validator named.
#[derive(Debug, Default)]
pub(super) struct WidthArrays {
    /// Only `/LastChar` or `/Widths`; every retained width is the producer's original value.
    pub(super) at: BTreeMap<ObjectId, Dictionary>,
    /// `/FirstChar`, `/LastChar` and `/Widths`, for a font that left any of them out.
    pub(super) stated: BTreeMap<ObjectId, Dictionary>,
}

impl WidthArrays {
    /// The entries written into the font object `id`, and the rewrite that writes them.
    pub(super) fn of(&self, id: ObjectId) -> Option<(&Dictionary, Rewrite)> {
        self.at
            .get(&id)
            .map(|entries| (entries, Rewrite::FontWidthArrayRestated))
            .or_else(|| {
                self.stated
                    .get(&id)
                    .map(|entries| (entries, Rewrite::FontWidthsStated))
            })
    }
}

/// Why an array that cannot be narrowed without choosing metrics stays refused.
const NOT_RESTATABLE: &str = "a simple font's Widths array does not cover its stated character \
    range, but the array is empty or its range cannot be read. This reader may then use a \
    substitute's metrics; inventing an entry would not state the widths the page already uses";

/// Why a font that leaves out its widths and cannot be read is not given them.
const NOT_LAID_OUT: &str = "a simple font states a FontDescriptor but not all of FirstChar, \
    LastChar and Widths, which ISO 32000-1 Table 111 allows only a standard 14 font and then \
    only with all four entries absent, and ISO 32000-2 Table 109 allows no font; the three \
    entries are written as the advances this reader lays the glyphs by, and this font is one \
    this reader cannot load or one written directly inside another object, so there is no such \
    advance to state or no object to state it in";

/// Why a font that states no descriptor and that no route embeds is not given one.
const NO_DESCRIPTOR_UNDRAWN: &str = "a simple font a content stream uses only in text rendering \
    mode 3 states no FontDescriptor, nor FirstChar, LastChar and Widths, which ISO 32000-2 Table \
    109 requires of every font in a PDF 2.0 file and ISO 32000-1 Table 111 of every font but the \
    standard 14; the embedding route that writes all four is taken only for a font that is drawn, \
    and writing a descriptor for a font no route embeds is a remedy this version does not carry \
    out";

/// The requirement whose fonts are given the three entries.
const STATES_ITS_WIDTHS: &str = "fonts/simple-font-states-its-widths";

/// How many width entries the restatement reads, all fonts together.
///
/// A simple font's codes are single bytes, so a legitimate array holds about 256 entries and
/// [`pdf_archive::MAX_NAMED`] fonts about a mebi-entry; that product is the bound. It is needed
/// because the fonts are the whole population rather than thirty-two (trap 305): every font may
/// name one shared `/Widths` array by reference, and each font's reading — and copy — of a long
/// one would otherwise cost fonts times the array.
const MAX_READ_WIDTHS: usize = pdf_archive::MAX_NAMED * 256;

/// Why the arrays were not restated.
const WIDTHS_OVER_BUDGET: &str = "the simple fonts of this file whose Widths arrays do not \
    match their character ranges state more width entries together than the 1 048 576 this \
    conversion reads of them (256 for each of 4096 fonts)";

/// Restates the font objects the widths-length requirement found, every one of them.
///
/// **The fonts are the requirement's whole population**, [`super::sites::places_failed`]'s, and
/// not the thirty-two places [`Outcome::Failed`] keeps (`doc/adr/2616`): a font past the kept
/// prefix keeps its array and the file fails the requirement it answered. The copies are bounded
/// by [`MAX_READ_WIDTHS`].
pub(super) fn restated(document: &Document, input: &Report) -> Result<WidthArrays, Because> {
    let mut out = WidthArrays::default();
    let mut read = 0_usize;
    // The fonts the embedding route answers, which writes all four entries with the face: by
    // object, and `None` for a font written directly inside another object, which that route
    // writes out as an object of its own.
    let mut embedded = BTreeSet::new();
    for judgement in input.failures() {
        if judgement.id == "fonts/font-programs-embedded" {
            for place in super::sites::places_failed(judgement)? {
                embedded.insert(place.object);
            }
        }
    }
    for judgement in input.failures() {
        if !matches!(judgement.outcome, Outcome::Failed { .. }) {
            continue;
        }
        if judgement.id == STATES_ITS_WIDTHS {
            for place in super::sites::places_failed(judgement)? {
                laid_out(document, place.object, &embedded, &mut out)?;
            }
            continue;
        }
        if judgement.id != "fonts/widths-array-covers-character-range" {
            continue;
        }
        for place in super::sites::places_failed(judgement)? {
            let id = place.object.ok_or(Because::NotBuiltYet(NOT_RESTATABLE))?;
            if out.at.contains_key(&id) {
                continue;
            }
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
            read = read.saturating_add(widths.len());
            if read > MAX_READ_WIDTHS {
                return Err(Because::NotBuiltYet(WIDTHS_OVER_BUDGET));
            }
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

/// Gives the font at `at` the `/FirstChar`, `/LastChar` and `/Widths` it lays its glyphs out by,
/// where it states a descriptor; a font with none is left to the embedding route where that
/// route takes it, in `embedded`, and refused otherwise.
///
/// A font written directly inside another object has no object here to edit, and is left to the
/// embedding route where that route takes such a font: the route writes it out as an object with
/// all four entries, and the output's second verdict holds whatever it did not (`doc/adr/2686`).
fn laid_out(
    document: &Document,
    at: Option<ObjectId>,
    embedded: &BTreeSet<Option<ObjectId>>,
    out: &mut WidthArrays,
) -> Result<(), Because> {
    let Some(id) = at else {
        return if embedded.contains(&None) {
            Ok(())
        } else {
            Err(Because::NotBuiltYet(NOT_LAID_OUT))
        };
    };
    if out.stated.contains_key(&id) {
        return Ok(());
    }
    let object = document.get(id);
    let dict = object.as_dict().ok_or(Because::NotBuiltYet(NOT_LAID_OUT))?;
    if document.get_key(dict, "FontDescriptor").is_null() {
        return if embedded.contains(&Some(id)) {
            Ok(())
        } else {
            Err(Because::NotBuiltYet(NO_DESCRIPTOR_UNDRAWN))
        };
    }
    let name = document
        .get_key(dict, "BaseFont")
        .as_name()
        .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
        .unwrap_or_default();
    let font =
        LoadedFont::load(document, dict, &name).map_err(|_| Because::NotBuiltYet(NOT_LAID_OUT))?;
    let mut entries = Dictionary::new();
    super::fonts::state_laid_out_widths(&mut entries, &font);
    out.stated.insert(id, entries);
    Ok(())
}
