//! ISO 32000-2 §9.8.3.3's glyph classes, and which of them a character can be *shown* to be in.
//!
//! Table 122 makes `/FD` "[a] dictionary whose keys identify a class of glyphs in a `CIDFont`",
//! whose values "shall override the corresponding values in the main font descriptor dictionary
//! for that class of glyphs", and §9.8.3.3 says what a key may be:
//!
//! > The key for each entry in an FD dictionary shall be the name of a class of glyphs -that is,
//! > a particular subset of the `CIDFont`'s character collection.
//!
//! > The names of the glyph classes depend on the character collection, as identified by the
//! > Registry , Ordering , and Supplement entries in the `CIDSystemInfo` dictionary.
//!
//! # What this module does and does not know
//!
//! **Which CIDs a class holds is the character collection's to say, and that is registered data
//! published outside this standard** — the same boundary Table 116's predefined `CMap`s sit
//! behind (ADR 0007). Nothing here reads a CID.
//!
//! What it reads instead is the **character**, which for the one route where a descriptor decides
//! a glyph at all — a `CIDFont` whose program the document did not embed — is already in hand:
//! §9.7.4.2 says that with the program absent "CIDs shall not participate in glyph selection", so
//! a substitute is reached through §9.10.2's Unicode value and by nothing else. Table 123
//! describes each class by the characters its glyphs stand for — "Proportional Latin glyphs",
//! "Full-width hanzi (Chinese) glyphs", "Hangul and jamo glyphs" — and those descriptions are the
//! standard's own, so a character *can* be shown to be outside a class, and sometimes inside
//! exactly one of the classes a given file names.
//!
//! # And where it cannot decide, it refuses by name
//!
//! Three of Table 123's descriptions distinguish a class from another by the **form** of the
//! glyph rather than by the character it stands for, and a Unicode value is the same on both
//! sides of that distinction:
//!
//! - every `…Rot` class — "Same as `HRoman` but rotated for use in vertical writing";
//! - `Ruby`, "Glyphs used for setting ruby (small glyphs that serve to annotate other glyphs with
//!   meanings or readings)".
//!
//! Two more are described by something no Unicode value states at all: `Dingbats`, "Special
//! symbols", and `Generic`, "Typeface-independent glyphs, such as line-drawing".
//!
//! Those five are refused by name, always. So are a class Table 123 does not list for the
//! collection the file states, a class under an ordering Table 123 does not tabulate (every
//! `Identity` ordering), and — the case that makes the rest safe — **two stated classes that
//! could both hold the same character**, which is `Proportional` and `HRoman` stated together:
//! Table 123 separates "Proportional Latin glyphs" from "Half-width Latin glyphs" by a width the
//! character does not carry. A refusal keeps the main descriptor, which is what the font had
//! before `/FD` was read at all.
//!
//! **A class stated alone is not that ambiguity.** A file stating only `Proportional` has said
//! which of its glyphs these metrics are for, and the substitute this tree chooses is a
//! proportional Latin face whatever the class name: it has no half-width Latin glyphs to confuse
//! it with. What is refused is a file that states *both* and so asks a question the character
//! cannot answer.

/// What a character is, as far as Table 123's own descriptions divide characters.
///
/// Not a Unicode property and not meant to be one: each variant exists because some cell of
/// Table 123 names it, and a character belongs to at most one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Kind {
    /// A Latin digit, which Table 123 gives `AlphaNum` — "Numeric glyphs" — for.
    LatinDigit,
    /// Any other Latin character at its ordinary, neither full- nor half-width, code point.
    Latin,
    /// A full-width Latin form: Unicode's FULLWIDTH FORMS of the ASCII range.
    FullWidthLatin,
    /// A Greek or Cyrillic character, which Table 123 names only in `Alphabetic`.
    GreekOrCyrillic,
    /// Full-width hiragana or katakana.
    Kana,
    /// Half-width katakana, which Unicode encodes separately from the full-width forms.
    HalfWidthKana,
    /// A CJK ideograph — Table 123's hanzi, kanji and hanja.
    Ideograph,
    /// A hangul syllable or a jamo.
    Hangul,
}

/// Which characters Table 123 says a class holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Membership {
    /// The kinds of character the class's description names.
    Characters(&'static [Kind]),
    /// A class Table 123 separates from another by the *form* of its glyphs.
    ///
    /// §9.10.2's Unicode value is the same on both sides of such a distinction, so no character
    /// decides it.
    Form,
    /// A class Table 123 describes by something no Unicode value states.
    Undescribed,
}

