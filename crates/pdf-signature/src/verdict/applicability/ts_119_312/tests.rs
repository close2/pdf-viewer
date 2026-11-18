//! ETSI TS 119 312 V2.1.1's tables, pinned cell by cell against the document, and REQ-4.5-01 h)'s
//! assessment over them, one suite per rule with only that rule's member differing.
//!
//! Every expected row is read from the table it names; every expected issue follows from the row
//! the suite falls in and clause 3.4's reading of its class. Instants are seconds since the epoch.

use super::*;
use crate::ecdsa::Curve as EcCurve;

/// 2026-10-01T00:00:00Z.
const OCTOBER_2026: Instant = Instant::from_unix_seconds(1_790_812_800);
/// 2027-01-01T00:00:00Z, the first instant after an `L[2026]` mechanism's phase-out day.
const JANUARY_2027: Instant = Instant::from_unix_seconds(1_798_761_600);

/// One suite at one pair of instants.
fn assess(digest: Digest, family: Family, key_bits: usize, q: Option<usize>) -> Assessment {
    assess_at(digest, family, key_bits, q, OCTOBER_2026, OCTOBER_2026)
}

fn assess_at(
    digest: Digest,
    family: Family,
    key_bits: usize,
    subgroup_bits: Option<usize>,
    signed: Instant,
    now: Instant,
) -> Assessment {
    Assessment::of(&Asked {
        digest,
        family,
        key_bits,
        subgroup_bits,
        signed,
        now,
    })
}

/// Table 1, as printed: short name, references, R/L.
#[test]
fn table_1_is_transcribed() {
    let rows: Vec<_> = HASH_FUNCTIONS
        .iter()
        .map(|row| (row.name, row.reference, row.class))
        .collect();
    let r = Class::Recommended;
    let sha2 = "FIPS Publication 180-4 [1]";
    let sha3 = "FIPS Publication 202 [15]";
    assert_eq!(
        rows,
        [
            ("SHA-224", sha2, Class::legacy_until(2028)),
            ("SHA-256", sha2, r),
            ("SHA-384", sha2, r),
            ("SHA-512", sha2, r),
            ("SHA3-256", sha3, r),
            ("SHA3-384", sha3, r),
            ("SHA3-512", sha3, r),
        ]
    );
}

/// Table 2, as printed, with its notes 1 and 2 as conditions.
#[test]
fn table_2_is_transcribed() {
    let rows: Vec<_> = SIGNATURE_ALGORITHMS
        .iter()
        .map(|row| (row.name, row.reference, row.class, row.condition))
        .collect();
    let (r, none) = (Class::Recommended, Condition::None);
    assert_eq!(
        rows,
        [
            ("RSA-PKCS#1v1_5", "IETF RFC 8017 [3]", Class::LEGACY, none),
            ("RSA-PSS", "IETF RFC 8017 [3]", r, none),
            (
                "DSA",
                "FIPS Publication 186-5, ISO/IEC 14888-3 [4]",
                r,
                none
            ),
            ("ECDSA", "FIPS Publication 186-5 [2]", r, none),
            (
                "EdDSA",
                "IETF RFC 8032 [23]",
                r,
                Condition::ByThisDocumentAlone
            ),
            ("ML-DSA", "NIST FIPS Publication 204 [26]", r, none),
            ("SLH-DSA", "NIST FIPS Publication 205 [27]", r, none),
            ("EC-SDSA-opt", "ISO/IEC 14888-3 [4]", Class::LEGACY, none),
            ("LMS", "IETF RFC 8554 [28]", r, Condition::StateManaged),
            ("XMSS", "IETF RFC 8391 [29]", r, Condition::StateManaged),
        ]
    );
}

