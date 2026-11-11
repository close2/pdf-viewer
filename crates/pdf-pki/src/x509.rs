//! An X.509 certificate, read for what verifying a signature and validating a certification path
//! need from it.
//!
//! ISO 32000-2 §12.8.3.3.1 puts the requirement on the file and says what the certificate is for:
//!
//! > At minimum the CMS object shall include the signer's X.509 signing certificate. This
//! > certificate shall be used to verify the signature value in Contents .
//!
//! So a program that answers a signature's second question needs a certificate reader, and this is
//! it — over [`crate::der`], under this crate's `#![forbid(unsafe_code)]`, allocating nothing that
//! a length in the file sizes.
//!
//! # What is read, for which question, and what is deliberately not
//!
//! **For the second question**: the serial number and the issuer, because RFC 5652's `SignerInfo`
//! names its certificate by that pair; the subject key identifier extension, because it names one
//! the other way; and `subjectPublicKeyInfo`, which is the key.
//!
//! **For the third** — "is the signer anyone to believe" — the fields a certification path turns
//! on: the version, the validity dates, the basic constraints, the key usage, the extended key
//! usage, the authority key identifier, and `tbsCertificate` with the issuer's signature over it.
//! Nothing here weighs them. `pdf_signature::trust` does, and only on a path that ends at an anchor a
//! host supplied, because a `notAfter` acted on without one would be the worst of both: an air of
//! validation over a certificate that could have been made up five minutes ago by whoever wrote
//! the file.
//!
//! **Carried for the policy tree and the name constraints**: `certificatePolicies`,
//! `policyMappings`, `policyConstraints`, `inhibitAnyPolicy`, `nameConstraints` and
//! `subjectAltName`, each as the encoding inside its `extnValue`. This module recognises them and
//! `pdf_signature::trust`'s `policies` and `names` decode them, because a malformed one is a refusal of
//! the path rather than of the certificate's reading (ADR 2724).
//!
//! # A name is compared, never decoded
//!
//! An X.501 `Name` is a nest of relative distinguished names holding strings in five encodings,
//! and this module does not decode one. It compares the issuer in a certificate with the issuer in
//! the `SignerInfo` **as encoded bytes**, which is what matching a signer to its certificate needs
//! and all it needs. Nothing here can therefore print who a certificate says it belongs to — and
//! that is a feature: a subject name out of an unverified certificate is a claim by whoever wrote
//! the file, and showing one beside the words "verifies" is how a viewer says *valid* without
//! using the word.

use crate::der::{DerError, INTEGER, OBJECT_IDENTIFIER, OCTET_STRING, Reader, SEQUENCE, Value};

mod keys;

pub use keys::{
    DsaPublicKey, ED448_KEY_OCTETS, ED25519_KEY_OCTETS, EcCurve, EcPublicKey, EdCurve, EdPublicKey,
    ID_DSA, ID_DSA_WITH_SHA1, ID_DSA_WITH_SHA2, ID_ED448, ID_ED25519, RsaPublicKey, is_dsa,
};

/// RFC 8017's `rsaEncryption`, `1.2.840.113549.1.1.1`.
///
/// One of the two public-key algorithm identifiers this program acts on, the other being
/// [`ID_DSA`]. Everything else is carried to the report as the object identifier the
/// file states — see [`PublicKey::Unverifiable`] and [`dotted`].
pub const RSA_ENCRYPTION: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x01];

/// RFC 5480 section 2.1.1's `id-ecPublicKey`, `1.2.840.10045.2.1`.
///
/// Table 260 names this family by pointing at that RFC — "ECDSA Algorithm Support ( defined by
/// Internet RFC 5480 )" — and the identifier itself is `const_oid`'s reading of the registry
/// rather than digits typed here. Which curve, and whether this program computes on it, is
/// [`EcCurve`]'s question.
const ID_EC_PUBLIC_KEY: const_oid::ObjectIdentifier = const_oid::db::rfc5912::ID_EC_PUBLIC_KEY;

/// RFC 5280's `id-ce-subjectKeyIdentifier`, `2.5.29.14`.
const SUBJECT_KEY_IDENTIFIER: &[u8] = &[0x55, 0x1D, 0x0E];

/// RFC 5280's `id-ce-keyUsage`, `2.5.29.15`.
const KEY_USAGE: const_oid::ObjectIdentifier = const_oid::db::rfc5280::ID_CE_KEY_USAGE;

/// RFC 5280 section 4.2.1.12's `id-ce-extKeyUsage`, which RFC 6960 section 4.2.2.2 reads.
const EXTENDED_KEY_USAGE: const_oid::ObjectIdentifier = const_oid::db::rfc5280::ID_CE_EXT_KEY_USAGE;

/// The same identifier as octets, for a caller deciding whether it recognised this extension.
///
/// `pdf_signature::trust` is that caller: whether a critical `extKeyUsage` is *unrecognised* depends on
/// whether the use was stated, which is a question this module cannot answer.
pub const EXTENDED_KEY_USAGE_OID: &[u8] = EXTENDED_KEY_USAGE.as_bytes();

/// RFC 5280 section 4.2.1.4's `id-ce-certificatePolicies`, `2.5.29.32`.
const CERTIFICATE_POLICIES: &[u8] = &[0x55, 0x1D, 0x20];

/// RFC 5280 section 4.2.1.5's `id-ce-policyMappings`, `2.5.29.33`.
const POLICY_MAPPINGS: &[u8] = &[0x55, 0x1D, 0x21];

/// RFC 5280 section 4.2.1.6's `id-ce-subjectAltName`, `2.5.29.17`.
const SUBJECT_ALT_NAME: &[u8] = &[0x55, 0x1D, 0x11];

/// RFC 5280 section 4.2.1.10's `id-ce-nameConstraints`, `2.5.29.30`.
const NAME_CONSTRAINTS: &[u8] = &[0x55, 0x1D, 0x1E];

/// RFC 5280 section 4.2.1.11's `id-ce-policyConstraints`, `2.5.29.36`.
const POLICY_CONSTRAINTS: &[u8] = &[0x55, 0x1D, 0x24];

/// RFC 5280 section 4.2.1.14's `id-ce-inhibitAnyPolicy`, `2.5.29.54`.
const INHIBIT_ANY_POLICY: &[u8] = &[0x55, 0x1D, 0x36];

/// RFC 5280's `id-ce-basicConstraints`, `2.5.29.19`.
const BASIC_CONSTRAINTS: const_oid::ObjectIdentifier =
    const_oid::db::rfc5280::ID_CE_BASIC_CONSTRAINTS;

/// RFC 5280's `id-ce-authorityKeyIdentifier`, `2.5.29.35`.
const AUTHORITY_KEY_IDENTIFIER: const_oid::ObjectIdentifier =
    const_oid::db::rfc5280::ID_CE_AUTHORITY_KEY_IDENTIFIER;

/// X.690's `BOOLEAN`, primitive and universal.
const BOOLEAN: u8 = 0x01;

/// X.690's `BIT STRING`, primitive and universal.
const BIT_STRING: u8 = 0x03;

/// X.690's `UTCTime`, primitive and universal.
const UTC_TIME: u8 = 0x17;

/// X.690's `GeneralizedTime`, primitive and universal.
const GENERALIZED_TIME: u8 = 0x18;

/// How many extensions are looked at before the walk stops.
///
/// RFC 5280 bounds the number of extensions at nothing at all, and only one of them is read here,
/// so this is a budget on a loop whose input is a stranger's file rather than an opinion about
/// certificates. A certificate with more than this many extensions still yields its key; what it
/// loses is a subject key identifier past the bound, which makes the signer *unmatched* rather
/// than wrongly matched.
pub const MAX_EXTENSIONS: usize = 64;