/// Table 123's row for a character collection, or `None` where it tabulates none.
///
/// The clause makes the class names "depend on the character collection", so an ordering with no
/// row — `Identity`, and anything unregistered — leaves every key in the `/FD` dictionary a name
/// this reader has no meaning for.
fn tabulated(registry: &str, ordering: &str) -> Option<&'static [&'static str]> {
    /// Table 123's Adobe-GB1 and Adobe-CNS1 rows, which are the same nine names.
    const HAN: &[&str] = &[
        "Alphabetic",
        "Dingbats",
        "Generic",
        "Hanzi",
        "HRoman",
        "HRomanRot",
        "Kana",
        "Proportional",
        "ProportionalRot",
    ];
    /// Its Adobe-Korea1 and Adobe-KR row, which the table prints once for both.
    const KOREAN: &[&str] = &[
        "Alphabetic",
        "Dingbats",
        "Generic",
        "Hangul",
        "Hanja",
        "HRoman",
        "HRomanRot",
        "Kana",
        "Proportional",
        "ProportionalRot",
    ];
    if registry != "Adobe" {
        return None;
    }
    match ordering {
        "GB1" | "CNS1" => Some(HAN),
        "Japan1" => Some(&[
            "Alphabetic",
            "AlphaNum",
            "Dingbats",
            "DingbatsRot",
            "Generic",
            "GenericRot",
            "HKana",
            "HKanaRot",
            "HRoman",
            "HRomanRot",
            "Kana",
            "Kanji",
            "Proportional",
            "ProportionalRot",
            "Ruby",
        ]),
        "Japan2" => Some(&["Alphabetic", "Dingbats", "HojoKanji"]),
        "Korea1" | "KR" => Some(KOREAN),
        _ => None,
    }
}

/// What Table 123's description of a class says about the characters its glyphs stand for.
fn membership(class: &str) -> Membership {
    match class {
        // "Full-width Latin, Greek, and Cyrillic glyphs".
        "Alphabetic" => Membership::Characters(&[Kind::FullWidthLatin, Kind::GreekOrCyrillic]),
        // "Numeric glyphs".
        "AlphaNum" => Membership::Characters(&[Kind::LatinDigit]),
        // "Proportional Latin glyphs" and "Half-width Latin glyphs": one kind of character each,
        // and the same kind, which is why a file stating both is refused below.
        "Proportional" | "HRoman" => Membership::Characters(&[Kind::Latin, Kind::LatinDigit]),
        // "Japanese kana (katakana and hiragana) glyphs", which Adobe-Japan1's row calls
        // "Full-width kana (katakana and hiragana) glyphs".
        "Kana" => Membership::Characters(&[Kind::Kana]),
        // "Half-width kana (katakana and hiragana) glyphs".
        "HKana" => Membership::Characters(&[Kind::HalfWidthKana]),
        // "Full-width hanzi (Chinese) glyphs", "Full-width kanji (Chinese) glyphs",
        // "Full-width hanja (Chinese) glyphs" and Adobe-Japan2's "Full-width kanji glyphs".
        "Hanzi" | "Kanji" | "Hanja" | "HojoKanji" => Membership::Characters(&[Kind::Ideograph]),
        // "Hangul and jamo glyphs".
        "Hangul" => Membership::Characters(&[Kind::Hangul]),
        // "Special symbols" and "Typeface-independent glyphs, such as line-drawing".
        "Dingbats" | "Generic" => Membership::Undescribed,
        // "Same as … but rotated for use in vertical writing", and ruby's small annotating
        // glyphs: both are the same characters in another shape.
        _ => Membership::Form,
    }
}

/// Which of Table 123's kinds a character is, where the table's descriptions name one.
///
/// `None` is not a gap to be filled by a guess: it is a character none of Table 123's
/// descriptions picks out, so no class can be shown to hold it.
pub(crate) fn kind_of(character: char) -> Option<Kind> {
    let code = character as u32;
    match code {
        // Table 123 reaches ASCII twice over — "Proportional Latin glyphs" and "Numeric glyphs"
        // — and Unicode's FULLWIDTH FORMS are the third, "Full-width Latin … glyphs".
        0x30..=0x39 => Some(Kind::LatinDigit),
        // Basic Latin, then Latin-1 Supplement and the two Latin Extended blocks.
        0x20..=0x7E | 0xA1..=0x24F => Some(Kind::Latin),
        // Greek, Greek Extended, Cyrillic and its supplement.
        0x370..=0x3FF | 0x400..=0x52F | 0x1F00..=0x1FFF => Some(Kind::GreekOrCyrillic),
        // Hangul Jamo, Compatibility Jamo, Extended-A and the syllables.
        0x1100..=0x11FF | 0x3130..=0x318F | 0xA960..=0xA97F | 0xAC00..=0xD7FF => Some(Kind::Hangul),
        // Hiragana and katakana, both full-width blocks.
        0x3041..=0x30FF => Some(Kind::Kana),
        // CJK Unified Ideographs, its extension A, and the compatibility ideographs.
        // CJK Unified Ideographs, its extension A, the compatibility ideographs, and the
        // supplementary planes the later extensions sit in.
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x20000..=0x3134F => {
            Some(Kind::Ideograph)
        }
        0xFF01..=0xFF5E => Some(Kind::FullWidthLatin),
        // Halfwidth katakana, which Unicode encodes apart from the full-width forms above.
        0xFF61..=0xFF9F => Some(Kind::HalfWidthKana),
        _ => None,
    }
}

