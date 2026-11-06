//! NIST's Public Key Interoperability Test Suite, sections 4.8 to 4.13: certificate policies,
//! require explicit policy, policy mappings, inhibit policy mapping, inhibit anyPolicy and name
//! constraints — every case of the six, each a test named by its PKITS number.
//!
//! The certificates in `pkits/` are PKITS's own (the 2011 data set, `PKITS_data.zip` at
//! <https://csrc.nist.gov/projects/pki-testing>), copied unchanged, the trust anchor
//! `TrustAnchorRootCertificate.crt` among them. Each case's path, initial inputs and expected
//! result are the ones `PKITS.pdf` states for it, and PKITS states them from RFC 5280 section 6.1
//! (ADR 2725). Where PKITS states the authorities- or user-constrained policy set, it is compared
//! too; where it says only "not empty" or nothing, only the verdict is.
//!
//! The paths are validated without revocation material: PKITS's CRLs answer section 6.3's question,
//! which these six sections do not ask, and [`super::Revocation::NotChecked`] leaves the verdict to
//! the steps under test.

use super::{PathInputs, Purpose, Trust, TrustAnchors, policies::Oid, validate_constrained};
use crate::revocation::Material;
use crate::x509::{self, Instant};

/// One of a case's runs: section 6.1.1's inputs and what PKITS expects of them.
#[expect(
    clippy::struct_excessive_bools,
    reason = "the four are section 6.1.1's own boolean inputs and PKITS's verdict, as PKITS states them"
)]
struct Run {
    policy_set: Option<&'static [&'static [u8]]>,
    explicit: bool,
    mapping_inhibit: bool,
    any_inhibit: bool,
    valid: bool,
    authorities: Option<&'static [&'static [u8]]>,
    user: Option<&'static [&'static [u8]]>,
}

/// 2020-01-01T00:00:00Z, inside every PKITS 2011 certificate's validity.
const AT: Instant = Instant::from_unix_seconds(1_577_836_800);

fn certificate(name: &str) -> Vec<u8> {
    let path = format!("{}/src/trust/pkits/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
}

fn sorted(set: &[&[u8]]) -> Vec<Oid> {
    let mut set: Vec<Oid> = set.iter().map(|oid| oid.to_vec()).collect();
    set.sort();
    set
}

/// Validates `path` (anchor side first, end entity last) under each run and compares with PKITS.
fn run(id: &str, path: &[&str], runs: &[Run]) {
    let anchor_bytes = certificate("TrustAnchorRootCertificate.crt");
    let anchor = x509::parse(&anchor_bytes).expect("PKITS's anchor reads");
    let held: Vec<Vec<u8>> = path.iter().map(|name| certificate(name)).collect();
    let read: Vec<_> = held
        .iter()
        .map(|bytes| x509::parse(bytes).expect("a PKITS certificate reads"))
        .collect();
    let (target, others) = read.split_last().expect("a PKITS path is not empty");
    let anchors = TrustAnchors::of(std::slice::from_ref(&anchor));
    let material = Material::none();
    for (number, expected) in (1_usize..).zip(runs) {
        let flag = |set: bool| set.then_some(0);
        let inputs = PathInputs {
            policy_set: expected
                .policy_set
                .map(|set| set.iter().map(|oid| oid.to_vec()).collect()),
            require_explicit_policy: flag(expected.explicit),
            inhibit_policy_mapping: flag(expected.mapping_inhibit),
            inhibit_any_policy: flag(expected.any_inhibit),
            name_constraints: None,
        };
        let validation = validate_constrained(
            target,
            others,
            &anchors,
            &material,
            AT,
            Purpose::Unstated,
            &inputs,
        );
        let anchored = matches!(validation.trust, Trust::Anchored { .. });
        assert_eq!(
            anchored, expected.valid,
            "PKITS {id} run {number}: {:?}",
            validation.trust
        );
        if !anchored {
            continue;
        }
        let mut authorities = validation.policies.authorities_constrained.clone();
        authorities.sort();
        let mut user = validation.policies.user_constrained.clone();
        user.sort();
        if let Some(wanted) = expected.authorities {
            assert_eq!(
                authorities,
                sorted(wanted),
                "PKITS {id} run {number}: authorities-constrained set"
            );
        }
        if let Some(wanted) = expected.user {
            assert_eq!(
                user,
                sorted(wanted),
                "PKITS {id} run {number}: user-constrained set"
            );
        }
    }
}

/// PKITS 4.8.1, "`All Certificates Same Policy Test1`".
#[test]
fn pkits_4_8_1_all_certificates_same_policy_test1() {
    run(
        "4.8.1",
        &["GoodCACert.crt", "ValidCertificatePathTest1EE.crt"],
        &[
            Run {
                policy_set: None,
                explicit: true,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: true,
                authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                user: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
            },
            Run {
                policy_set: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                explicit: true,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: true,
                authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                user: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
            },
            Run {
                policy_set: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x02]]),
                explicit: true,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: false,
                authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                user: Some(&[]),
            },
            Run {
                policy_set: Some(&[
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01],
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x02],
                ]),
                explicit: true,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: true,
                authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                user: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
            },
        ],
    );
}

/// PKITS 4.8.2, "`All Certificates No Policies Test2`".
#[test]
fn pkits_4_8_2_all_certificates_no_policies_test2() {
    run(
        "4.8.2",
        &[
            "NoPoliciesCACert.crt",
            "AllCertificatesNoPoliciesTest2EE.crt",
        ],
        &[
            Run {
                policy_set: None,
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: true,
                authorities: Some(&[]),
                user: Some(&[]),
            },
            Run {
                policy_set: None,
                explicit: true,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: false,
                authorities: Some(&[]),
                user: Some(&[]),
            },
        ],
    );
}

/// PKITS 4.8.3, "`Different Policies Test3`".
#[test]
fn pkits_4_8_3_different_policies_test3() {
    run(
        "4.8.3",
        &[
            "GoodCACert.crt",
            "PoliciesP2subCACert.crt",
            "DifferentPoliciesTest3EE.crt",
        ],
        &[
            Run {
                policy_set: None,
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: true,
                authorities: Some(&[]),
                user: Some(&[]),
            },
            Run {
                policy_set: None,
                explicit: true,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: false,
                authorities: Some(&[]),
                user: Some(&[]),
            },
            Run {
                policy_set: Some(&[
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01],
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x02],
                ]),
                explicit: true,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: false,
                authorities: Some(&[]),
                user: Some(&[]),
            },
        ],
    );
}