/// What stopped a certificate from being read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum X509Error {
    /// The ASN.1 encoding itself is malformed.
    #[error("the certificate is not well-formed ASN.1: {0}")]
    Encoding(#[from] DerError),
    /// The outermost value is not RFC 5280's `Certificate SEQUENCE`.
    #[error("the certificate is not an X.509 Certificate")]
    NotACertificate,
    /// `TBSCertificate`'s members are not in RFC 5280's order or shape.
    #[error("the certificate's TBSCertificate is malformed")]
    MalformedTbs,
    /// `subjectPublicKeyInfo` is absent or is not an algorithm identifier and a `BIT STRING`.
    #[error("the certificate states no readable subjectPublicKeyInfo")]
    NoPublicKey,
    /// The key is `rsaEncryption` but its `RSAPublicKey` is not a modulus and an exponent.
    #[error("the certificate's RSA public key is malformed")]
    MalformedRsaKey,
    /// The key is `id-dsa` but its `Dss-Parms` or its `DSAPublicKey` is not shaped like one.
    #[error("the certificate's DSA public key is malformed")]
    MalformedDsaKey,
    /// The key is `id-ecPublicKey` but its `subjectPublicKey` is not there to read.
    ///
    /// The octets are not checked for being a point here — that is arithmetic and belongs to
    /// `pdf_signature::ecdsa::verify`, which refuses one by name. This is the encoding failing first: a
    /// `BIT STRING` with unused bits, which a SEC1 point never has.
    #[error("the certificate's elliptic-curve public key is malformed")]
    MalformedEcKey,
    /// The key is `id-Ed25519` or `id-Ed448` and its `subjectPublicKey` is not there to read.
    ///
    /// RFC 8410 section 4 puts the key in the `BIT STRING` raw, so this is the same failure as
    /// above and for the same reason: whether the octets are a point is `pdf_signature::eddsa::verify`'s
    /// question.
    #[error("the certificate's Edwards-curve public key is malformed")]
    MalformedEdKey,
    /// The key is `id-dsa` and states no domain parameters, so there is nothing to verify with.
    ///
    /// RFC 3279 section 2.3.2 permits their absence and says where they come from instead — the
    /// issuer's certificate, or "other means" — and ends with a `MUST`: "\[i]f the
    /// `subjectPublicKeyInfo` `AlgorithmIdentifier` field omits the parameters component, the CA signed
    /// the subject with a signature algorithm other than DSA, and the subject's DSA parameters are
    /// not available through other means, then clients MUST reject the certificate." This program
    /// walks no chain and has no other means, so it rejects — by name, and never by guessing at
    /// parameters somebody else's key might have used.
    #[error("the certificate's DSA public key states no domain parameters")]
    NoDsaParameters,
}

/// A point in time, as seconds before or after 1970-01-01T00:00:00Z.
///
/// RFC 5280 section 6.1.1 makes "the current date/time" input (b) to path validation rather
/// than something the algorithm goes and finds, and this type is that input's shape. It is why
/// `pdf_signature::trust` needs no clock: a caller that has one supplies the instant, and a caller
/// that has none — a test, a batch run reproducing a verdict — supplies the instant it means.
///
/// Seconds and not a richer calendar type because that is all the comparison in RFC 5280
/// section 6.1.3 (a)(2) needs: whether an instant lies between two others.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant(i64);

impl Instant {
    /// An instant from seconds since the Unix epoch.
    #[must_use]
    pub const fn from_unix_seconds(seconds: i64) -> Self {
        Self(seconds)
    }

    /// The seconds since the Unix epoch this instant is.
    #[must_use]
    pub const fn unix_seconds(self) -> i64 {
        self.0
    }
}

/// RFC 5280 section 4.1.2.5's `Validity ::= SEQUENCE { notBefore Time, notAfter Time }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Validity {
    /// The first instant at which the certificate is current.
    pub not_before: Instant,
    /// The last instant at which it is.
    pub not_after: Instant,
}

impl Validity {
    /// Whether the period includes an instant, as RFC 5280 section 6.1.3 (a)(2) asks.
    ///
    /// "The certificate validity period includes the current time."
    ///
    /// Inclusive at both ends, which is what section 4.1.2.5's "the period of time over which
    /// the CA warrants that it will maintain information about the status of the certificate"
    /// says: the two instants are in the period rather than either side of it.
    #[must_use]
    pub const fn includes(&self, at: Instant) -> bool {
        self.not_before.0 <= at.0 && at.0 <= self.not_after.0
    }
}

/// RFC 5280 section 4.2.1.9's `BasicConstraints`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BasicConstraints {
    /// `cA`, whose default is `FALSE`.
    ///
    /// Section 4.2.1.9: "The cA boolean indicates whether the certified public key may be used to
    /// verify certificate signatures."
    pub ca: bool,
    /// `pathLenConstraint`, where the certificate states one.
    ///
    /// Section 4.2.1.9: "Where pathLenConstraint does not appear, no limit is imposed."
    pub path_len: Option<u32>,
}

/// RFC 5280 section 4.2.1.3's `KeyUsage BIT STRING`, as the bits it asserts.
///
/// Only the two a certification path turns on are named: `keyCertSign`, which section 6.1.4 (n)
/// requires of every certificate that signs another, and `digitalSignature` with
/// `nonRepudiation` beside it, which are what an end-entity signing certificate asserts. The
/// rest of the nine are carried in the word and asked about by nothing here. `cRLSign` is the
/// fourth, which RFC 5280 section 6.3.3 (f) asks of a CRL's issuer.
///
/// [`Default`] is a `keyUsage` extension asserting no bit at all — legal, meaningless in practice,
/// and the one value a test can build to watch a check refuse.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyUsage {
    /// The named bits, in the order section 4.2.1.3 lists them, bit 0 first.
    bits: u16,
}

impl KeyUsage {
    /// `digitalSignature`, bit 0.
    #[must_use]
    pub const fn digital_signature(self) -> bool {
        self.bits & (1 << 0) != 0
    }

    /// `nonRepudiation`, bit 1 — "contentCommitment" in the comment section 4.2.1.3 attaches.
    #[must_use]
    pub const fn non_repudiation(self) -> bool {
        self.bits & (1 << 1) != 0
    }

    /// `keyCertSign`, bit 5.
    #[must_use]
    pub const fn key_cert_sign(self) -> bool {
        self.bits & (1 << 5) != 0
    }

    /// `cRLSign`, bit 6.
    #[must_use]
    pub const fn crl_sign(self) -> bool {
        self.bits & (1 << 6) != 0
    }
}

