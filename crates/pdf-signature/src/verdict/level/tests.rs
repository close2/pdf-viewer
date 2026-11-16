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

/// A root generated for these tests, `C=AT, O=quorra`, 2026 to 2036, which the written list
/// publishes as a `CA/QC` service.
const LISTED_ROOT: &str = "\
    3082033d30820225a003020102020160300d06092a864886f70d01010b05003040310b3009060355040613024154310f\
    300d060355040a0c0671756f7272613120301e06035504030c1771756f727261206c6973746564207465737420726f6f\
    74301e170d3236303130313030303030305a170d3336303130313030303030305a3040310b3009060355040613024154\
    310f300d060355040a0c0671756f7272613120301e06035504030c1771756f727261206c697374656420746573742072\
    6f6f7430820122300d06092a864886f70d01010105000382010f003082010a0282010100c00b49339fbc12459e1fefe4\
    437d524051cfa34b4b6be6a836f97e38669d780383db58b423c80244380af4f53130c2c2b6f1f9d5752c650128fbcedd\
    91a4bfe57499ccc6abb3b74d76dbb788220ac4a74dbfc39520a14289b5ab503d7b253b93cacbaa6e13105eb7208c5b0b\
    e6e2ad598955eac0ccfdc889cde8c11465e83b2e252963a2a049347b9b6b80001456fde7c8b84ae011bcfe4d39a69f40\
    ffb504c8fe806804f4cae6957ba1d184823303aeffaa1c18dcaaec85eadf02232914b7371a2f9865930ee33c8ce1a437\
    4ebfe5bd12ce5e22bb0ebb1b4646d52cf969cedbc383ad2c6994caf20fd5ea5db6831962cbfba801a3ba372c30f49f9a\
    4a9f24f30203010001a3423040300f0603551d130101ff040530030101ff300e0603551d0f0101ff040403020106301d\
    0603551d0e04160414c68d04232ec93ac5ecb061ed184aeca5370f793e300d06092a864886f70d01010b050003820101\
    002e2caf4345242280a02887f057f3509dd865b734f052f7da54014e2d60d520454dd3e047eb27924d028e4456d9f35d\
    7a4c7cfcd2ea509b12f0ef8386090dfb1d920b3d386356709e97a6201522e664415bbbec75398f0d5330950223a01bca\
    b054d4df351065a9dad9a7f10624dcb59979924042860ad7ea711b9eba977a8e00602b4c542f2359b1d6fe35ccd9f3d5\
    fc7369e192077a8cd4d4349a41a80a6aacae093bff8fac16ea93664c5260d293c7adbb3f077f1ab21495574fb7e5dfc5\
    805d758ca7a20c952b7cc620bad63e062ff203a7c3370433bd4c808f1336fb7122f8fb9dcaabd251d225d41e877545f6\
    9c03dec9fbb0a67feb2b7e5b5ddb1a63df";

