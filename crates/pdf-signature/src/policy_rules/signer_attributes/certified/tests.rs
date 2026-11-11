//! RFC 5755 section 5's conditions, each answered for a fixture attribute certificate whose
//! authority's hierarchy and holder are known.

use std::borrow::Cow;

use super::*;
use crate::attribute_certificate::tests::fixtures::{
    AA, AA_CA, AA_ROOT, AC, AC_BY_A_CA, AC_BY_SUBJECT, AC_OTHER_HOLDER, AC_UNSUPPORTED_CRITICAL,
};
use crate::policy_rules::certification_path::tests::{WITHIN, cms, hex, hierarchy, value};
use crate::policy_rules::certification_path::{TrustPoint, search};
use crate::policy_rules::tests::facts;

/// 2026-01-01, the first instant every fixture attribute certificate is valid at.
const FIRST_DAY: Instant = Instant::from_unix_seconds(1_767_225_600);
/// 2027-01-01 plus one second: past every fixture attribute certificate's validity, within its
/// authority's.
const PAST_VALIDITY: Instant = Instant::from_unix_seconds(1_798_761_601);

pub(in crate::policy_rules) fn trees(certificates: &[&[u8]]) -> TrustTrees<'static> {
    TrustTrees {
        points: certificates
            .iter()
            .map(|certificate| TrustPoint {
                certificate: Cow::Owned(certificate.to_vec()),
                path_length: None,
                reliable_until: None,
                inputs: trust::PathInputs::default(),
                unprocessed: Vec::new(),
                mandated_path: None,
            })
            .collect(),
        trusted_lists: 0,
    }
}

/// The certificates every judgement here reads: the signer's hierarchy, both authorities and
/// their root.
struct Held {
    signer: Vec<u8>,
    intermediate: Vec<u8>,
    root: Vec<u8>,
    authority: Vec<u8>,
    authority_ca: Vec<u8>,
    authority_root: Vec<u8>,
}

fn held() -> Held {
    let hierarchy = hierarchy();
    Held {
        signer: hierarchy.signer,
        intermediate: hierarchy.intermediate,
        root: hierarchy.root,
        authority: hex(AA),
        authority_ca: hex(AA_CA),
        authority_root: hex(AA_ROOT),
    }
}

/// `certificate` judged in a signature carrying `carried` besides the signer's two
/// certificates, the signer's path searched to its own root, the issuer's to `trees`, at `at`.
fn judged(
    certificate: &str,
    held: &Held,
    carried: &[&[u8]],
    trees: Option<&TrustTrees<'_>>,
    at: Instant,
    revocation: bool,
) -> Outcome {
    let mut signature = cms(&held.signer, &held.intermediate);
    signature
        .certificates
        .extend(carried.iter().map(|bytes| value(bytes)));
    let signer_trees = self::trees(&[&held.root]);
    let signer_path = search(&signer_trees, &signature, &facts(None).validated_at(WITHIN));
    assert_eq!(signer_path.verdict(), Verdict::Met, "the signer's own path");
    let bytes = hex(certificate);
    let conditions = Conditions {
        trees,
        signer_path: Some(&signer_path),
        at: Some(at),
        revocation,
    };
    judge(value(&bytes), &signature, conditions).outcome
}

#[test]
fn a_certificate_meeting_every_condition_is_valid() {
    // RFC 5755 section 5: the holder is the signer (item 1), the signature verifies under the
    // authority's key and the authority's path reaches the condition's trust tree (items 2 and
    // 4), the authority is not a CA and may sign (item 3), the instant is within the validity
    // period, its first instant included (item 5), and nothing targets or is unsupported.
    let held = held();
    let tree = trees(&[&held.authority_root]);
    for (certificate, at) in [(AC, WITHIN), (AC, FIRST_DAY), (AC_BY_SUBJECT, WITHIN)] {
        assert_eq!(
            judged(
                certificate,
                &held,
                &[&held.authority],
                Some(&tree),
                at,
                false
            ),
            Outcome::Valid
        );
    }
}

#[test]
fn each_failed_condition_makes_the_certificate_invalid() {
    let held = held();
    let tree = trees(&[&held.authority_root]);
    let carried: &[&[u8]] = &[&held.authority, &held.authority_ca];
    let outcome = |certificate, at| judged(certificate, &held, carried, Some(&tree), at, false);
    // Item 1 and section 4.2.2: a `baseCertificateID` of another serial number is not the signer.
    assert_eq!(
        outcome(AC_OTHER_HOLDER, WITHIN),
        Outcome::Invalid(Invalid::NotTheSigners)
    );
    // Item 2: one octet of the role's value changed, so the authority's signature does not verify.
    let mut tampered = hex(AC);
    let role = tampered
        .windows(6)
        .position(|window| window == b"urn:qu")
        .expect("the role's value");
    tampered[role] ^= 0x01;
    let tampered = tampered.iter().fold(String::new(), |mut text, octet| {
        use std::fmt::Write;
        write!(text, "{octet:02x}").expect("a String takes every write");
        text
    });
    assert_eq!(
        outcome(&tampered, WITHIN),
        Outcome::Invalid(Invalid::Signature)
    );
    // Item 3 and section 4.5: an attribute authority may not be a CA.
    assert_eq!(
        outcome(AC_BY_A_CA, WITHIN),
        Outcome::Invalid(Invalid::IssuerIsCa)
    );
    // Item 5: one second past `notAfterTime`.
    assert_eq!(
        outcome(AC, PAST_VALIDITY),
        Outcome::Invalid(Invalid::Outside)
    );
    // Item 7: a critical extension this program does not support.
    assert_eq!(
        outcome(AC_UNSUPPORTED_CRITICAL, WITHIN),
        Outcome::Invalid(Invalid::UnsupportedCritical(
            "1.3.6.1.4.1.39857.1.1".to_owned()
        ))
    );
}

#[test]
fn what_this_program_cannot_decide_is_named_not_guessed() {
    let held = held();
    let tree = trees(&[&held.authority_root]);
    // The authority's certificate is not carried, so neither its signature nor its path is known.
    assert_eq!(
        judged(AC, &held, &[], Some(&tree), WITHIN, false),
        Outcome::Undecided(Undecided::NoIssuerCertificate)
    );
    // A set of constraints that states no trust trees names nothing to reach.
    assert_eq!(
        judged(AC, &held, &[&held.authority], None, WITHIN, false),
        Outcome::Undecided(Undecided::NoTrustTrees)
    );
    // The signer's root is not the authority's: no chain, which EN 319 102-1 calls indeterminate.
    let elsewhere = trees(&[&held.root]);
    assert!(matches!(
        judged(
            AC,
            &held,
            &[&held.authority],
            Some(&elsewhere),
            WITHIN,
            false
        ),
        Outcome::Undecided(Undecided::Path(Found::NoChain, 0))
    ));
    // Every condition holds, and the policy requires a revocation check this program does not make.
    assert_eq!(
        judged(AC, &held, &[&held.authority], Some(&tree), WITHIN, true),
        Outcome::Undecided(Undecided::Revocation)
    );
    // A failed condition is decisive over an undecided one: the holder is somebody else's even
    // where the authority's certificate is missing.
    assert_eq!(
        judged(AC_OTHER_HOLDER, &held, &[], Some(&tree), WITHIN, true),
        Outcome::Invalid(Invalid::NotTheSigners)
    );
}
