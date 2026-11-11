//! Synthetic ETSI TR 102 272 policy documents, each built to the clause 6 member it tests.

use super::*;
use crate::policy::Specification;

/// `id-sha256`, 2.16.840.1.101.3.4.2.1.
pub(super) const SHA256: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01];
/// `sha256WithRSAEncryption`, 1.2.840.113549.1.1.11.
pub(super) const SHA256_RSA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0B];
/// A policy identifier of the tests' own, 1.2.3.4.
pub(super) const POLICY: &[u8] = &[0x2A, 0x03, 0x04];
/// A commitment type of the tests' own, 1.2.3.5.
pub(super) const PROOF_OF_ORIGIN: &[u8] = &[0x2A, 0x03, 0x05];

pub(super) fn tlv(tag: u8, contents: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    let length = contents.len();
    if length < 0x80 {
        out.push(u8::try_from(length).expect("below 0x80"));
    } else {
        let octets: Vec<u8> = length
            .to_be_bytes()
            .into_iter()
            .skip_while(|octet| *octet == 0)
            .collect();
        out.push(0x80 | u8::try_from(octets.len()).expect("a usize has at most eight octets"));
        out.extend(octets);
    }
    out.extend_from_slice(contents);
    out
}

pub(super) fn seq(parts: &[Vec<u8>]) -> Vec<u8> {
    tlv(SEQUENCE, &parts.concat())
}

pub(super) fn oid(octets: &[u8]) -> Vec<u8> {
    tlv(OBJECT_IDENTIFIER, octets)
}

pub(super) fn tagged(number: u8, inner: &[u8]) -> Vec<u8> {
    tlv(0xA0 | number, inner)
}

pub(super) fn time(spelled: &str) -> Vec<u8> {
    tlv(GENERALIZED_TIME, spelled.as_bytes())
}

/// What a fixture varies; every other member is the smallest clause 6 admits.
struct Fixture {
    not_after: Option<&'static str>,
    signed: Vec<&'static [u8]>,
    unsigned: Vec<&'static [u8]>,
    algorithms: Option<Vec<Vec<u8>>>,
    commitment_rules: Vec<Vec<u8>>,
    common_signer_rules: bool,
    own_hash: bool,
    /// Clause 6.6.1's `CertificateTrustPoint`s of the signing-certificate trust condition.
    trust_points: Vec<Vec<u8>>,
    /// Clause 6.10's lists after `signerAlgorithmConstraints`, each already context-tagged.
    issuer_algorithms: Vec<Vec<u8>>,
    /// Clause 6.8's `TimestampTrustCondition` members, each already context-tagged.
    timestamp_condition: Vec<Vec<u8>>,
    /// Clause 6.9's `AttributeTrustCondition`, its members already encoded.
    attribute_condition: Option<Vec<Vec<u8>>>,
}

impl Default for Fixture {
    fn default() -> Self {
        Self {
            not_after: None,
            signed: vec![cms::ID_CONTENT_TYPE, cms::ID_MESSAGE_DIGEST],
            unsigned: Vec::new(),
            algorithms: None,
            commitment_rules: Vec::new(),
            common_signer_rules: true,
            own_hash: true,
            trust_points: Vec::new(),
            issuer_algorithms: Vec::new(),
            timestamp_condition: Vec::new(),
            attribute_condition: None,
        }
    }
}

fn identifiers(list: &[&[u8]]) -> Vec<u8> {
    seq(&list.iter().map(|octets| oid(octets)).collect::<Vec<_>>())
}

fn signer_and_verifier_rules(signed: &[&[u8]], unsigned: &[&[u8]]) -> Vec<u8> {
    tagged(
        0,
        &seq(&[
            seq(&[identifiers(signed), identifiers(unsigned)]),
            seq(&[identifiers(&[])]),
        ]),
    )
}

impl Fixture {
    fn info(&self) -> Vec<u8> {
        let mut period = vec![time("20260101000000Z")];
        if let Some(end) = self.not_after {
            period.push(time(end));
        }
        let mut common = Vec::new();
        if self.common_signer_rules {
            common.push(signer_and_verifier_rules(&self.signed, &self.unsigned));
        }
        // Clause 6.3 requires both trust conditions in the common rules or in every commitment
        // rule. The signing-certificate condition is clause 6.7's two members, its revocation
        // requirement clause 6.6.2's `noCheck` for both kinds of certificate; the time-stamp
        // condition's members are all optional, and a fixture states none unless it asks.
        let no_check = seq(&[tlv(ENUMERATED, &[4])]);
        common.push(tagged(
            1,
            &seq(&[
                seq(&self.trust_points),
                seq(&[no_check.clone(), tagged(0, &no_check)]),
            ]),
        ));
        common.push(tagged(2, &seq(&self.timestamp_condition)));
        if let Some(condition) = &self.attribute_condition {
            common.push(tagged(3, &seq(condition)));
        }
        if self.algorithms.is_some() || !self.issuer_algorithms.is_empty() {
            let mut lists = Vec::new();
            if let Some(algorithms) = &self.algorithms {
                lists.push(tagged(0, &seq(algorithms)));
            }
            lists.extend(self.issuer_algorithms.iter().cloned());
            common.push(tagged(4, &seq(&lists)));
        }
        let validation = seq(&[seq(&period), seq(&common), seq(&self.commitment_rules)]);
        seq(&[
            oid(POLICY),
            time("20251201000000Z"),
            seq(&[]),
            tlv(0x0C, b"tests"),
            validation,
        ])
    }

