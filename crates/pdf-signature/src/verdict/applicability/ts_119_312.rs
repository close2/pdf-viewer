//! ETSI TS 119 312 V2.1.1 (2026-06), the cryptographic suites: the tables TS 119 172-4
//! REQ-4.5-01 h) assesses a signature's suite against, and that assessment (ADR 2896).
//!
//! # What is held, and how
//!
//! The document is ETSI's and its notice permits no reproduction without ETSI's written permission
//! (ADR 1085), so what is carried here is each table's identifiers, references and classes, cell by
//! cell, and the rules around them are paraphrased under their clause numbers. Six tables are
//! held: Table 1 (hash functions, clause 5.1), Table 2 (signature algorithms, clause 6.2.1), Table 3
//! (elliptic curves, clause 6.2.2.3), Table 4 (signature suites, clause 7.3), Table 6 (RSA key
//! sizes, clause 8.4) and Table 7 (DSA key sizes, clause 8.4). Tables 3.1, 3.2 and 3.3 (the ML-DSA
//! and SLH-DSA parameter sets and the hybrid combinations) are not, because this tree computes none
//! of those schemes and a suite it cannot compute is REQ-4.2-03 e)'s unsupported suite before it is
//! anything of h)'s; Table 4's rows for them are held because Table 4 is held whole.
//!
//! # The classes
//!
//! Clause 3.4 marks each mechanism recommended (`R`, no end date yet) or legacy, and a legacy mark
//! carries a phase-out year: `L[yyyy]` is phased out no later than 31 December of that year,
//! `L[yyyy+]` on that day unless a later release extends it, and a bare `L` is the same as
//! `L[2034+]` ([`Class`]).
//!
//! # The assessment
//!
//! [`Assessment::of`] asks each table about one suite — the digest, the signature algorithm, the
//! curve or the key's size, and the combination — and states a potential security issue for every
//! component a table does not list, every legacy component, and every legacy component whose
//! phase-out had passed at the best signature time or at the instant of validation. Clause 7.3's
//! rule that an elliptic-curve suite is recommended only where the digest's output length equals
//! the curve's key size is asked as its own rule.
//!
//! Clause 9's time periods are asked after it: clause 9.2's imprint hash of every time-stamp
//! token taken as a time ([`Assessment::imprints`]), and clause 9.3's signer's key against its
//! certificate's validity ([`Assessment::signer_key`]). Clause 9.5's other keys — a CA's, a
//! revocation issuer's, a time-stamping unit's — are not asked, because the report is not handed
//! the certificates of the path that validated (ADR 2927).

use crate::cms::Digest;
use crate::eddsa;
use crate::signature::Family;
use crate::x509::{self, Instant};

/// The document the tables are from, as the report names it.
pub const SOURCE: &str = "ETSI TS 119 312 V2.1.1 (2026-06)";

/// Clause 3.4's classification of a mechanism.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// `R`: recommended, with no end date defined yet.
    Recommended,
    /// `L[yyyy]` or `L[yyyy+]`: legacy, phased out at the end of `year`.
    Legacy {
        /// The year on whose 31 December the mechanism is phased out.
        year: u16,
        /// `L[yyyy+]` (and a bare `L`): a later release may move the date; `L[yyyy]` says no
        /// later than that day.
        extendable: bool,
    },
}

impl Class {
    /// A bare `L`, which clause 3.4's note equates with `L[2034+]`.
    pub const LEGACY: Self = Self::Legacy {
        year: 2034,
        extendable: true,
    };

    /// `L[yyyy]`.
    #[must_use]
    pub const fn legacy_until(year: u16) -> Self {
        Self::Legacy {
            year,
            extendable: false,
        }
    }

    /// The first instant after the phase-out day, 1 January of the next year at midnight UTC;
    /// `None` for a recommended mechanism.
    #[must_use]
    pub fn phased_out_from(self) -> Option<Instant> {
        match self {
            Self::Recommended => None,
            Self::Legacy { year, .. } => {
                let days = x509::days_from_civil(i64::from(year).checked_add(1)?, 1, 1)?;
                Some(Instant::from_unix_seconds(days.checked_mul(86_400)?))
            }
        }
    }
}

/// A hash function as the tables name one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hash {
    /// SHA-224.
    Sha224,
    /// SHA-256.
    Sha256,
    /// SHA-384.
    Sha384,
    /// SHA-512.
    Sha512,
    /// SHA3-256.
    Sha3_256,
    /// SHA3-384.
    Sha3_384,
    /// SHA3-512.
    Sha3_512,
    /// SHAKE256, which the tables name only as a scheme's internal hash.
    Shake256,
}

