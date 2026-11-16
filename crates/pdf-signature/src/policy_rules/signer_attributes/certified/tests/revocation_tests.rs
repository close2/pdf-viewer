//! An attribute certificate's chain under the revocation requirement a policy states for attribute
//! certificates (ADR 2890): TR 102 272 clause 6.9's `attrRevReq`, whose checks clause 6.6.2 splits
//! between end certificates and CA certificates. The chain is the attribute certificate, its
//! authority's certificate — an end certificate too, since RFC 5755 section 4.5 forbids it to be a
//! CA's — and the CA certificates of the authority's path below the trust point.
//!
//! The material was built once with `openssl` over the attribute hierarchy
//! [`crate::attribute_certificate::tests::fixtures`] holds, with one CA added between the attribute
//! root and a second authority; the keys are this tree's own and the DER is pasted in. Each
//! expected value is read off the clause the test names, and each revocation has the answered
//! chain beside it, changing one member (trap 13).

use super::{Held, held, judged_with, trees};
use crate::attribute_certificate::tests::fixtures::AC;
use crate::attribute_certificate::tests::hex;
use crate::policy_rules::certification_path::tests::WITHIN;
use crate::policy_rules::revocation_checks::{Check, Checked, Kind, Requirement};
use crate::policy_rules::signer_attributes::certified::{Invalid, Outcome, Undecided};
use crate::revocation::{Evidence, Material, Undetermined};
use crate::x509::Instant;
use fixtures::{
    AA_MIDDLE, AA_UNDER_A_CA, AC_NO_REVOCATION, AC_UNDER_A_CA, CRL_AA_FOR_AC, CRL_MIDDLE_EMPTY,
    CRL_ROOT_EMPTY, CRL_ROOT_REVOKING_AA, CRL_ROOT_REVOKING_MIDDLE, OCSP_AC_GOOD, OCSP_AC_REVOKED,
    OCSP_UNDER_GOOD,
};

/// 2026-03-01T00:00:00Z, the revocation date every revoking fixture states: before [`WITHIN`].
const REVOKED_AT: Instant = Instant::from_unix_seconds(1_772_323_200);

pub(crate) mod fixtures {
    //! ECDSA P-256 with SHA-256 throughout. Every CRL was issued 2026-05-01 for an update by
    //! 2027-06-01; every OCSP response was produced 2026-11-16, signed by the attribute authority
    //! that issued the certificate it names (RFC 6960 section 4.2.2.2's second criterion), for an
    //! update by 2027-12-21.

    /// Issued by [`AA_UNDER_A_CA`], serial number 0x16, its holder the signer by `baseCertificateID`.
    pub(crate) const AC_UNDER_A_CA: &str = "\
        3082013c3081e40201013041a03f3027a42530233121301f06035504030c1871756f727261207465737420696e746572\
        6d65646961746502142098d48ac135f5264c4317afd4dd554de2d88312a03b3039a43730353133303106035504030c2a\
        71756f72726120746573742061747472696275746520617574686f7269747920756e6465722061204341300a06082a86\
        48ce3d0403020201163022180f32303236303130313030303030305a180f32303237303130313030303030305a302c30\
        2a060355044831233021a11f861d75726e3a71756f7272613a746573743a726f6c653a617070726f766572300a06082a\
        8648ce3d040302034700304402201f1926fd921802608c8c5638629330a47311db31fa8a15accf85bf89b9f0aec00220\
        27d9ed2e7f667569ae4ec692bc1cf64d4644a99313c6add0b2193375ba7f1f5c";

    /// Issued by [`crate::attribute_certificate::tests::fixtures::AA`], serial number 0x17, carrying
    /// RFC 5755 section 4.3.6's `noRevAvail`.
    pub(crate) const AC_NO_REVOCATION: &str = "\
        3082013f3081e60201013041a03f3027a42530233121301f06035504030c1871756f727261207465737420696e746572\
        6d65646961746502142098d48ac135f5264c4317afd4dd554de2d88312a030302ea42c302a3128302606035504030c1f\
        71756f72726120746573742061747472696275746520617574686f72697479300a06082a8648ce3d0403020201173022\
        180f32303236303130313030303030305a180f32303237303130313030303030305a302c302a060355044831233021a1\
        1f861d75726e3a71756f7272613a746573743a726f6c653a617070726f766572300b30090603551d3804020500300a06\
        082a8648ce3d0403020348003045022100bf8a1433b00fa571f064452098739ee7424fc11d2b42ea82696dfb16a094da\
        6e02206a8723fe7d0e3fdd7934c06d9d71f0d393052ce401166660a3a140def3db4ca7";

