//! Synthetic ETSI TS 119 172-2 policy documents, each built to the clause 4 element it tests, one
//! constraint broken per test.

use super::*;
use crate::cms::{self, Digest};
use crate::policy_rules::tests::{SHA256, facts, oid, policy, seq, signed_data, tlv, verdict_of};
use crate::policy_rules::ts_119_172_3::{Constraint, SIGNATURE_USAGE};
use crate::policy_rules::{self, Enforcement, NotEnforced, enforce};

/// The base64 of thirty-two zero octets, the stated digest of every fixture.
const ZERO_DIGEST: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

/// A whole document over `rules`, its signing period ending at `not_after` where given.
fn document_of(rules: &str, not_after: Option<&str>) -> String {
    let after = not_after.map_or(String::new(), |at| format!("<NotAfter>{at}</NotAfter>"));
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<SignaturePolicy xmlns="http://uri.etsi.org/19172/v1.1.1#" xmlns:ds="http://www.w3.org/2000/09/xmldsig#">
  <Digest>
    <ds:DigestMethod Algorithm="http://www.w3.org/2001/04/xmlenc#sha256"/>
    <ds:DigestValue>{ZERO_DIGEST}</ds:DigestValue>
    <ds:CanonicalizationMethod Algorithm="http://www.w3.org/2001/10/xml-exc-c14n#"/>
  </Digest>
  <PolicyComponents>
    <GeneralDetails>
      <SigPolicyDetails>
        <PolicyIdentifier>urn:oid:1.2.3.4</PolicyIdentifier>
        <PolicyName><tsl:Name xmlns:tsl="http://uri.etsi.org/02231/v2#" xml:lang="en">tests</tsl:Name></PolicyName>
      </SigPolicyDetails>
      <OtherDetails>
        <DateOfIssue>2025-12-01T00:00:00Z</DateOfIssue>
        <SigningPeriod><NotBefore>2026-01-01T00:00:00Z</NotBefore>{after}</SigningPeriod>
      </OtherDetails>
    </GeneralDetails>
    <PolicyRules>{rules}</PolicyRules>
  </PolicyComponents>
</SignaturePolicy>"#
    )
}

