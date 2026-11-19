//! ITU-T X.520 clause 7's string preparation, the six steps every string matching rule of clause
//! 8.1 and the rules built on them apply to "each presented and attribute value": transcode, map,
//! normalise, prohibit, check bidi, and remove insignificant characters. "Failure in any step
//! shall cause the assertion to be UNDEFINED", which is `None` here. X.520 is cited and
//! paraphrased (ADR 1085).
//!
//! The steps are written from the clause; the character data they need is reviewed data rather
//! than tables typed here (ADR 3158): RFC 3454 table B.2, which clause 7.2 names for case folding,
//! from `stringprep`'s `tables`; the Unicode general categories clauses 7.2 and 7.4 name (`Cc`,
//! `Cf`, `Zs`, `Zl`, `Zp`, combining marks, unassigned, private use) from `unicode-properties`;
//! and Normalization Form KC from `unicode-normalization`. `stringprep::x520prep` is not called:
//! with case folding it skips clause 7.3's normalisation, and it tests the first code point of
//! the input rather than of the string it prepared, so its answer is not clause 7's.

use stringprep::tables::case_fold_for_nfkc;
use unicode_normalization::UnicodeNormalization as _;
use unicode_properties::{GeneralCategory, GeneralCategoryGroup, UnicodeGeneralCategory as _};

use crate::der::Value;

/// Which characters clause 7.6 removes, which differs from rule to rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Insignificant {
    /// Clause 7.6.1, for the case-ignore and case-exact rules: leading, trailing and repeated
    /// spaces; a string of spaces alone is one space.
    Spaces,
    /// Clause 7.6.2, for `numericStringMatch`: every space; a string of spaces alone is empty.
    AllSpaces,
    /// Clause 8.2.8, for `telephoneNumberMatch`: "all hyphens and spaces are insignificant".
    HyphensAndSpaces,
}

/// The string syntaxes a rule's assertion admits, by their universal tags.
pub(super) const UTF8_STRING: u8 = 0x0C;
/// `NumericString`.
pub(super) const NUMERIC_STRING: u8 = 0x12;
/// `PrintableString`.
pub(super) const PRINTABLE_STRING: u8 = 0x13;
/// `TeletexString`, which clause 7.1 lets this program decline to transcode.
const TELETEX_STRING: u8 = 0x14;
/// `UniversalString`.
const UNIVERSAL_STRING: u8 = 0x1C;
/// `BMPString`.
const BMP_STRING: u8 = 0x1E;

/// X.520 clause 6's `UnboundedDirectoryString`, the syntax of `caseIgnoreMatch` and
/// `caseExactMatch`: `TeletexString`, `PrintableString`, `BMPString`, `UniversalString` or
/// `UTF8String`.
pub(super) const DIRECTORY_STRING: &[u8] = &[
    TELETEX_STRING,
    PRINTABLE_STRING,
    BMP_STRING,
    UNIVERSAL_STRING,
    UTF8_STRING,
];

/// `value`, prepared by clause 7 for a rule whose syntax admits the string types `syntax`, with
/// clause 7.2's case folding where `fold` and clause 7.6's removal as `removal` says. `None` where
/// the value is not one of the admitted types (X.501 clause 8.9.2.1 (a)(3)) or a step fails.
pub(super) fn prepared(
    value: &Value<'_>,
    syntax: &[u8],
    fold: bool,
    removal: Insignificant,
) -> Option<String> {
    if !syntax.contains(&value.identifier) {
        return None;
    }
    let text = transcoded(value)?;
    let mapped = mapped(&text, fold);
    // Clause 7.3.
    let normalized: String = mapped.nfkc().collect();
    prohibit(&normalized)?;
    // Clause 7.5: "There are no bidirectional restrictions."
    Some(removed(&normalized, removal))
}

