//! ETSI TR 102 272 clause 6.9's `attributeValueConstraints`: a signer attribute whose type only
//! that list names is within the attribute trust condition where its values are among the values
//! the list states (ADR 2895). ETSI's text is cited and paraphrased (ADR 1085).
//!
//! # Equal by the type's own rule
//!
//! Each entry of the list is an X.501 `AttributeTypeAndValue`, and whether a carried value is the
//! value an entry states is ITU-T X.501 clause 8.9.2.1's evaluation of an attribute value
//! assertion: undefined where the type is unknown, where it has no equality matching rule, or where
//! the value does not conform to the syntax of the rule's assertion; true where the values match by
//! the type's equality matching rule; false otherwise. X.501 clause 13.4.7 says that an attribute
//! type without an equality matching rule has its values never compared, and clause 13.4.2 that a
//! subtype keeps its supertype's matching rules. So the rule is a fact about each type, which ITU-T
//! X.520 states in each type's definition. An undefined answer is [`Match::Undecided`]: the
//! condition it reaches is not evaluated, never met and never broken.
//!
//! # The types and their rules
//!
//! [`rule`] holds the rule of every X.520 type in the `id-at` arc that states one, its collective
//! subtypes included (ADR 3158): the string rules of clause 8.1 and `telephoneNumberMatch` and
//! `facsimileNumberMatch` of clause 8.2 over clause 7's preparation (`prepare`), and the rules
//! over structured syntaxes (`structured`). What stays undefined, and why:
//!
//! - **Types with no equality rule** (`searchGuide`, `telexNumber`, `preferredDeliveryMethod`,
//!   `enhancedSearchGuide`, `uiiFormat`, `utmCoordinates`, `epcFormat`, `tagLocation` and the
//!   collective `telexNumber`): X.501 clause 13.4.7, never compared. **`id-at-role`** (2.5.4.72)
//!   is one of them: X.520 leaves its definition to ITU-T X.509, whose clause 16.5.1 defines it
//!   over `RoleSyntax` with no equality matching rule and leaves its values' specification outside
//!   that Recommendation (ADR 3161).
//! - **A value outside its rule's syntax**: `countryCode3n`'s `NumericString` under
//!   `caseIgnoreMatch`, whose syntax is `UnboundedDirectoryString`, and `dc`'s `IA5String`, whose
//!   rule `caseIgnoreIA5Match` X.520 names and does not define; a `TeletexString`, which clause 7.1
//!   lets this program decline to transcode; a constructed (BER) bit or octet string; and a URI, a
//!   domain name, an e-mail address or a Jabber identifier that is not one by the RFCs its type
//!   names (`uri`, `domain`, `jid`).
//! - **A clear-text password against an encrypted one** under `userPwdMatch` (clause 8.10.1): the
//!   clear text is to be encrypted by the other side's algorithm, and no Recommendation defines
//!   how a password is encrypted by any algorithm (`structured::user_pwd_match`, ADR 3161).
//!
//! The rules over internationalised identifiers — clause 8.9.1's `uriMatch`, 8.9.2's
//! `dnsNameMatch`, 8.9.3's `intEmailMatch` and 8.9.4's `jidMatch` — are written from the RFCs the
//! clauses name, over `code_points`' derived properties and `punycode` (ADR 3161).

mod code_points;
mod domain;
mod jid;
mod precis;
mod prepare;
mod punycode;
mod structured;
mod uri;

use crate::der::{SEQUENCE, SET, Value};
use crate::trust::names::Match;
use prepare::{
    DIRECTORY_STRING, Insignificant, NUMERIC_STRING, PRINTABLE_STRING, UTF8_STRING, prepared,
};

