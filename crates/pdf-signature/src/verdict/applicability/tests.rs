//! ETSI TS 119 172-4 REQ-4.5-01's report items and REQ-4.2-03 d) and e), a test per item.
//!
//! Every expected value is the clause's item read against the fixture's own construction: the
//! verdict fixture's signature (`openssl cms -sign`, this tree's), the revocation fixtures' CA, CRL
//! and OCSP response (this tree's), and a certificate generated for this module with `openssl req
//! -x509 -subj "/C=AT/pseudonym=Fuchs42/CN=Fuchs42 (pseudonym)" -addext
//! "subjectAltName=email:fuchs42@example.org,URI:https://example.org/fuchs42"` on a P-256 key this
//! tree made. Instants are the fixtures' stated times as seconds since the epoch.

use super::ts_119_312::{Class, Issue, Rule};
use super::*;
use crate::revocation::tests::fixtures as revocation_fixtures;
use crate::signature::Signed;
use crate::verdict::tests::{AT, fixtures, opened};
use crate::x509_fixtures::hex;

/// The pseudonymous signer's certificate.
const PSEUDONYMOUS: &str = "\
        308201fa308201a1a003020102020151300a06082a8648ce3d040302303d310b\
        30090603550406130241543110300e06035504410c0746756368733432311c30\
        1a06035504030c1346756368733432202870736575646f6e796d29301e170d32\
        36313131353233333734395a170d3336313131323233333734395a303d310b30\
        090603550406130241543110300e06035504410c0746756368733432311c301a\
        06035504030c1346756368733432202870736575646f6e796d29305930130607\
        2a8648ce3d020106082a8648ce3d03010703420004cf2257128e27d788b3d33b\
        35916cf2ebf22da83fa5ffc172f74d1d9bc1867745322e44b8d94c6ddab5b17e\
        3f8a2f3e942f89361151c7620f1529bd0e02c57d25a3819130818e301d060355\
        1d0e041604149fc413575b6a4660e26196dc145db2247e3b84ef301f0603551d\
        230418301680149fc413575b6a4660e26196dc145db2247e3b84ef300f060355\
        1d130101ff040530030101ff303b0603551d1104343032811366756368733432\
        406578616d706c652e6f7267861b68747470733a2f2f6578616d706c652e6f72\
        672f66756368733432300a06082a8648ce3d04030203470030440220261c9f80\
        ee545ee722645f3e512a9a5e5813fa3a12942e40b8cd614087e6c27002200d73\
        a6d0cc5515b61e7cc4b4475c77797d4ec39c781c51f49d74463e628b8c4e";

/// b): every subject attribute in the certificate's order, and every `subjectAltName` name.
/// c): the `pseudonym` attribute's value is the indication.
#[test]
fn the_signer_is_its_subject_and_alternative_names_and_a_pseudonym_is_indicated() {
    let der = hex(PSEUDONYMOUS);
    let certificate = x509::parse(&der).expect("the fixture parses");
    let signer = signer(&certificate).expect("its names read");
    let subject: Vec<_> = signer
        .subject
        .iter()
        .map(|attribute| (attribute.kind.as_str(), attribute.text.as_deref()))
        .collect();
    assert_eq!(
        subject,
        [
            ("2.5.4.6", Some("AT")),
            ("2.5.4.65", Some("Fuchs42")),
            ("2.5.4.3", Some("Fuchs42 (pseudonym)")),
        ]
    );
    assert_eq!(
        signer.alternative_names,
        [
            Name::Rfc822("fuchs42@example.org".to_owned()),
            Name::Uri("https://example.org/fuchs42".to_owned()),
        ]
    );
    assert_eq!(signer.pseudonym.as_deref(), Some("Fuchs42"));
}

