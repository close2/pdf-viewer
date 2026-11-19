//! The equality matching rules whose syntax is not a string: ITU-T X.520 clause 8.2's
//! `bitStringMatch`, `octetStringMatch`, `uniqueMemberMatch` and `uUIDPairMatch`, and ITU-T X.501
//! clause 13.4.7's `objectIdentifierMatch` and `distinguishedNameMatch`, which X.520 clause 8's
//! note leaves to X.501. Each takes the attribute value the signer carries and the value the
//! policy presents, and answers X.501 clause 8.9.2.1's three ways; a value that is not its rule's
//! syntax is undefined. The ITU-T texts are cited and paraphrased (ADR 1085).

use crate::der::{OBJECT_IDENTIFIER, OCTET_STRING, SEQUENCE, SET, Value};
use crate::trust::names::Match;

/// A `BIT STRING`'s universal tag, primitive.
const BIT_STRING: u8 = 0x03;

/// How deep a distinguished name may hold another before the comparison is undefined: a
/// `member` value inside a name is a name, and the recursion is bounded rather than trusted.
pub(super) const MAX_NESTING: u8 = 4;

/// `true` and `false` as an answer.
fn answer(equal: bool) -> Match {
    if equal { Match::Yes } else { Match::No }
}

/// X.501 clause 8.9.2.1's conjunction: one false is false, otherwise one undefined is undefined.
fn all(answers: impl IntoIterator<Item = Match>) -> Match {
    answers
        .into_iter()
        .fold(Match::Yes, |sum, next| match (sum, next) {
            (Match::No, _) | (_, Match::No) => Match::No,
            (Match::Undecided, _) | (_, Match::Undecided) => Match::Undecided,
            _ => Match::Yes,
        })
}

/// X.520 clause 8.2.4: true where the attribute value "has the same number of bits as the
/// presented value and the bits match on a bitwise basis". None of the types this rule serves
/// here is defined with a `NamedBitList`, so trailing zero bits count. A constructed (BER)
/// encoding is not read, and is undefined.
pub(super) fn bit_string(carried: &Value<'_>, stated: &Value<'_>) -> Match {
    match (bits(carried), bits(stated)) {
        (Some(carried), Some(stated)) => answer(carried == stated),
        _ => Match::Undecided,
    }
}

/// A primitive `BIT STRING`'s bits, as its length in bits and its octets with the unused bits of
/// the last cleared, so that two encodings of the same bits compare equal.
fn bits(value: &Value<'_>) -> Option<(usize, Vec<u8>)> {
    if value.identifier != BIT_STRING {
        return None;
    }
    let (&unused, octets) = value.contents.split_first()?;
    if unused > 7 || (octets.is_empty() && unused != 0) {
        return None;
    }
    let length = octets
        .len()
        .checked_mul(8)?
        .checked_sub(usize::from(unused))?;
    let mut octets = octets.to_vec();
    if let Some(last) = octets.last_mut() {
        *last &= 0xFF_u8 << unused;
    }
    Some((length, octets))
}

/// X.520 clause 8.2.5: true "if, and only if the strings are the same length and corresponding
/// octets are identical". A constructed encoding is undefined, as for bit strings.
pub(super) fn octet_string(carried: &Value<'_>, stated: &Value<'_>) -> Match {
    if carried.identifier != OCTET_STRING || stated.identifier != OCTET_STRING {
        return Match::Undecided;
    }
    answer(carried.contents == stated.contents)
}

/// X.501 clause 13.4.7: two object identifiers match "if and only if they both have the same
/// number of integral components, and each integral component of the first is equal to the
/// corresponding component of the second". X.690's encoding of an identifier is a bijection
/// of its components, so equal contents are equal components.
pub(super) fn object_identifier(carried: &Value<'_>, stated: &Value<'_>) -> Match {
    if carried.identifier != OBJECT_IDENTIFIER || stated.identifier != OBJECT_IDENTIFIER {
        return Match::Undecided;
    }
    answer(carried.contents == stated.contents)
}

/// X.520 clause 8.2.15: each component of the presented `UUIDPair` "equal to the corresponding
/// component of the second", of the same length and with equal octets. The type is extensible, so
/// every component is compared, and both must hold the same number.
pub(super) fn uuid_pair(carried: &Value<'_>, stated: &Value<'_>) -> Match {
    match (components(carried), components(stated)) {
        (Some(carried), Some(stated)) => {
            let uuids = |list: &[Value<'_>]| {
                list.len() >= 2
                    && list[..2]
                        .iter()
                        .all(|uuid| uuid.identifier == OCTET_STRING && uuid.contents.len() == 16)
            };
            if !uuids(&carried) || !uuids(&stated) {
                return Match::Undecided;
            }
            answer(
                carried.len() == stated.len()
                    && carried.iter().zip(&stated).all(|(left, right)| {
                        left.identifier == right.identifier && left.contents == right.contents
                    }),
            )
        }
        _ => Match::Undecided,
    }
}

