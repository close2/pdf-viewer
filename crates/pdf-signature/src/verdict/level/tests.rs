//! ETSI TS 119 172-4 clause 4.4.2's process, a test per branch of each requirement.
//!
//! The inputs are clause 4.4's and clause 4.5's outputs (ETSI TS 119 615 V1.3.1 clauses 4.4.3 and
//! 4.5.3) written out, and clause 4.2's result as a flag; each expected value is the branch the
//! test names, read off TS 119 172-4 V1.1.1's text, and Annex A's identifiers off its item 2) b).
//! [`determine`] over a real signature is pinned by the last two tests, with this crate's own
//! fixture signature (`crate::verdict::tests::fixtures`).

use super::*;
use crate::trusted_list::determination::qscd::Qscd;
use crate::trusted_list::determination::{Determination, Indication};

/// Clause 4.4's outputs with `status` and `results`, and clause 4.5's with `device_status` and
/// `device`.
fn determined(
    status: QcStatus,
    results: &[Indication],
    device_status: QcStatus,
    device: Option<QscdIndication>,
) -> Qualification {
    Qualification::Determined {
        territory: "AT".to_owned(),
        sequence_number: 1,
        determination: Determination {
            status,
            sub_status: Vec::new(),
            results: results.to_vec(),
            sets_of_qe: [Vec::new(), Vec::new(), Vec::new()],
        },
        qscd: Qscd {
            status: device_status,
            sub_status: Vec::new(),
            result: device,
        },
    }
}

/// A certificate clause 4.4 passed with `results`, whose key clause 4.5 found `QSCD_YES`.
fn on_a_device(results: &[Indication]) -> Qualification {
    determined(
        QcStatus::Passed,
        results,
        QcStatus::Passed,
        Some(QscdIndication::Yes),
    )
}

/// REQ-4.4.2-06, first branch, for signatures: `QC_For_eSig` passed (-02 a)), `QSCD_YES` passed
/// (-05 a)), and clause 4.2 `TOTAL-PASSED` — an EU qualified electronic signature, Annex A i).
#[test]
fn a_qualified_certificate_on_a_device_with_a_passed_validation_is_a_qualified_signature() {
    let level = join(&on_a_device(&[Indication::QcForESig]), true);
    assert_eq!(
        level,
        Level {
            answer: Answer::Qualified(Purpose::Signature),
            decided_by: Requirement::Join,
            certificate: Some(Purpose::Signature),
            total_passed: true,
        }
    );
    assert_eq!(level.annex_a(), Some(Type::QualifiedSignature));
    assert_eq!(
        Type::QualifiedSignature.identifier(),
        Some("0.4.0.191724.1.2.1"),
        "Annex A: id-etsi-dst-euqesig is id-etsi-sars-SigType 1, id-etsi-sars-SigType is \
         id-etsi-sars 2, and id-etsi-sars is {{ 0 4 0 191724 1 }}"
    );
}

/// REQ-4.4.2-06, first branch, "respectively" for seals: `QC_For_eSeal` — Annex A iv).
#[test]
fn a_qualified_seal_certificate_on_a_device_with_a_passed_validation_is_a_qualified_seal() {
    let level = join(&on_a_device(&[Indication::QcForESeal]), true);
    assert_eq!(level.answer, Answer::Qualified(Purpose::Seal));
    assert_eq!(level.certificate, Some(Purpose::Seal));
    assert_eq!(level.annex_a(), Some(Type::QualifiedSeal));
    assert_eq!(Type::QualifiedSeal.identifier(), Some("0.4.0.191724.1.2.4"));
}

/// REQ-4.4.2-06, second branch: certificate and device determined, clause 4.2 not
/// `TOTAL-PASSED` — not determined as qualified, and NOTE 5 supports no Annex A type.
#[test]
fn without_a_passed_validation_the_join_does_not_determine_a_qualified_signature() {
    let level = join(&on_a_device(&[Indication::QcForESig]), false);
    assert_eq!(
        level,
        Level {
            answer: Answer::NotQualified,
            decided_by: Requirement::Join,
            certificate: Some(Purpose::Signature),
            total_passed: false,
        }
    );
    assert_eq!(level.annex_a(), None);
}

/// REQ-4.4.2-05 b): `QSCD_NO` stops the process, indeterminate; the certificate stays determined
/// under -02 a), so with a passed validation Annex A ii) — an advanced signature supported by an
/// EU qualified certificate.
#[test]
fn a_key_not_on_a_device_is_indeterminate_and_advanced_on_a_qualified_certificate() {
    let qualification = determined(
        QcStatus::Passed,
        &[Indication::QcForESig],
        QcStatus::Passed,
        Some(QscdIndication::No),
    );
    let level = join(&qualification, true);
    assert_eq!(
        level,
        Level {
            answer: Answer::Indeterminate,
            decided_by: Requirement::Device,
            certificate: Some(Purpose::Signature),
            total_passed: true,
        }
    );
    assert_eq!(
        level.annex_a(),
        Some(Type::AdvancedSignatureOnQualifiedCertificate)
    );
    assert_eq!(
        Type::AdvancedSignatureOnQualifiedCertificate.identifier(),
        Some("0.4.0.191724.1.2.2")
    );
    assert_eq!(join(&qualification, false).annex_a(), None);
}

