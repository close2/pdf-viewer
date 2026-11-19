//! The two derived properties internationalised identifiers are checked against: RFC 5892's for
//! IDNA2008's labels, which ITU-T X.520 clause 8.9.2's `dnsNameMatch` compares, and RFC 8264's
//! for the PRECIS string classes, which RFC 7622 profiles for a Jabber identifier and X.520
//! clause 8.9.4's `jidMatch` compares; with the contextual rules of RFC 5892 Appendix A, which
//! both use, and RFC 5893's Bidi Rule, which both apply to right-to-left text (ADR 3161).
//!
//! Both RFCs define the property as an algorithm over Unicode character properties rather than
//! as a table, so that it follows each Unicode version; the tables their appendices print are
//! illustrative (RFC 5892 section 4). The properties come from `icu_properties`' compiled data
//! (the Unicode Character Database, ADR 3161), Normalization Form KC from
//! `unicode-normalization`.

use icu_properties::props::{
    BidiClass, CanonicalCombiningClass, ChangesWhenNfkcCasefolded, DefaultIgnorableCodePoint,
    GeneralCategory, HangulSyllableType, JoinControl, JoiningType, NoncharacterCodePoint, Script,
    WhiteSpace,
};
use icu_properties::{CodePointMapData, CodePointSetData};
use unicode_normalization::UnicodeNormalization as _;

/// A code point's derived property value (RFC 5892 section 3, RFC 8264 section 8), once a
/// string class has decided its `ID_DIS or FREE_PVAL`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Property {
    /// Allowed anywhere.
    Valid,
    /// Allowed where RFC 5892 Appendix A's rule for a join control holds.
    ContextJ,
    /// Allowed where RFC 5892 Appendix A's rule for the code point holds.
    ContextO,
    /// Never allowed.
    Disallowed,
    /// Not assigned in the Unicode version this program carries, so not allowed.
    Unassigned,
}

/// RFC 8264 section 4's two string classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Class {
    /// Section 4.2's `IdentifierClass`, which `ID_DIS` excludes from.
    Identifier,
    /// Section 4.3's `FreeformClass`, which `FREE_PVAL` admits to.
    Freeform,
}

fn category(c: char) -> GeneralCategory {
    CodePointMapData::<GeneralCategory>::new().get(c)
}

/// Whether `c` is a combining mark, general category `M`, which RFC 5891 section 4.2.3.2 forbids
/// to begin a label.
pub(super) fn mark(c: char) -> bool {
    matches!(
        category(c),
        GeneralCategory::NonspacingMark
            | GeneralCategory::SpacingMark
            | GeneralCategory::EnclosingMark
    )
}

/// RFC 5892 section 2.6's `Exceptions` (F), which both algorithms consult first (RFC 8264 section
/// 9.6 takes them by reference).
fn exception(c: char) -> Option<Property> {
    Some(match c {
        '\u{DF}' | '\u{3C2}' | '\u{6FD}' | '\u{6FE}' | '\u{F0B}' | '\u{3007}' => Property::Valid,
        '\u{B7}'
        | '\u{375}'
        | '\u{5F3}'
        | '\u{5F4}'
        | '\u{30FB}'
        | '\u{660}'..='\u{669}'
        | '\u{6F0}'..='\u{6F9}' => Property::ContextO,
        '\u{640}' | '\u{7FA}' | '\u{302E}' | '\u{302F}' | '\u{3031}'..='\u{3035}' | '\u{303B}' => {
            Property::Disallowed
        }
        _ => return None,
    })
}

/// RFC 5892 section 2.10's `Unassigned` (J): `Cn` and not a noncharacter. Section 2.7's
/// `BackwardCompatible` (G), which comes before it, is empty.
fn unassigned(c: char) -> bool {
    category(c) == GeneralCategory::Unassigned
        && !CodePointSetData::new::<NoncharacterCodePoint>().contains(c)
}

/// RFC 5892 section 2.9's `OldHangulJamo` (I): Hangul syllable type L, V or T.
fn old_hangul_jamo(c: char) -> bool {
    matches!(
        CodePointMapData::<HangulSyllableType>::new().get(c),
        HangulSyllableType::LeadingJamo
            | HangulSyllableType::VowelJamo
            | HangulSyllableType::TrailingJamo
    )
}

/// RFC 5892 section 2.1's `LetterDigits` (A): `Ll`, `Lu`, `Lo`, `Nd`, `Lm`, `Mn` or `Mc`.
fn letter_digits(c: char) -> bool {
    matches!(
        category(c),
        GeneralCategory::LowercaseLetter
            | GeneralCategory::UppercaseLetter
            | GeneralCategory::OtherLetter
            | GeneralCategory::DecimalNumber
            | GeneralCategory::ModifierLetter
            | GeneralCategory::NonspacingMark
            | GeneralCategory::SpacingMark
    )
}

