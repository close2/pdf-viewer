//! ITU-T X.520 clause 8.9.4's `jidMatch` over clause 6.2.17's `Jid`: two Jabber identifiers each
//! normalised as RFC 7622 section 3 says, then compared part for part, octet for octet, a part
//! one holds and the other does not being a difference (ADR 3161). X.520 is cited and
//! paraphrased (ADR 1085).
//!
//! RFC 7622 section 3.2 parses before it transforms: the resourcepart is what follows the first
//! `/`, the localpart what precedes the first `@` before it, and the domainpart the rest. Each part
//! is then enforced by its own rules — the localpart by RFC 8265's `UsernameCaseMapped` with
//! section 3.3.1's eight further characters excluded, the resourcepart by `OpaqueString`, and the
//! domainpart by section 3.2's: a final dot stripped first, an IP literal as written, and
//! otherwise RFC 5895's case, width and normalisation mappings, each label an NR-LDH label or a
//! U-label with every A-label converted to its U-label, under RFC 5893's Bidi Rule. Every part is
//! 1 to 1023 octets once enforced. An identifier that is no JID by those rules is undefined (X.501
//! clause 8.9.2.1 (a)(3)).

use unicode_normalization::UnicodeNormalization as _;

use super::domain::{
    MAX_LABEL, bidi_holds, from_a_label, has_ace_prefix, to_a_label, valid_u_label,
};
use super::precis::{MAX_PART, opaque_string, username_case_mapped, width};
use super::prepare::UTF8_STRING;
use super::structured::answer;
use crate::der::Value;
use crate::trust::names::Match;

/// A JID's three parts, each enforced.
#[derive(Debug, PartialEq, Eq)]
struct Jid {
    local: Option<String>,
    domain: String,
    resource: Option<String>,
}

/// RFC 7622 section 3.3.1: code points the `IdentifierClass` allows and a localpart does not.
const EXCLUDED_FROM_LOCALPART: &[char] = &['"', '&', '\'', '/', ':', '<', '>', '@'];

fn sized(part: String) -> Option<String> {
    (!part.is_empty() && part.len() <= MAX_PART).then_some(part)
}

fn localpart(text: &str) -> Option<String> {
    let enforced = username_case_mapped(text)?;
    if enforced.contains(EXCLUDED_FROM_LOCALPART) {
        return None;
    }
    sized(enforced)
}

/// RFC 5890 section 2.3.1's NR-LDH label: letters, digits and hyphen, neither beginning nor
/// ending with a hyphen and with no hyphens in its third and fourth positions.
fn nr_ldh(label: &str) -> bool {
    let bytes = label.as_bytes();
    !label.is_empty()
        && label.len() <= MAX_LABEL
        && bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'-')
        && bytes.first() != Some(&b'-')
        && bytes.last() != Some(&b'-')
        && bytes.get(2..4) != Some(b"--")
}

fn domainpart(text: &str) -> Option<String> {
    let text = text.strip_suffix('.').unwrap_or(text);
    if text.starts_with('[') {
        // An IP literal (RFC 3986 section 3.2.2, RFC 6874), which section 3.2 compares as written.
        return (text.is_ascii() && text.ends_with(']'))
            .then(|| sized(text.to_owned()))
            .flatten();
    }
    // RFC 5895 section 2's steps 1 to 3, the three section 3.2.2 applies.
    let lowered = text.to_lowercase();
    let mapped: String = lowered
        .chars()
        .map(width)
        .collect::<String>()
        .nfc()
        .collect();
    let labels: Vec<String> = mapped
        .split('.')
        .map(|label| {
            if !label.is_ascii() {
                (valid_u_label(label) && to_a_label(label).is_some()).then(|| label.to_owned())
            } else if has_ace_prefix(label) {
                from_a_label(label)
            } else {
                nr_ldh(label).then(|| label.to_owned())
            }
        })
        .collect::<Option<_>>()?;
    if !bidi_holds(labels.iter().map(String::as_str)) {
        return None;
    }
    sized(labels.join("."))
}

/// RFC 7622 section 3: `text` as an enforced JID, or `None` where it is none.
fn normalized(text: &str) -> Option<Jid> {
    let (rest, resource) = match text.split_once('/') {
        Some((rest, resource)) => (rest, Some(resource)),
        None => (text, None),
    };
    let (local, domain) = match rest.split_once('@') {
        Some((local, domain)) => (Some(local), domain),
        None => (None, rest),
    };
    let local = match local {
        Some(text) => Some(localpart(text)?),
        None => None,
    };
    let resource = match resource {
        Some(text) => Some(sized(opaque_string(text)?)?),
        None => None,
    };
    Some(Jid {
        local,
        domain: domainpart(domain)?,
        resource,
    })
}

/// Clause 8.9.4's `jidMatch`.
pub(super) fn jid(carried: &Value<'_>, stated: &Value<'_>) -> Match {
    let enforced = |value: &Value<'_>| {
        (value.identifier == UTF8_STRING)
            .then(|| std::str::from_utf8(value.contents).ok())
            .flatten()
            .and_then(normalized)
    };
    match (enforced(carried), enforced(stated)) {
        (Some(carried), Some(stated)) => answer(carried == stated),
        _ => Match::Undecided,
    }
}