    fn octets(&self) -> Vec<u8> {
        let info = self.info();
        let mut members = vec![seq(&[oid(SHA256)]), info.clone()];
        if self.own_hash {
            // Clause 6.1: over the information without its outer type and length.
            let hash = Digest::Sha256.compute(&[contents(&info)]);
            members.push(tlv(OCTET_STRING, &hash));
        }
        seq(&members)
    }
}

fn contents(info: &[u8]) -> &[u8] {
    der::Reader::new(info)
        .expect("small")
        .next_value()
        .expect("well formed")
        .expect("one value")
        .contents
}

pub(super) fn signed_data<'a>(
    signed: Vec<&'a [u8]>,
    unsigned: Vec<&'a [u8]>,
    attributes: Vec<cms::Attribute<'a>>,
) -> SignedData<'a> {
    SignedData {
        content_type: cms::ID_DATA,
        encapsulated: None,
        certificates: Vec::new(),
        signers: 1,
        digest: Some(Digest::Sha256),
        digest_algorithm: SHA256,
        signature_algorithm: SHA256_RSA,
        signature_algorithm_parameters: None,
        signature: &[],
        signed_attributes: None,
        signer_issuer_and_serial: None,
        signer_key_identifier: None,
        message_digest: None,
        signing_certificate: None,
        signing_certificate_v2: None,
        signature_timestamp: None,
        signed_attribute_types: signed,
        unsigned_attribute_types: unsigned,
        attributes,
        attributes_truncated: false,
    }
}

pub(super) fn facts(at: Option<&str>) -> SignerFacts<'static> {
    SignerFacts {
        signing_time: at.map(|spelled| SigningTime {
            instant: x509::read_time_digits(spelled.as_bytes(), true).expect("a time"),
            spelled: spelled.to_owned(),
            source: TimeSource::SignatureTimestamp,
        }),
        key_bits: Some(2048),
        carries_signer_certificate: true,
        certificate_signature_algorithm: Some(SHA256_RSA.to_vec()),
        path_at: None,
        material: Material::none(),
    }
}

pub(super) fn verdict_of(
    findings: &[Finding],
    wanted: impl Fn(&Constraint) -> bool,
) -> Vec<Verdict> {
    findings
        .iter()
        .filter(|finding| wanted(&finding.constraint))
        .map(|finding| finding.verdict)
        .collect()
}

fn period(findings: &[Finding]) -> Vec<Verdict> {
    verdict_of(findings, |c| matches!(c, Constraint::SigningPeriod { .. }))
}

fn default_cms() -> SignedData<'static> {
    signed_data(
        vec![cms::ID_CONTENT_TYPE, cms::ID_MESSAGE_DIGEST],
        vec![],
        vec![],
    )
}

#[test]
fn a_policy_whose_signing_period_ended_before_the_signing_time_is_broken() {
    let octets = Fixture {
        not_after: Some("20260601000000Z"),
        ..Fixture::default()
    }
    .octets();
    let document = PolicyDocument::parse(&octets).expect("a clause 6 document");
    let cms = default_cms();
    let late = findings(&document, &cms, &facts(Some("20260701000000Z")));
    assert_eq!(period(&late), [Verdict::Broken]);
    let inside = findings(&document, &cms, &facts(Some("20260301000000Z")));
    assert_eq!(period(&inside), [Verdict::Met]);
    let early = findings(&document, &cms, &facts(Some("20251231000000Z")));
    assert_eq!(period(&early), [Verdict::Broken]);
    let unknown = findings(&document, &cms, &facts(None));
    assert_eq!(period(&unknown), [Verdict::NotEvaluated]);
    let sentence = late[0].to_string();
    assert!(
        sentence.contains("20260601000000Z") && sentence.contains("broken"),
        "{sentence}"
    );
}

