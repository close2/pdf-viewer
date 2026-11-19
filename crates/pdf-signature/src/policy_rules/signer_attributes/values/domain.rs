//! ITU-T X.520 clause 8.9.2's `dnsNameMatch` over clause 6.2.15's `DomainName`, and clause
//! 8.9.3's `intEmailMatch`, whose domain part it compares (ADR 3161). X.520 is cited and
//! paraphrased (ADR 1085).
//!
//! # The syntax
//!
//! Clause 6.2.15 holds a domain name to RFC 5890 section 2.3.1: labels separated by dots, each in
//! one of three forms — Basic Latin letters, digits and hyphen not beginning `xn--`, at most 63
//! octets; an A-label, `xn--` and the Punycode of a U-label (RFC 3492), which must convert to a
//! valid U-label; or a U-label, which holds code points outside Basic Latin, all within RFC
//! 5892's repertoire, and must convert to a valid A-label. A U-label's validity is RFC 5891
//! section 4.2's: Normalization Form C, the hyphen restrictions of 4.2.3.1, no leading combining
//! mark (4.2.3.2), every code point `PVALID` or contextual with its rule satisfied (4.2.2 and
//! 4.2.3.3), and RFC 5893's Bidi Rule on every label of a domain name that holds a right-to-left
//! label (4.2.3.4). An A-label's prefix is matched in either case and the label is lowercased
//! before it is decoded, as `caseIgnoreMatch` would compare it; it is valid where it decodes to a
//! valid U-label that encodes back to it. A value outside the syntax is undefined (X.501 clause
//! 8.9.2.1 (a)(3)).
//!
//! # The rule
//!
//! Label for label: a different number of labels is false; a label of the first form against an
//! A-label or a U-label is false; two of one form compare by `caseIgnoreMatch`; an A-label against
//! a U-label compares the U-label converted to an A-label. A wildcard is a leftmost `*` label: a
//! value holding more than one, or one elsewhere, is false, and one matches any label opposite it
//! — so `*.example.com` matches `a.example.com` and neither `a.b.example.com` nor `example.com`,
//! clause 8.9.2's own note.

use unicode_normalization::UnicodeNormalization as _;

use super::code_points;
use super::prepare::{Insignificant, UTF8_STRING, prepared_text};
use super::punycode;
use super::structured::{all, answer};
use crate::der::Value;
use crate::trust::names::Match;

/// RFC 5890 section 2.3.2.1's ACE prefix.
const ACE_PREFIX: &str = "xn--";
/// RFC 5890 section 2.3.2.1's limit on an A-label and on a label of the first form, in octets.
pub(super) const MAX_LABEL: usize = 63;

/// One label of a domain name, by clause 6.2.15's forms.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Label {
    /// Item a).
    Ldh(String),
    /// Item b), lowercased, with the U-label it decodes to.
    A { ascii: String, unicode: String },
    /// Item c).
    U(String),
    /// The leftmost `*` a fully-qualified name may carry.
    Wildcard,
}

pub(super) fn has_ace_prefix(text: &str) -> bool {
    text.get(..ACE_PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(ACE_PREFIX))
}

/// RFC 5891 section 4.2's checks of a U-label but the Bidi Rule, which is the domain name's.
pub(super) fn valid_u_label(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    let Some(&first) = chars.first() else {
        return false;
    };
    let hyphens = first == '-'
        || chars.last() == Some(&'-')
        || (chars.get(2) == Some(&'-') && chars.get(3) == Some(&'-'));
    text.nfc().eq(text.chars())
        && !hyphens
        && !code_points::mark(first)
        && code_points::all_allowed(&chars, code_points::idna)
}

/// A U-label as an A-label, where its encoding fits in a label.
pub(super) fn to_a_label(unicode: &str) -> Option<String> {
    let ascii = format!("{ACE_PREFIX}{}", punycode::encode(unicode)?);
    (ascii.len() <= MAX_LABEL).then_some(ascii)
}

/// An A-label, already lowercased, as the U-label it is the encoding of, where it is a valid one.
pub(super) fn from_a_label(ascii: &str) -> Option<String> {
    let encoded = ascii.strip_prefix(ACE_PREFIX)?;
    if ascii.len() > MAX_LABEL {
        return None;
    }
    let unicode = punycode::decode(encoded)?;
    let round_trip = punycode::encode(&unicode)?;
    (!unicode.is_ascii() && valid_u_label(&unicode) && round_trip == encoded).then_some(unicode)
}

fn label(text: &str) -> Option<Label> {
    if text == "*" {
        return Some(Label::Wildcard);
    }
    if !text.is_ascii() {
        to_a_label(text)?;
        return valid_u_label(text).then(|| Label::U(text.to_owned()));
    }
    if has_ace_prefix(text) {
        let ascii = text.to_ascii_lowercase();
        let unicode = from_a_label(&ascii)?;
        return Some(Label::A { ascii, unicode });
    }
    let ldh = !text.is_empty()
        && text.len() <= MAX_LABEL
        && text.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-');
    ldh.then(|| Label::Ldh(text.to_owned()))
}

