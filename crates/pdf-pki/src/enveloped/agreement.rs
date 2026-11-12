//! RFC 5652 section 6.2.2's `KeyAgreeRecipientInfo`, read and opened for an elliptic-curve key.
//!
//! RFC 5753 section 3.1 is what such a recipient is when its key is on an elliptic curve: the
//! sender's ephemeral public key in `originator`, SEC 1's Diffie–Hellman primitive between it and
//! the recipient's private key ([`crate::ec`]), SEC 1 section 3.6.1's key derivation over the
//! shared value and RFC 5753 section 7.2's `ECC-CMS-SharedInfo` (`x963_kdf`, `shared_info`),
//! and the content-encryption key unwrapped from `encryptedKey` under what that derives
//! ([`crate::key_wrap`]). Section 3.1.3 is the receiving agent's half, and it is this module.
//!
//! # What is run, and what is refused by name
//!
//! RFC 5753 section 7.1.4 names fifteen key-agreement identifiers. The ten
//! `dhSinglePass-stdDH-*` and `dhSinglePass-cofactorDH-*` ones are run, each with its own hash
//! (section 7.1.8: "dhSinglePass-stdDH-sha256kdf-scheme uses the KDF from \[SEC1] but uses
//! SHA-256 instead of SHA-1"); the cofactor form is the standard form on every curve
//! [`crate::ec`] computes on. The five `mqvSinglePass-*` are 1-Pass ECMQV, which section 8 makes
//! "OPTIONAL" and which needs the sender's static key and arithmetic no reviewed package here
//! offers; they are refused with their identifier, as is any other. Of section 7.1.5's four
//! key-wrap algorithms the three AES ones are run; `id-alg-CMS3DESwrap` (RFC 3217), a "MAY" in
//! section 8, is refused with its identifier (ADR 2821).

use sha1::Sha1;
use sha2::{Digest, Sha224, Sha256, Sha384, Sha512};

use super::{EnvelopeError, RecipientIdentifier, dotted, expect, octets, recipient_identifier};
use crate::der::{OCTET_STRING, SEQUENCE, Value};
use crate::ec;
use crate::x509::{self, EcCurve, PublicKey};

/// RFC 5753 section 7.1.4's `dhSinglePass-stdDH-sha1kdf-scheme`, `{ x9-63-scheme 2 }`.
const STD_SHA1: &[u8] = const_oid::db::rfc5753::DH_SINGLE_PASS_STD_DH_SHA_1_KDF_SCHEME.as_bytes();
/// `dhSinglePass-cofactorDH-sha1kdf-scheme`, `{ x9-63-scheme 3 }`.
const COFACTOR_SHA1: &[u8] =
    const_oid::db::rfc5753::DH_SINGLE_PASS_COFACTOR_DH_SHA_1_KDF_SCHEME.as_bytes();
/// `dhSinglePass-stdDH-sha224kdf-scheme`, `{ secg-scheme 11 0 }`.
const STD_SHA224: &[u8] =
    const_oid::db::rfc5753::DH_SINGLE_PASS_STD_DH_SHA_224_KDF_SCHEME.as_bytes();
/// `dhSinglePass-stdDH-sha256kdf-scheme`, `{ secg-scheme 11 1 }`.
const STD_SHA256: &[u8] =
    const_oid::db::rfc5753::DH_SINGLE_PASS_STD_DH_SHA_256_KDF_SCHEME.as_bytes();
/// `dhSinglePass-stdDH-sha384kdf-scheme`, `{ secg-scheme 11 2 }`.
const STD_SHA384: &[u8] =
    const_oid::db::rfc5753::DH_SINGLE_PASS_STD_DH_SHA_384_KDF_SCHEME.as_bytes();
/// `dhSinglePass-stdDH-sha512kdf-scheme`, `{ secg-scheme 11 3 }`.
const STD_SHA512: &[u8] =
    const_oid::db::rfc5753::DH_SINGLE_PASS_STD_DH_SHA_512_KDF_SCHEME.as_bytes();