/// c) with the pseudonym absent: the verdict fixture's signer states only a common name, and no
/// alternative name.
#[test]
fn a_signer_without_a_pseudonym_indicates_none() {
    let (_, signature) = opened(fixtures::signed());
    let der = signature
        .signer_certificate()
        .expect("the fixture carries its signer");
    let signer = signer(&x509::parse(&der).expect("it parses")).expect("its names read");
    assert_eq!(signer.pseudonym, None);
    assert!(signer.alternative_names.is_empty());
    assert!(
        signer
            .subject
            .iter()
            .any(|attribute| attribute.kind == "2.5.4.3"),
        "{:?}",
        signer.subject
    );
}

/// f): the fixture's four signed attributes in the file's order, named where this tree knows the
/// identifier, and no unsigned one — `openssl cms -sign` writes `contentType`, `signingTime`,
/// `messageDigest` and `smimeCapabilities` (RFC 8551 section 2.5.2).
#[test]
fn every_attribute_is_listed_with_whether_it_was_signed() {
    let (_, signature) = opened(fixtures::signed());
    let cms = signature.signed_data().expect("the fixture reads");
    let listed: Vec<_> = attributes(&cms)
        .into_iter()
        .map(|attribute| (attribute.identifier, attribute.name, attribute.signed))
        .collect();
    assert_eq!(
        listed,
        [
            (
                "1.2.840.113549.1.9.3".to_owned(),
                Some("content-type (RFC 5652)"),
                true
            ),
            (
                "1.2.840.113549.1.9.5".to_owned(),
                Some("signing-time (RFC 5652)"),
                true
            ),
            (
                "1.2.840.113549.1.9.4".to_owned(),
                Some("message-digest (RFC 5652)"),
                true
            ),
            (
                "1.2.840.113549.1.9.15".to_owned(),
                Some("smime-capabilities (RFC 8551)"),
                true
            ),
        ]
    );
}

/// f)'s NOTE 5: a `commitment-type-indication` names its commitment — `id-cti-ets-proofOfOrigin`,
/// `1.2.840.113549.1.9.16.6.1`, the first of ETSI EN 319 122-1 clause 5.2.3's six — and one
/// outside the six is named by its identifier alone.
#[test]
fn a_commitment_is_named_by_its_identifier() {
    let origin = [
        0x30, 0x0D, 0x06, 0x0B, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x10, 0x06, 0x01,
    ];
    let value = crate::der::Reader::new(&origin)
        .expect("a reader")
        .next_value()
        .expect("reads")
        .expect("a value");
    assert_eq!(
        commitment(&value),
        Some((
            "1.2.840.113549.1.9.16.6.1".to_owned(),
            Some("proof of origin")
        ))
    );
    let other = [0x30, 0x05, 0x06, 0x03, 0x2A, 0x03, 0x04];
    let value = crate::der::Reader::new(&other)
        .expect("a reader")
        .next_value()
        .expect("reads")
        .expect("a value");
    assert_eq!(commitment(&value), Some(("1.2.3.4".to_owned(), None)));
}

/// h) and REQ-4.2-03 d): a computed signature's suite is its digest, algorithm and key size.
/// REQ-4.2-03 e): each kind of unsupported suite is
/// `NotSupported` naming what the file named; an answer that never reached a suite says so.
#[test]
fn the_suite_is_reported_and_an_unsupported_one_is_named() {
    let verified = Authenticity::Verified {
        digest: Digest::Sha256,
        family: Family::Rsa,
        key_bits: 2048,
        over: Signed::TheDocumentsBytes,
    };
    assert_eq!(
        Cryptography::of(&verified),
        Cryptography::Suite {
            digest: Digest::Sha256,
            family: Family::Rsa,
            key_bits: 2048,
        }
    );
    for (authenticity, suite) in [
        (
            Authenticity::AlgorithmNotVerifiable {
                algorithm: "1.2.3".to_owned(),
            },
            "1.2.3",
        ),
        (
            Authenticity::KeyNotVerifiable {
                algorithm: "1.3.101.110".to_owned(),
            },
            "1.3.101.110",
        ),
        (
            Authenticity::CurveNotVerifiable {
                curve: "1.3.132.0.10".to_owned(),
            },
            "1.3.132.0.10",
        ),
        (
            Authenticity::UnknownDigest {
                algorithm: "1.2.840.113549.2.5".to_owned(),
            },
            "1.2.840.113549.2.5",
        ),
    ] {
        assert_eq!(
            Cryptography::of(&authenticity),
            Cryptography::NotSupported {
                suite: suite.to_owned()
            }
        );
    }
    assert_eq!(
        Cryptography::of(&Authenticity::NoSignerCertificate { certificates: 0 }),
        Cryptography::NotReached
    );
}