    /// A CA the attribute root issued, `keyCertSign` and `cRLSign`, serial number 0x2037a1.
    pub(crate) const AA_MIDDLE: &str = "\
        308201a430820149a00302010202032037a1300a06082a8648ce3d04030230253123302106035504030c1a71756f7272\
        6120746573742061747472696275746520726f6f74301e170d3236303130313030303030305a170d3330303130313030\
        303030305a302a3128302606035504030c1f71756f727261207465737420617474726962757465206d6964646c652043\
        413059301306072a8648ce3d020106082a8648ce3d0301070342000411be8f84176268546109204ba3902ee334023ef5\
        0df667fcb51c498730a3a01b3658ca31c6c09032a814a90ed111e545531a822b729a5f423bb76096efa3e779a3633061\
        300f0603551d130101ff040530030101ff300e0603551d0f0101ff040403020106301d0603551d0e04160414f24fe60b\
        9034323656dce8011f369d5c6f285565301f0603551d23041830168014b0d5dc4be2edffc109f56863ecf2c4dc456b87\
        17300a06082a8648ce3d0403020349003046022100e6bd075147d7a09b418674177d849358da473de5e2a4388c78fcb5\
        c65551be3e022100e2a5588103b33425407cd5ab174d37c3ae46dbe0e51a709d2bf52a64986d8546";

    /// An attribute authority [`AA_MIDDLE`] issued, `digitalSignature` only, no basic constraints.
    pub(crate) const AA_UNDER_A_CA: &str = "\
        308201a230820148a00302010202032037a2300a06082a8648ce3d040302302a3128302606035504030c1f71756f7272\
        61207465737420617474726962757465206d6964646c65204341301e170d3236303130313030303030305a170d333030\
        3130313030303030305a30353133303106035504030c2a71756f72726120746573742061747472696275746520617574\
        686f7269747920756e64657220612043413059301306072a8648ce3d020106082a8648ce3d03010703420004a40f18ed\
        7e8462e92441e2bc806afe8e2adfd04688df5a9c15e0cfe917d589cc1304d09dbae2f35e1f70a0fe49bc5bc3fd20fc50\
        d5723c0db4bdf0a2e69f1247a3523050300e0603551d0f0101ff040403020780301d0603551d0e041604148d0637f2e9\
        a12b98b8858282042151a6d9407be4301f0603551d23041830168014f24fe60b9034323656dce8011f369d5c6f285565\
        300a06082a8648ce3d04030203480030450221009683c7a675b1dd59dd1af71de222ed1b54b30d12b0d7fe06739731e1\
        fa0f770a02206cb1960245c2016ef46411e4ac51f141d99491e36bd3fc7a468a6803661a4ea2";

    /// The attribute root's CRL, listing nothing.
    pub(crate) const CRL_ROOT_EMPTY: &str = "\
        3081bc3064020101300a06082a8648ce3d04030230253123302106035504030c1a71756f727261207465737420617474\
        72696275746520726f6f74170d3236303530313030303030305a170d3237303630313030303030305aa00e300c300a06\
        03551d140403020101300a06082a8648ce3d040302034800304502200a8d5682d613409bde331ac51c7cdcb63d5aa975\
        35c1d05a6c26c12457bcf77c022100d054531b52678b268eeb592ed49b69dfd52be347f44fe82af3997a530ec5fdb6";