/// What a certificate's extensions say, as far as a certification path reads them.
///
/// **Ten are recognised and every other critical one is a refusal**, which is RFC 5280
/// section 4.2's own rule rather than a budget:
///
/// "A certificate-using system MUST reject the certificate if it encounters a critical extension
/// it does not recognize or a critical extension that contains information that it cannot
/// process."
///
/// [`Self::unrecognised_critical`] is how that rejection reaches `pdf_signature::trust`. The six
/// extensions of the policy tree and the name constraints are carried undecoded, and
/// `pdf_signature::trust` decodes each where section 6.1 processes it, refusing the path where one does
/// not read (ADR 2724).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Extensions<'a> {
    /// `subjectKeyIdentifier`'s octets, which name this certificate to the one below it.
    pub key_identifier: Option<&'a [u8]>,
    /// `authorityKeyIdentifier`'s `keyIdentifier` octets, which name its issuer.
    ///
    /// A hint for building a path and never a verdict: RFC 5280 section 6.1.3 (a)(4) chains on
    /// the *name*, so this narrows a search and decides nothing.
    pub authority_key_identifier: Option<&'a [u8]>,
    /// `basicConstraints`, where the certificate states it.
    pub basic_constraints: Option<BasicConstraints>,
    /// `keyUsage`, where the certificate states it.
    pub key_usage: Option<KeyUsage>,
    /// `extKeyUsage`'s encoded `ExtKeyUsageSyntax`, where the certificate states it.
    ///
    /// Carried as its encoding rather than as a decoded list because each consumer asks it one
    /// question, [`indicates_purpose`]: RFC 6960 section 4.2.2.2 makes `id-kp-OCSPSigning` here
    /// the whole of what designates an authorised responder, which `pdf_signature::revocation` reads,
    /// and RFC 3161 section 2.3 requires `id-kp-timeStamping` of a timestamp authority, which
    /// `pdf_signature::trust` reads for `pdf_signature::trust::Purpose`.
    ///
    /// **Reading it does not make it recognised.** A critical `extKeyUsage` still lands in
    /// [`Self::unrecognised_critical`], because RFC 5280 section 4.2.1.12 says what a critical one
    /// means — "the certificate SHALL only be used for one of the purposes indicated" — and only a
    /// caller with a use can check a purpose against it: `pdf_signature::trust` lifts the refusal where
    /// the caller stated a `pdf_signature::trust::Purpose` and keeps it where the caller did not.
    pub extended_key_usage: Option<&'a [u8]>,
    /// `certificatePolicies`' encoded `CertificatePolicies` — RFC 5280 section 4.2.1.4.
    pub certificate_policies: Option<&'a [u8]>,
    /// `policyMappings`' encoded `PolicyMappings` — section 4.2.1.5.
    pub policy_mappings: Option<&'a [u8]>,
    /// `subjectAltName`'s encoded `GeneralNames` — section 4.2.1.6, read for the name constraints.
    pub subject_alt_name: Option<&'a [u8]>,
    /// `nameConstraints`' encoded `NameConstraints` — section 4.2.1.10.
    pub name_constraints: Option<&'a [u8]>,
    /// `policyConstraints`' encoded `PolicyConstraints` — section 4.2.1.11.
    pub policy_constraints: Option<&'a [u8]>,
    /// `inhibitAnyPolicy`'s encoded `SkipCerts` — section 4.2.1.14.
    pub inhibit_any_policy: Option<&'a [u8]>,
    /// The first critical extension this reader does not recognise, as its encoded identifier.
    pub unrecognised_critical: Option<&'a [u8]>,
    /// Whether the walk stopped at [`MAX_EXTENSIONS`] with extensions left unread.
    ///
    /// A certificate that reaches this has extensions nobody here looked at, which — since one
    /// of them may be critical — is refused by `pdf_signature::trust` rather than read as an absence.
    pub truncated: bool,
}

/// A public key, as far as this program can act on one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicKey<'a> {
    /// RFC 8017's `rsaEncryption`, with the two integers `pdf_signature::pkcs1::verify` needs.
    Rsa(RsaPublicKey<'a>),
    /// RFC 3279's `id-dsa`, with the four integers `pdf_signature::dsa::verify` needs.
    Dsa(DsaPublicKey<'a>),
    /// RFC 5480's `id-ecPublicKey` on one of ISO/TS 32002 Table 3's six curves.
    Ec(EcPublicKey<'a>),
    /// RFC 8410's `id-Ed25519` or `id-Ed448`, ISO/TS 32002 Table 4's two.
    EdDsa(EdPublicKey<'a>),
    /// `id-ecPublicKey` on a curve this program does not compute on, or on none it can name.
    ///
    /// Two cases, and the difference between them is whose fault it is. `Some` is one of
    /// ISO/TS 32002 Table 3's Brainpool curves — a curve the standard admits and this program
    /// lacks, so a gap here.
    /// `None` is `ECParameters` that are not a `namedCurve` at all, which ISO/TS 32002 section
    /// 5.1.3 forbids outright: "The implicitCurve and specifiedCurve options shall not be used."
    /// A curve identifier outside both lists arrives as `Some` too, because the file's own digits
    /// are what a reader can check either way.
    EcCurveNotVerifiable {
        /// The `namedCurve` object identifier, as encoded, where the certificate states one.
        curve: Option<&'a [u8]>,
    },
    /// Any other algorithm, carried as the object identifier the certificate states.
    ///
    /// **Named by its number and not by a word**, which is `CLAUDE.md` principle 5 applied to a
    /// document this tree does not hold: neither ISO 32000-2 nor either Technical Specification
    /// prints a digit of an object identifier, so the file's own digits are the only claim that
    /// can be checked. [`dotted`] is what turns the encoding into them.
    Unverifiable {
        /// The `AlgorithmIdentifier`'s object identifier, as encoded.
        algorithm: &'a [u8],
    },
}

/// One X.509 certificate, read for a public key and for the two ways a signer names one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Certificate<'a> {
    /// `tbsCertificate.serialNumber`'s contents, as the file encodes the integer.
    pub serial_number: &'a [u8],
    /// `tbsCertificate.issuer`'s contents — the encoded `Name`, never decoded.
    pub issuer: &'a [u8],
    /// `tbsCertificate.subject`'s contents, likewise.
    pub subject: &'a [u8],
    /// `tbsCertificate.subject`'s whole encoding, header included.
    ///
    /// [`Self::subject`] is what a path *compares*; this is what a hasher of a `Name` wants, and
    /// RFC 6960 section 4.1.1 is the one thing that wants it: a `CertID`'s `issuerNameHash` is the
    /// "Hash of issuer's DN", which is the encoded `Name` and not a `SEQUENCE`'s contents.
    pub subject_encoding: &'a [u8],
    /// The `subjectKeyIdentifier` extension's octets, where the certificate carries one.
    pub key_identifier: Option<&'a [u8]>,
    /// `subjectPublicKeyInfo`, as far as this program reads it.
    pub public_key: PublicKey<'a>,
    /// `subjectPublicKeyInfo.subjectPublicKey`'s octets, the unused-bits octet excluded.
    ///
    /// RFC 6960 section 4.1.1 says exactly this slice and says why: a `CertID`'s `issuerKeyHash`
    /// is the "SHA-1 hash of responder's public key (excluding the tag and length fields)".
    /// Empty where the `BIT STRING` states unused trailing bits, which a key never does.
    pub public_key_bits: &'a [u8],
    /// `tbsCertificate.version`, as 1, 2 or 3 rather than as the 0, 1 or 2 it encodes.
    ///
    /// RFC 5280 section 6.1.4 (k) is the one step that asks: the basic constraints requirement
    /// it states is about "a version 3 certificate", and it says what to do with the others —
    /// "Conforming implementations may choose to reject all version 1 and version 2
    /// intermediate certificates."
    pub version: u8,
    /// `tbsCertificate`'s own encoding, which is what the issuer's signature covers.
    ///
    /// RFC 5280 section 4.1.1.3: the `signatureValue` is "generated upon the ASN.1 DER encoded
    /// tbsCertificate", so these are the octets `pdf_signature::trust` verifies over — the file's own
    /// bytes, never a re-encoding.
    pub tbs: &'a [u8],
    /// Whether the certificate, its `tbsCertificate` or its `subjectPublicKeyInfo` stated X.690
    /// clause 8.1.3.6's indefinite length.
    ///
    /// DER forbids it, [`crate::der`] accepts it for the reason that module states, and a
    /// certification path may not. **The `tbsCertificate`'s is the one that is load-bearing**:
    /// its extent would then have been found by scanning for an end-of-contents marker rather
    /// than read from a length, so [`Self::tbs`] is not the encoding its issuer signed in any
    /// checkable sense. The other two are reported with it because a producer that wrote one
    /// indefinite length at this level wrote a certificate nothing here should place in a path;
    /// a value *nested* inside `tbsCertificate` is not asked about, and does not need to be —
    /// whatever it holds is inside the octets the issuer signed either way.
    /// `pdf_signature::trust` refuses such a certificate by name.
    pub indefinite_lengths: bool,
    /// The outer `signatureAlgorithm`'s object identifier — how the issuer signed this.
    pub signature_algorithm: &'a [u8],
    /// The outer `signatureAlgorithm`'s `parameters`, where it states any.
    ///
    /// RFC 8017 Appendix A.2.3's `RSASSA-PSS-params` arrive here, which is why the member is
    /// carried rather than skipped: the padding is not decided by the identifier alone. Carried
    /// as the value rather than as its bytes because that is what `pdf_signature::pss::parameters`
    /// reads, which is the one consumer there is.
    pub signature_parameters: Option<Value<'a>>,
    /// `signatureValue`'s octets — the issuer's signature over [`Self::tbs`].
    pub signature: &'a [u8],
    /// `tbsCertificate.validity`, where both of its instants are readable.
    ///
    /// `None` rather than an error: a certificate whose dates this reader cannot make sense of
    /// still yields its public key, so question 2 is unaffected, and it is question 3 —
    /// `pdf_signature::trust` — that refuses to place such a certificate in a path.
    pub validity: Option<Validity>,
    /// What the extensions say, as far as a certification path reads them.
    pub extensions: Extensions<'a>,
}

impl Certificate<'_> {
    /// Whether this certificate is the one a `SignerInfo` names by issuer and serial number.
    ///
    /// RFC 5652's `IssuerAndSerialNumber` identifies "the certificate of the signer" by exactly
    /// that pair. Both sides are compared as the encodings the file wrote, with the serial's
    /// leading zero octets ignored so that a producer writing a non-minimal `INTEGER` still
    /// matches — the value is the same number either way, and refusing over the spelling would
    /// answer "no certificate" for a file that carries one.
    #[must_use]
    pub fn is_named_by(&self, issuer: &[u8], serial: &[u8]) -> bool {
        self.issuer == issuer && trim(self.serial_number) == trim(serial)
    }
}

/// Leading zero octets removed, so that two spellings of one integer compare equal.
fn trim(bytes: &[u8]) -> &[u8] {
    let leading = bytes
        .iter()
        .take_while(|&&byte| byte == 0)
        .count()
        .min(bytes.len());
    bytes.get(leading..).unwrap_or(&[])
}

/// An encoded object identifier as the dotted decimal a person reads.
///
/// X.690 clause 8.19: the first octet carries two components as `40 * first + second`, and every
/// component after them is a base-128 number with the high bit set on all but its last octet. The
/// result is `None` for an encoding that ends mid-component or whose components overflow, because
/// a partly-decoded identifier printed as though it were whole would be a wrong number rather than
/// an absent one.
///
/// This exists for one purpose: naming an algorithm this program will not verify. [`crate::der`]
/// deliberately decodes no identifier — every use there is a comparison against a constant — and
/// that stayed true until something had to *report* one it did not recognise.
#[must_use]
pub fn dotted(oid: &[u8]) -> Option<String> {
    use std::fmt::Write as _;
    let mut out = String::new();
    let mut value = 0u64;
    let mut started = false;
    let mut pending = false;
    for &octet in oid {
        // Seven bits per octet; a component wider than 64 bits is refused rather than wrapped.
        value = value
            .checked_mul(128)?
            .checked_add(u64::from(octet & 0x7F))?;
        pending = true;
        if octet & 0x80 != 0 {
            continue;
        }
        if started {
            let _ = write!(out, ".{value}");
        } else {
            // X.690 clause 8.19.4: the first two components share one number, and the first is
            // 0, 1 or 2 with the second bounded by 39 for the first two of those.
            let first = (value / 40).min(2);
            let second = value.checked_sub(first.checked_mul(40)?)?;
            let _ = write!(out, "{first}.{second}");
            started = true;
        }
        value = 0;
        pending = false;
    }
    (started && !pending).then_some(out)
}

/// Reads a certificate from its DER encoding.
///
/// # Errors
///
/// An [`X509Error`] naming what the bytes are instead.
pub fn parse(bytes: &[u8]) -> Result<Certificate<'_>, X509Error> {
    let mut reader = Reader::new(bytes)?;
    let Some(certificate) = reader.next_value()? else {
        return Err(X509Error::NotACertificate);
    };
    read(certificate)
}