/// `openssl cms -sign -binary -md sha256` over the verdict fixture's signed range, by a signer
/// [`LISTED_ROOT`] issued, `C=AT`, 2026 to 2036, stating no QC statement.
const LISTED_SIGNATURE: &str = "\
    308205df06092a864886f70d010702a08205d0308205cc020101310d300b0609608648016503040201300b06092a8648\
    86f70d010701a08203503082034c30820234a003020102020161300d06092a864886f70d01010b05003040310b300906\
    0355040613024154310f300d060355040a0c0671756f7272613120301e06035504030c1771756f727261206c69737465\
    64207465737420726f6f74301e170d3236303130313030303030305a170d3336303130313030303030305a3031310b30\
    090603550406130241543122302006035504030c1971756f727261206c69737465642074657374207369676e65723082\
    0122300d06092a864886f70d01010105000382010f003082010a0282010100dde82ace51d8d3d18197fb42d7dd61ba8a\
    1148ddb628784bdbe73a16f165005595ff0cbecdfbae050ec5690717b2756d4c18b913274ea6ecf653ef50f6587e8f5b\
    386aa627f329df28929fc02a64c6966e9ebbefa4fec3475a4846d4b82df87511231bb9847afa23b3b9bc47a93921d71c\
    c28001ed151d7f920a33b921d33a872ae741945ac0132b0d0b372a4edee64e5d5463a74037c15dfc4c65ab9c8ca9019d\
    e1d874ae3f4c51e027d8ca778651a7a8ae9965f83e9a6702b66457dad6af4c63a11cd7d1bce249056820d28e157faa3d\
    74d8d4611a82d9e912d6bb902638a82b16697b0219f4e0726b5cf7f601a4ceb0057bb66cf23c8e6dae8bd0721980dd02\
    03010001a360305e300c0603551d130101ff04023000300e0603551d0f0101ff0404030206c0301d0603551d0e041604\
    1462018c4b5b7783450334b3bf5ae826e1229e95f8301f0603551d23041830168014c68d04232ec93ac5ecb061ed184a\
    eca5370f793e300d06092a864886f70d01010b050003820101001f77adb59c4af372859641fbff00ed80ef9ccbe583bd\
    e5338debeaf2729203a3c3ed7cc42a0599e5b69336ad58a8621f48a130ad060f515db7e0db68ed0cebfd5def7e2a8166\
    0e0cc85254e85e424f7e9d63488c906e191a03c96fb9d6c03c64a6510a180dff920913378065f5572246b76d2b011cdc\
    849f1fbe88c21022872f05ce240a6ab65deb6ab5c4644238e8dc3ea5a8ebdd725879c8b854abc6ce417b13bf5d89a060\
    35e266472f3c874f67ddaf726e0cbb42b04dee7b26a1765fa288d6bdadb6ee8bea39f9fe2abf2a84867c7240febcff08\
    283e3d4d256d8a2e2bb105b81676949a577882ded97bc457cbce78071fb15d40704cf5362f308baa50d8318202553082\
    025102010130453040310b3009060355040613024154310f300d060355040a0c0671756f7272613120301e0603550403\
    0c1771756f727261206c6973746564207465737420726f6f74020161300b0609608648016503040201a081e430180609\
    2a864886f70d010903310b06092a864886f70d010701301c06092a864886f70d010905310f170d323631313135323335\
    3332385a302f06092a864886f70d01090431220420a4a4c68c931b0e4f52b432969673d37c545bc784291231a7aff3f2\
    568fc45797307906092a864886f70d01090f316c306a300b060960864801650304012a300b0609608648016503040116\
    300b0609608648016503040102300a06082a864886f70d0307300e06082a864886f70d030202020080300d06082a8648\
    86f70d0302020140300706052b0e030207300d06082a864886f70d0302020128300d06092a864886f70d010101050004\
    8201006290fecca2ab428ad68fd53d1b34806381fea08cd9a9ccb5979267f2b636bc360e99d977da1dcf24cc40a8f90c\
    88aaaccaeb450050983db91ab603005bd8e7949c9aadd41ecb7dae8d1d6c9c4631f3df2c3ae1741942d78b563b2a2899\
    f0751419e1d998f84cb258a4558400b85b9244933493bc4d9854bd415d3bc9337d007927a3363a6eaa1628dc29295dd4\
    6b9b4b662b313b756654c79f454853bfa66f3ee1122e3fd846a71cb6dd3d0ef7ee6fb5a2aa8c4f926a536ab6f793a4fa\
    3eff86945ab6ade44c4e3e0f9da4709ae034c994fd366aa30f32299447dcb4425fd9803173447ccbf78d124426ef281e\
    71805c310675f82e6fd4fc53ff6310801698b0";

/// The root's CRL, `thisUpdate` 2026-12-02, `nextUpdate` 2036-12-02, listing nothing.
const CRL_AFTER: &str = "\
    30820199308182020101300d06092a864886f70d01010b05003040310b3009060355040613024154310f300d06035504\
    0a0c0671756f7272613120301e06035504030c1771756f727261206c6973746564207465737420726f6f74170d323631\
    3230323030303030305a170d3336313230323030303030305aa00e300c300a0603551d140403020101300d06092a8648\
    86f70d01010b050003820101008934d274c028f94163907cf01850b740b2cc59ec0e68e1a885bbad1395e3309c3d5afc\
    6c6456d443d4f32b15fd0275a206c56de80e02b3e4519eab7c7118674b82ad5d9c936e6c70f555caa727e1a1f2d5d63a\
    1ec5a39c1b899a153746fc467a2150fa9fa568a58f36d730a9a479d8e441aa18da2790b93c69f7c04e76c9dc45547adc\
    af95d7bda6625c16c660a0f083c43f08294061bf2c22274a0634b9a8599be691e8999c397cbd7cf2564a5a7892d94922\
    067a49e84b5b123d3a80aa3919d26bf8586adb210b4bdf7d6d15e27ca5a88b29130c4a502ffbed8ffe0d0209275235dd\
    02f13b2e27a16224da2e528c875e49cf4e32c33238baab6c6442bda2cf";

