//! X.501 clause 8.9.2.1 over the values of TR 102 272 clause 6.9's `attributeValueConstraints`,
//! each expected value read off ITU-T X.520 clauses 7 and 8.1.1 and the type definitions of its
//! clause 6; the examples are X.520's own.

use super::*;
use crate::der::Reader;
use crate::policy_rules::tests::{seq, tlv};

/// `id-at-title`, 2.5.4.12: a subtype of `name`, so `caseIgnoreMatch` (X.520 clause 6.4.3).
const TITLE: &[u8] = &[0x55, 0x04, 0x0C];
/// `id-at-role`, 2.5.4.72: X.520 leaves its definition to ITU-T X.509.
const ROLE: &[u8] = &[0x55, 0x04, 0x48];
/// `id-at-dc`, 0.9.2342.19200300.100.1.25: `caseIgnoreMatch` stated over an `IA5String` syntax.
const DC: &[u8] = &[0x09, 0x92, 0x26, 0x89, 0x93, 0xF2, 0x2C, 0x64, 0x01, 0x19];

const UTF8: u8 = 0x0C;
const PRINTABLE: u8 = 0x13;
const IA5: u8 = 0x16;
const TELETEX: u8 = 0x14;
const BMP: u8 = 0x1E;

fn value(bytes: &[u8]) -> Value<'_> {
    Reader::new(bytes)
        .expect("small")
        .next_value()
        .expect("well formed")
        .expect("one value")
}

/// The answer for an attribute of `kind` holding `carried` against `stated` values of the type.
fn answer(kind: &[u8], carried: &[Vec<u8>], stated: &[Vec<u8>]) -> Match {
    let set = tlv(SET, &carried.concat());
    let listed: Vec<Listed<'_>> = stated
        .iter()
        .map(|bytes| Listed {
            kind,
            value: value(bytes),
        })
        .collect();
    within(kind, &value(&set), &listed)
}

fn string(tag: u8, text: &str) -> Vec<u8> {
    tlv(tag, text.as_bytes())
}

#[test]
fn case_ignore_match_folds_case_and_insignificant_spaces() {
    // X.520 clause 8.1.1: "Dundee" and "DUNDEE" match.
    let dundee = [string(UTF8, "Dundee")];
    assert_eq!(
        answer(TITLE, &[string(UTF8, "DUNDEE")], &dundee),
        Match::Yes
    );
    // Clause 7.6.1's example: the leading, trailing and doubled spaces of
    // "<SPACE><SPACE>foo<SPACE><SPACE>bar<SPACE><SPACE>" go, leaving "foo<SPACE>bar".
    assert_eq!(
        answer(
            TITLE,
            &[string(UTF8, "  foo  bar  ")],
            &[string(PRINTABLE, "foo bar")]
        ),
        Match::Yes
    );
    // Clause 7.2: a tab is mapped to a space before the spaces are counted.
    assert_eq!(
        answer(
            TITLE,
            &[string(UTF8, "foo\tbar")],
            &[string(UTF8, "foo bar")]
        ),
        Match::Yes
    );
    // The space inside a value is significant.
    assert_eq!(
        answer(TITLE, &[string(UTF8, "foobar")], &[string(UTF8, "foo bar")]),
        Match::No
    );
    // X.520 clause 6.4.3's example title, against another.
    let manager = [string(UTF8, "Manager, Distributed Applications")];
    assert_eq!(
        answer(
            TITLE,
            &[tlv(
                BMP,
                &"manager, distributed applications"
                    .encode_utf16()
                    .flat_map(u16::to_be_bytes)
                    .collect::<Vec<_>>()
            )],
            &manager
        ),
        Match::Yes,
        "a BMPString is transcoded before it is compared"
    );
    assert_eq!(
        answer(TITLE, &[string(UTF8, "Director")], &manager),
        Match::No
    );
}

