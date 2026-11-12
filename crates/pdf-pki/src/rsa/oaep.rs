//! RFC 8017 section 7.1's `RSAES-OAEP` decryption, and the parameters RFC 3560 section 3 carries
//! for it in a CMS key-transport recipient.
//!
//! ISO 32000-2 §7.6.5.3 says the key that encrypts the enveloped data "shall be encrypted for each
//! recipient, using that recipient's public key", and RFC 5652 section 6.2.1's
//! `KeyTransRecipientInfo` says how: under the `keyEncryptionAlgorithm` it names. RFC 3560 is that
//! algorithm when it is `id-RSAES-OAEP` — "the RSA encryption scheme defined in RFC 3447", whose
//! current text is RFC 8017 section 7.1 — and its parameters choose the hash, the mask generation
//! function and the label the scheme runs with.
//!
//! # The parameters, and their defaults
//!
//! RFC 3560 section 3 requires the `AlgorithmIdentifier`'s parameters to be present and to be
//! `RSAES-OAEP-params`, a `SEQUENCE` of three members, each with a
//! default a writer "MUST omit" when it is the one used. A reader "MUST recognize both" the stated
//! default and its absence, so [`OaepParameters::read`] fills an absent member with SHA-1, MGF1 over
//! SHA-1, and the empty label. The hash is any of RFC 8017 appendix A.2.1's
//! `OAEP-PSSDigestAlgorithms` — RFC 4055 section 2.1's five and SHA-512/224 and SHA-512/256 — with
//! RFC 4055's "NULL and absent parameters as legal and equivalent encodings"; the mask generation
//! function is MGF1, the only one RFC 8017's `PKCS1MGFAlgorithms` and RFC 4055 section 2.2 define,
//! over any of the same hashes; and the label is `id-pSpecified`'s octet string. Anything else is
//! refused with its object identifier (ADR 2826).
//!
//! # Why the decoding reads every octet
//!
//! Section 7.1.2's note: "an opponent cannot distinguish the different error conditions in Step
//! 3.g, whether by error message or timing". `decode_eme_oaep` folds `Y`, the label hash and the
//! separator search into one flag over the whole encoded message before it answers, and every
//! failure after the length checks is [`RsaError::Decryption`], as in section 7.2.2's decoding
//! beside it.

use sha1::Sha1;
use sha2::{Digest, Sha224, Sha256, Sha384, Sha512, Sha512_224, Sha512_256};

use super::{PrivateKey, RsaError};
use crate::bigint::Integer;
use crate::der::{NULL, OCTET_STRING, SEQUENCE, Value};

/// RFC 3560 section 3's `id-RSAES-OAEP`, `{ pkcs-1 7 }`.
pub const ID_RSAES_OAEP: &[u8] = const_oid::db::rfc5912::ID_RSAES_OAEP.as_bytes();
/// `id-mgf1`, `{ pkcs-1 8 }`.
const ID_MGF1: &[u8] = const_oid::db::rfc5912::ID_MGF_1.as_bytes();
/// `id-pSpecified`, `{ pkcs-1 9 }`.
const ID_P_SPECIFIED: &[u8] = const_oid::db::rfc5912::ID_P_SPECIFIED.as_bytes();

/// `id-sha1`, 1.3.14.3.2.26.
const ID_SHA1: &[u8] = const_oid::db::rfc5912::ID_SHA_1.as_bytes();
/// `id-sha224`, 2.16.840.1.101.3.4.2.4.
const ID_SHA224: &[u8] = const_oid::db::rfc5912::ID_SHA_224.as_bytes();
/// `id-sha256`, 2.16.840.1.101.3.4.2.1.
const ID_SHA256: &[u8] = const_oid::db::rfc5912::ID_SHA_256.as_bytes();
/// `id-sha384`, 2.16.840.1.101.3.4.2.2.
const ID_SHA384: &[u8] = const_oid::db::rfc5912::ID_SHA_384.as_bytes();
/// `id-sha512`, 2.16.840.1.101.3.4.2.3.
const ID_SHA512: &[u8] = const_oid::db::rfc5912::ID_SHA_512.as_bytes();
/// RFC 8017 appendix A.2.1's `id-sha512-224`, `{ hashAlgs 5 }`, 2.16.840.1.101.3.4.2.5, which the
/// object-identifier database the workspace pins does not list.
const ID_SHA512_224: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x05];
/// `id-sha512-256`, `{ hashAlgs 6 }`, 2.16.840.1.101.3.4.2.6.
const ID_SHA512_256: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x06];

