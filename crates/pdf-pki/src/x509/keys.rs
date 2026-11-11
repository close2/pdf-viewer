//! The public keys a certificate's `subjectPublicKeyInfo` can hold, as data.
//!
//! RFC 5280 section 4.1.2.7 puts the key in the certificate, and RFC 8017 appendix A.1.1, RFC 3279
//! section 2.3.2, RFC 5480 section 2.1.1 and RFC 8410 section 4 say how each family writes it.
//! What is here is the borrowed shape of each and the facts that follow from the shape alone — a
//! width in bits, a curve's name and object identifier — and nothing that computes with one: the
//! arithmetic that verifies a signature under these keys is `pdf_signature`'s (`pkcs1`, `dsa`,
//! `ecdsa` and `eddsa`), which names these types under the old module-local names.

use const_oid::ObjectIdentifier;
use const_oid::db::{rfc5639, rfc5912, rfc8410};

use crate::bigint::significant_bits;

/// The public half of an RSA key, as the two integers RFC 8017 section 3.1 defines it as.
///
/// Both are big-endian, exactly as an X.509 `RSAPublicKey` writes them, and both are borrowed
/// from the certificate: nothing here owns or copies a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RsaPublicKey<'a> {
    /// `n`, the modulus.
    pub modulus: &'a [u8],
    /// `e`, the public exponent.
    pub exponent: &'a [u8],
}

impl RsaPublicKey<'_> {
    /// The modulus's width in bits, which is the size a person means by "a 2048-bit key".
    ///
    /// Zero for a modulus of zero, which `pdf_signature::pkcs1::verify` refuses anyway.
    #[must_use]
    pub fn bits(&self) -> usize {
        significant_bits(self.modulus)
    }
}

/// RFC 3279 section 2.3.2's `id-dsa`, `1.2.840.10040.4.1`.
///
/// ```text
/// id-dsa OBJECT IDENTIFIER ::= { iso(1) member-body(2) us(840) x9-57(10040) x9cm(4) 1 }
/// ```
///
/// This is what a certificate's `subjectPublicKeyInfo` states for a DSA key, and what some
/// producers state as a `SignerInfo`'s `signatureAlgorithm` as well.
pub const ID_DSA: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x38, 0x04, 0x01];

/// RFC 3279 section 2.2.2's `id-dsa-with-sha1`, `1.2.840.10040.4.3`.
pub const ID_DSA_WITH_SHA1: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x38, 0x04, 0x03];

/// The `id-dsa-with-sha2` arc, `2.16.840.1.101.3.4.3` — one identifier per digest.
///
/// RFC 5758 section 3.1 assigns `1` to SHA-224 and `2` to SHA-256. The NIST Computer Security
/// Objects Register, which is the authority that assigns this arc, carries `3` and `4` for SHA-384
/// and SHA-512 and `5` to `8` for the SHA-3 family — and Table 260 permits SHA-384 and SHA-512 for
/// this `/SubFilter`, so a conforming file can state either.
///
/// **Recognising an identifier is not computing its digest**: which function was used is
/// `SignerInfo`'s own `digestAlgorithm`, read by `pdf_signature::cms::Digest`, and a signature
/// naming one this program cannot compute is reported by that number instead of this one.
pub const ID_DSA_WITH_SHA2: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x03];

/// Whether an object identifier names a DSA signature or a DSA key.
#[must_use]
pub fn is_dsa(oid: &[u8]) -> bool {
    if oid == ID_DSA || oid == ID_DSA_WITH_SHA1 {
        return true;
    }
    // The arc plus exactly one more component, which is the digest. A prefix test over a longer
    // tail would accept identifiers nobody has assigned.
    matches!(oid.split_at_checked(ID_DSA_WITH_SHA2.len()),
        Some((arc, [_])) if arc == ID_DSA_WITH_SHA2)
}

/// The public half of a DSA key: RFC 3279's `Dss-Parms` and the `y` beside it.
///
/// All four are big-endian, exactly as the certificate writes them, and all four are borrowed:
/// nothing here owns or copies a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DsaPublicKey<'a> {
    /// `p`, the prime modulus.
    pub p: &'a [u8],
    /// `q`, the prime divisor of `p - 1`.
    pub q: &'a [u8],
    /// `g`, the generator of the order-`q` subgroup.
    pub g: &'a [u8],
    /// `y`, the public key itself — RFC 3279 section 2.3.2's `DSAPublicKey ::= INTEGER`.
    pub y: &'a [u8],
}

