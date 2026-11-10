//! TS 119 172-2 clause 4.23.3.1 item 2's retention of expired revocation notices, answered by RFC
//! 6960 section 4.4.4's archive cutoff, over a P-256 CA and a signer it issued, minted for this
//! test with OCSP responses that state cutoffs and responses that do not. No corpus signature
//! states a machine-readable policy (ADR 1709), so every requirement here is constructed, and each
//! expected value is read off the clause the test names. Every positive has its negative beside it,
//! changing one thing (trap 13).

use std::borrow::Cow;

use super::{Requirement, RetentionCheck, RetentionFound, retention};
use crate::policy_rules::Verdict;
use crate::policy_rules::certification_path::tests::{cms, hex};
use crate::policy_rules::certification_path::{Path, TrustPoint, TrustTrees, search};
use crate::policy_rules::duration::Duration;
use crate::policy_rules::tests::facts;
use crate::revocation::{Material, StatedRetention, Undetermined};
use crate::x509::Instant;

/// 2027-01-01T00:00:00Z: inside both certificates' validity, and every response's `producedAt`.
pub(crate) const AT: Instant = Instant::from_unix_seconds(1_798_761_600);
/// 2017-01-01T00:00:00Z, the ten-year cutoff.
const TEN_YEARS_BACK: Instant = Instant::from_unix_seconds(1_483_228_800);
/// 2026-01-01T00:00:00Z, the one-year cutoff.
const ONE_YEAR_BACK: Instant = Instant::from_unix_seconds(1_767_225_600);

pub(crate) mod fixtures {
    //! Minted once with Python's `cryptography` package — keys, certificates, and each response's
    //! `ResponseData` assembled octet by octet and signed with ECDSA over SHA-256 — and pasted in;
    //! `openssl ocsp -resp_text` reads each response and prints its archive cutoff.

    /// The self-signed P-256 CA, `keyCertSign` and `cRLSign`, 2026 to 2036: the trust point.
    pub(crate) const CA: &str = "\
        308201783082011da003020102020130300a06082a8648ce3d04030230233121\
        301f06035504030c1871756f72726120726574656e74696f6e20746573742043\
        41301e170d3236303130313030303030305a170d333630313031303030303030\
        5a30233121301f06035504030c1871756f72726120726574656e74696f6e2074\
        6573742043413059301306072a8648ce3d020106082a8648ce3d030107034200\
        0468ee676bd2dffc9c92b56ec95d0407204b45431d22f5c5f098570d757e0a77\
        7edadd5c8539f662d8f58bd51b3c8208961654b56f56f0f6fab8854598d068bc\
        7aa3423040300f0603551d130101ff040530030101ff300e0603551d0f0101ff\
        040403020106301d0603551d0e041604146b41e2db47a5c463f48adec5db67d6\
        7aac8d2e1b300a06082a8648ce3d04030203490030460221009fb35da05cff55\
        769188731b9781bf00f8c5d18414cffbb80c6beec1a396ab04022100b3b4bf54\
        bfad000e1f6f421dbed5fa0bbf178d2789b4e6af58f791804788a0f3";

    /// The signer the CA issued, serial `31`, 2026 to 2030.
    pub(crate) const SIGNER: &str = "\
        3082016c30820112a003020102020131300a06082a8648ce3d04030230233121\
        301f06035504030c1871756f72726120726574656e74696f6e20746573742043\
        41301e170d3236303130313030303030305a170d333030313031303030303030\
        5a30273125302306035504030c1c71756f72726120726574656e74696f6e2074\
        657374207369676e65723059301306072a8648ce3d020106082a8648ce3d0301\
        070342000439e2e999e5dacc3bbbbc2cc3324c51a031977eec8e513ba23d3da4\
        57702b61874012ee26cea04bed6aefbf15fcb0b54a37c2470335263ab24b5b15\
        447db250c5a3333031300e0603551d0f0101ff0404030206c0301f0603551d23\
        0418301680146b41e2db47a5c463f48adec5db67d67aac8d2e1b300a06082a86\
        48ce3d04030203480030450220096d01eb6610c24ff0c5163f5caea0a1315f31\
        6549368f4b1dfd0932da020c80022100d1be390eaef792669bcd84e9d359c1a1\
        9525245c11e753f387db5ed4fafa9411";