#[test]
fn every_value_must_be_listed_and_one_listed_value_is_enough() {
    let listed = [string(UTF8, "Director"), string(UTF8, "Manager")];
    assert_eq!(
        answer(TITLE, &[string(UTF8, "manager")], &listed),
        Match::Yes,
        "the second listed value matches"
    );
    assert_eq!(
        answer(
            TITLE,
            &[string(UTF8, "manager"), string(UTF8, "director")],
            &listed
        ),
        Match::Yes
    );
    assert_eq!(
        answer(
            TITLE,
            &[string(UTF8, "manager"), string(UTF8, "clerk")],
            &listed
        ),
        Match::No,
        "one value the policy does not consider valid"
    );
    // A value of the right type listed only for another type decides nothing for this one.
    let set = tlv(SET, &string(UTF8, "manager"));
    let other = [Listed {
        kind: ROLE,
        value: value(&listed[1]),
    }];
    assert_eq!(within(TITLE, &value(&set), &other), Match::No);
}

#[test]
fn an_undefined_comparison_is_undecided_never_met_or_broken() {
    let listed = [string(UTF8, "Manager")];
    // X.501 clause 8.9.2.1 (a)(1) and (2): a type whose rule this program does not hold.
    assert_eq!(
        answer(ROLE, &[string(UTF8, "Manager")], &listed),
        Match::Undecided
    );
    // (a)(3): `dc`'s IA5String is not caseIgnoreMatch's DirectoryString syntax; and a title
    // written as an IA5String is no DirectoryString either.
    assert_eq!(
        answer(DC, &[string(IA5, "example")], &[string(IA5, "example")]),
        Match::Undecided
    );
    assert_eq!(
        answer(TITLE, &[string(IA5, "Manager")], &listed),
        Match::Undecided
    );
    // Clause 7.1: this program declines to transcode a TeletexString.
    assert_eq!(
        answer(TITLE, &[string(TELETEX, "Manager")], &listed),
        Match::Undecided
    );
    // Clause 7.4: an empty string is prohibited.
    assert_eq!(
        answer(TITLE, &[string(UTF8, "")], &listed),
        Match::Undecided
    );
    // Clause 7.4: a combining character first is prohibited.
    assert_eq!(
        answer(TITLE, &[string(UTF8, "\u{301}Manager")], &listed),
        Match::Undecided
    );
    // An undecided value is not outweighed by a decided one, but a value matching nothing is.
    assert_eq!(
        answer(
            TITLE,
            &[string(UTF8, "manager"), string(TELETEX, "Manager")],
            &listed
        ),
        Match::Undecided
    );
    assert_eq!(
        answer(
            TITLE,
            &[string(UTF8, "clerk"), string(TELETEX, "Manager")],
            &listed
        ),
        Match::No
    );
    // No value at all, or values that are not a SET.
    assert_eq!(answer(TITLE, &[], &listed), Match::Undecided);
    let not_a_set = seq(&[string(UTF8, "Manager")]);
    assert_eq!(
        within(
            TITLE,
            &value(&not_a_set),
            &[Listed {
                kind: TITLE,
                value: value(&listed[0])
            }]
        ),
        Match::Undecided
    );
}

#[test]
fn the_rule_is_held_for_every_listed_type_and_no_other() {
    for (arcs, held) in RULES {
        let mut kind = vec![0x55, 0x04];
        kind.extend_from_slice(arcs);
        assert_eq!(rule(&kind), Some(*held), "2.5.4.{arcs:?}");
    }
    // `name` and `title` state `caseIgnoreMatch`, `collectiveLocalityName` keeps its supertype's,
    // `uniqueIdentifier` is a bit string, and `telephoneNumber` has its own rule.
    assert_eq!(rule(&[0x55, 0x04, 41]), Some(Rule::CaseIgnore));
    assert_eq!(rule(TITLE), Some(Rule::CaseIgnore));
    assert_eq!(rule(&[0x55, 0x04, 7, 1]), Some(Rule::CaseIgnore));
    assert_eq!(rule(&[0x55, 0x04, 45]), Some(Rule::BitString));
    assert_eq!(rule(&[0x55, 0x04, 20]), Some(Rule::TelephoneNumber));
    // `searchGuide` (14) and `telexNumber` (21) have no equality rule; `role` (72) is not X.520's;
    // `uri` (83), `dnsName` (100) and `jid` (105) have rules this program has not built.
    for last in [14, 21, 72, 83, 100, 105] {
        assert_eq!(rule(&[0x55, 0x04, last]), None, "2.5.4.{last}");
    }
    assert_eq!(rule(DC), None);
}