impl Hash {
    /// The hash a CMS digest algorithm is, where the tables name it; MD5, SHA-1 and RIPEMD-160
    /// appear in none of them.
    #[must_use]
    pub const fn of(digest: Digest) -> Option<Self> {
        Some(match digest {
            Digest::Sha256 => Self::Sha256,
            Digest::Sha384 => Self::Sha384,
            Digest::Sha512 => Self::Sha512,
            Digest::Sha3_256 => Self::Sha3_256,
            Digest::Sha3_384 => Self::Sha3_384,
            Digest::Sha3_512 => Self::Sha3_512,
            Digest::Shake256 => Self::Shake256,
            Digest::Md5 | Digest::Sha1 | Digest::Ripemd160 => return None,
        })
    }

    /// The output length in bits, which clause 7.3 compares with a curve's key size.
    #[must_use]
    pub const fn output_bits(self) -> usize {
        match self {
            Self::Sha224 => 224,
            Self::Sha256 | Self::Sha3_256 => 256,
            Self::Sha384 | Self::Sha3_384 => 384,
            // SHAKE256 at the 512 bits ISO/TS 32001 section 5.1.4 fixes for a signature.
            Self::Sha512 | Self::Sha3_512 | Self::Shake256 => 512,
        }
    }
}

/// A signature scheme as the tables name one; Table 2 names `EdDSA` once and Table 4 names its
/// two variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    /// RSA under RFC 8017's PKCS #1 v1.5 padding.
    RsaPkcs1v15,
    /// RSA under RSASSA-PSS.
    RsaPss,
    /// DSA.
    Dsa,
    /// ECDSA.
    Ecdsa,
    /// Ed25519.
    Ed25519,
    /// Ed448.
    Ed448,
    /// ML-DSA (FIPS 204).
    MlDsa,
    /// SLH-DSA (FIPS 205).
    SlhDsa,
    /// EC-SDSA-opt (ISO/IEC 14888-3).
    EcSdsaOpt,
    /// LMS (RFC 8554).
    Lms,
    /// XMSS (RFC 8391).
    Xmss,
}

impl Scheme {
    /// The scheme a verified signature's family is.
    #[must_use]
    pub const fn of(family: Family) -> Self {
        match family {
            Family::Rsa => Self::RsaPkcs1v15,
            Family::RsaPss { .. } => Self::RsaPss,
            Family::Dsa => Self::Dsa,
            Family::Ecdsa(_) => Self::Ecdsa,
            Family::EdDsa(eddsa::Curve::Ed25519) => Self::Ed25519,
            Family::EdDsa(eddsa::Curve::Ed448) => Self::Ed448,
        }
    }
}

/// A condition Table 2 attaches to a class by a note.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    /// No note.
    None,
    /// Table 2's note 1: recommended by TS 119 312 itself, the mechanism not being one ECCG's
    /// agreed mechanisms list.
    ByThisDocumentAlone,
    /// Table 2's note 2: stateful, recommended only where the signer can enforce its state
    /// management.
    StateManaged,
}

/// One row of Table 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HashFunction {
    /// The hash.
    pub hash: Hash,
    /// The short name column.
    pub name: &'static str,
    /// The references column.
    pub reference: &'static str,
    /// The R/L column.
    pub class: Class,
}

/// TS 119 312 Table 1, clause 5.1's hash functions.
pub const HASH_FUNCTIONS: [HashFunction; 7] = [
    HashFunction {
        hash: Hash::Sha224,
        name: "SHA-224",
        reference: "FIPS Publication 180-4 [1]",
        class: Class::legacy_until(2028),
    },
    HashFunction {
        hash: Hash::Sha256,
        name: "SHA-256",
        reference: "FIPS Publication 180-4 [1]",
        class: Class::Recommended,
    },
    HashFunction {
        hash: Hash::Sha384,
        name: "SHA-384",
        reference: "FIPS Publication 180-4 [1]",
        class: Class::Recommended,
    },
    HashFunction {
        hash: Hash::Sha512,
        name: "SHA-512",
        reference: "FIPS Publication 180-4 [1]",
        class: Class::Recommended,
    },
    HashFunction {
        hash: Hash::Sha3_256,
        name: "SHA3-256",
        reference: "FIPS Publication 202 [15]",
        class: Class::Recommended,
    },
    HashFunction {
        hash: Hash::Sha3_384,
        name: "SHA3-384",
        reference: "FIPS Publication 202 [15]",
        class: Class::Recommended,
    },
    HashFunction {
        hash: Hash::Sha3_512,
        name: "SHA3-512",
        reference: "FIPS Publication 202 [15]",
        class: Class::Recommended,
    },
];