/// Table 3, as printed, the family carried down its rows; every curve this tree verifies on is
/// one of its rows.
#[test]
fn table_3_is_transcribed() {
    let rows: Vec<_> = CURVES
        .iter()
        .map(|row| (row.family, row.name, row.reference, row.class))
        .collect();
    let r = Class::Recommended;
    let (brainpool, nist) = ("IETF RFC 5639 [5]", "NIST Special Publication 800-186 [22]");
    assert_eq!(
        rows,
        [
            ("FR", "FRP256v1", "ANSSI [i.21]", r),
            ("Brainpool", "brainpoolP256r1", brainpool, r),
            ("Brainpool", "brainpoolP384r1", brainpool, r),
            ("Brainpool", "brainpoolP512r1", brainpool, r),
            ("NIST", "P-256", nist, r),
            ("NIST", "P-384", nist, r),
            ("NIST", "P-521", nist, r),
        ]
    );
    for curve in EcCurve::ALL {
        assert!(
            CURVES.iter().any(|row| row.name == curve.name()),
            "{curve:?}"
        );
    }
}

/// Table 4, as printed: entry name, hash function column, signature algorithm column, R/L.
#[test]
fn table_4_is_transcribed() {
    use Hash::{Sha3_256, Sha3_384, Sha3_512, Sha224, Sha256, Sha384, Sha512, Shake256};
    let rows: Vec<_> = SIGNATURE_SUITES
        .iter()
        .map(|row| (row.name, row.hashes, row.internal, row.algorithm, row.class))
        .collect();
    let (r, l) = (Class::Recommended, Class::LEGACY);
    let sha2: &[Hash] = &[Sha256, Sha384, Sha512];
    let sha3: &[Hash] = &[Sha3_256, Sha3_384, Sha3_512];
    let shake: &[Hash] = &[Shake256];
    let expected: [(&str, &[Hash], bool, &str, Class); 25] = [
        (
            "sha224-with-rsa",
            &[Sha224],
            false,
            "RSA-PKCSv1_5",
            Class::legacy_until(2028),
        ),
        ("sha256-with-rsa", &[Sha256], false, "RSA-PKCSv1_5", l),
        ("sha384-with-rsa", &[Sha384], false, "RSA-PKCSv1_5", l),
        ("sha512-with-rsa", &[Sha512], false, "RSA-PKCSv1_5", l),
        (
            "rsa-pss with mgf1SHA2-Identifier",
            sha2,
            false,
            "RSA-PSS",
            r,
        ),
        (
            "rsa-pss with mgf1SHA3-Identifier",
            sha3,
            false,
            "RSA-PSS",
            r,
        ),
        ("sha2-with-dsa", sha2, false, "DSA", r),
        ("sha3-with-dsa", sha3, false, "DSA", r),
        ("sha2-with-ecdsa", sha2, false, "ECDSA", r),
        ("sha3-with-ecdsa", sha3, false, "ECDSA", r),
        ("sha2-with-ecsdsa", sha2, false, "EC-SDSA-opt", l),
        ("sha3-with-ecsdsa", sha3, false, "EC-SDSA-opt", l),
        ("ed25519", &[Sha512], true, "Ed25519", r),
        ("ed448", shake, true, "Ed448", r),
        ("ml-dsa-44", shake, true, "ML-DSA-44", r),
        ("ml-dsa-65", shake, true, "ML-DSA-65", r),
        ("ml-dsa-87", shake, true, "ML-DSA-87", r),
        ("slh-dsa-sha2-192s", &[Sha256], true, "SLH-DSA-SHA2-192s", r),
        ("slh-dsa-sha2-192f", &[Sha256], true, "SLH-DSA-SHA2-192f", r),
        ("slh-dsa-shake-192s", shake, true, "SLH-DSA-SHAKE-192s", r),
        ("slh-dsa-shake-192f", shake, true, "SLH-DSA-SHAKE-192f", r),
        ("slh-dsa-sha2-256s", &[Sha512], true, "SLH-DSA-SHA2-256s", r),
        ("slh-dsa-sha2-256f", &[Sha512], true, "SLH-DSA-SHA2-256f", r),
        ("slh-dsa-shake-256s", shake, true, "SLH-DSA-SHAKE-256s", r),
        ("slh-dsa-shake-256f", shake, true, "SLH-DSA-SHAKE-256f", r),
    ];
    assert_eq!(rows, expected);
}

