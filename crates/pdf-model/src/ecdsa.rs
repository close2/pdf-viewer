//! ECDSA signature verification: Table 260's third algorithm family, as ISO/TS 32002 enlarges it.
//!
//! ISO 32000-2's Table 260 gives a PDF signature three algorithm families and names this one by
//! reference — "ECDSA Algorithm Support ( defined by Internet RFC 5480 ) | ANSI X9.62 … (PDF 2.0
//! )" — for the `adbe.pkcs7.detached`, `ETSI.CAdES.detached` and `ETSI.RFC3161` `/SubFilter`
//! values, and **No** for the other two.
//!
//! ISO/TS 32002:2022 is what says *which curves*. Its section 5.1.1 states the extension — this
//! document "extends the elliptic curve digital signature support in Table 260 to add support for
//! more recent ECDSA curves, as defined in IETF RFCs 5639 and 6932, and to add support for
//! Edwards-curve Digital Signature Algorithm (EdDSA) based digital signatures as defined in IETF
//! RFC 8419" — and ISO/TS 32002 Table 3, in section 5.1.3, enumerates six, with the digests each
//! admits: P-256
//! (SHA-256, SHA3-256), P-384 (SHA-384, SHA3-384), P-521 (SHA512, SHA3-512), brainpoolP256r1,
//! brainpoolP384r1 and brainpoolP512r1. The Edwards pair is a different group law and lives in
//! [`crate::eddsa`].
//!
//! # Three of the six, and the other three are named rather than silent
//!
//! [`Curve`] holds P-256, P-384 and P-521. The Brainpool three are [`UnsupportedCurve`]: their
//! `RustCrypto` packages are release-candidate-only on this tree's `digest` 0.11 line and
//! brainpoolP512r1 has no package at all (measured 2026-08-23; ADR 0532 has the table). A
//! certificate stating one reaches a reader as *that curve, refused*, rather than as a shrug —
//! which is the whole of what this module owes a curve it cannot compute on.
//!
//! **Every sentence quoted here is ISO/TS 32002's and is in prose rather than in a blockquote**,
//! for the reason `cms::Digest` records: `tools/conformance` checks a rustdoc blockquote verbatim
//! against `doc/md/`'s ISO 32000-2, and words from another document would be unattributable there.
//! The quotation marks still mean verbatim.
//!
//! **The identifiers are not transcribed.** Every object identifier here is a `const` out of
//! `const_oid`'s database, grouped by the RFC that assigns it, so the digits are a second party's
//! reading of a registry rather than this project's memory of one. That is the cost `cms::Digest`
//! pays and this module does not.
//!
//! # What the signature value is, and what it is not
//!
//! ISO/TS 32002 section 5.1.3 requires the `namedCurve` form and settles the encoding in the same
//! breath: "The implicitCurve and specifiedCurve options shall not be used", with the note that
//! "[t]his restriction implies that ECDSA signature values are required to be represented using the
//! DER-encoded ECDSA-Sig-Value type in IETF RFC 5753:2010, section 7.2."
//!
//! So `SEQUENCE { r INTEGER, s INTEGER }` is the encoding this module reads, and it is read with
//! [`crate::der`] rather than with the strict reader the curve packages carry — four of the
//! corpus's signature values begin `30 80`, X.690 clause 8.1.3.6's indefinite length, and a
//! strict reader loses them (ADR 0331). BSI TR-03111's *plain* `r ‖ s`, which two corpus
//! signatures use under `0.4.0.127.0.7.1.1.4.1.3`, is **not** this encoding and is not something
//! the standard admits; it reaches a reader by its own identifier.
//!
//! # Where the arithmetic comes from, and why it is not in this file
//!
//! ADR 0331 is an owner decision: signature verification's arithmetic is a reviewed dependency
//! rather than in-tree code, because the defect class that matters in a verifier is *wrong
//! arithmetic* and a widely-used library has been run over shapes nobody here thought of. A curve
//! crate's prime, base point and order are reviewed constants on exactly that footing. So this
//! module is the *encoding*, the *budget* and the *vocabulary*; `p256`, `p384`, `p521` and
//! `ecdsa` are the group law.
//!
//! Two properties of that dependency are worth stating rather than assuming:
//!
//! - **The public key is validated by construction.** `VerifyingKey::from_sec1_bytes` decodes the
//!   SEC1 point and rejects one that is not on the curve or is the identity, which is the check
//!   ADR 0314 named as the thing a half-written verifier omits.
//! - **`r` and `s` are range-checked before any arithmetic.** `Signature::from_scalars` refuses a
//!   scalar that is zero or is not less than the curve order — ANSI X9.62's step 1, and the same
//!   check FIPS 186-4 section 4.7 step 1 makes for DSA. It is the check that keeps a forgery out
//!   of a scheme whose comparison has no safe direction.
//!
//! # Constant time, and why it is not the property being bought here
//!
//! ADR 0229's argument survives: **there is no secret.** The point, `r`, `s` and the digest all
//! came out of a file a stranger wrote, so a timing channel has nothing to leak. The `ecdsa`
//! crate's verification is written with `_vartime` scalar multiplication for exactly that reason.
//! What the dependency is taken for is correctness under review, not side-channel resistance.

