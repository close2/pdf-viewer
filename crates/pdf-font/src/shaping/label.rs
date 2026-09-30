//! One line of an interface's own text — an outline item's title, a tab's file name, a page label
//! — joined and ordered for display.
//!
//! A field's value is laid out by `pdf-model`'s variable text, which breaks it into lines inside a
//! box the document drew. A label is not: the window decides where it goes, and it is one line.
//! So this is the same two answers [`super::shape`] and [`super::Paragraphs`] give a field, over
//! the whole label at once, with nothing about lines or boxes in it:
//!
//! - **The direction a label reads in is its own**, found by UAX #9's rules P2 and P3 from its
//!   first strong character, and never the window's or the document's. ISO 32000-2 §12.3.3 has a
//!   title be "[t]he text that shall be displayed on the screen for this item" and states nothing
//!   about its direction; §14.9.2.1's `/Lang` names a natural language, which is not a direction.
//!   ADR 1417 has the choice and its argument.
//! - Rule L1 treats the whole label as one line, since that is what it is drawn as; L2 orders it;
//!   L4 mirrors a character at an odd level.
//! - Joining is the Unicode Standard's section 9.2, reached as presentation forms (ADR 1414), and
//!   computed before the order, since joining is stated in logical order.

use std::ops::Range;

use super::{Paragraphs, displayed, shape, visual_order};

/// One glyph of a label, in the order it is displayed from the left.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Glyph {
    /// The character to draw: a presentation form, a mirrored bracket, or the stored character.
    pub character: char,
    /// The bytes of the stored text it draws, which for a lam-alef ligature are both letters'.
    pub stored: Range<usize>,
    /// Whether it resolved to an odd level, so that its logical start is its right edge.
    pub right_to_left: bool,
    /// Whether its letter wanted a joined form that has no presentation form, and is drawn in its
    /// nominal shape ([`super::Shaped::unformed`]).
    pub unformed: bool,
}

/// A label, shaped and in display order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    /// The glyphs, left to right on the screen.
    glyphs: Vec<Glyph>,
    /// For each glyph in *logical* order, where it is displayed.
    position: Vec<usize>,
    /// Whether the label's paragraph is right to left by rules P2 and P3.
    right_to_left: bool,
}