/// The same root's CRL, `thisUpdate` 2026-11-01, `nextUpdate` 2036-11-01, listing nothing.
const CRL_BEFORE: &str = "\
    30820199308182020101300d06092a864886f70d01010b05003040310b3009060355040613024154310f300d06035504\
    0a0c0671756f7272613120301e06035504030c1771756f727261206c6973746564207465737420726f6f74170d323631\
    3130313030303030305a170d3336313130313030303030305aa00e300c300a0603551d140403020102300d06092a8648\
    86f70d01010b05000382010100606ef00c1137d74a15d4611eb116bd17207a1eb9629f2c9c87039d74b1063a4aa07e91\
    499439155c9e3925a06d62ede64a8b54821aaf8671c1de37028abe165595f4cac21ec1f24fee84e2f73f716a6251b091\
    11f73fca3b05ff63de7ed60646f41e4b18460fc431ca63eef7a4e1ce0409c048b1e19d1bca44cd71a10e87fa237bf421\
    48118ce70e22b1bce3db4d7542f4642efa7834865e94c9c79a1e18eff91be4d87be50e23b7bd04da0a0689abbb5a4cae\
    9cf7fc86bb40409fc96fde884f654956d74761b4aae822dacd30a39430b3af71185e3a21e537421d514a89d5b4b32893\
    e9933da83fc1917bc91588632418473efdcd5cac51c8efb123f7ce0910";

/// 2026-12-01T00:00:00Z: inside the listed hierarchy's validity, after the signer was issued and
/// before [`CRL_AFTER`] was.
const LISTED_AT: crate::x509::Instant = crate::x509::Instant::from_unix_seconds(1_796_083_200);

/// The listed fixture's verdict and level with `crl` as the document's material, the best
/// signature time the reader's clock at [`LISTED_AT`], and the host accepting nothing short of a
/// computed status — the path `viewer_core::notes` takes.
fn listed_level(crl: &str) -> Option<Level> {
    use crate::trusted_list::tests::{GRANTED_AT_LATER, Written, list_xml};
    let (document, signature) =
        crate::verdict::tests::opened(crate::verdict::tests::fixtures::document(LISTED_SIGNATURE));
    let root = crate::x509_fixtures::hex(LISTED_ROOT);
    let service = Written {
        identity: Some(&root),
        ..GRANTED_AT_LATER
    };
    let lists = [
        TrustedList::read(list_xml("AT", &[("A provider", &[service])]).as_bytes())
            .expect("the written list reads"),
    ];
    let crl = crate::x509_fixtures::hex(crl);
    let material = Material::read(&[&crl], &[]);
    assert!(material.refused.is_empty(), "{:?}", material.refused);
    let found = crate::trusted_list::listed_anchors(&lists, &signature, LISTED_AT);
    let [(listing, identities)] = found.as_slice() else {
        panic!("one service issued the signer: {found:?}");
    };
    let parsed: Vec<_> = identities
        .iter()
        .map(|der| crate::x509::parse(der).expect("the identity parses"))
        .collect();
    let trust = signature.trust(&TrustAnchors::of(&parsed), &material, LISTED_AT);
    let anchored = Anchored::listed(&trust, listing.clone()).expect("the path reaches the root");
    let integrity = signature.integrity(document.bytes());
    let authenticity = signature.authenticity(document.bytes());
    let verdict = Verdict::reached(
        &integrity,
        &authenticity,
        &anchored,
        Acceptance::RevocationMustBeGood,
        Timestamps::None,
        crate::verdict::BestSignatureTime::now(LISTED_AT),
    );
    let valid = verdict.valid().expect("valid: the CRL lists nothing");
    determine(
        valid,
        &Asked {
            signature: &signature,
            integrity: &integrity,
            authenticity: &authenticity,
            lists: &lists,
            material: &material,
            timestamps: Timestamps::None,
        },
    )
}

/// REQ-4.2-03 c) ii) and REQ-4.4.2-06 end to end: a listed signer whose issuer's CRL was issued
/// after the best signature time passes clause 4.2's validation (`TOTAL-PASSED`). Its certificate
/// states no `QcCompliance`, so clause 4.4 holds no `QC_For_eSig` and the process stops at
/// REQ-4.4.2-02 b); with the validation passed, Annex A's type is the advanced one.
#[test]
fn a_listed_signer_with_revocation_issued_after_the_best_signature_time_is_total_passed() {
    let level = listed_level(CRL_AFTER).expect("a listed path has a level");
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
}

/// The control, the CRL alone differing: issued a month before the best signature time, it is not
/// fresh under REQ-4.2-03 c) ii)'s zero, so clause 4.2's validation does not pass and no Annex A
/// type is said.
#[test]
fn the_same_signer_with_revocation_issued_before_the_best_signature_time_is_not_total_passed() {
    let level = listed_level(CRL_BEFORE).expect("a listed path has a level");
    assert!(!level.total_passed, "{level:?}");
    assert_eq!(level.annex_a(), None);
}