/// RFC 5892 section 2.2's Unstable (B), `toNFKC(toCaseFold(toNFKC(cp))) != cp`, read as the
/// Unicode property `Changes_When_NFKC_Casefolded`. That property is `toNFKC_Casefold(cp) != cp`,
/// the same composition with default-ignorable code points then removed; the two differ only on
/// a default-ignorable code point, which section 2.3's `IgnorableProperties`, the next step,
/// disallows whichever answer this one gives — and the one default-ignorable pair before this
/// step, the join controls, is decided by section 2.8 first. So the derived property is the same.
fn unstable(c: char) -> bool {
    CodePointSetData::new::<ChangesWhenNfkcCasefolded>().contains(c)
}

/// RFC 5892 section 2.4's `IgnorableBlocks` (D): Combining Diacritical Marks for Symbols, Musical
/// Symbols, and Ancient Greek Musical Notation, by Blocks.txt's ranges.
fn ignorable_block(c: char) -> bool {
    matches!(c, '\u{20D0}'..='\u{20FF}' | '\u{1D100}'..='\u{1D1FF}' | '\u{1D200}'..='\u{1D24F}')
}

/// RFC 5892 section 3: a code point's IDNA2008 derived property.
pub(super) fn idna(c: char) -> Property {
    if let Some(value) = exception(c) {
        return value;
    }
    if unassigned(c) {
        return Property::Unassigned;
    }
    // Section 2.5's LDH (E).
    if matches!(c, '-' | '0'..='9' | 'a'..='z') {
        return Property::Valid;
    }
    if CodePointSetData::new::<JoinControl>().contains(c) {
        return Property::ContextJ;
    }
    // Sections 2.2 to 2.4 and 2.9, each of them disallowing.
    if unstable(c)
        || CodePointSetData::new::<DefaultIgnorableCodePoint>().contains(c)
        || CodePointSetData::new::<WhiteSpace>().contains(c)
        || CodePointSetData::new::<NoncharacterCodePoint>().contains(c)
        || ignorable_block(c)
        || old_hangul_jamo(c)
    {
        return Property::Disallowed;
    }
    if letter_digits(c) {
        Property::Valid
    } else {
        Property::Disallowed
    }
}

/// RFC 8264 section 8: a code point's PRECIS derived property in `class`, `ID_DIS or FREE_PVAL`
/// resolved by the class.
pub(super) fn precis(c: char, class: Class) -> Property {
    if let Some(value) = exception(c) {
        return value;
    }
    if unassigned(c) {
        return Property::Unassigned;
    }
    // Section 9.11's ASCII7 (K).
    if matches!(c, '\u{21}'..='\u{7E}') {
        return Property::Valid;
    }
    if CodePointSetData::new::<JoinControl>().contains(c) {
        return Property::ContextJ;
    }
    // OldHangulJamo (I), section 9.13's PrecisIgnorableProperties (M), section 9.12's Controls (L).
    if old_hangul_jamo(c)
        || CodePointSetData::new::<DefaultIgnorableCodePoint>().contains(c)
        || CodePointSetData::new::<NoncharacterCodePoint>().contains(c)
        || category(c) == GeneralCategory::Control
    {
        return Property::Disallowed;
    }
    let class_only = match class {
        Class::Identifier => Property::Disallowed,
        Class::Freeform => Property::Valid,
    };
    // Section 9.17's HasCompat (Q).
    if c.to_string().nfkc().ne(std::iter::once(c)) {
        return class_only;
    }
    if letter_digits(c) {
        return Property::Valid;
    }
    // Sections 9.18, 9.14, 9.15 and 9.16: OtherLetterDigits (R), Spaces (N), Symbols (O) and
    // Punctuation (P).
    if matches!(
        category(c),
        GeneralCategory::TitlecaseLetter
            | GeneralCategory::LetterNumber
            | GeneralCategory::OtherNumber
            | GeneralCategory::EnclosingMark
            | GeneralCategory::SpaceSeparator
            | GeneralCategory::MathSymbol
            | GeneralCategory::CurrencySymbol
            | GeneralCategory::ModifierSymbol
            | GeneralCategory::OtherSymbol
            | GeneralCategory::ConnectorPunctuation
            | GeneralCategory::DashPunctuation
            | GeneralCategory::OpenPunctuation
            | GeneralCategory::ClosePunctuation
            | GeneralCategory::InitialPunctuation
            | GeneralCategory::FinalPunctuation
            | GeneralCategory::OtherPunctuation
    ) {
        return class_only;
    }
    Property::Disallowed
}

/// Whether every code point of `text` is allowed by `property`: valid, or contextual with its
/// rule satisfied in `text` (RFC 5891 section 4.2.3.3, RFC 8264 section 8's CONTEXTUAL RULE
/// REQUIRED).
pub(super) fn all_allowed(text: &[char], property: impl Fn(char) -> Property) -> bool {
    (0..text.len()).all(|at| match property(text[at]) {
        Property::Valid => true,
        Property::ContextJ | Property::ContextO => context_holds(text, at),
        Property::Disallowed | Property::Unassigned => false,
    })
}

fn script(c: char) -> Script {
    CodePointMapData::<Script>::new().get(c)
}