#[test]
fn a_mandated_attribute_the_signer_left_out_is_broken_and_one_provided_is_met() {
    let octets = Fixture {
        unsigned: vec![cms::ID_AA_TIME_STAMP_TOKEN],
        ..Fixture::default()
    }
    .octets();
    let document = PolicyDocument::parse(&octets).expect("a clause 6 document");
    let without = signed_data(vec![cms::ID_CONTENT_TYPE], vec![], vec![]);
    let found = findings(&document, &without, &facts(None));
    assert_eq!(
        verdict_of(&found, |c| matches!(c, Constraint::SignedAttribute(_))),
        [Verdict::Met, Verdict::Broken]
    );
    assert_eq!(
        verdict_of(&found, |c| matches!(c, Constraint::UnsignedAttribute(_))),
        [Verdict::Broken]
    );
    let with = signed_data(
        vec![cms::ID_CONTENT_TYPE, cms::ID_MESSAGE_DIGEST],
        vec![cms::ID_AA_TIME_STAMP_TOKEN],
        vec![],
    );
    let found = findings(&document, &with, &facts(None));
    let attributes = verdict_of(&found, |c| {
        matches!(
            c,
            Constraint::SignedAttribute(_) | Constraint::UnsignedAttribute(_)
        )
    });
    assert_eq!(attributes, [Verdict::Met; 3]);
}

/// TR 102 272 clauses 6.7 and 6.10 over the fixture hierarchy: the trust condition's path to the
/// policy's own root, and the issuer lists judged along that path.
#[test]
fn the_signing_certificate_trust_condition_and_issuer_lists_are_judged_on_the_path() {
    use crate::policy_rules::certification_path::tests::{WITHIN, cms, hierarchy};
    let held = hierarchy();
    let signed = cms(&held.signer, &held.intermediate);
    let judged = |points: Vec<Vec<u8>>, issuer_algorithms: Vec<Vec<u8>>| {
        let octets = Fixture {
            trust_points: points,
            issuer_algorithms,
            ..Fixture::default()
        }
        .octets();
        let document = PolicyDocument::parse(&octets).expect("a clause 6 document");
        findings(&document, &signed, &facts(None).validated_at(WITHIN))
    };
    // The fixture's time-stamp condition states no trees, so clause 6.8 searches the authority's
    // path to these same points; the signature carries no time-stamp token, and that condition is
    // the time-stamp test's subject rather than this one's.
    let signing = |condition: &str| condition == SIGNING_CERT_TRUST;
    let trust =
        |c: &Constraint| matches!(c, Constraint::TrustPath { condition, .. } if signing(condition));
    let issuers = |c: &Constraint| matches!(c, Constraint::IssuerAlgorithm { .. });
    let revocation = |c: &Constraint| matches!(c, Constraint::RevocationCheck { condition, .. } if signing(condition));
    // The fixture hierarchy signs with SHA-256 and RSA under 2048-bit keys (RFC 4055's
    // sha256WithRSAEncryption); 0x0800 is 2048 bits and 0x0C00 is 3072.
    let rsa = |bits: &[u8]| seq(&[oid(SHA256_RSA), tlv(der::INTEGER, bits)]);
    let lists = |bits: &[u8]| vec![tagged(1, &seq(&[rsa(bits)])), tagged(2, &seq(&[rsa(bits)]))];
    let root = seq(std::slice::from_ref(&held.root));
    let met = judged(vec![root.clone()], lists(&[0x08, 0x00]));
    assert_eq!(
        verdict_of(&met, trust),
        [Verdict::Met],
        "clause 6.7, over clause 6.6.1's trust point"
    );
    assert_eq!(
        verdict_of(&met, revocation),
        [Verdict::Met, Verdict::Met],
        "clause 6.6.2's noCheck, for the end certificate and for the CA certificates, mandates nothing"
    );
    assert_eq!(
        verdict_of(&met, issuers),
        [Verdict::Met, Verdict::Met],
        "clause 6.10: the signer's certificate and the intermediate, each under a listed algorithm by a 2048-bit issuer key"
    );
    let short = judged(vec![root.clone()], lists(&[0x0C, 0x00]));
    assert_eq!(
        verdict_of(&short, issuers),
        [Verdict::Broken, Verdict::Broken],
        "a 3072-bit minimum breaks both issuers' 2048-bit keys"
    );
    let unlisted = judged(
        vec![root.clone()],
        vec![
            tagged(1, &seq(&[seq(&[oid(SHA256)])])),
            tagged(2, &seq(&[seq(&[oid(SHA256)])])),
        ],
    );
    assert_eq!(
        verdict_of(&unlisted, issuers),
        [Verdict::Broken, Verdict::Broken]
    );
    // Clause 6.6.1's pathLenConstraint of zero admits no intermediate.
    let limited = judged(
        vec![seq(&[
            held.root.clone(),
            tagged(0, &tlv(der::INTEGER, &[0])),
        ])],
        vec![],
    );
    assert_eq!(verdict_of(&limited, trust), [Verdict::Broken]);
    // The signer's own certificate as the trust point: it issued nothing, so no chain ends there.
    let none = judged(
        vec![seq(std::slice::from_ref(&held.signer))],
        lists(&[0x08, 0x00]),
    );
    assert_eq!(
        verdict_of(&none, trust),
        [Verdict::NotEvaluated],
        "EN 319 102-1 clause 5.2.6.4 step 2 a)"
    );
    assert_eq!(
        verdict_of(&none, issuers),
        [Verdict::NotEvaluated, Verdict::NotEvaluated],
        "the issuers' keys and the CA certificates are known only from a path"
    );
}

