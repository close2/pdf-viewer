//! Synthetic ETSI TS 119 172-3 policy documents, each built to the clause 4 rule it tests, one
//! constraint broken per test.

use super::*;
use crate::policy_rules::tests::{
    POLICY, PROOF_OF_ORIGIN, SHA256, SHA256_RSA, facts, oid, policy, seq, signed_data, tagged,
    time, tlv, verdict_of,
};
use crate::policy_rules::{self, Enforcement, NotEnforced, enforce};

fn ia5(text: &str) -> Vec<u8> {
    tlv(IA5_STRING, text.as_bytes())
}

fn utf8(text: &str) -> Vec<u8> {
    tlv(UTF8_STRING, text.as_bytes())
}

fn enumeration(number: u8) -> Vec<u8> {
    tlv(ENUMERATED, &[number])
}

/// A non-negative `INTEGER` below 2^31, minimally encoded.
fn int(number: u32) -> Vec<u8> {
    let octets: Vec<u8> = number
        .to_be_bytes()
        .into_iter()
        .skip_while(|octet| *octet == 0)
        .collect();
    match octets.first() {
        None => tlv(INTEGER, &[0]),
        Some(first) if first & 0x80 != 0 => tlv(INTEGER, &[&[0][..], &octets].concat()),
        Some(_) => tlv(INTEGER, &octets),
    }
}

/// A `PolicyRuleWithScope` holding one `BasicRule` alternative.
fn basic(number: u8, rule: &[u8], scope: Option<u8>) -> Vec<u8> {
    let mut members = vec![tagged(1, &tagged(number, rule))];
    members.extend(scope.map(enumeration));
    seq(&members)
}

/// The components of a policy with the given rules and signing period's end.
fn components(rules: &[Vec<u8>], not_after: Option<&str>) -> Vec<u8> {
    let mut period = vec![time("20260101000000Z")];
    period.extend(not_after.map(time));
    let details = seq(&[time("20251201000000Z"), tagged(0, &seq(&period))]);
    let general = seq(&[
        seq(&[oid(POLICY), seq(&[seq(&[tlv(0x13, b"en"), utf8("tests")])])]),
        tagged(1, &details),
    ]);
    seq(&[general, seq(rules)])
}

/// A whole `SignaturePolicy` over `components`, its digest computed as clause 4.3.2 says.
fn document_of(components: &[u8]) -> Vec<u8> {
    let digest = Digest::Sha256.compute(&[components]);
    seq(&[
        seq(&[seq(&[oid(SHA256)]), tlv(OCTET_STRING, &digest)]),
        components.to_vec(),
    ])
}

fn document(rules: &[Vec<u8>]) -> Vec<u8> {
    document_of(&components(rules, None))
}

/// The findings of `rules` against the default signature, signed at `at` by its timestamp.
fn judged(rules: &[Vec<u8>], cms: &SignedData<'_>, facts: &SignerFacts) -> Vec<Finding> {
    let octets = document(rules);
    let parsed = PolicyDocument::parse(&octets).expect("a clause 4 document");
    findings(&parsed, cms, facts)
}

fn default_cms() -> SignedData<'static> {
    signed_data(
        vec![cms::ID_CONTENT_TYPE, cms::ID_MESSAGE_DIGEST],
        vec![],
        vec![],
    )
}

fn component_verdicts(findings: &[Finding], wanted: impl Fn(&Constraint) -> bool) -> Vec<Verdict> {
    verdict_of(
        findings,
        |constraint| matches!(constraint, policy_rules::Constraint::PolicyComponent(inner) if wanted(inner)),
    )
}

/// The plant's document: a signing period that ended before the timestamp's instant, bound by
/// the digest of its whole octets, is enforced and broken; and the same octets read as TR 102
/// 272's type are refused by name as the other syntax.
#[test]
fn a_signing_period_that_ended_before_the_signing_time_is_broken() {
    let octets = document_of(&components(
        &[basic(
            2,
            &seq(&[tagged(1, &seq(&[int(1), enumeration(1)]))]),
            None,
        )],
        Some("20260601000000Z"),
    ));
    let whole = Digest::Sha256.compute(&[&octets]);
    let Enforcement::Enforced { syntax, findings } = enforce(
        &policy(&octets, &whole, "1.2.3.4"),
        &default_cms(),
        &facts(Some("20260701000000Z")),
    ) else {
        panic!("a bound document is enforced");
    };
    assert_eq!(syntax, Syntax::Ts1191723);
    let broken: Vec<&Finding> = findings
        .iter()
        .filter(|finding| finding.verdict == Verdict::Broken)
        .collect();
    assert_eq!(broken.len(), 1, "{findings:?}");
    assert!(matches!(
        broken[0].constraint,
        policy_rules::Constraint::SigningPeriod { .. }
    ));
    assert_eq!(
        policy_rules::PolicyDocument::parse(&octets),
        Err(PolicyDocumentError::OtherSyntax(Syntax::Tr102272))
    );
    assert_eq!(Syntax::of(&octets), Some(Syntax::Ts1191723));
}