/// PKITS 4.8.4, "`Different Policies Test4`".
#[test]
fn pkits_4_8_4_different_policies_test4() {
    run(
        "4.8.4",
        &[
            "GoodCACert.crt",
            "GoodsubCACert.crt",
            "DifferentPoliciesTest4EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: Some(&[]),
            user: Some(&[]),
        }],
    );
}

/// PKITS 4.8.5, "`Different Policies Test5`".
#[test]
fn pkits_4_8_5_different_policies_test5() {
    run(
        "4.8.5",
        &[
            "GoodCACert.crt",
            "PoliciesP2subCA2Cert.crt",
            "DifferentPoliciesTest5EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: Some(&[]),
            user: Some(&[]),
        }],
    );
}

/// PKITS 4.8.6, "`Overlapping Policies Test6`".
#[test]
fn pkits_4_8_6_overlapping_policies_test6() {
    run(
        "4.8.6",
        &[
            "PoliciesP1234CACert.crt",
            "PoliciesP1234subCAP123Cert.crt",
            "PoliciesP1234subsubCAP123P12Cert.crt",
            "OverlappingPoliciesTest6EE.crt",
        ],
        &[
            Run {
                policy_set: None,
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: true,
                authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                user: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
            },
            Run {
                policy_set: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: true,
                authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                user: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
            },
            Run {
                policy_set: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x02]]),
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: false,
                authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                user: Some(&[]),
            },
        ],
    );
}

/// PKITS 4.8.7, "`Different Policies Test7`".
#[test]
fn pkits_4_8_7_different_policies_test7() {
    run(
        "4.8.7",
        &[
            "PoliciesP123CACert.crt",
            "PoliciesP123subCAP12Cert.crt",
            "PoliciesP123subsubCAP12P1Cert.crt",
            "DifferentPoliciesTest7EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: Some(&[]),
            user: Some(&[]),
        }],
    );
}

/// PKITS 4.8.8, "`Different Policies Test8`".
#[test]
fn pkits_4_8_8_different_policies_test8() {
    run(
        "4.8.8",
        &[
            "PoliciesP12CACert.crt",
            "PoliciesP12subCAP1Cert.crt",
            "PoliciesP12subsubCAP1P2Cert.crt",
            "DifferentPoliciesTest8EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: Some(&[]),
            user: Some(&[]),
        }],
    );
}

/// PKITS 4.8.9, "`Different Policies Test9`".
#[test]
fn pkits_4_8_9_different_policies_test9() {
    run(
        "4.8.9",
        &[
            "PoliciesP123CACert.crt",
            "PoliciesP123subCAP12Cert.crt",
            "PoliciesP123subsubCAP12P2Cert.crt",
            "PoliciesP123subsubsubCAP12P2P1Cert.crt",
            "DifferentPoliciesTest9EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: Some(&[]),
            user: Some(&[]),
        }],
    );
}

/// PKITS 4.8.10, "`All Certificates Same Policies Test10`".
#[test]
fn pkits_4_8_10_all_certificates_same_policies_test10() {
    run(
        "4.8.10",
        &[
            "PoliciesP12CACert.crt",
            "AllCertificatesSamePoliciesTest10EE.crt",
        ],
        &[
            Run {
                policy_set: None,
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: true,
                authorities: Some(&[
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01],
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x02],
                ]),
                user: None,
            },
            Run {
                policy_set: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: true,
                authorities: Some(&[
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01],
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x02],
                ]),
                user: None,
            },
            Run {
                policy_set: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x02]]),
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: true,
                authorities: Some(&[
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01],
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x02],
                ]),
                user: None,
            },
        ],
    );
}

/// PKITS 4.8.11, "`All Certificates AnyPolicy Test11`".
#[test]
fn pkits_4_8_11_all_certificates_anypolicy_test11() {
    run(
        "4.8.11",
        &[
            "anyPolicyCACert.crt",
            "AllCertificatesanyPolicyTest11EE.crt",
        ],
        &[
            Run {
                policy_set: None,
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: true,
                authorities: Some(&[&[0x55, 0x1D, 0x20, 0x00]]),
                user: Some(&[&[0x55, 0x1D, 0x20, 0x00]]),
            },
            Run {
                policy_set: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: true,
                authorities: Some(&[&[0x55, 0x1D, 0x20, 0x00]]),
                user: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
            },
        ],
    );
}

/// PKITS 4.8.12, "`Different Policies Test12`".
#[test]
fn pkits_4_8_12_different_policies_test12() {
    run(
        "4.8.12",
        &["PoliciesP3CACert.crt", "DifferentPoliciesTest12EE.crt"],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: Some(&[]),
            user: Some(&[]),
        }],
    );
}

/// PKITS 4.8.13, "`All Certificates Same Policies Test13`".
#[test]
fn pkits_4_8_13_all_certificates_same_policies_test13() {
    run(
        "4.8.13",
        &[
            "PoliciesP123CACert.crt",
            "AllCertificatesSamePoliciesTest13EE.crt",
        ],
        &[
            Run {
                policy_set: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: true,
                authorities: Some(&[
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01],
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x02],
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x03],
                ]),
                user: None,
            },
            Run {
                policy_set: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x02]]),
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: true,
                authorities: Some(&[
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01],
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x02],
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x03],
                ]),
                user: None,
            },
            Run {
                policy_set: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x03]]),
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: true,
                authorities: Some(&[
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01],
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x02],
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x03],
                ]),
                user: None,
            },
        ],
    );
}

/// PKITS 4.8.14, "`AnyPolicy Test14`".
#[test]
fn pkits_4_8_14_anypolicy_test14() {
    run(
        "4.8.14",
        &["anyPolicyCACert.crt", "AnyPolicyTest14EE.crt"],
        &[
            Run {
                policy_set: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: true,
                authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                user: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
            },
            Run {
                policy_set: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x02]]),
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: false,
                authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                user: Some(&[]),
            },
        ],
    );
}