/// One of RFC 8017 appendix A.2.1's `OAEP-PSSDigestAlgorithms`: the hash `RSAES-OAEP` runs with,
/// or the one its MGF1 does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OaepHash {
    /// SHA-1, the default of both `hashFunc` and MGF1.
    Sha1,
    /// SHA-224.
    Sha224,
    /// SHA-256.
    Sha256,
    /// SHA-384.
    Sha384,
    /// SHA-512.
    Sha512,
    /// SHA-512/224.
    Sha512_224,
    /// SHA-512/256.
    Sha512_256,
}

impl OaepHash {
    /// The hash an object identifier names, or `None` for one outside the set.
    fn of(oid: &[u8]) -> Option<Self> {
        Some(match oid {
            ID_SHA1 => Self::Sha1,
            ID_SHA224 => Self::Sha224,
            ID_SHA256 => Self::Sha256,
            ID_SHA384 => Self::Sha384,
            ID_SHA512 => Self::Sha512,
            ID_SHA512_224 => Self::Sha512_224,
            ID_SHA512_256 => Self::Sha512_256,
            _ => return None,
        })
    }

    /// `hLen`, the length in octets of the hash's output.
    #[must_use]
    pub fn length(self) -> usize {
        match self {
            Self::Sha1 => 20,
            Self::Sha224 | Self::Sha512_224 => 28,
            Self::Sha256 | Self::Sha512_256 => 32,
            Self::Sha384 => 48,
            Self::Sha512 => 64,
        }
    }

    /// The hash of `parts` joined.
    fn digest(self, parts: &[&[u8]]) -> Vec<u8> {
        fn run<D: Digest>(parts: &[&[u8]]) -> Vec<u8> {
            let mut hasher = D::new();
            for part in parts {
                hasher.update(part);
            }
            hasher.finalize().to_vec()
        }
        match self {
            Self::Sha1 => run::<Sha1>(parts),
            Self::Sha224 => run::<Sha224>(parts),
            Self::Sha256 => run::<Sha256>(parts),
            Self::Sha384 => run::<Sha384>(parts),
            Self::Sha512 => run::<Sha512>(parts),
            Self::Sha512_224 => run::<Sha512_224>(parts),
            Self::Sha512_256 => run::<Sha512_256>(parts),
        }
    }
}

/// RFC 3560 section 3's `RSAES-OAEP-params`, with each absent member its default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OaepParameters<'a> {
    /// `hashFunc`: section 7.1.2's `Hash`, whose output length is `hLen`.
    pub hash: OaepHash,
    /// `maskGenFunc`'s hash: the `Hash` option of the MGF1 that is section 7.1.2's `MGF`.
    pub mask_hash: OaepHash,
    /// `pSourceFunc`'s octets: section 7.1.2's label `L`.
    pub label: &'a [u8],
}

/// Why a key-transport recipient's `RSAES-OAEP-params` were not run.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum OaepParameterError {
    /// The parameters are not RFC 3560 section 3's syntax; the string names what is wrong.
    #[error("the RSAES-OAEP parameters are malformed: {0}")]
    Malformed(&'static str),
    /// A member names an algorithm this reader does not run: the member, and the object
    /// identifier in dotted form.
    #[error("the RSAES-OAEP {member} is {algorithm}, which this reader does not run")]
    Unsupported {
        /// `hashFunc`, `maskGenFunc`, MGF1's hash or `pSourceFunc`.
        member: &'static str,
        /// The identifier, dotted.
        algorithm: String,
    },
}