/// d) ii) 4) and 5) from a CRL: the revocation fixtures' CRL by their CA, `thisUpdate` 2026-06-01,
/// `nextUpdate` 2036-06-01, listing serial `2A` revoked 2026-09-14T20:09:02Z for `keyCompromise`;
/// and from the OCSP response about the same serial, `producedAt` 2026-09-14T20:09:14Z,
/// `nextUpdate` 2036-09-11T20:09:14Z, revoked at the same instant. Each is the material's
/// statement, by its kind.
#[test]
fn revocation_times_are_what_the_material_states_about_the_signers_certificate() {
    let leaf = hex(revocation_fixtures::REVOKED_LEAF);
    let leaf = x509::parse(&leaf).expect("the leaf parses");
    let crl = hex(revocation_fixtures::CRL);
    let ocsp = hex(revocation_fixtures::OCSP_REVOKED_BY_CA);
    let material = Material::read(&[&crl], &[&ocsp]);
    assert!(material.refused.is_empty(), "{:?}", material.refused);
    let mut times = Vec::new();
    revocation_times(&leaf, &material, &mut times);
    let at = Instant::from_unix_seconds;
    let list = Assurance::StatedBy(Evidence::CertificateRevocationList);
    let response = Assurance::StatedBy(Evidence::OcspResponse);
    assert_eq!(
        times,
        [
            Timing {
                kind: TimeKind::ListIssued {
                    next_update: Some(at(2_095_891_200)),
                },
                at: at(1_780_272_000),
                assurance: list.clone(),
            },
            Timing {
                kind: TimeKind::Revocation { suspended: false },
                at: at(1_789_416_542),
                assurance: list,
            },
            Timing {
                kind: TimeKind::ResponseProduced {
                    next_update: Some(at(2_104_776_554)),
                },
                at: at(1_789_416_554),
                assurance: response.clone(),
            },
            Timing {
                kind: TimeKind::Revocation { suspended: false },
                at: at(1_789_416_542),
                assurance: response,
            },
        ]
    );
}

/// d) ii) 4) and 5) need the material to be about the signer: a certificate by another issuer
/// with another serial gets no time from the same material.
#[test]
fn material_about_another_certificate_states_no_time() {
    let der = hex(PSEUDONYMOUS);
    let other = x509::parse(&der).expect("it parses");
    let crl = hex(revocation_fixtures::CRL);
    let ocsp = hex(revocation_fixtures::OCSP_REVOKED_BY_CA);
    let material = Material::read(&[&crl], &[&ocsp]);
    let mut times = Vec::new();
    revocation_times(&other, &material, &mut times);
    assert!(times.is_empty(), "{times:?}");
}

