//! The signer attributes a signature carries, read in both syntaxes, and TS 119 172 clause 4.26's
//! constraints on one set of them judged directly; each syntax's own tests drive the same
//! judgement through its parser.

use super::*;
use crate::der::{self, SET};
use crate::policy_rules::tests::{oid, seq, signed_data, tagged, tlv};

/// `id-at-role`, `2.5.4.72` (ITU-T X.520), the attribute EN 319 122-1 clause 5.2.6.1's NOTE 1
/// names for a claimed role.
pub(in crate::policy_rules) const ROLE: &[u8] = &[0x55, 0x04, 0x48];
/// `id-at-title`, `2.5.4.12` (ITU-T X.520): a second attribute type.
pub(in crate::policy_rules) const TITLE: &[u8] = &[0x55, 0x04, 0x0C];

/// An X.501 `Attribute ::= SEQUENCE { type, values SET OF }` with one `UTF8String` value.
fn attribute(kind: &[u8]) -> Vec<u8> {
    seq(&[oid(kind), tlv(SET, &tlv(0x0C, b"x"))])
}

/// An attribute certificate's place: this reader counts it and never opens it, so its contents
/// are an empty `SEQUENCE`.
fn certificate() -> Vec<u8> {
    seq(&[])
}

/// EN 319 122-1 clause 5.2.6.1's `SignerAttributeV2`, `EXPLICIT TAGS`: each member's context tag
/// wraps its `SEQUENCE OF`, and each `CertifiedAttributesV2` entry's tag wraps its certificate.
pub(in crate::policy_rules) fn v2(
    claimed: &[&[u8]],
    certified: usize,
    assertions: usize,
) -> Vec<u8> {
    let mut members = Vec::new();
    if !claimed.is_empty() {
        let list: Vec<_> = claimed.iter().map(|kind| attribute(kind)).collect();
        members.push(tagged(0, &seq(&list)));
    }
    if certified > 0 {
        let list: Vec<_> = (0..certified).map(|_| tagged(0, &certificate())).collect();
        members.push(tagged(1, &seq(&list)));
    }
    if assertions > 0 {
        // `SignedAssertion ::= SEQUENCE { signedAssertionID, signedAssertion OPTIONAL }`, with
        // clause 5.2.6.3's `id-ets-signedSAML` (`0.4.0.19122.3.0`).
        let assertion = seq(&[oid(&[0x04, 0x00, 0x81, 0x95, 0x32, 0x03, 0x00])]);
        members.push(tagged(2, &seq(&vec![assertion; assertions])));
    }
    seq(&members)
}

/// A signature whose only signer-attributes attribute is a signed one of `kind` holding `value`.
pub(in crate::policy_rules) fn carrying<'a>(kind: &'a [u8], value: &'a [u8]) -> SignedData<'a> {
    signed_data(
        vec![cms::ID_CONTENT_TYPE, cms::ID_MESSAGE_DIGEST, kind],
        vec![],
        vec![cms::Attribute {
            kind,
            signed: true,
            values: 1,
            first: der::Reader::new(value)
                .expect("a fixture")
                .next_value()
                .expect("well formed"),
        }],
    )
}

#[test]
fn every_member_of_signer_attributes_v2_is_counted_by_its_kind() {
    // EN 319 122-1 clause 5.2.6.1: claimed attributes, attribute certificates, signed assertions.
    let value = v2(&[ROLE, TITLE], 2, 1);
    assert_eq!(
        carried(&carrying(cms::ID_AA_ETS_SIGNER_ATTR_V2, &value)),
        Ok(Carried {
            claimed: vec![ROLE, TITLE],
            certified: 2,
            assertions: 1,
        })
    );
}

#[test]
fn the_first_signer_attributes_is_read_as_a_list_of_claimed_and_certified() {
    // RFC 5126 section 5.11.3: `SEQUENCE OF CHOICE { claimedAttributes [0], certifiedAttributes
    // [1] AttributeCertificate }`, explicitly tagged.
    let value = seq(&[
        tagged(0, &seq(&[attribute(ROLE)])),
        tagged(1, &certificate()),
        tagged(0, &seq(&[attribute(TITLE)])),
    ]);
    assert_eq!(
        carried(&carrying(cms::ID_AA_ETS_SIGNER_ATTR, &value)),
        Ok(Carried {
            claimed: vec![ROLE, TITLE],
            certified: 1,
            assertions: 0,
        })
    );
}

#[test]
fn a_value_that_is_not_its_clauses_syntax_is_unreadable() {
    // Implicitly tagged, against the module's `EXPLICIT TAGS`: the attribute sits directly in [0].
    let implicit = seq(&[tagged(0, &attribute(ROLE))]);
    // Members out of the order `SignerAttributeV2` declares them in.
    let reordered = seq(&[
        tagged(1, &seq(&[tagged(0, &certificate())])),
        tagged(0, &seq(&[attribute(ROLE)])),
    ]);
    // A `CertifiedAttributesV2` entry that is neither alternative.
    let neither = seq(&[tagged(1, &seq(&[certificate()]))]);
    for value in [implicit, reordered, neither] {
        assert_eq!(
            carried(&carrying(cms::ID_AA_ETS_SIGNER_ATTR_V2, &value)),
            Err(Unreadable::Malformed)
        );
    }
    // A signed attribute with two values: both clauses require one.
    let value = v2(&[ROLE], 0, 0);
    let mut two = carrying(cms::ID_AA_ETS_SIGNER_ATTR_V2, &value);
    two.attributes[0].values = 2;
    assert_eq!(carried(&two), Err(Unreadable::NotOneValue));
}