/// PKITS 4.8.15, "`User Notice Qualifier Test15`".
#[test]
fn pkits_4_8_15_user_notice_qualifier_test15() {
    run(
        "4.8.15",
        &["UserNoticeQualifierTest15EE.crt"],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
            user: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
        }],
    );
}

/// PKITS 4.8.16, "`User Notice Qualifier Test16`".
#[test]
fn pkits_4_8_16_user_notice_qualifier_test16() {
    run(
        "4.8.16",
        &["GoodCACert.crt", "UserNoticeQualifierTest16EE.crt"],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
            user: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
        }],
    );
}

/// PKITS 4.8.17, "`User Notice Qualifier Test17`".
#[test]
fn pkits_4_8_17_user_notice_qualifier_test17() {
    run(
        "4.8.17",
        &["GoodCACert.crt", "UserNoticeQualifierTest17EE.crt"],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
            user: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
        }],
    );
}

/// PKITS 4.8.18, "`User Notice Qualifier Test18`".
#[test]
fn pkits_4_8_18_user_notice_qualifier_test18() {
    run(
        "4.8.18",
        &["PoliciesP12CACert.crt", "UserNoticeQualifierTest18EE.crt"],
        &[
            Run {
                policy_set: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: true,
                authorities: Some(&[
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01],
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x02],
                ]),
                user: None,
            },
            Run {
                policy_set: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x02]]),
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: true,
                authorities: Some(&[
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01],
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x02],
                ]),
                user: None,
            },
        ],
    );
}

/// PKITS 4.8.19, "`User Notice Qualifier Test19`".
#[test]
fn pkits_4_8_19_user_notice_qualifier_test19() {
    run(
        "4.8.19",
        &["UserNoticeQualifierTest19EE.crt"],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
            user: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
        }],
    );
}

/// PKITS 4.8.20, "`CPS Pointer Qualifier Test20`".
#[test]
fn pkits_4_8_20_cps_pointer_qualifier_test20() {
    run(
        "4.8.20",
        &["GoodCACert.crt", "CPSPointerQualifierTest20EE.crt"],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
            user: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
        }],
    );
}

/// PKITS 4.9.1, "`Valid RequireExplicitPolicy Test1`".
#[test]
fn pkits_4_9_1_valid_requireexplicitpolicy_test1() {
    run(
        "4.9.1",
        &[
            "requireExplicitPolicy10CACert.crt",
            "requireExplicitPolicy10subCACert.crt",
            "requireExplicitPolicy10subsubCACert.crt",
            "requireExplicitPolicy10subsubsubCACert.crt",
            "ValidrequireExplicitPolicyTest1EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.9.2, "`Valid RequireExplicitPolicy Test2`".
#[test]
fn pkits_4_9_2_valid_requireexplicitpolicy_test2() {
    run(
        "4.9.2",
        &[
            "requireExplicitPolicy5CACert.crt",
            "requireExplicitPolicy5subCACert.crt",
            "requireExplicitPolicy5subsubCACert.crt",
            "requireExplicitPolicy5subsubsubCACert.crt",
            "ValidrequireExplicitPolicyTest2EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.9.3, "`Invalid RequireExplicitPolicy Test3`".
#[test]
fn pkits_4_9_3_invalid_requireexplicitpolicy_test3() {
    run(
        "4.9.3",
        &[
            "requireExplicitPolicy4CACert.crt",
            "requireExplicitPolicy4subCACert.crt",
            "requireExplicitPolicy4subsubCACert.crt",
            "requireExplicitPolicy4subsubsubCACert.crt",
            "InvalidrequireExplicitPolicyTest3EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: Some(&[]),
            user: None,
        }],
    );
}

/// PKITS 4.9.4, "`Valid RequireExplicitPolicy Test4`".
#[test]
fn pkits_4_9_4_valid_requireexplicitpolicy_test4() {
    run(
        "4.9.4",
        &[
            "requireExplicitPolicy0CACert.crt",
            "requireExplicitPolicy0subCACert.crt",
            "requireExplicitPolicy0subsubCACert.crt",
            "requireExplicitPolicy0subsubsubCACert.crt",
            "ValidrequireExplicitPolicyTest4EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.9.5, "`Invalid RequireExplicitPolicy Test5`".
#[test]
fn pkits_4_9_5_invalid_requireexplicitpolicy_test5() {
    run(
        "4.9.5",
        &[
            "requireExplicitPolicy7CACert.crt",
            "requireExplicitPolicy7subCARE2Cert.crt",
            "requireExplicitPolicy7subsubCARE2RE4Cert.crt",
            "requireExplicitPolicy7subsubsubCARE2RE4Cert.crt",
            "InvalidrequireExplicitPolicyTest5EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: Some(&[]),
            user: None,
        }],
    );
}

/// PKITS 4.9.6, "`Valid Self-Issued requireExplicitPolicy Test6`".
#[test]
fn pkits_4_9_6_valid_self_issued_requireexplicitpolicy_test6() {
    run(
        "4.9.6",
        &[
            "requireExplicitPolicy2CACert.crt",
            "requireExplicitPolicy2SelfIssuedCACert.crt",
            "ValidSelfIssuedrequireExplicitPolicyTest6EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.9.7, "`Invalid Self-Issued requireExplicitPolicy Test7`".
#[test]
fn pkits_4_9_7_invalid_self_issued_requireexplicitpolicy_test7() {
    run(
        "4.9.7",
        &[
            "requireExplicitPolicy2CACert.crt",
            "requireExplicitPolicy2SelfIssuedCACert.crt",
            "requireExplicitPolicy2subCACert.crt",
            "InvalidSelfIssuedrequireExplicitPolicyTest7EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: Some(&[]),
            user: None,
        }],
    );
}

/// PKITS 4.9.8, "`Invalid Self-Issued requireExplicitPolicy Test8`".
#[test]
fn pkits_4_9_8_invalid_self_issued_requireexplicitpolicy_test8() {
    run(
        "4.9.8",
        &[
            "requireExplicitPolicy2CACert.crt",
            "requireExplicitPolicy2SelfIssuedCACert.crt",
            "requireExplicitPolicy2subCACert.crt",
            "requireExplicitPolicy2SelfIssuedsubCACert.crt",
            "InvalidSelfIssuedrequireExplicitPolicyTest8EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: Some(&[]),
            user: None,
        }],
    );
}

/// PKITS 4.10.1, "`Valid Policy Mapping Test1`".
#[test]
fn pkits_4_10_1_valid_policy_mapping_test1() {
    run(
        "4.10.1",
        &["Mapping1to2CACert.crt", "ValidPolicyMappingTest1EE.crt"],
        &[
            Run {
                policy_set: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: true,
                authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                user: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
            },
            Run {
                policy_set: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x02]]),
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: false,
                authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                user: Some(&[]),
            },
            Run {
                policy_set: None,
                explicit: false,
                mapping_inhibit: true,
                any_inhibit: false,
                valid: false,
                authorities: None,
                user: None,
            },
        ],
    );
}

/// PKITS 4.10.2, "`Invalid Policy Mapping Test2`".
#[test]
fn pkits_4_10_2_invalid_policy_mapping_test2() {
    run(
        "4.10.2",
        &["Mapping1to2CACert.crt", "InvalidPolicyMappingTest2EE.crt"],
        &[
            Run {
                policy_set: None,
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: false,
                authorities: Some(&[]),
                user: Some(&[]),
            },
            Run {
                policy_set: None,
                explicit: false,
                mapping_inhibit: true,
                any_inhibit: false,
                valid: false,
                authorities: Some(&[]),
                user: Some(&[]),
            },
        ],
    );
}

/// PKITS 4.10.3, "`Valid Policy Mapping Test3`".
#[test]
fn pkits_4_10_3_valid_policy_mapping_test3() {
    run(
        "4.10.3",
        &[
            "P12Mapping1to3CACert.crt",
            "P12Mapping1to3subCACert.crt",
            "P12Mapping1to3subsubCACert.crt",
            "ValidPolicyMappingTest3EE.crt",
        ],
        &[
            Run {
                policy_set: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: false,
                authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x02]]),
                user: Some(&[]),
            },
            Run {
                policy_set: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x02]]),
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: true,
                authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x02]]),
                user: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x02]]),
            },
        ],
    );
}