/// Tables 6 and 7, as printed: the key-size column's bounds, the end date (`n/a` as `None`) and
/// the recommendation. Each row's end date is the 31 December of its class's year, which is what
/// clause 3.4's `L[yyyy]` says.
#[test]
fn tables_6_and_7_are_transcribed() {
    let rsa: Vec<_> = RSA_KEY_SIZES
        .iter()
        .map(|row| (row.modulus, row.subgroup, row.end, row.class))
        .collect();
    assert_eq!(
        rsa,
        [
            (
                (1_900, Some(3_000)),
                None,
                Some((2026, 12, 31)),
                Class::legacy_until(2026)
            ),
            ((3_000, None), None, None, Class::Recommended),
        ]
    );
    let dsa: Vec<_> = DSA_KEY_SIZES
        .iter()
        .map(|row| (row.modulus, row.subgroup, row.end, row.class))
        .collect();
    assert_eq!(
        dsa,
        [
            (
                (1_900, Some(3_000)),
                Some((200, Some(250))),
                Some((2026, 12, 31)),
                Class::legacy_until(2026)
            ),
            ((3_000, None), Some((250, None)), None, Class::Recommended),
        ]
    );
    for row in RSA_KEY_SIZES.iter().chain(&DSA_KEY_SIZES) {
        match row.class {
            Class::Legacy { year, .. } => assert_eq!(row.end, Some((year, 12, 31))),
            Class::Recommended => assert_eq!(row.end, None),
        }
    }
}

/// Clause 3.4: a bare `L` is `L[2034+]`, and an `L[yyyy]` mechanism is phased out once 31
/// December of its year has ended; `R` has no end date.
#[test]
fn a_class_is_phased_out_after_the_31_december_of_its_year() {
    assert_eq!(
        Class::LEGACY,
        Class::Legacy {
            year: 2034,
            extendable: true
        }
    );
    assert_eq!(
        Class::legacy_until(2026).phased_out_from(),
        Some(JANUARY_2027)
    );
    // 2035-01-01T00:00:00Z.
    assert_eq!(
        Class::LEGACY.phased_out_from(),
        Some(Instant::from_unix_seconds(2_051_222_400))
    );
    assert_eq!(Class::Recommended.phased_out_from(), None);
}

/// A suite of recommended members only — RSASSA-PSS, SHA-256, a 3072-bit modulus — raises no
/// issue, and each table's row is named.
#[test]
fn a_recommended_suite_raises_no_issue() {
    let assessment = assess(
        Digest::Sha256,
        Family::RsaPss {
            mask: Digest::Sha256,
        },
        3_072,
        None,
    );
    let r = Class::Recommended;
    assert_eq!(
        assessment.components,
        [
            Component {
                rule: Rule::HashFunctions,
                listed: Some(("SHA-256", r))
            },
            Component {
                rule: Rule::SignatureAlgorithms,
                listed: Some(("RSA-PSS", r))
            },
            Component {
                rule: Rule::SignatureSuites,
                listed: Some(("rsa-pss with mgf1SHA2-Identifier", r))
            },
            Component {
                rule: Rule::RsaKeySizes,
                listed: Some(("at least 3 000 bits", r))
            },
        ]
    );
    assert_eq!(assessment.issues, []);
}

