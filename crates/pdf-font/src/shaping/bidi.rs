//! Unicode Standard Annex #9: the order a line of Unicode text is displayed in.
//!
//! Resolution — rules P2 and P3 for a paragraph's level, the explicit rules X1 to X10, the weak
//! types W1 to W7, the neutrals N0 to N2 and the implicit I1 and I2 — is Servo's `unicode-bidi`,
//! taken rather than written because it passes every line of the UCD's `BidiCharacterTest.txt`
//! (ADR 1413). What depends on where a line *ends* is done here, per line, because only the
//! layout knows that: rule L1 resets the separators and the trailing white space, and
//! [`visual_order`] is rule L2. Rule L4 is [`mirrored`]'s data. Rule L3 is not applied, and ADR
//! 1413 says why: the annex leaves it to what the font's marks expect, and the faces this is for
//! draw a right-to-left mark over the base that follows it in display order.

use std::ops::Range;

use unicode_bidi::{BidiClass, BidiInfo, Level};

use super::tables::MIRRORING;

/// A value's paragraphs, resolved.
#[derive(Debug)]
pub struct Paragraphs<'text> {
    info: BidiInfo<'text>,
}

impl<'text> Paragraphs<'text> {
    /// Resolves `text`, each paragraph's direction found by rules P2 and P3.
    ///
    /// `None` where every character resolves to level 0, which is the case in which the display
    /// order is the stored order and nothing needs asking again.
    #[must_use]
    pub fn new(text: &'text str) -> Option<Self> {
        Self::with_direction(text, None)
    }

    /// Resolves `text` under a stated paragraph direction, `Some(true)` for right to left; `None`
    /// finds it by rules P2 and P3.
    ///
    /// The stated direction is what the conformance file's cases carry. ISO 32000-2 gives a field
    /// value no direction of its own, so the layout asks with `None`.
    #[must_use]
    pub fn with_direction(text: &'text str, right_to_left: Option<bool>) -> Option<Self> {
        let level = right_to_left.map(|rtl| if rtl { Level::rtl() } else { Level::ltr() });
        let info = BidiInfo::new(text, level);
        let resolved = info.levels.iter().any(|level| level.number() > 0);
        (resolved || right_to_left == Some(true)).then_some(Self { info })
    }

    /// The level of every byte of `line` after rule L1, the separators and the white space
    /// before them or at the end of the line set back to their paragraph's level.
    ///
    /// `line` is a byte range of the text on character boundaries. A line holding more than one
    /// paragraph — a value can carry U+2029 in the middle of what a layout shows as one line —
    /// is reset paragraph by paragraph, because L1's level is each paragraph's own.
    #[must_use]
    pub fn line_levels(&self, line: Range<usize>) -> Vec<u8> {
        let levels = &self.info.levels;
        let classes = &self.info.original_classes;
        let end = line.end.min(levels.len());
        let start = line.start.min(end);
        let mut out: Vec<u8> = levels
            .get(start..end)
            .unwrap_or_default()
            .iter()
            .map(Level::number)
            .collect();
        for paragraph in &self.info.paragraphs {
            let from = paragraph.range.start.max(start);
            let to = paragraph.range.end.min(end);
            if from >= to {
                continue;
            }
            let (Some(slice), Some(kinds)) = (
                out.get_mut(from.saturating_sub(start)..to.saturating_sub(start)),
                classes.get(from..to),
            ) else {
                continue;
            };
            reset_line(slice, kinds, paragraph.level.number());
        }
        out
    }

    /// The level the character starting at `byte` resolved to, before rule L1.
    ///
    /// What rule L4 reads: a character is displayed mirrored where this is odd.
    #[must_use]
    pub fn level(&self, byte: usize) -> u8 {
        self.info.levels.get(byte).map_or(0, Level::number)
    }

    /// The level of the paragraph holding `byte`: the one P2 and P3 found, or the one stated.
    #[must_use]
    pub fn paragraph_level(&self, byte: usize) -> u8 {
        self.info
            .paragraphs
            .iter()
            .find(|paragraph| paragraph.range.contains(&byte))
            .map_or(0, |paragraph| paragraph.level.number())
    }

    /// Whether the character starting at `byte` is one rule X9 removes: an embedding or override
    /// control, or a boundary neutral.
    ///
    /// Such a character keeps a level here — the one before it, as the annex's section on
    /// retaining explicit formatting characters suggests — so a layout that draws whatever glyph
    /// a face gives it draws it somewhere sensible.
    #[must_use]
    pub fn removed_by_x9(&self, byte: usize) -> bool {
        use BidiClass::{BN, LRE, LRO, PDF, RLE, RLO};
        matches!(
            self.info.original_classes.get(byte),
            Some(RLE | LRE | RLO | LRO | PDF | BN)
        )
    }
}