/// Whether a class's glyphs are the character collection's own script rather than Latin.
///
/// Which characters a face has to be able to draw is what [`crate::substituted::script_sample`]
/// decides for a whole font, and a class face is chosen the same way for the class's own glyphs:
/// Table 123 describes `Proportional`, `HRoman`, `AlphaNum` and `Alphabetic` by Latin, Greek and
/// Cyrillic, which a Latin face has, and the rest by kana, ideographs and hangul, which it does
/// not. Asking a proportional Latin class's face for 的 would refuse every Latin face on the
/// machine over a character that class never draws.
pub(crate) fn holds_the_collections_script(class: &str) -> bool {
    match membership(class) {
        Membership::Characters(kinds) => kinds.iter().any(|kind| {
            matches!(
                kind,
                Kind::Kana | Kind::HalfWidthKana | Kind::Ideograph | Kind::Hangul
            )
        }),
        Membership::Form | Membership::Undescribed => false,
    }
}

/// One `/FD` dictionary's keys, decided once against Table 123 and the collection.
#[derive(Debug, Default)]
pub(crate) struct Decision {
    /// Which stated class holds each kind of character, where exactly one of them does.
    ///
    /// An index into the list of classes the file stated, in the file's own order.
    assigned: Vec<(Kind, usize)>,
    /// Every class no glyph will be assigned to, and the sentence that says why.
    refused: Vec<(String, &'static str)>,
}

impl Decision {
    /// Decides a whole `/FD` dictionary: which classes can hold which characters, and which
    /// cannot be decided at all.
    ///
    /// `collection` is §9.7.3's `/CIDSystemInfo` registry and ordering, which §9.8.3.3 makes the
    /// class names depend on.
    pub(crate) fn read(stated: &[String], collection: Option<(&str, &str)>) -> Self {
        let Some((registry, ordering)) = collection else {
            return Self::all_refused(
                stated,
                "the descendant states no /CIDSystemInfo, which §9.8.3.3 makes the class names \
                 depend on",
            );
        };
        let Some(row) = tabulated(registry, ordering) else {
            return Self::all_refused(
                stated,
                "Table 123 tabulates no glyph classes for this character collection",
            );
        };

        let mut decision = Self::default();
        let mut holders: Vec<(Kind, usize)> = Vec::new();
        for (index, class) in stated.iter().enumerate() {
            if !row.contains(&class.as_str()) {
                decision.refused.push((
                    class.clone(),
                    "Table 123 does not list this class for the character collection the \
                     descendant states",
                ));
                continue;
            }
            match membership(class) {
                Membership::Characters(kinds) => {
                    for kind in kinds {
                        holders.push((*kind, index));
                    }
                }
                Membership::Form => decision.refused.push((
                    class.clone(),
                    "Table 123 separates this class from another by the form of its glyphs, and \
                     §9.10.2's Unicode value is the same for both",
                )),
                Membership::Undescribed => decision.refused.push((
                    class.clone(),
                    "Table 123 describes this class by something no Unicode value states",
                )),
            }
        }

        for (kind, index) in &holders {
            let contested = holders
                .iter()
                .any(|(other, elsewhere)| other == kind && elsewhere != index);
            if contested {
                let class = stated[*index].clone();
                if !decision.refused.iter().any(|(name, _)| *name == class) {
                    decision.refused.push((
                        class,
                        "two of this dictionary's classes hold the same kind of character, which \
                         no Unicode value tells apart",
                    ));
                }
                continue;
            }
            decision.assigned.push((*kind, *index));
        }
        decision
    }

    /// Every class refused for one reason, for the two cases that refuse the whole dictionary.
    fn all_refused(stated: &[String], reason: &'static str) -> Self {
        Self {
            assigned: Vec::new(),
            refused: stated.iter().map(|name| (name.clone(), reason)).collect(),
        }
    }

