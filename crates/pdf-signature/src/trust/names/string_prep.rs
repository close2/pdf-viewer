//! RFC 4518's six-step string preparation, as RFC 5280 section 7.1 asks for it before two
//! distinguished-name attribute values are compared: with step 2's case folding by RFC 3454 table
//! B.2, step 6 as section 2.6.1's insignificant space handling, and every value treated as a stored
//! one. RFC 4518 section 2: "Failure in any step causes the assertion to evaluate to Undefined",
//! which is `None` here and [`super::Match::Undecided`] to the caller.
//!
//! The lists of steps 2 and 4 are the RFC's own: section 2.2 prints "a complete list" of the
//! control and separator code points it maps, and those are written here as printed rather than
//! read off a later Unicode's general categories, which would map characters Unicode 3.2 — the
//! repertoire section 2 names — did not assign, where step 4 prohibits them. RFC 3454's tables A.1,
//! B.2, C.3, C.4, C.5 and C.8 are `stringprep`'s, reviewed data rather than tables typed here (ADR
//! 3158), and step 3's Normalization Form KC is `unicode-normalization`'s (ADR 3170).

use stringprep::tables::{
    case_fold_for_nfkc, change_display_properties_or_deprecated, non_character_code_point,
    private_use, surrogate_code, unassigned_code_point,
};
use unicode_normalization::UnicodeNormalization as _;
use unicode_properties::{GeneralCategoryGroup, UnicodeGeneralCategory as _};

/// `text`, already Unicode (step 1, transcoding, is the caller's decoding of the value), through
/// steps 2 to 6. `None` where step 4 prohibits a code point.
pub(super) fn prepared(text: &str) -> Option<String> {
    let mut mapped = String::with_capacity(text.len());
    for c in text.chars() {
        match mapping(c) {
            Mapping::Nothing => {}
            Mapping::Space => mapped.push(' '),
            Mapping::Itself => mapped.extend(case_fold_for_nfkc(c)),
        }
    }
    // Step 3, "normalized to Unicode Form KC (compatibility composed)".
    let normalized: String = mapped.nfkc().collect();
    // Step 4. Step 5, "Bidirectional characters are ignored", asks nothing.
    if normalized.chars().any(prohibited) {
        return None;
    }
    Some(spaces_handled(&normalized))
}

/// What step 2 does with one code point.
enum Mapping {
    /// Mapped to nothing.
    Nothing,
    /// Mapped to SPACE (U+0020).
    Space,
    /// Kept, then case folded by RFC 3454 table B.2, which RFC 5280 section 7.1 adds to the step.
    Itself,
}

/// RFC 4518 section 2.2's map, in the section's own lists.
fn mapping(c: char) -> Mapping {
    if mapped_to_nothing(c) {
        Mapping::Nothing
    } else if mapped_to_space(c) {
        Mapping::Space
    } else {
        Mapping::Itself
    }
}

/// The code points section 2.2 maps to nothing: SOFT HYPHEN, MONGOLIAN TODO SOFT HYPHEN, COMBINING
/// GRAPHEME JOINER, the variation selectors, OBJECT REPLACEMENT CHARACTER, ZERO WIDTH SPACE, and
/// every control but the six white-space ones, of which "\[t]he following is a complete list".
///
/// The section prints the second range of variation selectors as "FF00-FE0F", which is empty as
/// written; the block it names is FE00 to FE0F.
const fn mapped_to_nothing(c: char) -> bool {
    matches!(
        c,
        '\u{AD}'
            | '\u{1806}'
            | '\u{34F}'
            | '\u{180B}'..='\u{180D}'
            | '\u{FE00}'..='\u{FE0F}'
            | '\u{FFFC}'
            | '\u{200B}'
            | '\u{00}'..='\u{08}'
            | '\u{0E}'..='\u{1F}'
            | '\u{7F}'..='\u{84}'
            | '\u{86}'..='\u{9F}'
            | '\u{6DD}'
            | '\u{70F}'
            | '\u{180E}'
            | '\u{200C}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{2063}'
            | '\u{206A}'..='\u{206F}'
            | '\u{FEFF}'
            | '\u{FFF9}'..='\u{FFFB}'
            | '\u{1D173}'..='\u{1D17A}'
            | '\u{E0001}'
            | '\u{E0020}'..='\u{E007F}'
    )
}