    /// The CA's own response for the signer, `good`, produced at 2027-01-01T00:00:00Z, with an archive
    /// cutoff of 2017-01-01T00:00:00Z: a retention interval of ten years.
    pub(crate) const OCSP_TEN_YEARS: &str = "\
        308201360a0100a082012f3082012b06092b06010505073001010482011c3082\
        01183081bea21604146b41e2db47a5c463f48adec5db67d67aac8d2e1b180f32\
        303237303130313030303030305a30819230818f3056300d0609608648016503\
        040201050004208ecf99df89598e06d8c4e6a95d2c6fbd6ec27cf7a1f7f60778\
        b470b5e968be060420a0c2d9a71669be30f3b2d539fea6fec35b666bb07d8df7\
        ae689c592c0093fd040201318000180f32303237303130313030303030305aa1\
        223020301e06092b06010505073001060411180f323031373031303130303030\
        30305a300a06082a8648ce3d0403020349003046022100a7ec1034ab74a2a8b7\
        a4a4df8e55e3a354056e2c8a81b5e698529f07a99399e2022100b83905e89682\
        784562aaf1dd01df746596f746a16d05855486922518461988ff";

    /// The same, with a cutoff of 2026-01-01T00:00:00Z: one year, 31 536 000 seconds.
    pub(crate) const OCSP_ONE_YEAR: &str = "\
        308201350a0100a082012e3082012a06092b06010505073001010482011b3082\
        01173081bea21604146b41e2db47a5c463f48adec5db67d67aac8d2e1b180f32\
        303237303130313030303030305a30819230818f3056300d0609608648016503\
        040201050004208ecf99df89598e06d8c4e6a95d2c6fbd6ec27cf7a1f7f60778\
        b470b5e968be060420a0c2d9a71669be30f3b2d539fea6fec35b666bb07d8df7\
        ae689c592c0093fd040201318000180f32303237303130313030303030305aa1\
        223020301e06092b06010505073001060411180f323032363031303130303030\
        30305a300a06082a8648ce3d0403020348003045022100e29e6df4583ad035d6\
        980a99e4503645b90e62a805d81441404d77e6f4c82e5602203d0a5380132eff\
        73330c35fc49d87b849853c5e4722b89c1248403ff3c5ffec9";

    /// The same, with no `singleExtensions`.
    pub(crate) const OCSP_NO_CUTOFF: &str = "\
        3082010d0a0100a08201063082010206092b06010505073001010481f43081f1\
        308198a21604146b41e2db47a5c463f48adec5db67d67aac8d2e1b180f323032\
        37303130313030303030305a306d306b3056300d060960864801650304020105\
        0004208ecf99df89598e06d8c4e6a95d2c6fbd6ec27cf7a1f7f60778b470b5e9\
        68be060420a0c2d9a71669be30f3b2d539fea6fec35b666bb07d8df7ae689c59\
        2c0093fd040201318000180f32303237303130313030303030305a300a06082a\
        8648ce3d0403020348003045022100d4b606224ca76398beff806a78460c527a\
        d7222d08475d71c2f158e8dd05b17e0220561eb701bb641dbe3244f1996e5a72\
        7f8bbb02aacfd9d800b950c1969cb4ad95";