    /// The attribute root's CRL, listing the first authority's certificate revoked 2026-03-01.
    pub(crate) const CRL_ROOT_REVOKING_AA: &str = "\
        3081dc308182020101300a06082a8648ce3d04030230253123302106035504030c1a71756f7272612074657374206174\
        7472696275746520726f6f74170d3236303530313030303030305a170d3237303630313030303030305a301c301a0209\
        00a00bbd3db5e090b6170d3236303330313030303030305aa00e300c300a0603551d140403020101300a06082a8648ce\
        3d04030203490030460221008feda11aa6f6873f9f4332faf5b0142f7bd5cc03fce4505cfbdb37ad5e13748f022100b1\
        4d2aaa2f566ade4e262aed836e51679de3f7936d963a0f40c866d4c6ec28ac";

    /// The attribute root's CRL, listing [`AA_MIDDLE`] revoked 2026-03-01.
    pub(crate) const CRL_ROOT_REVOKING_MIDDLE: &str = "\
        3081d3307c020101300a06082a8648ce3d04030230253123302106035504030c1a71756f727261207465737420617474\
        72696275746520726f6f74170d3236303530313030303030305a170d3237303630313030303030305a30163014020320\
        37a1170d3236303330313030303030305aa00e300c300a0603551d140403020101300a06082a8648ce3d040302034700\
        304402203a30c1fcaa5b29ab3a90f1f96be89e9b5c228be2120e4c22f14783889e49d3b1022052f176204822d4060c1f\
        76224dbe898a3c4459f3709fde3bf57c624ff363bde0";

    /// [`AA_MIDDLE`]'s CRL, listing nothing.
    pub(crate) const CRL_MIDDLE_EMPTY: &str = "\
        3081c03069020101300a06082a8648ce3d040302302a3128302606035504030c1f71756f727261207465737420617474\
        726962757465206d6964646c65204341170d3236303530313030303030305a170d3237303630313030303030305aa00e\
        300c300a0603551d140403020101300a06082a8648ce3d04030203470030440220127b68f749ce960eb1b3d55b3c5a25\
        29cf93701dfff5f10779f3fc731b33ab9d02207117b5cfab642c8fdd11b6148152cc9801230810f8273f31b329d1bd51\
        11b965";

    /// A CRL signed by the first authority, whose key usage lacks `cRLSign`, listing nothing.
    pub(crate) const CRL_AA_FOR_AC: &str = "\
        3081c13069020101300a06082a8648ce3d040302302a3128302606035504030c1f71756f727261207465737420617474\
        72696275746520617574686f72697479170d3236303530313030303030305a170d3237303630313030303030305aa00e\
        300c300a0603551d140403020101300a06082a8648ce3d0403020348003045022100cdf01550151d10cac93b6ed1fe03\
        c810c58b5ed32762a29e96c6048e8084613402201476692c7251176db38e7b0d114ae4a7f20a120c016519bbdc49d7e1\
        a9ee9929";

    /// The first authority's answer for serial number 0x11, [`AC`](super::AC): good.
    pub(crate) const OCSP_AC_GOOD: &str = "\
        3082011c0a0100a08201153082011106092b0601050507300101048201023081ff3081a5a12c302a3128302606035504\
        030c1f71756f72726120746573742061747472696275746520617574686f72697479180f323032363131313630363035\
        35365a30643062303a300906052b0e03021a05000414214b4afcf7e20291395bf3341a73c4a39ddbf671041467797203\
        d0dc7847365c7b177e2e037fccd033480201118000180f32303236313131363036303535365aa011180f323032373132\
        32313036303535365a300a06082a8648ce3d04030203490030460221008b1fab11a8e2537c8a27f50a2d2b8d82bd00e0\
        a43fe9094f091bdefc4e9960c5022100bee7b483ad3cefaed6fbcc6de76a551ab45ec52d9ae53abb574c5ae9a32da7cd";

    /// The first authority's answer for serial number 0x11: revoked 2026-03-01.
    pub(crate) const OCSP_AC_REVOKED: &str = "\
        3082012d0a0100a08201263082012206092b0601050507300101048201133082010f3081b6a12c302a31283026060355\
        04030c1f71756f72726120746573742061747472696275746520617574686f72697479180f3230323631313136303630\
        3535365a30753073303a300906052b0e03021a05000414214b4afcf7e20291395bf3341a73c4a39ddbf6710414677972\
        03d0dc7847365c7b177e2e037fccd03348020111a111180f32303236303330313030303030305a180f32303236313131\
        363036303535365aa011180f32303237313232313036303535365a300a06082a8648ce3d0403020348003045022075dd\
        788e71c0735c0c3f874cb312582ae1974a6126002df363fcf8a682ffc803022100a55e38d9c2505e4b70e627d290c7b6\
        b1e76fb8b167e890cc94571b4c1006580a";

