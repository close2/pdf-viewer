//! RSA decryption: RFC 8017's private key, its RSADP primitive, and the `RSAES-PKCS1-v1_5`
//! decoding a CMS key-transport recipient is written in.
//!
//! ISO 32000-2 §7.6.5.3 puts the step in one sentence — the key that encrypts the enveloped data
//! "shall be encrypted for each recipient, using that recipient's public key", and "\[t]o decrypt
//! the document, that key shall be decrypted using the recipient's private key" — and RFC 5652
//! section 6.2.1 names the recipient structure that carries it, `KeyTransRecipientInfo`, whose
//! algorithm the documents this tree holds all state as RFC 8017's `rsaEncryption`. That
//! identifier is RFC 8017 section 7.2's scheme, `RSAES-PKCS1-v1_5`, and this module is its
//! decryption operation (section 7.2.2) over the primitive RSADP (section 5.1.2).
//!
//! # The private key is an input, and it is the one secret in this crate
//!
//! The key arrives as bytes a host supplied — never from the document, never from a path this
//! crate opens (ADR 1134): RFC 5958's `OneAsymmetricKey` (RFC 5208's `PrivateKeyInfo`, which is
//! what `openssl pkcs8` and a PKCS #12 key bag hold) or RFC 8017 appendix A.1.2's bare
//! `RSAPrivateKey`. Only the pair `(n, d)` is kept, which is the first of section 5.1.2's two key
//! forms; the Chinese-remainder components are read past, because the first form is complete and
//! one exponentiation is a small cost on a document opened once.
//!
//! Unlike everything else in [`crate::bigint`]'s callers, `d` is secret, so this module keeps it
//! out of every `Debug` output, and the exponentiation is `crypto-bigint`'s constant-time `pow`
//! rather than its `_vartime` form. Section 7.2.2's note asks for the same of the decoding — an
//! opponent must not "distinguish the different error conditions in Step 3, whether by error
//! message or timing" — so [`PrivateKey::decrypt`] reports every failure after the length check
//! as the one error RFC 8017 itself names, "decryption error", and reads every octet of the
//! encoded message whichever check fails first.

use crate::bigint::{Integer, MAX_BITS, Modulus, modpow, significant_bits};
use crate::der::{DerError, INTEGER, OCTET_STRING, Reader, SEQUENCE, Value};
use crate::x509::RSA_ENCRYPTION;

/// The widest modulus this module decrypts under, in bits: [`crate::bigint::MAX_BITS`].
///
/// A host's key is not a stranger's number, but the ciphertext it is applied to is, and the width
/// of the work is the key's; the bound is the arithmetic seam's own so that one figure governs
/// every exponentiation in the tree.
pub const MAX_MODULUS_BITS: usize = MAX_BITS;

/// Why a private key could not be read or a ciphertext could not be decrypted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RsaError {
    /// The key's bytes are not the DER encoding of either structure this module reads.
    #[error("the private key is not RFC 5958's OneAsymmetricKey or RFC 8017's RSAPrivateKey")]
    Malformed,
    /// The key is DER that this module cannot read at all.
    #[error("the private key does not parse as DER: {0}")]
    Der(DerError),
    /// A `OneAsymmetricKey` for an algorithm other than RFC 8017's `rsaEncryption`.
    #[error("the private key is not an RSA key")]
    NotRsa,
    /// The modulus is wider than [`MAX_MODULUS_BITS`], or even, or below two.
    #[error("the RSA modulus is unusable or wider than {MAX_MODULUS_BITS} bits")]
    Modulus,
    /// RFC 8017 section 7.2.2's one error, for every failure after the key was read.
    #[error("decryption error")]
    Decryption,
}

impl From<DerError> for RsaError {
    fn from(error: DerError) -> Self {
        Self::Der(error)
    }
}

/// An RSA private key in RFC 8017 section 3.2's first form, the pair `(n, d)`.
pub struct PrivateKey {
    /// The modulus `n`.
    modulus: Modulus,
    /// The private exponent `d`, the secret.
    exponent: Integer,
    /// `k`, the modulus's length in octets: RFC 8017 section 7.2.2's ciphertext length.
    length: usize,
}