/// REQ-4.4.2-05 b) for a seal: `QSCD_INDETERMINATE` (PRO-4.5.4-03 a) 2) sets it with
/// `PROCESS_PASSED_WITH_WARNING`) — indeterminate, Annex A v).
#[test]
fn an_indeterminate_device_for_a_seal_is_indeterminate_and_advanced_on_a_qualified_certificate() {
    let level = join(
        &determined(
            QcStatus::Passed,
            &[Indication::QcForESeal],
            QcStatus::PassedWithWarning,
            Some(QscdIndication::Indeterminate),
        ),
        true,
    );
    assert_eq!(level.answer, Answer::Indeterminate);
    assert_eq!(level.decided_by, Requirement::Device);
    assert_eq!(
        level.annex_a(),
        Some(Type::AdvancedSealOnQualifiedCertificate)
    );
    assert_eq!(
        Type::AdvancedSealOnQualifiedCertificate.identifier(),
        Some("0.4.0.191724.1.2.5")
    );
}

/// REQ-4.4.2-05 names `PROCESS_PASSED`: `QSCD_YES` under `PROCESS_PASSED_WITH_WARNING` is the
/// other value, and the process stops.
#[test]
fn a_device_found_with_a_warning_does_not_meet_req_05() {
    let level = join(
        &determined(
            QcStatus::Passed,
            &[Indication::QcForESig],
            QcStatus::PassedWithWarning,
            Some(QscdIndication::Yes),
        ),
        true,
    );
    assert_eq!(level.answer, Answer::Indeterminate);
    assert_eq!(level.decided_by, Requirement::Device);
}

/// REQ-4.4.2-05 b): clause 4.5 failed and set no result (PRO-4.5.4-02).
#[test]
fn a_failed_device_determination_is_indeterminate() {
    let level = join(
        &determined(
            QcStatus::Passed,
            &[Indication::QcForESig],
            QcStatus::Failed,
            None,
        ),
        true,
    );
    assert_eq!(level.answer, Answer::Indeterminate);
    assert_eq!(level.decided_by, Requirement::Device);
}

/// REQ-4.4.2-02 b): a certificate clause 4.4 found `Not_Qualified` stops the process before the
/// device is asked — even a `QSCD_YES` does not reach the join — and with a passed validation
/// NOTE 5 supports an advanced signature or seal, Annex A iii) or vi), which no single identifier
/// names.
#[test]
fn a_certificate_not_qualified_is_indeterminate_and_advanced() {
    let qualification = on_a_device(&[Indication::NotQualified]);
    let level = join(&qualification, true);
    assert_eq!(
        level,
        Level {
            answer: Answer::Indeterminate,
            decided_by: Requirement::Certificate,
            certificate: None,
            total_passed: true,
        }
    );
    assert_eq!(level.annex_a(), Some(Type::Advanced));
    assert_eq!(Type::Advanced.identifier(), None);
    assert_eq!(join(&qualification, false).annex_a(), None);
}

/// REQ-4.4.2-02 names `PROCESS_PASSED`: `QC_For_eSig` under `PROCESS_PASSED_WITH_WARNING` (PRO-4.4.4-36
/// b)) is the other value, and the process stops at -02.
#[test]
fn a_certificate_found_with_a_warning_does_not_meet_req_02() {
    let level = join(
        &determined(
            QcStatus::PassedWithWarning,
            &[Indication::QcForESig],
            QcStatus::Passed,
            Some(QscdIndication::Yes),
        ),
        true,
    );
    assert_eq!(level.answer, Answer::Indeterminate);
    assert_eq!(level.decided_by, Requirement::Certificate);
    assert_eq!(level.certificate, None);
}

/// REQ-4.4.2-02 b): a failed clause 4.4, `INDET_QC_For_eSig` and a QWAC alone each hold neither
/// `QC_For_eSig` nor `QC_For_eSeal` under `PROCESS_PASSED`.
#[test]
fn every_other_certificate_answer_stops_at_req_02() {
    for (status, results) in [
        (QcStatus::Failed, vec![Indication::QcForESig]),
        (QcStatus::Passed, vec![Indication::IndetQcForESig]),
        (QcStatus::Passed, vec![Indication::Qwac]),
        (QcStatus::Passed, vec![Indication::NotQualifiedForESeal]),
        // Table 4's error pair, which a passed run never holds (PRO-4.4.4-32 b)).
        (
            QcStatus::Passed,
            vec![Indication::QcForESig, Indication::QcForESeal],
        ),
    ] {
        let level = join(&on_a_device_with(status, &results), true);
        assert_eq!(
            (level.answer, level.decided_by),
            (Answer::Indeterminate, Requirement::Certificate),
            "{status:?} {results:?}"
        );
    }
}