/// One `PolicyRule` holding `inner`, recommended for `scope` where given.
fn rule(inner: &str, scope: Option<&str>) -> String {
    match scope {
        Some(scope) => format!(r#"<PolicyRule RecommendedScope="{scope}">{inner}</PolicyRule>"#),
        None => format!("<PolicyRule>{inner}</PolicyRule>"),
    }
}

/// An augmentation rule, which every fixture that needs some rule can carry.
const AUGMENTATION: &str = "<AugmentationRules><PreviousValidationRequired>true\
    </PreviousValidationRequired><LevelId>urn:level</LevelId></AugmentationRules>";

fn judged(rules: &[String], cms: &SignedData<'_>, facts: &SignerFacts) -> Vec<Finding> {
    let octets = document_of(&rules.concat(), None);
    let reading = Reading::parse(octets.as_bytes()).expect("a clause 4 document");
    let document = reading.document().expect("a clause 4 document");
    findings(&document, cms, facts)
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
/// the digest of its whole octets, is enforced and broken; and the same octets read as either
/// ASN.1 syntax are refused by name as not that syntax.
#[test]
fn a_signing_period_that_ended_before_the_signing_time_is_broken() {
    let octets = document_of(&rule(AUGMENTATION, None), Some("2026-06-01T00:00:00Z"));
    let octets = octets.as_bytes();
    let whole = Digest::Sha256.compute(&[octets]);
    let Enforcement::Enforced { syntax, findings } = enforce(
        &policy(octets, &whole, "1.2.3.4"),
        &default_cms(),
        &facts(Some("20260701000000Z")),
    ) else {
        panic!("a bound document is enforced");
    };
    assert_eq!(syntax, Syntax::Ts1191722);
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
        policy_rules::PolicyDocument::parse(octets),
        Err(PolicyDocumentError::OtherSyntax(Syntax::Tr102272))
    );
    assert_eq!(
        ts_119_172_3::PolicyDocument::parse(octets),
        Err(PolicyDocumentError::OtherSyntax(Syntax::Ts1191723))
    );
    assert_eq!(Syntax::of(octets), Some(Syntax::Ts1191722));
    let inside = enforce(
        &policy(octets, &whole, "1.2.3.4"),
        &default_cms(),
        &facts(Some("20260301000000Z")),
    );
    assert!(matches!(inside, Enforcement::Enforced { findings, .. }
        if findings.iter().all(|finding| finding.verdict != Verdict::Broken)));
}

#[test]
fn an_asn1_or_foreign_document_is_refused_by_name_as_not_this_syntax() {
    let der = seq(&[seq(&[oid(SHA256)])]);
    assert_eq!(
        Reading::parse(&der).map(|_| ()),
        Err(PolicyDocumentError::OtherSyntax(Syntax::Ts1191722))
    );
    let foreign = r#"<SignaturePolicy xmlns="urn:another"/>"#;
    assert_eq!(
        Reading::parse(foreign.as_bytes()).map(|_| ()),
        Err(PolicyDocumentError::OtherSyntax(Syntax::Ts1191722))
    );
}

#[test]
fn a_media_type_list_that_leaves_out_pdf_is_broken() {
    let media = |list: &str| {
        rule(
            &format!(
                "<DataToBeSignedRules><AnyOfMimeType>{list}</AnyOfMimeType></DataToBeSignedRules>"
            ),
            None,
        )
    };
    let any = |c: &Constraint| matches!(c, Constraint::AnyOfMediaType(_));
    let found = judged(&[media("text/xml image/png")], &default_cms(), &facts(None));
    assert_eq!(component_verdicts(&found, any), [Verdict::Broken]);
    let found = judged(
        &[media("text/xml Application/PDF")],
        &default_cms(),
        &facts(None),
    );
    assert_eq!(component_verdicts(&found, any), [Verdict::Met]);
}

#[test]
fn a_cardinality_one_document_does_not_meet_is_broken() {
    let relation = |bound: &str| {
        rule(
            &format!(
                "<SigToDTBSRelationRules><DTBSCardinality>{bound}</DTBSCardinality>\
                 <SigDTBSRelativePosition>EnvelopedSig</SigDTBSRelativePosition>\
                 </SigToDTBSRelationRules>"
            ),
            None,
        )
    };
    let count = |c: &Constraint| matches!(c, Constraint::DataObjectCount { .. });
    let found = judged(
        &[relation(
            r#"<MinimumValue qualifier="HigherThan">1</MinimumValue>"#,
        )],
        &default_cms(),
        &facts(None),
    );
    assert_eq!(component_verdicts(&found, count), [Verdict::Broken]);
    let found = judged(
        &[relation(
            r#"<MaximumValue qualifier="LessOrEqualTo">1</MaximumValue>"#,
        )],
        &default_cms(),
        &facts(None),
    );
    assert_eq!(component_verdicts(&found, count), [Verdict::Met]);
    // A qualifier of the other bound is not this bound's.
    let wrong = document_of(
        &relation(r#"<MaximumValue qualifier="HigherThan">1</MaximumValue>"#),
        None,
    );
    assert_eq!(
        Reading::parse(wrong.as_bytes()).and_then(|reading| reading.document().map(|_| ())),
        Err(PolicyDocumentError::ElementMalformed(
            "DTBSCardinality.MaximumValue"
        ))
    );
}

#[test]
fn a_position_other_than_enveloped_is_broken_and_asic_is_not_evaluated() {
    let position = |list: &str| {
        rule(
            &format!(
                "<SigToDTBSRelationRules><DTBSCardinality/>\
                 <SigDTBSRelativePosition>{list}</SigDTBSRelativePosition></SigToDTBSRelationRules>"
            ),
            None,
        )
    };
    let wanted = |c: &Constraint| matches!(c, Constraint::RelativePosition(_));
    let verdicts = |list: &str| {
        component_verdicts(
            &judged(&[position(list)], &default_cms(), &facts(None)),
            wanted,
        )
    };
    assert_eq!(verdicts("DetachedSig"), [Verdict::Broken]);
    assert_eq!(verdicts("EnvelopedSig"), [Verdict::Met]);
    // A combination is one signature in both positions at once (clause 4.19.1).
    assert_eq!(verdicts("EnvelopingSig EnvelopedSig"), [Verdict::Broken]);
    assert_eq!(verdicts("ASiC"), [Verdict::NotEvaluated]);
}

#[test]
fn a_signing_certificate_the_signature_does_not_carry_is_broken() {
    let rules = [rule(
        "<SigningCertRules><MandatedSigningCertInfo>signingCertOnly</MandatedSigningCertInfo>\
         <SigningCertTrustConditions/></SigningCertRules>",
        None,
    )];
    let mut without = facts(None);
    without.carries_signer_certificate = false;
    let info =
        |c: &policy_rules::Constraint| matches!(c, policy_rules::Constraint::CertificateInfo(_));
    assert_eq!(
        verdict_of(&judged(&rules, &default_cms(), &without), info),
        [Verdict::Broken]
    );
    assert_eq!(
        verdict_of(&judged(&rules, &default_cms(), &facts(None)), info),
        [Verdict::Met]
    );
}

#[test]
fn a_timestamp_later_than_the_delay_allows_is_broken() {
    let delay = |seconds: u32| {
        rule(
            &format!(
                "<TimeEvidencesRules><RulesForSetOfEvidences>\
                 <EvidenceIdentifiers><Identifier>urn:tst</Identifier></EvidenceIdentifiers>\
                 <LevelOfAssurance>urn:high</LevelOfAssurance><TimeStampTrustCondition>\
                 <SignatureTimeStampDelay><DeltaSeconds>{seconds}</DeltaSeconds>\
                 <DeltaMinutes>0</DeltaMinutes><DeltaHours>0</DeltaHours><DeltaDays>0</DeltaDays>\
                 </SignatureTimeStampDelay></TimeStampTrustCondition>\
                 </RulesForSetOfEvidences></TimeEvidencesRules>"
            ),
            None,
        )
    };
    let claimed = tlv(0x18, b"20260301000000Z");
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
}

#[test]
fn a_signer_attribute_the_policy_prohibits_is_broken() {
    let rules = [rule(
        "<SignerAttributesConstraints><NoSignerAttributesAllowed/></SignerAttributesConstraints>",
        None,
    )];
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
        component_verdicts(&judged(&rules, &carrying, &facts(None)), none),
        [Verdict::Broken]
    );
    assert_eq!(
        component_verdicts(&judged(&rules, &default_cms(), &facts(None)), none),
        [Verdict::Met]
    );
}

/// One `AlgConstraints` entry.
fn suite(algorithm: &str, usages: Option<&str>, key: Option<u32>) -> String {
    let usages = usages.map_or(String::new(), |list| format!("<Usages>{list}</Usages>"));
    let key = key.map_or(String::new(), |bits| {
        format!("<MinKeyLength>{bits}</MinKeyLength>")
    });
    format!(
        "<AlgConstraints><AlgId>{algorithm}</AlgId>{usages}\
         <ExpirationDate>2030-01-01T00:00:00Z</ExpirationDate>{key}</AlgConstraints>"
    )
}

fn suites(entries: &[String]) -> String {
    rule(
        &format!(
            "<CryptoSuitesRules>{}</CryptoSuitesRules>",
            entries.concat()
        ),
        None,
    )
}

const RSA_SHA256: &str = "http://www.w3.org/2001/04/xmldsig-more#rsa-sha256";
const SHA256_URI: &str = "http://www.w3.org/2001/04/xmlenc#sha256";

#[test]
fn an_algorithm_a_suite_does_not_admit_or_a_key_too_short_is_broken() {
    let algorithm = |c: &Constraint| matches!(c, Constraint::SuiteAlgorithm { .. });
    let verdicts = |entries: &[String]| {
        component_verdicts(
            &judged(&[suites(entries)], &default_cms(), &facts(None)),
            algorithm,
        )
    };
    // The XML Signature URIs and RFC 3061's URN place the same two algorithms.
    assert_eq!(
        verdicts(&[
            suite(SHA256_URI, Some(SIGNATURE_USAGE), None),
            suite(RSA_SHA256, Some(SIGNATURE_USAGE), Some(2048)),
        ]),
        [Verdict::Met, Verdict::Met]
    );
    assert_eq!(
        verdicts(&[
            suite(
                "urn:oid:2.16.840.1.101.3.4.2.1",
                Some(SIGNATURE_USAGE),
                None
            ),
            suite(
                "URN:OID:1.2.840.113549.1.1.11",
                Some(SIGNATURE_USAGE),
                Some(4096)
            ),
        ]),
        [Verdict::Met, Verdict::Broken]
    );
    // The signature algorithm is not listed.
    assert_eq!(
        verdicts(&[suite(SHA256_URI, Some(SIGNATURE_USAGE), None)]),
        [Verdict::Met, Verdict::Broken]
    );
    // An absent `Usages` is every usage, the signing certificate's included.
    assert_eq!(
        verdicts(&[suite(SHA256_URI, None, None), suite(RSA_SHA256, None, None)]),
        [Verdict::Met, Verdict::Met, Verdict::Met]
    );
}

#[test]
fn an_algorithm_named_by_a_uri_this_program_cannot_place_is_not_evaluated() {
    let found = judged(
        &[suites(&[
            suite(SHA256_URI, Some(SIGNATURE_USAGE), None),
            suite("urn:example:rsa", Some(SIGNATURE_USAGE), None),
        ])],
        &default_cms(),
        &facts(None),
    );
    assert_eq!(
        component_verdicts(&found, |c| matches!(c, Constraint::SuiteAlgorithm { .. })),
        [Verdict::Met, Verdict::NotEvaluated]
    );
    assert_eq!(
        component_verdicts(&found, |c| matches!(c, Constraint::UnplacedAlgorithm(_))),
        [Verdict::NotEvaluated]
    );
    assert_eq!(
        component_verdicts(&found, |c| matches!(
            c,
            Constraint::AlgorithmExpiration { .. }
        )),
        [Verdict::NotEvaluated, Verdict::NotEvaluated]
    );
}

#[test]
fn a_rule_recommended_for_generation_alone_is_named_and_not_enforced() {
    let media = "<DataToBeSignedRules><AnyOfMimeType>text/xml</AnyOfMimeType>\
                 </DataToBeSignedRules>";
    let outside = |c: &Constraint| matches!(c, Constraint::OutsideValidation { .. });
    let any = |c: &Constraint| matches!(c, Constraint::AnyOfMediaType(_));
    let found = judged(
        &[rule(media, Some("Generation"))],
        &default_cms(),
        &facts(None),
    );
    assert_eq!(component_verdicts(&found, outside), [Verdict::NotEvaluated]);
    assert!(component_verdicts(&found, any).is_empty());
    let found = judged(
        &[rule(media, Some("Generation Validation"))],
        &default_cms(),
        &facts(None),
    );
    assert_eq!(component_verdicts(&found, any), [Verdict::Broken]);
}

#[test]
fn a_commitment_rule_is_enforced_only_where_its_matching_indicator_selects_the_signature() {
    let commitment = |matching: &str| {
        rule(
            &format!(
                r#"<CommitmentRules><ComitmentRulesComponent MatchingIndicator="{matching}">
                <Commitment><Identifier>urn:oid:1.2.3.5</Identifier></Commitment>
                <DataToBeSignedRules><AnyOfMimeType>text/xml</AnyOfMimeType></DataToBeSignedRules>
                </ComitmentRulesComponent></CommitmentRules>"#
            ),
            None,
        )
    };
    let any = |c: &Constraint| matches!(c, Constraint::AnyOfMediaType(_));
    // The default signature states no commitment type: `All` and `AtLeastOne` do not select it.
    for (matching, expected) in [
        ("All", vec![]),
        ("AtLeastOne", vec![]),
        ("None", vec![Verdict::Broken]),
    ] {
        let found = judged(&[commitment(matching)], &default_cms(), &facts(None));
        assert_eq!(component_verdicts(&found, any), expected, "{matching}");
    }
}

#[test]
fn the_rules_this_program_does_not_evaluate_are_named_and_never_met() {
    let found = judged(
        &[
            rule(AUGMENTATION, None),
            rule(
                "<SigFormatsAndLevels><SigFormats><Format>http://uri.etsi.org/ades/format/pades\
                 </Format></SigFormats></SigFormatsAndLevels>",
                None,
            ),
            rule(
                "<QualifyingPropertiesRules><LevelRules><LevelIdentifier>urn:b-b</LevelIdentifier>\
                 </LevelRules></QualifyingPropertiesRules>",
                None,
            ),
            rule("<SCDLoARules>urn:qscd</SCDLoARules>", None),
            rule(
                r#"<OtherRule><x:Rule xmlns:x="urn:extension"/></OtherRule>"#,
                None,
            ),
        ],
        &default_cms(),
        &facts(None),
    );
    assert!(found.len() >= 5, "{found:?}");
    assert!(
        found
            .iter()
            .all(|finding| finding.verdict == Verdict::NotEvaluated),
        "{found:?}"
    );
    assert!(found.iter().any(|finding| matches!(
        &finding.constraint,
        policy_rules::Constraint::Extension(name) if name == "{urn:extension}Rule"
    )));
}

#[test]
fn only_a_bound_document_naming_the_signers_policy_is_enforced() {
    let octets = document_of(&rule(AUGMENTATION, None), None);
    let octets = octets.as_bytes();
    let whole = Digest::Sha256.compute(&[octets]);
    let cms = default_cms();
    let at = facts(None);
    assert!(matches!(
        enforce(&policy(octets, &[1; 32], "1.2.3.4"), &cms, &at),
        Enforcement::NotEnforced(NotEnforced::NotBound(_))
    ));
    // Signing the stated digest over canonicalised components re-computes it, and the zero
    // digest the fixture states is not what its components digest to.
    assert_eq!(
        enforce(&policy(octets, &[0; 32], "1.2.3.4"), &cms, &at),
        Enforcement::NotEnforced(NotEnforced::CanonicalDigestDiffers {
            canonicalization: "http://www.w3.org/2001/10/xml-exc-c14n#".to_owned()
        })
    );
    // A method none of the three Recommendations names is said, not enforced.
    let unnamed = String::from_utf8_lossy(octets).replace(
        "http://www.w3.org/2001/10/xml-exc-c14n#",
        "http://www.w3.org/TR/1999/REC-xpath-19991116",
    );
    assert_eq!(
        enforce(&policy(unnamed.as_bytes(), &[0; 32], "1.2.3.4"), &cms, &at),
        Enforcement::NotEnforced(NotEnforced::CanonicalDigestNotComputed {
            canonicalization: "http://www.w3.org/TR/1999/REC-xpath-19991116".to_owned()
        })
    );
    assert_eq!(
        enforce(&policy(octets, &whole, "1.2.3.9"), &cms, &at),
        Enforcement::NotEnforced(NotEnforced::IdentifierDiffers {
            document: "urn:oid:1.2.3.4".to_owned()
        })
    );
}

/// `text` with every element of clause 4.1.2's namespace written with the prefix `p`, bound to
/// the namespace as the clause's own list prints it, without the final `#`.
fn prefixed(text: &str) -> String {
    let mut out = String::with_capacity(text.len().saturating_mul(2));
    let mut rest = text;
    while let Some(at) = rest.find('<') {
        let (before, after) = rest.split_at(at.saturating_add(1));
        out.push_str(before);
        rest = after;
        if let Some(closed) = rest.strip_prefix('/') {
            out.push('/');
            rest = closed;
        }
        if !["ds:", "tsl:", "?"]
            .iter()
            .any(|skip| rest.starts_with(skip))
        {
            out.push_str("p:");
        }
    }
    out.push_str(rest);
    out.replace(
        "xmlns=\"http://uri.etsi.org/19172/v1.1.1#\"",
        "xmlns:p=\"http://uri.etsi.org/19172/v1.1.1\"",
    )
}

#[test]
fn a_prefixed_namespace_without_its_final_hash_is_read_the_same() {
    let plain = document_of(
        &rule(AUGMENTATION, Some("Validation")),
        Some("2026-06-01T00:00:00+02:00"),
    );
    let read = |text: &str| {
        let reading = Reading::parse(text.as_bytes())?;
        let document = reading.document()?;
        Ok::<_, PolicyDocumentError>((document.signing_period, document.rules.len()))
    };
    let (period, rules) = read(&plain).expect("a clause 4 document");
    assert_eq!(read(&prefixed(&plain)), Ok((period.clone(), rules)));
    let end = period.and_then(|(_, end)| end).expect("a stated end");
    assert_eq!(Some(end.instant), date_time("2026-05-31T22:00:00Z"));
    // The prefix bound to another namespace is not this syntax.
    let foreign = prefixed(&plain).replace("v1.1.1\"", "v9\"");
    assert_eq!(
        read(&foreign),
        Err(PolicyDocumentError::OtherSyntax(Syntax::Ts1191722))
    );
}

#[test]
fn dates_and_base64_are_read_as_their_schemas_write_them() {
    assert_eq!(
        date_time("2026-01-01T00:00:00.250-01:30"),
        date_time("2026-01-01T01:30:00Z")
    );
    assert_eq!(date_time("2026-01-01T00:00:00"), None);
    assert_eq!(date_time("2026-01-01T00:00:00+15:00"), None);
    assert_eq!(base64(ZERO_DIGEST), Some(vec![0; 32]));
    assert_eq!(base64("TWFu\n TWE="), Some(b"ManMa".to_vec()));
    assert_eq!(base64("TQ=="), Some(b"M".to_vec()));
    assert_eq!(base64("TQ==TWFu"), None);
    assert_eq!(base64("TWF"), None);
}

#[test]
fn malformed_xml_is_refused_by_name() {
    let refused = |text: &str| Reading::parse(text.as_bytes()).map(|_| ());
    for text in [
        r#"<!DOCTYPE p [<!ENTITY a "b">]><SignaturePolicy/>"#,
        "<SignaturePolicy xmlns=\"http://uri.etsi.org/19172/v1.1.1#\">",
        "<SignaturePolicy xmlns=\"http://uri.etsi.org/19172/v1.1.1#\"></Digest>",
        "<q:SignaturePolicy/>",
    ] {
        assert!(
            matches!(refused(text), Err(PolicyDocumentError::Xml(_))),
            "{text}"
        );
    }
    assert!(matches!(
        Reading::parse(b"<SignaturePolicy>\xFF</SignaturePolicy>").map(|_| ()),
        Err(PolicyDocumentError::Xml(_))
    ));
}

/// `depth` nested elements under the root.
fn nested(depth: usize) -> String {
    format!(
        "<SignaturePolicy xmlns=\"http://uri.etsi.org/19172/v1.1.1#\">{}{}</SignaturePolicy>",
        "<a>".repeat(depth),
        "</a>".repeat(depth)
    )
}

/// The root and `count` empty elements beside one another.
fn wide(count: usize) -> String {
    format!(
        "<SignaturePolicy xmlns=\"http://uri.etsi.org/19172/v1.1.1#\">{}</SignaturePolicy>",
        "<a/>".repeat(count)
    )
}

#[test]
fn the_depth_and_entry_budgets_hold_at_their_bounds_and_refuse_past_them() {
    let depth = usize::from(der::MAX_DEPTH);
    assert!(Reading::parse(nested(depth.saturating_sub(1)).as_bytes()).is_ok());
    assert_eq!(
        Reading::parse(nested(depth).as_bytes()).map(|_| ()),
        Err(PolicyDocumentError::NestedTooDeep)
    );
    // The root and its namespace declaration are two of the entries.
    assert!(Reading::parse(wide(MAX_ENTRIES.saturating_sub(2)).as_bytes()).is_ok());
    assert_eq!(
        Reading::parse(wide(MAX_ENTRIES.saturating_sub(1)).as_bytes()).map(|_| ()),
        Err(PolicyDocumentError::TooManyEntries)
    );
    let mut long = wide(0).into_bytes();
    long.resize(MAX_POLICY_OCTETS.saturating_add(1), b' ');
    assert_eq!(
        Reading::parse(&long).map(|_| ()),
        Err(PolicyDocumentError::TooLarge {
            octets: MAX_POLICY_OCTETS.saturating_add(1)
        })
    );
}

/// The memory probe, run with `--ignored` under `getrusage` beside the floor test: hostile
/// documents filling the byte budget with nesting, with elements and with declarations, each
/// refused a thousand times.
#[test]
#[ignore = "a probe for getrusage, not a gate"]
fn hostile_policy_documents_are_refused_within_their_budgets() {
    let deep = "<a>".repeat(MAX_POLICY_OCTETS / 3);
    let many = wide(MAX_POLICY_OCTETS.saturating_sub(100) / 4);
    let declarations = format!(
        "<SignaturePolicy {}/>",
        "xmlns:p=\"u\" ".repeat(MAX_POLICY_OCTETS.saturating_sub(30) / 13)
    );
    for _ in 0..1000 {
        assert!(Reading::parse(deep.as_bytes()).is_err());
        assert!(Reading::parse(many.as_bytes()).is_err());
        assert!(Reading::parse(declarations.as_bytes()).is_err());
    }
}

/// TS 119 172-2 clause 4.3.2's binding under the two later Recommendations, end to end: the
/// signer signed the value the document's `Digest` states, and the document is enforced only
/// where its `PolicyComponents`, canonicalised by the method its `ds:CanonicalizationMethod`
/// names, digest to it (ADR 2710).
///
/// **Constructed, and said so**: no TS 119 172-2 document naming either method is on this disk
/// — the signature corpora state no machine-readable policy at all (ADR 1709) — so the document
/// is built here and each expected form is derived by hand from the Recommendations. Every
/// rendering rule that tells the methods apart is in it: the root declares the `ds` prefix the
/// components never use, and carries `xml:lang` and an absolute `xml:base`, and the components
/// carry a relative `xml:base`. Exclusive XML Canonicalization section 3 renders only the default
/// namespace they use and copies no `xml` attribute; its section 4's `PrefixList` brings `ds` back
/// by Canonical XML 1.0's rule; Canonical XML 1.1 section 2.4 copies `xml:lang` and joins the two
/// `xml:base` values. Each stated digest is the SHA-256 of its form followed by [`BODY`].
///
/// **The plant is the matrix**: every document is also enforced under each other method's
/// stated digest, and each of those is `CanonicalDigestDiffers` — so a canonicaliser that
/// ignored the method, or the parameter, would bind a document to the wrong commitment and fail
/// here.
#[test]
fn a_document_bound_through_exclusive_or_1_1_canonicalisation_is_enforced() {
    const BODY: &str = concat!(
        "<GeneralDetails><SigPolicyDetails><PolicyIdentifier>urn:oid:1.2.3.4</PolicyIdentifier>",
        "</SigPolicyDetails></GeneralDetails><PolicyRules><PolicyRule><AugmentationRules>",
        "<PreviousValidationRequired>true</PreviousValidationRequired><LevelId>urn:level</LevelId>",
        "</AugmentationRules></PolicyRule></PolicyRules></PolicyComponents>",
    );
    const P: &str = "http://uri.etsi.org/19172/v1.1.1#";
    const DS: &str = "http://www.w3.org/2000/09/xmldsig#";
    let exclusive = format!(r#"<PolicyComponents xmlns="{P}" Id="c" xml:base="v1/">"#);
    let listed = format!(r#"<PolicyComponents xmlns="{P}" xmlns:ds="{DS}" Id="c" xml:base="v1/">"#);
    let version_11 = format!(r#"<PolicyComponents xmlns="{P}" xmlns:ds="{DS}" Id="c" "#)
        + r#"xml:base="http://example.org/policies/v1/" xml:lang="en">"#;
    let methods = [
        (
            "http://www.w3.org/2001/10/xml-exc-c14n#",
            "",
            exclusive,
            "Ispncf+UbqbIvsk+tl3wLLyb9pxTuIQA47WgdMmG+rI=",
        ),
        (
            "http://www.w3.org/2001/10/xml-exc-c14n#",
            r#"<ec:InclusiveNamespaces xmlns:ec="http://www.w3.org/2001/10/xml-exc-c14n#" PrefixList="ds"/>"#,
            listed,
            "BDKidfatq1hOQR1ix++JVyFDe5Py4KQb+oKyOOL3u9M=",
        ),
        (
            "http://www.w3.org/2006/12/xml-c14n11",
            "",
            version_11,
            "x6tn5/VSijScXmKeZl0FE80IALGOkhnQeoNQ/zWNppU=",
        ),
    ];
    let document = |method: &str, parameter: &str, stated: &str| {
        format!(
            r#"<SignaturePolicy xmlns="{P}" xmlns:ds="{DS}" xml:lang="en" xml:base="http://example.org/policies/"><Digest><ds:DigestMethod Algorithm="http://www.w3.org/2001/04/xmlenc#sha256"/><ds:DigestValue>{stated}</ds:DigestValue><ds:CanonicalizationMethod Algorithm="{method}">{parameter}</ds:CanonicalizationMethod></Digest>
<PolicyComponents Id='c'   xml:base="v1/">{BODY}</SignaturePolicy>"#
        )
    };
    let cms = default_cms();
    let at = facts(None);
    for (method, parameter, form, stated) in &methods {
        let signed = Digest::Sha256.compute(&[form.as_bytes(), BODY.as_bytes()]);
        assert_eq!(base64(stated).as_deref(), Some(signed.as_slice()), "{form}");
        let octets = document(method, parameter, stated);
        let found = enforce(&policy(octets.as_bytes(), &signed, "1.2.3.4"), &cms, &at);
        assert!(
            matches!(found, Enforcement::Enforced { .. }),
            "{method} {parameter}: {found:?}"
        );
        for (other, other_parameter, other_form, other_stated) in &methods {
            if other_form == form {
                continue;
            }
            let signed = Digest::Sha256.compute(&[other_form.as_bytes(), BODY.as_bytes()]);
            let octets = document(method, parameter, other_stated);
            assert_eq!(
                enforce(&policy(octets.as_bytes(), &signed, "1.2.3.4"), &cms, &at),
                Enforcement::NotEnforced(NotEnforced::CanonicalDigestDiffers {
                    canonicalization: (*method).to_owned()
                }),
                "{method} {parameter} under {other} {other_parameter}'s digest"
            );
        }
    }
}