#![expect(
    clippy::doc_markdown,
    reason = "ISO/TS 32002's sentences are quoted verbatim and its ASN.1 names are camel case \
              throughout; a quotation with backticks added to please a lint is no longer a \
              quotation (the same reasoning `pdf_syntax::filter::Delimiting` records)"
)]

use const_oid::ObjectIdentifier;
use const_oid::db::{rfc5639, rfc5912};
use ecdsa::signature::hazmat::PrehashVerifier;
use ecdsa::{EcdsaCurve, Signature, VerifyingKey};
use elliptic_curve::sec1::{FromSec1Point, ModulusSize, ToSec1Point};
use elliptic_curve::{AffinePoint, CurveArithmetic, FieldBytes, FieldBytesSize};

use crate::der::{INTEGER, Reader, SEQUENCE};

/// `elliptic-curve`, reached through `ecdsa` rather than named as a dependency of its own.
///
/// All four packages are built against one version of it, and reaching it through one of them is
/// what keeps that true: a second direct dependency could resolve to a different version and
/// silently become a second, incompatible set of types.
use ecdsa::elliptic_curve;

/// One of ISO/TS 32002 Table 3's curves that this program computes on.
///
/// Three of the table's six. The other three are [`UnsupportedCurve`], and the split is a fact
/// about the packages rather than about the standard — see the module documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Curve {
    /// ISO/TS 32002 Table 3's "P-256", RFC 5912's `secp256r1` — `1.2.840.10045.3.1.7`.
    P256,
    /// ISO/TS 32002 Table 3's "P-384", RFC 5912's `secp384r1` — `1.3.132.0.34`.
    P384,
    /// ISO/TS 32002 Table 3's "P-521", RFC 5912's `secp521r1` — `1.3.132.0.35`.
    P521,
}

/// One of ISO/TS 32002 Table 3's curves that this program does **not** compute on.
///
/// Carried as a value rather than collapsed into "unknown" so that a report can say which curve a
/// certificate stated. All three are curves the standard admits, so this is a gap in this program
/// and never a defect in the file — which is the difference between this and an identifier
/// [`Curve::of`] and this type both fail to recognise, where the file has left the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsupportedCurve {
    /// RFC 5639's `brainpoolP256r1` — `1.3.36.3.3.2.8.1.1.7`.
    BrainpoolP256r1,
    /// RFC 5639's `brainpoolP384r1` — `1.3.36.3.3.2.8.1.1.11`.
    BrainpoolP384r1,
    /// RFC 5639's `brainpoolP512r1` — `1.3.36.3.3.2.8.1.1.13`.
    BrainpoolP512r1,
}

impl UnsupportedCurve {
    /// The `namedCurve` object identifier this curve is stated by.
    #[must_use]
    pub fn oid(self) -> ObjectIdentifier {
        match self {
            Self::BrainpoolP256r1 => rfc5639::BRAINPOOL_P_256_R_1,
            Self::BrainpoolP384r1 => rfc5639::BRAINPOOL_P_384_R_1,
            Self::BrainpoolP512r1 => rfc5639::BRAINPOOL_P_512_R_1,
        }
    }

    /// The name ISO/TS 32002 Table 3 spells this curve with.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::BrainpoolP256r1 => "brainpoolP256r1",
            Self::BrainpoolP384r1 => "brainpoolP384r1",
            Self::BrainpoolP512r1 => "brainpoolP512r1",
        }
    }

    /// Whether an encoded `namedCurve` identifier is one of these three.
    #[must_use]
    pub fn of(oid: &[u8]) -> Option<Self> {
        [
            Self::BrainpoolP256r1,
            Self::BrainpoolP384r1,
            Self::BrainpoolP512r1,
        ]
        .into_iter()
        .find(|curve| curve.oid().as_bytes() == oid)
    }
}