#[test]
fn an_unsigned_copy_is_not_read_and_the_bound_holds_at_its_edge() {
    let value = v2(&[ROLE], 1, 0);
    let mut unsigned = carrying(cms::ID_AA_ETS_SIGNER_ATTR_V2, &value);
    unsigned.attributes[0].signed = false;
    assert_eq!(carried(&unsigned), Ok(Carried::default()));
    let at_bound = v2(&[ROLE; MAX_SIGNER_ATTRIBUTES], 0, 0);
    assert_eq!(
        carried(&carrying(cms::ID_AA_ETS_SIGNER_ATTR_V2, &at_bound))
            .map(|carried| carried.claimed.len()),
        Ok(MAX_SIGNER_ATTRIBUTES)
    );
    let past = v2(&[ROLE; MAX_SIGNER_ATTRIBUTES], 1, 0);
    assert_eq!(
        carried(&carrying(cms::ID_AA_ETS_SIGNER_ATTR_V2, &past)),
        Err(Unreadable::TooMany)
    );
}

fn set(accepted: Accepted, mandated: &[(&'static [u8], usize)]) -> SetConstraints<'static> {
    SetConstraints {
        accepted,
        trust_trees: false,
        revocation: false,
        mandated: mandated
            .iter()
            .map(|(kind, value_constraints)| Mandated {
                identifier: Identifier::Oid(kind),
                value_constraints: *value_constraints,
            })
            .collect(),
    }
}

/// The verdicts of each kind of check, in the order the findings came.
fn judged(set: &SetConstraints<'_>, value: &[u8]) -> Vec<(Verdict, &'static str)> {
    let mut out = Vec::new();
    judge_set(
        set,
        &carrying(cms::ID_AA_ETS_SIGNER_ATTR_V2, value),
        &mut out,
    );
    out.iter()
        .map(|finding| {
            let Constraint::SignerAttribute(check) = &finding.constraint else {
                panic!("{finding:?}");
            };
            let kind = match check {
                Check::Unreadable(_) => "unreadable",
                Check::Mandated { .. } => "mandated",
                Check::MustBePresent { .. } => "present",
                Check::Level { .. } => "level",
                Check::Certified { .. } => "certified",
                Check::Values { .. } | Check::ConditionValues => "values",
            };
            (finding.verdict, kind)
        })
        .collect()
}

#[test]
fn a_set_that_identifies_nothing_holds_every_signer_attribute_to_its_level() {
    // TS 119 172-2 clause 4.26.1: without an identifier, the level applies to any signer
    // attribute, so an attribute certificate under a claimed-only level breaks it.
    let claimed = set(Accepted::Claimed, &[]);
    assert_eq!(
        judged(&claimed, &v2(&[ROLE], 0, 0)),
        [(Verdict::Met, "level")]
    );
    assert_eq!(
        judged(&claimed, &v2(&[ROLE], 1, 0)),
        [(Verdict::Broken, "level")]
    );
    // Under `any`, the certificate is accepted and its validity is what is left.
    assert_eq!(
        judged(&set(Accepted::Any, &[]), &v2(&[], 1, 1)),
        [
            (Verdict::Met, "level"),
            (Verdict::NotEvaluated, "certified")
        ]
    );
}

#[test]
fn an_identified_attribute_must_be_carried_at_the_level_the_set_accepts() {
    // Clause 4.26.1 item 4: the identified attribute shall be incorporated, and it shall meet
    // item 1's level. Claimed where only certified is accepted breaks both.
    let certified = set(Accepted::Certified, &[(ROLE, 0)]);
    assert_eq!(
        judged(&certified, &v2(&[ROLE], 0, 0)),
        [(Verdict::Broken, "level"), (Verdict::Broken, "present")]
    );
    // A certificate might carry it; only a reader of it could say.
    assert_eq!(
        judged(&certified, &v2(&[], 1, 0)),
        [
            (Verdict::Met, "level"),
            (Verdict::NotEvaluated, "present"),
            (Verdict::NotEvaluated, "certified"),
        ]
    );
    // Nothing carried: the requirement is broken.
    assert_eq!(
        judged(&certified, &seq(&[])),
        [(Verdict::Met, "level"), (Verdict::Broken, "present")]
    );
}

#[test]
fn a_set_that_identifies_types_constrains_only_those() {
    // The set names the role; a claimed title is outside it, so the level holds and the role is
    // missing.
    let claimed = set(Accepted::Claimed, &[(ROLE, 0)]);
    assert_eq!(
        judged(&claimed, &v2(&[TITLE], 0, 0)),
        [(Verdict::Met, "level"), (Verdict::Broken, "present")]
    );
    // A certificate beside the claimed role may certify a role too, at a level the set refuses.
    assert_eq!(
        judged(&claimed, &v2(&[ROLE], 1, 0)),
        [(Verdict::NotEvaluated, "level"), (Verdict::Met, "present")]
    );
    // A value constraint on a carried attribute is named, never met.
    assert_eq!(
        judged(&set(Accepted::Claimed, &[(ROLE, 1)]), &v2(&[ROLE], 0, 0)),
        [
            (Verdict::Met, "level"),
            (Verdict::NotEvaluated, "values"),
            (Verdict::Met, "present"),
        ]
    );
}

#[test]
fn unreadable_signer_attributes_are_named_and_never_judged() {
    let implicit = seq(&[tagged(0, &attribute(ROLE))]);
    assert_eq!(
        judged(&set(Accepted::Claimed, &[(ROLE, 0)]), &implicit),
        [(Verdict::NotEvaluated, "unreadable")]
    );
}