/// Table 6: the same suite with a 2048-bit modulus is `L[2026]`; one below 1 900 bits is in no
/// row.
#[test]
fn an_rsa_modulus_is_sized_by_table_6() {
    let legacy = assess(
        Digest::Sha256,
        Family::RsaPss {
            mask: Digest::Sha256,
        },
        2_048,
        None,
    );
    assert_eq!(
        legacy.issues,
        [Issue::Legacy {
            rule: Rule::RsaKeySizes,
            entry: "at least 1 900 and below 3 000 bits",
            class: Class::legacy_until(2026),
        }]
    );
    let short = assess(
        Digest::Sha256,
        Family::RsaPss {
            mask: Digest::Sha256,
        },
        1_024,
        None,
    );
    assert_eq!(short.issues, [Issue::NotListed(Rule::RsaKeySizes)]);
    // The bounds: 1 900 is in the legacy row, 2 999 too, 3 000 in the recommended one.
    assert!(RSA_KEY_SIZES[0].admits(1_900, None));
    assert!(RSA_KEY_SIZES[0].admits(2_999, None));
    assert!(!RSA_KEY_SIZES[0].admits(3_000, None));
    assert!(!RSA_KEY_SIZES[0].admits(1_899, None));
}

/// Clause 3.4 over time: an `L[2026]` modulus is phased out at validation where the instant of
/// validation is past 2026, at the best signature time where that is, and not yet at the last
/// second of 2026.
#[test]
fn a_phase_out_is_compared_with_both_instants() {
    let entry = "at least 1 900 and below 3 000 bits";
    let class = Class::legacy_until(2026);
    let at = |signed, now| {
        assess_at(
            Digest::Sha256,
            Family::RsaPss {
                mask: Digest::Sha256,
            },
            2_048,
            None,
            signed,
            now,
        )
        .issues
    };
    assert_eq!(
        at(OCTOBER_2026, JANUARY_2027),
        [Issue::PhasedOut {
            rule: Rule::RsaKeySizes,
            entry,
            class,
            by: Moment::Validation
        }]
    );
    assert_eq!(
        at(JANUARY_2027, JANUARY_2027),
        [Issue::PhasedOut {
            rule: Rule::RsaKeySizes,
            entry,
            class,
            by: Moment::BestSignatureTime
        }]
    );
    let last_second = Instant::from_unix_seconds(JANUARY_2027.unix_seconds() - 1);
    assert_eq!(
        at(OCTOBER_2026, last_second),
        [Issue::Legacy {
            rule: Rule::RsaKeySizes,
            entry,
            class
        }]
    );
}

/// Tables 2 and 4: PKCS #1 v1.5 against RSASSA-PSS, everything else equal — the algorithm and the
/// suite are both a bare `L`.
#[test]
fn pkcs1_v1_5_is_legacy_by_table_2_and_table_4() {
    let assessment = assess(Digest::Sha256, Family::Rsa, 3_072, None);
    assert_eq!(
        assessment.issues,
        [
            Issue::Legacy {
                rule: Rule::SignatureAlgorithms,
                entry: "RSA-PKCS#1v1_5",
                class: Class::LEGACY
            },
            Issue::Legacy {
                rule: Rule::SignatureSuites,
                entry: "sha256-with-rsa",
                class: Class::LEGACY
            },
        ]
    );
}

/// Tables 1 and 4: SHA-1 against SHA-256, everything else equal — no row of either lists it; and
/// an RSA suite with a SHA-3 digest is in Table 1 but in no RSA PKCS #1 v1.5 row of Table 4.
#[test]
fn a_digest_no_table_lists_is_said() {
    let sha1 = assess(
        Digest::Sha1,
        Family::RsaPss {
            mask: Digest::Sha256,
        },
        3_072,
        None,
    );
    assert_eq!(
        sha1.issues,
        [
            Issue::NotListed(Rule::HashFunctions),
            Issue::NotListed(Rule::SignatureSuites),
        ]
    );
    let sha3 = assess(Digest::Sha3_256, Family::Rsa, 3_072, None);
    assert_eq!(
        sha3.issues,
        [
            Issue::Legacy {
                rule: Rule::SignatureAlgorithms,
                entry: "RSA-PKCS#1v1_5",
                class: Class::LEGACY
            },
            Issue::NotListed(Rule::SignatureSuites),
        ]
    );
}