#[test]
fn the_first_syntax_is_refused_by_name_as_the_other() {
    // TR 102 272's top level opens with an `AlgorithmIdentifier` whose first value is an OID.
    let octets = seq(&[seq(&[oid(SHA256)]), seq(&[]), tlv(OCTET_STRING, &[0; 32])]);
    assert_eq!(Syntax::of(&octets), Some(Syntax::Tr102272));
    assert_eq!(
        PolicyDocument::parse(&octets),
        Err(PolicyDocumentError::OtherSyntax(Syntax::Ts1191723))
    );
}

#[test]
fn a_media_type_list_that_leaves_out_pdf_is_broken() {
    let rule = |member: u8, types: &[&str]| {
        let list = seq(&types.iter().map(|text| utf8(text)).collect::<Vec<_>>());
        basic(0, &seq(&[seq(&[tagged(member, &list)])]), None)
    };
    let found = judged(
        &[
            rule(0, &["text/plain"]),
            rule(0, &["Application/PDF"]),
            rule(1, &["application/pdf"]),
            rule(1, &["image/png"]),
        ],
        &default_cms(),
        &facts(None),
    );
    assert_eq!(
        component_verdicts(&found, |c| matches!(
            c,
            Constraint::AnyOfMediaType(_) | Constraint::NoneOfMediaType(_)
        )),
        [Verdict::Broken, Verdict::Met, Verdict::Broken, Verdict::Met]
    );
}

#[test]
fn a_cardinality_one_document_does_not_meet_is_broken() {
    let bound = |member: u8, number: u32, qualifier: u8| {
        tagged(member, &seq(&[int(number), enumeration(qualifier)]))
    };
    let found = judged(
        &[
            basic(2, &seq(&[bound(0, 1, 0)]), None),
            basic(2, &seq(&[bound(0, 1, 1), bound(1, 1, 1)]), None),
            basic(1, &seq(&[seq(&[bound(1, 2, 2)])]), None),
        ],
        &default_cms(),
        &facts(None),
    );
    assert_eq!(
        component_verdicts(&found, |c| matches!(c, Constraint::DataObjectCount { .. })),
        [Verdict::Broken, Verdict::Met, Verdict::Met, Verdict::Broken]
    );
}

#[test]
fn a_position_other_than_enveloped_is_broken_and_asic_is_not_evaluated() {
    let positions = |values: &[u8]| {
        basic(
            3,
            &seq(&values
                .iter()
                .map(|value| enumeration(*value))
                .collect::<Vec<_>>()),
            None,
        )
    };
    let found = judged(
        &[
            positions(&[2]),
            positions(&[1]),
            positions(&[3]),
            positions(&[1, 0]),
        ],
        &default_cms(),
        &facts(None),
    );
    assert_eq!(
        component_verdicts(&found, |c| matches!(c, Constraint::RelativePosition(_))),
        [
            Verdict::Broken,
            Verdict::Met,
            Verdict::NotEvaluated,
            Verdict::Broken
        ]
    );
    assert_eq!(
        PolicyDocument::parse(&document(&[positions(&[3, 1])])),
        Err(PolicyDocumentError::ComponentMalformed(
            "SigDTBSRelativePosition"
        ))
    );
}

#[test]
fn a_signing_certificate_the_signature_does_not_carry_is_broken() {
    let rule = basic(6, &seq(&[seq(&[seq(&[]), seq(&[])]), enumeration(0)]), None);
    let mut without = facts(None);
    without.carries_signer_certificate = false;
    let found = judged(&[rule], &default_cms(), &without);
    assert_eq!(
        verdict_of(&found, |c| matches!(
            c,
            policy_rules::Constraint::CertificateInfo(CertificateInfo::SignerOnly)
        )),
        [Verdict::Broken]
    );
    assert_eq!(
        verdict_of(&found, |c| matches!(
            c,
            policy_rules::Constraint::TrustCondition(SIGNING_CERT_TRUST)
        )),
        [Verdict::NotEvaluated]
    );
}