impl<'a> OaepParameters<'a> {
    /// The three defaults: "sha1Identifier", "mgf1SHA1Identifier" and
    /// "pSpecifiedEmptyIdentifier".
    pub const DEFAULT: OaepParameters<'static> = OaepParameters {
        hash: OaepHash::Sha1,
        mask_hash: OaepHash::Sha1,
        label: &[],
    };

    /// Reads `id-RSAES-OAEP`'s parameters as RFC 3560 section 3 states them:
    ///
    /// ```text
    /// RSAES-OAEP-params  ::=  SEQUENCE  {
    ///   hashFunc    [0] AlgorithmIdentifier DEFAULT sha1Identifier,
    ///   maskGenFunc [1] AlgorithmIdentifier DEFAULT mgf1SHA1Identifier,
    ///   pSourceFunc [2] AlgorithmIdentifier DEFAULT
    ///                       pSpecifiedEmptyIdentifier  }
    /// ```
    ///
    /// # Errors
    ///
    /// [`OaepParameterError::Malformed`] where the parameters are absent — "the parameters field
    /// MUST contain RSAES-OAEP-params" — or are not that syntax, members out of order included,
    /// and [`OaepParameterError::Unsupported`] naming a hash outside RFC 8017 appendix A.2.1's
    /// set, a mask generation function other than MGF1, or a label source other than
    /// `id-pSpecified`.
    pub fn read(parameters: Option<Value<'a>>) -> Result<Self, OaepParameterError> {
        let parameters = parameters
            .filter(|value| value.identifier == SEQUENCE)
            .ok_or(OaepParameterError::Malformed(
                "RFC 3560 section 3 says the parameters field MUST contain RSAES-OAEP-params",
            ))?;
        let mut read = Self::DEFAULT;
        // The tag number of the member read last: DER writes a SEQUENCE's members in the order
        // its definition gives them, so a member at or below the last is out of place.
        let mut last: Option<u8> = None;
        let mut members = parameters.children().map_err(|_| malformed())?;
        while let Some(member) = members.next_value().map_err(|_| malformed())? {
            let number = member.tag_number();
            if !member.is_constructed()
                || !(member.is_context(0) || member.is_context(1) || member.is_context(2))
                || last.is_some_and(|last| number <= last)
            {
                return Err(malformed());
            }
            last = Some(number);
            // Each member is `[n] AlgorithmIdentifier`, explicitly tagged: one `SEQUENCE` inside.
            let mut inside = member.children().map_err(|_| malformed())?;
            let identifier = inside
                .next_value()
                .map_err(|_| malformed())?
                .filter(|value| value.identifier == SEQUENCE)
                .ok_or_else(malformed)?;
            if inside.next_value().map_err(|_| malformed())?.is_some() {
                return Err(malformed());
            }
            let (oid, algorithm_parameters) = algorithm_identifier(identifier)?;
            match number {
                0 => read.hash = hash(oid, algorithm_parameters, "hashFunc")?,
                1 => {
                    if oid != ID_MGF1 {
                        return Err(unsupported("maskGenFunc", oid));
                    }
                    // RFC 4055 section 2.2: "The parameters field associated with id-mgf1 MUST
                    // have a hashAlgorithm value".
                    let inner = algorithm_parameters
                        .filter(|value| value.identifier == SEQUENCE)
                        .ok_or(OaepParameterError::Malformed(
                            "MGF1 states no hash, which RFC 4055 section 2.2 says it MUST",
                        ))?;
                    let (oid, inner_parameters) = algorithm_identifier(inner)?;
                    read.mask_hash = hash(oid, inner_parameters, "MGF1 hash")?;
                }
                _ => {
                    if oid != ID_P_SPECIFIED {
                        return Err(unsupported("pSourceFunc", oid));
                    }
                    // "id-pSpecified object identifier, indicating that P is explicitly provided
                    // as an OCTET STRING in the parameters".
                    read.label = algorithm_parameters
                        .filter(|value| value.identifier == OCTET_STRING)
                        .ok_or(OaepParameterError::Malformed(
                            "id-pSpecified carries no OCTET STRING",
                        ))?
                        .contents;
                }
            }
        }
        Ok(read)
    }
}