/// Table 3 and clause 7.3: P-256 with SHA-256 is recommended; the same curve with SHA-384 is in
/// Table 4's row but not of equal lengths; P-521 with SHA-512 is not either, since no hash Table 1
/// lists is 521 bits long; brainpoolP512r1 with SHA3-512 is.
#[test]
fn an_elliptic_curve_suite_needs_equal_lengths() {
    let p256 = assess(Digest::Sha256, Family::Ecdsa(EcCurve::P256), 256, None);
    assert_eq!(p256.issues, []);
    assert_eq!(
        p256.components[2],
        Component {
            rule: Rule::Curves,
            listed: Some(("P-256", Class::Recommended))
        }
    );
    let longer = assess(Digest::Sha384, Family::Ecdsa(EcCurve::P256), 256, None);
    assert_eq!(
        longer.issues,
        [Issue::UnequalLengths {
            hash_bits: 384,
            curve_bits: 256
        }]
    );
    let p521 = assess(Digest::Sha512, Family::Ecdsa(EcCurve::P521), 521, None);
    assert_eq!(
        p521.issues,
        [Issue::UnequalLengths {
            hash_bits: 512,
            curve_bits: 521
        }]
    );
    let brainpool = assess(
        Digest::Sha3_512,
        Family::Ecdsa(EcCurve::BrainpoolP512r1),
        512,
        None,
    );
    assert_eq!(brainpool.issues, []);
}

/// Table 4's `EdDSA` rows state the hash as internal: Ed448's SHAKE256 is not asked of Table 1,
/// which does not list it, while Ed448 under a SHA-512 digest is in no row of Table 4. Ed25519
/// with SHA-512 is recommended.
#[test]
fn an_eddsa_suites_hash_is_its_own() {
    let ed448 = assess(
        Digest::Shake256,
        Family::EdDsa(eddsa::Curve::Ed448),
        456,
        None,
    );
    assert_eq!(ed448.issues, []);
    assert!(
        ed448
            .components
            .iter()
            .all(|component| component.rule != Rule::HashFunctions)
    );
    let other = assess(
        Digest::Sha512,
        Family::EdDsa(eddsa::Curve::Ed448),
        456,
        None,
    );
    assert_eq!(other.issues, [Issue::NotListed(Rule::SignatureSuites)]);
    let ed25519 = assess(
        Digest::Sha512,
        Family::EdDsa(eddsa::Curve::Ed25519),
        256,
        None,
    );
    assert_eq!(ed25519.issues, []);
}

/// Table 7 asks for both `p` and `q`: 2048/224 is `L[2026]`, 3072/256 is recommended, 3072/224 is
/// in neither row, and a key whose `q` was not measured is said as not assessed.
#[test]
fn a_dsa_key_is_sized_by_both_of_its_primes() {
    let entry = "p of at least 1 900 and below 3 000 bits, q of at least 200 and below 250 bits";
    assert_eq!(
        assess(Digest::Sha256, Family::Dsa, 2_048, Some(224)).issues,
        [Issue::Legacy {
            rule: Rule::DsaKeySizes,
            entry,
            class: Class::legacy_until(2026)
        }]
    );
    assert_eq!(
        assess(Digest::Sha256, Family::Dsa, 3_072, Some(256)).issues,
        []
    );
    assert_eq!(
        assess(Digest::Sha256, Family::Dsa, 3_072, Some(224)).issues,
        [Issue::NotListed(Rule::DsaKeySizes)]
    );
    assert_eq!(
        assess(Digest::Sha256, Family::Dsa, 3_072, None).issues,
        [Issue::SubgroupNotMeasured]
    );
}