#[test]
fn a_timestamp_later_than_the_delay_allows_is_broken() {
    let delay = |seconds: u32| {
        let condition = seq(&[tagged(3, &seq(&[int(seconds), int(0), int(0), int(0)]))]);
        basic(
            7,
            &seq(&[seq(&[seq(&[ia5("urn:tst")]), ia5("high"), condition])]),
            None,
        )
    };
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
    let found = judged(
        &[delay(60), delay(7200)],
        &cms,
        &facts(Some("20260301010000Z")),
    );
    assert_eq!(
        component_verdicts(&found, |c| matches!(c, Constraint::TimestampDelay { .. })),
        [Verdict::Broken, Verdict::Met]
    );
    assert_eq!(
        component_verdicts(&found, |c| matches!(c, Constraint::TimeEvidence { .. })),
        [Verdict::NotEvaluated, Verdict::NotEvaluated]
    );
    // No claimed signing time: TS 119 172-2 clause 4.25.1 has no instant to measure from.
    let unclaimed = judged(
        &[delay(60)],
        &default_cms(),
        &facts(Some("20260301010000Z")),
    );
    assert_eq!(
        component_verdicts(&unclaimed, |c| matches!(
            c,
            Constraint::TimestampDelay { .. }
        )),
        [Verdict::NotEvaluated]
    );
}

#[test]
fn a_signer_attribute_the_policy_prohibits_is_broken() {
    let rule = basic(8, &seq(&[tlv(BOOLEAN, &[0xFF]), seq(&[])]), None);
    let carrying = signed_data(
        vec![
            cms::ID_CONTENT_TYPE,
            cms::ID_MESSAGE_DIGEST,
            cms::ID_AA_ETS_SIGNER_ATTR_V2,
        ],
        vec![],
        vec![],
    );
    let none = |c: &Constraint| matches!(c, Constraint::NoSignerAttributes);
    assert_eq!(
        component_verdicts(
            &judged(std::slice::from_ref(&rule), &carrying, &facts(None)),
            none
        ),
        [Verdict::Broken]
    );
    assert_eq!(
        component_verdicts(&judged(&[rule], &default_cms(), &facts(None)), none),
        [Verdict::Met]
    );
}

fn suite(algorithm: &[u8], usages: &[&str], key: Option<u32>, hash: Option<u32>) -> Vec<u8> {
    let mut members = vec![
        oid(algorithm),
        seq(&usages.iter().map(|usage| ia5(usage)).collect::<Vec<_>>()),
    ];
    members.extend(key.map(|bits| tagged(0, &int(bits))));
    members.extend(hash.map(|bits| tagged(1, &int(bits))));
    seq(&members)
}

#[test]
fn an_algorithm_a_suite_does_not_admit_or_a_key_too_short_is_broken() {
    const TSTK: &str = "http://uri.etsi.org/19172/v1.1.1/Usage#Tstk";
    let suites = |entries: &[Vec<u8>]| basic(11, &seq(entries), None);
    let found = judged(
        &[
            suites(&[
                suite(SHA256, &[SIGNATURE_USAGE], None, Some(256)),
                suite(SHA256_RSA, &[SIGNATURE_USAGE, TSTK], Some(3072), None),
                suite(SHA256_RSA, &[SIGNING_CERT_USAGE[0]], None, None),
            ]),
            suites(&[suite(SHA256, &[SIGNATURE_USAGE], None, Some(384))]),
        ],
        &default_cms(),
        &facts(None),
    );
    let verdicts = |usage: Usage| {
        component_verdicts(
            &found,
            |c| matches!(c, Constraint::SuiteAlgorithm { usage: u, .. } if *u == usage),
        )
    };
    // The first list: the digest is long enough, the 2048-bit key is short of 3072, and the
    // certificate's algorithm is listed. The second: a 256-bit hash is short of 384, and the
    // signature algorithm is not listed for the signature at all.
    assert_eq!(
        verdicts(Usage::Signature),
        [
            Verdict::Met,
            Verdict::Broken,
            Verdict::Broken,
            Verdict::Broken
        ]
    );
    assert_eq!(verdicts(Usage::SigningCertificate), [Verdict::Met]);
    assert_eq!(
        component_verdicts(
            &found,
            |c| matches!(c, Constraint::SuiteUsage(u) if u == TSTK)
        ),
        [Verdict::NotEvaluated]
    );
}