/// One row of Table 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignatureAlgorithm {
    /// The schemes the row covers; one, except `EdDSA`'s two variants.
    pub schemes: &'static [Scheme],
    /// The short name column.
    pub name: &'static str,
    /// The references column.
    pub reference: &'static str,
    /// The R/L column.
    pub class: Class,
    /// The note the R/L column points at.
    pub condition: Condition,
}

/// TS 119 312 Table 2, clause 6.2.1's signature algorithms.
pub const SIGNATURE_ALGORITHMS: [SignatureAlgorithm; 10] = [
    SignatureAlgorithm {
        schemes: &[Scheme::RsaPkcs1v15],
        name: "RSA-PKCS#1v1_5",
        reference: "IETF RFC 8017 [3]",
        class: Class::LEGACY,
        condition: Condition::None,
    },
    SignatureAlgorithm {
        schemes: &[Scheme::RsaPss],
        name: "RSA-PSS",
        reference: "IETF RFC 8017 [3]",
        class: Class::Recommended,
        condition: Condition::None,
    },
    SignatureAlgorithm {
        schemes: &[Scheme::Dsa],
        name: "DSA",
        reference: "FIPS Publication 186-5, ISO/IEC 14888-3 [4]",
        class: Class::Recommended,
        condition: Condition::None,
    },
    SignatureAlgorithm {
        schemes: &[Scheme::Ecdsa],
        name: "ECDSA",
        reference: "FIPS Publication 186-5 [2]",
        class: Class::Recommended,
        condition: Condition::None,
    },
    SignatureAlgorithm {
        schemes: &[Scheme::Ed25519, Scheme::Ed448],
        name: "EdDSA",
        reference: "IETF RFC 8032 [23]",
        class: Class::Recommended,
        condition: Condition::ByThisDocumentAlone,
    },
    SignatureAlgorithm {
        schemes: &[Scheme::MlDsa],
        name: "ML-DSA",
        reference: "NIST FIPS Publication 204 [26]",
        class: Class::Recommended,
        condition: Condition::None,
    },
    SignatureAlgorithm {
        schemes: &[Scheme::SlhDsa],
        name: "SLH-DSA",
        reference: "NIST FIPS Publication 205 [27]",
        class: Class::Recommended,
        condition: Condition::None,
    },
    SignatureAlgorithm {
        schemes: &[Scheme::EcSdsaOpt],
        name: "EC-SDSA-opt",
        reference: "ISO/IEC 14888-3 [4]",
        class: Class::LEGACY,
        condition: Condition::None,
    },
    SignatureAlgorithm {
        schemes: &[Scheme::Lms],
        name: "LMS",
        reference: "IETF RFC 8554 [28]",
        class: Class::Recommended,
        condition: Condition::StateManaged,
    },
    SignatureAlgorithm {
        schemes: &[Scheme::Xmss],
        name: "XMSS",
        reference: "IETF RFC 8391 [29]",
        class: Class::Recommended,
        condition: Condition::StateManaged,
    },
];

/// One row of Table 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Curve {
    /// The curve family column, carried down from the row that prints it.
    pub family: &'static str,
    /// The short curve name column.
    pub name: &'static str,
    /// The references column.
    pub reference: &'static str,
    /// The R/L column.
    pub class: Class,
}

/// TS 119 312 Table 3, the curves clause 6.2.2.3 confines ECDSA to.
pub const CURVES: [Curve; 7] = [
    Curve {
        family: "FR",
        name: "FRP256v1",
        reference: "ANSSI [i.21]",
        class: Class::Recommended,
    },
    Curve {
        family: "Brainpool",
        name: "brainpoolP256r1",
        reference: "IETF RFC 5639 [5]",
        class: Class::Recommended,
    },
    Curve {
        family: "Brainpool",
        name: "brainpoolP384r1",
        reference: "IETF RFC 5639 [5]",
        class: Class::Recommended,
    },
    Curve {
        family: "Brainpool",
        name: "brainpoolP512r1",
        reference: "IETF RFC 5639 [5]",
        class: Class::Recommended,
    },
    Curve {
        family: "NIST",
        name: "P-256",
        reference: "NIST Special Publication 800-186 [22]",
        class: Class::Recommended,
    },
    Curve {
        family: "NIST",
        name: "P-384",
        reference: "NIST Special Publication 800-186 [22]",
        class: Class::Recommended,
    },
    Curve {
        family: "NIST",
        name: "P-521",
        reference: "NIST Special Publication 800-186 [22]",
        class: Class::Recommended,
    },
];

