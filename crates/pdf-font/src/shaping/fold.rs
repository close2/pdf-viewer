//! What a character is *for matching*: a presentation form compared as the letters it is a form
//! of.
//!
//! The opposite direction from [`super::shape`]. A document's `/ToUnicode` may map its glyphs to
//! the Arabic Presentation Forms — ISO 32000-2 §9.10.2 hands that mapping through as stated, and
//! the readback keeps it — while a person types the nominal letters. `UnicodeData.txt` states
//! the relation between the two as each form's decomposition, and this module is that field,
//! compiled in by `build.rs` with nothing parsed at launch.
//!
//! **Folding is a matching choice, never a rewrite of what a page says.** A caller compares the
//! folded characters and reports ranges of the text it was handed (ADR 1465).

use unicode_bidi::BidiClass;

use super::tables::{PRESENTATION_DECOMPOSITIONS, PRESENTATION_LETTERS};

/// The characters `UnicodeData.txt` decomposes the presentation form `character` to, in logical
/// order, or `None` for every character that is not one.
///
/// Covers Alphabetic Presentation Forms and Arabic Presentation Forms-A and -B, under every tag
/// (`<initial>`, `<medial>`, `<final>`, `<isolated>`, `<compat>`, `<font>`) and none: the Arabic
/// positional forms fold to their letter, the lam-alef ligatures and the Latin `ﬁ` to their two
/// letters, a Hebrew letter with a point to the letter and the point. The isolated forms of the
/// vowel marks, which decompose to a space and a mark, are deliberately not folded: a space is a
/// word break to a search, and the page shows none.
#[must_use]
pub fn fold(character: char) -> Option<&'static [char]> {
    if !matches!(character, '\u{fb00}'..='\u{fdff}' | '\u{fe70}'..='\u{feff}') {
        return None;
    }
    let (_, start, length) = PRESENTATION_DECOMPOSITIONS
        .binary_search_by(|(form, _, _)| form.cmp(&character))
        .ok()
        .and_then(|at| PRESENTATION_DECOMPOSITIONS.get(at))?;
    let start = usize::from(*start);
    PRESENTATION_LETTERS.get(start..start.saturating_add(usize::from(*length)))
}

/// Whether `character` is strongly right to left: Unicode Standard Annex #9's bidirectional
/// class `R` or `AL`.
///
/// The characters whose stored order a display reverses, and so the ones whose order on a page
/// is a question at all.
#[must_use]
pub fn right_to_left(character: char) -> bool {
    character >= '\u{590}'
        && matches!(
            unicode_bidi::bidi_class(character),
            BidiClass::R | BidiClass::AL
        )
}

#[cfg(test)]
mod tests {
    use super::{fold, right_to_left};

    /// [`fold`]'s letters as a string, for comparing with `UnicodeData.txt`'s fields.
    fn folded(character: char) -> Option<String> {
        fold(character).map(|letters| letters.iter().collect())
    }

    /// `UnicodeData.txt`'s own lines for the four cases, and a character outside the blocks.
    #[test]
    fn a_presentation_form_folds_to_what_it_decomposes_to() {
        // FE94;ARABIC LETTER TEH MARBUTA FINAL FORM;Lo;0;AL;<final> 0629
        assert_eq!(folded('\u{fe94}').as_deref(), Some("\u{629}"));
        // FEFB;ARABIC LIGATURE LAM WITH ALEF ISOLATED FORM;Lo;0;AL;<isolated> 0644 0627
        assert_eq!(folded('\u{fefb}').as_deref(), Some("\u{644}\u{627}"));
        // FCCB;ARABIC LIGATURE LAM WITH KHAH INITIAL FORM;Lo;0;AL;<initial> 0644 062E
        assert_eq!(folded('\u{fccb}').as_deref(), Some("\u{644}\u{62e}"));
        // FB01;LATIN SMALL LIGATURE FI;Ll;0;L;<compat> 0066 0069
        assert_eq!(folded('\u{fb01}').as_deref(), Some("fi"));
        // FB2C;HEBREW LETTER SHIN WITH DAGESH AND SHIN DOT;Lo;0;R;FB49 05C1 — nested once.
        assert_eq!(folded('\u{fb2c}').as_deref(), Some("\u{5e9}\u{5bc}\u{5c1}"));
        // FE70;ARABIC FATHATAN ISOLATED FORM;Lo;0;AL;<isolated> 0020 064B — a space, not folded.
        assert_eq!(folded('\u{fe70}').as_deref(), None);
        assert_eq!(folded('a').as_deref(), None);
        assert_eq!(
            folded('\u{627}').as_deref(),
            None,
            "a nominal letter is itself"
        );
    }

    #[test]
    fn arabic_and_hebrew_are_right_to_left_and_digits_are_not() {
        assert!(right_to_left('\u{627}'));
        assert!(right_to_left('\u{fe94}'));
        assert!(right_to_left('\u{5d0}'));
        assert!(!right_to_left('1'));
        assert!(
            !right_to_left('\u{661}'),
            "ARABIC-INDIC DIGIT ONE is AN, a number"
        );
        assert!(!right_to_left(' '));
        assert!(!right_to_left('a'));
    }
}