/// The whole report over the verdict fixture, at [`AT`] on the reader's clock: a) the scope, d)
/// ii) 1) the `signingTime` attribute's 2026-09-14T23:33:43Z as a claim, no document or signature
/// timestamp and no revocation material, and 6) the best signature time last, fixed by nothing but
/// the clock; h) SHA-256 under RSA PKCS #1 v1.5 with a 2048-bit key.
#[test]
fn the_report_over_a_signature_holds_each_item_in_the_clauses_order() {
    let (document, signature) = opened(fixtures::signed());
    let authenticity = signature.authenticity(document.bytes());
    let anchors = TrustAnchors::of(&[]);
    let report = Report::of(&Asked {
        signature: &signature,
        authenticity: &authenticity,
        when: BestSignatureTime::now(AT),
        chain: &Chain::default(),
        index: 0,
        material: &Material::none(),
        anchors: &anchors,
        now: AT,
        signatures: &[],
        lists: &[],
        path: &[],
    });
    assert_eq!(report.scope, Scope::EuQualifiedUsingTrustedLists);
    assert_eq!(
        report.times,
        [
            Timing {
                kind: TimeKind::ClaimedSigningTime(ClaimedIn::SigningTimeAttribute),
                at: Instant::from_unix_seconds(1_789_428_823),
                assurance: Assurance::Claimed,
            },
            Timing {
                kind: TimeKind::BestSignatureTime,
                at: AT,
                assurance: Assurance::Best(Proof::NothingButTheReadersClock),
            },
        ]
    );
    assert_eq!(
        report.cryptography,
        Cryptography::Suite {
            digest: Digest::Sha256,
            family: Family::Rsa,
            key_bits: 2048,
        }
    );
    assert!(report.signer.is_ok());
    assert_eq!(report.attributes.len(), 4);
    // h) against ETSI TS 119 312: SHA-256 is recommended (its Table 1), PKCS #1 v1.5 is a bare
    // legacy `L` (its Table 2) and so is `sha256-with-rsa` (its Table 4), and a 2048-bit modulus
    // is `L[2026]` (its Table 6) — not yet phased out at 2026-10-01, the best signature time and
    // the instant of validation both. The signer's certificate is valid until 2036-09-11, past
    // both phase-outs of its key — Table 2's at the end of 2034 and Table 6's at the end of 2026 —
    // so clause 9.3 raises each against that `notAfter`; Table 4's row is the signature's, not the
    // key's, and is not asked again (ADR 2927).
    let not_after = Instant::from_unix_seconds(2_104_788_759);
    let security = report.security.expect("a suite was reached");
    assert_eq!(
        security.issues,
        [
            Issue::Legacy {
                rule: Rule::SignatureAlgorithms,
                entry: "RSA-PKCS#1v1_5",
                class: Class::LEGACY,
            },
            Issue::Legacy {
                rule: Rule::SignatureSuites,
                entry: "sha256-with-rsa",
                class: Class::LEGACY,
            },
            Issue::Legacy {
                rule: Rule::RsaKeySizes,
                entry: "at least 1 900 and below 3 000 bits",
                class: Class::legacy_until(2026),
            },
            Issue::KeyOutlived {
                rule: Rule::SignatureAlgorithms,
                entry: "RSA-PKCS#1v1_5",
                class: Class::LEGACY,
                not_after,
            },
            Issue::KeyOutlived {
                rule: Rule::RsaKeySizes,
                entry: "at least 1 900 and below 3 000 bits",
                class: Class::legacy_until(2026),
                not_after,
            },
        ]
    );
}

