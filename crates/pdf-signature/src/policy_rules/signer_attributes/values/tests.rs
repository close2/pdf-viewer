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
    // Outside ASCII the case folding and normalisation tables are needed.
    assert_eq!(
        answer(TITLE, &[string(UTF8, "Gesch\u{e4}ftsf\u{fc}hrer")], &listed),
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
    // X.520 clause 6's `name` and its subtypes, and the types stating `caseIgnoreMatch`.
    for last in CASE_IGNORE {
        assert_eq!(
            rule(&[0x55, 0x04, last]),
            Some(Rule::CaseIgnore),
            "2.5.4.{last}"
        );
    }
    // `uniqueIdentifier` (45) is a bit string, `telephoneNumber` (20) has its own rule, and
    // `role` (72) is not X.520's.
    for last in [45, 20, 72] {
        assert_eq!(rule(&[0x55, 0x04, last]), None, "2.5.4.{last}");
    }
    assert_eq!(rule(DC), None);
}
