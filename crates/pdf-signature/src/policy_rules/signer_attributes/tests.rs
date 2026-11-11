//! The signer attributes a signature carries, read in both syntaxes, and TS 119 172 clause 4.26's
//! constraints on one set of them judged directly; each syntax's own tests drive the same
//! judgement through its parser.

use super::*;
use crate::attribute_certificate::tests::fixtures::{AA, AA_ROOT, AC, AC_OTHER_HOLDER};
use crate::attribute_certificate::tests::hex;
use crate::der::{self, SET};
use crate::policy_rules::certification_path::search;
use crate::policy_rules::certification_path::tests::{WITHIN, hierarchy, value as read_value};
use crate::policy_rules::tests::{facts, oid, seq, signed_data, tagged, tlv};
use crate::x509;
use certified::tests::trees;

/// `id-at-role`, `2.5.4.72` (ITU-T X.520), the attribute EN 319 122-1 clause 5.2.6.1's NOTE 1
/// names for a claimed role.
pub(in crate::policy_rules) const ROLE: &[u8] = &[0x55, 0x04, 0x48];
/// `id-at-title`, `2.5.4.12` (ITU-T X.520): a second attribute type.
pub(in crate::policy_rules) const TITLE: &[u8] = &[0x55, 0x04, 0x0C];

/// An X.501 `Attribute ::= SEQUENCE { type, values SET OF }` with one `UTF8String` value.
fn attribute(kind: &[u8]) -> Vec<u8> {
    seq(&[oid(kind), tlv(SET, &tlv(0x0C, b"x"))])
}

/// An attribute certificate: the fixture whose holder is the fixture signer, carrying a role.
fn certificate() -> Vec<u8> {
    hex(AC)
}

/// EN 319 122-1 clause 5.2.6.1's `SignerAttributeV2`, `EXPLICIT TAGS`: each member's context tag
/// wraps its `SEQUENCE OF`, and each `CertifiedAttributesV2` entry's tag wraps its certificate.
pub(in crate::policy_rules) fn v2(
    claimed: &[&[u8]],
    certified: usize,
    assertions: usize,
) -> Vec<u8> {
    v2_carrying(claimed, &vec![certificate(); certified], assertions)
}