/// PKITS 4.10.4, "`Invalid Policy Mapping Test4`".
#[test]
fn pkits_4_10_4_invalid_policy_mapping_test4() {
    run(
        "4.10.4",
        &[
            "P12Mapping1to3CACert.crt",
            "P12Mapping1to3subCACert.crt",
            "P12Mapping1to3subsubCACert.crt",
            "InvalidPolicyMappingTest4EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: Some(&[]),
            user: Some(&[]),
        }],
    );
}

/// PKITS 4.10.5, "`Valid Policy Mapping Test5`".
#[test]
fn pkits_4_10_5_valid_policy_mapping_test5() {
    run(
        "4.10.5",
        &[
            "P1Mapping1to234CACert.crt",
            "P1Mapping1to234subCACert.crt",
            "ValidPolicyMappingTest5EE.crt",
        ],
        &[
            Run {
                policy_set: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: true,
                authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                user: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
            },
            Run {
                policy_set: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x06]]),
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: false,
                authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                user: Some(&[]),
            },
        ],
    );
}

/// PKITS 4.10.6, "`Valid Policy Mapping Test6`".
#[test]
fn pkits_4_10_6_valid_policy_mapping_test6() {
    run(
        "4.10.6",
        &[
            "P1Mapping1to234CACert.crt",
            "P1Mapping1to234subCACert.crt",
            "ValidPolicyMappingTest6EE.crt",
        ],
        &[
            Run {
                policy_set: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: true,
                authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                user: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
            },
            Run {
                policy_set: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x06]]),
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: false,
                authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                user: Some(&[]),
            },
        ],
    );
}

/// PKITS 4.10.7, "`Invalid Mapping From anyPolicy Test7`".
#[test]
fn pkits_4_10_7_invalid_mapping_from_anypolicy_test7() {
    run(
        "4.10.7",
        &[
            "MappingFromanyPolicyCACert.crt",
            "InvalidMappingFromanyPolicyTest7EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.10.8, "`Invalid Mapping To anyPolicy Test8`".
#[test]
fn pkits_4_10_8_invalid_mapping_to_anypolicy_test8() {
    run(
        "4.10.8",
        &[
            "MappingToanyPolicyCACert.crt",
            "InvalidMappingToanyPolicyTest8EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.10.9, "`Valid Policy Mapping Test9`".
#[test]
fn pkits_4_10_9_valid_policy_mapping_test9() {
    run(
        "4.10.9",
        &[
            "PanyPolicyMapping1to2CACert.crt",
            "ValidPolicyMappingTest9EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
            user: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
        }],
    );
}

/// PKITS 4.10.10, "`Invalid Policy Mapping Test10`".
#[test]
fn pkits_4_10_10_invalid_policy_mapping_test10() {
    run(
        "4.10.10",
        &[
            "GoodCACert.crt",
            "GoodsubCAPanyPolicyMapping1to2CACert.crt",
            "InvalidPolicyMappingTest10EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: Some(&[]),
            user: Some(&[]),
        }],
    );
}

/// PKITS 4.10.11, "`Valid Policy Mapping Test11`".
#[test]
fn pkits_4_10_11_valid_policy_mapping_test11() {
    run(
        "4.10.11",
        &[
            "GoodCACert.crt",
            "GoodsubCAPanyPolicyMapping1to2CACert.crt",
            "ValidPolicyMappingTest11EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
            user: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
        }],
    );
}

/// PKITS 4.10.12, "`Valid Policy Mapping Test12`".
#[test]
fn pkits_4_10_12_valid_policy_mapping_test12() {
    run(
        "4.10.12",
        &["P12Mapping1to3CACert.crt", "ValidPolicyMappingTest12EE.crt"],
        &[
            Run {
                policy_set: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: true,
                authorities: Some(&[
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01],
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x02],
                ]),
                user: None,
            },
            Run {
                policy_set: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x02]]),
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: true,
                authorities: Some(&[
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01],
                    &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x02],
                ]),
                user: None,
            },
        ],
    );
}

