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
        // rule; their contents are not read, so an empty sequence stands for each.
        common.push(tagged(1, &seq(&[])));
        common.push(tagged(2, &seq(&[])));
        if let Some(algorithms) = &self.algorithms {
            common.push(tagged(4, &seq(&[tagged(0, &seq(algorithms))])));
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

pub(super) fn facts(at: Option<&str>) -> SignerFacts {
    SignerFacts {
        signing_time: at.map(|spelled| SigningTime {
            instant: x509::read_time_digits(spelled.as_bytes(), true).expect("a time"),
            spelled: spelled.to_owned(),
            source: TimeSource::SignatureTimestamp,
        }),
        key_bits: Some(2048),
        carries_signer_certificate: true,
        certificate_signature_algorithm: Some(SHA256_RSA.to_vec()),
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