impl DsaPublicKey<'_> {
    /// The key's width in bits, which is `p`'s — the number Table 260 puts its ceiling on.
    #[must_use]
    pub fn bits(&self) -> usize {
        significant_bits(self.p)
    }

    /// `N` in FIPS 186-4's notation: the bit length of `q`, which decides how much of a digest is
    /// used.
    #[must_use]
    pub fn subgroup_bits(&self) -> usize {
        significant_bits(self.q)
    }
}

/// One of ISO/TS 32002 Table 3's six curves, every one of which this program computes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcCurve {
    /// ISO/TS 32002 Table 3's "P-256", RFC 5912's `secp256r1` — `1.2.840.10045.3.1.7`.
    P256,
    /// ISO/TS 32002 Table 3's "P-384", RFC 5912's `secp384r1` — `1.3.132.0.34`.
    P384,
    /// ISO/TS 32002 Table 3's "P-521", RFC 5912's `secp521r1` — `1.3.132.0.35`.
    P521,
    /// ISO/TS 32002 Table 3's "brainpoolP256r1", RFC 5639 section 3.4's curve —
    /// `1.3.36.3.3.2.8.1.1.7`, assigned by its section 4.1.
    BrainpoolP256r1,
    /// ISO/TS 32002 Table 3's "brainpoolP384r1", RFC 5639 section 3.6's curve —
    /// `1.3.36.3.3.2.8.1.1.11`, assigned by its section 4.1.
    BrainpoolP384r1,
    /// ISO/TS 32002 Table 3's "brainpoolP512r1", RFC 5639 section 3.7's curve —
    /// `1.3.36.3.3.2.8.1.1.13`, assigned by its section 4.1.
    BrainpoolP512r1,
}

impl EcCurve {
    /// ISO/TS 32002 Table 3's six, in the table's order.
    pub const ALL: [Self; 6] = [
        Self::P256,
        Self::P384,
        Self::P521,
        Self::BrainpoolP256r1,
        Self::BrainpoolP384r1,
        Self::BrainpoolP512r1,
    ];

    /// The `namedCurve` object identifier this curve is stated by.
    ///
    /// RFC 5480 section 2.1.1 is what puts one of these in a certificate's `ECParameters`, and
    /// ISO/TS 32002 section 5.1.3 requires that form: "Certificates for ECDSA keys used in PDF
    /// signatures shall specify curve parameters (`ECParameters`) for the subject's public key
    /// using the namedCurve option".
    #[must_use]
    pub fn oid(self) -> ObjectIdentifier {
        match self {
            Self::P256 => rfc5912::SECP_256_R_1,
            Self::P384 => rfc5912::SECP_384_R_1,
            Self::P521 => rfc5912::SECP_521_R_1,
            Self::BrainpoolP256r1 => rfc5639::BRAINPOOL_P_256_R_1,
            Self::BrainpoolP384r1 => rfc5639::BRAINPOOL_P_384_R_1,
            Self::BrainpoolP512r1 => rfc5639::BRAINPOOL_P_512_R_1,
        }
    }

    /// The name ISO/TS 32002 Table 3 spells this curve with.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::P256 => "P-256",
            Self::P384 => "P-384",
            Self::P521 => "P-521",
            Self::BrainpoolP256r1 => "brainpoolP256r1",
            Self::BrainpoolP384r1 => "brainpoolP384r1",
            Self::BrainpoolP512r1 => "brainpoolP512r1",
        }
    }

    /// Whether an encoded `namedCurve` identifier is one of ISO/TS 32002 Table 3's six.
    #[must_use]
    pub fn of(oid: &[u8]) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|curve| curve.oid().as_bytes() == oid)
    }

    /// The field element width in octets, which is also `r`'s and `s`'s.
    ///
    /// P-521's field is 521 bits, so this is 66 rather than 65: SEC1 pads to whole octets. The
    /// Brainpool primes are exactly 256, 384 and 512 bits (RFC 5639 sections 3.4, 3.6 and 3.7).
    #[must_use]
    pub fn field_octets(self) -> usize {
        match self {
            Self::P256 | Self::BrainpoolP256r1 => 32,
            Self::P384 | Self::BrainpoolP384r1 => 48,
            Self::BrainpoolP512r1 => 64,
            Self::P521 => 66,
        }
    }

    /// The field width in bits, which is what a report calls the key's size.
    ///
    /// Table 260 states a ceiling in bits for RSA and DSA and states none for this family —
    /// ISO/TS 32002 Table 3 names curves instead — so this is the curve's own width and not a
    /// limit. The document's name is kept on the table's own line because the conformance
    /// checker reads a line at a time, so a reference broken between the two resolves against
    /// ISO 32000-2's Table 3 — the escape sequences in literal strings — in silence (ADR 0760).
    #[must_use]
    pub fn bits(self) -> usize {
        match self {
            Self::P256 | Self::BrainpoolP256r1 => 256,
            Self::P384 | Self::BrainpoolP384r1 => 384,
            Self::BrainpoolP512r1 => 512,
            Self::P521 => 521,
        }
    }
}