/// The one refusal for a structure that is not RFC 3560 section 3's.
fn malformed() -> OaepParameterError {
    OaepParameterError::Malformed("not RFC 3560 section 3's RSAES-OAEP-params")
}

/// An `AlgorithmIdentifier`'s object identifier and its parameters, with nothing after them.
fn algorithm_identifier(
    value: Value<'_>,
) -> Result<(&[u8], Option<Value<'_>>), OaepParameterError> {
    let mut parts = value.children().map_err(|_| malformed())?;
    let oid = parts
        .next_value()
        .map_err(|_| malformed())?
        .and_then(|value| value.object_identifier())
        .ok_or_else(malformed)?;
    let parameters = parts.next_value().map_err(|_| malformed())?;
    if parts.next_value().map_err(|_| malformed())?.is_some() {
        return Err(malformed());
    }
    Ok((oid, parameters))
}

/// A hash `AlgorithmIdentifier`: one of the seven, its parameters `NULL` or absent — RFC 4055
/// section 2.1, "All implementations MUST accept both NULL and absent parameters as legal and
/// equivalent encodings".
fn hash(
    oid: &[u8],
    parameters: Option<Value<'_>>,
    member: &'static str,
) -> Result<OaepHash, OaepParameterError> {
    let hash = OaepHash::of(oid).ok_or_else(|| unsupported(member, oid))?;
    if parameters.is_some_and(|value| value.identifier != NULL || !value.contents.is_empty()) {
        return Err(OaepParameterError::Malformed(
            "a hash identifier's parameters are neither NULL nor absent",
        ));
    }
    Ok(hash)
}

/// [`OaepParameterError::Unsupported`] for `member` naming `oid`.
fn unsupported(member: &'static str, oid: &[u8]) -> OaepParameterError {
    OaepParameterError::Unsupported {
        member,
        algorithm: crate::enveloped::dotted(oid),
    }
}

impl PrivateKey {
    /// RFC 8017 section 7.1.2, `RSAES-OAEP-DECRYPT (K, C, L)`, with the hash, MGF1's hash and the
    /// label `parameters` state.
    ///
    /// # Errors
    ///
    /// [`RsaError::Decryption`] for every failure the section names — a ciphertext whose length
    /// is not `k`, a `k` below `2hLen + 2`, a representative not below `n`, and an encoded
    /// message that is not `0x00 || maskedSeed || maskedDB` over `lHash || PS || 0x01 || M` —
    /// and for nothing else.
    pub fn decrypt_oaep(
        &self,
        ciphertext: &[u8],
        parameters: &OaepParameters<'_>,
    ) -> Result<Vec<u8>, RsaError> {
        let h_len = parameters.hash.length();
        // Step 1.a, the label's length against the hash's input limit, cannot fail: the smallest
        // limit of the seven, SHA-1's and SHA-224's 2^61 - 1 octets, is past any slice in memory.
        // Step 1.b: "If the length of the ciphertext C is not k octets". Step 1.c: "If k < 2hLen +
        // 2".
        if ciphertext.len() != self.length
            || self.length < h_len.saturating_mul(2).saturating_add(2)
        {
            return Err(RsaError::Decryption);
        }
        // Step 2: OS2IP, RSADP, I2OSP.
        let representative = Integer::from_be_bytes(ciphertext).ok_or(RsaError::Decryption)?;
        let message = self.rsadp(&representative)?;
        let encoded = message.be_bytes(self.length);
        // Step 3.a: "let lHash = Hash(L)".
        let label_hash = parameters.hash.digest(&[parameters.label]);
        decode_eme_oaep(&encoded, &label_hash, parameters.mask_hash).ok_or(RsaError::Decryption)
    }
}