/// One row of Table 4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Suite {
    /// The suite's entry name column.
    pub name: &'static str,
    /// The hash function column's hashes.
    pub hashes: &'static [Hash],
    /// Whether the hash function column says the hash is internal to the algorithm.
    pub internal: bool,
    /// The signature algorithm column, as Table 4 spells it.
    pub algorithm: &'static str,
    /// The scheme that column names.
    pub scheme: Scheme,
    /// The R/L column.
    pub class: Class,
}

/// SHA-256, SHA-384 or SHA-512, the hash column of six of Table 4's rows.
const SHA2: &[Hash] = &[Hash::Sha256, Hash::Sha384, Hash::Sha512];
/// SHA3-256, SHA3-384 or SHA3-512, the hash column of five of Table 4's rows.
const SHA3: &[Hash] = &[Hash::Sha3_256, Hash::Sha3_384, Hash::Sha3_512];

/// A recommended Table 4 row whose hash is internal to its algorithm.
const fn internal(
    name: &'static str,
    hash: &'static [Hash],
    algorithm: &'static str,
    scheme: Scheme,
) -> Suite {
    Suite {
        name,
        hashes: hash,
        internal: true,
        algorithm,
        scheme,
        class: Class::Recommended,
    }
}

/// TS 119 312 Table 4, clause 7.3's signature suites, all 25 rows in the table's order.
pub const SIGNATURE_SUITES: [Suite; 25] = [
    Suite {
        name: "sha224-with-rsa",
        hashes: &[Hash::Sha224],
        internal: false,
        algorithm: "RSA-PKCSv1_5",
        scheme: Scheme::RsaPkcs1v15,
        class: Class::legacy_until(2028),
    },
    Suite {
        name: "sha256-with-rsa",
        hashes: &[Hash::Sha256],
        internal: false,
        algorithm: "RSA-PKCSv1_5",
        scheme: Scheme::RsaPkcs1v15,
        class: Class::LEGACY,
    },
    Suite {
        name: "sha384-with-rsa",
        hashes: &[Hash::Sha384],
        internal: false,
        algorithm: "RSA-PKCSv1_5",
        scheme: Scheme::RsaPkcs1v15,
        class: Class::LEGACY,
    },
    Suite {
        name: "sha512-with-rsa",
        hashes: &[Hash::Sha512],
        internal: false,
        algorithm: "RSA-PKCSv1_5",
        scheme: Scheme::RsaPkcs1v15,
        class: Class::LEGACY,
    },
    Suite {
        name: "rsa-pss with mgf1SHA2-Identifier",
        hashes: SHA2,
        internal: false,
        algorithm: "RSA-PSS",
        scheme: Scheme::RsaPss,
        class: Class::Recommended,
    },
    Suite {
        name: "rsa-pss with mgf1SHA3-Identifier",
        hashes: SHA3,
        internal: false,
        algorithm: "RSA-PSS",
        scheme: Scheme::RsaPss,
        class: Class::Recommended,
    },
    Suite {
        name: "sha2-with-dsa",
        hashes: SHA2,
        internal: false,
        algorithm: "DSA",
        scheme: Scheme::Dsa,
        class: Class::Recommended,
    },
    Suite {
        name: "sha3-with-dsa",
        hashes: SHA3,
        internal: false,
        algorithm: "DSA",
        scheme: Scheme::Dsa,
        class: Class::Recommended,
    },
    Suite {
        name: "sha2-with-ecdsa",
        hashes: SHA2,
        internal: false,
        algorithm: "ECDSA",
        scheme: Scheme::Ecdsa,
        class: Class::Recommended,
    },
    Suite {
        name: "sha3-with-ecdsa",
        hashes: SHA3,
        internal: false,
        algorithm: "ECDSA",
        scheme: Scheme::Ecdsa,
        class: Class::Recommended,
    },
    Suite {
        name: "sha2-with-ecsdsa",
        hashes: SHA2,
        internal: false,
        algorithm: "EC-SDSA-opt",
        scheme: Scheme::EcSdsaOpt,
        class: Class::LEGACY,
    },
    Suite {
        name: "sha3-with-ecsdsa",
        hashes: SHA3,
        internal: false,
        algorithm: "EC-SDSA-opt",
        scheme: Scheme::EcSdsaOpt,
        class: Class::LEGACY,
    },
    Suite {
        name: "ed25519",
        hashes: &[Hash::Sha512],
        internal: true,
        algorithm: "Ed25519",
        scheme: Scheme::Ed25519,
        class: Class::Recommended,
    },
    Suite {
        name: "ed448",
        hashes: &[Hash::Shake256],
        internal: true,
        algorithm: "Ed448",
        scheme: Scheme::Ed448,
        class: Class::Recommended,
    },
    internal("ml-dsa-44", &[Hash::Shake256], "ML-DSA-44", Scheme::MlDsa),
    internal("ml-dsa-65", &[Hash::Shake256], "ML-DSA-65", Scheme::MlDsa),
    internal("ml-dsa-87", &[Hash::Shake256], "ML-DSA-87", Scheme::MlDsa),
    internal(
        "slh-dsa-sha2-192s",
        &[Hash::Sha256],
        "SLH-DSA-SHA2-192s",
        Scheme::SlhDsa,
    ),
    internal(
        "slh-dsa-sha2-192f",
        &[Hash::Sha256],
        "SLH-DSA-SHA2-192f",
        Scheme::SlhDsa,
    ),
    internal(
        "slh-dsa-shake-192s",
        &[Hash::Shake256],
        "SLH-DSA-SHAKE-192s",
        Scheme::SlhDsa,
    ),
    internal(
        "slh-dsa-shake-192f",
        &[Hash::Shake256],
        "SLH-DSA-SHAKE-192f",
        Scheme::SlhDsa,
    ),
    internal(
        "slh-dsa-sha2-256s",
        &[Hash::Sha512],
        "SLH-DSA-SHA2-256s",
        Scheme::SlhDsa,
    ),
    internal(
        "slh-dsa-sha2-256f",
        &[Hash::Sha512],
        "SLH-DSA-SHA2-256f",
        Scheme::SlhDsa,
    ),
    internal(
        "slh-dsa-shake-256s",
        &[Hash::Shake256],
        "SLH-DSA-SHAKE-256s",
        Scheme::SlhDsa,
    ),
    internal(
        "slh-dsa-shake-256f",
        &[Hash::Shake256],
        "SLH-DSA-SHAKE-256f",
        Scheme::SlhDsa,
    ),
];