/// `dhSinglePass-cofactorDH-sha224kdf-scheme`, `{ secg-scheme 14 0 }`.
const COFACTOR_SHA224: &[u8] =
    const_oid::db::rfc5753::DH_SINGLE_PASS_COFACTOR_DH_SHA_224_KDF_SCHEME.as_bytes();
/// `dhSinglePass-cofactorDH-sha256kdf-scheme`, `{ secg-scheme 14 1 }`.
const COFACTOR_SHA256: &[u8] =
    const_oid::db::rfc5753::DH_SINGLE_PASS_COFACTOR_DH_SHA_256_KDF_SCHEME.as_bytes();
/// `dhSinglePass-cofactorDH-sha384kdf-scheme`, `{ secg-scheme 14 2 }`.
const COFACTOR_SHA384: &[u8] =
    const_oid::db::rfc5753::DH_SINGLE_PASS_COFACTOR_DH_SHA_384_KDF_SCHEME.as_bytes();
/// `dhSinglePass-cofactorDH-sha512kdf-scheme`, `{ secg-scheme 14 3 }`.
const COFACTOR_SHA512: &[u8] =
    const_oid::db::rfc5753::DH_SINGLE_PASS_COFACTOR_DH_SHA_512_KDF_SCHEME.as_bytes();

/// RFC 3565 section 2.3.2's `id-aes128-wrap`, `{ aes 5 }`.
pub const ID_AES128_WRAP: &[u8] = const_oid::db::rfc5911::ID_AES_128_WRAP.as_bytes();
/// `id-aes192-wrap`, `{ aes 25 }`.
pub const ID_AES192_WRAP: &[u8] = const_oid::db::rfc5911::ID_AES_192_WRAP.as_bytes();
/// `id-aes256-wrap`, `{ aes 45 }`.
pub const ID_AES256_WRAP: &[u8] = const_oid::db::rfc5911::ID_AES_256_WRAP.as_bytes();

/// RFC 5480 section 2.1.1's `id-ecPublicKey`, which RFC 5753 section 3.1.1 requires of the
/// `originatorKey`'s algorithm.
const ID_EC_PUBLIC_KEY: &[u8] = const_oid::db::rfc5912::ID_EC_PUBLIC_KEY.as_bytes();

/// The sender's public key, by section 6.2.2's three alternatives: "Implementations MUST support
/// all three alternatives for specifying the sender's public key."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Originator<'a> {
    /// `issuerAndSerialNumber` or `subjectKeyIdentifier [0]`: a certificate, which this reader
    /// looks for in the envelope's own `originatorInfo`.
    Certificate(RecipientIdentifier<'a>),
    /// `originatorKey [1]`, "the algorithm identifier and sender's key agreement public key".
    Key {
        /// `algorithm`'s object identifier.
        algorithm: &'a [u8],
        /// `algorithm`'s parameters, where it states any.
        parameters: Option<Value<'a>>,
        /// `publicKey`'s bits: for `id-ecPublicKey`, RFC 5753 section 7.2's `ECPoint`.
        point: &'a [u8],
    },
}

/// One `RecipientEncryptedKey`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecipientEncryptedKey<'a> {
    /// `rid`: `issuerAndSerialNumber`, or `rKeyId`'s `subjectKeyIdentifier`.
    pub recipient: RecipientIdentifier<'a>,
    /// `encryptedKey`: the content-encryption key wrapped in the pairwise key.
    pub encrypted_key: &'a [u8],
}

/// RFC 5652 section 6.2.2's `KeyAgreeRecipientInfo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyAgreement<'a> {
    /// `originator`.
    pub originator: Originator<'a>,
    /// `ukm`'s octets, where present.
    pub ukm: Option<Vec<u8>>,
    /// `keyEncryptionAlgorithm`'s object identifier: the key-agreement scheme.
    pub algorithm: &'a [u8],
    /// The `KeyWrapAlgorithm` its parameters carry (RFC 5753 section 7.1.4): the identifier.
    pub wrap_algorithm: &'a [u8],
    /// The same `KeyWrapAlgorithm`'s parameters, where it states any.
    pub wrap_parameters: Option<Value<'a>>,
    /// `recipientEncryptedKeys`, in the encoding's order.
    pub recipients: Vec<RecipientEncryptedKey<'a>>,
}