fn on_a_device_with(status: QcStatus, results: &[Indication]) -> Qualification {
    determined(status, results, QcStatus::Passed, Some(QscdIndication::Yes))
}

/// REQ-4.4.2-02 b): where clause 4.4 did not run — no list of the issuer's territory, an issuer
/// naming no country — its `QC-Status` holds no `PROCESS_PASSED`.
#[test]
fn a_clause_4_4_that_did_not_run_stops_at_req_02() {
    for qualification in [
        Qualification::NoListForTerritory("AT".to_owned()),
        Qualification::IssuerStatesNoCountry,
        Qualification::NoCertificate,
    ] {
        let level = join(&qualification, true);
        assert_eq!(level.decided_by, Requirement::Certificate);
        assert_eq!(level.answer, Answer::Indeterminate);
    }
}

/// ADR 2834: a locally valid signature's anchor is the reader's own, not REQ-4.2-03 a)'s, so it
/// has no level at all.
#[test]
fn a_locally_valid_signature_has_no_level() {
    use crate::verdict::tests::fixtures;
    let (document, signature) = crate::verdict::tests::opened(fixtures::signed());
    let root = crate::x509_fixtures::hex(fixtures::ROOT);
    let parsed = [crate::x509::parse(&root).expect("the root parses")];
    let at = crate::verdict::tests::AT;
    let trust = signature.trust(&TrustAnchors::of(&parsed), &Material::none(), at);
    let anchored = Anchored::of(&trust).expect("the path reaches the root");
    let integrity = signature.integrity(document.bytes());
    let authenticity = signature.authenticity(document.bytes());
    let verdict = Verdict::reached(
        &integrity,
        &authenticity,
        &anchored,
        Acceptance::UnknownRevocationAccepted,
        Timestamps::None,
        crate::verdict::BestSignatureTime::now(at),
    );
    let valid = verdict.valid().expect("locally valid");
    let asked = Asked {
        signature: &signature,
        integrity: &integrity,
        authenticity: &authenticity,
        lists: &[],
        material: &Material::none(),
        timestamps: Timestamps::None,
    };
    assert_eq!(determine(valid, &asked), None);
}

/// REQ-4.2-03 c) i): a listed path a host's acceptance made valid with no revocation material is
/// not clause 4.2's `TOTAL-PASSED`, because that validation checks the status and accepts no
/// unknown one. The fixture's issuer names no country, so the process stops at REQ-4.4.2-02 with
/// no Annex A type.
#[test]
fn a_listed_path_valid_only_by_the_hosts_acceptance_is_not_total_passed() {
    use crate::trusted_list::tests::{GRANTED_AT_LATER, Written, list_xml};
    use crate::verdict::tests::fixtures;
    let (document, signature) = crate::verdict::tests::opened(fixtures::signed());
    let root = crate::x509_fixtures::hex(fixtures::ROOT);
    let service = Written {
        identity: Some(&root),
        ..GRANTED_AT_LATER
    };
    let lists = [
        TrustedList::read(list_xml("AT", &[("A provider", &[service])]).as_bytes())
            .expect("the written list reads"),
    ];
    let at = crate::verdict::tests::AT;
    let found = crate::trusted_list::listed_anchors(&lists, &signature, at);
    let [(listing, identities)] = found.as_slice() else {
        panic!("one service issued the signer: {found:?}");
    };
    let parsed: Vec<_> = identities
        .iter()
        .map(|der| crate::x509::parse(der).expect("the identity parses"))
        .collect();
    let trust = signature.trust(&TrustAnchors::of(&parsed), &Material::none(), at);
    let anchored = Anchored::listed(&trust, listing.clone()).expect("the path reaches the root");
    let integrity = signature.integrity(document.bytes());
    let authenticity = signature.authenticity(document.bytes());
    let verdict = Verdict::reached(
        &integrity,
        &authenticity,
        &anchored,
        Acceptance::UnknownRevocationAccepted,
        Timestamps::None,
        crate::verdict::BestSignatureTime::now(at),
    );
    let valid = verdict.valid().expect("valid under the host's acceptance");
    let asked = Asked {
        signature: &signature,
        integrity: &integrity,
        authenticity: &authenticity,
        lists: &lists,
        material: &Material::none(),
        timestamps: Timestamps::None,
    };
    assert_eq!(
        determine(valid, &asked),
        Some(Level {
            answer: Answer::Indeterminate,
            decided_by: Requirement::Certificate,
            certificate: None,
            total_passed: false,
        })
    );
}