/// Table 4's RSASSA-PSS rows are named by MGF1's hash — "rsa-pss with mgf1SHA2-Identifier" and
/// "rsa-pss with mgf1SHA3-Identifier" — and list the message digests, so a row admits a signature
/// whose digest and MGF1 hash are both among its hashes. One pairing per member, everything else
/// equal: SHA-256 under MGF1-SHA-256 and SHA3-256 under MGF1-SHA3-256 are each their own row; MGF1
/// over SHA-1 is Annex D Table D.3's discontinued `mgf1SHA-1Identifier` and in no row; a SHA-256
/// digest under MGF1-SHA3-256, and the reverse, are in neither row.
#[test]
fn table_4_s_pss_row_is_chosen_by_mgf1_s_hash_as_well_as_the_digest() {
    let row = |digest: Digest, mask: Digest| {
        assess(digest, Family::RsaPss { mask }, 3_072, None)
            .components
            .into_iter()
            .find(|component| component.rule == Rule::SignatureSuites)
            .and_then(|component| component.listed)
            .map(|(name, _)| name)
    };
    assert_eq!(
        row(Digest::Sha256, Digest::Sha256),
        Some("rsa-pss with mgf1SHA2-Identifier")
    );
    assert_eq!(
        row(Digest::Sha384, Digest::Sha512),
        Some("rsa-pss with mgf1SHA2-Identifier")
    );
    assert_eq!(
        row(Digest::Sha3_256, Digest::Sha3_256),
        Some("rsa-pss with mgf1SHA3-Identifier")
    );
    assert_eq!(row(Digest::Sha256, Digest::Sha1), None);
    assert_eq!(row(Digest::Sha256, Digest::Sha3_256), None);
    assert_eq!(row(Digest::Sha3_256, Digest::Sha256), None);
    // The unlisted pairing is the issue, and only the suite's: Table 1 lists the digest.
    assert_eq!(
        assess(
            Digest::Sha256,
            Family::RsaPss { mask: Digest::Sha1 },
            3_072,
            None
        )
        .issues,
        [Issue::NotListed(Rule::SignatureSuites)]
    );
}

/// Clause 9.2: "A hash function used to compute the imprint of a message placed in a time-stamp
/// token should never be a legacy mechanism at the time of time stamp creation." Each token's
/// imprint hash asks Table 1: SHA-256 and SHA3-512 are recommended and raise nothing; SHA-1 and
/// MD5 are in no row, so the document does not admit them at all, each said once per token; and
/// a token adds a component whatever it raises, after the suite's own.
#[test]
fn a_time_stamp_imprint_s_hash_is_asked_of_table_1() {
    let suite = || {
        assess(
            Digest::Sha256,
            Family::RsaPss {
                mask: Digest::Sha256,
            },
            3_072,
            None,
        )
    };
    let mut recommended = suite();
    recommended.imprints(&[Digest::Sha256, Digest::Sha3_512]);
    assert_eq!(recommended.issues, []);
    assert_eq!(
        recommended.components[4..],
        [
            Component {
                rule: Rule::TimeStampImprints,
                listed: Some(("SHA-256", Class::Recommended)),
            },
            Component {
                rule: Rule::TimeStampImprints,
                listed: Some(("SHA3-512", Class::Recommended)),
            },
        ]
    );
    let mut unlisted = suite();
    unlisted.imprints(&[Digest::Sha1, Digest::Sha256, Digest::Md5]);
    assert_eq!(
        unlisted.issues,
        [
            Issue::NotListed(Rule::TimeStampImprints),
            Issue::NotListed(Rule::TimeStampImprints),
        ]
    );
    assert_eq!(Rule::TimeStampImprints.citation(), "Table 1, clause 9.2");
}