#[test]
fn an_algorithm_the_policy_does_not_admit_or_a_key_too_short_is_broken() {
    let cms = default_cms();
    let algorithm = |c: &Constraint| matches!(c, Constraint::SignerAlgorithm { .. });
    let judged = |entries: Vec<Vec<u8>>| {
        let octets = Fixture {
            algorithms: Some(entries),
            ..Fixture::default()
        }
        .octets();
        let document = PolicyDocument::parse(&octets).expect("a clause 6 document");
        verdict_of(&findings(&document, &cms, &facts(None)), algorithm)
    };
    // 0x0C00 is 3072 bits, past the fixtures' 2048-bit key; 0x0800 is 2048.
    let short = judged(vec![
        seq(&[oid(SHA256)]),
        seq(&[oid(SHA256_RSA), tlv(der::INTEGER, &[0x0C, 0x00])]),
    ]);
    assert_eq!(short, [Verdict::Met, Verdict::Broken]);
    let unlisted = judged(vec![seq(&[oid(SHA256)])]);
    assert_eq!(unlisted, [Verdict::Met, Verdict::Broken]);
    let enough = judged(vec![
        seq(&[oid(SHA256)]),
        seq(&[oid(SHA256_RSA), tlv(der::INTEGER, &[0x08, 0x00])]),
    ]);
    assert_eq!(enough, [Verdict::Met, Verdict::Met]);
}

#[test]
fn a_commitment_type_no_rule_selects_is_broken() {
    let rule = seq(&[
        seq(&[seq(&[oid(PROOF_OF_ORIGIN)])]),
        signer_and_verifier_rules(&[], &[]),
    ]);
    let octets = Fixture {
        commitment_rules: vec![rule],
        common_signer_rules: false,
        ..Fixture::default()
    }
    .octets();
    let document = PolicyDocument::parse(&octets).expect("a clause 6 document");
    let commitment = |c: &Constraint| matches!(c, Constraint::CommitmentType { .. });
    let none = signed_data(vec![], vec![], vec![]);
    assert_eq!(
        verdict_of(&findings(&document, &none, &facts(None)), commitment),
        [Verdict::Broken]
    );
    let indication = seq(&[oid(PROOF_OF_ORIGIN)]);
    let value = der::Reader::new(&indication)
        .expect("small")
        .next_value()
        .expect("well formed");
    let stated = signed_data(
        vec![cms::ID_AA_ETS_COMMITMENT_TYPE],
        vec![],
        vec![cms::Attribute {
            kind: cms::ID_AA_ETS_COMMITMENT_TYPE,
            signed: true,
            values: 1,
            first: value,
        }],
    );
    assert_eq!(
        verdict_of(&findings(&document, &stated, &facts(None)), commitment),
        [Verdict::Met]
    );
}

#[test]
fn clause_6_3s_placement_rules_refuse_a_policy_that_breaks_them() {
    let in_both = Fixture {
        commitment_rules: vec![seq(&[
            seq(&[tlv(der::NULL, &[])]),
            signer_and_verifier_rules(&[], &[]),
        ])],
        ..Fixture::default()
    }
    .octets();
    assert_eq!(
        PolicyDocument::parse(&in_both),
        Err(PolicyDocumentError::RuleInBothPlaces(
            "signer and verifier rules"
        ))
    );
    // With no commitment rule at all, "every commitment rule" holds vacuously, so the refusal
    // needs one rule that leaves the signer and verifier rules out.
    let in_neither = Fixture {
        commitment_rules: vec![seq(&[seq(&[tlv(der::NULL, &[])])])],
        common_signer_rules: false,
        ..Fixture::default()
    }
    .octets();
    assert_eq!(
        PolicyDocument::parse(&in_neither),
        Err(PolicyDocumentError::RuleInNeitherPlace(
            "signer and verifier rules"
        ))
    );
}

#[test]
fn a_policy_whose_own_hash_disagrees_with_its_information_is_refused() {
    let mut octets = Fixture::default().octets();
    *octets.last_mut().expect("a document") ^= 0x01;
    assert_eq!(
        PolicyDocument::parse(&octets),
        Err(PolicyDocumentError::OwnHashDisagrees)
    );
}

#[test]
fn ts_119_172_3s_syntax_is_told_apart_rather_than_misread() {
    // TS 119 172-3's `SignaturePolicy` opens with a `Digest`, whose first member is itself an
    // `AlgorithmIdentifier` sequence.
    let octets = seq(&[
        seq(&[seq(&[oid(SHA256)]), tlv(OCTET_STRING, &[0; 32])]),
        seq(&[]),
    ]);
    assert_eq!(
        PolicyDocument::parse(&octets),
        Err(PolicyDocumentError::OtherSyntax(Syntax::Tr102272))
    );
}

#[test]
fn a_nesting_past_the_depth_budget_is_refused_by_the_pre_pass() {
    let mut nested = tlv(der::NULL, &[]);
    for _ in 0..=der::MAX_DEPTH {
        nested = seq(&[nested]);
    }
    assert_eq!(
        PolicyDocument::parse(&nested),
        Err(PolicyDocumentError::Encoding(DerError::TooDeep))
    );
}