/// A string as a `BMPString`, two octets per UTF-16 unit.
fn bmp(text: &str) -> Vec<u8> {
    let units: Vec<u8> = text.encode_utf16().flat_map(u16::to_be_bytes).collect();
    tlv(BMP, &units)
}

/// X.520 clause 7 outside ASCII: clause 7.2's mapping and RFC 3454 table B.2's folding, clause
/// 7.3's Form KC, clause 7.4's prohibitions, each as the clause states it.
#[test]
fn clause_7_prepares_text_outside_ascii() {
    let one =
        |carried: &[u8], stated: &[u8]| answer(TITLE, &[carried.to_vec()], &[stated.to_vec()]);
    // B.2 folds the capitals with their diaeresis, and Form KC composes the decomposed ones.
    assert_eq!(
        one(
            &string(UTF8, "Gesch\u{e4}ftsf\u{fc}hrer"),
            &string(UTF8, "GESCHA\u{308}FTSFU\u{308}HRER")
        ),
        Match::Yes
    );
    // B.2 maps U+00DF to "ss"; the same text as a BMPString transcodes to the same string.
    assert_eq!(
        one(&bmp("Stra\u{df}e"), &string(PRINTABLE, "STRASSE")),
        Match::Yes
    );
    // Form KC takes the fullwidth forms to ASCII.
    assert_eq!(
        one(
            &string(
                UTF8,
                "\u{ff2d}\u{ff41}\u{ff4e}\u{ff41}\u{ff47}\u{ff45}\u{ff52}"
            ),
            &string(UTF8, "manager")
        ),
        Match::Yes
    );
    // SOFT HYPHEN is mapped to nothing, NO-BREAK SPACE (Zs) to SPACE.
    assert_eq!(
        one(&string(UTF8, "Man\u{ad}ager"), &string(UTF8, "manager")),
        Match::Yes
    );
    assert_eq!(
        one(
            &string(UTF8, "Chief\u{a0}\u{a0}Clerk"),
            &string(UTF8, "chief clerk")
        ),
        Match::Yes
    );
    // Another letter is another string.
    assert_eq!(
        one(&string(UTF8, "Gesch\u{e4}ft"), &string(UTF8, "Geschaft")),
        Match::No
    );
    // Private use and unassigned code points are prohibited, so the assertion is undefined.
    assert_eq!(
        one(&string(UTF8, "Man\u{e000}"), &string(UTF8, "man")),
        Match::Undecided
    );
    assert_eq!(
        one(&string(UTF8, "Man\u{378}"), &string(UTF8, "man")),
        Match::Undecided
    );
    // Clause 8.1.1's `caseExactMatch` (`urnC`, 2.5.4.89) prepares without folding.
    let urn_c: &[u8] = &[0x55, 0x04, 89];
    assert_eq!(
        answer(
            urn_c,
            &[string(PRINTABLE, "Urn C")],
            &[string(PRINTABLE, " Urn  C ")]
        ),
        Match::Yes
    );
    assert_eq!(
        answer(
            urn_c,
            &[string(PRINTABLE, "Urn C")],
            &[string(PRINTABLE, "urn c")]
        ),
        Match::No
    );
}

const NUMERIC: u8 = 0x12;