/// The equality matching rule of an attribute type, where this program holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    /// X.520 clause 8.1.1's `caseIgnoreMatch`.
    CaseIgnore,
    /// X.520 clause 8.1.1's `caseExactMatch`.
    CaseExact,
    /// X.520 clause 8.1.4's `numericStringMatch`.
    NumericString,
    /// X.520 clause 8.1.7's `caseIgnoreListMatch`.
    CaseIgnoreList,
    /// X.520 clause 8.2.4's `bitStringMatch`.
    BitString,
    /// X.520 clause 8.2.5's `octetStringMatch`.
    OctetString,
    /// X.520 clause 8.2.8's `telephoneNumberMatch`.
    TelephoneNumber,
    /// X.520 clause 8.2.11's `uniqueMemberMatch`.
    UniqueMember,
    /// X.520 clause 8.2.12's `protocolInformationMatch`.
    ProtocolInformation,
    /// X.520 clause 8.2.13's `facsimileNumberMatch`.
    FacsimileNumber,
    /// X.520 clause 8.2.15's `uUIDPairMatch`.
    UuidPair,
    /// X.501 clause 13.4.7's `objectIdentifierMatch`.
    ObjectIdentifier,
    /// X.501 clause 13.4.7's `distinguishedNameMatch`.
    DistinguishedName,
    /// X.520 clause 8.2.10's `presentationAddressMatch`.
    PresentationAddress,
    /// X.520 clause 8.9.1's `uriMatch`.
    Uri,
    /// X.520 clause 8.9.2's `dnsNameMatch`.
    DnsName,
    /// X.520 clause 8.9.3's `intEmailMatch`.
    IntEmail,
    /// X.520 clause 8.9.4's `jidMatch`.
    Jid,
    /// X.520 clause 8.10.1's `userPwdMatch`.
    UserPwd,
}

