//! The signer's certificate, read for the one thing a verifier needs from it: the public key.
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
//! # What is read and what is deliberately not
//!
//! Read: the serial number and the issuer, because RFC 5652's `SignerInfo` names its certificate
//! by that pair; the subject key identifier extension, because it names one the other way; and
//! `subjectPublicKeyInfo`, which is the key. That is the whole of what verifying needs.
//!
//! **Not read: every field that is a *trust* decision.** The validity dates, the basic constraints,
//! the key usage, the issuer's own signature over this certificate, the chain above it. Those are
//! a signature's *third* question — "is the signer anyone to believe" — and this program answers
//! none of it, has no certificate store and makes no network request. Reading a `notAfter` and
//! saying nothing about who issued the certificate would be the worst of both: an air of validation
//! over a certificate that could have been made up five minutes ago by whoever wrote the file.
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
use crate::{dsa, ecdsa, eddsa, pkcs1};

/// RFC 8017's `rsaEncryption`, `1.2.840.113549.1.1.1`.
///
/// One of the two public-key algorithm identifiers this program acts on, the other being
/// [`crate::dsa::ID_DSA`]. Everything else is carried to the report as the object identifier the
/// file states — see [`PublicKey::Unverifiable`] and [`dotted`].
pub const RSA_ENCRYPTION: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x01];

/// RFC 5480 section 2.1.1's `id-ecPublicKey`, `1.2.840.10045.2.1`.
///
/// Table 260 names this family by pointing at that RFC — "ECDSA Algorithm Support ( defined by
/// Internet RFC 5480 )" — and the identifier itself is `const_oid`'s reading of the registry
/// rather than digits typed here. Which curve, and whether this program computes on it, is
/// [`crate::ecdsa::Curve`]'s question.
const ID_EC_PUBLIC_KEY: const_oid::ObjectIdentifier = const_oid::db::rfc5912::ID_EC_PUBLIC_KEY;

/// RFC 5280's `id-ce-subjectKeyIdentifier`, `2.5.29.14`.
const SUBJECT_KEY_IDENTIFIER: &[u8] = &[0x55, 0x1D, 0x0E];

/// How many extensions are looked at before the walk stops.
///
/// RFC 5280 bounds the number of extensions at nothing at all, and only one of them is read here,
/// so this is a budget on a loop whose input is a stranger's file rather than an opinion about
/// certificates. A certificate with more than this many extensions still yields its key; what it
/// loses is a subject key identifier past the bound, which makes the signer *unmatched* rather
/// than wrongly matched.
const MAX_EXTENSIONS: usize = 64;

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
    /// [`crate::ecdsa::verify`], which refuses one by name. This is the encoding failing first: a
    /// `BIT STRING` with unused bits, which a SEC1 point never has.
    #[error("the certificate's elliptic-curve public key is malformed")]
    MalformedEcKey,
    /// The key is `id-Ed25519` or `id-Ed448` and its `subjectPublicKey` is not there to read.
    ///
    /// RFC 8410 section 4 puts the key in the `BIT STRING` raw, so this is the same failure as
    /// above and for the same reason: whether the octets are a point is [`crate::eddsa::verify`]'s
    /// question.
    #[error("the certificate's Edwards-curve public key is malformed")]
    MalformedEdKey,
    /// The key is `id-dsa` and states no domain parameters, so there is nothing to verify with.
    ///
    /// RFC 3279 section 2.3.2 permits their absence and says where they come from instead — the
    /// issuer's certificate, or "other means" — and ends with a `MUST`: "[i]f the
    /// `subjectPublicKeyInfo` `AlgorithmIdentifier` field omits the parameters component, the CA signed
    /// the subject with a signature algorithm other than DSA, and the subject's DSA parameters are
    /// not available through other means, then clients MUST reject the certificate." This program
    /// walks no chain and has no other means, so it rejects — by name, and never by guessing at
    /// parameters somebody else's key might have used.
    #[error("the certificate's DSA public key states no domain parameters")]
    NoDsaParameters,
}