/// Clause 7.1: each string as Unicode. `PrintableString` and `NumericString` are subsets of
/// ASCII; this program declines `TeletexString`, which the clause allows.
fn transcoded(value: &Value<'_>) -> Option<String> {
    match value.identifier {
        UTF8_STRING | PRINTABLE_STRING | NUMERIC_STRING => {
            std::str::from_utf8(value.contents).ok().map(str::to_owned)
        }
        BMP_STRING => {
            let (units, rest) = value.contents.as_chunks::<2>();
            if !rest.is_empty() {
                return None;
            }
            char::decode_utf16(units.iter().map(|pair| u16::from_be_bytes(*pair)))
                .collect::<Result<String, _>>()
                .ok()
        }
        UNIVERSAL_STRING => {
            let (quads, rest) = value.contents.as_chunks::<4>();
            if !rest.is_empty() {
                return None;
            }
            quads
                .iter()
                .map(|quad| char::from_u32(u32::from_be_bytes(*quad)))
                .collect()
        }
        _ => None,
    }
}

/// Clause 7.2: the characters it names and every other control or format character mapped to
/// nothing, the six it names and every other separator mapped to SPACE, then RFC 3454 table B.2's
/// case folding where the rule asks for it.
fn mapped(text: &str, fold: bool) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        let c = match c {
            '\u{AD}'
            | '\u{1806}'
            | '\u{34F}'
            | '\u{180B}'..='\u{180D}'
            | '\u{FFFC}'
            | '\u{200B}' => {
                continue;
            }
            // The clause prints the variation selectors as "U+180B-180D,FF00-FE0F"; the second
            // range as printed is empty, and the block it names is FE00 to FE0F.
            '\u{FE00}'..='\u{FE0F}' => continue,
            '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' | '\u{85}' => ' ',
            c if matches!(
                c.general_category(),
                GeneralCategory::Control | GeneralCategory::Format
            ) =>
            {
                continue;
            }
            c if c.general_category_group() == GeneralCategoryGroup::Separator => ' ',
            c => c,
        };
        if fold {
            out.extend(case_fold_for_nfkc(c));
        } else {
            out.push(c);
        }
    }
    out
}

/// Clause 7.4: unassigned (which takes in the non-characters, whose category is unassigned too),
/// private-use and surrogate code points, U+FFFD, a combining character first, and the empty
/// string are prohibited.
fn prohibit(text: &str) -> Option<()> {
    let first = text.chars().next()?;
    if first.general_category_group() == GeneralCategoryGroup::Mark {
        return None;
    }
    let prohibited = text.chars().any(|c| {
        c == '\u{FFFD}'
            || matches!(
                c.general_category(),
                GeneralCategory::Unassigned
                    | GeneralCategory::PrivateUse
                    | GeneralCategory::Surrogate
            )
    });
    (!prohibited).then_some(())
}

/// Clause 7.6, where "a space is defined to be the SPACE (U+0020) code point followed by no
/// combining marks": a SPACE a mark follows is kept as a character of the text.
fn removed(text: &str, removal: Insignificant) -> String {
    let chars: Vec<char> = text.chars().collect();
    let is_space = |at: usize| {
        chars[at] == ' '
            && chars
                .get(at.saturating_add(1))
                .is_none_or(|next| next.general_category_group() != GeneralCategoryGroup::Mark)
    };
    let mut out = String::with_capacity(text.len());
    match removal {
        Insignificant::AllSpaces | Insignificant::HyphensAndSpaces => {
            for (at, &c) in chars.iter().enumerate() {
                let hyphen = removal == Insignificant::HyphensAndSpaces && c == '-';
                if !is_space(at) && !hyphen {
                    out.push(c);
                }
            }
        }
        Insignificant::Spaces => {
            // Clause 7.6.1's example: "<SPACE><SPACE>foo<SPACE><SPACE>bar<SPACE><SPACE>" becomes
            // "foo<SPACE>bar", and a string of spaces alone becomes one space.
            let mut pending = false;
            for (at, &c) in chars.iter().enumerate() {
                if is_space(at) {
                    pending = !out.is_empty();
                } else {
                    if pending {
                        out.push(' ');
                        pending = false;
                    }
                    out.push(c);
                }
            }
            if out.is_empty() {
                out.push(' ');
            }
        }
    }
    out
}