/// TS 119 312 clause 9.5 over the path handed in, the signer's first (ADR 2928). The authority
/// at position 1 has a 3072-bit PKCS #1 v1.5 key on a certificate valid to 2030, and the signer's
/// certificate relying on it is valid to 2036-09-11: Table 2's bare `L` ends with 2034, inside the
/// period that runs to the later `notAfter`, and Table 6's row is recommended; its modulus is not
/// smaller than the signer's 2048 bits. The authority at position 2 has a P-256 key, whose rows are
/// recommended, and an RSA key under an elliptic-curve key is a pair the document gives no measure
/// across.
#[test]
fn the_authorities_keys_are_asked_by_clause_9_5_in_the_report() {
    use super::ts_119_312::KeyRole;
    use crate::trust::{CertifiedKey, KeyParameters};
    let (document, signature) = opened(fixtures::signed());
    let authenticity = signature.authenticity(document.bytes());
    let anchors = TrustAnchors::of(&[]);
    let validity = |not_after| {
        Some(x509::Validity {
            not_before: Instant::from_unix_seconds(1_767_225_600),
            not_after: Instant::from_unix_seconds(not_after),
        })
    };
    let signer_not_after = Instant::from_unix_seconds(2_104_788_759);
    let path = [
        CertifiedKey {
            key: KeyParameters::Rsa {
                modulus_bits: 2_048,
                pss: None,
            },
            validity: validity(signer_not_after.unix_seconds()),
        },
        CertifiedKey {
            key: KeyParameters::Rsa {
                modulus_bits: 3_072,
                pss: Some(false),
            },
            // 2030-01-01T00:00:00Z.
            validity: validity(1_893_456_000),
        },
        CertifiedKey {
            key: KeyParameters::Ecdsa(crate::ecdsa::Curve::P256),
            // 2040-01-01T00:00:00Z.
            validity: validity(2_208_988_800),
        },
    ];
    let report = Report::of(&Asked {
        signature: &signature,
        authenticity: &authenticity,
        when: BestSignatureTime::now(AT),
        chain: &Chain::default(),
        index: 0,
        material: &Material::none(),
        anchors: &anchors,
        now: AT,
        signatures: &[],
        lists: &[],
        path: &path,
    });
    let security = report.security.expect("a suite was reached");
    let clause_9_5: Vec<&Issue> = security
        .issues
        .iter()
        .filter(|issue| {
            matches!(
                issue,
                Issue::OtherKeyOutlived { .. }
                    | Issue::OtherKeyNotListed { .. }
                    | Issue::OtherKeyValidityNotRead { .. }
                    | Issue::WeakerThanCertified { .. }
                    | Issue::NotCompared { .. }
            )
        })
        .collect();
    assert_eq!(
        clause_9_5,
        [
            &Issue::OtherKeyOutlived {
                key: KeyRole::CertificationAuthority { position: 1 },
                rule: Rule::SignatureAlgorithms,
                entry: "RSA-PKCS#1v1_5",
                class: Class::LEGACY,
                until: signer_not_after,
            },
            &Issue::NotCompared {
                certifier: KeyRole::CertificationAuthority { position: 2 },
                certified: KeyRole::CertificationAuthority { position: 1 },
            },
        ]
    );
}

/// a): the scope text REQ-4.5-01 a) prints, carried as printed (`doc/questions/A855`). Each
/// expected string is ETSI TS 119 172-4 V1.1.1 clause 4.5's item a), its heading and its sentence,
/// with the item's closing semicolon left to the list it belongs to.
#[test]
fn the_scope_is_the_clauses_heading_and_sentence() {
    let scope = Scope::EuQualifiedUsingTrustedLists;
    assert_eq!(
        scope.heading(),
        "Signature applicability rules checking (validation rules) for European qualified \
         electronic signatures/seals using trusted lists"
    );
    assert_eq!(
        scope.text(),
        "Validation of digital signature to identify whether it can be considered technically \
         suitable to implement a European qualified electronic signature/seal using EUMS trusted \
         lists in the sense of the applicable European legislation at the time of signing, i.e. \
         either Directive 1999/93/EC or Regulation (EU) No 910/2014"
    );
}