    /// [`AA_UNDER_A_CA`]'s answer for serial number 0x16, [`AC_UNDER_A_CA`]: good.
    pub(crate) const OCSP_UNDER_GOOD: &str = "\
        308201270a0100a08201203082011c06092b06010505073001010482010d308201093081b0a137303531333031060355\
        04030c2a71756f72726120746573742061747472696275746520617574686f7269747920756e6465722061204341180f\
        32303236313131363036303535375a30643062303a300906052b0e03021a05000414df91889e755f81b290c557817ae2\
        8b8f7e930f8604148d0637f2e9a12b98b8858282042151a6d9407be40201168000180f32303236313131363036303535\
        375aa011180f32303237313232313036303535375a300a06082a8648ce3d0403020348003045022100e40cc5813f0c25\
        b02e0792c2892eddc2f77de0213f56e7f19a8f71cab4daf49f02201a44434e5390fe0713732314d56accf85dfc2c6b33\
        00c11fe76820b0dac7a310";
}

/// A requirement of `end` on the end certificates and `authorities` on the CA certificates, with
/// no freshness and no retention: TR 102 272 clause 6.6.2's `CertRevReq`.
pub(in crate::policy_rules) const fn requirement(end: Check, authorities: Check) -> Requirement {
    Requirement {
        end_entity: end,
        authorities,
        freshness: None,
        retention: None,
    }
}

/// `certificate` judged under `requirement` from `crls` and `ocsps`, with `carried` beside the
/// signer's certificates and the authority's path searched to the attribute root.
fn under(
    held: &Held,
    certificate: &str,
    carried: &[&[u8]],
    requirement: &Requirement,
    crls: &[&str],
    ocsps: &[&str],
) -> Outcome {
    let crls: Vec<Vec<u8>> = crls.iter().map(|text| hex(text)).collect();
    let ocsps: Vec<Vec<u8>> = ocsps.iter().map(|text| hex(text)).collect();
    let crls: Vec<&[u8]> = crls.iter().map(Vec::as_slice).collect();
    let ocsps: Vec<&[u8]> = ocsps.iter().map(Vec::as_slice).collect();
    let material = Material::read(&crls, &ocsps);
    assert!(material.refused.is_empty(), "{:?}", material.refused);
    let tree = trees(&[&held.authority_root]);
    judged_with(
        certificate,
        held,
        carried,
        Some(&tree),
        WITHIN,
        Some(requirement),
        &material,
    )
}

/// What a revoked chain's check found, as `(kind, position, evidence)`.
fn revoked(outcome: &Outcome) -> Option<(Kind, usize, Evidence)> {
    match outcome {
        Outcome::Invalid(Invalid::Revoked(check)) => match check.checked {
            Checked::Revoked {
                position,
                from,
                effective,
            } => {
                assert_eq!(effective, REVOKED_AT);
                Some((check.kind, position, from))
            }
            _ => None,
        },
        _ => None,
    }
}

/// What an unanswered check found, as `(kind, position, evidence, why, noRevAvail)`.
fn unanswered(outcome: &Outcome) -> Option<(Kind, usize, Evidence, Undetermined, bool)> {
    match outcome {
        Outcome::Undecided(Undecided::Revocation(check, no_revocation)) => match &check.checked {
            Checked::Unanswered {
                position,
                missing,
                why,
            } => Some((check.kind, *position, *missing, why.clone(), *no_revocation)),
            _ => None,
        },
        _ => None,
    }
}