/// Clause 9.3: "Signer's keys shall remain suitable during the certificate maintenance period
/// (commonly called validity period from notBefore to notAfter) of the associated certificate."
///
/// A 2 048-bit RSASSA-PSS key is Table 6's `L[2026]`, suitable through 31 December 2026: a
/// certificate whose `notAfter` is that day's last second raises nothing more, and one whose
/// `notAfter` is the next instant raises the issue against Table 6 alone, Table 2's RSA-PSS being
/// recommended. A PKCS #1 v1.5 key of 3 072 bits is Table 2's bare `L`, `L[2034+]`, so a
/// certificate valid into 2035 raises it against Table 2 — and not against Table 4, whose row is
/// the signature's combination with its digest rather than the key (ADR 2927). A component no row
/// lists has its issue already and is not asked again; a validity that did not read is said.
#[test]
fn a_signer_s_key_is_asked_to_outlast_its_certificate() {
    let year_2035 = Instant::from_unix_seconds(2_051_222_400);
    let last_second_of_2026 = Instant::from_unix_seconds(JANUARY_2027.unix_seconds() - 1);
    let last_second_of_2034 = Instant::from_unix_seconds(year_2035.unix_seconds() - 1);
    let pss = |key_bits| {
        assess(
            Digest::Sha256,
            Family::RsaPss {
                mask: Digest::Sha256,
            },
            key_bits,
            None,
        )
    };
    let outlived = |mut assessment: Assessment, not_after: Option<Instant>| {
        let before = assessment.issues.len();
        assessment.signer_key(not_after);
        assessment.issues.split_off(before)
    };

    assert_eq!(outlived(pss(2_048), Some(last_second_of_2026)), []);
    assert_eq!(
        outlived(pss(2_048), Some(JANUARY_2027)),
        [Issue::KeyOutlived {
            rule: Rule::RsaKeySizes,
            entry: "at least 1 900 and below 3 000 bits",
            class: Class::legacy_until(2026),
            not_after: JANUARY_2027,
        }]
    );
    assert_eq!(outlived(pss(3_072), Some(year_2035)), []);

    let pkcs1 = || assess(Digest::Sha256, Family::Rsa, 3_072, None);
    assert_eq!(outlived(pkcs1(), Some(last_second_of_2034)), []);
    assert_eq!(
        outlived(pkcs1(), Some(year_2035)),
        [Issue::KeyOutlived {
            rule: Rule::SignatureAlgorithms,
            entry: "RSA-PKCS#1v1_5",
            class: Class::LEGACY,
            not_after: year_2035,
        }]
    );

    let too_small = pss(1_024);
    assert!(
        too_small
            .issues
            .contains(&Issue::NotListed(Rule::RsaKeySizes))
    );
    assert_eq!(outlived(too_small, Some(year_2035)), []);

    assert_eq!(outlived(pss(3_072), None), [Issue::ValidityNotRead]);
    assert_eq!(Rule::SignerKeyLifetime.citation(), "clause 9.3");
}