/// The same, for a certificate already read as one value of an enclosing structure.
///
/// This is the shape CMS hands over: `SignedData`'s `certificates [0] IMPLICIT` holds the
/// certificates as members, so `pdf_signature::cms::SignedData::certificates` carries them as values
/// rather than as byte strings and nothing re-parses an outer header.
///
/// # Errors
///
/// An [`X509Error`] naming what the value is instead.
pub fn read(certificate: Value<'_>) -> Result<Certificate<'_>, X509Error> {
    if certificate.identifier != SEQUENCE {
        return Err(X509Error::NotACertificate);
    }
    // `Certificate ::= SEQUENCE { tbsCertificate, signatureAlgorithm, signatureValue }`. All
    // three are read: the second and third are this certificate's *issuer's* signature over the
    // first, which is RFC 5280 section 6.1.3 (a)(1) and is what `pdf_signature::trust` checks.
    let mut outer = certificate.children()?;
    let Some(tbs) = outer.next_value()? else {
        return Err(X509Error::MalformedTbs);
    };
    if tbs.identifier != SEQUENCE {
        return Err(X509Error::MalformedTbs);
    }
    let (signature_algorithm, signature_parameters) = match outer.next_value()? {
        Some(algorithm) if algorithm.identifier == SEQUENCE => {
            let mut parts = algorithm.children()?;
            let oid = parts.next_value()?.and_then(|oid| oid.object_identifier());
            (oid.unwrap_or(&[]), parts.next_value()?)
        }
        _ => (&[][..], None),
    };
    let signature = match outer.next_value()? {
        Some(value) if value.identifier == BIT_STRING => key_octets(&value).unwrap_or(&[]),
        _ => &[],
    };
    let mut indefinite_lengths = certificate.had_indefinite_length() || tbs.had_indefinite_length();
    let mut members = tbs.children()?;
    let Some(first) = members.next_value()? else {
        return Err(X509Error::MalformedTbs);
    };
    // `version [0] EXPLICIT Version DEFAULT v1` — absent on a version 1 certificate, so the serial
    // number is either the first member or the second, and which it is is decided by the tag
    // rather than by counting.
    let (version, serial) = if first.is_context(0) {
        let Some(serial) = members.next_value()? else {
            return Err(X509Error::MalformedTbs);
        };
        // `Version ::= INTEGER { v1(0), v2(1), v3(2) }` inside the `[0] EXPLICIT`, so the number
        // on the wire is one less than the version everybody says out loud. A `[0]` holding
        // something this reader cannot make a number of is read as version 1, which is the
        // strictest answer available: section 6.1.4 (k) then refuses it as an intermediate.
        let stated = first
            .children()?
            .next_value()?
            .filter(|value| value.identifier == INTEGER)
            .and_then(|value| value.contents.last().copied())
            .unwrap_or(0);
        (stated.saturating_add(1), serial)
    } else {
        (1, first)
    };
    if serial.identifier != INTEGER {
        return Err(X509Error::MalformedTbs);
    }
    // signature AlgorithmIdentifier, issuer Name, validity Validity, subject Name,
    // subjectPublicKeyInfo SubjectPublicKeyInfo — positional, which is what RFC 5280's SEQUENCE
    // makes them, and every one of the four skipped or taken here is a `SEQUENCE` so no shape
    // test could tell them apart anyway.
    let Some(_algorithm) = members.next_value()? else {
        return Err(X509Error::MalformedTbs);
    };
    let Some(issuer) = members.next_value()? else {
        return Err(X509Error::MalformedTbs);
    };
    let Some(validity) = members.next_value()? else {
        return Err(X509Error::MalformedTbs);
    };
    let validity = read_validity(validity)?;
    let Some(subject) = members.next_value()? else {
        return Err(X509Error::MalformedTbs);
    };
    let Some(spki) = members.next_value()? else {
        return Err(X509Error::NoPublicKey);
    };
    let public_key = read_public_key(spki)?;
    // The same `BIT STRING` [`read_public_key`] decoded, kept undecoded: RFC 6960 hashes these
    // octets, and re-encoding a key to hash it would be this program choosing a producer's
    // encoding for it.
    let public_key_bits = spki_key_octets(&spki).unwrap_or(&[]);
    // `extensions [3] EXPLICIT Extensions OPTIONAL`, after two optional unique identifiers.
    let mut extensions = Extensions::default();
    while let Some(member) = members.next_value()? {
        if member.is_context(3) {
            extensions = read_extensions(member)?;
            break;
        }
    }
    indefinite_lengths |= spki.had_indefinite_length();
    Ok(Certificate {
        serial_number: serial.contents,
        issuer: issuer.contents,
        subject: subject.contents,
        subject_encoding: subject.encoding(),
        key_identifier: extensions.key_identifier,
        public_key,
        public_key_bits,
        version,
        tbs: tbs.encoding(),
        indefinite_lengths,
        signature_algorithm,
        signature_parameters,
        signature,
        validity,
        extensions,
    })
}

/// `Validity ::= SEQUENCE { notBefore Time, notAfter Time }` (RFC 5280 section 4.1.2.5).
///
/// `None` where either instant is not one this reader can place on a line, which keeps a
/// certificate with an unreadable date usable for question 2 and unusable for question 3.
fn read_validity(validity: Value<'_>) -> Result<Option<Validity>, X509Error> {
    if validity.identifier != SEQUENCE {
        return Err(X509Error::MalformedTbs);
    }
    let mut instants = validity.children()?;
    let (Some(before), Some(after)) = (instants.next_value()?, instants.next_value()?) else {
        return Err(X509Error::MalformedTbs);
    };
    Ok(match (read_time(&before), read_time(&after)) {
        (Some(not_before), Some(not_after)) => Some(Validity {
            not_before,
            not_after,
        }),
        _ => None,
    })
}

/// One `Time ::= CHOICE { utcTime UTCTime, generalTime GeneralizedTime }`.
///
/// RFC 5280 section 4.1.2.5.1 fixes the `UTCTime` form — "YYMMDDHHMMSSZ" — and section 4.1.2.5.2
/// the other — "YYYYMMDDHHMMSSZ" — and the first says the same thing twice about the century:
/// "Where YY is greater than or equal to 50, the year SHALL be interpreted as 19YY", and "Where YY
/// is less than 50, the year SHALL be interpreted as 20YY."
///
/// Both must end in `Z`, both must state seconds, and `GeneralizedTime` "MUST NOT include
/// fractional seconds" — so anything else is `None` rather than guessed at. A reader that
/// accepted a local-time offset would be placing a certificate's expiry at an instant its issuer
/// did not write.
#[must_use]
pub fn read_time(value: &Value<'_>) -> Option<Instant> {
    let (four_digit_year, digits) = match value.identifier {
        UTC_TIME => (false, value.contents),
        GENERALIZED_TIME => (true, value.contents),
        _ => return None,
    };
    read_time_digits(digits, four_digit_year)
}

/// The digits of one of those two forms, placed on a line.
///
/// Separate from [`read_time`] because RFC 3161 section 2.4.2's `genTime` is the same
/// `GeneralizedTime` with a fraction of a second permitted after the seconds, which RFC 5280
/// section 4.1.2.5.2 forbids: `pdf_signature::timestamp` strips the fraction and hands the whole seconds
/// here rather than keeping a second copy of the calendar.
#[must_use]
pub fn read_time_digits(digits: &[u8], four_digit_year: bool) -> Option<Instant> {
    // "YYMMDDHHMMSSZ" is thirteen and "YYYYMMDDHHMMSSZ" fifteen; both forms are fixed by the
    // clauses above, so a length that is neither is a `Time` this reader will not place.
    let expected = if four_digit_year { 15 } else { 13 };
    if digits.len() != expected || digits.last() != Some(&b'Z') {
        return None;
    }
    // Every offset below is a `checked_add` on a length this function has already fixed, which is
    // the crate's rule about arithmetic over a stranger's bytes rather than caution about these
    // particular constants.
    let number = |at: usize, width: usize| -> Option<i64> {
        let field = digits.get(at..at.checked_add(width)?)?;
        let text = std::str::from_utf8(field).ok()?;
        text.parse::<i64>().ok()
    };
    let (year, rest) = if four_digit_year {
        (number(0, 4)?, 4usize)
    } else {
        let two = number(0, 2)?;
        let year = if two >= 50 {
            two.checked_add(1900)?
        } else {
            two.checked_add(2000)?
        };
        (year, 2usize)
    };
    let month = number(rest, 2)?;
    let day = number(rest.checked_add(2)?, 2)?;
    let hour = number(rest.checked_add(4)?, 2)?;
    let minute = number(rest.checked_add(6)?, 2)?;
    let second = number(rest.checked_add(8)?, 2)?;
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        // A leap second is `60` and X.690 permits it; it is one second of slack on a bound that
        // is measured in years, so it is admitted rather than refused.
        || second > 60
    {
        return None;
    }
    let days = days_from_civil(year, month, day)?;
    let seconds = days
        .checked_mul(86_400)?
        .checked_add(hour.checked_mul(3_600)?)?
        .checked_add(minute.checked_mul(60)?)?
        .checked_add(second)?;
    Some(Instant(seconds))
}

/// Days from 1970-01-01 to a proleptic Gregorian date, negative before it.
///
/// Howard Hinnant's `days_from_civil`, which is exact for every year this arithmetic can reach
/// and needs no table: the calendar is shifted so that it starts in March, which puts the leap
/// day at the end of a year and makes the day-of-era a closed form. `None` only on an overflow,
/// which the field widths above already exclude.
#[must_use]
pub fn days_from_civil(year: i64, month: i64, day: i64) -> Option<i64> {
    let year = if month <= 2 {
        year.checked_sub(1)?
    } else {
        year
    };
    let shifted = if year >= 0 {
        year
    } else {
        year.checked_sub(399)?
    };
    let era = shifted.checked_div(400)?;
    let year_of_era = year.checked_sub(era.checked_mul(400)?)?;
    let month_shift = if month > 2 {
        month.checked_sub(3)?
    } else {
        month.checked_add(9)?
    };
    let day_of_year = month_shift
        .checked_mul(153)?
        .checked_add(2)?
        .checked_div(5)?
        .checked_add(day)?
        .checked_sub(1)?;
    let day_of_era = year_of_era
        .checked_mul(365)?
        .checked_add(year_of_era.checked_div(4)?)?
        .checked_sub(year_of_era.checked_div(100)?)?
        .checked_add(day_of_year)?;
    era.checked_mul(146_097)?
        .checked_add(day_of_era)?
        .checked_sub(719_468)
}

/// `SubjectPublicKeyInfo ::= SEQUENCE { algorithm AlgorithmIdentifier, subjectPublicKey BIT
/// STRING }`.
fn read_public_key(spki: Value<'_>) -> Result<PublicKey<'_>, X509Error> {
    if spki.identifier != SEQUENCE {
        return Err(X509Error::NoPublicKey);
    }
    let mut parts = spki.children()?;
    let Some(algorithm) = parts.next_value()? else {
        return Err(X509Error::NoPublicKey);
    };
    let Some(oid) = algorithm.children()?.next_value()? else {
        return Err(X509Error::NoPublicKey);
    };
    let Some(oid) = oid.object_identifier() else {
        return Err(X509Error::NoPublicKey);
    };
    let Some(bits) = parts.next_value()? else {
        return Err(X509Error::NoPublicKey);
    };
    if oid == RSA_ENCRYPTION {
        return read_rsa_key(bits);
    }
    if is_dsa(oid) {
        // `AlgorithmIdentifier ::= SEQUENCE { algorithm, parameters ANY OPTIONAL }`, and for
        // `id-dsa` the parameters are `Dss-Parms`. The member after the identifier is therefore
        // read out of the *algorithm* value rather than out of the key.
        let mut members = algorithm.children()?;
        let _identifier = members.next_value()?;
        return read_dsa_key(members.next_value()?, bits);
    }
    if oid == ID_EC_PUBLIC_KEY.as_bytes() {
        // The same shape as `id-dsa`'s: RFC 5480 section 2.1.1 puts `ECParameters` in the
        // `AlgorithmIdentifier`'s `parameters`, so the curve is read out of the *algorithm* value.
        let mut members = algorithm.children()?;
        let _identifier = members.next_value()?;
        return read_ec_key(members.next_value()?, bits);
    }
    if let Some(curve) = EdCurve::of(oid) {
        // RFC 8410 section 3: "the parameters field MUST be absent", and section 4 puts the key in
        // the `BIT STRING` with no structure around it at all.
        let key = key_octets(&bits).ok_or(X509Error::MalformedEdKey)?;
        return Ok(PublicKey::EdDsa(EdPublicKey { curve, key }));
    }
    Ok(PublicKey::Unverifiable { algorithm: oid })
}

/// RFC 5480 section 2.1.1's `ECParameters ::= CHOICE { namedCurve OBJECT IDENTIFIER }` and the
/// SEC1 point the `BIT STRING` carries.
///
/// That clause is what ISO/TS 32002 section 5.1.3 requires be used — "Certificates for ECDSA keys
/// used in PDF signatures shall specify curve parameters (`ECParameters`) for the subject's public
/// key using the namedCurve option" — so anything that is not an `OBJECT IDENTIFIER` here is a
/// file departing from the Technical Specification and is reported as stating no curve rather than
/// guessed at.
fn read_ec_key<'a>(
    parameters: Option<Value<'a>>,
    bits: Value<'a>,
) -> Result<PublicKey<'a>, X509Error> {
    let curve = parameters
        .filter(|parameters| parameters.identifier == OBJECT_IDENTIFIER)
        .and_then(|parameters| parameters.object_identifier());
    let Some(curve) = curve else {
        return Ok(PublicKey::EcCurveNotVerifiable { curve: None });
    };
    let Some(supported) = EcCurve::of(curve) else {
        return Ok(PublicKey::EcCurveNotVerifiable { curve: Some(curve) });
    };
    let point = key_octets(&bits).ok_or(X509Error::MalformedEcKey)?;
    Ok(PublicKey::Ec(EcPublicKey {
        curve: supported,
        point,
    }))
}

