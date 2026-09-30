//! Cursive joining: which positional form each character of a cursive script takes.
//!
//! The Unicode Standard, section 9.2, states it as seven rules over a character's
//! `Joining_Type` and the `Joining_Type` of the nearest non-transparent characters on either side,
//! and then three rules forming the obligatory lam-alef ligature. Its "right" and "left" are
//! *visual*, and in a right-to-left script the character on the visual right is the one before
//! in logical order — so everything here walks the text in logical order and reads "right" as
//! *before* and "left" as *after*.
//!
//! The forms are reached as code points: `UnicodeData.txt` gives each Arabic Presentation Forms
//! character a compatibility decomposition tagged `<isolated>`, `<final>`, `<initial>` or
//! `<medial>` to the character it is a form of, and that inverse is [`shape`]'s output. A face
//! that maps those code points draws the forms; ADR 1414 has why that is the route taken here
//! and what reaching a face's own `GSUB` instead would add.

use super::tables::{
    ALEF_GROUP, JOINING_TYPES, LAM_GROUP, PRESENTATION_FORMS, PRESENTATION_LIGATURES,
};

/// A character's `Joining_Type`, from `DerivedJoiningType.txt`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoiningType {
    /// `U`: joins on neither side, and every character the file does not list.
    NonJoining,
    /// `R`: joins to the character on its right, which is the one before it.
    Right,
    /// `L`: joins to the character on its left, which is the one after it.
    Left,
    /// `D`: joins on both sides.
    Dual,
    /// `C`: causes joining on both sides without changing form itself, as the tatweel does.
    JoinCausing,
    /// `T`: invisible to joining — rule R1 has a transparent character leave the joining of the
    /// spacing characters around it as though it were not there.
    Transparent,
}

impl JoiningType {
    /// Table 9-4's right join-causing: dual-joining, left-joining or join-causing.
    ///
    /// A character of this type before `X` gives `X` something to join to on its right.
    const fn causes_right_join(self) -> bool {
        matches!(self, Self::Dual | Self::Left | Self::JoinCausing)
    }

    /// Table 9-4's left join-causing: dual-joining, right-joining or join-causing.
    const fn causes_left_join(self) -> bool {
        matches!(self, Self::Dual | Self::Right | Self::JoinCausing)
    }
}

/// The `Joining_Type` of one character.
#[must_use]
pub fn joining_type(character: char) -> JoiningType {
    let found = JOINING_TYPES.binary_search_by(|(first, last, _)| {
        if *last < character {
            std::cmp::Ordering::Less
        } else if *first > character {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    });
    match found.ok().and_then(|at| JOINING_TYPES.get(at)) {
        Some((_, _, b'R')) => JoiningType::Right,
        Some((_, _, b'L')) => JoiningType::Left,
        Some((_, _, b'D')) => JoiningType::Dual,
        Some((_, _, b'C')) => JoiningType::JoinCausing,
        Some((_, _, b'T')) => JoiningType::Transparent,
        _ => JoiningType::NonJoining,
    }
}

/// The positional form rules R2 to R7 choose, in the notation of Table 9-5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    /// `Xn`, the non-joining form, which rule R7 gives a character no other rule reaches.
    Isolated,
    /// `Xr`, joined on the right only: the last letter of a joined run.
    Final,
    /// `Xl`, joined on the left only: the first.
    Initial,
    /// `Xm`, joined on both sides.
    Medial,
}

impl Position {
    /// The number `build.rs` wrote for this position in the presentation-form tables.
    const fn table_code(self) -> u8 {
        match self {
            Self::Isolated => 0,
            Self::Final => 1,
            Self::Initial => 2,
            Self::Medial => 3,
        }
    }
}

/// The form rules R1 to R7 give each character of a run in logical order.
///
/// `None` for a character that takes no form of its own: a non-joining, join-causing or
/// transparent one. The run is read as it stands, so a caller that has line breaks in it gets
/// them as non-joining characters, which is what they are.
#[must_use]
pub fn positions(text: &[char]) -> Vec<Option<Position>> {
    let types: Vec<JoiningType> = text
        .iter()
        .map(|character| joining_type(*character))
        .collect();
    // The nearest non-transparent neighbour on each side (R1), found in one pass each way.
    let mut before = vec![JoiningType::NonJoining; text.len()];
    let mut seen = JoiningType::NonJoining;
    for (slot, kind) in before.iter_mut().zip(&types) {
        *slot = seen;
        if *kind != JoiningType::Transparent {
            seen = *kind;
        }
    }
    let mut after = vec![JoiningType::NonJoining; text.len()];
    seen = JoiningType::NonJoining;
    for (slot, kind) in after.iter_mut().zip(&types).rev() {
        *slot = seen;
        if *kind != JoiningType::Transparent {
            seen = *kind;
        }
    }
    types
        .iter()
        .zip(before.iter().zip(&after))
        .map(|(kind, (before, after))| {
            let right = before.causes_right_join();
            let left = after.causes_left_join();
            match kind {
                // R2, and R7 where it does not apply.
                JoiningType::Right => Some(if right {
                    Position::Final
                } else {
                    Position::Isolated
                }),
                // R3.
                JoiningType::Left => Some(if left {
                    Position::Initial
                } else {
                    Position::Isolated
                }),
                // R4, R5 and R6.
                JoiningType::Dual => Some(match (right, left) {
                    (true, true) => Position::Medial,
                    (true, false) => Position::Final,
                    (false, true) => Position::Initial,
                    (false, false) => Position::Isolated,
                }),
                JoiningType::NonJoining | JoiningType::JoinCausing | JoiningType::Transparent => {
                    None
                }
            }
        })
        .collect()
}

