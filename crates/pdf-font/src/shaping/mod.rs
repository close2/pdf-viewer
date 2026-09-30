//! Text this program writes itself: the order its characters are displayed in and the form each
//! takes.
//!
//! Everything else in this crate starts from a code a document wrote. ISO 32000-2 §12.7.4.3 is
//! the one place a processor writes the codes: "the PDF processor shall construct an appearance
//! stream dynamically at rendering time", from a value §7.9.2.2.1 makes Unicode text — "UTF-16BE,
//! UTF-8 and Unicode character encoding are described in The Unicode Standard by the Unicode
//! Consortium". Unicode text is stored in logical order and in nominal forms, so a value in a
//! right-to-left, cursive script cannot be drawn by setting its characters one after another:
//!
//! - [`Paragraphs`] and [`visual_order`] are Unicode Standard Annex #9's bidirectional
//!   algorithm, its resolution taken from Servo's `unicode-bidi` and measured against the UCD's
//!   `BidiCharacterTest.txt` (`tests/bidi_character_test.rs`), its rule L1 applied here per laid
//!   out line, and [`mirrored`] its rule L4 (ADR 1413).
//! - [`shape`] is the Unicode Standard's cursive joining, section 9.2's rules R1 to R7 and its
//!   obligatory lam-alef ligature, over `DerivedJoiningType.txt` and `ArabicShaping.txt`, with the
//!   positional forms reached as the Arabic Presentation Forms code points `UnicodeData.txt`
//!   decomposes to them (ADR 1414).
//! - [`Label`] is both over one line of an interface's own text, whose direction is its own
//!   (ADR 1417).
//!
//! Every table is compiled in by `build.rs` from `data/unicode/`, so nothing is parsed at launch.
//! **Content a document positioned is never passed through here**: its glyphs are where the
//! producer put them, and `doc/stack.md`'s reason for having no shaper is still the reason.

mod bidi;
pub mod face;
mod joining;
mod label;

pub use bidi::{Paragraphs, mirrored, visual_order};
pub use joining::{JoiningType, Position, Shaped, joining_type, positions, shape};
pub use label::{Glyph, Label};

/// The character UAX #9's rule L4 displays for `character`, which starts at byte `at` of the
/// text `paragraphs` resolved.
///
/// Its mirror image where the character resolved to an odd level and `BidiMirroring.txt` names
/// one, and the character itself otherwise — including wherever `paragraphs` is `None`, which
/// is a text every character of which is left to right.
#[must_use]
pub fn displayed(character: char, at: usize, paragraphs: Option<&Paragraphs>) -> char {
    paragraphs
        .filter(|paragraphs| paragraphs.level(at) % 2 == 1)
        .and_then(|_| mirrored(character))
        .unwrap_or(character)
}

/// Every character a text displays once shaped and mirrored, each with the stored characters it
/// displays — a form its letter, a lam-alef ligature its two, a mirrored bracket the one written.
///
/// The set a face has to cover to draw the text, and the meaning §9.10.2's first method should
/// read back from each glyph. Line breaks are not in it, because they are not drawn. `None`
/// where a letter's joined form has no presentation form (a [`Shaped::unformed`]), because no
/// face reached by character can then draw the text as the joining rules shape it.
#[must_use]
pub fn displayed_characters(
    text: &str,
    paragraphs: Option<&Paragraphs>,
) -> Option<std::collections::BTreeMap<char, String>> {
    let characters: Vec<(usize, char)> = text.char_indices().collect();
    let letters: Vec<char> = characters.iter().map(|(_, character)| *character).collect();
    let mut out = std::collections::BTreeMap::new();
    for item in shape(&letters) {
        if item.unformed {
            return None;
        }
        let Some(&(at, original)) = characters.get(item.source) else {
            continue;
        };
        if matches!(original, '\r' | '\n') {
            continue;
        }
        let mut meaning = String::from(original);
        if let Some(&(_, second)) = item.joined_with.and_then(|index| characters.get(index)) {
            meaning.push(second);
        }
        out.entry(displayed(item.character, at, paragraphs))
            .or_insert(meaning);
    }
    Some(out)
}

/// The tables `build.rs` writes from `data/unicode/`.
mod tables {
    include!(concat!(env!("OUT_DIR"), "/unicode.rs"));
}