impl std::fmt::Debug for PrivateKey {
    /// The width and nothing else: `d` is the one secret here and `n` identifies whose it is.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PrivateKey")
            .field("bits", &self.modulus.value.bits())
            .finish_non_exhaustive()
    }
}

impl PrivateKey {
    /// Reads a private key from RFC 5958's `OneAsymmetricKey` or RFC 8017's `RSAPrivateKey`.
    ///
    /// The two are told apart by their second member: a `OneAsymmetricKey` follows its version
    /// with the `privateKeyAlgorithm` `SEQUENCE`, an `RSAPrivateKey` with the modulus `INTEGER`.
    ///
    /// # Errors
    ///
    /// [`RsaError::Der`] or [`RsaError::Malformed`] for bytes that are neither structure,
    /// [`RsaError::NotRsa`] for another algorithm's key, and [`RsaError::Modulus`] for a modulus
    /// the arithmetic cannot use.
    pub fn from_der(bytes: &[u8]) -> Result<Self, RsaError> {
        let outer = single(bytes)?;
        let mut members = outer.children()?;
        let version = members.next_value()?.ok_or(RsaError::Malformed)?;
        if version.identifier != INTEGER {
            return Err(RsaError::Malformed);
        }
        let second = members.next_value()?.ok_or(RsaError::Malformed)?;
        if second.identifier == SEQUENCE {
            // RFC 5958 section 2: `privateKeyAlgorithm` and then `privateKey`, an `OCTET STRING`
            // whose contents are the algorithm's own encoding — for `rsaEncryption`, RFC 8017's
            // `RSAPrivateKey` (RFC 8017 appendix A.1.2).
            let algorithm = second
                .children()?
                .next_value()?
                .and_then(|value| value.object_identifier())
                .ok_or(RsaError::Malformed)?;
            if algorithm != RSA_ENCRYPTION {
                return Err(RsaError::NotRsa);
            }
            let inner = members.next_value()?.ok_or(RsaError::Malformed)?;
            if inner.identifier != OCTET_STRING {
                return Err(RsaError::Malformed);
            }
            return Self::from_rsa_private_key(single(inner.contents)?);
        }
        Self::from_rsa_private_key(outer)
    }

    /// RFC 8017 appendix A.1.2's `RSAPrivateKey`: `version`, `modulus`, `publicExponent`,
    /// `privateExponent`, then the components only the second key form uses.
    fn from_rsa_private_key(key: Value<'_>) -> Result<Self, RsaError> {
        let mut members = key.children()?;
        let mut integer = || -> Result<&[u8], RsaError> {
            let value = members.next_value()?.ok_or(RsaError::Malformed)?;
            if value.identifier == INTEGER {
                Ok(value.contents)
            } else {
                Err(RsaError::Malformed)
            }
        };
        let _version = integer()?;
        let modulus = integer()?;
        let _public_exponent = integer()?;
        let private_exponent = integer()?;
        let bits = significant_bits(modulus);
        if bits > MAX_MODULUS_BITS {
            return Err(RsaError::Modulus);
        }
        let modulus = Integer::from_be_bytes(modulus).ok_or(RsaError::Modulus)?;
        let exponent = Integer::from_be_bytes(private_exponent).ok_or(RsaError::Malformed)?;
        // RFC 8017 section 3.2: "d is a positive integer less than n". A key that is not cannot
        // decrypt anything, and an exponent wider than the modulus would be work the key's width
        // does not account for.
        if exponent.is_zero() || !exponent.less_than(&modulus) {
            return Err(RsaError::Malformed);
        }
        let modulus = Modulus::new(&modulus).ok_or(RsaError::Modulus)?;
        Ok(Self {
            modulus,
            exponent,
            length: bits.div_ceil(8),
        })
    }

    /// `k`, the modulus's length in octets.
    #[must_use]
    pub fn length(&self) -> usize {
        self.length
    }