/// The contents of a `BIT STRING` that carries a whole number of octets.
///
/// X.690 clause 8.6.2.2: the first contents octet says how many bits of the last one are unused. A
/// key is a whole number of octets, so that octet is zero and the rest is the encapsulated value.
fn key_octets<'a>(bits: &Value<'a>) -> Option<&'a [u8]> {
    let (&unused, encapsulated) = bits.contents.split_first()?;
    (unused == 0).then_some(encapsulated)
}

/// A `SubjectPublicKeyInfo`'s `subjectPublicKey` octets, the unused-bits octet excluded.
///
/// The same `BIT STRING` [`read_public_key`] decodes, kept undecoded: RFC 6960 section 4.1.1
/// hashes these octets, and re-encoding a key to hash it would be this program choosing a
/// producer's encoding on its behalf.
fn spki_key_octets<'a>(spki: &Value<'a>) -> Option<&'a [u8]> {
    let mut members = spki.children().ok()?;
    let _algorithm = members.next_value().ok()??;
    let bits = members.next_value().ok()??;
    (bits.identifier == BIT_STRING)
        .then(|| key_octets(&bits))
        .flatten()
}

/// `RSAPublicKey ::= SEQUENCE { modulus INTEGER, publicExponent INTEGER }` (RFC 8017 section A.1.1).
fn read_rsa_key(bits: Value<'_>) -> Result<PublicKey<'_>, X509Error> {
    let encapsulated = key_octets(&bits).ok_or(X509Error::MalformedRsaKey)?;
    let mut inner = Reader::new(encapsulated)?;
    let Some(key) = inner.next_value()? else {
        return Err(X509Error::MalformedRsaKey);
    };
    let mut numbers = key.children()?;
    let (Some(modulus), Some(exponent)) = (numbers.next_value()?, numbers.next_value()?) else {
        return Err(X509Error::MalformedRsaKey);
    };
    if modulus.identifier != INTEGER || exponent.identifier != INTEGER {
        return Err(X509Error::MalformedRsaKey);
    }
    Ok(PublicKey::Rsa(RsaPublicKey {
        modulus: modulus.contents,
        exponent: exponent.contents,
    }))
}

/// RFC 3279 section 2.3.2's `Dss-Parms ::= SEQUENCE { p INTEGER, q INTEGER, g INTEGER }` and the
/// `DSAPublicKey ::= INTEGER` the `BIT STRING` encapsulates.
///
/// That clause states where `y` sits: "The DSA public key MUST be ASN.1 DER encoded as an INTEGER;
/// this encoding shall be used as the contents (i.e., the value) of the `subjectPublicKey`
/// component (a BIT STRING) of the `SubjectPublicKeyInfo` data element."
fn read_dsa_key<'a>(
    parameters: Option<Value<'a>>,
    bits: Value<'a>,
) -> Result<PublicKey<'a>, X509Error> {
    let parameters = parameters.ok_or(X509Error::NoDsaParameters)?;
    if parameters.identifier != SEQUENCE {
        return Err(X509Error::NoDsaParameters);
    }
    let mut numbers = parameters.children()?;
    let (Some(p), Some(q), Some(g)) = (
        numbers.next_value()?,
        numbers.next_value()?,
        numbers.next_value()?,
    ) else {
        return Err(X509Error::MalformedDsaKey);
    };
    let encapsulated = key_octets(&bits).ok_or(X509Error::MalformedDsaKey)?;
    let mut inner = Reader::new(encapsulated)?;
    let Some(y) = inner.next_value()? else {
        return Err(X509Error::MalformedDsaKey);
    };
    if [p.identifier, q.identifier, g.identifier, y.identifier]
        .iter()
        .any(|&identifier| identifier != INTEGER)
    {
        return Err(X509Error::MalformedDsaKey);
    }
    Ok(PublicKey::Dsa(DsaPublicKey {
        p: p.contents,
        q: q.contents,
        g: g.contents,
        y: y.contents,
    }))
}

