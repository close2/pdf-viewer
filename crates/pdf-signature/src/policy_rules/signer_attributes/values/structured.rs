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
pub(super) fn answer(equal: bool) -> Match {
    if equal { Match::Yes } else { Match::No }
}

/// X.501 clause 8.9.2.1's conjunction: one false is false, otherwise one undefined is undefined.
pub(super) fn all(answers: impl IntoIterator<Item = Match>) -> Match {
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

/// X.520 clause 6.9.1's `PresentationAddress`, under its module's explicit tags: the three
/// optional selectors `[0]` to `[2]`, each an `OCTET STRING`, and `[3]`'s non-empty `SET OF`
/// network addresses. Later members, which the type's extension marker admits, are not read.
struct PresentationAddress<'a> {
    selectors: [Option<&'a [u8]>; 3],
    addresses: Vec<&'a [u8]>,
}

fn presentation_address<'a>(value: &Value<'a>) -> Option<PresentationAddress<'a>> {
    let members = components(value).filter(|_| value.identifier == SEQUENCE)?;
    let mut selectors = [None; 3];
    let mut addresses = None;
    let mut last = None;
    for member in &members {
        let number = (0..4).find(|&number| member.identifier == 0xA0 | number)?;
        // The members come in their tags' order, each at most once.
        if last.is_some_and(|last| number <= last) {
            return None;
        }
        last = Some(number);
        let inner = components_of(member)?;
        let [inner] = inner.as_slice() else {
            return None;
        };
        if number < 3 {
            if inner.identifier != OCTET_STRING {
                return None;
            }
            selectors[usize::from(number)] = Some(inner.contents);
        } else {
            let set = components(inner).filter(|_| inner.identifier == SET)?;
            if set.is_empty() || set.iter().any(|address| address.identifier != OCTET_STRING) {
                return None;
            }
            addresses = Some(set.iter().map(|address| address.contents).collect());
        }
    }
    Some(PresentationAddress {
        selectors,
        addresses: addresses?,
    })
}

/// The values an explicitly tagged member holds.
fn components_of<'a>(value: &Value<'a>) -> Option<Vec<Value<'a>>> {
    let mut reader = value.children().ok()?;
    let mut out = Vec::new();
    while let Some(next) = reader.next_value().ok()? {
        out.push(next);
    }
    Some(out)
}

/// X.520 clause 8.2.10's `presentationAddressMatch`: true where the presented and the stored
/// address have equal selectors — a selector one states and the other does not being unequal —
/// and every presented network address is among the stored ones. The policy's value is the
/// presented one (ADR 3158).
pub(super) fn presentation_address_match(carried: &Value<'_>, stated: &Value<'_>) -> Match {
    match (presentation_address(carried), presentation_address(stated)) {
        (Some(stored), Some(presented)) => answer(
            stored.selectors == presented.selectors
                && presented
                    .addresses
                    .iter()
                    .all(|address| stored.addresses.contains(address)),
        ),
        _ => Match::Undecided,
    }
}

/// X.520 clause 6.14.2's `UserPwd`: the clear text, or the `encrypted` sequence's algorithm
/// identifier and encrypted string.
enum UserPwd<'a> {
    Clear(Value<'a>),
    Encrypted {
        algorithm: Value<'a>,
        encrypted: Value<'a>,
    },
}

fn user_pwd<'a>(value: &Value<'a>) -> Option<UserPwd<'a>> {
    match value.identifier {
        super::prepare::UTF8_STRING => Some(UserPwd::Clear(*value)),
        SEQUENCE => {
            let members = components(value)?;
            match members.as_slice() {
                [algorithm, encrypted, ..]
                    if algorithm.identifier == SEQUENCE && encrypted.identifier == OCTET_STRING =>
                {
                    Some(UserPwd::Encrypted {
                        algorithm: *algorithm,
                        encrypted: *encrypted,
                    })
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// X.520 clause 8.10.1's `userPwdMatch`, the policy's value presented (ADR 3158). Two clear
/// texts compare by `caseExactMatch`, which `clear` is; two encrypted passwords are unequal where
/// their algorithm identifiers and parameters differ — compared as DER encodes them, one encoding
/// to a value — and otherwise compare by `octetStringMatch`. A clear text against an encrypted
/// password is the clear text encrypted by the other's algorithm, and no algorithm for that is
/// defined: clause 6.14.2 sends the reader to ITU-T X.509 Annex B, which lists none for passwords,
/// and clause 6.15.15's `pwdEncAlg` to an Annex L X.520 does not contain. That comparison is
/// undefined (ADR 3161).
pub(super) fn user_pwd_match(
    carried: &Value<'_>,
    stated: &Value<'_>,
    clear: impl Fn(&Value<'_>, &Value<'_>) -> Match,
) -> Match {
    match (user_pwd(carried), user_pwd(stated)) {
        (Some(UserPwd::Clear(stored)), Some(UserPwd::Clear(presented))) => {
            clear(&stored, &presented)
        }
        (
            Some(UserPwd::Encrypted {
                algorithm: stored_algorithm,
                encrypted: stored,
            }),
            Some(UserPwd::Encrypted {
                algorithm: presented_algorithm,
                encrypted: presented,
            }),
        ) => {
            if stored_algorithm.contents == presented_algorithm.contents {
                octet_string(&stored, &presented)
            } else {
                Match::No
            }
        }
        _ => Match::Undecided,
    }
}