    /// RFC 8017 section 7.2.2, `RSAES-PKCS1-V1_5-DECRYPT (K, C)`.
    ///
    /// # Errors
    ///
    /// [`RsaError::Decryption`] for every failure the section names — a ciphertext whose length
    /// is not `k`, a `k` below 11, a representative not below `n`, and an encoded message that is
    /// not `0x00 || 0x02 || PS || 0x00 || M` with eight or more octets of `PS` — and for nothing
    /// else.
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, RsaError> {
        // Step 1: "If the length of the ciphertext C is not k octets (or if k < 11), output
        // 'decryption error' and stop."
        if ciphertext.len() != self.length || self.length < 11 {
            return Err(RsaError::Decryption);
        }
        // Step 2: OS2IP, RSADP, I2OSP.
        let representative = Integer::from_be_bytes(ciphertext).ok_or(RsaError::Decryption)?;
        let message = self.rsadp(&representative)?;
        let encoded = message.be_bytes(self.length);
        decode_eme_pkcs1_v1_5(&encoded).ok_or(RsaError::Decryption)
    }

    /// RFC 8017 section 5.1.2, RSADP, in the key's first form: `m = c^d mod n`.
    fn rsadp(&self, representative: &Integer) -> Result<Integer, RsaError> {
        // Step 1: "If the ciphertext representative c is not between 0 and n - 1, output
        // 'ciphertext representative out of range' and stop." Section 7.2.2 step 2.b turns that
        // output into its own "decryption error".
        if !representative.less_than(&self.modulus.value) {
            return Err(RsaError::Decryption);
        }
        Ok(modpow(representative, &self.exponent, &self.modulus))
    }
}

/// RFC 8017 section 7.2.2 step 3: `EM = 0x00 || 0x02 || PS || 0x00 || M`, `PS` at least eight
/// non-zero octets, or `None`.
///
/// Every octet is read and every condition is folded into one flag before the answer is taken,
/// so that which check failed is visible neither in the result nor in the work done — the
/// section's note, against Bleichenbacher's attack.
fn decode_eme_pkcs1_v1_5(encoded: &[u8]) -> Option<Vec<u8>> {
    let first = encoded.first().copied().unwrap_or(1);
    let second = encoded.get(1).copied().unwrap_or(0);
    let mut good = u8::from(first == 0x00) & u8::from(second == 0x02);
    // The index of the first zero octet after the two-octet header, or zero where none is found.
    let mut separator = 0usize;
    let mut looking = 1u8;
    for (index, &octet) in encoded.iter().enumerate().skip(2) {
        let is_zero = u8::from(octet == 0);
        let take = is_zero & looking;
        // A branch-free select: `index` where this is the first zero, the old value otherwise.
        let mask = usize::from(take).wrapping_neg();
        separator = (index & mask) | (separator & !mask);
        looking &= !take & 1;
    }
    // "if there is no octet with hexadecimal value 0x00 to separate PS from M": still looking.
    good &= !looking & 1;
    // "or if the length of PS is less than 8 octets": PS runs from index 2 to the separator.
    good &= u8::from(separator >= 2 + 8);
    (good == 1).then(|| {
        encoded
            .get(separator.saturating_add(1)..)
            .unwrap_or_default()
            .to_vec()
    })
}

/// The one value `bytes` encodes, with nothing after it.
fn single(bytes: &[u8]) -> Result<Value<'_>, RsaError> {
    let mut reader = Reader::new(bytes)?;
    let value = reader.next_value()?.ok_or(RsaError::Malformed)?;
    if value.identifier != SEQUENCE || reader.next_value()?.is_some() {
        return Err(RsaError::Malformed);
    }
    Ok(value)
}

