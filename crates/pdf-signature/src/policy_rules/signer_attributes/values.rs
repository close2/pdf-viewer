//! ETSI TR 102 272 clause 6.9's `attributeValueConstraints`: a signer attribute whose type only
//! that list names is within the attribute trust condition where its values are among the values
//! the list states (ADR 2895). ETSI's text is cited and paraphrased (ADR 1085).
//!
//! # Equal by the type's own rule
//!
//! Each entry of the list is an X.501 `AttributeTypeAndValue`, and whether a carried value is the
//! value an entry states is ITU-T X.501 clause 8.9.2.1's evaluation of an attribute value
//! assertion: undefined where the type is unknown, where it has no equality matching rule, or where
//! the value does not conform to the rule's syntax; true where the values match by the type's
//! equality matching rule; false otherwise. X.501 clause 13.4.7 says that an attribute type
//! without an equality matching rule has its values never compared, and clause 13.4.2 that a
//! subtype keeps its supertype's matching rules. So the rule is a fact about each type, which ITU-T
//! X.520 states in each type's definition. An undefined answer is [`Match::Undecided`]: the
//! condition it reaches is not evaluated, never met and never broken.
//!
//! # The types this program holds the rule for
//!
//! [`CASE_IGNORE`] is every X.520 type in the `id-at` arc whose definition states `caseIgnoreMatch`
//! as its equality matching rule, or that is a subtype of `name`, which states it, and whose
//! syntax is a `DirectoryString` or one of its alternatives. X.520 clause 8.1.1 makes two values
//! equal when the strings that clause 7's preparation makes of them are the same. Every other type
//! is undecided by name. That covers `id-at-role` (2.5.4.72), which X.520 leaves to ITU-T X.509,
//! a text this tree does not hold, and every type whose rule is not `caseIgnoreMatch`. Of those,
//! `dc`'s syntax is `IA5String`, which `caseIgnoreMatch`'s `DirectoryString` syntax does not
//! include, so X.520 states its rule but clause 8.9.2.1 (a)(3) leaves every value undefined.
//!
//! # Preparation, where it is exact
//!
//! X.520 clause 7's six steps are transcoding, mapping with case folding by RFC 3454 table B.2,
//! normalisation to Unicode form KC, prohibition, a bidirectional check that prohibits nothing,
//! and the removal of insignificant spaces; a failure in any step makes the assertion undefined.
//! On ASCII text, folding is ASCII lowercase, form KC changes nothing, and the one prohibition
//! that can apply is the empty string's. The steps for any other character need the Unicode
//! tables, which are not in this tree, so a value holding one is undecided, as
//! [`crate::trust::names`] decides for a distinguished name's values under RFC 5280 section 7.1.

use crate::der::{SET, Value};
use crate::trust::names::Match;

/// The last arc of each X.520 type, under `id-at` (2.5.4), whose equality matching rule is
/// `caseIgnoreMatch` over a `DirectoryString` syntax, in X.520's order: `knowledgeInformation`
/// (2), `name` (41) and its subtypes `commonName` (3), `surname` (4), `givenName` (42),
/// `initials` (43), `generationQualifier` (44), `pseudonym` (65), `countryName` (6),
/// `localityName` (7), `stateOrProvinceName` (8), `organizationName` (10),
/// `organizationalUnitName` (11), `title` (12) and `dmdName` (54); then `dnQualifier` (46),
/// `serialNumber` (5), `streetAddress` (9), `houseIdentifier` (51), `organizationIdentifier`
/// (97), `description` (13), `businessCategory` (15), `postalCode` (17), `postOfficeBox` (18),
/// `physicalDeliveryOfficeName` (19) and `destinationIndicator` (27). `PrintableString`, the
/// syntax of `countryName`, `dnQualifier`, `serialNumber` and `destinationIndicator`, is one of
/// `DirectoryString`'s alternatives.
pub const CASE_IGNORE: [u8; 26] = [
    2, 41, 3, 4, 42, 43, 44, 65, 6, 7, 8, 10, 11, 12, 54, 46, 5, 9, 51, 97, 13, 15, 17, 18, 19, 27,
];

/// The equality matching rule of an attribute type, where this program holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    /// X.520 clause 8.1.1's `caseIgnoreMatch`.
    CaseIgnore,
}

/// The equality matching rule X.520 states for `kind`, an object identifier's contents, where it
/// is one this program holds.
#[must_use]
pub fn rule(kind: &[u8]) -> Option<Rule> {
    match kind {
        [0x55, 0x04, last] if CASE_IGNORE.contains(last) => Some(Rule::CaseIgnore),
        _ => None,
    }
}