#[test]
fn a_rule_scoped_to_generation_is_named_and_not_enforced() {
    let fewer_than_one = seq(&[tagged(0, &seq(&[int(1), enumeration(0)]))]);
    let found = judged(
        &[basic(2, &fewer_than_one, Some(0))],
        &default_cms(),
        &facts(None),
    );
    assert_eq!(
        component_verdicts(&found, |c| matches!(
            c,
            Constraint::OutsideValidation {
                scope: Scope::Generation,
                ..
            }
        )),
        [Verdict::NotEvaluated]
    );
    assert!(
        found
            .iter()
            .all(|finding| finding.verdict != Verdict::Broken)
    );
}

#[test]
fn a_commitment_rule_is_enforced_only_where_its_matching_indicator_selects_the_signature() {
    let fewer_than_one = basic_alternative(2, &seq(&[tagged(0, &seq(&[int(1), enumeration(0)]))]));
    let commitment = |matching: u8| {
        seq(&[tagged(
            0,
            &seq(&[seq(&[
                seq(&[seq(&[oid(PROOF_OF_ORIGIN)])]),
                enumeration(matching),
                seq(std::slice::from_ref(&fewer_than_one)),
            ])]),
        )])
    };
    let indication = seq(&[oid(PROOF_OF_ORIGIN)]);
    let indication = der::Reader::new(&indication)
        .expect("small")
        .next_value()
        .expect("well formed");
    let stating = signed_data(
        vec![cms::ID_CONTENT_TYPE, cms::ID_MESSAGE_DIGEST],
        vec![],
        vec![cms::Attribute {
            kind: cms::ID_AA_ETS_COMMITMENT_TYPE,
            signed: true,
            values: 1,
            first: indication,
        }],
    );
    let count = |rules: &[Vec<u8>], cms: &SignedData<'_>| {
        component_verdicts(&judged(rules, cms, &facts(None)), |c| {
            matches!(c, Constraint::DataObjectCount { .. })
        })
    };
    // `all`: applies to the signature stating the commitment and not to one stating none.
    assert_eq!(count(&[commitment(0)], &stating), [Verdict::Broken]);
    assert!(count(&[commitment(0)], &default_cms()).is_empty());
    // `none`: the other way round.
    assert!(count(&[commitment(1)], &stating).is_empty());
    assert_eq!(count(&[commitment(1)], &default_cms()), [Verdict::Broken]);
}

/// A `BasicRule` alternative on its own, as a commitment rule lists it.
fn basic_alternative(number: u8, rule: &[u8]) -> Vec<u8> {
    tagged(number, rule)
}

#[test]
fn the_rules_this_program_does_not_evaluate_are_named_and_never_met() {
    let found = judged(
        &[
            basic(4, &seq(&[tagged(1, &seq(&[ia5("urn:level:B-T")]))]), None),
            basic(
                5,
                &seq(&[tlv(BOOLEAN, &[0]), ia5("urn:level"), enumeration(1)]),
                None,
            ),
            basic(9, &seq(&[seq(&[tagged(0, &ia5("urn:level:B-B"))])]), None),
            basic(10, &ia5("urn:loa:high"), None),
            basic(12, &seq(&[oid(&[0x2A, 0x03, 0x09])]), None),
        ],
        &default_cms(),
        &facts(None),
    );
    assert_eq!(found.len(), 6, "a signing period and five rules: {found:?}");
    assert!(
        found[1..]
            .iter()
            .all(|finding| finding.verdict == Verdict::NotEvaluated),
        "{found:?}"
    );
    for finding in &found[1..] {
        assert!(finding.to_string().contains("not evaluated"), "{finding}");
    }
}

#[test]
fn a_digest_that_disagrees_with_its_components_is_refused() {
    let mut octets = document(&[basic(10, &ia5("urn:loa"), None)]);
    let last = octets.len() - 1;
    octets[last] ^= 1;
    assert_eq!(
        PolicyDocument::parse(&octets),
        Err(PolicyDocumentError::DigestDisagrees)
    );
}

#[test]
fn a_document_in_the_basic_encoding_rules_is_read() {
    // Clause 4.1.3.2 admits BER: the outer value here states the indefinite length.
    let definite = document(&[basic(10, &ia5("urn:loa"), None)]);
    let contents = der::Reader::new(&definite)
        .expect("small")
        .next_value()
        .expect("well formed")
        .expect("one value")
        .contents;
    let indefinite = [&[0x30, 0x80][..], contents, &[0, 0]].concat();
    assert!(PolicyDocument::parse(&indefinite).is_ok());
}