#[test]
fn the_entry_budget_holds_at_its_bound_and_refuses_past_it() {
    let many: Vec<&'static [u8]> = vec![cms::ID_CONTENT_TYPE; MAX_ENTRIES];
    let at = Fixture {
        signed: many.clone(),
        ..Fixture::default()
    }
    .octets();
    assert!(PolicyDocument::parse(&at).is_ok());
    let mut past = many;
    past.push(cms::ID_CONTENT_TYPE);
    let past = Fixture {
        signed: past,
        ..Fixture::default()
    }
    .octets();
    assert_eq!(
        PolicyDocument::parse(&past),
        Err(PolicyDocumentError::TooManyEntries)
    );
}

#[test]
fn a_document_past_the_byte_budget_is_refused_before_it_is_walked() {
    let mut octets = vec![0u8; MAX_POLICY_OCTETS];
    octets.push(0);
    assert_eq!(
        PolicyDocument::parse(&octets),
        Err(PolicyDocumentError::TooLarge {
            octets: octets.len()
        })
    );
}

pub(super) fn policy<'a>(
    octets: &'a [u8],
    value: &'a [u8],
    identifier: &str,
) -> SignaturePolicy<'a> {
    SignaturePolicy {
        identifier: identifier.to_owned(),
        hash: PolicyHash::Stated {
            digest: Digest::Sha256,
            value,
        },
        qualifiers: Vec::new(),
        store: Some(Store::Document {
            specification: Specification::Uri("urn:test".to_owned()),
            octets,
        }),
    }
}

#[test]
fn only_a_bound_document_naming_the_signers_policy_is_enforced() {
    let octets = Fixture::default().octets();
    let cms = default_cms();
    let whole = Digest::Sha256.compute(&[&octets]);
    assert!(matches!(
        enforce(&policy(&octets, &whole, "1.2.3.4"), &cms, &facts(None)),
        Enforcement::Enforced { .. }
    ));
    // Annex B.11's second way: the signer signed the policy's own hash.
    let document = PolicyDocument::parse(&octets).expect("a clause 6 document");
    let own = document
        .own_hash
        .expect("the fixture states one")
        .1
        .to_vec();
    assert!(matches!(
        enforce(&policy(&octets, &own, "1.2.3.4"), &cms, &facts(None)),
        Enforcement::Enforced { .. }
    ));
    assert!(matches!(
        enforce(&policy(&octets, &[0x55; 32], "1.2.3.4"), &cms, &facts(None)),
        Enforcement::NotEnforced(NotEnforced::NotBound(_))
    ));
    assert_eq!(
        enforce(&policy(&octets, &whole, "1.2.3.9"), &cms, &facts(None)),
        Enforcement::NotEnforced(NotEnforced::IdentifierDiffers {
            document: "1.2.3.4".to_owned()
        })
    );
}

/// The memory probe's population, ignored by the ordinary run: a document as long as the byte
/// budget admits that opens a thousand nested values, and one a single entry past the entry
/// budget, each parsed a thousand times.
#[test]
#[ignore = "the memory probe's population, run under getrusage"]
fn hostile_policy_documents_are_refused_within_their_budgets() {
    let mut deep = vec![0x30_u8; MAX_POLICY_OCTETS];
    for octet in deep.iter_mut().skip(1).step_by(2) {
        *octet = 0x80;
    }
    let mut many: Vec<&'static [u8]> = vec![cms::ID_CONTENT_TYPE; MAX_ENTRIES];
    many.push(cms::ID_CONTENT_TYPE);
    let past = Fixture {
        signed: many,
        ..Fixture::default()
    }
    .octets();
    for _ in 0..1000 {
        assert!(PolicyDocument::parse(&deep).is_err());
        assert_eq!(
            PolicyDocument::parse(&past),
            Err(PolicyDocumentError::TooManyEntries)
        );
    }
}

/// Clause 6.8's `DeltaTime`, of seconds alone.
fn delta_seconds(seconds: u8) -> Vec<u8> {
    let int = |value: u8| tlv(der::INTEGER, &[value]);
    seq(&[int(seconds), int(0), int(0), int(0)])
}