impl Curve {
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
        }
    }

    /// The name ISO/TS 32002 Table 3 spells this curve with.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::P256 => "P-256",
            Self::P384 => "P-384",
            Self::P521 => "P-521",
        }
    }

    /// Whether an encoded `namedCurve` identifier is one of the three this program computes on.
    #[must_use]
    pub fn of(oid: &[u8]) -> Option<Self> {
        [Self::P256, Self::P384, Self::P521]
            .into_iter()
            .find(|curve| curve.oid().as_bytes() == oid)
    }

    /// The field element width in octets, which is also `r`'s and `s`'s.
    ///
    /// P-521's field is 521 bits, so this is 66 rather than 65: SEC1 pads to whole octets.
    #[must_use]
    pub fn field_octets(self) -> usize {
        match self {
            Self::P256 => 32,
            Self::P384 => 48,
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
            Self::P256 => 256,
            Self::P384 => 384,
            Self::P521 => 521,
        }
    }
}

/// The `ecdsa-with-SHA*` and `id-ecdsa-with-sha3-*` identifiers a `SignerInfo` may state.
///
/// ISO/TS 32002 Table 3 pairs each curve with SHA-2 and SHA-3 digests, and RFC 5758 section 3.2
/// and RFC 9688 are the documents that assign an identifier to each pairing. **Recognising one is
/// not choosing a digest with it**: which function was used is `SignerInfo`'s own
/// `digestAlgorithm`, which ISO/TS 32002 section 5.1.4 requires to be the same: "[i]f the
/// signedAttrs field is present in the SignerInfo field for the signer, then the same message
/// digest algorithm shall be used to compute both the digest of the SignedData encapContentInfo
/// eContent and the digest of the DER-encoded signedAttrs passed to the signature algorithm." So a
/// file that makes the two disagree gets `NotUnderThatKey`, which is a false negative and is the
/// safe direction. `1.2.840.10045.2.1` is here too: some producers state the *key*
/// algorithm where a signature algorithm belongs, and it names no digest at all.
#[must_use]
pub fn is_ecdsa(oid: &[u8]) -> bool {
    [
        rfc5912::ID_EC_PUBLIC_KEY,
        rfc5912::ECDSA_WITH_SHA_224,
        rfc5912::ECDSA_WITH_SHA_256,
        rfc5912::ECDSA_WITH_SHA_384,
        rfc5912::ECDSA_WITH_SHA_512,
        const_oid::db::rfc9688::ID_ECDSA_WITH_SHA_3_224,
        const_oid::db::rfc9688::ID_ECDSA_WITH_SHA_3_256,
        const_oid::db::rfc9688::ID_ECDSA_WITH_SHA_3_384,
        const_oid::db::rfc9688::ID_ECDSA_WITH_SHA_3_512,
    ]
    .iter()
    .any(|known| known.as_bytes() == oid)
}

/// An ECDSA public key, as far as verifying one needs.
///
/// The curve came out of the certificate's `ECParameters` and the point out of its
/// `subjectPublicKey`, still SEC1-encoded: whether those octets are a point on that curve is
/// [`verify`]'s question rather than [`crate::x509`]'s, because answering it is arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicKey<'a> {
    /// Which of the three curves the certificate's `namedCurve` stated.
    pub curve: Curve,
    /// `subjectPublicKey`'s octets, SEC1 section 2.3.3's encoding of a point.
    pub point: &'a [u8],
}

/// What stopped an ECDSA signature from being verified — a statement about the file, in every case.
///
/// None of these is "the signature is bad". A signature that is read and found not to match is
/// [`Ok(false)`](verify); these are the cases where the arithmetic never ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum EcdsaError {
    /// The signature value is not RFC 5753 section 7.2's `ECDSA-Sig-Value`.
    ///
    /// ISO/TS 32002 section 5.1.3's NOTE 2 makes that the required representation, so this is a
    /// file departing from the Technical Specification — BSI TR-03111's plain `r ‖ s`, most
    /// likely, which two corpus signatures use and which states its own algorithm identifier.
    #[error("the signature value is not a DER ECDSA-Sig-Value")]
    NotASignatureValue,
    /// `r` or `s` is written as a negative `INTEGER`.
    ///
    /// X.690 clause 8.3.2 makes the leading bit the sign, and both are unsigned quantities, so an
    /// encoding whose first octet has that bit set without a preceding zero is not one of them.
    #[error("the signature's r or s is encoded as a negative integer")]
    NegativeScalar,
    /// `r` or `s` has more significant octets than the curve's field.
    ///
    /// The width is fixed by the curve, so this is the budget on this module and it needs no
    /// number of its own: a value wider than the field cannot be a scalar of it.
    #[error("the signature's r or s is wider than the curve's field")]
    ScalarTooWide,
    /// `r` or `s` is zero, or is not less than the curve's order.
    ///
    /// ANSI X9.62's first verification step, refused before any arithmetic runs.
    #[error("the signature's r or s is zero or is not less than the curve order")]
    ScalarOutOfRange,
    /// `subjectPublicKey` is not a SEC1 point on the curve the certificate names.
    ///
    /// Either the encoding is not a point at all, or it is a point that does not satisfy the
    /// curve equation, or it is the identity. All three are refused by the same construction and
    /// none of them is a signature this program should go on to check.
    #[error("the certificate's public key is not a point on the curve it names")]
    MalformedPoint,
}