#[test]
fn a_chain_the_material_answers_unrevoked_leaves_the_certificate_valid() {
    // TR 102 272 clause 6.6.2's `eitherCheck` on the end certificates: the attribute certificate
    // (position 0) answered by its authority's OCSP response, the authority's certificate
    // (position 1) by the attribute root's CRL, neither listing it, both current at the instant
    // (RFC 6960 section 2.4, RFC 5280 section 6.3.3). The authority's path holds no CA below the
    // root, so the CA check has nothing to ask.
    let held = held();
    let carried: &[&[u8]] = &[&held.authority];
    let either = requirement(Check::Either, Check::Crl);
    assert_eq!(
        under(
            &held,
            AC,
            carried,
            &either,
            &[CRL_ROOT_EMPTY],
            &[OCSP_AC_GOOD]
        ),
        Outcome::Valid
    );
    // A stated requirement that mandates no check asks nothing of the file (clause 6.6.2's
    // `noCheck`), where before this round any stated requirement left the certificate undecided.
    let none = requirement(Check::NoCheck, Check::NoCheck);
    assert_eq!(under(&held, AC, carried, &none, &[], &[]), Outcome::Valid);
}

#[test]
fn a_revoked_certificate_of_the_chain_makes_the_attribute_certificate_invalid() {
    // RFC 5755 section 5's additional check 1 lets the verifier's configuration reject an
    // attribute certificate, and the policy is that configuration; RFC 5280 section 6.1.3 (a)(3)
    // makes a revoked certificate on the authority's path one that does not validate.
    let held = held();
    let carried: &[&[u8]] = &[&held.authority];
    let either = requirement(Check::Either, Check::NoCheck);
    // The attribute certificate itself, by its authority's OCSP response.
    assert_eq!(
        revoked(&under(
            &held,
            AC,
            carried,
            &either,
            &[CRL_ROOT_EMPTY],
            &[OCSP_AC_REVOKED]
        )),
        Some((Kind::AttributeEndEntities, 0, Evidence::OcspResponse))
    );
    // The authority's certificate, by the attribute root's CRL.
    assert_eq!(
        revoked(&under(
            &held,
            AC,
            carried,
            &either,
            &[CRL_ROOT_REVOKING_AA],
            &[OCSP_AC_GOOD]
        )),
        Some((
            Kind::AttributeEndEntities,
            1,
            Evidence::CertificateRevocationList
        ))
    );
}

#[test]
fn a_check_the_material_does_not_answer_leaves_the_certificate_undecided() {
    // ETSI EN 319 102-1 clause 5.2.6.4's indeterminate: this program answers revocation only from
    // the file, and absent evidence is not evidence of revocation.
    let held = held();
    let carried: &[&[u8]] = &[&held.authority];
    let either = requirement(Check::Either, Check::NoCheck);
    // No OCSP response for the attribute certificate, and no CRL from its authority.
    assert!(matches!(
        unanswered(&under(&held, AC, carried, &either, &[CRL_ROOT_EMPTY], &[])),
        Some((
            Kind::AttributeEndEntities,
            0,
            Evidence::OcspResponse,
            _,
            false
        ))
    ));
    // The attribute certificate answered, its authority's certificate not.
    assert!(matches!(
        unanswered(&under(&held, AC, carried, &either, &[], &[OCSP_AC_GOOD])),
        Some((
            Kind::AttributeEndEntities,
            1,
            Evidence::OcspResponse,
            _,
            false
        ))
    ));
    // RFC 5280 section 6.3.3 (f): the authority's key usage lacks `cRLSign` — RFC 5755 section
    // 4.5 asks only that it admit signatures — so a CRL under its key answers nothing.
    let crl = requirement(Check::Crl, Check::NoCheck);
    assert_eq!(
        unanswered(&under(
            &held,
            AC,
            carried,
            &crl,
            &[CRL_AA_FOR_AC, CRL_ROOT_EMPTY],
            &[]
        )),
        Some((
            Kind::AttributeEndEntities,
            0,
            Evidence::CertificateRevocationList,
            Undetermined::KeyUsageForbidsCrlSigning,
            false
        ))
    );
    // Clause 6.6.2's `other`: a mechanism only a policy extension defines.
    let other = requirement(Check::Other, Check::NoCheck);
    assert!(matches!(
        under(&held, AC, carried, &other, &[], &[]),
        Outcome::Undecided(Undecided::Revocation(check, false))
            if check.checked == Checked::OtherMechanism
    ));
}