/// Every `id-at` type X.520 states a rule for that this program holds, by the arcs below `id-at`
/// (2.5.4): one arc, or two for a collective subtype (`collectiveLocalityName` is 2.5.4.7.1), each
/// with the rule its definition states or its supertype's (X.501 clause 13.4.2).
pub const RULES: &[(&[u8], Rule)] = &[
    // `name` (41) and its subtypes: `commonName`, `surname`, `countryName`, `localityName`,
    // `stateOrProvinceName`, `organizationName`, `organizationalUnitName`, `title`, `givenName`,
    // `initials`, `generationQualifier`, `dmdName`, `pseudonym`, `countryCode3c` and
    // `countryCode3n`, and the collective subtypes of four of them.
    (&[41], Rule::CaseIgnore),
    (&[3], Rule::CaseIgnore),
    (&[4], Rule::CaseIgnore),
    (&[6], Rule::CaseIgnore),
    (&[7], Rule::CaseIgnore),
    (&[7, 1], Rule::CaseIgnore),
    (&[8], Rule::CaseIgnore),
    (&[8, 1], Rule::CaseIgnore),
    (&[10], Rule::CaseIgnore),
    (&[10, 1], Rule::CaseIgnore),
    (&[11], Rule::CaseIgnore),
    (&[11, 1], Rule::CaseIgnore),
    (&[12], Rule::CaseIgnore),
    (&[42], Rule::CaseIgnore),
    (&[43], Rule::CaseIgnore),
    (&[44], Rule::CaseIgnore),
    (&[54], Rule::CaseIgnore),
    (&[65], Rule::CaseIgnore),
    (&[98], Rule::CaseIgnore),
    (&[99], Rule::CaseIgnore),
    // The types stating `caseIgnoreMatch` themselves, and their collective subtypes:
    // `knowledgeInformation`, `serialNumber`, `streetAddress`, `description`, `businessCategory`,
    // `postalCode`, `postOfficeBox`, `physicalDeliveryOfficeName`, `destinationIndicator`,
    // `dnQualifier`, `houseIdentifier` and `organizationIdentifier`.
    (&[2], Rule::CaseIgnore),
    (&[5], Rule::CaseIgnore),
    (&[9], Rule::CaseIgnore),
    (&[9, 1], Rule::CaseIgnore),
    (&[13], Rule::CaseIgnore),
    (&[15], Rule::CaseIgnore),
    (&[17], Rule::CaseIgnore),
    (&[17, 1], Rule::CaseIgnore),
    (&[18], Rule::CaseIgnore),
    (&[18, 1], Rule::CaseIgnore),
    (&[19], Rule::CaseIgnore),
    (&[19, 1], Rule::CaseIgnore),
    (&[27], Rule::CaseIgnore),
    (&[46], Rule::CaseIgnore),
    (&[51], Rule::CaseIgnore),
    (&[97], Rule::CaseIgnore),
    // `urnC`.
    (&[89], Rule::CaseExact),
    // `x121Address`, `internationalISDNNumber` and its collective subtype.
    (&[24], Rule::NumericString),
    (&[25], Rule::NumericString),
    (&[25, 1], Rule::NumericString),
    // `postalAddress`, its collective subtype, and `registeredAddress`.
    (&[16], Rule::CaseIgnoreList),
    (&[16, 1], Rule::CaseIgnoreList),
    (&[26], Rule::CaseIgnoreList),
    // `uniqueIdentifier`, `uii` and `epc`.
    (&[45], Rule::BitString),
    (&[90], Rule::BitString),
    (&[91], Rule::BitString),
    // `userPassword` and `tagAfi`.
    (&[35], Rule::OctetString),
    (&[92], Rule::OctetString),
    // `telephoneNumber` and its collective subtype.
    (&[20], Rule::TelephoneNumber),
    (&[20, 1], Rule::TelephoneNumber),
    // `facsimileTelephoneNumber` and its collective subtype.
    (&[23], Rule::FacsimileNumber),
    (&[23, 1], Rule::FacsimileNumber),
    (&[48], Rule::ProtocolInformation),
    (&[50], Rule::UniqueMember),
    (&[77], Rule::UuidPair),
    // `supportedApplicationContext`, `communicationsService`, `communicationsNetwork`, `tagOid`
    // and `objectIdentifier`.
    (&[30], Rule::ObjectIdentifier),
    (&[66], Rule::ObjectIdentifier),
    (&[67], Rule::ObjectIdentifier),
    (&[78], Rule::ObjectIdentifier),
    (&[106], Rule::ObjectIdentifier),
    // `member`, `owner`, `roleOccupant`, `seeAlso` and `distinguishedName`.
    (&[31], Rule::DistinguishedName),
    (&[32], Rule::DistinguishedName),
    (&[33], Rule::DistinguishedName),
    (&[34], Rule::DistinguishedName),
    (&[49], Rule::DistinguishedName),
    (&[29], Rule::PresentationAddress),
    // `uri` and its subtypes `urn`, `url`, `uiiInUrn`, `contentUrl`, `epcInUrn` and `ldapUrl`.
    (&[83], Rule::Uri),
    (&[86], Rule::Uri),
    (&[87], Rule::Uri),
    (&[80], Rule::Uri),
    (&[81], Rule::Uri),
    (&[94], Rule::Uri),
    (&[95], Rule::Uri),
    (&[100], Rule::DnsName),
    (&[104], Rule::IntEmail),
    (&[105], Rule::Jid),
    (&[85], Rule::UserPwd),
];