/// Verifies an ECDSA signature over a digest already computed.
///
/// `signature` is the `SignerInfo`'s `signature` — RFC 5753 section 7.2's DER `ECDSA-Sig-Value`,
/// which ISO/TS 32002 section 5.1.3 NOTE 2 requires. `digest` is the hash of whatever RFC 5652
/// section 5.4 says the signature is over; the truncation ANSI X9.62 applies to it — the leftmost
/// bits of the digest, to the curve order's width — is the dependency's, because it is arithmetic.
///
/// # Errors
///
/// An [`EcdsaError`] naming what stopped the check. A signature that is checked and does not
/// match is `Ok(false)`.
pub fn verify(key: PublicKey<'_>, signature: &[u8], digest: &[u8]) -> Result<bool, EcdsaError> {
    let (r, s) = signature_value(signature)?;
    let width = key.curve.field_octets();
    let r = scalar_octets(r, width)?;
    let s = scalar_octets(s, width)?;
    match key.curve {
        Curve::P256 => verify_on::<p256::NistP256>(key.point, &r, &s, digest),
        Curve::P384 => verify_on::<p384::NistP384>(key.point, &r, &s, digest),
        Curve::P521 => verify_on::<p521::NistP521>(key.point, &r, &s, digest),
    }
}

/// The same, once the curve is a type.
///
/// One function rather than three copies: the three arms of [`verify`] differ only in which
/// curve's constants the compiler substitutes, and a security check written three times is a
/// security check that can be edited twice.
fn verify_on<C>(point: &[u8], r: &[u8], s: &[u8], digest: &[u8]) -> Result<bool, EcdsaError>
where
    C: EcdsaCurve + CurveArithmetic,
    AffinePoint<C>: FromSec1Point<C> + ToSec1Point<C>,
    FieldBytesSize<C>: ModulusSize,
{
    let mut r_bytes = FieldBytes::<C>::default();
    let mut s_bytes = FieldBytes::<C>::default();
    // `scalar_octets` produced exactly this many, from this curve's own `field_octets`.
    r_bytes.copy_from_slice(r);
    s_bytes.copy_from_slice(s);
    // ANSI X9.62's step 1: zero, or not less than the order, and the arithmetic never runs.
    let signature =
        Signature::<C>::from_scalars(r_bytes, s_bytes).map_err(|_| EcdsaError::ScalarOutOfRange)?;
    // SEC1 section 2.3.4's decoding, which is also the point validation: off the curve, not a
    // point, or the identity are one refusal here rather than three silences later.
    let key = VerifyingKey::<C>::from_sec1_bytes(point).map_err(|_| EcdsaError::MalformedPoint)?;
    Ok(key.verify_prehash(digest, &signature).is_ok())
}

/// RFC 5753 section 7.2's `ECDSA-Sig-Value ::= SEQUENCE { r INTEGER, s INTEGER }`.
///
/// Read through [`crate::der`], which tolerates the indefinite lengths four of the corpus's
/// signature values are written with, and which bounds its own recursion and allocation.
fn signature_value(bytes: &[u8]) -> Result<(&[u8], &[u8]), EcdsaError> {
    let mut reader = Reader::new(bytes).map_err(|_| EcdsaError::NotASignatureValue)?;
    let Ok(Some(value)) = reader.next_value() else {
        return Err(EcdsaError::NotASignatureValue);
    };
    if value.identifier != SEQUENCE {
        return Err(EcdsaError::NotASignatureValue);
    }
    let Ok(mut numbers) = value.children() else {
        return Err(EcdsaError::NotASignatureValue);
    };
    let (Ok(Some(r)), Ok(Some(s))) = (numbers.next_value(), numbers.next_value()) else {
        return Err(EcdsaError::NotASignatureValue);
    };
    if r.identifier != INTEGER || s.identifier != INTEGER {
        return Err(EcdsaError::NotASignatureValue);
    }
    Ok((r.contents, s.contents))
}

