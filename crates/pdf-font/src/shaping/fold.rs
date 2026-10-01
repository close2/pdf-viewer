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
//!
//! The same file states two more relations a search compares by: each character's canonical
//! decomposition ([`decompose`]), and which characters are nonspacing marks a writer may leave
//! off ([`mark_class`]), which is how a word typed without its vowel points finds one printed with
//! them (ADR 1477).

use unicode_bidi::BidiClass;

use super::tables::{
    CANONICAL_DECOMPOSITIONS, CANONICAL_LETTERS, COMBINING_MARKS, PRESENTATION_DECOMPOSITIONS,
    PRESENTATION_LETTERS,
};

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

/// Pushes `character`'s full canonical decomposition onto `into`, or `character` itself where it
/// has none.
///
/// The Unicode Standard's canonical equivalence (section 3.7): `é` and `e` followed by U+0301 are
/// two spellings of one text, and a comparison that told them apart would be comparing how a
/// producer's encoder happened to compose. `UnicodeData.txt`'s untagged decomposition field,
/// expanded to its fixed point by `build.rs`, and the Hangul syllables by section 3.12's
/// arithmetic, which is where the standard states them. The marks come out in the order the field
/// lists them; canonical *ordering* is the caller's, which [`mark_class`] answers for.
pub fn decompose(character: char, into: &mut Vec<char>) {
    // No character below U+00C0 decomposes, and that is every ASCII character a page holds.
    if character < '\u{c0}' {
        into.push(character);
        return;
    }
    if let Some(index) = u32::from(character).checked_sub(HANGUL_FIRST)
        && index < HANGUL_COUNT
    {
        // Section 3.12: a syllable is a leading consonant, a vowel and an optional trailing
        // consonant, each a conjoining jamo found by division.
        let jamo = |base: u32, offset: u32| char::from_u32(base.saturating_add(offset));
        into.extend(jamo(0x1100, index / (21 * 28)));
        into.extend(jamo(0x1161, index % (21 * 28) / 28));
        if index % 28 != 0 {
            into.extend(jamo(0x11a7, index % 28));
        }
        return;
    }
    let expansion = CANONICAL_DECOMPOSITIONS
        .binary_search_by(|(composed, _, _)| composed.cmp(&character))
        .ok()
        .and_then(|at| CANONICAL_DECOMPOSITIONS.get(at))
        .and_then(|(_, start, length)| {
            let start = usize::from(*start);
            CANONICAL_LETTERS.get(start..start.saturating_add(usize::from(*length)))
        });
    match expansion {
        Some(parts) => into.extend_from_slice(parts),
        None => into.push(character),
    }
}

/// The first Hangul syllable, U+AC00, and how many there are.
const HANGUL_FIRST: u32 = 0xac00;
/// Section 3.12's `SCount`: 19 leading consonants by 21 vowels by 28 trailing positions.
const HANGUL_COUNT: u32 = 19 * 21 * 28;

/// `character`'s canonical combining class, where it is a nonspacing mark that class places, and
/// `None` for every other character.
///
/// General category `Mn` with a class other than zero: an Arabic haraka, the superscript alef and
/// the Quranic annotation signs, a Hebrew point or accent, a Latin, Greek or Cyrillic combining
/// accent. A nonspacing mark of class zero — a Devanagari or Thai vowel sign — spells its syllable
/// and is a letter here (ADR 1477). The class is what canonical ordering sorts a run of marks by,
/// so two runs that differ only in the order a producer stored marks on different sides of a
/// letter compare equal once each is sorted by it.
#[must_use]
pub fn mark_class(character: char) -> Option<u8> {
    if character < '\u{300}' {
        return None;
    }
    let at = COMBINING_MARKS.partition_point(|(first, _, _)| *first <= character);
    let (_, last, class) = COMBINING_MARKS.get(at.checked_sub(1)?)?;
    (character <= *last).then_some(*class)
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
    use super::{decompose, fold, mark_class, right_to_left};

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

    /// [`decompose`]'s characters as a string.
    fn decomposed(character: char) -> String {
        let mut into = Vec::new();
        decompose(character, &mut into);
        into.into_iter().collect()
    }

    /// `UnicodeData.txt`'s own lines, a nested decomposition, a Hangul syllable by section 3.12's
    /// worked example, and the two kinds of character that decompose to nothing.
    #[test]
    fn a_character_decomposes_canonically_to_its_fixed_point() {
        // 00E9;LATIN SMALL LETTER E WITH ACUTE;Ll;0;L;0065 0301
        assert_eq!(decomposed('\u{e9}'), "e\u{301}");
        // 1EA5;...A WITH CIRCUMFLEX AND ACUTE;Ll;0;L;00E2 0301, and 00E2 is 0061 0302.
        assert_eq!(decomposed('\u{1ea5}'), "a\u{302}\u{301}");
        // 0622;ARABIC LETTER ALEF WITH MADDA ABOVE;Lo;0;AL;0627 0653
        assert_eq!(decomposed('\u{622}'), "\u{627}\u{653}");
        // Section 3.12: U+D4DB is U+1111 U+1171 U+11B6.
        assert_eq!(decomposed('\u{d4db}'), "\u{1111}\u{1171}\u{11b6}");
        // FB01's decomposition is tagged <compat>: not canonical, so not here.
        assert_eq!(decomposed('\u{fb01}'), "\u{fb01}");
        assert_eq!(decomposed('e'), "e");
    }

    /// The marks ADR 1477 names, by their `UnicodeData.txt` lines, and the class-zero signs it
    /// keeps as letters.
    #[test]
    fn a_mark_is_a_nonspacing_mark_with_a_combining_class() {
        // 064E;ARABIC FATHA;Mn;30   0651;ARABIC SHADDA;Mn;33   0670;...SUPERSCRIPT ALEF;Mn;35
        assert_eq!(mark_class('\u{64e}'), Some(30));
        assert_eq!(mark_class('\u{651}'), Some(33));
        assert_eq!(mark_class('\u{670}'), Some(35));
        // 06D6;ARABIC SMALL HIGH LIGATURE SAD WITH LAM WITH ALEF MAKSURA;Mn;230 — a Quranic sign.
        assert_eq!(mark_class('\u{6d6}'), Some(230));
        // 05B8;HEBREW POINT QAMATS;Mn;18   05BC;HEBREW POINT DAGESH OR MAPIQ;Mn;21
        assert_eq!(mark_class('\u{5b8}'), Some(18));
        assert_eq!(mark_class('\u{5bc}'), Some(21));
        // 0301;COMBINING ACUTE ACCENT;Mn;230
        assert_eq!(mark_class('\u{301}'), Some(230));
        // 0941;DEVANAGARI VOWEL SIGN U;Mn;0 — spells its syllable.
        assert_eq!(mark_class('\u{941}'), None);
        // 0903;DEVANAGARI SIGN VISARGA;Mc;0, and two letters.
        assert_eq!(mark_class('\u{903}'), None);
        assert_eq!(mark_class('\u{628}'), None);
        assert_eq!(mark_class('a'), None);
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