/// d) iii) 3) and d) i) for a token, end to end over the `openssl ts` token of
/// `trusted_list::time_stamp`'s tests, carried as a signature timestamp attribute.
///
/// With Austria's list publishing the unit's root as a granted `TSA/QTST` service of the unit's
/// provider, the token is established through that service, ETSI TS 119 615 clause 4.7 answers
/// `Qualified` with `PROCESS_PASSED` by the list of `AT` (PRO-4.7.4-03, the two runs agreeing),
/// and d) i) names the service. With no list nothing is asked; with a list publishing the root only
/// as a `CA/QC` service the token is not established, so clause 4.7 is not asked (its clause
/// 4.7.1's proviso), and d) i) expresses the absence.
#[test]
fn a_tokens_qualification_and_listing_are_reported_from_the_lists() {
    use crate::trusted_list::CA_QC;
    use crate::trusted_list::tests::Written;
    use crate::trusted_list::time_stamp::tests::{ASKED, PROVIDER, ROOT, TOKEN, list, qtst};
    use crate::trusted_list::time_stamp::{Answer, Indication, Outputs, TSA_QTST};
    let root = hex(ROOT);
    let bytes = cms::fixtures::detached_with_signature_timestamp(&[0x00; 32], &hex(TOKEN));
    let outer = cms::signed_data(&bytes).expect("a SignedData");
    let (token_cms, info) = timestamp::signature_timestamp_token(&outer)
        .expect("the attribute is there")
        .expect("the token reads");
    let claim = timestamp::Claim::from(&info);
    let assured = |lists: &[TrustedList]| {
        let time = timestamp::signature_timestamp_established(
            &outer,
            &TrustAnchors::none(),
            lists,
            &Material::none(),
            AskedAt::TheCallersInstant(ASKED),
        )
        .expect("the attribute is there");
        token(&claim, &time, Some(&token_cms), lists)
    };

    let lists = [list("AT", PROVIDER, &[qtst(&root)])];
    let Assurance::Token {
        established,
        eu_qualified,
        listed,
        ..
    } = assured(&lists)
    else {
        panic!("a token's assurance");
    };
    assert!(established);
    assert_eq!(
        eu_qualified,
        EuQualified::Determined(Answer::Determined(Outputs {
            results: Indication::Qualified,
            status: crate::trusted_list::Status::Passed,
            sub_status: Vec::new(),
            country: Some("AT".to_owned()),
            tsp_name: None,
        }))
    );
    let [service] = listed.as_slice() else {
        panic!("one listed service: {listed:?}");
    };
    assert_eq!(service.service_type, TSA_QTST);

    let Assurance::Token {
        eu_qualified,
        listed,
        ..
    } = assured(&[])
    else {
        panic!("a token's assurance");
    };
    assert_eq!(eu_qualified, EuQualified::NoListSupplied);
    assert!(listed.is_empty());

    let issuer_only = Written {
        service_type: CA_QC,
        ..qtst(&root)
    };
    let lists = [list("AT", PROVIDER, &[issuer_only])];
    let Assurance::Token {
        established,
        eu_qualified,
        listed,
        ..
    } = assured(&lists)
    else {
        panic!("a token's assurance");
    };
    assert!(!established);
    assert_eq!(eu_qualified, EuQualified::TokenNotEstablished);
    assert!(listed.is_empty(), "the absence d) i) asks to be expressed");
}

/// h) against ETSI TS 119 312 clause 9.2: "A hash function used to compute the imprint of a
/// message placed in a time-stamp token should never be a legacy mechanism at the time of time
/// stamp creation." The report asks Table 1 about every token it takes as a time, after the suite's
/// own components. The fixture is the timestamp module's two-revision document, whose second
/// document timestamp covers the first: asked about the first, d) ii) 2) takes the second's token
/// as a time, and its `messageImprint` is SHA-256, which Table 1 lists as recommended. The suite is
/// set by hand, because what is pinned here is the token's component and not the suite's.
#[test]
fn every_tokens_imprint_hash_is_asked_of_table_1_in_the_report() {
    use super::ts_119_312::Component;
    use crate::timestamp::chain;
    use crate::timestamp::tests::fixtures::two_timestamps;
    let document = pdf_syntax::Document::open(two_timestamps(false)).expect("the fixture opens");
    let found = crate::signature::signatures(&document);
    let anchors = TrustAnchors::none();
    let chain = chain(&document, &anchors, &[], &Material::none(), AT);
    let authenticity = Authenticity::Verified {
        digest: Digest::Sha256,
        family: Family::Rsa,
        key_bits: 3_072,
        over: Signed::TheDocumentsBytes,
    };
    let report = Report::of(&Asked {
        signature: &found[0],
        authenticity: &authenticity,
        when: BestSignatureTime::now(AT),
        chain: &chain,
        index: 0,
        material: &Material::none(),
        anchors: &anchors,
        now: AT,
        signatures: &found,
        lists: &[],
        path: &[],
    });
    assert_eq!(
        report
            .times
            .iter()
            .filter(|timing| timing.kind == TimeKind::DocumentTimestamp)
            .count(),
        1,
        "the later timestamp is the one time stamp over the first: {:?}",
        report.times
    );
    let security = report.security.expect("a suite was reached");
    let imprints: Vec<&Component> = security
        .components
        .iter()
        .filter(|component| component.rule == Rule::TimeStampImprints)
        .collect();
    assert_eq!(
        imprints,
        [&Component {
            rule: Rule::TimeStampImprints,
            listed: Some(("SHA-256", Class::Recommended)),
        }]
    );
    assert_eq!(
        security.components.last().map(|component| component.rule),
        Some(Rule::TimeStampImprints),
        "the token's component follows the suite's"
    );
}