/// A DER `INTEGER`'s contents as exactly `width` octets, big-endian.
///
/// X.690 clause 8.3.2 encodes an unsigned value whose top bit is set with a leading zero octet, so
/// a 32-octet `r` is written in 33; and a value with leading zero *bits* is written in fewer than
/// the field's octets. Both are the same number and both belong in the field's width, which is why
/// this is a re-alignment rather than a length check.
fn scalar_octets(contents: &[u8], width: usize) -> Result<Vec<u8>, EcdsaError> {
    let significant = contents
        .iter()
        .position(|&octet| octet != 0)
        .map_or(&[][..], |first| &contents[first..]);
    // A first octet with the sign bit set and no zero octet in front of it is a negative integer,
    // which neither `r` nor `s` can be.
    if contents.first().is_some_and(|&octet| octet & 0x80 != 0) {
        return Err(EcdsaError::NegativeScalar);
    }
    if significant.len() > width {
        return Err(EcdsaError::ScalarTooWide);
    }
    let mut out = vec![0u8; width];
    // The check above is what makes this exact rather than saturating; the spelling is the
    // workspace's, which forbids a bare subtraction on lengths that came out of a file.
    let start = width.saturating_sub(significant.len());
    out.get_mut(start..)
        .ok_or(EcdsaError::ScalarTooWide)?
        .copy_from_slice(significant);
    Ok(out)
}

/// Keys, certificates and signatures built once with `openssl` and pasted in.
///
/// **Test vectors rather than oracles**, on exactly the footing `pkcs1`'s and `dsa`'s constants
/// are on and for the reason ADR 0314 states: the corpus contains one DER-encoded ECDSA signature
/// in 67 460 documents, and nobody here holds the private key of the certificate that signed it,
/// so a *positive* verification over bytes this tree chose needs a key this tree made. What a
/// vector pins is that this module walks the encodings and hands the arithmetic the right numbers;
/// that the arithmetic is right is what the dependency is taken for.
///
/// Each was made once with:
///
/// ```sh
/// openssl ecparam -name prime256v1 -genkey -noout -out key.pem   # secp384r1, secp521r1
/// openssl req -x509 -key key.pem -sha256 -days 3650 -subj /CN=pdf-viewer -outform der -out cert.der
/// printf 'the signed bytes' | openssl dgst -sha256 -sign key.pem -out sig.der
/// ```
#[cfg(test)]
pub(crate) mod fixtures {
    /// The bytes every signature here was made over, shared with `pkcs1`'s and `dsa`'s fixtures.
    pub(crate) const MESSAGE: &[u8] = b"the signed bytes";

    /// A self-signed P-256 certificate.
    pub(crate) const P256_CERTIFICATE: &str = "\
        3082019230820139a00302010202142b0a19c8802368bab4b68df0de9fb55411\
        c0a3cb300a06082a8648ce3d040302301f311d301b06035504030c147064662d\
        76696577657220703235362074657374301e170d323630383233313634333230\
        5a170d3336303832303136343332305a301f311d301b06035504030c14706466\
        2d766965776572207032353620746573743059301306072a8648ce3d02010608\
        2a8648ce3d03010703420004f42873471a6522e626551feba346bdbce730a597\
        9ef97d143a1f6ae4e638c8547874bcaace8dc18b854303535fa89b367bf6aa43\
        40dc7deb67a63c2b896f8a57a3533051301d0603551d0e041604140ea314ecef\
        af42c43be92bf61b9dcd462b769c16301f0603551d230418301680140ea314ec\
        efaf42c43be92bf61b9dcd462b769c16300f0603551d130101ff040530030101\
        ff300a06082a8648ce3d040302034700304402200fd179aaf3f139577e7c88ec\
        bf04eed0338a025ec06b3260b12c4fddaa37a9460220708bd0512baaea93c128\
        947680c1a592075a09ddb65ece294bdf6767bc97c912";

    /// `ECDSA-Sig-Value` over `SHA-256(MESSAGE)` under the key above.
    pub(crate) const P256_SIGNATURE: &str = "\
        3045022100ee23ffbf5e22ffedb9a0068f06a0414811feb0ef3cdb464d953064\
        83f30e6f3d02202833d1b2f34889ffc71b7705ef134c161f2521aea3801ab20a\
        e388cd9523a39a";

    /// A self-signed P-384 certificate.
    pub(crate) const P384_CERTIFICATE: &str = "\
        308201d030820156a00302010202142a6699a1b3d9f17c0265d954d1e9348bfe\
        15c5b1300a06082a8648ce3d040303301f311d301b06035504030c147064662d\
        76696577657220703338342074657374301e170d323630383233313634333230\
        5a170d3336303832303136343332305a301f311d301b06035504030c14706466\
        2d766965776572207033383420746573743076301006072a8648ce3d02010605\
        2b8104002203620004cc4457f6a580c84ab9ace554d8318838924fa0ea69c2d5\
        b3d5f1ef3c0630bfc445e0a7b1eb2014c75d8d084120ea2e9ce319ac2757759a\
        bb82baf42d558cb9f13d61804720f5f36dc29e0b754858c1dd55df42ec26f222\
        43176a1403a65cf6f4a3533051301d0603551d0e04160414117acaa376dbac1c\
        1661cc6decc26790169602a4301f0603551d23041830168014117acaa376dbac\
        1c1661cc6decc26790169602a4300f0603551d130101ff040530030101ff300a\
        06082a8648ce3d0403030368003065023100a9aaf1bdcdae3662c3033cb9d855\
        0dca0808c83ab772cca2ee29d045986f70bf296b7f3ecddc0b0cfd167f741c57\
        9dde02305eb11ae9c5241244cf586e207e13c5175ff101f29120b6bd177a9fb8\
        f96ce92bcbfd64921918e4a1738eede16f59966a";