/// The members of a `SEQUENCE` or `SET`, or `None` where `value` is neither or does not read.
fn components<'a>(value: &Value<'a>) -> Option<Vec<Value<'a>>> {
    if value.identifier != SEQUENCE && value.identifier != SET {
        return None;
    }
    let mut reader = value.children().ok()?;
    let mut out = Vec::new();
    while let Some(next) = reader.next_value().ok()? {
        out.push(next);
    }
    Some(out)
}

/// X.501 clause 13.4.7's `distinguishedNameMatch`: true where "the number of RDNs in each is the
/// same", "corresponding RDNs have the same number of `AttributeTypeAndValue`", and the values of
/// corresponding `AttributeTypeAndValue`s — those of identical types in corresponding RDNs — match
/// by their own type's equality rule (X.501 clause 9.4), which `equal` is.
pub(super) fn distinguished_name(
    carried: &Value<'_>,
    stated: &Value<'_>,
    depth: u8,
    equal: fn(&[u8], &Value<'_>, &Value<'_>, u8) -> Match,
) -> Match {
    if depth >= MAX_NESTING || carried.identifier != SEQUENCE || stated.identifier != SEQUENCE {
        return Match::Undecided;
    }
    let (Some(carried), Some(stated)) = (rdns(carried), rdns(stated)) else {
        return Match::Undecided;
    };
    if carried.len() != stated.len() {
        return Match::No;
    }
    all(carried.iter().zip(&stated).map(|(target, presented)| {
        if target.len() != presented.len() {
            return Match::No;
        }
        all(presented.iter().map(|(kind, value)| {
            // The best answer any target member of the same type gives.
            target
                .iter()
                .filter(|(other, _)| other == kind)
                .fold(Match::No, |best, (_, other)| {
                    match (best, equal(kind, other, value, depth.saturating_add(1))) {
                        (Match::Yes, _) | (_, Match::Yes) => Match::Yes,
                        (Match::Undecided, _) | (_, Match::Undecided) => Match::Undecided,
                        _ => Match::No,
                    }
                })
        }))
    }))
}

/// A `NameAndOptionalUID`'s `dn` and `uid`.
fn name_and_uid<'a>(value: &Value<'a>) -> Option<(Value<'a>, Option<Value<'a>>)> {
    match sequence(value)?.as_slice() {
        [dn] => Some((*dn, None)),
        [dn, uid] => Some((*dn, Some(*uid))),
        _ => None,
    }
}

/// A `SEQUENCE`'s members, or `None` where `value` is not one or does not read.
pub(super) fn sequence<'a>(value: &Value<'a>) -> Option<Vec<Value<'a>>> {
    if value.identifier == SEQUENCE {
        components(value)
    } else {
        None
    }
}

/// One RDN's members, as type contents and value.
type Rdn<'a> = Vec<(&'a [u8], Value<'a>)>;

/// An `RDNSequence`'s RDNs, each a non-empty `SET` of `AttributeTypeAndValue`; `None` where the
/// name does not read as one.
fn rdns<'a>(name: &Value<'a>) -> Option<Vec<Rdn<'a>>> {
    components(name)?
        .iter()
        .map(|rdn| {
            if rdn.identifier != SET {
                return None;
            }
            let members = components(rdn)?;
            if members.is_empty() {
                return None;
            }
            members
                .iter()
                .map(|member| match components(member)?.as_slice() {
                    [kind, value]
                        if member.identifier == SEQUENCE
                            && kind.identifier == OBJECT_IDENTIFIER =>
                    {
                        Some((kind.contents, *value))
                    }
                    _ => None,
                })
                .collect()
        })
        .collect()
}

/// X.520 clause 8.2.11: `NameAndOptionalUID ::= SEQUENCE { dn DistinguishedName, uid
/// UniqueIdentifier OPTIONAL }`. True "if, and only if the dn components of the attribute value
/// and the presented value match according to the distinguishedNameMatch rule, and the uid
/// component is absent from the attribute value or matches the corresponding component from the
/// presented value according to the bitStringMatch rule" — so the attribute value's `uid` is the
/// one whose absence is enough, and a presented value without one does not match one present.
pub(super) fn unique_member(
    carried: &Value<'_>,
    stated: &Value<'_>,
    depth: u8,
    equal: fn(&[u8], &Value<'_>, &Value<'_>, u8) -> Match,
) -> Match {
    let (Some((carried_dn, carried_uid)), Some((stated_dn, stated_uid))) =
        (name_and_uid(carried), name_and_uid(stated))
    else {
        return Match::Undecided;
    };
    let names = distinguished_name(&carried_dn, &stated_dn, depth, equal);
    let uid = match (carried_uid, stated_uid) {
        (None, _) => Match::Yes,
        (Some(carried), Some(stated)) => bit_string(&carried, &stated),
        (Some(_), None) => Match::No,
    };
    all([names, uid])
}
