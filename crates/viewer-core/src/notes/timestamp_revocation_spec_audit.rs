//! ISO 32000-2 §12.8.3.4.8: a known compromised TSA cannot supply historical proof.
//! This exercises the time selector with genuine signed CMS/CRL inputs, not a full PDF report.

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "keep the signed-material controls beside the selector assertion"
)]
fn a_known_key_compromise_on_the_timestamp_path_does_not_set_best_signature_time() {
    use pdf_signature::revocation::{Material, Revocation, RevocationReason};
    use pdf_signature::timestamp::{AskedAt, Time, signature_timestamp_established, token_of};
    use pdf_signature::trust::TrustAnchors;
    use pdf_signature::x509::{Instant, parse};

    const ROOT: &[u8] =
        include_bytes!("../../tests/fixtures/signature_timestamp_revocation/root.der");
    const CRL: &[u8] =
        include_bytes!("../../tests/fixtures/signature_timestamp_revocation/tsa-revoked-crl.der");
    const GOOD_CRL: &[u8] =
        include_bytes!("../../tests/fixtures/signature_timestamp_revocation/tsa-good-crl.der");
    const TOKEN: &[u8] =
        include_bytes!("../../tests/fixtures/signature_timestamp_revocation/token.der");
    const CMS: &[u8] = include_bytes!(
        "../../tests/fixtures/signature_timestamp_revocation/cms-with-signature-timestamp.der"
    );
    // Fixture token genTime is recorded in the README. Check its TSA path one hour later, before
    // either certificate expires and inside both CRLs' signed update windows.
    const VERIFY_NOW: Instant = Instant::from_unix_seconds(1_794_849_143);

    let root = parse(ROOT).expect("the fixture's test root certificate parses");
    let anchors = TrustAnchors::of(std::slice::from_ref(&root));
    let signature = pdf_signature::signature::Signature {
        timestamp: false,
        handler: None,
        sub_filter: Some("ETSI.CAdES.detached".to_owned()),
        byte_range: Vec::new(),
        contents: CMS.to_vec(),
        certificate_chain: false,
        chain: Vec::new(),
        name: None,
        signed_at: None,
        location: None,
        reason: None,
        contact: None,
        changes: None,
        certification: false,
        format_version: None,
        indirect_values: Vec::new(),
        reference_digests: Vec::new(),
    };
    let cms = signature.signed_data().expect("fixture CMS parses");
    let token_cms =
        pdf_signature::cms::signed_data(TOKEN).expect("the RFC 3161 token parses as CMS");
    let info = token_of(&token_cms).expect("the token's encapsulated TSTInfo reads");
    assert_eq!(info.gen_time.unix_seconds(), 1_794_845_543);
    assert!(
        pdf_signature::timestamp::signature_timestamp(&cms)
            .expect("one signature timestamp attribute")
            .expect("the nested token parses")
            .covers_the_signature,
        "the token imprint is the digest of the actual CMS SignerInfo.signature octets"
    );

    let good_material = Material::read(&[GOOD_CRL], &[]);
    assert!(
        good_material.refused.is_empty(),
        "{:?}",
        good_material.refused
    );
    let good_status = signature_timestamp_established(
        &cms,
        &anchors,
        &[],
        &good_material,
        AskedAt::TheCallersInstant(VERIFY_NOW),
    )
    .expect("timestamp attribute exists");
    assert!(
        matches!(
            &good_status,
            Time::Established {
                revocation: Revocation::Good { .. },
                ..
            }
        ),
        "the clean current-time CRL is the positive revocation control: {good_status:?}"
    );
    assert_eq!(
        super::signature_timestamps_instant(&signature, &anchors, &[], &good_material, VERIFY_NOW),
        Some(info.gen_time),
        "valid-token and Good TSA-revocation control: its genTime is usable"
    );

    let material = Material::read(&[CRL], &[]);
    assert!(material.refused.is_empty(), "{:?}", material.refused);
    let established = signature_timestamp_established(
        &cms,
        &anchors,
        &[],
        &material,
        AskedAt::TheCallersInstant(VERIFY_NOW),
    )
    .expect("timestamp attribute exists");
    assert!(
        matches!(
            &established,
            Time::Established {
                revocation: Revocation::Revoked {
                    reason: Some(RevocationReason::KeyCompromise),
                    ..
                },
                ..
            }
        ),
        "the token may be structurally/path established while its TSA revocation makes its time unusable: {established:?}"
    );

    // This negative is checked before the expected-to-fail selector assertion, so a future
    // regression cannot hide the independent invalid-signature control behind the first failure.
    let mut bad_crl = CRL.to_vec();
    let last = bad_crl.len().saturating_sub(1);
    bad_crl[last] ^= 1;
    let invalid_material = Material::read(&[&bad_crl], &[]);
    assert!(invalid_material.refused.is_empty(), "the DER still parses");
    let invalid_answer = signature_timestamp_established(
        &cms,
        &anchors,
        &[],
        &invalid_material,
        AskedAt::TheCallersInstant(VERIFY_NOW),
    )
    .expect("timestamp attribute exists");
    assert!(
        matches!(
            &invalid_answer,
            Time::Established {
                revocation: Revocation::Unknown { .. },
                ..
            }
        ),
        "a bad CRL signature is Unknown evidence, never the keyCompromise answer: {invalid_answer:?}"
    );

    assert_eq!(
        super::signature_timestamps_instant(&signature, &anchors, &[], &material, VERIFY_NOW),
        None,
        "§12.8.3.4.8: keyCompromise makes genTime unreliable; decline historical proof"
    );
}