    /// The ten-year response signed by the signer's own key, which no criterion of RFC 6960
    /// section 4.2.2.2 authorises, and carrying no certificate.
    pub(crate) const OCSP_TEN_YEARS_BY_SIGNER: &str = "\
        308201360a0100a082012f3082012b06092b06010505073001010482011c3082\
        01183081bea2160414bcb2de3892c1c87180afe326081bf2c4a76d496a180f32\
        303237303130313030303030305a30819230818f3056300d0609608648016503\
        040201050004208ecf99df89598e06d8c4e6a95d2c6fbd6ec27cf7a1f7f60778\
        b470b5e968be060420a0c2d9a71669be30f3b2d539fea6fec35b666bb07d8df7\
        ae689c592c0093fd040201318000180f32303237303130313030303030305aa1\
        223020301e06092b06010505073001060411180f323031373031303130303030\
        30305a300a06082a8648ce3d0403020349003046022100c46a63338ccee96704\
        53a9966761bea186eeea9159456af6ac21aa91aa4209600221008a0166f74e88\
        e0317379659f4c42a12fa725f71663a207ae71cbb3b2bc8ff0ca";

    /// The ten-year cutoff written as a `UTCTime`, which is not the extension's syntax.
    pub(crate) const OCSP_UTC_CUTOFF: &str = "\
        308201320a0100a082012b3082012706092b0601050507300101048201183082\
        01143081bca21604146b41e2db47a5c463f48adec5db67d67aac8d2e1b180f32\
        303237303130313030303030305a30819030818d3056300d0609608648016503\
        040201050004208ecf99df89598e06d8c4e6a95d2c6fbd6ec27cf7a1f7f60778\
        b470b5e968be060420a0c2d9a71669be30f3b2d539fea6fec35b666bb07d8df7\
        ae689c592c0093fd040201318000180f32303237303130313030303030305aa1\
        20301e301c06092b0601050507300106040f170d313730313031303030303030\
        5a300a06082a8648ce3d0403020347003044022001321971cae6cb7d40bfcf14\
        05a2a006a54cb17926525d2ae6e5363125ac9ee202204a081c281c533337120c\
        cdbc25d927e4f5a93f92b61496bba23feb5f701b47dc";
}

fn trees(point: &[u8]) -> TrustTrees<'_> {
    TrustTrees {
        points: vec![TrustPoint {
            certificate: Cow::Borrowed(point),
            path_length: None,
            reliable_until: None,
            inputs: crate::trust::PathInputs::default(),
            unprocessed: Vec::new(),
            mandated_path: None,
        }],
        trusted_lists: 0,
    }
}

/// The signer's path to the CA, validated at [`AT`].
fn path() -> Path {
    let (ca, signer) = (hex(fixtures::CA), hex(fixtures::SIGNER));
    search(
        &trees(&ca),
        &cms(&signer, &ca),
        &facts(None).validated_at(AT),
    )
    .path
    .expect("the signer's path validates to the CA")
}

fn requiring(bound: &str) -> Requirement {
    Requirement {
        retention: Some(Duration::parse(bound).expect("an xs:duration")),
        ..Requirement::none()
    }
}

fn checked(bound: &str, responses: &[&str]) -> RetentionCheck {
    let held: Vec<Vec<u8>> = responses.iter().map(|text| hex(text)).collect();
    let slices: Vec<&[u8]> = held.iter().map(Vec::as_slice).collect();
    let material = Material::read(&[], &slices);
    retention(&requiring(bound), Some(&path()), &material).expect("a retention is mandated")
}

const fn stated(archive_cutoff: Instant) -> StatedRetention {
    StatedRetention {
        produced_at: AT,
        archive_cutoff,
    }
}