/// PKITS 4.10.13, "`Valid Policy Mapping Test13`".
#[test]
fn pkits_4_10_13_valid_policy_mapping_test13() {
    run(
        "4.10.13",
        &[
            "P1anyPolicyMapping1to2CACert.crt",
            "ValidPolicyMappingTest13EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
            user: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
        }],
    );
}

/// PKITS 4.10.14, "`Valid Policy Mapping Test14`".
#[test]
fn pkits_4_10_14_valid_policy_mapping_test14() {
    run(
        "4.10.14",
        &[
            "P1anyPolicyMapping1to2CACert.crt",
            "ValidPolicyMappingTest14EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
            user: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
        }],
    );
}

/// PKITS 4.11.1, "`Invalid inhibitPolicyMapping Test1`".
#[test]
fn pkits_4_11_1_invalid_inhibitpolicymapping_test1() {
    run(
        "4.11.1",
        &[
            "inhibitPolicyMapping0CACert.crt",
            "inhibitPolicyMapping0subCACert.crt",
            "InvalidinhibitPolicyMappingTest1EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: Some(&[]),
            user: Some(&[]),
        }],
    );
}

/// PKITS 4.11.2, "`Valid inhibitPolicyMapping Test2`".
#[test]
fn pkits_4_11_2_valid_inhibitpolicymapping_test2() {
    run(
        "4.11.2",
        &[
            "inhibitPolicyMapping1P12CACert.crt",
            "inhibitPolicyMapping1P12subCACert.crt",
            "ValidinhibitPolicyMappingTest2EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
            user: None,
        }],
    );
}

/// PKITS 4.11.3, "`Invalid inhibitPolicyMapping Test3`".
#[test]
fn pkits_4_11_3_invalid_inhibitpolicymapping_test3() {
    run(
        "4.11.3",
        &[
            "inhibitPolicyMapping1P12CACert.crt",
            "inhibitPolicyMapping1P12subCACert.crt",
            "inhibitPolicyMapping1P12subsubCACert.crt",
            "InvalidinhibitPolicyMappingTest3EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: Some(&[]),
            user: Some(&[]),
        }],
    );
}

/// PKITS 4.11.4, "`Valid inhibitPolicyMapping Test4`".
#[test]
fn pkits_4_11_4_valid_inhibitpolicymapping_test4() {
    run(
        "4.11.4",
        &[
            "inhibitPolicyMapping1P12CACert.crt",
            "inhibitPolicyMapping1P12subCACert.crt",
            "inhibitPolicyMapping1P12subsubCACert.crt",
            "ValidinhibitPolicyMappingTest4EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x02]]),
            user: None,
        }],
    );
}

/// PKITS 4.11.5, "`Invalid inhibitPolicyMapping Test5`".
#[test]
fn pkits_4_11_5_invalid_inhibitpolicymapping_test5() {
    run(
        "4.11.5",
        &[
            "inhibitPolicyMapping5CACert.crt",
            "inhibitPolicyMapping5subCACert.crt",
            "inhibitPolicyMapping5subsubCACert.crt",
            "inhibitPolicyMapping5subsubsubCACert.crt",
            "InvalidinhibitPolicyMappingTest5EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: Some(&[]),
            user: Some(&[]),
        }],
    );
}

/// PKITS 4.11.6, "`Invalid inhibitPolicyMapping Test6`".
#[test]
fn pkits_4_11_6_invalid_inhibitpolicymapping_test6() {
    run(
        "4.11.6",
        &[
            "inhibitPolicyMapping1P12CACert.crt",
            "inhibitPolicyMapping1P12subCAIPM5Cert.crt",
            "inhibitPolicyMapping1P12subsubCAIPM5Cert.crt",
            "InvalidinhibitPolicyMappingTest6EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: Some(&[]),
            user: Some(&[]),
        }],
    );
}

/// PKITS 4.11.7, "`Valid Self-Issued inhibitPolicyMapping Test7`".
#[test]
fn pkits_4_11_7_valid_self_issued_inhibitpolicymapping_test7() {
    run(
        "4.11.7",
        &[
            "inhibitPolicyMapping1P1CACert.crt",
            "inhibitPolicyMapping1P1SelfIssuedCACert.crt",
            "inhibitPolicyMapping1P1subCACert.crt",
            "ValidSelfIssuedinhibitPolicyMappingTest7EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
            user: None,
        }],
    );
}

/// PKITS 4.11.8, "`Invalid Self-Issued inhibitPolicyMapping Test8`".
#[test]
fn pkits_4_11_8_invalid_self_issued_inhibitpolicymapping_test8() {
    run(
        "4.11.8",
        &[
            "inhibitPolicyMapping1P1CACert.crt",
            "inhibitPolicyMapping1P1SelfIssuedCACert.crt",
            "inhibitPolicyMapping1P1subCACert.crt",
            "inhibitPolicyMapping1P1subsubCACert.crt",
            "InvalidSelfIssuedinhibitPolicyMappingTest8EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: Some(&[]),
            user: Some(&[]),
        }],
    );
}

/// PKITS 4.11.9, "`Invalid Self-Issued inhibitPolicyMapping Test9`".
#[test]
fn pkits_4_11_9_invalid_self_issued_inhibitpolicymapping_test9() {
    run(
        "4.11.9",
        &[
            "inhibitPolicyMapping1P1CACert.crt",
            "inhibitPolicyMapping1P1SelfIssuedCACert.crt",
            "inhibitPolicyMapping1P1subCACert.crt",
            "inhibitPolicyMapping1P1subsubCACert.crt",
            "InvalidSelfIssuedinhibitPolicyMappingTest9EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: Some(&[]),
            user: Some(&[]),
        }],
    );
}

/// PKITS 4.11.10, "`Invalid Self-Issued inhibitPolicyMapping Test10`".
#[test]
fn pkits_4_11_10_invalid_self_issued_inhibitpolicymapping_test10() {
    run(
        "4.11.10",
        &[
            "inhibitPolicyMapping1P1CACert.crt",
            "inhibitPolicyMapping1P1SelfIssuedCACert.crt",
            "inhibitPolicyMapping1P1subCACert.crt",
            "inhibitPolicyMapping1P1SelfIssuedsubCACert.crt",
            "InvalidSelfIssuedinhibitPolicyMappingTest10EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: Some(&[]),
            user: Some(&[]),
        }],
    );
}