/// One entry of `attributeValueConstraints`: a type, and a value the policy considers valid for
/// it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Listed<'a> {
    /// The type, as an object identifier's contents.
    pub kind: &'a [u8],
    /// The value, as the policy encodes it.
    pub value: Value<'a>,
}

/// Whether an attribute of type `kind` whose `SET OF` values is `values` is within `listed`: every
/// one of its values matches, by the type's equality matching rule, a value `listed` states for
/// the type (ADR 2895). [`Match::No`] where some value matches none of them; [`Match::Undecided`]
/// where none is that but some comparison is undefined, or where the set holds no value to judge.
#[must_use]
pub fn within(kind: &[u8], values: &Value<'_>, listed: &[Listed<'_>]) -> Match {
    let Some(mut reader) = (values.identifier == SET)
        .then(|| values.children().ok())
        .flatten()
    else {
        return Match::Undecided;
    };
    let mut answer = None;
    loop {
        let value = match reader.next_value() {
            Ok(Some(value)) => value,
            Ok(None) => break,
            Err(_) => return Match::Undecided,
        };
        let best =
            listed
                .iter()
                .filter(|entry| entry.kind == kind)
                .fold(Match::No, |best, entry| {
                    match (best, equal(kind, &value, &entry.value)) {
                        (Match::Yes, _) | (_, Match::Yes) => Match::Yes,
                        (Match::Undecided, _) | (_, Match::Undecided) => Match::Undecided,
                        _ => Match::No,
                    }
                });
        answer = Some(match (answer, best) {
            (_, Match::No) | (Some(Match::No), _) => Match::No,
            (_, Match::Undecided) | (Some(Match::Undecided), _) => Match::Undecided,
            _ => Match::Yes,
        });
    }
    answer.unwrap_or(Match::Undecided)
}

/// X.501 clause 8.9.2.1 for one carried value of type `kind` against one value the policy states.
fn equal(kind: &[u8], carried: &Value<'_>, stated: &Value<'_>) -> Match {
    match rule(kind) {
        None => Match::Undecided,
        Some(Rule::CaseIgnore) => match (prepared(carried), prepared(stated)) {
            (Some(carried), Some(stated)) => {
                if carried == stated {
                    Match::Yes
                } else {
                    Match::No
                }
            }
            _ => Match::Undecided,
        },
    }
}

/// X.520 clause 7's preparation of a `DirectoryString` value, where it is exact: `None` where the
/// value is not one of `DirectoryString`'s alternatives this program transcodes, where it holds a
/// character outside ASCII, or where a step fails.
fn prepared(value: &Value<'_>) -> Option<String> {
    // X.520's `DirectoryString` is a `TeletexString`, `PrintableString`, `BMPString`,
    // `UniversalString` or `UTF8String`. Clause 7.1 lets an implementation decline to transcode,
    // which makes the assertion undefined; this program declines `TeletexString`.
    const UTF8_STRING: u8 = 0x0C;
    const PRINTABLE_STRING: u8 = 0x13;
    const UNIVERSAL_STRING: u8 = 0x1C;
    const BMP_STRING: u8 = 0x1E;
    // Step 1, transcoding to Unicode.
    let text: String = match value.identifier {
        UTF8_STRING | PRINTABLE_STRING => std::str::from_utf8(value.contents).ok()?.to_owned(),
        BMP_STRING => {
            if !value.contents.len().is_multiple_of(2) {
                return None;
            }
            let units = value
                .contents
                .chunks_exact(2)
                .map(|pair| u16::from_be_bytes([pair[0], pair[1]]));
            char::decode_utf16(units)
                .collect::<Result<String, _>>()
                .ok()?
        }
        UNIVERSAL_STRING => {
            if !value.contents.len().is_multiple_of(4) {
                return None;
            }
            value
                .contents
                .chunks_exact(4)
                .map(|quad| {
                    char::from_u32(u32::from_be_bytes([quad[0], quad[1], quad[2], quad[3]]))
                })
                .collect::<Option<String>>()?
        }
        _ => return None,
    };
    if !text.is_ascii() {
        return None;
    }
    // Step 2: the white-space controls become SPACE, every other control nothing, and case folds.
    let mapped: String = text
        .chars()
        .filter_map(|c| match c {
            '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' => Some(' '),
            c if c.is_ascii_control() => None,
            c => Some(c.to_ascii_lowercase()),
        })
        .collect();
    // Step 4 prohibits the empty string.
    if mapped.is_empty() {
        return None;
    }
    // Step 6, clause 7.6.1: leading and trailing spaces go, a run of them is one, and a string of
    // nothing else is one space.
    let words: Vec<&str> = mapped.split(' ').filter(|word| !word.is_empty()).collect();
    Some(if words.is_empty() {
        " ".to_owned()
    } else {
        words.join(" ")
    })
}

#[cfg(test)]
mod tests;