/// TR 102 272 clause 6.8's `signatureTimestampDelay`: "If the signature timestamp is later that
/// the time in the signing-time attribute by more than the value given in
/// signatureTimestampDelay, the signature shall be considered invalid" (the clause, cited and not
/// quoted, ADR 1085). Claimed at 00:00:00 and stamped at 00:01:40, a delay of 60 s is broken and
/// one of 120 s met; with no signing-time attribute there is nothing to measure from, which ETSI
/// EN 319 102-1 clause 5.5.4 step 5 leaves indeterminate. The plant that keeps naming the member
/// fails the first two.
#[test]
fn the_signature_time_stamp_delay_is_held_against_the_claimed_signing_time() {
    let claimed = tlv(GENERALIZED_TIME, b"20260301000000Z");
    let claimed = der::Reader::new(&claimed)
        .expect("small")
        .next_value()
        .expect("well formed");
    let cms = signed_data(
        vec![
            cms::ID_CONTENT_TYPE,
            cms::ID_MESSAGE_DIGEST,
            cms::ID_SIGNING_TIME,
        ],
        vec![],
        vec![cms::Attribute {
            kind: cms::ID_SIGNING_TIME,
            signed: true,
            values: 1,
            first: claimed,
        }],
    );
    let stamped = facts(Some("20260301000140Z"));
    let delay = |seconds: u8, cms: &SignedData<'_>| {
        let octets = Fixture {
            timestamp_condition: vec![tagged(4, &delta_seconds(seconds))],
            ..Fixture::default()
        }
        .octets();
        let document = PolicyDocument::parse(&octets).expect("a clause 6 document");
        verdict_of(&findings(&document, cms, &stamped), |c| {
            matches!(
                c,
                Constraint::PolicyComponent(ts_119_172_3::Constraint::TimestampDelay { .. })
            ) || matches!(c, Constraint::ConditionMember { .. })
        })
    };
    assert_eq!(delay(60, &cms), [Verdict::Broken]);
    assert_eq!(delay(120, &cms), [Verdict::Met]);
    assert_eq!(delay(120, &default_cms()), [Verdict::NotEvaluated]);
}

/// TR 102 272 clause 6.8's `cautionPeriod`: revocation status information "shall not be collected
/// and used" before the period after the signing time ends (the clause, cited), and Annex B.5.3
/// has the verifier wait for that end before checking revocation. Stamped at 20:00 on the day the
/// minted lists were issued at 21:40:29 and validated at 2027-01-01: an hour's caution admits
/// them and the authority's checks are met; two hours' leaves both checks unanswered, never
/// broken; and a year's has not ended at the instant validated at, so the period itself is not
/// evaluated. The plant that ignores the member answers all three alike.
#[test]
fn the_caution_period_decides_which_material_answers_and_when_a_verifier_may_ask() {
    use crate::policy_rules::certification_path::tests::hex;
    use crate::policy_rules::revocation_checks::tests::fixtures as minted;

    let token = hex(minted::TOKEN);
    let bytes = cms::fixtures::detached_with_signature_timestamp(&[0x00; 32], &token);
    let signed = cms::signed_data(&bytes).expect("a SignedData");
    let (root_crl, intermediate_crl) = (hex(minted::ROOT_CRL), hex(minted::INTERMEDIATE_CRL));
    let material = Material::read(&[&root_crl, &intermediate_crl], &[]);
    let at = Instant::from_unix_seconds(1_798_761_600);
    let crl_check = || seq(&[tlv(ENUMERATED, &[0])]);
    let judged = |caution: Vec<u8>| {
        let octets = Fixture {
            timestamp_condition: vec![
                tagged(0, &seq(&[seq(&[hex(minted::ROOT)])])),
                tagged(1, &seq(&[crl_check(), tagged(0, &crl_check())])),
                tagged(3, &caution),
            ],
            ..Fixture::default()
        }
        .octets();
        let document = PolicyDocument::parse(&octets).expect("a clause 6 document");
        findings(
            &document,
            &signed,
            &facts(Some("20261105200000Z"))
                .validated_at(at)
                .with_material(&material),
        )
    };
    let checks = |c: &Constraint| matches!(c, Constraint::RevocationCheck { condition, .. } if *condition == TIMESTAMP_TRUST);
    let period = |c: &Constraint| matches!(c, Constraint::CautionPeriod { .. });
    let int = |value: &[u8]| tlv(der::INTEGER, value);
    let hour = seq(&[int(&[0]), int(&[0]), int(&[1]), int(&[0])]);
    let two_hours = seq(&[int(&[0]), int(&[0]), int(&[2]), int(&[0])]);
    let year = seq(&[int(&[0]), int(&[0]), int(&[0]), int(&[0x01, 0x6D])]);

    let found = judged(hour);
    assert_eq!(verdict_of(&found, checks), [Verdict::Met, Verdict::Met]);
    assert_eq!(verdict_of(&found, period), [Verdict::Met]);
    let found = judged(two_hours);
    assert_eq!(
        verdict_of(&found, checks),
        [Verdict::NotEvaluated, Verdict::NotEvaluated]
    );
    assert_eq!(verdict_of(&found, period), [Verdict::Met]);
    assert!(
        found.iter().any(|finding| finding
            .to_string()
            .contains("issued at or after 2026-11-05T22:00:00Z")),
        "the check's sentence names the bound"
    );
    let found = judged(year);
    assert_eq!(verdict_of(&found, period), [Verdict::NotEvaluated]);
    assert_eq!(
        verdict_of(&found, |c| matches!(c, Constraint::ConditionMember { .. })),
        Vec::<Verdict>::new(),
        "neither member is named as unevaluated any more"
    );
}