/// The equality matching rule X.520 states for `kind`, an object identifier's contents, where it
/// is one this program holds.
#[must_use]
pub fn rule(kind: &[u8]) -> Option<Rule> {
    let arcs = kind.strip_prefix(&[0x55, 0x04])?;
    // Every arc below `id-at` that `RULES` names is below 128, so each is one octet of X.690's
    // encoding and the comparison is of the arcs themselves.
    RULES
        .iter()
        .find(|(listed, _)| *listed == arcs)
        .map(|&(_, rule)| rule)
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
                    match (best, equal(kind, &value, &entry.value, 0)) {
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

/// X.501 clause 8.9.2.1 for one value of type `kind` the signer carries against one value the
/// policy presents, `depth` names deep inside a distinguished name.
fn equal(kind: &[u8], carried: &Value<'_>, stated: &Value<'_>, depth: u8) -> Match {
    let Some(rule) = rule(kind) else {
        return Match::Undecided;
    };
    match rule {
        Rule::CaseIgnore => strings(
            carried,
            stated,
            DIRECTORY_STRING,
            true,
            Insignificant::Spaces,
        ),
        Rule::CaseExact => strings(
            carried,
            stated,
            DIRECTORY_STRING,
            false,
            Insignificant::Spaces,
        ),
        Rule::NumericString => strings(
            carried,
            stated,
            &[NUMERIC_STRING],
            true,
            Insignificant::AllSpaces,
        ),
        Rule::TelephoneNumber => strings(
            carried,
            stated,
            &[PRINTABLE_STRING],
            true,
            Insignificant::HyphensAndSpaces,
        ),
        Rule::FacsimileNumber => {
            // Clause 8.2.13: the presented `TelephoneNumber` against the attribute value's first
            // element, by `telephoneNumberMatch`; the parameters are not evaluated.
            match first_of(carried) {
                Some(number) => strings(
                    &number,
                    stated,
                    &[PRINTABLE_STRING],
                    true,
                    Insignificant::HyphensAndSpaces,
                ),
                None => Match::Undecided,
            }
        }
        Rule::CaseIgnoreList => case_ignore_list(carried, stated),
        Rule::BitString => structured::bit_string(carried, stated),
        Rule::OctetString => structured::octet_string(carried, stated),
        Rule::ProtocolInformation => {
            // Clause 8.2.12: the presented octet string against the stored `nAddress`, by
            // `octetStringMatch`.
            match first_of(carried) {
                Some(address) => structured::octet_string(&address, stated),
                None => Match::Undecided,
            }
        }
        Rule::UuidPair => structured::uuid_pair(carried, stated),
        Rule::ObjectIdentifier => structured::object_identifier(carried, stated),
        Rule::DistinguishedName => structured::distinguished_name(carried, stated, depth, equal),
        Rule::UniqueMember => structured::unique_member(carried, stated, depth, equal),
        Rule::PresentationAddress => structured::presentation_address_match(carried, stated),
        Rule::Uri => uri::uri(carried, stated),
        Rule::DnsName => domain::dns_name(carried, stated),
        Rule::IntEmail => domain::int_email(carried, stated),
        Rule::Jid => jid::jid(carried, stated),
        Rule::UserPwd => structured::user_pwd_match(carried, stated, |stored, presented| {
            strings(
                stored,
                presented,
                &[UTF8_STRING],
                false,
                Insignificant::Spaces,
            )
        }),
    }
}

/// The first member of a `SEQUENCE`, which is where clauses 8.2.12 and 8.2.13 take the element
/// they compare.
fn first_of<'a>(value: &Value<'a>) -> Option<Value<'a>> {
    if value.identifier != SEQUENCE {
        return None;
    }
    value.children().ok()?.next_value().ok()?
}

/// Two strings by a clause 8.1 string rule: true where clause 7 prepares them to the same string.
fn strings(
    carried: &Value<'_>,
    stated: &Value<'_>,
    syntax: &[u8],
    fold: bool,
    removal: Insignificant,
) -> Match {
    match (
        prepared(carried, syntax, fold, removal),
        prepared(stated, syntax, fold, removal),
    ) {
        (Some(carried), Some(stated)) => {
            if carried == stated {
                Match::Yes
            } else {
                Match::No
            }
        }
        _ => Match::Undecided,
    }
}

/// Clause 8.1.7: true "if, and only if the number of strings in each is the same, and the
/// corresponding strings match" as `caseIgnoreMatch` does.
fn case_ignore_list(carried: &Value<'_>, stated: &Value<'_>) -> Match {
    let (Some(carried), Some(stated)) =
        (structured::sequence(carried), structured::sequence(stated))
    else {
        return Match::Undecided;
    };
    let pairs = carried
        .iter()
        .zip(&stated)
        .map(|(left, right)| strings(left, right, DIRECTORY_STRING, true, Insignificant::Spaces));
    let answer = pairs.fold(Match::Yes, |sum, next| match (sum, next) {
        (Match::No, _) | (_, Match::No) => Match::No,
        (Match::Undecided, _) | (_, Match::Undecided) => Match::Undecided,
        _ => Match::Yes,
    });
    if carried.len() == stated.len() {
        answer
    } else {
        Match::No
    }
}

#[cfg(test)]
mod identifier_tests;
#[cfg(test)]
mod tests;