/// Every extension, read for the ten a certification path uses and refused for the rest.
///
/// `Extension ::= SEQUENCE { extnID OBJECT IDENTIFIER, critical BOOLEAN DEFAULT FALSE, extnValue
/// OCTET STRING }`, and each recognised extension's value is a further DER encoding *inside* that
/// octet string — so every one of them is two octet strings deep.
///
/// The walk stops at [`MAX_EXTENSIONS`] and says so in [`Extensions::truncated`] rather than
/// returning what it managed: a truncated walk cannot claim there was no critical extension it
/// did not recognise, and that claim is the whole value of the field.
fn read_extensions(extensions: Value<'_>) -> Result<Extensions<'_>, X509Error> {
    let mut read = Extensions::default();
    let Some(list) = extensions.children()?.next_value()? else {
        return Ok(read);
    };
    let mut entries = list.children()?;
    let mut seen = 0usize;
    while let Some(entry) = entries.next_value()? {
        seen = seen.saturating_add(1);
        if seen > MAX_EXTENSIONS {
            read.truncated = true;
            return Ok(read);
        }
        let mut parts = entry.children()?;
        let Some(id) = parts.next_value()? else {
            continue;
        };
        let Some(oid) = id.object_identifier() else {
            continue;
        };
        // `critical BOOLEAN DEFAULT FALSE` sits between the identifier and the value where it is
        // written at all, so both are found by tag rather than by position. X.690 clause 11.1
        // makes the only `TRUE` encoding `FF`, and a BER producer's other non-zero octet is read
        // as true too: the conservative direction, since true is what makes an extension binding.
        let mut critical = false;
        let mut value = None;
        for part in std::iter::from_fn(|| parts.next_value().transpose()) {
            let part = part?;
            match part.identifier {
                BOOLEAN => critical = part.contents.iter().any(|&octet| octet != 0),
                OCTET_STRING => value = Some(part.contents),
                _ => {}
            }
        }
        let inner = match value {
            Some(bytes) => Reader::new(bytes)?.next_value()?,
            None => None,
        };
        // Read beside the chain rather than inside it, so that a *critical* `extKeyUsage` still
        // falls through to the refusal below: reading an extension is not recognising it, and the
        // field's own documentation says why the distinction is load-bearing here.
        if oid == EXTENDED_KEY_USAGE.as_bytes() {
            read.extended_key_usage = Some(value.unwrap_or(&[]));
        }
        if oid == SUBJECT_KEY_IDENTIFIER {
            read.key_identifier = inner
                .filter(|value| value.identifier == OCTET_STRING)
                .map(|value| value.contents);
        } else if oid == AUTHORITY_KEY_IDENTIFIER.as_bytes() {
            read.authority_key_identifier = inner.and_then(|value| authority_key_id(&value));
        } else if oid == BASIC_CONSTRAINTS.as_bytes() {
            read.basic_constraints = inner.and_then(|value| basic_constraints(&value));
        } else if oid == KEY_USAGE.as_bytes() {
            read.key_usage = inner.and_then(|value| key_usage(&value));
        } else if let Some(slot) = match oid {
            CERTIFICATE_POLICIES => Some(&mut read.certificate_policies),
            POLICY_MAPPINGS => Some(&mut read.policy_mappings),
            SUBJECT_ALT_NAME => Some(&mut read.subject_alt_name),
            NAME_CONSTRAINTS => Some(&mut read.name_constraints),
            POLICY_CONSTRAINTS => Some(&mut read.policy_constraints),
            INHIBIT_ANY_POLICY => Some(&mut read.inhibit_any_policy),
            _ => None,
        } {
            // Carried as written; an absent `extnValue` is carried as empty, which no decoder in
            // `pdf_signature::trust` reads, so it is refused there rather than taken as an absence.
            *slot = Some(value.unwrap_or(&[]));
        } else if critical && read.unrecognised_critical.is_none() {
            // RFC 5280 section 4.2: "A certificate-using system MUST reject the certificate if it
            // encounters a critical extension it does not recognize". The first one is kept so a
            // report can name it; the rest change nothing, since one is already a refusal.
            read.unrecognised_critical = Some(oid);
        }
    }
    Ok(read)
}

/// `anyExtendedKeyUsage`, RFC 5280 section 4.2.1.12's `{ id-ce-extKeyUsage 0 }`.
pub const ANY_EXTENDED_KEY_USAGE: &[u8] = &[0x55, 0x1D, 0x25, 0x00];

/// `id-kp-timeStamping`, RFC 3161 section 2.3's `{ id-kp 8 }`.
///
/// The clause prints the arc in full — "iso(1) identified-organization(3) dod(6) internet(1)
/// security(5) mechanisms(5) pkix(7) kp (3) timestamping (8)" — which is why this identifier is a
/// constant here and not a number out of a registry this tree does not hold.
pub const ID_KP_TIME_STAMPING: &[u8] = &[0x2B, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x08];

/// Whether an `ExtKeyUsageSyntax` indicates `purpose`, or `anyExtendedKeyUsage` beside it.
///
/// RFC 5280 section 4.2.1.12 states both halves: "If multiple purposes are indicated the
/// application need not recognize all purposes indicated, as long as the intended purpose is
/// present", and `anyExtendedKeyUsage` exists so that a CA can "include extended key usages to
/// satisfy such applications, but does not wish to restrict usages of the key".
///
/// Bounded by [`MAX_EXTENSIONS`] purposes, because the list's length comes out of the file.
#[must_use]
pub fn indicates_purpose(encoded: &[u8], purpose: &[u8]) -> bool {
    let Ok(mut reader) = Reader::new(encoded) else {
        return false;
    };
    let Ok(Some(list)) = reader.next_value() else {
        return false;
    };
    let Ok(mut purposes) = list.children() else {
        return false;
    };
    let mut seen = 0usize;
    while let Ok(Some(stated)) = purposes.next_value() {
        seen = seen.saturating_add(1);
        if seen > MAX_EXTENSIONS {
            return false;
        }
        if stated.identifier == OBJECT_IDENTIFIER
            && (stated.contents == purpose || stated.contents == ANY_EXTENDED_KEY_USAGE)
        {
            return true;
        }
    }
    false
}

/// `AuthorityKeyIdentifier ::= SEQUENCE { keyIdentifier [0] OPTIONAL, … }` (section 4.2.1.1).
///
/// Only the first member is taken. The other two name the issuer's own issuer and serial, which
/// would be a second way to search and not a second way to decide.
fn authority_key_id<'a>(value: &Value<'a>) -> Option<&'a [u8]> {
    if value.identifier != SEQUENCE {
        return None;
    }
    let mut members = value.children().ok()?;
    let first = members.next_value().ok()??;
    first.is_context(0).then_some(first.contents)
}