#[cfg(test)]
pub(crate) mod tests {
    #![expect(
        clippy::arithmetic_side_effects,
        reason = "test code: offsets into fixtures of a known, small size"
    )]

    use super::{PrivateKey, RsaError, decode_eme_pkcs1_v1_5};
    use crate::bigint::{Integer, Modulus, modpow};

    /// A 1024-bit two-prime RSA key, RFC 5958's `OneAsymmetricKey`, made for these tests with
    /// `openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:1024` and `openssl pkcs8 -topk8
    /// -nocrypt -outform DER`. Its public exponent is 65537. A key is data, not an expected
    /// value: every expected value below is a message these tests encode themselves.
    pub(crate) const PKCS8: &str = "\
30820276020100300d06092a864886f70d0101010500048202603082025c02010002818100eb365a97770f46d9cf516a\
dba156d86d4a45f4de743c1d0f42d5011695197f9905cc16e982af627712a36ebeecad2188874759f439475a4d5b7211\
9d28730c45684d2427652db31da8e8c4f295037635754eb1aed97fbb4172ae6272976cde84490032330c9f27bfae7697\
758d46cf0640538c525889a9cfb49518ac4fd4e4c1020301000102818100b59a79c7883d5b4ec02af47e2ecb86be9b21\
f835e938a0972fda95c481c9f62287bf53d166fd52e0ec9b12de868e05de0bd4a33755222a2e9ff42a7554cdbf223cb4\
031530bf60ede5a055940029bd022c048d0f2725f2ccc2535a23c1b7e8bd6e32edc4b66aaa19fdc60fb846c0b432b699\
f682860208a6b2c1b7444736ef81024100f97df577c47692c3370dfb8fdefc354ead53c94411461e1ce0b5dab4d8a5de\
4725e88ce446430d54a95d8b531e5180ffd90429e678cbbb155a1f4901f79def45024100f15909f842822d70b6820cff\
08c4ee31546456876aa9c21f676e87ddfbd5a3e3d0ce685aefba48c72e382f50a9ae75b68898c76c2cc82de7961e031e\
4fa8894d02406df163ca73790d13999f3fcc16a76d964e2b2df620e6953e84879ee932cf65345530356622d0497f9562\
f3929d39ad6f11631453cbc77b99dd2d0c2f21bf274102404de149924f03eadde046ff229d15cf3723e06cfbadb3b1ec\
a857c6881f67c547126e48c66f6d5166d4c7549d56e3f12041dbeb1e4125278affaa179f845d025102401b1387537ac7\
25cd2c47f7ac5328431c043e6d1ce2c546e5111abbe2801f0fa01426f251c8f7e71040755aa7b8cfb2a4e6a4978b5233\
237579efb9b3c6c245b7";

    /// Where the `RSAPrivateKey` starts inside [`PKCS8`]: after the outer header (4 octets), the
    /// version (3), the algorithm identifier (15) and the `OCTET STRING`'s header (4).
    pub(crate) const BARE_AT: usize = 26;

    pub(crate) fn hex(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|at| u8::from_str_radix(&text[at..at + 2], 16).unwrap())
            .collect()
    }

    /// The modulus of [`PKCS8`], read from the key by hand: the `RSAPrivateKey`'s second member.
    fn modulus_and_public_exponent() -> (Vec<u8>, Vec<u8>) {
        let bytes = hex(PKCS8);
        let mut outer = crate::der::Reader::new(&bytes[BARE_AT..]).unwrap();
        let key = outer.next_value().unwrap().unwrap();
        let mut members = key.children().unwrap();
        let _version = members.next_value().unwrap().unwrap();
        let n = members.next_value().unwrap().unwrap().contents.to_vec();
        let e = members.next_value().unwrap().unwrap().contents.to_vec();
        (n, e)
    }

    /// RFC 8017 section 5.1.1's RSAEP, `c = m^e mod n`, so that a test can encrypt what it wrote.
    pub(crate) fn rsaep(encoded: &[u8]) -> Vec<u8> {
        let (n, e) = modulus_and_public_exponent();
        let modulus = Modulus::new(&Integer::from_be_bytes(&n).unwrap()).unwrap();
        let m = Integer::from_be_bytes(encoded).unwrap();
        modpow(&m, &Integer::from_be_bytes(&e).unwrap(), &modulus).be_bytes(encoded.len())
    }

    /// RFC 8017 section 7.2.1 step 2, `EM = 0x00 || 0x02 || PS || 0x00 || M`, with a `PS` of
    /// non-zero octets this test chooses rather than random ones.
    pub(crate) fn eme_pkcs1_v1_5(message: &[u8], k: usize) -> Vec<u8> {
        let mut encoded = vec![0x00, 0x02];
        encoded.resize(k - message.len() - 1, 0x5A);
        encoded.push(0x00);
        encoded.extend_from_slice(message);
        encoded
    }

    #[test]
    fn a_message_encoded_and_encrypted_by_rfc_8017_decrypts_to_itself() {
        // RFC 8017 sections 7.2.1 and 7.2.2 are inverses: what RSAEP over section 7.2.1's
        // encoding produced, section 7.2.2 gives back.
        let key = PrivateKey::from_der(&hex(PKCS8)).unwrap();
        assert_eq!(key.length(), 128);
        let message = b"sixteen octets!!";
        let ciphertext = rsaep(&eme_pkcs1_v1_5(message, 128));
        assert_eq!(key.decrypt(&ciphertext).unwrap(), message);
    }

    #[test]
    fn the_bare_rsa_private_key_reads_as_the_same_key() {
        // RFC 8017 appendix A.1.2's structure is what RFC 5958's `privateKey` holds for
        // `rsaEncryption`; read on its own it is the same key.
        let bytes = hex(PKCS8);
        let key = PrivateKey::from_der(&bytes[BARE_AT..]).unwrap();
        let ciphertext = rsaep(&eme_pkcs1_v1_5(b"m", 128));
        assert_eq!(key.decrypt(&ciphertext).unwrap(), b"m");
    }

    #[test]
    fn every_step_three_failure_is_the_one_decryption_error() {
        // RFC 8017 section 7.2.2 step 3's four conditions, each planted into an otherwise good
        // encoding, and step 1's length check.
        let key = PrivateKey::from_der(&hex(PKCS8)).unwrap();
        let good = eme_pkcs1_v1_5(b"payload", 128);
        let mut first = good.clone();
        first[0] = 0x01;
        let mut second = good.clone();
        second[1] = 0x01;
        let mut no_separator = good.clone();
        let at = no_separator.len() - 8;
        no_separator[at] = 0x5A;
        let mut short_ps = vec![0x00, 0x02, 1, 2, 3, 4, 5, 6, 7, 0x00];
        short_ps.resize(128, 0x41);
        for planted in [first, second, no_separator, short_ps] {
            assert_eq!(key.decrypt(&rsaep(&planted)), Err(RsaError::Decryption));
        }
        assert_eq!(key.decrypt(&[0; 127]), Err(RsaError::Decryption));
        // A representative not below n: all ones is wider than any 1024-bit modulus allows.
        assert_eq!(key.decrypt(&[0xFF; 128]), Err(RsaError::Decryption));
    }

    #[test]
    fn eight_octets_of_padding_is_the_least_the_decoding_accepts() {
        // "if the length of PS is less than 8 octets": eight is accepted and seven is not.
        let mut eight = vec![0x00, 0x02, 9, 9, 9, 9, 9, 9, 9, 9, 0x00, 0x41];
        assert_eq!(decode_eme_pkcs1_v1_5(&eight), Some(vec![0x41]));
        eight.remove(2);
        assert_eq!(decode_eme_pkcs1_v1_5(&eight), None);
        // An empty M after the separator is a message of no octets, not a failure.
        assert_eq!(
            decode_eme_pkcs1_v1_5(&[0x00, 0x02, 1, 1, 1, 1, 1, 1, 1, 1, 0x00]),
            Some(Vec::new())
        );
    }

    #[test]
    fn a_key_for_another_algorithm_is_named() {
        // RFC 5958's `privateKeyAlgorithm` (octets 11 to 19 of [`PKCS8`]) changed to name
        // `id-RSASSA-PSS`, 1.2.840.113549.1.1.10: a key, but not one `rsaEncryption` names.
        let mut bytes = hex(PKCS8);
        assert_eq!(bytes[19], 0x01);
        bytes[19] = 0x0A;
        assert_eq!(PrivateKey::from_der(&bytes).unwrap_err(), RsaError::NotRsa);
        assert_eq!(
            PrivateKey::from_der(&[0x30, 0x00]).err(),
            Some(RsaError::Malformed)
        );
        assert!(format!("{:?}", PrivateKey::from_der(&hex(PKCS8)).unwrap()).contains("1024"));
    }
}