/// Clause 8.1.4 removes every space (clause 7.6.2) over `NumericString`; clause 8.2.8 removes
/// hyphens and spaces over `TelephoneNumber`; clause 8.2.13 compares a presented telephone number
/// with the first element of a stored facsimile number.
#[test]
fn numeric_and_telephone_rules_drop_their_insignificant_characters() {
    let x121: &[u8] = &[0x55, 0x04, 24];
    assert_eq!(
        answer(
            x121,
            &[string(NUMERIC, "1234 5678")],
            &[string(NUMERIC, "12345678")]
        ),
        Match::Yes
    );
    assert_eq!(
        answer(
            x121,
            &[string(NUMERIC, "1234 5679")],
            &[string(NUMERIC, "12345678")]
        ),
        Match::No
    );
    // A `PrintableString` is not the rule's syntax.
    assert_eq!(
        answer(
            x121,
            &[string(PRINTABLE, "12345678")],
            &[string(NUMERIC, "12345678")]
        ),
        Match::Undecided
    );
    let telephone: &[u8] = &[0x55, 0x04, 20];
    assert_eq!(
        answer(
            telephone,
            &[string(PRINTABLE, "+1 555-0100")],
            &[string(PRINTABLE, "+15550100")]
        ),
        Match::Yes
    );
    assert_eq!(
        answer(
            telephone,
            &[string(PRINTABLE, "+1 555-0101")],
            &[string(PRINTABLE, "+15550100")]
        ),
        Match::No
    );
    let facsimile: &[u8] = &[0x55, 0x04, 23];
    let stored = seq(&[string(PRINTABLE, "+43 1 234-5"), tlv(0x03, &[0x00])]);
    assert_eq!(
        answer(
            facsimile,
            std::slice::from_ref(&stored),
            &[string(PRINTABLE, "+4312345")]
        ),
        Match::Yes
    );
    assert_eq!(
        answer(facsimile, &[stored], &[string(PRINTABLE, "+4312346")]),
        Match::No
    );
}

/// Clause 8.1.7: as many strings, each matching by `caseIgnoreMatch`.
#[test]
fn a_case_ignore_list_matches_string_for_string() {
    let postal: &[u8] = &[0x55, 0x04, 16];
    let stored = seq(&[string(UTF8, "10 Main St"), string(UTF8, "Vienna")]);
    assert_eq!(
        answer(
            postal,
            std::slice::from_ref(&stored),
            &[seq(&[
                string(PRINTABLE, "10 main st"),
                string(UTF8, " VIENNA ")
            ])]
        ),
        Match::Yes
    );
    assert_eq!(
        answer(
            postal,
            std::slice::from_ref(&stored),
            &[seq(&[string(UTF8, "10 Main St")])]
        ),
        Match::No
    );
    assert_eq!(
        answer(
            postal,
            &[stored],
            &[seq(&[string(UTF8, "10 Main St"), string(UTF8, "Graz")])]
        ),
        Match::No
    );
}

/// X.520 clause 8.2.4: the same number of bits, bit for bit, unused bits ignored; clause 8.2.5:
/// the same octets; X.501 clause 13.4.7: the same arcs; clause 8.2.15: both UUIDs equal; clause
/// 8.2.12: the presented octet string against the stored `nAddress`.
#[test]
fn the_structured_rules_compare_what_their_clauses_state() {
    let unique: &[u8] = &[0x55, 0x04, 45];
    // Two bits `11`, once with the six unused bits clear and once not.
    assert_eq!(
        answer(unique, &[tlv(0x03, &[6, 0xC0])], &[tlv(0x03, &[6, 0xC1])]),
        Match::Yes
    );
    // One bit `1` is a different length.
    assert_eq!(
        answer(unique, &[tlv(0x03, &[6, 0xC0])], &[tlv(0x03, &[7, 0x80])]),
        Match::No
    );
    let afi: &[u8] = &[0x55, 0x04, 92];
    assert_eq!(
        answer(afi, &[tlv(0x04, &[1, 2])], &[tlv(0x04, &[1, 2])]),
        Match::Yes
    );
    assert_eq!(
        answer(afi, &[tlv(0x04, &[1, 2])], &[tlv(0x04, &[1, 2, 0])]),
        Match::No
    );
    let context: &[u8] = &[0x55, 0x04, 30];
    assert_eq!(
        answer(
            context,
            &[tlv(0x06, &[0x55, 0x04, 3])],
            &[tlv(0x06, &[0x55, 0x04, 3])]
        ),
        Match::Yes
    );
    assert_eq!(
        answer(
            context,
            &[tlv(0x06, &[0x55, 0x04, 3])],
            &[tlv(0x06, &[0x55, 0x04, 4])]
        ),
        Match::No
    );
    let pair: &[u8] = &[0x55, 0x04, 77];
    let uuids = |last: u8| seq(&[tlv(0x04, &[0x11; 16]), tlv(0x04, &[last; 16])]);
    assert_eq!(answer(pair, &[uuids(0x22)], &[uuids(0x22)]), Match::Yes);
    assert_eq!(answer(pair, &[uuids(0x22)], &[uuids(0x23)]), Match::No);
    // A UUID of fifteen octets is not the syntax.
    let short = seq(&[tlv(0x04, &[0x11; 16]), tlv(0x04, &[0x22; 15])]);
    assert_eq!(
        answer(
            pair,
            std::slice::from_ref(&short),
            std::slice::from_ref(&short)
        ),
        Match::Undecided
    );
    let protocol: &[u8] = &[0x55, 0x04, 48];
    let stored = seq(&[tlv(0x04, &[9, 9]), tlv(SET, &tlv(0x06, &[0x55, 0x04, 3]))]);
    assert_eq!(
        answer(
            protocol,
            std::slice::from_ref(&stored),
            &[tlv(0x04, &[9, 9])]
        ),
        Match::Yes
    );
    assert_eq!(answer(protocol, &[stored], &[tlv(0x04, &[9])]), Match::No);
}