/// `BasicConstraints ::= SEQUENCE { cA BOOLEAN DEFAULT FALSE, pathLenConstraint INTEGER OPTIONAL }`.
///
/// An empty `SEQUENCE` is the commonest end-entity spelling and means `cA FALSE` with no limit,
/// which is what the defaults say and what this returns.
fn basic_constraints(value: &Value<'_>) -> Option<BasicConstraints> {
    if value.identifier != SEQUENCE {
        return None;
    }
    let mut members = value.children().ok()?;
    let mut constraints = BasicConstraints {
        ca: false,
        path_len: None,
    };
    for member in std::iter::from_fn(|| members.next_value().transpose()) {
        let member = member.ok()?;
        match member.identifier {
            BOOLEAN => constraints.ca = member.contents.iter().any(|&octet| octet != 0),
            // Section 4.2.1.9: "Where it appears, the pathLenConstraint field MUST be greater
            // than or equal to zero." A negative or oversized one is read as absent, which
            // imposes no limit here but cannot widen one: section 6.1.4 (m) only ever *lowers*
            // `max_path_length`, and `pdf_signature::trust` starts it at its own bound regardless.
            INTEGER => {
                constraints.path_len = member
                    .contents
                    .iter()
                    .try_fold(0u32, |total, &octet| {
                        total.checked_mul(256)?.checked_add(u32::from(octet))
                    })
                    .filter(|_| member.contents.first().is_none_or(|&first| first < 0x80));
            }
            _ => {}
        }
    }
    Some(constraints)
}