/// TS 119 312 clause 9.5's time-stamping unit: the key of the certificate a token's `SignerInfo`
/// names among the ones the token carries, with that certificate's validity. The fixture token's
/// unit, which `openssl ts` signed with PKCS #1 v1.5, has a 2048-bit key on a certificate valid
/// from 2026-01-01 to 2036-01-01: Table 2's bare `L` ends with 2034 and Table 6's `L[2026]` with
/// 2026, both inside that period. A token carrying no certificate gives no key, rather than
/// another certificate's.
#[test]
fn a_tokens_unit_is_the_certificate_its_signer_info_names() {
    use super::ts_119_312::{Assessment, KeyRole, OtherKey};
    use crate::trust::{CertifiedKey, KeyParameters};
    let token = hex(timestamp::tests::fixtures::TOKEN);
    let cms = cms::signed_data(&token).expect("the token reads");
    let not_after = Instant::from_unix_seconds(2_082_758_400);
    let carried = unit(&cms).expect("the unit's key is carried");
    assert_eq!(
        carried,
        CertifiedKey {
            key: KeyParameters::Rsa {
                modulus_bits: 2_048,
                pss: Some(false),
            },
            validity: Some(x509::Validity {
                not_before: Instant::from_unix_seconds(1_767_225_600),
                not_after,
            }),
        }
    );
    let mut assessment = Assessment {
        components: Vec::new(),
        issues: Vec::new(),
    };
    assessment.other_keys(&[OtherKey {
        role: KeyRole::TimeStampingUnit,
        key: carried.key,
        until: Some(not_after),
    }]);
    assert_eq!(
        assessment.issues,
        [
            Issue::OtherKeyOutlived {
                key: KeyRole::TimeStampingUnit,
                rule: Rule::SignatureAlgorithms,
                entry: "RSA-PKCS#1v1_5",
                class: Class::LEGACY,
                until: not_after,
            },
            Issue::OtherKeyOutlived {
                key: KeyRole::TimeStampingUnit,
                rule: Rule::RsaKeySizes,
                entry: "at least 1 900 and below 3 000 bits",
                class: Class::legacy_until(2026),
                until: not_after,
            },
        ]
    );
    let mut bare = cms.clone();
    bare.certificates.clear();
    assert_eq!(unit(&bare), None);
}