/// One row of Table 6 or Table 7: a range of a key's sizes in bits, from inclusive to below
/// exclusive, with its end date and class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeySize {
    /// The modulus's (RSA) or the prime `p`'s (DSA) range of bit lengths.
    pub modulus: (usize, Option<usize>),
    /// DSA's subgroup order `q`'s range of bit lengths; `None` for RSA.
    pub subgroup: Option<(usize, Option<usize>)>,
    /// The end date column, year, month and day; `None` where it says n/a.
    pub end: Option<(u16, u8, u8)>,
    /// The recommendation column.
    pub class: Class,
}

impl KeySize {
    /// Whether a key of these sizes is in this row.
    #[must_use]
    pub fn admits(&self, modulus: usize, subgroup: Option<usize>) -> bool {
        let within = |(from, below): (usize, Option<usize>), bits: usize| {
            bits >= from && below.is_none_or(|below| bits < below)
        };
        within(self.modulus, modulus)
            && match (self.subgroup, subgroup) {
                (None, _) => true,
                (Some(range), Some(bits)) => within(range, bits),
                (Some(_), None) => false,
            }
    }
}

/// TS 119 312 Table 6, clause 8.4's RSA modulus sizes.
pub const RSA_KEY_SIZES: [KeySize; 2] = [
    KeySize {
        modulus: (1_900, Some(3_000)),
        subgroup: None,
        end: Some((2026, 12, 31)),
        class: Class::legacy_until(2026),
    },
    KeySize {
        modulus: (3_000, None),
        subgroup: None,
        end: None,
        class: Class::Recommended,
    },
];

/// TS 119 312 Table 7, clause 8.4's DSA sizes of `p` and of `q`.
pub const DSA_KEY_SIZES: [KeySize; 2] = [
    KeySize {
        modulus: (1_900, Some(3_000)),
        subgroup: Some((200, Some(250))),
        end: Some((2026, 12, 31)),
        class: Class::legacy_until(2026),
    },
    KeySize {
        modulus: (3_000, None),
        subgroup: Some((250, None)),
        end: None,
        class: Class::Recommended,
    },
];