/// Clause 4.23.3.1 item 2 asks for a period *exceeding* the bound, and RFC 6960 section 4.4.4
/// makes `producedAt` less the cutoff the responder's retention interval: ten years exceeds nine,
/// and a year does not exceed a year.
#[test]
fn a_stated_retention_exceeding_the_bound_meets_it_and_an_equal_one_does_not() {
    let ten = checked("P9Y", &[fixtures::OCSP_TEN_YEARS]);
    assert_eq!(ten.found, RetentionFound::Exceeds(stated(TEN_YEARS_BACK)));
    assert_eq!(ten.verdict(), Verdict::Met);
    let one = checked("P1Y", &[fixtures::OCSP_ONE_YEAR]);
    assert_eq!(one.found, RetentionFound::Short(stated(ONE_YEAR_BACK)));
    assert_eq!(
        one.verdict(),
        Verdict::Broken,
        "the issuer's authorised responder states a retention of one year, which does not exceed one year"
    );
    assert_eq!(
        checked("P11M", &[fixtures::OCSP_ONE_YEAR]).verdict(),
        Verdict::Met
    );
}

/// XML Schema Part 2 section 3.2.6.1 lets the seconds carry a fraction: one year is 31 536 000
/// seconds, which exceeds 31 535 999.5 and 31 535 999, and exceeds neither 31 536 000 nor
/// 31 536 000.5.
#[test]
fn a_fractional_bound_is_compared_exactly() {
    let verdict = |bound| checked(bound, &[fixtures::OCSP_ONE_YEAR]).verdict();
    assert_eq!(verdict("PT31535999.5S"), Verdict::Met);
    assert_eq!(verdict("PT31535999S"), Verdict::Met);
    assert_eq!(verdict("PT31536000S"), Verdict::Broken);
    assert_eq!(verdict("PT31536000.5S"), Verdict::Broken);
}

/// One authorised statement that exceeds the bound meets it whatever the others state, and the
/// longest stated is the one a short finding names.
#[test]
fn the_longest_stated_retention_decides() {
    let both = checked("P5Y", &[fixtures::OCSP_ONE_YEAR, fixtures::OCSP_TEN_YEARS]);
    assert_eq!(both.found, RetentionFound::Exceeds(stated(TEN_YEARS_BACK)));
    let short = checked("P20Y", &[fixtures::OCSP_ONE_YEAR, fixtures::OCSP_TEN_YEARS]);
    assert_eq!(short.found, RetentionFound::Short(stated(TEN_YEARS_BACK)));
    assert_eq!(short.verdict(), Verdict::Broken);
}

/// Where nothing in the file states the issuer's retention, the requirement is not evaluated:
/// absent evidence of a long retention is not evidence of a short one.
#[test]
fn a_retention_nothing_authorised_states_is_not_evaluated() {
    let unstated = |responses: &[&str]| checked("P1Y", responses).found;
    assert_eq!(
        unstated(&[fixtures::OCSP_NO_CUTOFF]),
        RetentionFound::Unstated(Undetermined::NoArchiveCutoff),
        "an authorised response with no cutoff says nothing about retention"
    );
    assert_eq!(
        unstated(&[fixtures::OCSP_UTC_CUTOFF]),
        RetentionFound::Unstated(Undetermined::NoArchiveCutoff),
        "RFC 6960 section 4.4.4 fixes the extension's syntax as GeneralizedTime"
    );
    assert_eq!(
        unstated(&[fixtures::OCSP_TEN_YEARS_BY_SIGNER]),
        RetentionFound::Unstated(Undetermined::ResponderNotAuthorised),
        "RFC 6960 section 4.2.2.2: a response neither the CA nor its delegate signed speaks for nobody"
    );
    assert_eq!(
        unstated(&[]),
        RetentionFound::Unstated(Undetermined::NotCovered)
    );
    assert_eq!(
        checked("P1Y", &[fixtures::OCSP_NO_CUTOFF]).verdict(),
        Verdict::NotEvaluated
    );
    let ten = hex(fixtures::OCSP_TEN_YEARS);
    let material = Material::read(&[], &[&ten]);
    assert_eq!(
        retention(&requiring("P1Y"), None, &material).map(|check| check.found),
        Some(RetentionFound::NoPath),
        "with no path the signing certificate's issuer is not known"
    );
    assert_eq!(
        retention(&Requirement::none(), Some(&path()), &material),
        None,
        "the constraint is not mandated where the member is absent"
    );
}