/// TS 119 312 clause 9.5's time-stamping unit, asked in the report itself: the two-revision
/// document whose tokens carry and name [`timestamp::tests::fixtures::TOKEN`]'s unit, asked about
/// the first timestamp, takes the second's token as a time (d) ii) 2)), and `Report::of` asks that
/// token's unit — 2048-bit RSA under PKCS #1 v1.5, its certificate valid to 2036-01-01 — against
/// Tables 2 and 6, as [`a_tokens_unit_is_the_certificate_its_signer_info_names`] does by hand. The
/// same document whose tokens name no certificate raises neither.
#[test]
fn the_report_asks_the_unit_of_each_token_it_takes_as_a_time() {
    use super::ts_119_312::KeyRole;
    use crate::timestamp::chain;
    use crate::timestamp::tests::fixtures::{two_timestamps, two_timestamps_by_a_unit};
    let unit_issues = |bytes: Vec<u8>| {
        let document = pdf_syntax::Document::open(bytes).expect("the fixture opens");
        let found = crate::signature::signatures(&document);
        let anchors = TrustAnchors::none();
        let chain = chain(&document, &anchors, &[], &Material::none(), AT);
        let authenticity = Authenticity::Verified {
            digest: Digest::Sha256,
            family: Family::Rsa,
            key_bits: 3_072,
            over: Signed::TheDocumentsBytes,
        };
        let report = Report::of(&Asked {
            signature: &found[0],
            authenticity: &authenticity,
            when: BestSignatureTime::now(AT),
            chain: &chain,
            index: 0,
            material: &Material::none(),
            anchors: &anchors,
            now: AT,
            signatures: &found,
            lists: &[],
            path: &[],
        });
        report
            .security
            .expect("a suite was reached")
            .issues
            .into_iter()
            .filter(|issue| {
                matches!(
                    issue,
                    Issue::OtherKeyOutlived {
                        key: KeyRole::TimeStampingUnit,
                        ..
                    }
                )
            })
            .collect::<Vec<_>>()
    };
    let not_after = Instant::from_unix_seconds(2_082_758_400);
    assert_eq!(
        unit_issues(two_timestamps_by_a_unit()),
        [
            Issue::OtherKeyOutlived {
                key: KeyRole::TimeStampingUnit,
                rule: Rule::SignatureAlgorithms,
                entry: "RSA-PKCS#1v1_5",
                class: Class::LEGACY,
                until: not_after,
            },
            Issue::OtherKeyOutlived {
                key: KeyRole::TimeStampingUnit,
                rule: Rule::RsaKeySizes,
                entry: "at least 1 900 and below 3 000 bits",
                class: Class::legacy_until(2026),
                until: not_after,
            },
        ]
    );
    assert_eq!(unit_issues(two_timestamps(false)), []);
}

/// TS 119 312 clause 9.5's OCSP responder: the delegate a response about the signer's
/// certificate names and carries (RFC 6960 section 4.2.2.2 criterion 3), certified by the
/// signer's issuer. The fixture's responder has a 3072-bit key under a CA whose key is 2048 bits,
/// so the CA's modulus is smaller than the key it certifies — clause 9.5's *shall*. A response
/// about another certificate names no responder for this one.
#[test]
fn a_delegated_responder_is_asked_and_compared_with_its_issuer() {
    use super::ts_119_312::{Assessment, KeyRole, Parameter};
    use crate::trust::{CertifiedKey, KeyParameters};
    let ca = hex(revocation_fixtures::CA);
    let leaf = hex(revocation_fixtures::GOOD_LEAF);
    let (ca, leaf) = (
        x509::parse(&ca).expect("the CA parses"),
        x509::parse(&leaf).expect("the leaf parses"),
    );
    let response = hex(revocation_fixtures::OCSP_GOOD_BY_DELEGATE);
    let material = Material::read(&[], &[&response]);
    let responders = ocsp_responders(&leaf, &material);
    let [responder] = responders.as_slice() else {
        panic!("one delegate: {responders:?}");
    };
    assert_eq!(
        responder.key,
        KeyParameters::Rsa {
            modulus_bits: 3_072,
            pss: Some(false),
        }
    );
    assert!(responder.validity.is_some());
    let path = [
        CertifiedKey::of(&leaf, None),
        CertifiedKey::of(&ca, Some(leaf.signature_algorithm)),
    ];
    let (_, pairs) = other_keys(&path, &responders, &[]);
    let mut assessment = Assessment {
        components: Vec::new(),
        issues: Vec::new(),
    };
    assessment.certifications(&pairs);
    let ca_role = KeyRole::CertificationAuthority { position: 1 };
    assert_eq!(
        assessment.issues,
        [Issue::WeakerThanCertified {
            certifier: ca_role,
            certified: KeyRole::OcspResponder,
            parameter: Parameter::Modulus,
            certifier_bits: 2_048,
            certified_bits: 3_072,
        }]
    );
    let other = hex(PSEUDONYMOUS);
    let other = x509::parse(&other).expect("it parses");
    assert_eq!(ocsp_responders(&other, &material), []);
}