/// Which of the document's rules a finding is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    /// TS 119 312 Table 1, the digest.
    HashFunctions,
    /// TS 119 312 Table 2, the signature algorithm.
    SignatureAlgorithms,
    /// TS 119 312 Table 3, the elliptic curve.
    Curves,
    /// TS 119 312 Table 4, the combination.
    SignatureSuites,
    /// Clause 7.3's rule that an elliptic-curve suite is recommended only where the digest's
    /// output length equals the curve's key size.
    EqualLengths,
    /// TS 119 312 Table 6, the RSA modulus's size.
    RsaKeySizes,
    /// TS 119 312 Table 7, the DSA key's sizes.
    DsaKeySizes,
    /// Clause 9.2's rule on the hash of a time-stamp token's imprint, asked of Table 1.
    TimeStampImprints,
    /// Clause 9.3's rule that the signer's key remains suitable over its certificate's validity,
    /// asked of the key's rows in Tables 2, 3, 6 and 7.
    SignerKeyLifetime,
}

impl Rule {
    /// Where the rule is printed, for a sentence a person reads.
    #[must_use]
    pub const fn citation(self) -> &'static str {
        match self {
            Self::HashFunctions => "Table 1, clause 5.1",
            Self::SignatureAlgorithms => "Table 2, clause 6.2.1",
            Self::Curves => "Table 3, clause 6.2.2.3",
            Self::SignatureSuites => "Table 4, clause 7.3",
            Self::EqualLengths => "clause 7.3",
            Self::RsaKeySizes => "Table 6, clause 8.4",
            Self::DsaKeySizes => "Table 7, clause 8.4",
            Self::TimeStampImprints => "Table 1, clause 9.2",
            Self::SignerKeyLifetime => "clause 9.3",
        }
    }
}

/// What one table says about one component of the suite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Component {
    /// The table asked.
    pub rule: Rule,
    /// The row that lists the component, by the table's name for it, with its class; `None`
    /// where no row does.
    pub listed: Option<(&'static str, Class)>,
}

/// When a legacy component's phase-out had passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Moment {
    /// Already at the best signature time.
    BestSignatureTime,
    /// Not at the best signature time, but at the instant of validation.
    Validation,
}

/// A potential security issue, as REQ-4.5-01 h) asks for one: what failed, against which rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Issue {
    /// The table lists no row for the component, so the document does not admit it.
    NotListed(Rule),
    /// The table lists the component as legacy, and its phase-out has not passed.
    Legacy {
        /// The table.
        rule: Rule,
        /// The row's name for the component.
        entry: &'static str,
        /// Its class.
        class: Class,
    },
    /// The table lists the component as legacy, and its phase-out had passed.
    PhasedOut {
        /// The table.
        rule: Rule,
        /// The row's name for the component.
        entry: &'static str,
        /// Its class.
        class: Class,
        /// By when.
        by: Moment,
    },
    /// Clause 7.3: the digest's output length is not the curve's key size.
    UnequalLengths {
        /// The digest's output length in bits.
        hash_bits: usize,
        /// The curve's key size in bits.
        curve_bits: usize,
    },
    /// A DSA key whose `q` this reader could not measure, so Table 7 could not be asked.
    SubgroupNotMeasured,
    /// Clause 9.3: a row of the signer's key is legacy, and its phase-out falls inside the
    /// validity period of the signer's certificate, so the key does not remain suitable for as
    /// long as the certificate says it may be used.
    KeyOutlived {
        /// The table whose row it is: 2, 3, 6 or 7.
        rule: Rule,
        /// The row's name for the component.
        entry: &'static str,
        /// Its class.
        class: Class,
        /// The certificate's `notAfter`, which the phase-out precedes.
        not_after: Instant,
    },
    /// The signer's certificate's validity did not read, so clause 9.3 could not be asked.
    ValidityNotRead,
}

/// What [`Assessment::of`] reads: the suite, and the two instants a phase-out is compared with.
#[derive(Debug, Clone, Copy)]
pub struct Asked {
    /// The digest.
    pub digest: Digest,
    /// The signature algorithm, with its curve.
    pub family: Family,
    /// The key's size: the modulus's bits for RSA, `p`'s for DSA, the curve's for the others.
    pub key_bits: usize,
    /// DSA's `q` in bits, where the signer's certificate reads; `None` for the other families.
    pub subgroup_bits: Option<usize>,
    /// The best signature time.
    pub signed: Instant,
    /// The instant of validation.
    pub now: Instant,
}

/// REQ-4.5-01 h)'s assessment of one suite against TS 119 312.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assessment {
    /// Each table's answer, in the document's order.
    pub components: Vec<Component>,
    /// The potential security issues, in the same order; empty where every component is
    /// recommended and the combination admitted.
    pub issues: Vec<Issue>,
}