/// Reads a `KeyAgreeRecipientInfo` from the members of `kari [1]`'s implicit `SEQUENCE`.
///
/// `version` "MUST always be 3"; it is read past rather than checked, as the key-transport
/// reader reads past its own, because nothing in the structure's meaning hangs on it.
pub(super) fn read(info: Value<'_>) -> Result<KeyAgreement<'_>, EnvelopeError> {
    let mut members = info.children()?;
    let _version = members
        .next_value()?
        .ok_or(EnvelopeError::Malformed("KeyAgreeRecipientInfo.version"))?;
    // "originator [0] EXPLICIT OriginatorIdentifierOrKey".
    let explicit = members
        .next_value()?
        .filter(|value| value.is_context(0) && value.is_constructed())
        .ok_or(EnvelopeError::Malformed("KeyAgreeRecipientInfo.originator"))?;
    let choice = explicit
        .children()?
        .next_value()?
        .ok_or(EnvelopeError::Malformed("KeyAgreeRecipientInfo.originator"))?;
    let originator = originator(choice)?;
    let mut next = members
        .next_value()?
        .ok_or(EnvelopeError::Malformed("keyEncryptionAlgorithm"))?;
    // "ukm [1] EXPLICIT UserKeyingMaterial OPTIONAL", "UserKeyingMaterial ::= OCTET STRING".
    let mut ukm = None;
    if next.is_context(1) {
        let inner = next
            .children()?
            .next_value()?
            .ok_or(EnvelopeError::Malformed("ukm"))?;
        // Primitive or, under BER, constructed (X.690 clause 8.7).
        if inner.identifier & !CONSTRUCTED != OCTET_STRING {
            return Err(EnvelopeError::Malformed("ukm"));
        }
        ukm = Some(octets(inner)?);
        next = members
            .next_value()?
            .ok_or(EnvelopeError::Malformed("keyEncryptionAlgorithm"))?;
    }
    if next.identifier != SEQUENCE {
        return Err(EnvelopeError::Malformed("keyEncryptionAlgorithm"));
    }
    let mut algorithm = next.children()?;
    let identifier = algorithm
        .next_value()?
        .and_then(|value| value.object_identifier())
        .ok_or(EnvelopeError::Malformed("keyEncryptionAlgorithm"))?;
    // RFC 5753 section 7.1.4: "the associated parameters field contains KeyWrapAlgorithm".
    let wrap = algorithm
        .next_value()?
        .filter(|value| value.identifier == SEQUENCE)
        .ok_or(EnvelopeError::Malformed("KeyWrapAlgorithm"))?;
    let mut wrap_parts = wrap.children()?;
    let wrap_algorithm = wrap_parts
        .next_value()?
        .and_then(|value| value.object_identifier())
        .ok_or(EnvelopeError::Malformed("KeyWrapAlgorithm"))?;
    let wrap_parameters = wrap_parts.next_value()?;
    // "RecipientEncryptedKeys ::= SEQUENCE OF RecipientEncryptedKey".
    let recipients = encrypted_keys(expect(&mut members, SEQUENCE, "recipientEncryptedKeys")?)?;
    Ok(KeyAgreement {
        originator,
        ukm,
        algorithm: identifier,
        wrap_algorithm,
        wrap_parameters,
        recipients,
    })
}

/// `OriginatorIdentifierOrKey`, the `CHOICE` inside `originator [0] EXPLICIT`.
fn originator(choice: Value<'_>) -> Result<Originator<'_>, EnvelopeError> {
    if choice.is_context(1) && choice.is_constructed() {
        // "originatorKey [1] OriginatorPublicKey", implicitly tagged:
        // "OriginatorPublicKey ::= SEQUENCE { algorithm AlgorithmIdentifier, publicKey BIT STRING }".
        let mut parts = choice.children()?;
        let mut algorithm =
            expect(&mut parts, SEQUENCE, "OriginatorPublicKey.algorithm")?.children()?;
        let identifier = algorithm
            .next_value()?
            .and_then(|value| value.object_identifier())
            .ok_or(EnvelopeError::Malformed("OriginatorPublicKey.algorithm"))?;
        let parameters = algorithm.next_value()?;
        let bits = expect(&mut parts, BIT_STRING, "OriginatorPublicKey.publicKey")?;
        // X.690 clause 8.6.2.2: the initial octet counts the unused bits of the last one, and an
        // `ECPoint` is whole octets.
        let Some((0, point)) = bits.contents.split_first() else {
            return Err(EnvelopeError::Malformed("OriginatorPublicKey.publicKey"));
        };
        Ok(Originator::Key {
            algorithm: identifier,
            parameters,
            point,
        })
    } else {
        Ok(Originator::Certificate(recipient_identifier(
            choice,
            "KeyAgreeRecipientInfo.originator",
        )?))
    }
}

/// `RecipientEncryptedKeys ::= SEQUENCE OF RecipientEncryptedKey`.
fn encrypted_keys(keys: Value<'_>) -> Result<Vec<RecipientEncryptedKey<'_>>, EnvelopeError> {
    let mut recipients = Vec::new();
    let mut entries = keys.children()?;
    while let Some(entry) = entries.next_value()? {
        if entry.identifier != SEQUENCE {
            return Err(EnvelopeError::Malformed("RecipientEncryptedKey"));
        }
        let mut parts = entry.children()?;
        let rid = parts
            .next_value()?
            .ok_or(EnvelopeError::Malformed("RecipientEncryptedKey.rid"))?;
        let recipient = if rid.is_context(0) && rid.is_constructed() {
            // "rKeyId [0] IMPLICIT RecipientKeyIdentifier", "RecipientKeyIdentifier ::= SEQUENCE
            // { subjectKeyIdentifier SubjectKeyIdentifier, date GeneralizedTime OPTIONAL, other
            // OtherKeyAttribute OPTIONAL }". The key identifier names the certificate; `date`
            // "specifies which of the recipient's previously distributed UKMs was used" and
            // `other` helps "locate the public keying material used by the sender", and neither
            // changes which certificate is meant, so neither is part of the match.
            let identifier = expect(&mut rid.children()?, OCTET_STRING, "rKeyId")?;
            RecipientIdentifier::SubjectKeyIdentifier(identifier.contents)
        } else {
            recipient_identifier(rid, "RecipientEncryptedKey.rid")?
        };
        let encrypted_key = expect(
            &mut parts,
            OCTET_STRING,
            "RecipientEncryptedKey.encryptedKey",
        )?;
        recipients.push(RecipientEncryptedKey {
            recipient,
            encrypted_key: encrypted_key.contents,
        });
    }
    Ok(recipients)
}

/// X.690 clause 8.1.2.5's constructed bit.
const CONSTRUCTED: u8 = 0x20;

/// X.690's universal tag for `BIT STRING`.
const BIT_STRING: u8 = 0x03;

/// The hash SEC 1 section 3.6.1's KDF runs under, as the key-agreement identifier names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KdfHash {
    /// SHA-1.
    Sha1,
    /// SHA-224.
    Sha224,
    /// SHA-256.
    Sha256,
    /// SHA-384.
    Sha384,
    /// SHA-512.
    Sha512,
}

impl KdfHash {
    /// The hash a `dhSinglePass-*` identifier names, or `None` for any other identifier.
    fn of(algorithm: &[u8]) -> Option<Self> {
        Some(match algorithm {
            STD_SHA1 | COFACTOR_SHA1 => Self::Sha1,
            STD_SHA224 | COFACTOR_SHA224 => Self::Sha224,
            STD_SHA256 | COFACTOR_SHA256 => Self::Sha256,
            STD_SHA384 | COFACTOR_SHA384 => Self::Sha384,
            STD_SHA512 | COFACTOR_SHA512 => Self::Sha512,
            _ => return None,
        })
    }
}

impl KeyAgreement<'_> {
    /// RFC 5753 section 3.1.3's receiving agent: the content-encryption key `encrypted_key` holds,
    /// for the holder of `key`, the private key of the certificate `rid` named.
    ///
    /// `originator_certificates` are the envelope's `originatorInfo` certificates, where the
    /// sender's key is named by certificate rather than carried.
    pub(super) fn content_key(
        &self,
        encrypted_key: &[u8],
        key: &ec::PrivateKey,
        originator_certificates: &[Value<'_>],
    ) -> Result<Vec<u8>, EnvelopeError> {
        let hash = KdfHash::of(self.algorithm)
            .ok_or_else(|| EnvelopeError::KeyAgreement(dotted(self.algorithm)))?;
        let kek_octets = match self.wrap_algorithm {
            ID_AES128_WRAP => 16,
            ID_AES192_WRAP => 24,
            ID_AES256_WRAP => 32,
            other => return Err(EnvelopeError::KeyWrap(dotted(other))),
        };
        // RFC 3565 section 2.3.2: "In all cases the parameters field MUST be absent." The
        // sender's `ECC-CMS-SharedInfo` carries the same identifier, so a wrap stated with
        // parameters leaves what the sender derived its key from unknown.
        if self.wrap_parameters.is_some() {
            return Err(EnvelopeError::Malformed(
                "an AES key wrap with parameters, which RFC 3565 section 2.3.2 says MUST be absent",
            ));
        }
        let point = self.originator_point(key.curve(), originator_certificates)?;
        // SEC 1 section 3.3.1; a point that is not on the key's curve is a key that does not
        // open this recipient, the one error every such failure is.
        let z = key.agree(point).map_err(|_| EnvelopeError::Decryption)?;
        let info = shared_info(self.wrap_algorithm, self.ukm.as_deref(), kek_octets);
        let kek = match hash {
            KdfHash::Sha1 => x963_kdf::<Sha1>(&z, &info, kek_octets),
            KdfHash::Sha224 => x963_kdf::<Sha224>(&z, &info, kek_octets),
            KdfHash::Sha256 => x963_kdf::<Sha256>(&z, &info, kek_octets),
            KdfHash::Sha384 => x963_kdf::<Sha384>(&z, &info, kek_octets),
            KdfHash::Sha512 => x963_kdf::<Sha512>(&z, &info, kek_octets),
        };
        crate::key_wrap::unwrap(&kek, encrypted_key).map_err(|_| EnvelopeError::Decryption)
    }

    /// The sender's public point: carried in `originatorKey`, or read from the certificate the
    /// originator names among `originatorInfo`'s.
    fn originator_point<'p>(
        &'p self,
        curve: EcCurve,
        originator_certificates: &'p [Value<'p>],
    ) -> Result<&'p [u8], EnvelopeError> {
        match self.originator {
            Originator::Key {
                algorithm,
                parameters,
                point,
            } => {
                // RFC 5753 section 3.1.1: "The originatorKey algorithm field MUST contain the
                // id-ecPublicKey object identifier".
                if algorithm != ID_EC_PUBLIC_KEY {
                    return Err(EnvelopeError::KeyAgreement(dotted(algorithm)));
                }
                // "The parameters associated with id-ecPublicKey MUST be absent, ECParameters,
                // or NULL ... If the parameters are ECParameters, then they MUST be namedCurve."
                // Absent or NULL, the curve is the recipient's — section 6.2.2's recipients "use
                // the same key agreement algorithm and domain parameters" — and named, it must be.
                if let Some(parameters) = parameters
                    && parameters.identifier != crate::der::NULL
                {
                    let named = parameters
                        .object_identifier()
                        .ok_or(EnvelopeError::Malformed(
                            "originatorKey's parameters are neither NULL nor a namedCurve",
                        ))?;
                    if named != curve.oid().as_bytes() {
                        return Err(EnvelopeError::Malformed(
                            "originatorKey names another curve than the recipient's key",
                        ));
                    }
                }
                Ok(point)
            }
            Originator::Certificate(identifier) => {
                for certificate in originator_certificates {
                    let Ok(parsed) = x509::read(*certificate) else {
                        continue;
                    };
                    if !identifier.names(&parsed) {
                        continue;
                    }
                    return match parsed.public_key {
                        PublicKey::Ec(key) if key.curve == curve => Ok(key.point),
                        _ => Err(EnvelopeError::Malformed(
                            "the originator's certificate holds no key on the recipient's curve",
                        )),
                    };
                }
                Err(EnvelopeError::Malformed(
                    "the originator names a certificate originatorInfo does not carry",
                ))
            }
        }
    }
}