    /// `ECDSA-Sig-Value` over `SHA-384(MESSAGE)` under the key above.
    pub(crate) const P384_SIGNATURE: &str = "\
        3066023100f4944551bf129ec3fddbc2c616ec81273a49572ad6f58b688692ee\
        0804014381a4957b488bbeabca378b0da1c0af1e35023100c9eed2c1e73593b4\
        26107e9855fc15813cf3736a48095f770a36ade2c9c66bada240c25eea6c33d6\
        441f5a9187827575";

    /// A self-signed P-521 certificate — the one whose field is not a whole number of octets.
    pub(crate) const P521_CERTIFICATE: &str = "\
        3082021b3082017ca00302010202144de91816cb9f538acfa9ab4326e4299952\
        ddf607300a06082a8648ce3d040304301f311d301b06035504030c147064662d\
        76696577657220703532312074657374301e170d323630383233313634333230\
        5a170d3336303832303136343332305a301f311d301b06035504030c14706466\
        2d7669657765722070353231207465737430819b301006072a8648ce3d020106\
        052b81040023038186000400910d6d39eac60d1e9af4277f4e6430cc038b390c\
        92a48e52028fc47a915a0002daf0f343af85ba56ac80d2c4148ba9c753b4da6a\
        9c14cb0481d4974c530e27496b01f9a639186bbd430673bc1fb2e779ab1be9e8\
        08a1c84b6e0e7f9d1f2cab82cc433cd3c6dea50d18341c8a0643bda722299d03\
        70fe499d909138ee7ed813023828caa3533051301d0603551d0e04160414ca7e\
        1e6a2166992df77bde425424737434cf6b77301f0603551d23041830168014ca\
        7e1e6a2166992df77bde425424737434cf6b77300f0603551d130101ff040530\
        030101ff300a06082a8648ce3d04030403818c00308188024200a99dc37b4807\
        d0a60fc6f34c1954ba893f3e27af21d2d84691f48580b23cfaedcd3bc7bdc1bf\
        f35179c6729ceccfb89833aeccd2a1f7d9f358bc5c58611865969d0242011316\
        9d615936b00b169b9467bb1132711daab428e7664449e31628f117f9d48613e2\
        20f34d36f681d7f2f3579c046830b5b9031cbce57c48a4e8023ed398cf35a1";

    /// `ECDSA-Sig-Value` over `SHA-512(MESSAGE)` under the key above.
    pub(crate) const P521_SIGNATURE: &str = "\
        3081880242018a518117269185272c76bc2532abc386144389439a85ea06564e\
        d09de8af2f5ab347fa01b3c11e6b42d224e9c32ea96e302f9c722e083917b63f\
        99256f6487a33b024201a7897c57f47a806f5740c7f058438da2edd24d2ce453\
        cc600bbb8ff402644a57016bbba4db2d591bc51ffcf299f17c12ac29dc930cf9\
        c31e64645e7ff178b249ef";

    /// A self-signed brainpoolP256r1 certificate: a curve ISO/TS 32002 Table 3 names and this
    /// program refuses, so that the refusal has a witness rather than a comment.
    pub(crate) const BP256_CERTIFICATE: &str = "\
        308201963082013ca0030201020214562e870e03edd9391fba57a2f8dce46a73\
        16eb9b300a06082a8648ce3d0403023020311e301c06035504030c157064662d\
        7669657765722062703235362074657374301e170d3236303832333136343332\
        305a170d3336303832303136343332305a3020311e301c06035504030c157064\
        662d7669657765722062703235362074657374305a301406072a8648ce3d0201\
        06092b24030302080101070342000458144bbaa35fd9cfec1eb98b6122949a15\
        59042d8f3ada010674dfb5dfa6f3182f28a5745c546fde7f996118e0317fbd1e\
        3c9dd88ef9f810eac74eebb17bb5f9a3533051301d0603551d0e041604145e20\
        70dad17c4363ad1972c332a23324701b7873301f0603551d230418301680145e\
        2070dad17c4363ad1972c332a23324701b7873300f0603551d130101ff040530\
        030101ff300a06082a8648ce3d0403020348003045022100a146372341966f42\
        8445f70216d135c6b73674709b75c48610ba96c1adb01fc9022078e776dd8102\
        ab953865c503d9f47dc5abefc33546ec737cfc711dce82995f76";

