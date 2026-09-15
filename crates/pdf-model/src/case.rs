//! Full case folding, which is what ISO 32000-2 §12.3.5.2 means by case normalization.
//!
//! The clause restricts a collection's names and then compares them with the case taken out:
//!
//! > In addition to the restriction on naming folders, as just described, it is further required
//! > that two file names in the same folder do not map to the same string following case
//! > normalization.
//!
//! and sends a reader to Unicode Standard Annex #21, Case Mappings, for what that mapping is.
//! That annex defines caseless matching as a comparison of two strings after the operation it
//! calls toCasefold — map every character to its case folding — and names CaseFolding.txt in the
//! Unicode Character Database as the data for it (Annex #21 sections 2.3 and 2.5). The annex
//! itself was superseded by the Unicode Standard's core specification at version 4.0, which
//! carries the same operation under Default Case Algorithms; the data file is the part that is
//! still published and versioned, and it is what this module is generated from.
//!
//! # Why folding rather than lowercasing
//!
//! Lowercasing is the operation that looks equivalent and is not. Three characters make the
//! difference visible, and each of them is a name a collection can hold:
//!
//! - U+00DF LATIN SMALL LETTER SHARP S lowercases to itself and folds to `ss`, so `Straße` and
//!   `STRASSE` are one name folded and two lowercased;
//! - U+017F LATIN SMALL LETTER LONG S lowercases to itself and folds to `s`;
//! - U+03C2 GREEK SMALL LETTER FINAL SIGMA lowercases to itself and folds to U+03C3, which is
//!   what U+03A3 GREEK CAPITAL LETTER SIGMA also folds to — so a name ending in a final sigma
//!   and the same name in capitals are one name.
//!
//! In every one of those the lowercasing answer is *no collision*, which is the answer that
//! reports nothing. That is why the debt was worth a data file: an instrument whose failure mode
//! is silence cannot be left approximately right (ADR 1086).
//!
//! # Full, and not Turkic
//!
//! `CaseFolding.txt` publishes four statuses and its own note says which combination is which
//! operation: C plus S is the simple folding, C plus F the full one. This table is C plus F,
//! because the simple folding exists for implementations that cannot let a string grow and
//! taking it would leave U+00DF folding to itself — the defect above, rebuilt.
//!
//! The file's status T is the pair of mappings for Turkic languages, in which U+0049 LATIN
//! CAPITAL LETTER I folds to the dotless U+0131 rather than to `i`. The data file states that
//! excluding them is the default, and this table excludes them: a folder name in a PDF carries
//! no language, so there is nothing in a document that could choose the Turkic reading, and
//! choosing it anyway would make `FILE` and `fıle` one name in every document on earth.

// The table is `static FOLDING: &[(char, &str)]`, sorted by code point, and the build script's
// header says what it is generated from. Included rather than committed for `pdf-spec`'s reason:
// the data file is the source of truth and a checked-in copy of its contents would be a second
// one (ADR 1086).
include!(concat!(env!("OUT_DIR"), "/case_folding.rs"));

/// One character's full case folding, or `None` where it folds to itself.
///
/// `None` rather than a one-character string, because the great majority of characters fold to
/// themselves and the caller can keep the character it already has.
#[must_use]
pub fn folded(character: char) -> Option<&'static str> {
    FOLDING
        .binary_search_by_key(&character, |(from, _)| *from)
        .ok()
        .map(|at| FOLDING[at].1)
}

/// A string with its case folded away: Unicode Standard Annex #21's toCasefold.
///
/// The annex defines the operation character by character, so this is a map and not a scan: no
/// context around a character changes its folding, which is the difference between folding and
/// lowercasing U+03A3 GREEK CAPITAL LETTER SIGMA.
///
/// Two strings are a caseless match when their foldings are equal, which is the comparison
/// §12.3.5.2 asks for on two names in one folder.
#[must_use]
pub fn fold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match folded(character) {
            Some(folded) => out.push_str(folded),
            None => out.push(character),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{fold, folded};

    /// The calibration this module exists for (trap 13): the three characters where folding and
    /// lowercasing disagree, with the lowercasing answer computed here rather than remembered.
    ///
    /// `char::to_lowercase` is what `collection::case_normalised` used until ADR 1086, so this
    /// test *is* the planted defect: each assertion pair names a collision the old function
    /// answered `false` to, and it fails if the table ever stops holding the row that finds it.
    #[test]
    fn folding_finds_the_three_collisions_lowercasing_misses() {
        let lowercased =
            |text: &str| -> String { text.chars().flat_map(char::to_lowercase).collect() };

        // U+00DF folds to `ss`, so the two spellings of a German street are one name.
        assert_eq!(fold("Straße"), fold("STRASSE"));
        assert_ne!(lowercased("Straße"), lowercased("STRASSE"));

        // U+017F, the long s, folds to `s`.
        assert_eq!(fold("\u{17f}ummer"), fold("Summer"));
        assert_ne!(lowercased("\u{17f}ummer"), lowercased("Summer"));

        // U+03C2, the final sigma, and U+03A3, the capital, both fold to U+03C3.
        assert_eq!(
            fold("\u{3bf}\u{3b4}\u{3bf}\u{3c2}"),
            fold("\u{39f}\u{394}\u{39f}\u{3a3}")
        );
        assert_ne!(
            lowercased("\u{3bf}\u{3b4}\u{3bf}\u{3c2}"),
            lowercased("\u{39f}\u{394}\u{39f}\u{3a3}")
        );
    }

    /// The data file's status T, excluded — which is its own stated default and this module's
    /// choice for the reason the header gives.
    #[test]
    fn the_turkic_mappings_are_not_in_the_table() {
        // U+0049 takes the non-Turkic row, `i`, so `I` and `i` are one name.
        assert_eq!(folded('I'), Some("i"));
        // U+0131 LATIN SMALL LETTER DOTLESS I folds to itself, so it is a name of its own.
        assert_eq!(folded('\u{131}'), None);
        assert_ne!(fold("FILE"), fold("f\u{131}le"));
        // U+0130 takes its status F row rather than its status T one: `i` and a combining dot.
        assert_eq!(folded('\u{130}'), Some("i\u{307}"));
    }

    /// A folding may make a string longer, which is the whole of what separates status F from
    /// status S, and a caller that assumed otherwise would truncate a name.
    #[test]
    fn a_folded_string_may_be_longer_than_the_one_it_came_from() {
        // U+FB03 LATIN SMALL LIGATURE FFI folds to three characters.
        assert_eq!(fold("\u{fb03}"), "ffi");
        assert!(fold("\u{fb03}").len() > "\u{fb03}".chars().count());
    }

    /// What the table does *not* do. Folding is not normalisation: the annex says so itself, and
    /// a reader that expected one from the other would be wrong about composed characters.
    #[test]
    fn folding_leaves_a_character_that_has_no_case_alone() {
        assert_eq!(folded('7'), None);
        assert_eq!(folded('\u{4e2d}'), None);
        assert_eq!(fold("report.pdf"), "report.pdf");
    }
}