    /// Which of the stated classes a character belongs to, where Table 123 shows exactly one.
    pub(crate) fn class_for(&self, character: char) -> Option<usize> {
        let kind = kind_of(character)?;
        self.assigned
            .iter()
            .find(|(held, _)| *held == kind)
            .map(|(_, index)| *index)
    }

    /// The classes this reader declined to apply, each with the reason.
    pub(crate) fn refused(&self) -> &[(String, &'static str)] {
        &self.refused
    }
}

#[cfg(test)]
mod tests {
    use super::{Decision, Kind, kind_of};

    /// Table 123's descriptions, read as statements about characters.
    #[test]
    fn a_character_is_the_kind_table_123_describes() {
        for (character, kind) in [
            ('A', Some(Kind::Latin)),
            ('7', Some(Kind::LatinDigit)),
            ('\u{FF21}', Some(Kind::FullWidthLatin)),
            ('\u{0391}', Some(Kind::GreekOrCyrillic)),
            ('\u{0410}', Some(Kind::GreekOrCyrillic)),
            ('\u{3042}', Some(Kind::Kana)),
            ('\u{FF71}', Some(Kind::HalfWidthKana)),
            ('\u{7684}', Some(Kind::Ideograph)),
            ('\u{D55C}', Some(Kind::Hangul)),
            // "Special symbols" and "line-drawing" are not what a Unicode value states.
            ('\u{2500}', None),
            ('\u{2702}', None),
        ] {
            assert_eq!(kind_of(character), kind, "for {character:?}");
        }
    }

    /// A class stated alone takes the characters Table 123 describes it by, and no others.
    #[test]
    fn one_stated_class_takes_the_characters_table_123_gives_it() {
        let stated = vec!["Proportional".to_owned()];
        let decision = Decision::read(&stated, Some(("Adobe", "Japan1")));
        assert_eq!(decision.class_for('A'), Some(0));
        assert_eq!(decision.class_for('7'), Some(0));
        // A kanji is not a proportional Latin glyph, so the main descriptor keeps it — and that
        // is not a refusal, because nothing was undecidable.
        assert_eq!(decision.class_for('\u{7684}'), None);
        assert_eq!(decision.class_for('\u{FF21}'), None);
        assert!(decision.refused().is_empty());
    }

    /// The two Latin classes stated together are what no character tells apart.
    #[test]
    fn two_classes_holding_one_kind_of_character_are_both_refused() {
        let stated = vec!["Proportional".to_owned(), "HRoman".to_owned()];
        let decision = Decision::read(&stated, Some(("Adobe", "Japan1")));
        assert_eq!(decision.class_for('A'), None);
        assert_eq!(decision.refused().len(), 2, "{:?}", decision.refused());
    }

    /// `AlphaNum` takes the digits and leaves the letters, which is Table 123's own division.
    #[test]
    fn a_contested_kind_is_refused_and_an_uncontested_one_is_not() {
        let stated = vec!["Proportional".to_owned(), "AlphaNum".to_owned()];
        let decision = Decision::read(&stated, Some(("Adobe", "Japan1")));
        assert_eq!(decision.class_for('A'), Some(0));
        assert_eq!(decision.class_for('7'), None);
        assert_eq!(decision.refused().len(), 2, "{:?}", decision.refused());
    }

    /// The five classes no character decides, and the two collections that decide none.
    #[test]
    fn a_class_no_character_decides_is_refused_by_name() {
        for (stated, collection) in [
            (
                vec!["ProportionalRot".to_owned()],
                Some(("Adobe", "Japan1")),
            ),
            (vec!["Ruby".to_owned()], Some(("Adobe", "Japan1"))),
            (vec!["Dingbats".to_owned()], Some(("Adobe", "Japan1"))),
            (vec!["Generic".to_owned()], Some(("Adobe", "GB1"))),
            // Table 123 lists no `Kanji` for a Chinese collection.
            (vec!["Kanji".to_owned()], Some(("Adobe", "GB1"))),
            // Every `Identity` ordering, which Table 123 does not tabulate.
            (vec!["Proportional".to_owned()], Some(("Adobe", "Identity"))),
            (vec!["Proportional".to_owned()], None),
        ] {
            let decision = Decision::read(&stated, collection);
            assert_eq!(decision.refused().len(), 1, "over {stated:?}");
            assert_eq!(decision.class_for('A'), None, "over {stated:?}");
            assert_eq!(decision.class_for('\u{7684}'), None, "over {stated:?}");
        }
    }
}