/// A public key, as far as this program can act on one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicKey<'a> {
    /// RFC 8017's `rsaEncryption`, with the two integers [`crate::pkcs1::verify`] needs.
    Rsa(pkcs1::PublicKey<'a>),
    /// RFC 3279's `id-dsa`, with the four integers [`crate::dsa::verify`] needs.
    Dsa(dsa::PublicKey<'a>),
    /// RFC 5480's `id-ecPublicKey` on one of ISO/TS 32002 Table 3's first three curves.
    Ec(ecdsa::PublicKey<'a>),
    /// RFC 8410's `id-Ed25519`, the first of ISO/TS 32002 Table 4's two.
    Ed25519(eddsa::PublicKey<'a>),
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
    /// can be checked. [`dotted`] is what turns the encoding into them. `id-Ed448` arrives here —
    /// ISO/TS 32002 Table 4 names the curve and [`crate::eddsa`] says why it is not computed.
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
    /// The `subjectKeyIdentifier` extension's octets, where the certificate carries one.
    pub key_identifier: Option<&'a [u8]>,
    /// `subjectPublicKeyInfo`, as far as this program reads it.
    pub public_key: PublicKey<'a>,
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
/// certificates as members, so [`crate::cms::SignedData::certificates`] carries them as values
/// rather than as byte strings and nothing re-parses an outer header.
///
/// # Errors
///
/// An [`X509Error`] naming what the value is instead.
pub fn read(certificate: Value<'_>) -> Result<Certificate<'_>, X509Error> {
    if certificate.identifier != SEQUENCE {
        return Err(X509Error::NotACertificate);
    }
    // `Certificate ::= SEQUENCE { tbsCertificate, signatureAlgorithm, signatureValue }`. Only the
    // first is read: the other two are this certificate's *issuer's* signature over it, which is
    // question three and is not checked anywhere in this program.
    let Some(tbs) = certificate.children()?.next_value()? else {
        return Err(X509Error::MalformedTbs);
    };
    if tbs.identifier != SEQUENCE {
        return Err(X509Error::MalformedTbs);
    }
    let mut members = tbs.children()?;
    let Some(first) = members.next_value()? else {
        return Err(X509Error::MalformedTbs);
    };
    // `version [0] EXPLICIT Version DEFAULT v1` — absent on a version 1 certificate, so the serial
    // number is either the first member or the second, and which it is is decided by the tag
    // rather than by counting.
    let serial = if first.is_context(0) {
        let Some(serial) = members.next_value()? else {
            return Err(X509Error::MalformedTbs);
        };
        serial
    } else {
        first
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
    let Some(_validity) = members.next_value()? else {
        return Err(X509Error::MalformedTbs);
    };
    let Some(subject) = members.next_value()? else {
        return Err(X509Error::MalformedTbs);
    };
    let Some(spki) = members.next_value()? else {
        return Err(X509Error::NoPublicKey);
    };
    let public_key = read_public_key(spki)?;
    // `extensions [3] EXPLICIT Extensions OPTIONAL`, after two optional unique identifiers.
    let mut key_identifier = None;
    while let Some(member) = members.next_value()? {
        if member.is_context(3) {
            key_identifier = read_key_identifier(member)?;
            break;
        }
    }
    Ok(Certificate {
        serial_number: serial.contents,
        issuer: issuer.contents,
        subject: subject.contents,
        key_identifier,
        public_key,
    })
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
    if dsa::is_dsa(oid) {
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
    if oid == eddsa::ID_ED25519.as_bytes() {
        // RFC 8410 section 3: "the parameters field MUST be absent", and section 4 puts the key in
        // the `BIT STRING` with no structure around it at all.
        let key = key_octets(&bits).ok_or(X509Error::MalformedEdKey)?;
        return Ok(PublicKey::Ed25519(eddsa::PublicKey { key }));
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
    let Some(supported) = ecdsa::Curve::of(curve) else {
        return Ok(PublicKey::EcCurveNotVerifiable { curve: Some(curve) });
    };
    let point = key_octets(&bits).ok_or(X509Error::MalformedEcKey)?;
    Ok(PublicKey::Ec(ecdsa::PublicKey {
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
    Ok(PublicKey::Rsa(pkcs1::PublicKey {
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
    Ok(PublicKey::Dsa(dsa::PublicKey {
        p: p.contents,
        q: q.contents,
        g: g.contents,
        y: y.contents,
    }))
}

/// The `subjectKeyIdentifier` extension's octets, where the certificate carries one.
///
/// `Extension ::= SEQUENCE { extnID OBJECT IDENTIFIER, critical BOOLEAN DEFAULT FALSE, extnValue
/// OCTET STRING }`, and the value of this extension is itself a DER `OCTET STRING`, so the
/// identifier is two octet strings deep.
fn read_key_identifier(extensions: Value<'_>) -> Result<Option<&[u8]>, X509Error> {
    let Some(list) = extensions.children()?.next_value()? else {
        return Ok(None);
    };
    let mut entries = list.children()?;
    let mut seen = 0usize;
    while let Some(entry) = entries.next_value()? {
        seen = seen.saturating_add(1);
        if seen > MAX_EXTENSIONS {
            return Ok(None);
        }
        let mut parts = entry.children()?;
        let Some(id) = parts.next_value()? else {
            continue;
        };
        if id.object_identifier() != Some(SUBJECT_KEY_IDENTIFIER) {
            continue;
        }
        // The optional `critical` sits between the identifier and the value, so the octet string
        // is found by tag rather than by position.
        while let Some(part) = parts.next_value()? {
            if part.identifier != OCTET_STRING {
                continue;
            }
            let mut inner = Reader::new(part.contents)?;
            if let Some(identifier) = inner.next_value()?
                && identifier.identifier == OCTET_STRING
            {
                return Ok(Some(identifier.contents));
            }
        }
    }
    Ok(None)
}

/// Certificates built once with `openssl` and pasted in, shared with `signature.rs`'s tests.
///
/// **Test vectors rather than oracles**, on the footing `pkcs1`'s constants are on: RFC 5280
/// defines the structure and RFC 8017 the arithmetic, and what a vector pins is that this tree
/// walks and computes them. No corpus document can stand in for the first: the certificates in
/// the corpus's signatures are real ones whose *private* keys nobody here holds, so a positive
/// verification over bytes this tree chose needs a key this tree made.
#[cfg(test)]
pub(crate) mod fixtures {
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

    /// A self-signed P-256 certificate, for the one `/SubFilter` that may not carry such a key.
    ///
    /// RFC 5480's `id-ecPublicKey`. This program verifies that family through CMS since the
    /// six-hundred-and-eighty-ninth session ([`crate::ecdsa`]), and Table 260's `adbe.x509.rsa_sha1`
    /// column still says **No** to it — so this fixture is what carries that "No" into a test.
    /// [`crate::ecdsa::fixtures`] holds the certificates that are meant to verify.
    pub(crate) const EC_CERTIFICATE: &str = "\
        3082019030820135a003020102021402b9a053ffb599e1d104c767b0ac9516e1\
        6a2827300a06082a8648ce3d040302301d311b301906035504030c127064662d\
        7669657765722065632074657374301e170d3236303830383033343930365a17\
        0d3336303830353033343930365a301d311b301906035504030c127064662d76\
        696577657220656320746573743059301306072a8648ce3d020106082a8648ce\
        3d03010703420004008481356aeb1a6299f333e9e81ebdf2616a4a0618f4ff28\
        6b314b3f849e086b96fd50fcf8b02568b9cb67b003b732130449f0ad0c4542e0\
        ecd6831ab775bc70a3533051301d0603551d0e0416041449a6344e98d3f2463c\
        150455fe60ddd0c08642b5301f0603551d2304183016801449a6344e98d3f246\
        3c150455fe60ddd0c08642b5300f0603551d130101ff040530030101ff300a06\
        082a8648ce3d04030203490030460221008976ebbd9a912abfad19d1f907ce18\
        0c7ff17039ff46c70e836f1c8dc9b073c0022100c2865a409279b22cf83ce663\
        acf64f6a0f1f8b52d1ccbcdb198d60ad5941bd31";

    /// The PKCS #1 v1.5 signature of `SHA-256(b"the signed bytes")` under the RSA key above.
    pub(crate) const PKCS1_SIGNATURE: &str = "\
        a8051d60075059a973c9845268a3c7848a60e36a1904180dd95d4226dad15bea\
        4050dd59907bbb865d915611e55ae1fa1017a71fe084b90695f6ed5ca75fdc91\
        7caf751663b994ed3f6f7cac36729a87b237668eec74aeb6040a682ff5c87e75\
        1763e17a5d010383187a73af9ee5e8e4f890802882baba7dbb0873847dfa55a1\
        db7bd5998ddd9456b79eb269184ac578e1451d29ea8748f6b8d2bfe4c9ec834f\
        9c9f917089d8210aec49c8538e72f05ed93fe4d4b9145acd94db022a4d16b93e\
        7ca648f59a45bcc30786e72cec2ce3147700cd6cebf2e48ff8eb1c777802003b\
        7869fdaa3f4f11773f78f2002bf1a66c019e62946c3c7aed327940e7e0121204";

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
    use super::fixtures::{CERTIFICATE, PKCS1_SIGNATURE, hex};
    use super::{PublicKey, X509Error, dotted, parse};
    use crate::pkcs1;

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

    /// The key this certificate carries is the key that verifies the signature `pkcs1` tests.
    ///
    /// The two modules are only joined here, and this is the join that matters: a certificate
    /// reader that produced a *plausible* modulus would pass every assertion above.
    #[test]
    fn the_key_read_out_of_a_certificate_verifies_a_signature_made_with_it() {
        let bytes = hex(CERTIFICATE);
        let certificate = parse(&bytes).expect("a certificate");
        let PublicKey::Rsa(key) = certificate.public_key else {
            panic!("rsaEncryption");
        };
        let signature = hex(PKCS1_SIGNATURE);
        let digest = crate::cms::Digest::Sha256.compute(&[b"the signed bytes"]);
        assert_eq!(
            pkcs1::verify(key, &signature, crate::cms::Digest::Sha256, &digest),
            Ok(true)
        );
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