/// The code points section 2.2 maps to SPACE: the six white-space controls, and every separator,
/// of which "\[t]he following is a complete list".
const fn mapped_to_space(c: char) -> bool {
    matches!(
        c,
        '\u{09}'..='\u{0D}'
            | '\u{85}'
            | '\u{20}'
            | '\u{A0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200A}'
            | '\u{2028}'..='\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
    )
}

/// RFC 4518 section 2.4: unassigned code points (RFC 3454 table A.1), characters that change
/// display properties or are deprecated (C.8), private use (C.3), non-characters (C.4), surrogate
/// codes (C.5) and the REPLACEMENT CHARACTER.
fn prohibited(c: char) -> bool {
    c == '\u{FFFD}'
        || unassigned_code_point(c)
        || change_display_properties_or_deprecated(c)
        || private_use(c)
        || non_character_code_point(c)
        || surrogate_code(c)
}

/// RFC 4518 section 2.6.1 for an attribute value, where "a space is defined to be the SPACE
/// (U+0020) code point followed by no combining marks": a value of spaces alone is "exactly two
/// SPACEs"; any other starts and ends with exactly one, and each inner run of spaces becomes
/// exactly two.
fn spaces_handled(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let is_space = |at: usize| {
        chars[at] == ' '
            && chars
                .get(at.saturating_add(1))
                .is_none_or(|next| next.general_category_group() != GeneralCategoryGroup::Mark)
    };
    let mut out = String::from(" ");
    let mut run = false;
    let mut any = false;
    for (at, &c) in chars.iter().enumerate() {
        if is_space(at) {
            run = any;
        } else {
            if run {
                out.push_str("  ");
                run = false;
            }
            out.push(c);
            any = true;
        }
    }
    out.push(' ');
    out
}

#[cfg(test)]
mod tests {
    use super::prepared;

    /// Section 2.6.1's own example: "foo<SPACE>bar<SPACE><SPACE>" becomes
    /// "<SPACE>foo<SPACE><SPACE>bar<SPACE>", and spaces alone become exactly two.
    #[test]
    fn insignificant_spaces_are_handled_as_the_section_prints_them() {
        assert_eq!(prepared("foo bar  ").as_deref(), Some(" foo  bar "));
        assert_eq!(prepared("   ").as_deref(), Some("  "));
        assert_eq!(prepared("").as_deref(), Some("  "));
    }

    /// Step 2 maps NO-BREAK SPACE to SPACE and SOFT HYPHEN to nothing, and folds by table B.2,
    /// which takes LATIN SMALL LETTER SHARP S to "ss"; step 3's compatibility composition takes
    /// the ligature U+FB01 to "fi" and the decomposed "e" with U+0301 to U+00E9.
    #[test]
    fn mapping_case_folding_and_compatibility_composition_apply_beyond_ascii() {
        assert_eq!(prepared("Stra\u{DF}e"), prepared("STRASSE"));
        assert_eq!(prepared("A\u{A0}B"), prepared("a b"));
        assert_eq!(prepared("Ad\u{AD}min"), prepared("admin"));
        assert_eq!(prepared("\u{FB01}le"), prepared("FILE"));
        assert_eq!(prepared("Caf\u{E9}"), prepared("cafe\u{301}"));
        assert_ne!(prepared("Caf\u{E9}"), prepared("cafe"));
    }

    /// The calibration (trap 13): step 4 refuses a private-use character, an unassigned one in
    /// Unicode 3.2 and the REPLACEMENT CHARACTER, where the comparison is undefined.
    #[test]
    fn a_prohibited_code_point_fails_the_preparation() {
        assert_eq!(prepared("a\u{E000}"), None);
        assert_eq!(prepared("a\u{FFFD}"), None);
        assert_eq!(prepared("a\u{0378}"), None);
        assert!(prepared("a").is_some());
    }
}