#[test]
fn the_deepest_rule_read_inside_a_commitment_rule_is_within_the_depth_budget() {
    let delay = seq(&[tagged(3, &seq(&[int(1), int(0), int(0), int(0)]))]);
    let evidences = basic_alternative(7, &seq(&[seq(&[seq(&[]), ia5("high"), delay])]));
    let rule = seq(&[tagged(
        0,
        &seq(&[seq(&[
            seq(&[seq(&[oid(PROOF_OF_ORIGIN)])]),
            enumeration(1),
            seq(&[evidences]),
        ])]),
    )]);
    assert!(PolicyDocument::parse(&document(&[rule])).is_ok());
}

#[test]
fn the_entry_budget_holds_at_its_bound_and_refuses_past_it() {
    // One entry for the scoped rule, one for the media-type rule, and the rest its types.
    let with = |types: usize| {
        let list = seq(&vec![utf8("a/b"); types]);
        document(&[basic(0, &seq(&[seq(&[tagged(0, &list)])]), None)])
    };
    assert!(PolicyDocument::parse(&with(MAX_ENTRIES - 2)).is_ok());
    assert_eq!(
        PolicyDocument::parse(&with(MAX_ENTRIES - 1)),
        Err(PolicyDocumentError::TooManyEntries)
    );
}

#[test]
fn a_nesting_past_the_depth_budget_or_a_document_past_the_byte_budget_is_refused() {
    // The grammar is of a fixed depth and a definite-length value this module does not read is
    // stepped over unopened, so indefinite lengths are what make the reader nest: finding where
    // one ends walks inside it, and that walk is what the depth budget holds.
    let levels = usize::from(der::MAX_DEPTH) + 1;
    let nested = [[0x30, 0x80].repeat(levels), [0, 0].repeat(levels)].concat();
    assert_eq!(
        PolicyDocument::parse(&nested),
        Err(PolicyDocumentError::Encoding(der::DerError::TooDeep))
    );
    let mut long = vec![0u8; MAX_POLICY_OCTETS];
    long.push(0);
    assert_eq!(
        PolicyDocument::parse(&long),
        Err(PolicyDocumentError::TooLarge { octets: long.len() })
    );
}

#[test]
fn only_a_bound_document_naming_the_signers_policy_is_enforced() {
    let octets = document(&[basic(10, &ia5("urn:loa"), None)]);
    let cms = default_cms();
    let whole = Digest::Sha256.compute(&[&octets]);
    let enforced = |value: &[u8], identifier: &str| {
        enforce(&policy(&octets, value, identifier), &cms, &facts(None))
    };
    assert!(matches!(
        enforced(&whole, "1.2.3.4"),
        Enforcement::Enforced {
            syntax: Syntax::Ts1191723,
            ..
        }
    ));
    let parsed = PolicyDocument::parse(&octets).expect("a clause 4 document");
    assert!(matches!(
        enforced(parsed.digest.1, "1.2.3.4"),
        Enforcement::Enforced { .. }
    ));
    assert!(matches!(
        enforced(&[0x55; 32], "1.2.3.4"),
        Enforcement::NotEnforced(NotEnforced::NotBound(_))
    ));
    assert_eq!(
        enforced(&whole, "1.2.3.9"),
        Enforcement::NotEnforced(NotEnforced::IdentifierDiffers {
            document: "1.2.3.4".to_owned()
        })
    );
}

/// The memory probe's population, ignored by the ordinary run: a document as long as the byte
/// budget admits that opens nested indefinite-length values behind this syntax's first member,
/// and one a single entry past the entry budget, each parsed a thousand times.
#[test]
#[ignore = "the memory probe's population, run under getrusage"]
fn hostile_policy_documents_are_refused_within_their_budgets() {
    let mut deep = vec![0x30_u8; MAX_POLICY_OCTETS];
    for octet in deep.iter_mut().skip(1).step_by(2) {
        *octet = 0x80;
    }
    let list = seq(&vec![utf8("a/b"); MAX_ENTRIES - 1]);
    let past = document(&[basic(0, &seq(&[seq(&[tagged(0, &list)])]), None)]);
    for _ in 0..1000 {
        assert!(PolicyDocument::parse(&deep).is_err());
        assert_eq!(
            PolicyDocument::parse(&past),
            Err(PolicyDocumentError::TooManyEntries)
        );
    }
}