impl Assessment {
    /// Asks Tables 1, 2, 3, 4, 6 and 7 and clause 7.3 about the suite.
    ///
    /// Table 1 is not asked where Table 4 states the digest as internal to the algorithm and it is
    /// that digest — `ed448`'s SHAKE256, which Table 1 does not list as a hash function in its own
    /// right. Table 3 is asked for ECDSA only, Table 6 for RSA under either padding, Table 7 for
    /// DSA; an elliptic curve's key size is its curve's (clause 6.2.2.3), and `EdDSA`'s are fixed
    /// (Table 7's note 2).
    #[must_use]
    pub fn of(asked: &Asked) -> Self {
        let scheme = Scheme::of(asked.family);
        let hash = Hash::of(asked.digest);
        // Table 4's two RSASSA-PSS rows are named by MGF1's hash — "rsa-pss with
        // mgf1SHA2-Identifier" and "... mgf1SHA3-Identifier" — and list the message digests. A row
        // admits a PSS signature whose digest and MGF1 hash are both among its hashes; any other
        // pairing, Annex D Table D.3's discontinued `mgf1SHA-1Identifier` among them, is in no row
        // (ADR 2905).
        let mask = match asked.family {
            Family::RsaPss { mask } => Some(Hash::of(mask)),
            _ => None,
        };
        let suite = SIGNATURE_SUITES.iter().find(|suite| {
            suite.scheme == scheme
                && hash.is_some_and(|hash| suite.hashes.contains(&hash))
                && mask.is_none_or(|mask| mask.is_some_and(|mask| suite.hashes.contains(&mask)))
        });
        let mut components = Vec::new();
        let mut issues = Vec::new();
        if !suite.is_some_and(|suite| suite.internal) {
            components.push(Component {
                rule: Rule::HashFunctions,
                listed: hash.and_then(|hash| {
                    HASH_FUNCTIONS
                        .iter()
                        .find(|row| row.hash == hash)
                        .map(|row| (row.name, row.class))
                }),
            });
        }
        components.push(Component {
            rule: Rule::SignatureAlgorithms,
            listed: SIGNATURE_ALGORITHMS
                .iter()
                .find(|row| row.schemes.contains(&scheme))
                .map(|row| (row.name, row.class)),
        });
        if let Family::Ecdsa(curve) = asked.family {
            components.push(Component {
                rule: Rule::Curves,
                listed: CURVES
                    .iter()
                    .find(|row| row.name == curve.name())
                    .map(|row| (row.name, row.class)),
            });
        }
        components.push(Component {
            rule: Rule::SignatureSuites,
            listed: suite.map(|suite| (suite.name, suite.class)),
        });
        match scheme {
            Scheme::RsaPkcs1v15 | Scheme::RsaPss => components.push(Component {
                rule: Rule::RsaKeySizes,
                listed: RSA_KEY_SIZES
                    .iter()
                    .find(|row| row.admits(asked.key_bits, None))
                    .map(|row| (key_size_entry(row), row.class)),
            }),
            Scheme::Dsa => match asked.subgroup_bits {
                Some(q) => components.push(Component {
                    rule: Rule::DsaKeySizes,
                    listed: DSA_KEY_SIZES
                        .iter()
                        .find(|row| row.admits(asked.key_bits, Some(q)))
                        .map(|row| (key_size_entry(row), row.class)),
                }),
                None => issues.push(Issue::SubgroupNotMeasured),
            },
            _ => {}
        }
        for component in &components {
            issues.extend(issue(component, asked));
            // Clause 7.3 follows Table 4, the row it qualifies.
            if component.rule == Rule::SignatureSuites
                && let (Family::Ecdsa(curve), Some(hash)) = (asked.family, hash)
                && suite.is_some()
                && hash.output_bits() != curve.bits()
            {
                issues.push(Issue::UnequalLengths {
                    hash_bits: hash.output_bits(),
                    curve_bits: curve.bits(),
                });
            }
        }
        Self { components, issues }
    }