/// Clause 9.5's *should*: another key resists over the period it is asked about, read as clause
/// 9.3 reads the signer's key (ADR 2928). A 2048-bit PSS authority whose period ends on the last
/// second of 2026 raises nothing and one a second later raises Table 6's row; a 3072-bit PKCS #1
/// v1.5 responder into 2035 raises Table 2's bare `L`; a P-256 unit stays recommended; an RSA key
/// whose padding was never seen asks only Table 6; an unclassified key is in no row of Table 2;
/// an unread validity is said.
#[test]
fn every_other_key_is_asked_to_resist_over_its_period() {
    let year_2035 = Instant::from_unix_seconds(2_051_222_400);
    let last_second_of_2026 = Instant::from_unix_seconds(JANUARY_2027.unix_seconds() - 1);
    let ca = KeyRole::CertificationAuthority { position: 1 };
    let rsa = |modulus_bits, pss| KeyParameters::Rsa { modulus_bits, pss };
    let asked = |role, key, until| {
        let mut assessment = assess(Digest::Sha256, Family::Ecdsa(EcCurve::P256), 256, None);
        let before = assessment.issues.len();
        assessment.other_keys(&[OtherKey { role, key, until }]);
        assessment.issues.split_off(before)
    };

    assert_eq!(
        asked(ca, rsa(2_048, Some(true)), Some(last_second_of_2026)),
        []
    );
    assert_eq!(
        asked(ca, rsa(2_048, Some(true)), Some(JANUARY_2027)),
        [Issue::OtherKeyOutlived {
            key: ca,
            rule: Rule::RsaKeySizes,
            entry: "at least 1 900 and below 3 000 bits",
            class: Class::legacy_until(2026),
            until: JANUARY_2027,
        }]
    );
    assert_eq!(
        asked(
            KeyRole::OcspResponder { about: 0 },
            rsa(3_072, Some(false)),
            Some(year_2035)
        ),
        [Issue::OtherKeyOutlived {
            key: KeyRole::OcspResponder { about: 0 },
            rule: Rule::SignatureAlgorithms,
            entry: "RSA-PKCS#1v1_5",
            class: Class::LEGACY,
            until: year_2035,
        }]
    );
    assert_eq!(
        asked(
            KeyRole::TimeStampingUnit,
            KeyParameters::Ecdsa(EcCurve::P256),
            Some(year_2035)
        ),
        []
    );
    assert_eq!(asked(ca, rsa(3_072, None), Some(year_2035)), []);
    assert_eq!(
        asked(ca, KeyParameters::Unclassified, Some(year_2035)),
        [Issue::OtherKeyNotListed {
            key: ca,
            rule: Rule::SignatureAlgorithms,
        }]
    );
    assert_eq!(
        asked(ca, rsa(3_072, Some(true)), None),
        [Issue::OtherKeyValidityNotRead { key: ca }]
    );
}

/// Clause 9.5's *shall*: a certifying key at least as strong as the key it certifies, compared
/// parameter by corresponding parameter within a family, and not compared across families
/// (ADR 2928): clause 3.1's levels bound a class from below and order no two keys.
#[test]
fn a_certifying_key_is_compared_with_the_key_it_certifies_only_within_its_family() {
    let ca = KeyRole::CertificationAuthority { position: 1 };
    let pairs = |certifier, certified| {
        let mut assessment = assess(Digest::Sha256, Family::Ecdsa(EcCurve::P256), 256, None);
        let before = assessment.issues.len();
        assessment.certifications(&[Certification {
            certifier: (ca, certifier),
            certified: (KeyRole::Signer, certified),
        }]);
        assessment.issues.split_off(before)
    };
    let rsa = |modulus_bits| KeyParameters::Rsa {
        modulus_bits,
        pss: None,
    };

    assert_eq!(pairs(rsa(3_072), rsa(3_072)), []);
    assert_eq!(pairs(rsa(4_096), rsa(3_072)), []);
    assert_eq!(
        pairs(rsa(2_048), rsa(3_072)),
        [Issue::WeakerThanCertified {
            certifier: ca,
            certified: KeyRole::Signer,
            parameter: Parameter::Modulus,
            certifier_bits: 2_048,
            certified_bits: 3_072,
        }]
    );
    assert_eq!(
        pairs(
            KeyParameters::Dsa {
                p_bits: 3_072,
                q_bits: 224,
            },
            KeyParameters::Dsa {
                p_bits: 2_048,
                q_bits: 256,
            }
        ),
        [Issue::WeakerThanCertified {
            certifier: ca,
            certified: KeyRole::Signer,
            parameter: Parameter::Subgroup,
            certifier_bits: 224,
            certified_bits: 256,
        }]
    );
    assert_eq!(
        pairs(
            KeyParameters::Ecdsa(EcCurve::P256),
            KeyParameters::Ecdsa(EcCurve::BrainpoolP384r1)
        ),
        [Issue::WeakerThanCertified {
            certifier: ca,
            certified: KeyRole::Signer,
            parameter: Parameter::CurveKeySize,
            certifier_bits: 256,
            certified_bits: 384,
        }]
    );
    assert_eq!(
        pairs(rsa(4_096), KeyParameters::Ecdsa(EcCurve::P256)),
        [Issue::NotCompared {
            certifier: ca,
            certified: KeyRole::Signer,
        }]
    );
}