/// RFC 5893 section 1.4 and section 2: in a domain name holding a right-to-left label, every
/// label satisfies the Bidi Rule. The wildcard is no label of the name it stands in for.
pub(super) fn bidi_holds<'a>(labels: impl Iterator<Item = &'a str> + Clone) -> bool {
    let unicode = labels.map(|text| text.chars().collect::<Vec<_>>());
    !unicode
        .clone()
        .any(|label| code_points::right_to_left(&label))
        || unicode
            .into_iter()
            .all(|label| code_points::bidi_rule(&label))
}

/// A domain name's labels, or `None` where it is outside clause 6.2.15's syntax.
fn labels(text: &str) -> Option<Vec<Label>> {
    let labels: Vec<Label> = text.split('.').map(label).collect::<Option<_>>()?;
    let unicode = labels.iter().filter_map(|label| match label {
        Label::Ldh(text) | Label::U(text) | Label::A { unicode: text, .. } => Some(text.as_str()),
        Label::Wildcard => None,
    });
    bidi_holds(unicode).then_some(labels)
}

/// Clause 8.9.2's items a) to c) for one pair of labels, and item e)'s wildcard.
fn label_match(left: &Label, right: &Label) -> Match {
    match (left, right) {
        (Label::Wildcard, _) | (_, Label::Wildcard) => Match::Yes,
        (Label::Ldh(left), Label::Ldh(right)) => answer(left.eq_ignore_ascii_case(right)),
        (Label::Ldh(_), _) | (_, Label::Ldh(_)) => Match::No,
        (Label::A { ascii: left, .. }, Label::A { ascii: right, .. }) => answer(left == right),
        (Label::U(left), Label::U(right)) => match (
            prepared_text(left, true, Insignificant::Spaces),
            prepared_text(right, true, Insignificant::Spaces),
        ) {
            (Some(left), Some(right)) => answer(left == right),
            _ => Match::Undecided,
        },
        (Label::A { ascii, .. }, Label::U(unicode))
        | (Label::U(unicode), Label::A { ascii, .. }) => match to_a_label(unicode) {
            Some(converted) => answer(converted.eq_ignore_ascii_case(ascii)),
            None => Match::Undecided,
        },
    }
}

/// Clause 8.9.2 over two domain names as text; `wildcards` is false for clause 6.2.16's domain
/// part, where an asterisk shall not be a label at all.
fn names(carried: &str, stated: &str, wildcards: bool) -> Match {
    for text in [carried, stated] {
        let stars: Vec<usize> = text
            .split('.')
            .enumerate()
            .filter_map(|(at, label)| (label == "*").then_some(at))
            .collect();
        if !wildcards && !stars.is_empty() {
            return Match::Undecided;
        }
        // Item d).
        if stars.len() > 1 || stars.iter().any(|&at| at != 0) {
            return Match::No;
        }
    }
    let (Some(carried), Some(stated)) = (labels(carried), labels(stated)) else {
        return Match::Undecided;
    };
    if carried.len() != stated.len() {
        return Match::No;
    }
    all(carried
        .iter()
        .zip(&stated)
        .map(|(left, right)| label_match(left, right)))
}

fn utf8<'a>(value: &Value<'a>) -> Option<&'a str> {
    (value.identifier == UTF8_STRING)
        .then(|| std::str::from_utf8(value.contents).ok())
        .flatten()
}

/// Clause 8.9.2's `dnsNameMatch`.
pub(super) fn dns_name(carried: &Value<'_>, stated: &Value<'_>) -> Match {
    match (utf8(carried), utf8(stated)) {
        (Some(carried), Some(stated)) => names(carried, stated, true),
        _ => Match::Undecided,
    }
}

/// Clause 8.9.3's `intEmailMatch`: the local parts by `caseIgnoreMatch` and the domain parts by
/// clause 8.9.2. The domain part follows the last `@`, which a local part may not hold unquoted.
pub(super) fn int_email(carried: &Value<'_>, stated: &Value<'_>) -> Match {
    let (Some((carried_local, carried_domain)), Some((stated_local, stated_domain))) = (
        utf8(carried).and_then(|text| text.rsplit_once('@')),
        utf8(stated).and_then(|text| text.rsplit_once('@')),
    ) else {
        return Match::Undecided;
    };
    let local = match (
        prepared_text(carried_local, true, Insignificant::Spaces),
        prepared_text(stated_local, true, Insignificant::Spaces),
    ) {
        (Some(left), Some(right)) => answer(left == right),
        _ => Match::Undecided,
    };
    all([local, names(carried_domain, stated_domain, false)])
}