/// RFC 8017 appendix B.2.1, `MGF1 (mgfSeed, maskLen)` under `hash`.
///
/// Step 1's "mask too long" is past reach: the longest mask asked for is a modulus's length, far
/// below `2^32 hLen`.
fn mgf1(hash: OaepHash, seed: &[u8], length: usize) -> Vec<u8> {
    let mut mask = Vec::with_capacity(length.saturating_add(hash.length()));
    let mut counter: u32 = 0;
    while mask.len() < length {
        // Step 3: "T = T || Hash(mgfSeed || C)", C the counter in four octets.
        mask.extend(hash.digest(&[seed, &counter.to_be_bytes()]));
        counter = counter.wrapping_add(1);
    }
    // Step 4: "Output the leading maskLen octets of T".
    mask.truncate(length);
    mask
}

/// RFC 8017 section 7.1.2 step 3.b to 3.g over `encoded`, `k` octets with `k >= 2hLen + 2`, where
/// `label_hash` is `lHash` and so `hLen` octets long; `None` is "decryption error".
fn decode_eme_oaep(encoded: &[u8], label_hash: &[u8], mask_hash: OaepHash) -> Option<Vec<u8>> {
    let h_len = label_hash.len();
    // Step 3.b: "EM = Y || maskedSeed || maskedDB".
    let (&y, rest) = encoded.split_first()?;
    let masked_seed = rest.get(..h_len)?;
    let masked_db = rest.get(h_len..)?;
    // Steps 3.c to 3.f.
    let seed_mask = mgf1(mask_hash, masked_db, h_len);
    let seed: Vec<u8> = masked_seed
        .iter()
        .zip(&seed_mask)
        .map(|(a, b)| a ^ b)
        .collect();
    let db_mask = mgf1(mask_hash, &seed, masked_db.len());
    let db: Vec<u8> = masked_db.iter().zip(&db_mask).map(|(a, b)| a ^ b).collect();
    // Step 3.g: "DB = lHash' || PS || 0x01 || M".
    let (stated_hash, after_hash) = db.split_at_checked(h_len)?;
    let mut good = u8::from(y == 0);
    let difference = stated_hash
        .iter()
        .zip(label_hash)
        .fold(0u8, |difference, (a, b)| difference | (a ^ b));
    good &= u8::from(difference == 0);
    // The index in `after_hash` of the 0x01 that ends PS; every octet before it must be zero.
    let mut separator = 0usize;
    let mut looking = 1u8;
    let mut stray = 0u8;
    for (index, &octet) in after_hash.iter().enumerate() {
        let is_one = u8::from(octet == 0x01);
        let is_zero = u8::from(octet == 0x00);
        let take = is_one & looking;
        // A branch-free select, as in section 7.2.2's decoding: `index` where this is the 0x01.
        let mask = usize::from(take).wrapping_neg();
        separator = (index & mask) | (separator & !mask);
        // An octet of PS that is neither 0x00 nor the 0x01 that ends it.
        stray |= looking & !is_zero & !is_one & 1;
        looking &= !take & 1;
    }
    // "If there is no octet with hexadecimal value 0x01 to separate PS from M, if lHash does not
    // equal lHash', or if Y is nonzero".
    good &= !looking & 1;
    good &= !stray & 1;
    (good == 1).then(|| {
        after_hash
            .get(separator.saturating_add(1)..)
            .unwrap_or_default()
            .to_vec()
    })
}

#[cfg(test)]
mod tests;