    /// `ECDSA-Sig-Value` over `SHA-256(MESSAGE)` under the brainpool key above.
    ///
    /// Present so that the refusal is reached with a *whole* signature rather than with an absent
    /// one: what stops this verifying is the curve, and a fixture missing its signature would
    /// prove only that something stopped it.
    pub(crate) const BP256_SIGNATURE: &str = "\
        304402201f69c424e73c9c10ab65f15e4d56e2533c315da8a9c1906b89fda8b2\
        7cf5655d022013a6b8ad85aadc62c19a9ff6f4e00657911480354f995b18d29f\
        7f75558f206b";

    /// Hexadecimal to bytes.
    pub(crate) fn hex(text: &str) -> Vec<u8> {
        text.as_bytes()
            .chunks(2)
            .filter_map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::{
        BP256_CERTIFICATE, MESSAGE, P256_CERTIFICATE, P256_SIGNATURE, P384_CERTIFICATE,
        P384_SIGNATURE, P521_CERTIFICATE, P521_SIGNATURE, hex,
    };
    use super::{Curve, EcdsaError, UnsupportedCurve, verify};
    use crate::cms::Digest;
    use crate::x509::{self, PublicKey};

    /// The key out of the certificate, for a fixture that carries one.
    fn key_of(certificate: &[u8]) -> super::PublicKey<'_> {
        let certificate = x509::parse(certificate).expect("a certificate");
        match certificate.public_key {
            PublicKey::Ec(key) => key,
            other => panic!("this certificate states a named-curve EC key, not {other:?}"),
        }
    }

    /// The whole path, on each of ISO/TS 32002 Table 3's three curves this program computes on.
    ///
    /// A signature made with a key verifies under the key the certificate carries, and stops
    /// verifying when one bit of the message moves. The pair is what makes this a test of the
    /// arithmetic rather than of the encodings: a verifier that answered `true` unconditionally
    /// would pass the first half.
    #[test]
    fn a_signature_verifies_under_its_own_certificates_key_on_each_curve() {
        for (certificate, signature, digest, curve) in [
            (
                P256_CERTIFICATE,
                P256_SIGNATURE,
                Digest::Sha256,
                Curve::P256,
            ),
            (
                P384_CERTIFICATE,
                P384_SIGNATURE,
                Digest::Sha384,
                Curve::P384,
            ),
            (
                P521_CERTIFICATE,
                P521_SIGNATURE,
                Digest::Sha512,
                Curve::P521,
            ),
        ] {
            let certificate = hex(certificate);
            let key = key_of(&certificate);
            assert_eq!(key.curve, curve, "the certificate's namedCurve");
            let signature = hex(signature);
            assert_eq!(
                verify(key, &signature, &digest.compute(&[MESSAGE])),
                Ok(true),
                "{} should verify",
                curve.name()
            );
            let mut moved = MESSAGE.to_vec();
            moved[0] ^= 0x01;
            assert_eq!(
                verify(key, &signature, &digest.compute(&[moved.as_slice()])),
                Ok(false),
                "{} should not verify a message one bit away",
                curve.name()
            );
        }
    }

    /// ANSI X9.62's first step, refused before the arithmetic — one value at a time.
    ///
    /// `r` and `s` are rebuilt into the fixture's own `ECDSA-Sig-Value` so that the four failures
    /// are the range check's rather than the builder's, which is why the last assertion verifies
    /// the untouched pair through the same builder. All-ones at the field width stands in for "not
    /// less than the order" without transcribing one: every curve's order is below `2^(8 · width)`.
    #[test]
    fn a_scalar_that_is_zero_or_not_below_the_order_never_reaches_the_arithmetic() {
        let certificate = hex(P256_CERTIFICATE);
        let key = key_of(&certificate);
        let digest = Digest::Sha256.compute(&[MESSAGE]);
        let width = Curve::P256.field_octets();
        // `2^256 − 1`, written as X.690 clause 8.3.2 requires a positive integer whose leading bit
        // is set to be written: one zero octet in front. Above every curve's order by
        // construction, since an order is below `2^(8 · width)`.
        let too_large = [&[0x00u8][..], &vec![0xFFu8; width]].concat();
        let fixture = hex(P256_SIGNATURE);
        let (r, s) = super::signature_value(&fixture).expect("an ECDSA-Sig-Value");
        let (r, s) = (r.to_vec(), s.to_vec());
        for (replaced_r, replaced_s) in [
            (vec![0u8], s.clone()),
            (r.clone(), vec![0u8]),
            (too_large.clone(), s.clone()),
            (r.clone(), too_large.clone()),
        ] {
            let value = sig_value(&replaced_r, &replaced_s);
            assert_eq!(
                verify(key, &value, &digest),
                Err(EcdsaError::ScalarOutOfRange),
                "r = {replaced_r:02x?}, s = {replaced_s:02x?}"
            );
        }
        assert_eq!(
            verify(key, &sig_value(&r, &s), &digest),
            Ok(true),
            "the untouched pair through the same builder"
        );
    }