/// TR 102 272 clause 6.8 over the authority that issued the signature's time-stamp token: its own
/// trust trees, its `ttsRevReq` answered per kind of certificate from the material beside the
/// signature (clause 6.6.2), and clause 6.10's `tsaCertAlgorithmConstraints` on the authority's
/// certificate (ADR 2720). The hierarchy is `revocation_checks`' minted one.
#[test]
fn the_time_stamp_trust_condition_is_judged_on_the_authoritys_own_path() {
    use crate::policy_rules::certification_path::tests::hex;
    use crate::policy_rules::revocation_checks::tests::fixtures as minted;

    let token = hex(minted::TOKEN);
    let bytes = cms::fixtures::detached_with_signature_timestamp(&[0x00; 32], &token);
    let signed = cms::signed_data(&bytes).expect("a SignedData");
    let (root_crl, intermediate_crl, revoked) = (
        hex(minted::ROOT_CRL),
        hex(minted::INTERMEDIATE_CRL),
        hex(minted::INTERMEDIATE_CRL_REVOKED),
    );
    // 2027-01-01T00:00:00Z, after every piece of the minted material was issued.
    let at = Instant::from_unix_seconds(1_798_761_600);
    let crl_check = || seq(&[tlv(ENUMERATED, &[0])]);
    let rsa = |bits: &[u8]| seq(&[oid(SHA256_RSA), tlv(der::INTEGER, bits)]);
    let judged = |material: &Material<'_>, minimum: &[u8]| {
        let octets = Fixture {
            timestamp_condition: vec![
                tagged(0, &seq(&[seq(&[hex(minted::ROOT)])])),
                tagged(1, &seq(&[crl_check(), tagged(0, &crl_check())])),
            ],
            issuer_algorithms: vec![tagged(4, &seq(&[rsa(minimum)]))],
            ..Fixture::default()
        }
        .octets();
        let document = PolicyDocument::parse(&octets).expect("a clause 6 document");
        findings(
            &document,
            &signed,
            &facts(None).validated_at(at).with_material(material),
        )
    };
    let stamp = |c: &Constraint| matches!(c, Constraint::TrustPath { condition, .. } | Constraint::RevocationCheck { condition, .. } if *condition == TIMESTAMP_TRUST);
    let tsa = |c: &Constraint| matches!(c, Constraint::IssuerAlgorithm { member, .. } if *member == TSA_ALGORITHMS);

    let good = Material::read(&[&root_crl, &intermediate_crl], &[]);
    let met = judged(&good, &[0x08, 0x00]);
    assert_eq!(
        verdict_of(&met, stamp),
        [Verdict::Met, Verdict::Met, Verdict::Met],
        "the authority's path to the condition's trust point, then clrCheck on the authority and on the intermediate"
    );
    assert_eq!(
        verdict_of(&met, tsa),
        [Verdict::Met],
        "the authority's certificate, issued under SHA-256 with RSA by a 2048-bit key"
    );
    assert_eq!(
        verdict_of(&judged(&good, &[0x0C, 0x00]), tsa),
        [Verdict::Broken],
        "a 3072-bit minimum the 2048-bit issuer key is under"
    );

    let listed = Material::read(&[&root_crl, &revoked], &[]);
    assert_eq!(
        verdict_of(&judged(&listed, &[0x08, 0x00]), stamp),
        [Verdict::Met, Verdict::Broken, Verdict::Met],
        "the intermediate's CRL lists the authority, revoked before the instant"
    );
    assert_eq!(
        verdict_of(&judged(&Material::none(), &[0x08, 0x00]), stamp),
        [Verdict::Met, Verdict::NotEvaluated, Verdict::NotEvaluated],
        "no material answers either check, which is indeterminate rather than broken"
    );

    // No ttsCertificateTrustTrees: clause 6.8 applies the signing-certificate condition's trees to
    // the authority, so the root stated there is the authority's trust point too.
    let octets = Fixture {
        trust_points: vec![seq(&[hex(minted::ROOT)])],
        ..Fixture::default()
    }
    .octets();
    let document = PolicyDocument::parse(&octets).expect("a clause 6 document");
    let inherited = findings(&document, &signed, &facts(None).validated_at(at));
    assert_eq!(verdict_of(&inherited, stamp), [Verdict::Met]);
}

/// Clause 6.9's `AttributeTrustCondition`: `attributeMandated`, `howCertAttribute`, and the
/// `attributeConstraints` lists given.
fn attribute_condition(
    mandated: bool,
    how: u8,
    types: Option<&[&[u8]]>,
    value_types: Option<&[&[u8]]>,
) -> Vec<Vec<u8>> {
    let mut members = vec![
        tlv(BOOLEAN, &[if mandated { 0xFF } else { 0x00 }]),
        tlv(ENUMERATED, &[how]),
    ];
    let mut lists = Vec::new();
    if let Some(types) = types {
        lists.push(tagged(0, &identifiers(types)));
    }
    if let Some(value_types) = value_types {
        let pairs: Vec<_> = value_types
            .iter()
            .map(|kind| seq(&[oid(kind), tlv(0x0C, b"x")]))
            .collect();
        lists.push(tagged(1, &seq(&pairs)));
    }
    if types.is_some() || value_types.is_some() {
        members.push(tagged(2, &seq(&lists)));
    }
    members
}