/// PKITS 4.11.11, "`Invalid Self-Issued inhibitPolicyMapping Test11`".
#[test]
fn pkits_4_11_11_invalid_self_issued_inhibitpolicymapping_test11() {
    run(
        "4.11.11",
        &[
            "inhibitPolicyMapping1P1CACert.crt",
            "inhibitPolicyMapping1P1SelfIssuedCACert.crt",
            "inhibitPolicyMapping1P1subCACert.crt",
            "inhibitPolicyMapping1P1SelfIssuedsubCACert.crt",
            "InvalidSelfIssuedinhibitPolicyMappingTest11EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: Some(&[]),
            user: Some(&[]),
        }],
    );
}

/// PKITS 4.12.1, "`Invalid inhibitAnyPolicy Test1`".
#[test]
fn pkits_4_12_1_invalid_inhibitanypolicy_test1() {
    run(
        "4.12.1",
        &[
            "inhibitAnyPolicy0CACert.crt",
            "InvalidinhibitAnyPolicyTest1EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: Some(&[]),
            user: Some(&[]),
        }],
    );
}

/// PKITS 4.12.2, "`Valid inhibitAnyPolicy Test2`".
#[test]
fn pkits_4_12_2_valid_inhibitanypolicy_test2() {
    run(
        "4.12.2",
        &[
            "inhibitAnyPolicy0CACert.crt",
            "ValidinhibitAnyPolicyTest2EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
            user: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
        }],
    );
}

/// PKITS 4.12.3, "`inhibitAnyPolicy Test3`".
#[test]
fn pkits_4_12_3_inhibitanypolicy_test3() {
    run(
        "4.12.3",
        &[
            "inhibitAnyPolicy1CACert.crt",
            "inhibitAnyPolicy1subCA1Cert.crt",
            "inhibitAnyPolicyTest3EE.crt",
        ],
        &[
            Run {
                policy_set: None,
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: false,
                valid: true,
                authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
                user: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
            },
            Run {
                policy_set: None,
                explicit: false,
                mapping_inhibit: false,
                any_inhibit: true,
                valid: false,
                authorities: Some(&[]),
                user: Some(&[]),
            },
        ],
    );
}

/// PKITS 4.12.4, "`Invalid inhibitAnyPolicy Test4`".
#[test]
fn pkits_4_12_4_invalid_inhibitanypolicy_test4() {
    run(
        "4.12.4",
        &[
            "inhibitAnyPolicy1CACert.crt",
            "inhibitAnyPolicy1subCA1Cert.crt",
            "InvalidinhibitAnyPolicyTest4EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: Some(&[]),
            user: Some(&[]),
        }],
    );
}

/// PKITS 4.12.5, "`Invalid inhibitAnyPolicy Test5`".
#[test]
fn pkits_4_12_5_invalid_inhibitanypolicy_test5() {
    run(
        "4.12.5",
        &[
            "inhibitAnyPolicy5CACert.crt",
            "inhibitAnyPolicy5subCACert.crt",
            "inhibitAnyPolicy5subsubCACert.crt",
            "InvalidinhibitAnyPolicyTest5EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: Some(&[]),
            user: Some(&[]),
        }],
    );
}

/// PKITS 4.12.6, "`Invalid inhibitAnyPolicy Test6`".
#[test]
fn pkits_4_12_6_invalid_inhibitanypolicy_test6() {
    run(
        "4.12.6",
        &[
            "inhibitAnyPolicy1CACert.crt",
            "inhibitAnyPolicy1subCAIAP5Cert.crt",
            "InvalidinhibitAnyPolicyTest6EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: Some(&[]),
            user: Some(&[]),
        }],
    );
}

/// PKITS 4.12.7, "`Valid Self-Issued inhibitAnyPolicy Test7`".
#[test]
fn pkits_4_12_7_valid_self_issued_inhibitanypolicy_test7() {
    run(
        "4.12.7",
        &[
            "inhibitAnyPolicy1CACert.crt",
            "inhibitAnyPolicy1SelfIssuedCACert.crt",
            "inhibitAnyPolicy1subCA2Cert.crt",
            "ValidSelfIssuedinhibitAnyPolicyTest7EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
            user: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
        }],
    );
}

/// PKITS 4.12.8, "`Invalid Self-Issued inhibitAnyPolicy Test8`".
#[test]
fn pkits_4_12_8_invalid_self_issued_inhibitanypolicy_test8() {
    run(
        "4.12.8",
        &[
            "inhibitAnyPolicy1CACert.crt",
            "inhibitAnyPolicy1SelfIssuedCACert.crt",
            "inhibitAnyPolicy1subCA2Cert.crt",
            "inhibitAnyPolicy1subsubCA2Cert.crt",
            "InvalidSelfIssuedinhibitAnyPolicyTest8EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: Some(&[]),
            user: Some(&[]),
        }],
    );
}

/// PKITS 4.12.9, "`Valid Self-Issued inhibitAnyPolicy Test9`".
#[test]
fn pkits_4_12_9_valid_self_issued_inhibitanypolicy_test9() {
    run(
        "4.12.9",
        &[
            "inhibitAnyPolicy1CACert.crt",
            "inhibitAnyPolicy1SelfIssuedCACert.crt",
            "inhibitAnyPolicy1subCA2Cert.crt",
            "inhibitAnyPolicy1SelfIssuedsubCA2Cert.crt",
            "ValidSelfIssuedinhibitAnyPolicyTest9EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
            user: Some(&[&[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x02, 0x01, 0x30, 0x01]]),
        }],
    );
}

/// PKITS 4.12.10, "`Invalid Self-Issued inhibitAnyPolicy Test10`".
#[test]
fn pkits_4_12_10_invalid_self_issued_inhibitanypolicy_test10() {
    run(
        "4.12.10",
        &[
            "inhibitAnyPolicy1CACert.crt",
            "inhibitAnyPolicy1SelfIssuedCACert.crt",
            "inhibitAnyPolicy1subCA2Cert.crt",
            "InvalidSelfIssuedinhibitAnyPolicyTest10EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: Some(&[]),
            user: Some(&[]),
        }],
    );
}