/// One character of shaped text, and where in the input it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shaped {
    /// The character to draw: a presentation form, or the input's own character where it takes
    /// no form or its nominal glyph is the form wanted.
    pub character: char,
    /// The index into the input of the character this one draws — for a ligature, of its first.
    pub source: usize,
    /// For a ligature, the index into the input of its second character, which draws nothing
    /// of its own.
    pub joined_with: Option<usize>,
    /// Whether the form rules R2 to R6 chose has no presentation form, so that
    /// [`Self::character`] is the nominal glyph where a joined one was wanted.
    ///
    /// A caller drawing the text decides what that is worth; drawing it anyway would be a letter
    /// in the wrong shape with nothing saying so.
    pub unformed: bool,
}

/// The presentation form `UnicodeData.txt` decomposes to `base` under `position`'s tag.
fn presentation_form(base: char, position: Position) -> Option<char> {
    let key = (base, position.table_code());
    PRESENTATION_FORMS
        .binary_search_by(|(candidate, form, _)| (*candidate, *form).cmp(&key))
        .ok()
        .and_then(|at| PRESENTATION_FORMS.get(at))
        .map(|(_, _, form)| *form)
}

/// The lam-alef ligature for one pair, in the position the ligature rules give it.
fn ligature(lam: char, alef: char, position: Position) -> Option<char> {
    let key = (lam, alef, position.table_code());
    PRESENTATION_LIGATURES
        .binary_search_by(|(first, second, form, _)| (*first, *second, *form).cmp(&key))
        .ok()
        .and_then(|at| PRESENTATION_LIGATURES.get(at))
        .map(|(_, _, _, form)| *form)
}

/// Shapes a run in logical order: the joining forms, and the lam-alef ligature where it forms.
///
/// The ligature rules follow the joining rules, as the standard orders them: an `ALEF r` whose
/// right neighbour — marks aside, rule L1 — is a `LAM m` becomes `(LAM-ALEF) r`, and one whose
/// right neighbour is a `LAM l` becomes `(LAM-ALEF) n`. The standard makes the ligature
/// obligatory where the style of the font supports it, and the pairs outside the four
/// `UnicodeData.txt` gives a presentation form are drawn as the two joined letters, which is how
/// a style without that ligature draws them. A mark between the two is kept, after the
/// ligature, which is where rule L1's own example puts it.
#[must_use]
pub fn shape(text: &[char]) -> Vec<Shaped> {
    let forms = positions(text);
    let mut out = Vec::with_capacity(text.len());
    let mut consumed = vec![false; text.len()];
    for (index, (&character, form)) in text.iter().zip(&forms).enumerate() {
        if consumed.get(index).copied().unwrap_or(false) {
            continue;
        }
        let Some(form) = *form else {
            out.push(Shaped {
                character,
                source: index,
                joined_with: None,
                unformed: false,
            });
            continue;
        };
        if let Some((alef_at, drawn)) = lam_alef(text, &forms, index, form) {
            out.push(Shaped {
                character: drawn,
                source: index,
                joined_with: Some(alef_at),
                unformed: false,
            });
            if let Some(slot) = consumed.get_mut(alef_at) {
                *slot = true;
            }
            continue;
        }
        // The nominal character is the isolated form: R7 names it `Xn`, and it is the glyph a
        // face maps the character itself to.
        let formed = match form {
            Position::Isolated => Some(character),
            joined => presentation_form(character, joined),
        };
        out.push(Shaped {
            character: formed.unwrap_or(character),
            source: index,
            joined_with: None,
            unformed: formed.is_none(),
        });
    }
    out
}