/// The DER of RFC 5753 section 7.2's `ECC-CMS-SharedInfo` for a key wrapped under
/// `wrap_algorithm` with a key of `kek_octets`:
///
/// ```text
/// ECC-CMS-SharedInfo ::= SEQUENCE {
///   keyInfo         AlgorithmIdentifier,
///   entityUInfo [0] EXPLICIT OCTET STRING OPTIONAL,
///   suppPubInfo [2] EXPLICIT OCTET STRING  }
/// ```
///
/// `keyInfo` is the wrap's identifier, and "the AES wraps have absent parameters"; `entityUInfo`
/// "contains the octet string ukm"; `suppPubInfo` "contains the length of the generated KEK, in
/// bits, represented as a 32-bit number".
pub(crate) fn shared_info(wrap_algorithm: &[u8], ukm: Option<&[u8]>, kek_octets: usize) -> Vec<u8> {
    let key_info = tlv(SEQUENCE, &tlv(OBJECT_IDENTIFIER, wrap_algorithm));
    let bits = u32::try_from(kek_octets.saturating_mul(8)).unwrap_or(u32::MAX);
    let mut contents = key_info;
    if let Some(ukm) = ukm {
        contents.extend(tlv(CONTEXT_0, &tlv(OCTET_STRING, ukm)));
    }
    contents.extend(tlv(CONTEXT_2, &tlv(OCTET_STRING, &bits.to_be_bytes())));
    tlv(SEQUENCE, &contents)
}