    /// Clause 9.2: "A hash function used to compute the imprint of a message placed in a
    /// time-stamp token should never be a legacy mechanism at the time of time stamp creation."
    ///
    /// Each token's `messageImprint` hash asks Table 1 and adds one component: a hash no row lists
    /// is [`Issue::NotListed`] — the document does not admit it at all, which is further from
    /// recommended than legacy — and one whose row is legacy, phased out or not, is
    /// [`Issue::Legacy`]. The classes are this version's, and a class only ever moves from
    /// recommended towards legacy, so a hash legacy now was no better when an earlier token was
    /// made, and one recommended now was recommended then (ADR 2905). Table 1's one legacy row is
    /// SHA-224, which no time-stamp token this reader takes as a time is computed with, so the
    /// rule's issue today is the unlisted one.
    pub fn imprints(&mut self, tokens: &[Digest]) {
        for &digest in tokens {
            let listed = Hash::of(digest).and_then(|hash| {
                HASH_FUNCTIONS
                    .iter()
                    .find(|row| row.hash == hash)
                    .map(|row| (row.name, row.class))
            });
            self.components.push(Component {
                rule: Rule::TimeStampImprints,
                listed,
            });
            match listed {
                None => self.issues.push(Issue::NotListed(Rule::TimeStampImprints)),
                Some((entry, class @ Class::Legacy { .. })) => {
                    self.issues.push(Issue::Legacy {
                        rule: Rule::TimeStampImprints,
                        entry,
                        class,
                    });
                }
                Some((_, Class::Recommended)) => {}
            }
        }
    }

    /// Clause 9.3: "Signer's keys shall remain suitable during the certificate maintenance period
    /// (commonly called validity period from notBefore to notAfter) of the associated
    /// certificate."
    ///
    /// The key is what Tables 2, 3, 6 and 7 classify — its algorithm, its curve, its modulus's or
    /// primes' sizes — and not Table 1's digest or Table 4's combination, which belong to one
    /// signature rather than to the key (ADR 2927). Suitable is clause 3.4's reading: a
    /// recommended row has no end date and stays suitable, a legacy row stops at its phase-out.
    /// So each of those components already listed adds [`Issue::KeyOutlived`] where its row's
    /// phase-out has passed by `not_after` — RFC 5280's validity includes its last second, so a
    /// phase-out on 1 January at midnight UTC and a `notAfter` at that instant is an issue. A
    /// component no row lists has its issue already, and is not asked again. Clause 9.3's note 3
    /// says a time stamp made with recommended mechanisms while the suite is still legacy is
    /// enough protection; a note is not a requirement, and the issue is stated whatever tokens
    /// cover the signature, as a potential one, which is what REQ-4.5-01 h) asks for.
    ///
    /// Classes only move from recommended towards legacy, so a key suitable at `notAfter` was
    /// suitable from `notBefore`; the start of the period is not asked. `None` — a certificate
    /// whose validity did not read — is [`Issue::ValidityNotRead`].
    pub fn signer_key(&mut self, not_after: Option<Instant>) {
        let Some(not_after) = not_after else {
            self.issues.push(Issue::ValidityNotRead);
            return;
        };
        let outlived: Vec<Issue> = self
            .components
            .iter()
            .filter(|component| {
                matches!(
                    component.rule,
                    Rule::SignatureAlgorithms
                        | Rule::Curves
                        | Rule::RsaKeySizes
                        | Rule::DsaKeySizes
                )
            })
            .filter_map(|component| {
                let (entry, class) = component.listed?;
                let from = class.phased_out_from()?;
                (not_after >= from).then_some(Issue::KeyOutlived {
                    rule: component.rule,
                    entry,
                    class,
                    not_after,
                })
            })
            .collect();
        self.issues.extend(outlived);
    }
}

/// Table 6's and Table 7's rows have no name column; a row is named by its key-size column, in
/// words that read the same as the cell.
fn key_size_entry(row: &KeySize) -> &'static str {
    match (row.modulus, row.subgroup) {
        ((1_900, Some(3_000)), None) => "at least 1 900 and below 3 000 bits",
        ((3_000, None), None) => "at least 3 000 bits",
        ((1_900, Some(3_000)), Some(_)) => {
            "p of at least 1 900 and below 3 000 bits, q of at least 200 and below 250 bits"
        }
        _ => "p of at least 3 000 bits, q of at least 250 bits",
    }
}

/// The issue one component raises, if any.
fn issue(component: &Component, asked: &Asked) -> Option<Issue> {
    let Some((entry, class)) = component.listed else {
        return Some(Issue::NotListed(component.rule));
    };
    let from = class.phased_out_from()?;
    let by = if asked.signed >= from {
        Some(Moment::BestSignatureTime)
    } else if asked.now >= from {
        Some(Moment::Validation)
    } else {
        None
    };
    Some(match by {
        Some(by) => Issue::PhasedOut {
            rule: component.rule,
            entry,
            class,
            by,
        },
        None => Issue::Legacy {
            rule: component.rule,
            entry,
            class,
        },
    })
}

#[cfg(test)]
mod tests;