/// An ECDSA public key, as far as verifying one needs.
///
/// The curve came out of the certificate's `ECParameters` and the point out of its
/// `subjectPublicKey`, still SEC1-encoded: whether those octets are a point on that curve is
/// `pdf_signature::ecdsa::verify`'s question rather than [`crate::x509`]'s, because answering it is arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EcPublicKey<'a> {
    /// Which of the six curves the certificate's `namedCurve` stated.
    pub curve: EcCurve,
    /// `subjectPublicKey`'s octets, SEC1 section 2.3.3's encoding of a point.
    pub point: &'a [u8],
}

/// RFC 8410 section 3's `id-Ed25519`, `1.3.101.112`.
///
/// The identifier a certificate's `subjectPublicKeyInfo` states for one of ISO/TS 32002 Table 4's
/// keys, and the one a `SignerInfo` states as its `signatureAlgorithm` for one of its signatures —
/// RFC 8419 makes them the same number, which is why one constant serves both.
pub const ID_ED25519: ObjectIdentifier = rfc8410::ID_ED_25519;

/// RFC 8410 section 3's `id-Ed448`, `1.3.101.113` — ISO/TS 32002 Table 4's other curve, and the
/// same double duty as [`ID_ED25519`].
pub const ID_ED448: ObjectIdentifier = rfc8410::ID_ED_448;

/// RFC 8032 section 5.1's `b / 8`: an Ed25519 public key is 32 octets.
pub const ED25519_KEY_OCTETS: usize = 32;

/// RFC 8032 section 5.2's `b / 8`: an Ed448 public key is 57 octets.
pub const ED448_KEY_OCTETS: usize = 57;

/// One of ISO/TS 32002 Table 4's two Edwards curves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdCurve {
    /// RFC 8032 section 5.1's Ed25519, `id-Ed25519`.
    Ed25519,
    /// RFC 8032 section 5.2's Ed448, `id-Ed448`.
    Ed448,
}

impl EdCurve {
    /// The identifier a certificate's key and a `SignerInfo`'s `signatureAlgorithm` both state
    /// for this curve (RFC 8419 section 2.4 makes them one number).
    #[must_use]
    pub fn oid(self) -> ObjectIdentifier {
        match self {
            Self::Ed25519 => ID_ED25519,
            Self::Ed448 => ID_ED448,
        }
    }

    /// Which curve an encoded identifier names, if either.
    #[must_use]
    pub fn of(oid: &[u8]) -> Option<Self> {
        [Self::Ed25519, Self::Ed448]
            .into_iter()
            .find(|curve| curve.oid().as_bytes() == oid)
    }

    /// The name RFC 8032 and ISO/TS 32002 Table 4 spell the curve with.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Ed25519 => "Ed25519",
            Self::Ed448 => "Ed448",
        }
    }

    /// RFC 8032's `b`, the encoding's width in bits — 256 (section 5.1) or 456 (section 5.2) —
    /// which is what a report calls the key's size.
    #[must_use]
    pub fn bits(self) -> usize {
        match self {
            Self::Ed25519 => 8 * ED25519_KEY_OCTETS,
            Self::Ed448 => 8 * ED448_KEY_OCTETS,
        }
    }
}

/// An `EdDSA` public key, as far as verifying one needs.
///
/// `subjectPublicKey`'s octets, which RFC 8410 section 4 puts there raw rather than wrapped in a
/// structure, and the curve the certificate's algorithm identifier named. Whether the octets
/// decompress to a point is `pdf_signature::eddsa::verify`'s question, because it is arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdPublicKey<'a> {
    /// Which of the two curves the certificate's `algorithm` stated.
    pub curve: EdCurve,
    /// The octets of the compressed Edwards point: `b / 8` of them.
    pub key: &'a [u8],
}