/// Rule L1 over one line of one paragraph, by byte.
///
/// Segment and paragraph separators take the paragraph level, as does any run of white space
/// or isolate controls before one of them or at the line's end; the classes are the original
/// ones, as the rule requires. A character X9 removed takes the level before it and does not
/// break a run of white space.
fn reset_line(levels: &mut [u8], classes: &[BidiClass], paragraph: u8) {
    use BidiClass::{B, BN, FSI, LRE, LRI, LRO, PDF, PDI, RLE, RLI, RLO, S, WS};
    let mut run_start: Option<usize> = None;
    let mut previous = paragraph;
    for at in 0..levels.len().min(classes.len()) {
        let class = classes.get(at).copied().unwrap_or(BidiClass::L);
        match class {
            B | S => {
                let from = run_start.take().unwrap_or(at);
                if let Some(run) = levels.get_mut(from..=at) {
                    run.fill(paragraph);
                }
            }
            WS | FSI | LRI | RLI | PDI => {
                run_start.get_or_insert(at);
            }
            RLE | LRE | RLO | LRO | PDF | BN => {
                run_start.get_or_insert(at);
                if let Some(level) = levels.get_mut(at) {
                    *level = previous;
                }
            }
            _ => run_start = None,
        }
        previous = levels.get(at).copied().unwrap_or(paragraph);
    }
    if let Some(from) = run_start
        && let Some(run) = levels.get_mut(from..)
    {
        run.fill(paragraph);
    }
}

/// Rule L2: the order to display a line's items in, as indices into `levels`.
///
/// From the highest level to the lowest odd one, every maximal run at that level or above is
/// reversed. The items may be characters, bytes or glyphs; the rule is the same over any
/// sequence carrying a level each.
#[must_use]
pub fn visual_order(levels: &[u8]) -> Vec<usize> {
    let levels: Vec<Level> = levels
        .iter()
        .map(|level| Level::new(*level).unwrap_or_else(|_| Level::rtl()))
        .collect();
    BidiInfo::reorder_visual(&levels)
}

/// Rule L4's data: the character whose glyph is `character`'s mirror image, where
/// `BidiMirroring.txt` states one.
///
/// A character resolved to an odd level with a `Bidi_Mirroring_Glyph` is drawn with that glyph —
/// an opening parenthesis in a right-to-left run is drawn as a closing one, so that it still
/// opens in the direction the run reads.
#[must_use]
pub fn mirrored(character: char) -> Option<char> {
    MIRRORING
        .binary_search_by(|(from, _)| from.cmp(&character))
        .ok()
        .and_then(|at| MIRRORING.get(at))
        .map(|(_, to)| *to)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A line of Arabic with a full stop and a trailing space: the paragraph is right to left by
    /// P2 and P3, every letter resolves to 1, and L1 sets the trailing space back to the
    /// paragraph's level, which is also 1 — so L2 reverses the whole line.
    #[test]
    fn an_arabic_line_is_displayed_reversed() {
        let text = "\u{628}\u{627}. ";
        let paragraphs = Paragraphs::new(text).expect("the text has right-to-left characters");
        let levels = paragraphs.line_levels(0..text.len());
        assert!(levels.iter().all(|level| *level == 1), "{levels:?}");
        let characters: Vec<u8> = text.char_indices().map(|(at, _)| levels[at]).collect();
        assert_eq!(visual_order(&characters), [3, 2, 1, 0]);
    }

    /// Latin text resolves to nothing right to left, so there is nothing to reorder.
    #[test]
    fn a_left_to_right_value_needs_no_reordering() {
        assert!(Paragraphs::new("Invoice 42 (draft)").is_none());
    }

    /// L1 at the end of a line in a left-to-right paragraph: the white space after an Arabic word
    /// goes back to level 0, so it is displayed at the line's end rather than inside the word.
    #[test]
    fn trailing_white_space_takes_the_paragraph_level() {
        let text = "a \u{628}\u{628} ";
        let paragraphs = Paragraphs::new(text).expect("the text has right-to-left characters");
        let levels = paragraphs.line_levels(0..text.len());
        assert_eq!(levels.last(), Some(&0));
        assert_eq!(levels.get(2), Some(&1));
    }

    /// L4's data for the pairs its own example names.
    #[test]
    fn a_parenthesis_mirrors_to_its_partner() {
        assert_eq!(mirrored('('), Some(')'));
        assert_eq!(mirrored(')'), Some('('));
        assert_eq!(mirrored('a'), None);
    }
}