/// X.690's universal tag for `OBJECT IDENTIFIER`.
const OBJECT_IDENTIFIER: u8 = crate::der::OBJECT_IDENTIFIER;
/// A constructed context-specific `[0]`.
const CONTEXT_0: u8 = 0xA0;
/// A constructed context-specific `[2]`.
const CONTEXT_2: u8 = 0xA2;

/// One DER value: `identifier`, X.690 clause 10.1's definite length in the fewest octets, and
/// `contents`.
fn tlv(identifier: u8, contents: &[u8]) -> Vec<u8> {
    let mut out = vec![identifier];
    let length = contents.len();
    if length < 0x80 {
        // Below 0x80, so it fits in the one octet of the short form.
        out.push(u8::try_from(length).unwrap_or(0));
    } else {
        let octets = length.to_be_bytes();
        let skip = octets.iter().take_while(|&&octet| octet == 0).count();
        let significant = &octets[skip..];
        // At most eight octets follow, so the count fits in the initial octet's seven bits.
        out.push(0x80 | u8::try_from(significant.len()).unwrap_or(0));
        out.extend_from_slice(significant);
    }
    out.extend_from_slice(contents);
    out
}

/// SEC 1 section 3.6.1's ANSI X9.63 key derivation: `keydatalen` octets of
/// `K1 || K2 || ...`, where `Ki = Hash(Z || Counter || SharedInfo)` and `Counter` is "a 4 octet,
/// big-endian octet string" starting at `00000001`.
///
/// Step 1's bound — `|Z| + |SharedInfo| + 4 < hashmaxlen`, at least 2^61 − 1 octets for these
/// hashes — and step 2's — `keydatalen < hashlen × (2^32 − 1)` — cannot be reached by a
/// key-encryption key of at most 32 octets over inputs a DER value bounds.
pub(crate) fn x963_kdf<D: Digest>(z: &[u8], shared_info: &[u8], keydatalen: usize) -> Vec<u8> {
    let mut key = Vec::with_capacity(keydatalen);
    let mut counter: u32 = 1;
    while key.len() < keydatalen {
        let mut hash = D::new();
        hash.update(z);
        hash.update(counter.to_be_bytes());
        hash.update(shared_info);
        key.extend_from_slice(&hash.finalize());
        counter = counter.wrapping_add(1);
    }
    key.truncate(keydatalen);
    key
}

#[cfg(test)]
mod tests;