/// [`v2`] with these attribute certificates.
pub(in crate::policy_rules) fn v2_carrying(
    claimed: &[&[u8]],
    certificates: &[Vec<u8>],
    assertions: usize,
) -> Vec<u8> {
    let certified = certificates.len();
    let mut members = Vec::new();
    if !claimed.is_empty() {
        let list: Vec<_> = claimed.iter().map(|kind| attribute(kind)).collect();
        members.push(tagged(0, &seq(&list)));
    }
    if certified > 0 {
        let list: Vec<_> = certificates
            .iter()
            .map(|certificate| tagged(0, certificate))
            .collect();
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
    let read = carried(&carrying(cms::ID_AA_ETS_SIGNER_ATTR_V2, &value)).expect("readable");
    assert_eq!(read.claimed, vec![ROLE, TITLE]);
    assert_eq!(read.assertions, 1);
    // Each certificate is the `AttributeCertificate` its `[0]` wraps, under `EXPLICIT TAGS`.
    let certificate = certificate();
    let wrapped: Vec<&[u8]> = read.certified.iter().map(Value::encoding).collect();
    assert_eq!(wrapped, vec![certificate.as_slice(); 2]);
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
    let read = carried(&carrying(cms::ID_AA_ETS_SIGNER_ATTR, &value)).expect("readable");
    assert_eq!(read.claimed, vec![ROLE, TITLE]);
    assert_eq!(read.certified.len(), 1);
    assert_eq!(read.assertions, 0);
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
        trust_trees: None,
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

/// The verdicts of each kind of check, in the order the findings came, for a signature that
/// carries no certificate.
fn judged(set: &SetConstraints<'_>, value: &[u8]) -> Vec<(Verdict, &'static str)> {
    let facts = facts(None);
    let context = Context {
        facts: &facts,
        signer_path: None,
        signer_trees: None,
    };
    let mut out = Vec::new();
    judge_set(
        set,
        &carrying(cms::ID_AA_ETS_SIGNER_ATTR_V2, value),
        context,
        &mut out,
    );
    kinds(&out)
}

/// Each finding's verdict and kind.
fn kinds(out: &[Finding]) -> Vec<(Verdict, &'static str)> {
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
                Check::Certificate { .. } => "certificate",
                Check::Assertions { .. } => "assertions",
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
    // Under `any`, the certificate is accepted and its validity is what is left; this signature
    // carries neither the signer's certificate nor the authority's, so it is undecided.
    assert_eq!(
        judged(&set(Accepted::Any, &[]), &v2(&[], 1, 1)),
        [
            (Verdict::Met, "level"),
            (Verdict::NotEvaluated, "certificate"),
            (Verdict::NotEvaluated, "assertions"),
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
    // A certificate carries a role, and whether it is valid is undecided here.
    assert_eq!(
        judged(&certified, &v2(&[], 1, 0)),
        [
            (Verdict::Met, "level"),
            (Verdict::NotEvaluated, "present"),
            (Verdict::NotEvaluated, "certificate"),
        ]
    );
    // A certificate that carries no title cannot supply the one mandated, whatever its validity,
    // and the set does not apply to it.
    assert_eq!(
        judged(&set(Accepted::Certified, &[(TITLE, 0)]), &v2(&[], 1, 0)),
        [(Verdict::Met, "level"), (Verdict::Broken, "present")]
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
    // A certificate beside the claimed role certifies a role too, at a level the set refuses:
    // clause 4.26.1 holds every identified signer attribute to the level, so the set is broken.
    assert_eq!(
        judged(&claimed, &v2(&[ROLE], 1, 0)),
        [(Verdict::Broken, "level"), (Verdict::Met, "present")]
    );
    // The same certificate under a set naming the title is outside it.
    assert_eq!(
        judged(&set(Accepted::Claimed, &[(TITLE, 0)]), &v2(&[TITLE], 1, 0)),
        [(Verdict::Met, "level"), (Verdict::Met, "present")]
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

/// A signature carrying `value` as its signed `signer-attributes-v2`, the signer's certificate,
/// its intermediate and the attribute authority's certificate, with the signer's path searched to
/// the signer's root; `judge` runs over it.
fn with_hierarchy(
    value: &[u8],
    judge: impl FnOnce(&SignedData<'_>, Context<'_>, &mut Vec<Finding>),
) -> Vec<(Verdict, &'static str)> {
    let held = hierarchy();
    let authority = hex(AA);
    let mut signature = carrying(cms::ID_AA_ETS_SIGNER_ATTR_V2, value);
    let signer = x509::parse(&held.signer).expect("the fixture signer");
    signature.certificates = vec![
        read_value(&held.signer),
        read_value(&held.intermediate),
        read_value(&authority),
    ];
    signature.signer_issuer_and_serial = Some((signer.issuer, signer.serial_number));
    let facts = facts(None).validated_at(WITHIN);
    let signer_trees = trees(&[&held.root]);
    let signer_path = search(&signer_trees, &signature, &facts);
    let context = Context {
        facts: &facts,
        signer_path: Some(&signer_path),
        signer_trees: Some(&signer_trees),
    };
    let mut out = Vec::new();
    judge(&signature, context, &mut out);
    kinds(&out)
}

#[test]
fn a_valid_certificate_certifies_the_attribute_a_set_mandates() {
    // TS 119 172-2 clause 4.26.1 items 2 and 4: the role certified at the level the set accepts,
    // by a certificate valid under RFC 5755 section 5 whose authority reaches the set's trust tree.
    let mut certified = set(Accepted::Certified, &[(ROLE, 0)]);
    certified.trust_trees = Some(trees(&[&hex(AA_ROOT)]));
    let judge = |value: &[u8]| {
        with_hierarchy(value, |signature, context, out| {
            judge_set(&certified, signature, context, out);
        })
    };
    assert_eq!(
        judge(&v2(&[], 1, 0)),
        [
            (Verdict::Met, "level"),
            (Verdict::Met, "present"),
            (Verdict::Met, "certificate"),
        ]
    );
    // Its holder is somebody else: it certifies nothing of the signer's, and every signer
    // attribute must meet the set's requirements, so both are broken.
    assert_eq!(
        judge(&v2_carrying(&[], &[hex(AC_OTHER_HOLDER)], 0)),
        [
            (Verdict::Met, "level"),
            (Verdict::Broken, "present"),
            (Verdict::Broken, "certificate"),
        ]
    );
}

#[test]
fn a_condition_without_trees_of_its_own_takes_the_signing_certificates() {
    // TR 102 272 clause 6.9: where `attrCertificateTrustTrees` is absent, the rules of the
    // signing-certificate condition apply, so the authority's path is searched to its trees.
    let condition = |types: Option<Vec<&'static [u8]>>| AttributeTrust {
        mandated: true,
        accepted: Accepted::Certified,
        trust_trees: None,
        revocation: false,
        types,
        value_types: None,
    };
    let judge = |condition: &AttributeTrust<'_>, roots: &[&[u8]]| {
        with_hierarchy(&v2(&[], 1, 0), |signature, context, out| {
            let signer_trees = trees(roots);
            let context = Context {
                signer_trees: Some(&signer_trees),
                ..context
            };
            judge_condition(condition, signature, context, out);
        })
    };
    let held = hierarchy();
    let authority_root = hex(AA_ROOT);
    let both: &[&[u8]] = &[&held.root, &authority_root];
    assert_eq!(
        judge(&condition(None), both),
        [(Verdict::Met, "mandated"), (Verdict::Met, "certificate")]
    );
    // The signer's trees alone do not reach the authority: indeterminate, so neither is decided.
    assert_eq!(
        judge(&condition(None), &[&held.root]),
        [
            (Verdict::NotEvaluated, "mandated"),
            (Verdict::NotEvaluated, "certificate"),
        ]
    );
    // Valid, but certifying a role where the condition considers only titles valid.
    assert_eq!(
        judge(&condition(Some(vec![TITLE])), both),
        [(Verdict::Broken, "mandated"), (Verdict::Met, "certificate")]
    );
}
