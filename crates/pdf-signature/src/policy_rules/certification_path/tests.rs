//! A signature policy's trust condition searched over the fixture hierarchy [`crate::trust`]'s own
//! tests issued: a root, one intermediate and a signer, every key held by this project, so a path
//! whose positive outcome is known in advance exists and each negative changes one thing beside it
//! (trap 13). No corpus signature states a machine-readable policy (ADR 1709), so every policy here
//! is constructed, and each expected value is read off the clause the test names.

use std::borrow::Cow;

use super::{Found, MAX_TRUST_POINTS, Search, TrustPoint, TrustTrees, search, verdict};
use crate::cms::{Digest, SignedData};
use crate::der::{Reader, Value};
use crate::policy_rules::tests::{facts, seq, signed_data, tlv};
use crate::policy_rules::{SignerFacts, Stated, Verdict};
use crate::trust::PathRefusal;
use crate::trust::tests::fixtures::{INTERMEDIATE, ROOT, SIGNER};
use crate::x509::{self, Instant};

/// 2026-06-01T00:00:00Z, inside every fixture certificate's validity period.
pub(crate) const WITHIN: Instant = Instant::from_unix_seconds(1_780_272_000);
/// 2027-06-01T00:00:00Z, after the fixture signer expired.
const AFTER_THE_SIGNER_EXPIRED: Instant = Instant::from_unix_seconds(1_811_808_000);

pub(crate) fn hex(text: &str) -> Vec<u8> {
    let digits: Vec<u8> = text.bytes().filter(u8::is_ascii_hexdigit).collect();
    digits
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair).expect("hex digits");
            u8::from_str_radix(text, 16).expect("hex digits")
        })
        .collect()
}

pub(crate) fn value(bytes: &[u8]) -> Value<'_> {
    Reader::new(bytes)
        .expect("well formed")
        .next_value()
        .expect("well formed")
        .expect("one value")
}

/// The fixture signer's `SignedData`, carrying the signer's certificate and the intermediate.
pub(crate) fn cms<'a>(signer: &'a [u8], intermediate: &'a [u8]) -> SignedData<'a> {
    let leaf = x509::parse(signer).expect("the fixture signer");
    let mut cms = signed_data(vec![], vec![], vec![]);
    cms.certificates = vec![value(signer), value(intermediate)];
    cms.signer_issuer_and_serial = Some((leaf.issuer, leaf.serial_number));
    cms
}

fn at(instant: Instant) -> SignerFacts<'static> {
    facts(None).validated_at(instant)
}

fn point(certificate: &[u8]) -> TrustPoint<'_> {
    TrustPoint {
        certificate: Cow::Borrowed(certificate),
        path_length: None,
        reliable_until: None,
        inputs: crate::trust::PathInputs::default(),
        unprocessed: Vec::new(),
        mandated_path: None,
    }
}