/// `KeyUsage ::= BIT STRING { digitalSignature(0), … decipherOnly(8) }` (section 4.2.1.3).
///
/// X.690 clause 8.6.2.2 puts the count of unused trailing bits in the first contents octet and
/// numbers the bits from the most significant end of the second, which is why bit 0 is `0x80` of
/// the first data octet rather than `0x01` of it.
fn key_usage(value: &Value<'_>) -> Option<KeyUsage> {
    if value.identifier != BIT_STRING {
        return None;
    }
    let (&unused, data) = value.contents.split_first()?;
    if unused > 7 {
        return None;
    }
    let mut bits = 0u16;
    for (index, &octet) in data.iter().take(2).enumerate() {
        for offset in 0..8u32 {
            if octet & (0x80 >> offset) != 0 {
                let position = u32::try_from(index)
                    .ok()?
                    .checked_mul(8)?
                    .checked_add(offset);
                if let Some(position) = position.filter(|&position| position < 16) {
                    bits |= 1u16 << position;
                }
            }
        }
    }
    Some(KeyUsage { bits })
}

#[cfg(test)]
mod fixtures {
    /// A self-signed 2048-bit certificate over the key `pkcs1`'s tests verify a signature with.
    ///
    /// **A test vector rather than an oracle**, on the same footing as `pkcs1`'s: RFC 5280 defines
    /// the structure, and what this pins is that these two hundred lines walk it. No corpus
    /// document could stand in — the certificates in the corpus's signatures are real ones whose
    /// keys nobody here holds, which is exactly why the *corpus* gate measures verification and
    /// this test measures parsing.
    pub(crate) const CERTIFICATE: &str = "\
        30820315308201fda0030201020214776792c587fa13475f8a307103e320f377\
        2065cc300d06092a864886f70d01010b0500301a3118301606035504030c0f70\
        64662d7669657765722074657374301e170d3236303830383033333534375a17\
        0d3336303830353033333534375a301a3118301606035504030c0f7064662d76\
        6965776572207465737430820122300d06092a864886f70d0101010500038201\
        0f003082010a0282010100c56a39fbe4fd00ac43c8080e81b5b2a314c57647dd\
        5317854109d621e44713cafabd7fbe5275f933f5956fd158c8fe6dab37447594\
        9366675f4feff22689459fd676925b9b55a1dec6274debe37905a3f843d322bf\
        4495164ec6e626f8c0f198f538d93e9ab8be31250ce1af107a53415c663ddcef\
        d8cef220613c58e1a9870ea2f67576e85d6457019c9b86422b3df59a664089e0\
        d5a9f97f921940eab4d95132a1b19870635d6372e4275c06d39f8943d6a971c4\
        6fa86199e5acbc24ed0a9f7c8aa50a5e57ef9df109c0bac20be6022abede06cc\
        603b1bed0e4d08e7c836af378d310edb7a752f2a9541f6da47b06291daaa0e25\
        ff4b29f6e2fc926f4789b70203010001a3533051301d0603551d0e04160414f9\
        7893bb0d13501fdd2770f997e124d19cc94f33301f0603551d23041830168014\
        f97893bb0d13501fdd2770f997e124d19cc94f33300f0603551d130101ff0405\
        30030101ff300d06092a864886f70d01010b050003820101009a997e66c47d6f\
        e8ed31eb9e3043ab13085fac7e118c1d1e4525847f1fdc8db7f8d2398b1fa786\
        93d541d4387874a3d2bc715ebba5420849e83ed9ebca6fa246834fd568ead25d\
        e1a3bbe9d6f89104b211e759f81c7d7545a2b3b95a9e0834bca043805aad2224\
        8ded832ca711f32c3063253c7b116bd789f730d0738a5be9af168fcb569da0bc\
        bc84b77c05f2a97d45c8475246ef543222fc6379b62de33f6f2d0fa50533b432\
        61c30c8f2c64533ed5474ba7ce8c8aae96ca3b128bec075dbe29f63be385ed57\
        0573cd49e8d892d706e10a8d1ff54dec1f258d676c4363d028839b8f6b5040bd\
        5168122013babed7b4b558679467ed2e7ca42a41e3deda7d01";

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
    use super::fixtures::{CERTIFICATE, hex};
    use super::{PublicKey, X509Error, dotted, parse};

    /// Everything this module claims to read, read.
    #[test]
    fn a_certificate_yields_its_key_and_the_two_names_a_signer_uses() {
        let bytes = hex(CERTIFICATE);
        let certificate = parse(&bytes).expect("a certificate");
        let PublicKey::Rsa(key) = certificate.public_key else {
            panic!("this certificate states rsaEncryption");
        };
        assert_eq!(key.bits(), 2048);
        assert_eq!(key.exponent, [0x01, 0x00, 0x01]);
        assert_eq!(
            certificate.serial_number.len(),
            20,
            "a twenty-octet serial number, as written"
        );
        assert_eq!(
            certificate.issuer, certificate.subject,
            "self-signed: the two names are the same encoding"
        );
        assert_eq!(
            certificate.key_identifier.map(<[u8]>::len),
            Some(20),
            "the subjectKeyIdentifier extension, one SHA-1 wide"
        );
        // And the signer identification it exists for, both ways round.
        assert!(certificate.is_named_by(certificate.issuer, certificate.serial_number));
        assert!(
            certificate.is_named_by(
                certificate.issuer,
                &[&[0x00][..], certificate.serial_number].concat()
            ),
            "a non-minimal INTEGER is the same number"
        );
        assert!(!certificate.is_named_by(certificate.issuer, &[0x01]));
        assert!(!certificate.is_named_by(&[], certificate.serial_number));
    }

    /// X.690 clause 8.19's encoding, decoded — including the two components in one octet.
    #[test]
    fn an_object_identifier_becomes_the_number_a_person_reads() {
        assert_eq!(
            dotted(&[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x01]).as_deref(),
            Some("1.2.840.113549.1.1.1")
        );
        assert_eq!(dotted(&[0x55, 0x1D, 0x0E]).as_deref(), Some("2.5.29.14"));
        assert_eq!(
            dotted(&[0x2B, 0x0E, 0x03, 0x02, 0x1A]).as_deref(),
            Some("1.3.14.3.2.26")
        );
        // The two names Table 260 puts beside RSA, so that this program can print them without
        // claiming to know what they are called.
        assert_eq!(
            dotted(&[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x02, 0x01]).as_deref(),
            Some("1.2.840.10045.2.1")
        );
        // An encoding that ends mid-component is refused rather than half-printed.
        assert_eq!(dotted(&[0x2A, 0x86]), None);
        assert_eq!(dotted(&[]), None);
    }

    /// Every way a certificate can fail to be one, named rather than shrugged at.
    #[test]
    fn what_is_not_a_certificate_says_what_it_is() {
        assert_eq!(parse(&[]), Err(X509Error::NotACertificate));
        assert_eq!(parse(&[0x04, 0x01, 0x00]), Err(X509Error::NotACertificate));
        // A SEQUENCE whose first member is not a TBSCertificate.
        assert_eq!(
            parse(&[0x30, 0x03, 0x02, 0x01, 0x01]),
            Err(X509Error::MalformedTbs)
        );
        // A TBSCertificate that stops after its serial number.
        assert_eq!(
            parse(&[0x30, 0x05, 0x30, 0x03, 0x02, 0x01, 0x01]),
            Err(X509Error::MalformedTbs)
        );
    }
}