/// The alef a lam at `index` ligates with, and the ligature, where rules L2 and L3 form one.
fn lam_alef(
    text: &[char],
    forms: &[Option<Position>],
    index: usize,
    form: Position,
) -> Option<(usize, char)> {
    let lam = *text.get(index)?;
    if LAM_GROUP.binary_search(&lam).is_err() {
        return None;
    }
    let position = match form {
        Position::Medial => Position::Final,
        Position::Initial => Position::Isolated,
        Position::Isolated | Position::Final => return None,
    };
    let (alef_at, &alef) = text
        .iter()
        .enumerate()
        .skip(index.saturating_add(1))
        .find(|(_, character)| joining_type(**character) != JoiningType::Transparent)?;
    if ALEF_GROUP.binary_search(&alef).is_err() || forms.get(alef_at)? != &Some(Position::Final) {
        return None;
    }
    Some((alef_at, ligature(lam, alef, position)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shaped(text: &str) -> Vec<char> {
        let characters: Vec<char> = text.chars().collect();
        shape(&characters)
            .iter()
            .map(|shaped| shaped.character)
            .collect()
    }

    /// The standard's own examples under rules R1 to R6, with each form's code point read from
    /// `UnicodeData.txt`: TATWEEL is join-causing and takes no form, MEEM is dual-joining (FEE1
    /// isolated, FEE2 final, FEE3 initial, FEE4 medial), ALEF right-joining (FE8D, FE8E), LAM
    /// dual-joining (FEDD to FEE0) and SHADDA transparent.
    #[test]
    fn the_standards_joining_examples_take_the_forms_it_names() {
        // R2: ALEF n + TATWEEL n becomes ALEF r + TATWEEL n.
        assert_eq!(shaped("\u{640}\u{627}"), ['\u{640}', '\u{fe8e}']);
        // R4: TATWEEL n + MEEM n + TATWEEL n becomes TATWEEL n + MEEM m + TATWEEL n.
        assert_eq!(
            shaped("\u{640}\u{645}\u{640}"),
            ['\u{640}', '\u{fee4}', '\u{640}']
        );
        // R5: MEEM n + TATWEEL n becomes MEEM r + TATWEEL n.
        assert_eq!(shaped("\u{640}\u{645}"), ['\u{640}', '\u{fee2}']);
        // R6: TATWEEL n + MEEM n becomes TATWEEL n + MEEM l.
        assert_eq!(shaped("\u{645}\u{640}"), ['\u{fee3}', '\u{640}']);
        // R1: MEEM n + SHADDA n + LAM n becomes MEEM r + SHADDA n + LAM l — visual order, so
        // logically LAM, SHADDA, MEEM: the shadda does not stop the lam and the meem joining.
        assert_eq!(
            shaped("\u{644}\u{651}\u{645}"),
            ['\u{fedf}', '\u{651}', '\u{fee2}']
        );
        // R7: a letter alone is its nominal character.
        assert_eq!(shaped("\u{645}"), ['\u{645}']);
    }

    /// Rules L2 and L3: the ligature takes the lam's side of the join, and a mark between the two
    /// letters stays after it (rule L1's own example).
    #[test]
    fn a_lam_before_an_alef_is_the_obligatory_ligature() {
        // LAM l + ALEF r, nothing before the lam: (LAM-ALEF) n, FEFB.
        assert_eq!(shaped("\u{644}\u{627}"), ['\u{fefb}']);
        // A dual-joining BEH before it makes the lam medial: (LAM-ALEF) r, FEFC, and the beh
        // initial, FE91.
        assert_eq!(shaped("\u{628}\u{644}\u{627}"), ['\u{fe91}', '\u{fefc}']);
        // ALEF WITH MADDA ABOVE is of the ALEF joining group too: FEF5 and FEF6.
        assert_eq!(shaped("\u{644}\u{622}"), ['\u{fef5}']);
        // Rule L1: ALEF r + FATHA n + LAM l gives (LAM-ALEF) n + FATHA n.
        assert_eq!(shaped("\u{644}\u{64e}\u{627}"), ['\u{fefb}', '\u{64e}']);
        // The source indices name the lam, and the alef as the ligature's second character.
        let characters: Vec<char> = "\u{644}\u{64e}\u{627}".chars().collect();
        let first = shape(&characters)[0];
        assert_eq!((first.source, first.joined_with), (0, Some(2)));
    }

    /// `سلام` — SEEN, LAM, ALEF, MEEM — worked from the tables: the seen is followed by a
    /// dual-joining lam, so it is initial (FEB3); the lam is between it and a right-joining alef,
    /// so medial, and the pair is (LAM-ALEF) r (FEFC); the alef does not join on its left, so the
    /// meem after it is isolated, its nominal character.
    #[test]
    fn a_word_is_joined_and_ligated_as_the_tables_state() {
        assert_eq!(
            shaped("\u{633}\u{644}\u{627}\u{645}"),
            ['\u{feb3}', '\u{fefc}', '\u{645}']
        );
    }

    /// A space is non-joining, so it ends a join, and a hamza is non-joining too (Table 9-3).
    #[test]
    fn a_non_joining_character_breaks_the_join() {
        assert_eq!(shaped("\u{645} \u{645}"), ['\u{645}', ' ', '\u{645}']);
        assert_eq!(
            shaped("\u{645}\u{621}\u{645}"),
            ['\u{645}', '\u{621}', '\u{645}']
        );
    }

    /// A letter that `UnicodeData.txt` gives no presentation form in a joined position is marked
    /// rather than passed as though its nominal glyph were that form: U+0860 SYRIAC LETTER
    /// MALAYALAM NGA is dual-joining and neither presentation-form block has a form of it.
    #[test]
    fn a_form_no_presentation_block_holds_is_marked_unformed() {
        let characters = ['\u{860}', '\u{860}'];
        let out = shape(&characters);
        assert!(out.iter().all(|shaped| shaped.unformed), "{out:?}");
    }
}