    /// A scalar wider than the field, and one written as a negative integer.
    #[test]
    fn a_scalar_this_module_cannot_place_in_the_field_is_named_rather_than_truncated() {
        let certificate = hex(P256_CERTIFICATE);
        let key = key_of(&certificate);
        let digest = Digest::Sha256.compute(&[MESSAGE]);
        let wide = vec![0x01u8; Curve::P256.field_octets() + 1];
        assert_eq!(
            verify(key, &sig_value(&wide, &[0x01]), &digest),
            Err(EcdsaError::ScalarTooWide)
        );
        assert_eq!(
            verify(key, &sig_value(&[0x80, 0x01], &[0x01]), &digest),
            Err(EcdsaError::NegativeScalar)
        );
    }

    /// BSI TR-03111's plain `r ‖ s` is not the encoding ISO/TS 32002 NOTE 2 requires.
    ///
    /// Two corpus signatures are written that way, under their own algorithm identifier. This
    /// pins that such a value is *named* rather than read as something it is not — a fixed-width
    /// concatenation happens to begin with whatever `r`'s first octet is, and a reader that
    /// shrugged would be guessing at a structure.
    #[test]
    fn a_plain_concatenation_is_not_an_ecdsa_sig_value() {
        let certificate = hex(P256_CERTIFICATE);
        let key = key_of(&certificate);
        let digest = Digest::Sha256.compute(&[MESSAGE]);
        let plain = vec![0x11u8; 2 * Curve::P256.field_octets()];
        assert_eq!(
            verify(key, &plain, &digest),
            Err(EcdsaError::NotASignatureValue)
        );
        assert_eq!(
            verify(key, &[], &digest),
            Err(EcdsaError::NotASignatureValue)
        );
    }

    /// A curve ISO/TS 32002 Table 3 names and this program does not compute on, named rather than
    /// dropped.
    #[test]
    fn a_brainpool_certificate_carries_its_curve_out_to_the_report() {
        let certificate = hex(BP256_CERTIFICATE);
        let certificate = x509::parse(&certificate).expect("a certificate");
        assert_eq!(
            certificate.public_key,
            PublicKey::EcCurveNotVerifiable {
                curve: Some(UnsupportedCurve::BrainpoolP256r1.oid().as_bytes())
            }
        );
    }

    /// Every identifier this module acts on, read back as the digits a person sees.
    ///
    /// The constants are `const_oid`'s — a second party's reading of the registries that assign
    /// them — so what this checks is that [`crate::x509::dotted`] decodes the same encoding the
    /// same way, in both directions, for all six of ISO/TS 32002 Table 3's curves.
    #[test]
    fn the_curve_identifiers_decode_to_the_numbers_the_constants_state() {
        for curve in [Curve::P256, Curve::P384, Curve::P521] {
            let oid = curve.oid();
            assert_eq!(
                x509::dotted(oid.as_bytes()).as_deref(),
                Some(oid.to_string().as_str()),
                "{}",
                curve.name()
            );
            assert_eq!(Curve::of(oid.as_bytes()), Some(curve));
        }
        for curve in [
            UnsupportedCurve::BrainpoolP256r1,
            UnsupportedCurve::BrainpoolP384r1,
            UnsupportedCurve::BrainpoolP512r1,
        ] {
            let oid = curve.oid();
            assert_eq!(
                x509::dotted(oid.as_bytes()).as_deref(),
                Some(oid.to_string().as_str()),
                "{}",
                curve.name()
            );
            assert_eq!(UnsupportedCurve::of(oid.as_bytes()), Some(curve));
            assert_eq!(Curve::of(oid.as_bytes()), None, "{}", curve.name());
        }
    }

    /// `SEQUENCE { r INTEGER, s INTEGER }` around two contents, for the tests above.
    fn sig_value(r: &[u8], s: &[u8]) -> Vec<u8> {
        let integer = |contents: &[u8]| {
            let mut out = vec![0x02, u8::try_from(contents.len()).expect("a short integer")];
            out.extend_from_slice(contents);
            out
        };
        let body = [integer(r), integer(s)].concat();
        let mut out = vec![0x30, u8::try_from(body.len()).expect("a short sequence")];
        out.extend_from_slice(&body);
        out
    }
}