/// One RDN of one `AttributeTypeAndValue`.
fn rdn(kind: &[u8], value: Vec<u8>) -> Vec<u8> {
    tlv(SET, &seq(&[tlv(0x06, kind), value]))
}

const COMMON_NAME: &[u8] = &[0x55, 0x04, 0x03];

/// X.501 clause 13.4.7's `distinguishedNameMatch`, each value by its own type's rule, and X.520
/// clause 8.2.11's `uniqueMemberMatch`, whose `uid` may be absent from the attribute value.
#[test]
fn names_match_rdn_for_rdn_by_each_value_s_own_rule() {
    let member: &[u8] = &[0x55, 0x04, 31];
    let alice = seq(&[
        rdn(COMMON_NAME, string(UTF8, "Alice")),
        rdn(TITLE, string(UTF8, "Clerk")),
    ]);
    let shouted = seq(&[
        rdn(COMMON_NAME, string(PRINTABLE, "ALICE")),
        rdn(TITLE, string(UTF8, " clerk")),
    ]);
    assert_eq!(
        answer(member, std::slice::from_ref(&alice), &[shouted]),
        Match::Yes
    );
    let bob = seq(&[
        rdn(COMMON_NAME, string(UTF8, "Bob")),
        rdn(TITLE, string(UTF8, "Clerk")),
    ]);
    assert_eq!(
        answer(member, std::slice::from_ref(&alice), &[bob]),
        Match::No
    );
    let shorter = seq(&[rdn(COMMON_NAME, string(UTF8, "Alice"))]);
    assert_eq!(
        answer(member, std::slice::from_ref(&alice), &[shorter]),
        Match::No
    );
    // A value of a type whose rule is not held leaves the name undefined.
    let roles = seq(&[rdn(ROLE, string(UTF8, "Clerk"))]);
    assert_eq!(
        answer(
            member,
            std::slice::from_ref(&roles),
            std::slice::from_ref(&roles)
        ),
        Match::Undecided
    );

    let unique: &[u8] = &[0x55, 0x04, 50];
    let uid = tlv(0x03, &[0, 0xAB]);
    let with_uid = seq(&[alice.clone(), uid.clone()]);
    let without = seq(&[alice]);
    assert_eq!(
        answer(
            unique,
            std::slice::from_ref(&without),
            std::slice::from_ref(&with_uid)
        ),
        Match::Yes
    );
    assert_eq!(
        answer(
            unique,
            std::slice::from_ref(&with_uid),
            std::slice::from_ref(&with_uid)
        ),
        Match::Yes
    );
    assert_eq!(answer(unique, &[with_uid], &[without]), Match::No);
}