/// PKITS 4.13.1, "`Valid DN nameConstraints Test1`".
#[test]
fn pkits_4_13_1_valid_dn_nameconstraints_test1() {
    run(
        "4.13.1",
        &[
            "nameConstraintsDN1CACert.crt",
            "ValidDNnameConstraintsTest1EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.2, "`Invalid DN nameConstraints Test2`".
#[test]
fn pkits_4_13_2_invalid_dn_nameconstraints_test2() {
    run(
        "4.13.2",
        &[
            "nameConstraintsDN1CACert.crt",
            "InvalidDNnameConstraintsTest2EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.3, "`Invalid DN nameConstraints Test3`".
#[test]
fn pkits_4_13_3_invalid_dn_nameconstraints_test3() {
    run(
        "4.13.3",
        &[
            "nameConstraintsDN1CACert.crt",
            "InvalidDNnameConstraintsTest3EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.4, "`Valid DN nameConstraints Test4`".
#[test]
fn pkits_4_13_4_valid_dn_nameconstraints_test4() {
    run(
        "4.13.4",
        &[
            "nameConstraintsDN1CACert.crt",
            "ValidDNnameConstraintsTest4EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.5, "`Valid DN nameConstraints Test5`".
#[test]
fn pkits_4_13_5_valid_dn_nameconstraints_test5() {
    run(
        "4.13.5",
        &[
            "nameConstraintsDN2CACert.crt",
            "ValidDNnameConstraintsTest5EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.6, "`Valid DN nameConstraints Test6`".
#[test]
fn pkits_4_13_6_valid_dn_nameconstraints_test6() {
    run(
        "4.13.6",
        &[
            "nameConstraintsDN3CACert.crt",
            "ValidDNnameConstraintsTest6EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.7, "`Invalid DN nameConstraints Test7`".
#[test]
fn pkits_4_13_7_invalid_dn_nameconstraints_test7() {
    run(
        "4.13.7",
        &[
            "nameConstraintsDN3CACert.crt",
            "InvalidDNnameConstraintsTest7EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.8, "`Invalid DN nameConstraints Test8`".
#[test]
fn pkits_4_13_8_invalid_dn_nameconstraints_test8() {
    run(
        "4.13.8",
        &[
            "nameConstraintsDN4CACert.crt",
            "InvalidDNnameConstraintsTest8EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.9, "`Invalid DN nameConstraints Test9`".
#[test]
fn pkits_4_13_9_invalid_dn_nameconstraints_test9() {
    run(
        "4.13.9",
        &[
            "nameConstraintsDN4CACert.crt",
            "InvalidDNnameConstraintsTest9EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.10, "`Invalid DN nameConstraints Test10`".
#[test]
fn pkits_4_13_10_invalid_dn_nameconstraints_test10() {
    run(
        "4.13.10",
        &[
            "nameConstraintsDN5CACert.crt",
            "InvalidDNnameConstraintsTest10EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.11, "`Valid DN nameConstraints Test11`".
#[test]
fn pkits_4_13_11_valid_dn_nameconstraints_test11() {
    run(
        "4.13.11",
        &[
            "nameConstraintsDN5CACert.crt",
            "ValidDNnameConstraintsTest11EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.12, "`Invalid DN nameConstraints Test12`".
#[test]
fn pkits_4_13_12_invalid_dn_nameconstraints_test12() {
    run(
        "4.13.12",
        &[
            "nameConstraintsDN1CACert.crt",
            "nameConstraintsDN1subCA1Cert.crt",
            "InvalidDNnameConstraintsTest12EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.13, "`Invalid DN nameConstraints Test13`".
#[test]
fn pkits_4_13_13_invalid_dn_nameconstraints_test13() {
    run(
        "4.13.13",
        &[
            "nameConstraintsDN1CACert.crt",
            "nameConstraintsDN1subCA2Cert.crt",
            "InvalidDNnameConstraintsTest13EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.14, "`Valid DN nameConstraints Test14`".
#[test]
fn pkits_4_13_14_valid_dn_nameconstraints_test14() {
    run(
        "4.13.14",
        &[
            "nameConstraintsDN1CACert.crt",
            "nameConstraintsDN1subCA2Cert.crt",
            "ValidDNnameConstraintsTest14EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.15, "`Invalid DN nameConstraints Test15`".
#[test]
fn pkits_4_13_15_invalid_dn_nameconstraints_test15() {
    run(
        "4.13.15",
        &[
            "nameConstraintsDN3CACert.crt",
            "nameConstraintsDN3subCA1Cert.crt",
            "InvalidDNnameConstraintsTest15EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.16, "`Invalid DN nameConstraints Test16`".
#[test]
fn pkits_4_13_16_invalid_dn_nameconstraints_test16() {
    run(
        "4.13.16",
        &[
            "nameConstraintsDN3CACert.crt",
            "nameConstraintsDN3subCA1Cert.crt",
            "InvalidDNnameConstraintsTest16EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.17, "`Invalid DN nameConstraints Test17`".
#[test]
fn pkits_4_13_17_invalid_dn_nameconstraints_test17() {
    run(
        "4.13.17",
        &[
            "nameConstraintsDN3CACert.crt",
            "nameConstraintsDN3subCA2Cert.crt",
            "InvalidDNnameConstraintsTest17EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.18, "`Valid DN nameConstraints Test18`".
#[test]
fn pkits_4_13_18_valid_dn_nameconstraints_test18() {
    run(
        "4.13.18",
        &[
            "nameConstraintsDN3CACert.crt",
            "nameConstraintsDN3subCA2Cert.crt",
            "ValidDNnameConstraintsTest18EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.19, "`Valid Self-Issued DN nameConstraints Test19`".
#[test]
fn pkits_4_13_19_valid_self_issued_dn_nameconstraints_test19() {
    run(
        "4.13.19",
        &[
            "nameConstraintsDN1CACert.crt",
            "nameConstraintsDN1SelfIssuedCACert.crt",
            "ValidDNnameConstraintsTest19EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.20, "`Invalid Self-Issued DN nameConstraints Test20`".
#[test]
fn pkits_4_13_20_invalid_self_issued_dn_nameconstraints_test20() {
    run(
        "4.13.20",
        &[
            "nameConstraintsDN1CACert.crt",
            "InvalidDNnameConstraintsTest20EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.21, "`Valid RFC822 nameConstraints Test21`".
#[test]
fn pkits_4_13_21_valid_rfc822_nameconstraints_test21() {
    run(
        "4.13.21",
        &[
            "nameConstraintsRFC822CA1Cert.crt",
            "ValidRFC822nameConstraintsTest21EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.22, "`Invalid RFC822 nameConstraints Test22`".
#[test]
fn pkits_4_13_22_invalid_rfc822_nameconstraints_test22() {
    run(
        "4.13.22",
        &[
            "nameConstraintsRFC822CA1Cert.crt",
            "InvalidRFC822nameConstraintsTest22EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.23, "`Valid RFC822 nameConstraints Test23`".
#[test]
fn pkits_4_13_23_valid_rfc822_nameconstraints_test23() {
    run(
        "4.13.23",
        &[
            "nameConstraintsRFC822CA2Cert.crt",
            "ValidRFC822nameConstraintsTest23EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.24, "`Invalid RFC822 nameConstraints Test24`".
#[test]
fn pkits_4_13_24_invalid_rfc822_nameconstraints_test24() {
    run(
        "4.13.24",
        &[
            "nameConstraintsRFC822CA2Cert.crt",
            "InvalidRFC822nameConstraintsTest24EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.25, "`Valid RFC822 nameConstraints Test25`".
#[test]
fn pkits_4_13_25_valid_rfc822_nameconstraints_test25() {
    run(
        "4.13.25",
        &[
            "nameConstraintsRFC822CA3Cert.crt",
            "ValidRFC822nameConstraintsTest25EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.26, "`Invalid RFC822 nameConstraints Test26`".
#[test]
fn pkits_4_13_26_invalid_rfc822_nameconstraints_test26() {
    run(
        "4.13.26",
        &[
            "nameConstraintsRFC822CA3Cert.crt",
            "InvalidRFC822nameConstraintsTest26EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.27, "`Valid DN and RFC822 nameConstraints Test27`".
#[test]
fn pkits_4_13_27_valid_dn_and_rfc822_nameconstraints_test27() {
    run(
        "4.13.27",
        &[
            "nameConstraintsDN1CACert.crt",
            "nameConstraintsDN1subCA3Cert.crt",
            "ValidDNandRFC822nameConstraintsTest27EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.28, "`Invalid DN and RFC822 nameConstraints Test28`".
#[test]
fn pkits_4_13_28_invalid_dn_and_rfc822_nameconstraints_test28() {
    run(
        "4.13.28",
        &[
            "nameConstraintsDN1CACert.crt",
            "nameConstraintsDN1subCA3Cert.crt",
            "InvalidDNandRFC822nameConstraintsTest28EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.29, "`Invalid DN and RFC822 nameConstraints Test29`".
#[test]
fn pkits_4_13_29_invalid_dn_and_rfc822_nameconstraints_test29() {
    run(
        "4.13.29",
        &[
            "nameConstraintsDN1CACert.crt",
            "nameConstraintsDN1subCA3Cert.crt",
            "InvalidDNandRFC822nameConstraintsTest29EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.30, "`Valid DNS nameConstraints Test30`".
#[test]
fn pkits_4_13_30_valid_dns_nameconstraints_test30() {
    run(
        "4.13.30",
        &[
            "nameConstraintsDNS1CACert.crt",
            "ValidDNSnameConstraintsTest30EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.31, "`Invalid DNS nameConstraints Test31`".
#[test]
fn pkits_4_13_31_invalid_dns_nameconstraints_test31() {
    run(
        "4.13.31",
        &[
            "nameConstraintsDNS1CACert.crt",
            "InvalidDNSnameConstraintsTest31EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.32, "`Valid DNS nameConstraints Test32`".
#[test]
fn pkits_4_13_32_valid_dns_nameconstraints_test32() {
    run(
        "4.13.32",
        &[
            "nameConstraintsDNS2CACert.crt",
            "ValidDNSnameConstraintsTest32EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.33, "`Invalid DNS nameConstraints Test33`".
#[test]
fn pkits_4_13_33_invalid_dns_nameconstraints_test33() {
    run(
        "4.13.33",
        &[
            "nameConstraintsDNS2CACert.crt",
            "InvalidDNSnameConstraintsTest33EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.34, "`Valid URI nameConstraints Test34`".
#[test]
fn pkits_4_13_34_valid_uri_nameconstraints_test34() {
    run(
        "4.13.34",
        &[
            "nameConstraintsURI1CACert.crt",
            "ValidURInameConstraintsTest34EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.35, "`Invalid URI nameConstraints Test35`".
#[test]
fn pkits_4_13_35_invalid_uri_nameconstraints_test35() {
    run(
        "4.13.35",
        &[
            "nameConstraintsURI1CACert.crt",
            "InvalidURInameConstraintsTest35EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.36, "`Valid URI nameConstraints Test36`".
#[test]
fn pkits_4_13_36_valid_uri_nameconstraints_test36() {
    run(
        "4.13.36",
        &[
            "nameConstraintsURI2CACert.crt",
            "ValidURInameConstraintsTest36EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: true,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.37, "`Invalid URI nameConstraints Test37`".
#[test]
fn pkits_4_13_37_invalid_uri_nameconstraints_test37() {
    run(
        "4.13.37",
        &[
            "nameConstraintsURI2CACert.crt",
            "InvalidURInameConstraintsTest37EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: None,
            user: None,
        }],
    );
}

/// PKITS 4.13.38, "`Invalid DNS nameConstraints Test38`".
#[test]
fn pkits_4_13_38_invalid_dns_nameconstraints_test38() {
    run(
        "4.13.38",
        &[
            "nameConstraintsDNS1CACert.crt",
            "InvalidDNSnameConstraintsTest38EE.crt",
        ],
        &[Run {
            policy_set: None,
            explicit: false,
            mapping_inhibit: false,
            any_inhibit: false,
            valid: false,
            authorities: None,
            user: None,
        }],
    );
}