#[test]
fn a_mandated_attribute_within_the_attribute_trust_condition_is_met_and_its_absence_broken() {
    use signer_attributes::Check;
    use signer_attributes::tests::{ROLE, TITLE, carrying, v2};
    // TR 102 272 clause 6.9: `attributeMandated` true requires an attribute within the
    // condition's constraints; `claimedAttribute` (0) accepts claimed ones, and
    // `attributeTypeConstraints` names the role as valid with any value.
    let octets = Fixture {
        attribute_condition: Some(attribute_condition(true, 0, Some(&[ROLE]), None)),
        ..Fixture::default()
    }
    .octets();
    let document = PolicyDocument::parse(&octets).expect("a clause 6 document");
    let mandated =
        |c: &Constraint| matches!(c, Constraint::SignerAttribute(Check::Mandated { .. }));
    let certified =
        |c: &Constraint| matches!(c, Constraint::SignerAttribute(Check::Certificate { .. }));
    let verdicts = |value: &[u8], wanted: &dyn Fn(&Constraint) -> bool| {
        verdict_of(
            &findings(
                &document,
                &carrying(cms::ID_AA_ETS_SIGNER_ATTR_V2, value),
                &facts(None),
            ),
            wanted,
        )
    };
    assert_eq!(verdicts(&v2(&[ROLE], 0, 0), &mandated), [Verdict::Met]);
    assert_eq!(verdicts(&v2(&[TITLE], 0, 0), &mandated), [Verdict::Broken]);
    // An attribute certificate is not of the accepted kind: it is passed over, never judged.
    assert_eq!(verdicts(&v2(&[], 1, 0), &mandated), [Verdict::Broken]);
    assert!(verdicts(&v2(&[], 1, 0), &certified).is_empty());
    let none = findings(&document, &default_cms(), &facts(None));
    assert_eq!(verdict_of(&none, mandated), [Verdict::Broken]);
    let sentence = none
        .iter()
        .find(|finding| mandated(&finding.constraint))
        .map(ToString::to_string)
        .unwrap_or_default();
    assert!(
        sentence.contains("claimed") && sentence.contains("broken"),
        "{sentence}"
    );
}

#[test]
fn a_certified_or_value_constrained_attribute_leaves_the_condition_not_evaluated() {
    use signer_attributes::Check;
    use signer_attributes::tests::{TITLE, carrying, v2};
    let mandated =
        |c: &Constraint| matches!(c, Constraint::SignerAttribute(Check::Mandated { .. }));
    let named = |c: &Constraint| {
        matches!(
            c,
            Constraint::SignerAttribute(Check::Certificate { .. } | Check::ConditionValues)
        )
    };
    // `either` (2) accepts a certified attribute, whose validity needs RFC 5755 section 5.
    let either = Fixture {
        attribute_condition: Some(attribute_condition(true, 2, None, None)),
        ..Fixture::default()
    }
    .octets();
    let document = PolicyDocument::parse(&either).expect("a clause 6 document");
    let value = v2(&[], 1, 0);
    let found = findings(
        &document,
        &carrying(cms::ID_AA_ETS_SIGNER_ATTR_V2, &value),
        &facts(None),
    );
    assert_eq!(verdict_of(&found, mandated), [Verdict::NotEvaluated]);
    assert_eq!(verdict_of(&found, named), [Verdict::NotEvaluated]);
    // A claimed title that only `attributeValueConstraints` names is valid for one value only.
    let by_value = Fixture {
        attribute_condition: Some(attribute_condition(true, 0, None, Some(&[TITLE]))),
        ..Fixture::default()
    }
    .octets();
    let document = PolicyDocument::parse(&by_value).expect("a clause 6 document");
    let value = v2(&[TITLE], 0, 0);
    let found = findings(
        &document,
        &carrying(cms::ID_AA_ETS_SIGNER_ATTR_V2, &value),
        &facts(None),
    );
    assert_eq!(verdict_of(&found, mandated), [Verdict::NotEvaluated]);
    assert_eq!(verdict_of(&found, named), [Verdict::NotEvaluated]);
    // Not mandated, and nothing certified carried: nothing to judge.
    let optional = Fixture {
        attribute_condition: Some(attribute_condition(false, 2, None, None)),
        ..Fixture::default()
    }
    .octets();
    let document = PolicyDocument::parse(&optional).expect("a clause 6 document");
    let found = findings(&document, &default_cms(), &facts(None));
    assert!(
        !found
            .iter()
            .any(|finding| matches!(finding.constraint, Constraint::SignerAttribute(_))),
        "{found:?}"
    );
}