#[test]
fn the_ca_check_covers_the_authoritys_path_below_its_trust_point() {
    // The chain AC_UNDER_A_CA (0), AA_UNDER_A_CA (1), AA_MIDDLE (2), and the attribute root, the
    // trust point, which is not checked. Clause 6.6.2's `caCerts` covers position 2 alone.
    let held = held();
    let carried: &[&[u8]] = &[&hex(AA_UNDER_A_CA), &hex(AA_MIDDLE)];
    let ca = requirement(Check::NoCheck, Check::Crl);
    assert_eq!(
        under(&held, AC_UNDER_A_CA, carried, &ca, &[CRL_ROOT_EMPTY], &[]),
        Outcome::Valid
    );
    assert_eq!(
        revoked(&under(
            &held,
            AC_UNDER_A_CA,
            carried,
            &ca,
            &[CRL_ROOT_REVOKING_MIDDLE],
            &[]
        )),
        Some((
            Kind::AttributeAuthorities,
            2,
            Evidence::CertificateRevocationList
        ))
    );
    assert!(matches!(
        unanswered(&under(&held, AC_UNDER_A_CA, carried, &ca, &[], &[])),
        Some((
            Kind::AttributeAuthorities,
            2,
            Evidence::CertificateRevocationList,
            Undetermined::NoMaterial,
            false
        ))
    ));
    // The end check over the same chain: the attribute certificate by its authority's OCSP
    // response, the authority's certificate by the middle CA's CRL; without that CRL, position 1
    // is the one unanswered.
    let end = requirement(Check::Either, Check::NoCheck);
    assert_eq!(
        under(
            &held,
            AC_UNDER_A_CA,
            carried,
            &end,
            &[CRL_MIDDLE_EMPTY],
            &[OCSP_UNDER_GOOD]
        ),
        Outcome::Valid
    );
    assert!(matches!(
        unanswered(&under(
            &held,
            AC_UNDER_A_CA,
            carried,
            &end,
            &[],
            &[OCSP_UNDER_GOOD]
        )),
        Some((
            Kind::AttributeEndEntities,
            1,
            Evidence::OcspResponse,
            _,
            false
        ))
    ));
}

#[test]
fn an_unanswered_check_on_a_certificate_stating_no_revocation_says_so() {
    // RFC 5755 section 4.3.6: `noRevAvail` is the issuer's word that no revocation information
    // will be made available. TS 119 172-2 clause 4.23.2.1 gives the policy's requirement
    // precedence over what a certificate says, so the mandated check stands, unanswered, and the
    // sentence names the extension; the certificate without it is the control.
    let held = held();
    let carried: &[&[u8]] = &[&held.authority];
    let ocsp = requirement(Check::Ocsp, Check::NoCheck);
    let stating = under(&held, AC_NO_REVOCATION, carried, &ocsp, &[], &[]);
    assert!(matches!(
        unanswered(&stating),
        Some((
            Kind::AttributeEndEntities,
            0,
            Evidence::OcspResponse,
            _,
            true
        ))
    ));
    let said = Words(&stating).to_string();
    assert!(said.contains("noRevAvail"), "{said}");
    assert!(
        said.contains("position 0 of the attribute certificate's chain"),
        "{said}"
    );
    let control = under(&held, AC, carried, &ocsp, &[], &[]);
    assert!(matches!(unanswered(&control), Some((_, 0, _, _, false))));
    assert!(!Words(&control).to_string().contains("noRevAvail"));
}

/// An outcome's words, as a finding's sentence carries them.
struct Words<'a>(&'a Outcome);

impl std::fmt::Display for Words<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.words(f)
    }
}

#[test]
fn the_reader_keeps_whether_no_revocation_is_available() {
    // RFC 5755 section 4.3.6's `noRevAvail`, `id-ce 56`, read as present on the certificate that
    // carries it and absent on one that does not.
    let read = |text: &str| {
        let bytes = hex(text);
        crate::attribute_certificate::read(crate::policy_rules::certification_path::tests::value(
            &bytes,
        ))
        .map(|certificate| certificate.no_revocation)
    };
    assert_eq!(read(AC_NO_REVOCATION), Ok(true));
    assert_eq!(read(AC), Ok(false));
}