impl Label {
    /// Joins and orders `text` for display as one line.
    ///
    /// Line breaks in `text` are displayed like any other character; a caller that draws a label
    /// on one line replaces them first, which is what a label's line is.
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self::line_of(text, 0..text.len())
    }

    /// Joins and orders one line of a paragraph a layout has broken across several.
    ///
    /// UAX #9 resolves a *paragraph*: rules P2 and P3 find its direction from its first strong
    /// character, and the weak and neutral types are resolved against their neighbours in it.
    /// Only rules L1 to L4 are a line's. So the levels are resolved over the whole of `paragraph`
    /// and `line` — a byte range of it on character boundaries — takes the ones its characters
    /// resolved to: a wrapped line reads in its paragraph's direction even where it begins with a
    /// word of the other one. Joining is over the line's characters, which is what is drawn
    /// together. [`Glyph::stored`] and [`Self::boundary`] count bytes of the line, not of the
    /// paragraph.
    #[must_use]
    pub fn line_of(paragraph: &str, line: Range<usize>) -> Self {
        let text = paragraph.get(line.clone()).unwrap_or_default();
        let offset = line.start;
        let characters: Vec<(usize, char)> = text.char_indices().collect();
        let letters: Vec<char> = characters.iter().map(|(_, character)| *character).collect();
        let shaped = shape(&letters);
        let paragraphs = Paragraphs::new(paragraph);
        let by_byte = paragraphs.as_ref().map(|paragraphs| {
            paragraphs.line_levels(line.start..line.start.saturating_add(text.len()))
        });
        let byte_of = |index: usize| characters.get(index).map_or(text.len(), |(at, _)| *at);
        let end_of = |index: usize| {
            characters.get(index).map_or(text.len(), |(at, character)| {
                at.saturating_add(character.len_utf8())
            })
        };
        let levels: Vec<u8> = shaped
            .iter()
            .map(|item| {
                by_byte
                    .as_ref()
                    .and_then(|levels| levels.get(byte_of(item.source)).copied())
                    .unwrap_or(0)
            })
            .collect();
        let order = if levels.iter().any(|level| *level > 0) {
            visual_order(&levels)
        } else {
            (0..shaped.len()).collect()
        };
        let mut glyphs = Vec::with_capacity(shaped.len());
        let mut position = vec![0; shaped.len()];
        for (shown_at, &index) in order.iter().enumerate() {
            let Some(item) = shaped.get(index) else {
                continue;
            };
            if let Some(slot) = position.get_mut(index) {
                *slot = shown_at;
            }
            let start = byte_of(item.source);
            let end = end_of(item.joined_with.unwrap_or(item.source)).max(end_of(item.source));
            glyphs.push(Glyph {
                character: displayed(
                    item.character,
                    offset.saturating_add(start),
                    paragraphs.as_ref(),
                ),
                stored: start..end,
                right_to_left: levels.get(index).is_some_and(|level| level % 2 == 1),
                unformed: item.unformed,
            });
        }
        let right_to_left = paragraphs
            .as_ref()
            .is_some_and(|paragraphs| paragraphs.paragraph_level(offset) % 2 == 1);
        Self {
            glyphs,
            position,
            right_to_left,
        }
    }

    /// The glyphs, in the order they are displayed from the left.
    #[must_use]
    pub fn glyphs(&self) -> &[Glyph] {
        &self.glyphs
    }

    /// Whether the label reads right to left as a whole, by rules P2 and P3.
    #[must_use]
    pub const fn right_to_left(&self) -> bool {
        self.right_to_left
    }

    /// How many displayed glyphs stand left of a caret at byte `at` of the stored text.
    ///
    /// The convention `pdf-model`'s variable text uses for a field (ADR 1413): a boundary is drawn
    /// at the edge of the glyph after it on that glyph's own reading side, so a caret before a
    /// right-to-left letter stands at its right edge; a caret at the end stands at the last
    /// glyph's far side. A byte inside a ligature is the ligature's start.
    #[must_use]
    pub fn boundary(&self, at: usize) -> usize {
        let count = self.glyphs.len();
        let shown = |logical: usize| self.position.get(logical).copied().unwrap_or(count);
        let reads_left = |shown_at: usize| {
            self.glyphs
                .get(shown_at)
                .is_some_and(|glyph| glyph.right_to_left)
        };
        let after = (0..self.position.len()).find(|&logical| {
            self.glyphs
                .get(shown(logical))
                .is_some_and(|glyph| glyph.stored.end > at)
        });
        match after {
            Some(logical) => {
                let shown_at = shown(logical);
                shown_at.saturating_add(usize::from(reads_left(shown_at)))
            }
            None => match count.checked_sub(1) {
                Some(last) => {
                    let shown_at = shown(last);
                    shown_at.saturating_add(usize::from(!reads_left(shown_at)))
                }
                None => 0,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drawn(text: &str) -> Vec<char> {
        Label::new(text)
            .glyphs()
            .iter()
            .map(|glyph| glyph.character)
            .collect()
    }

    /// A Latin label is its own characters in its own order, and a caret at the end stands after
    /// the last of them.
    #[test]
    fn a_latin_label_is_unchanged() {
        let label = Label::new("Chapter 3");
        assert_eq!(drawn("Chapter 3"), "Chapter 3".chars().collect::<Vec<_>>());
        assert!(!label.right_to_left());
        assert_eq!(label.boundary("Chapter 3".len()), 9);
        assert_eq!(label.boundary(0), 0);
    }

    /// "سلام", worked by hand from the tables: seen dual-joining with a joiner after it is
    /// initial (U+FEB3); lam before alef forms the ligature, final because the seen joins to it
    /// (U+FEFC, rule L3 with R3's final form); meem after alef, which joins on its right side only,
    /// is isolated, which is its nominal character. Displayed right to left, so the meem is
    /// leftmost.
    #[test]
    fn an_arabic_word_is_joined_and_displayed_right_to_left() {
        let text = "\u{633}\u{644}\u{627}\u{645}";
        assert_eq!(drawn(text), ['\u{645}', '\u{FEFC}', '\u{FEB3}']);
        let label = Label::new(text);
        assert!(label.right_to_left());
        // The ligature draws both the lam and the alef.
        assert_eq!(label.glyphs()[1].stored, 2..6);
        // A caret at the end of a right-to-left label stands at its left edge.
        assert_eq!(label.boundary(text.len()), 0);
        // And one at the start at its right edge.
        assert_eq!(label.boundary(0), 3);
    }

    /// A number inside a right-to-left label: the paragraph is right to left by P2 and P3 from
    /// the beh, the digits resolve to level 2 and keep their own order, so "12" reads left to
    /// right at the label's left end and the beh, isolated and so nominal, stands at its right.
    #[test]
    fn a_number_in_an_arabic_label_keeps_its_own_order() {
        assert_eq!(drawn("\u{628} 12"), ['1', '2', ' ', '\u{628}']);
    }

    /// A label's direction is its own first strong character's, never the window's: a Latin word
    /// first makes a left-to-right paragraph in which the Hebrew run is reversed in place.
    #[test]
    fn the_direction_is_the_labels_first_strong_character() {
        assert!(!Label::new("Intro \u{5E9}\u{5DC}").right_to_left());
        assert_eq!(
            drawn("Intro \u{5E9}\u{5DC}"),
            ['I', 'n', 't', 'r', 'o', ' ', '\u{5DC}', '\u{5E9}']
        );
        assert!(Label::new("\u{5E9}\u{5DC} Intro").right_to_left());
    }

    /// A wrapped line takes its paragraph's direction and not its own: "Intro של עו." broken
    /// after "Intro" leaves a second line that begins with Hebrew, and that line is still part of
    /// a left-to-right paragraph by P2 and P3 — so its Hebrew is one run reversed in place and the
    /// full stop, a neutral between that run and the paragraph's end, takes the paragraph's level
    /// and stays at the line's right. Laid out alone, the same line would be right to left with
    /// its full stop at the left.
    #[test]
    fn a_wrapped_line_reads_in_its_paragraphs_direction() {
        let paragraph = "Intro \u{5E9}\u{5DC} \u{5E2}\u{5D5}.";
        let second = "Intro ".len()..paragraph.len();
        let line = Label::line_of(paragraph, second.clone());
        assert!(!line.right_to_left(), "the paragraph is left to right");
        let alone = Label::new(&paragraph[second]);
        assert!(alone.right_to_left());
        let drawn = |label: &Label| -> Vec<char> {
            label.glyphs().iter().map(|glyph| glyph.character).collect()
        };
        assert_eq!(
            drawn(&line),
            ['\u{5D5}', '\u{5E2}', ' ', '\u{5DC}', '\u{5E9}', '.']
        );
        assert_eq!(
            drawn(&alone),
            ['.', '\u{5D5}', '\u{5E2}', ' ', '\u{5DC}', '\u{5E9}']
        );
        assert_eq!(
            line.glyphs()[0].stored,
            7..9,
            "bytes of the line, not the paragraph"
        );
    }

    /// And the other way round: an Arabic paragraph whose second line begins with a Latin word
    /// is still right to left on that line, so the Latin word stands at the line's right end —
    /// where the line begins — rather than at its left.
    #[test]
    fn a_wrapped_line_of_an_arabic_paragraph_stays_right_to_left() {
        let paragraph = "\u{628}\u{627}\u{628} PDF \u{628}";
        let second = "\u{628}\u{627}\u{628} ".len()..paragraph.len();
        let line = Label::line_of(paragraph, second.clone());
        assert!(line.right_to_left());
        assert!(!Label::new(&paragraph[second]).right_to_left());
        let drawn: Vec<char> = line.glyphs().iter().map(|glyph| glyph.character).collect();
        assert_eq!(drawn, ['\u{628}', ' ', 'P', 'D', 'F']);
    }

    /// Rule L4: a parenthesis in a right-to-left run is drawn as its mirror, so that it still
    /// opens in the direction the run reads.
    #[test]
    fn a_bracket_in_a_hebrew_label_is_mirrored() {
        // "(א)" after a Hebrew word: displayed "(א) word" read from the right, so the glyph at
        // the left is the stored ")" drawn as "(".
        let glyphs = drawn("\u{5DE} (\u{5D0})");
        assert_eq!(glyphs, ['(', '\u{5D0}', ')', ' ', '\u{5DE}']);
    }
}