fn joining(c: char) -> JoiningType {
    CodePointMapData::<JoiningType>::new().get(c)
}

/// RFC 5892 Appendix A: whether the rule for the code point at `at` of `text` holds. A code
/// point the appendix gives no rule is not allowed (RFC 8264 section 8: "missing a rule").
fn context_holds(text: &[char], at: usize) -> bool {
    let before = at.checked_sub(1).map(|index| text[index]);
    let following = text.get(at..).and_then(|rest| rest.get(1..)).unwrap_or(&[]);
    let after = following.first().copied();
    let virama_before = before.is_some_and(|c| {
        CodePointMapData::<CanonicalCombiningClass>::new().get(c) == CanonicalCombiningClass::Virama
    });
    match text[at] {
        // A.1: after a virama, or between a left- or dual-joining and a right- or dual-joining
        // code point with only transparent ones between.
        '\u{200C}' => {
            let left = text[..at]
                .iter()
                .rev()
                .find(|&&c| joining(c) != JoiningType::Transparent)
                .is_some_and(|&c| {
                    matches!(
                        joining(c),
                        JoiningType::LeftJoining | JoiningType::DualJoining
                    )
                });
            let right = following
                .iter()
                .find(|&&c| joining(c) != JoiningType::Transparent)
                .is_some_and(|&c| {
                    matches!(
                        joining(c),
                        JoiningType::RightJoining | JoiningType::DualJoining
                    )
                });
            virama_before || (left && right)
        }
        // A.2.
        '\u{200D}' => virama_before,
        // A.3: between two 'l's.
        '\u{B7}' => before == Some('l') && after == Some('l'),
        // A.4: before a Greek code point.
        '\u{375}' => after.is_some_and(|c| script(c) == Script::Greek),
        // A.5 and A.6: after a Hebrew code point.
        '\u{5F3}' | '\u{5F4}' => before.is_some_and(|c| script(c) == Script::Hebrew),
        // A.7: somewhere in the string a Hiragana, Katakana or Han code point.
        '\u{30FB}' => text
            .iter()
            .any(|&c| matches!(script(c), Script::Hiragana | Script::Katakana | Script::Han)),
        // A.8 and A.9: the two sets of Arabic-Indic digits are not mixed.
        '\u{660}'..='\u{669}' => !text.iter().any(|c| matches!(c, '\u{6F0}'..='\u{6F9}')),
        '\u{6F0}'..='\u{6F9}' => !text.iter().any(|c| matches!(c, '\u{660}'..='\u{669}')),
        _ => false,
    }
}

fn bidi(c: char) -> BidiClass {
    CodePointMapData::<BidiClass>::new().get(c)
}

/// RFC 5893 section 1.4: a label, or a PRECIS string, holding a code point of Bidi class R, AL
/// or AN is right to left.
pub(super) fn right_to_left(text: &[char]) -> bool {
    text.iter().any(|&c| {
        matches!(
            bidi(c),
            BidiClass::RightToLeft | BidiClass::ArabicLetter | BidiClass::ArabicNumber
        )
    })
}

/// RFC 5893 section 2's Bidi Rule, its six conditions over `text`.
pub(super) fn bidi_rule(text: &[char]) -> bool {
    use BidiClass as B;
    let Some(&first) = text.first() else {
        return false;
    };
    // The last code point that is not a nonspacing mark (conditions 3 and 6).
    let last = text.iter().rev().find(|&&c| bidi(c) != B::NonspacingMark);
    match bidi(first) {
        B::RightToLeft | B::ArabicLetter => {
            let allowed = text.iter().all(|&c| {
                matches!(
                    bidi(c),
                    B::RightToLeft
                        | B::ArabicLetter
                        | B::ArabicNumber
                        | B::EuropeanNumber
                        | B::EuropeanSeparator
                        | B::CommonSeparator
                        | B::EuropeanTerminator
                        | B::OtherNeutral
                        | B::BoundaryNeutral
                        | B::NonspacingMark
                )
            });
            let ends = last.is_some_and(|&c| {
                matches!(
                    bidi(c),
                    B::RightToLeft | B::ArabicLetter | B::EuropeanNumber | B::ArabicNumber
                )
            });
            let european = text.iter().any(|&c| bidi(c) == B::EuropeanNumber);
            let arabic = text.iter().any(|&c| bidi(c) == B::ArabicNumber);
            allowed && ends && !(european && arabic)
        }
        B::LeftToRight => {
            let allowed = text.iter().all(|&c| {
                matches!(
                    bidi(c),
                    B::LeftToRight
                        | B::EuropeanNumber
                        | B::EuropeanSeparator
                        | B::CommonSeparator
                        | B::EuropeanTerminator
                        | B::OtherNeutral
                        | B::BoundaryNeutral
                        | B::NonspacingMark
                )
            });
            let ends = last.is_some_and(|&c| matches!(bidi(c), B::LeftToRight | B::EuropeanNumber));
            allowed && ends
        }
        _ => false,
    }
}