fn trees(points: Vec<TrustPoint<'_>>) -> TrustTrees<'_> {
    TrustTrees {
        points,
        trusted_lists: 0,
    }
}

pub(crate) struct Hierarchy {
    pub(crate) root: Vec<u8>,
    pub(crate) intermediate: Vec<u8>,
    pub(crate) signer: Vec<u8>,
}

pub(crate) fn hierarchy() -> Hierarchy {
    Hierarchy {
        root: hex(ROOT),
        intermediate: hex(INTERMEDIATE),
        signer: hex(SIGNER),
    }
}

fn searched(trees: &TrustTrees<'_>, held: &Hierarchy, facts: &SignerFacts<'_>) -> Search {
    search(trees, &cms(&held.signer, &held.intermediate), facts)
}

#[test]
fn a_path_to_the_policys_own_trust_point_meets_the_condition() {
    let held = hierarchy();
    let found = searched(&trees(vec![point(&held.root)]), &held, &at(WITHIN));
    assert_eq!(
        found.found,
        Found::Reached {
            length: 2,
            unprocessed: Vec::new(),
            evaluated: Vec::new(),
            policies: None,
            mandated: None,
        },
        "TR 102 272 clause 6.6.1: the trust point starts path processing, and RFC 5280 section 6.1 validates the signer and its intermediate under it"
    );
    assert_eq!(found.verdict(), Verdict::Met);
    let path = found.path.expect("a path that validated is handed out");
    assert_eq!(
        path.certificates().len(),
        3,
        "signer, intermediate, trust point"
    );
    assert_eq!(
        path.authorities().len(),
        1,
        "one CA certificate below the trust point"
    );
    assert_eq!(
        path.end_entity().and_then(|issued| issued.issuer_key_bits),
        Some(2048),
        "the intermediate's RSA key made the signer's certificate"
    );
}

#[test]
fn a_path_longer_than_the_trust_points_constraint_is_broken_and_one_within_it_is_met() {
    let held = hierarchy();
    let mut limited = point(&held.root);
    // Clause 6.6.1: a value of zero admits only the trust point and an end-entity certificate, so
    // the one intermediate is one too many at zero and exactly enough at one.
    limited.path_length = Some(0);
    let found = searched(&trees(vec![limited.clone()]), &held, &at(WITHIN));
    assert_eq!(
        found.found,
        Found::Longer {
            intermediates: 1,
            limit: 0
        }
    );
    assert_eq!(found.verdict(), Verdict::Broken);
    limited.path_length = Some(1);
    let found = searched(&trees(vec![limited]), &held, &at(WITHIN));
    assert_eq!(found.verdict(), Verdict::Met);
}

#[test]
fn a_trust_point_that_issued_nothing_here_is_indeterminate_not_broken() {
    let held = hierarchy();
    // The signer's own certificate issued nothing: no chain ends at it.
    let found = searched(&trees(vec![point(&held.signer)]), &held, &at(WITHIN));
    // ETSI EN 319 102-1 clause 5.2.6.4 step 2 a): no chain is INDETERMINATE, not a failure.
    assert_eq!(found.found, Found::NoChain);
    assert_eq!(found.verdict(), Verdict::NotEvaluated);
    assert!(found.path.is_none());
}

#[test]
fn a_chain_rfc_5280_refuses_is_broken_unless_another_anchor_was_not_searched() {
    let held = hierarchy();
    let one = trees(vec![point(&held.root)]);
    let found = searched(&one, &held, &at(AFTER_THE_SIGNER_EXPIRED));
    assert_eq!(
        found.found,
        Found::Refused(PathRefusal::NotCurrent, Vec::new())
    );
    assert_eq!(
        found.verdict(),
        Verdict::Broken,
        "RFC 5280 section 6.1.3 (a)(2)"
    );
    // A trusted-list anchor might have validated another path, so the same refusal beside one is
    // not called broken.
    let with_list = TrustTrees {
        trusted_lists: 1,
        ..one
    };
    let found = searched(&with_list, &held, &at(AFTER_THE_SIGNER_EXPIRED));
    assert_eq!(found.unexamined, 1);
    assert_eq!(found.verdict(), Verdict::NotEvaluated);
}

#[test]
fn a_trust_point_past_its_reliable_instant_is_indeterminate() {
    let held = hierarchy();
    let mut sunset = point(&held.root);
    sunset.reliable_until = Some(Stated {
        instant: Instant::from_unix_seconds(1_767_225_600),
        spelled: "20260101000000Z".to_owned(),
    });
    let found = searched(&trees(vec![sunset.clone()]), &held, &at(WITHIN));
    // EN 319 102-1 clause 5.2.6.4 step 3): at or after the sunset date the chain is
    // indeterminate.
    assert_eq!(
        found.found,
        Found::PastReliable {
            until: "20260101000000Z".to_owned()
        }
    );
    assert_eq!(found.verdict(), Verdict::NotEvaluated);
    // The control: reliable until after the instant, the same trust point validates.
    sunset.reliable_until = Some(Stated {
        instant: Instant::from_unix_seconds(1_798_761_600),
        spelled: "20270101000000Z".to_owned(),
    });
    let found = searched(&trees(vec![sunset]), &held, &at(WITHIN));
    assert_eq!(found.verdict(), Verdict::Met);
}

#[test]
fn a_constraint_this_program_does_not_process_keeps_a_valid_path_from_being_met() {
    let held = hierarchy();
    let mut named = point(&held.root);
    named.unprocessed.push("name constraints");
    let found = searched(&trees(vec![named]), &held, &at(WITHIN));
    assert_eq!(found.verdict(), Verdict::NotEvaluated);
    assert!(
        found.path.is_some(),
        "the path still validated, and constraints along it read it"
    );
}

#[test]
fn with_no_instant_nothing_is_searched() {
    let held = hierarchy();
    let found = searched(&trees(vec![point(&held.root)]), &held, &facts(None));
    assert_eq!(found.found, Found::NoInstant);
    // A covering signature timestamp's instant is the fallback, as for the signing period.
    let found = searched(
        &trees(vec![point(&held.root)]),
        &held,
        &facts(Some("20260601000000Z")),
    );
    assert_eq!(found.verdict(), Verdict::Met);
}

#[test]
fn trust_points_past_the_bound_are_counted_unexamined_not_searched() {
    let held = hierarchy();
    let mut points: Vec<_> = (0..MAX_TRUST_POINTS).map(|_| point(&held.signer)).collect();
    // The one trust point that would have met the condition sits past the bound.
    points.push(point(&held.root));
    let found = searched(&trees(points), &held, &at(WITHIN));
    assert_eq!(found.found, Found::NoChain);
    assert_eq!(found.unexamined, 1);
    assert_eq!(
        verdict(&found.found, found.unexamined),
        Verdict::NotEvaluated
    );
}

/// RFC 5035 section 5.4.1's `SigningCertificateV2` with one SHA-256 `ESSCertIDv2` per certificate,
/// the `hashAlgorithm` omitted as its `DEFAULT`.
pub(crate) fn signing_certificate_v2(certificates: &[&[u8]]) -> Vec<u8> {
    let identifiers: Vec<Vec<u8>> = certificates
        .iter()
        .map(|certificate| seq(&[tlv(0x04, &Digest::Sha256.compute(&[certificate]))]))
        .collect();
    seq(&[seq(&identifiers)])
}

#[test]
fn a_full_path_reference_needs_a_hash_of_every_certificate_below_the_trust_point() {
    let held = hierarchy();
    let found = searched(&trees(vec![point(&held.root)]), &held, &at(WITHIN));
    let path = found.path.expect("a path");
    let whole = signing_certificate_v2(&[&held.signer, &held.intermediate]);
    let signer_only = signing_certificate_v2(&[&held.signer]);
    let mut signed = cms(&held.signer, &held.intermediate);
    signed.signing_certificate_v2 = Some(value(&whole));
    assert_eq!(
        path.referenced_by(&signed),
        Some(true),
        "TR 102 272 clause 6.5.1 fullPath"
    );
    signed.signing_certificate_v2 = Some(value(&signer_only));
    assert_eq!(
        path.referenced_by(&signed),
        Some(false),
        "the intermediate is not referenced"
    );
    signed.signing_certificate_v2 = None;
    assert_eq!(
        path.referenced_by(&signed),
        Some(false),
        "no attribute references nothing"
    );
}

/// A trust point mandating `certificates` as its certification path.
fn mandating<'a>(trust_point: &'a [u8], certificates: &[&'a [u8]]) -> TrustPoint<'a> {
    TrustPoint {
        mandated_path: Some(certificates.iter().map(|der| Cow::Borrowed(*der)).collect()),
        ..point(trust_point)
    }
}

/// The fixture signer's `SignedData`, carrying the signer's certificate and nothing above it.
fn carrying_the_signer_alone(held: &Hierarchy) -> SignedData<'_> {
    let mut signed = cms(&held.signer, &held.intermediate);
    signed.certificates.truncate(1);
    signed
}

/// A certificate of another fixture hierarchy, issued by nobody this one names.
fn unrelated() -> Vec<u8> {
    hex(crate::policy_rules::revocation_checks::tests::fixtures::INTERMEDIATE)
}

#[test]
fn a_mandated_path_supplies_the_certificates_the_signature_does_not_carry() {
    let held = hierarchy();
    let signed = carrying_the_signer_alone(&held);
    // The control: over the signature's own certificates no chain reaches the root.
    let found = search(&trees(vec![point(&held.root)]), &signed, &at(WITHIN));
    assert_eq!(found.found, Found::NoChain);
    // TS 119 172-2 clause 4.22.5.1 item 5: the component names the certification path to be used,
    // here listed explicitly; the intermediate it lists completes the path.
    let mandated = trees(vec![mandating(&held.root, &[&held.intermediate])]);
    let found = search(&mandated, &signed, &at(WITHIN));
    assert_eq!(
        found.found,
        Found::Reached {
            length: 2,
            unprocessed: Vec::new(),
            evaluated: Vec::new(),
            policies: None,
            mandated: Some(Vec::new()),
        }
    );
    assert_eq!(found.verdict(), Verdict::Met);
    let path = found.path.expect("a path that validated is handed out");
    assert_eq!(
        path.certificates()
            .iter()
            .map(|certificate| certificate.encoding.as_slice())
            .collect::<Vec<_>>(),
        [&held.signer[..], &held.intermediate[..], &held.root[..]],
        "signer, the policy's intermediate, trust point"
    );
    // A list naming the signer's and the trust point's own certificates as well is the same path.
    let whole = trees(vec![mandating(
        &held.root,
        &[&held.signer, &held.intermediate, &held.root],
    )]);
    assert_eq!(search(&whole, &signed, &at(WITHIN)).verdict(), Verdict::Met);
}

#[test]
fn a_mandated_path_replaces_the_certificates_the_signature_carries() {
    let held = hierarchy();
    let other = unrelated();
    // The signature carries the intermediate, and the policy mandates a path without it: the
    // signature's certificate does not stand in for the policy's (clause 4.22.5.1 item 5), so no
    // chain reaches the trust point, which EN 319 102-1 clause 5.2.6.4 calls indeterminate.
    let found = searched(
        &trees(vec![mandating(&held.root, &[&other])]),
        &held,
        &at(WITHIN),
    );
    assert_eq!(found.found, Found::NoChain);
    assert_eq!(found.verdict(), Verdict::NotEvaluated);
    // A path validates over the intermediate, but the other certificate the policy lists is not on
    // it, so it is not the path the clause says is to be used.
    let found = searched(
        &trees(vec![mandating(&held.root, &[&held.intermediate, &other])]),
        &held,
        &at(WITHIN),
    );
    assert_eq!(found.found, Found::MandatedPathNotFollowed { left_off: 1 });
    assert_eq!(found.verdict(), Verdict::NotEvaluated);
    // A mandated certificate this program cannot read leaves the path unfollowed: counted, never
    // broken.
    let found = searched(
        &trees(vec![mandating(&held.root, &[&[0x30, 0x00]])]),
        &held,
        &at(WITHIN),
    );
    assert_eq!(found.unexamined, 1);
    assert_eq!(found.verdict(), Verdict::NotEvaluated);
}

#[test]
fn a_mandated_path_has_preference_over_the_components_it_contradicts() {
    let held = hierarchy();
    let signed = carrying_the_signer_alone(&held);
    // pathLenConstraint zero admits no CA certificate below the trust point, and the mandated path
    // holds one. Clause 4.22.5.1 item 5: where the path's content contradicts an earlier component
    // of the trust point, the path's requirements take precedence.
    let mut short = mandating(&held.root, &[&held.intermediate]);
    short.path_length = Some(0);
    let found = search(&trees(vec![short]), &signed, &at(WITHIN));
    assert_eq!(
        found.found,
        Found::Reached {
            length: 2,
            unprocessed: Vec::new(),
            evaluated: Vec::new(),
            policies: None,
            mandated: Some(vec!["pathLenConstraint"]),
        }
    );
    assert_eq!(found.verdict(), Verdict::Met);
    // An acceptable policy set no certificate asserts, with an explicit policy required from the
    // start: RFC 5280 section 6.1.5 (g) refuses the path under these inputs (the control, without
    // a mandated path), and the mandated path has preference over them.
    let inputs = crate::trust::PathInputs {
        policy_set: Some(vec![vec![0x2A, 0x03, 0x04]]),
        require_explicit_policy: Some(0),
        ..crate::trust::PathInputs::default()
    };
    let constrained = TrustPoint {
        inputs: inputs.clone(),
        ..point(&held.root)
    };
    let found = searched(&trees(vec![constrained]), &held, &at(WITHIN));
    assert!(
        matches!(found.found, Found::Refused(..)),
        "{:?}",
        found.found
    );
    assert_eq!(found.verdict(), Verdict::Broken);
    let preferred = TrustPoint {
        inputs,
        ..mandating(&held.root, &[&held.intermediate])
    };
    let found = search(&trees(vec![preferred]), &signed, &at(WITHIN));
    assert!(
        matches!(
            &found.found,
            Found::Reached { evaluated, mandated: Some(yielded), .. }
                if evaluated.is_empty()
                    && *yielded == [super::ACCEPTABLE_POLICY_SET, "policy constraints"]
        ),
        "{:?}",
        found.found
    );
    assert_eq!(found.verdict(), Verdict::Met);
}

#[test]
fn a_stated_acceptable_policy_set_requires_the_path_to_be_valid_for_one_of_its_policies() {
    // A796 (ADR 2798): RFC 3125 section 6.6.1 and TS 119 172-2 clause 4.22.5.1 make the set the
    // policies acceptable under the signature policy, read as requiring one of them, which is RFC
    // 5280 section 6.1.1 (f)'s `initial-explicit-policy`. The fixture certificates assert no
    // policy, so section 6.1.5 (g) finds the `valid_policy_tree` NULL with `explicit_policy` zero.
    let held = hierarchy();
    let stated = |policy_set| TrustPoint {
        inputs: crate::trust::PathInputs {
            policy_set,
            ..crate::trust::PathInputs::default()
        },
        ..point(&held.root)
    };
    let found = searched(
        &trees(vec![stated(Some(vec![vec![0x2A, 0x03, 0x04]]))]),
        &held,
        &at(WITHIN),
    );
    assert!(
        matches!(&found.found, Found::Refused(_, named) if *named == [super::ACCEPTABLE_POLICY_SET]),
        "{:?}",
        found.found
    );
    assert_eq!(found.verdict(), Verdict::Broken);
    // The control: no set stated, so section 6.1.1 (c)'s `any-policy` and no explicit policy.
    let found = searched(&trees(vec![stated(None)]), &held, &at(WITHIN));
    assert_eq!(found.verdict(), Verdict::Met);
}
