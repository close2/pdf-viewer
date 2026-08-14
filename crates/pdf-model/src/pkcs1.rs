//! RSASSA-PKCS1-v1_5 verification: the arithmetic behind a signature's *second* question.
//!
//! ISO 32000-2 §12.8.3.2 is titled "PKCS #1 signatures" and says what the standard expects of a
//! processor that verifies one:
//!
//! > The PKCS #1 standard supports several public-key cryptographic algorithms and digest
//! > methods, including RSA encryption, DSA signatures, and SHA-1 and MD5 digests.
//!
//! and Table 260 states the sizes: "RSA Algorithm Support | Up to 1024-bit (PDF 1.3) Up to
//! 2048-bit (PDF 1.5) Up to 4096-bit (PDF 1.5)". This module is the RSA half of that. ADR 0229 has
//! the decision.
//!
//! **Table 260 names three algorithm families and this tree now verifies two**: DSA is
//! [`crate::dsa`] since the four-hundred-and-seventy-ninth session, and ECDSA — with the `EdDSA`
//! that ISO/TS 32002 adds beside it — is refused with an argument rather than half-written (ADR
//! 0314). Both refusals are reported by the object identifier the file states rather than skipped
//! ([`crate::signature::Authenticity`]).
//!
//! **`id-RSASSA-PSS` is not this construction and is deliberately not treated as it.** It shares
//! RFC 8017's `pkcs-1` arc and states a different padding, so a reader that matched the arc would
//! verify the wrong thing. It is [`crate::pss`], since the four-hundred-and-eighty-seventh
//! session (ADR 0322); the one thing the two schemes share is [`rsavp1`], because RFC 8017
//! itself invokes that primitive by name from both.
//!
//! # What is verified, and by which construction
//!
//! Internet RFC 8017 section 8.2.2 verifies a PKCS #1 v1.5 signature by *re-encoding* rather than by
//! taking the recovered block apart:
//!
//! - `m = s^e mod n`, the signature raised to the public exponent — [`modpow`];
//! - the expected block `EM = 0x00 || 0x01 || PS || 0x00 || T` built from the digest
//!   the verifier computed itself — [`encode`];
//! - the two compared as byte strings.
//!
//! **Encode-and-compare rather than parse-and-check, and that is a decision rather than a
//! convenience.** A verifier that parses the recovered block — skipping padding until it finds a
//! zero, then reading whatever `DigestInfo` follows — accepts blocks with trailing bytes nobody
//! signed, which is Bleichenbacher's forgery against small public exponents. Comparing whole
//! blocks of exactly `k` octets has no room for one. The same property is why a *mistake* in this
//! module is safe in one direction: assuming the wrong padding, the wrong digest or the wrong
//! algorithm produces "does not verify", never a false "verifies".
//!
//! # Why the construction is in the tree and the arithmetic is not
//!
//! The *scheme* — encode-and-compare, the budgets, the refusal names — stays here: ADR 0229
//! declined the `rsa` crate because a whole-scheme dependency parses keys and signatures with a
//! strict DER stack this corpus contradicts, and that reasoning stands (ADR 0331 re-measured it).
//! The *arithmetic* under it is `RustCrypto`'s `crypto-bigint` since the four-hundred-and-ninety-
//! sixth session, by the project owner's decision: [`crate::bigint`] is the seam, and ADR 0331
//! has the argument. ADR 0229's observation survives the port — **there is no secret** in a
//! verification, every number came out of the file, and nothing here needs constant time.
//!
//! # The budgets, and what each costs
//!
//! - [`MAX_MODULUS_BITS`] — twice Table 260's largest, so that a key beyond the standard is
//!   refused by name rather than by running out of stack.
//! - [`MAX_EXPONENT_BITS`] — the modular exponentiation's work is one squaring per exponent bit,
//!   so an unbounded exponent is unbounded work over a number a stranger chose. The two public
//!   exponents in practical use are 3 and 65537, seventeen bits between them.
//!
//! Both are stated as this program's budgets rather than as anything the standard says, and both
//! are reported: [`Pkcs1Error`] names which one a file exceeded.

use crate::bigint::{Integer, Modulus, modpow, significant_bits};
use crate::cms::Digest;

/// The widest modulus this module will exponentiate, in bits.
///
/// Table 260's ceiling is 4096 ("Up to 4096-bit (PDF 1.5)"), and this is twice it: a key larger
/// than the standard describes is a file to report rather than a file to refuse silently, and one
/// larger than *this* is refused with [`Pkcs1Error::ModulusTooLarge`]. The cost of the headroom is
/// one kilobyte of stack per big integer.
pub const MAX_MODULUS_BITS: usize = crate::bigint::MAX_BITS;

/// The largest public exponent this module will raise a signature to, in bits.
///
/// [`modpow`] costs one modular squaring per bit of the exponent, so this is a *time* budget on
/// arithmetic whose inputs come out of the file. The exponents real keys use are 3 and 65537, and
/// this leaves room for eight times as many bits as the larger of them needs while keeping the
/// worst case at 512 modular multiplications.
pub const MAX_EXPONENT_BITS: usize = 256;

/// What stopped a signature from being verified — a statement about the file, in every case.
///
/// None of these is "the signature is bad". A signature that is read, exponentiated and found not
/// to match is [`Ok(false)`](verify); these are the cases where the arithmetic never ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Pkcs1Error {
    /// The modulus is wider than [`MAX_MODULUS_BITS`].
    #[error("the signer's RSA modulus is wider than {MAX_MODULUS_BITS} bits")]
    ModulusTooLarge,
    /// The public exponent is wider than [`MAX_EXPONENT_BITS`].
    #[error("the signer's RSA public exponent is wider than {MAX_EXPONENT_BITS} bits")]
    ExponentTooLarge,
    /// The modulus is zero, one, or even.
    ///
    /// RFC 8017 section 3.1 makes an RSA modulus "a product of u distinct odd primes", so an even
    /// one is not an RSA modulus at all — and the Montgomery reduction [`modpow`] uses exists only
    /// for an odd one, which is why this is refused rather than worked around.
    #[error("the signer's RSA modulus is not an odd number greater than one")]
    ModulusNotOdd,
    /// The public exponent is zero.
    #[error("the signer's RSA public exponent is zero")]
    ExponentZero,
    /// RFC 8017 section 8.2.2 step 1, whose words are "[i]f the length of the signature S is not
    /// k octets, output \"invalid signature\" and stop".
    #[error("the signature is not as long as the modulus it was made with")]
    SignatureLength,
    /// RFC 8017 section 5.2.2 step 1: the signature, as an integer, is not less than the modulus.
    #[error("the signature is not less than the modulus, so it is not an RSA signature value")]
    SignatureNotReduced,
    /// The modulus is too short to hold the padded block RFC 8017 section 9.2 requires: step 3 is
    /// "[i]f emLen < tLen + 11, output \"intended encoded message length too short\" and stop".
    #[error("the modulus is too short to carry a PKCS #1 v1.5 block for this digest")]
    ModulusTooShortForDigest,
}

/// The public half of an RSA key, as the two integers RFC 8017 section 3.1 defines it as.
///
/// Both are big-endian, exactly as an X.509 `RSAPublicKey` writes them, and both are borrowed
/// from the certificate: nothing here owns or copies a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicKey<'a> {
    /// `n`, the modulus.
    pub modulus: &'a [u8],
    /// `e`, the public exponent.
    pub exponent: &'a [u8],
}

impl PublicKey<'_> {
    /// The modulus's width in bits, which is the size a person means by "a 2048-bit key".
    ///
    /// Zero for a modulus of zero, which [`verify`] refuses anyway.
    #[must_use]
    pub fn bits(&self) -> usize {
        significant_bits(self.modulus)
    }
}

/// Whether `signature` is an RSASSA-PKCS1-v1_5 signature over `message_digest` under `key`.
///
/// `message_digest` is the digest the *caller* computed over whatever RFC 5652 says is signed —
/// this module never hashes anything, so that nothing here can disagree with
/// [`crate::cms::Digest`] about what was hashed.
///
/// # Errors
///
/// A [`Pkcs1Error`] where the key or the signature is outside this module's budgets or is not
/// shaped like an RSA one at all. A signature that is well-formed and simply does not verify is
/// `Ok(false)`, and the two are kept apart because only the second says anything about the file's
/// honesty.
pub fn verify(
    key: PublicKey<'_>,
    signature: &[u8],
    digest: Digest,
    message_digest: &[u8],
) -> Result<bool, Pkcs1Error> {
    let (message, modulus_bits) = rsavp1(key, signature)?;
    let length = modulus_bits.div_ceil(8);
    let expected = encode(digest, message_digest, length)?;
    Ok(message.be_bytes(length) == expected)
}

/// RFC 8017 section 5.2.2's `RSAVP1` primitive, with section 8.2.2 step 1's length check ahead
/// of it: `m = s^e mod n`, returned beside the modulus's width in bits.
///
/// **Shared by both of the RFC's signature schemes, because the RFC itself shares it**: section
/// 8.2.2 step 2.b and section 8.1.2 step 2.b both read "m = RSAVP1 ((n, e), s)", so this is the
/// one construction [`verify`] and [`crate::pss::verify`] have in common — everything after it
/// is a different padding and stays in its own module.
///
/// The length check belongs with the primitive rather than with either scheme: `k` is the
/// modulus's length in octets, both schemes state the same step 1 over it, and a signature of
/// any other length cannot equal an encoded block whatever it contains.
///
/// # Errors
///
/// A [`Pkcs1Error`] naming the budget or shape refusal; see [`verify`].
pub(crate) fn rsavp1(key: PublicKey<'_>, signature: &[u8]) -> Result<(Integer, usize), Pkcs1Error> {
    let modulus = Integer::from_be_bytes(key.modulus).ok_or(Pkcs1Error::ModulusTooLarge)?;
    let exponent = Integer::from_be_bytes(key.exponent).ok_or(Pkcs1Error::ExponentTooLarge)?;
    if exponent.bits() > MAX_EXPONENT_BITS {
        return Err(Pkcs1Error::ExponentTooLarge);
    }
    if exponent.is_zero() {
        return Err(Pkcs1Error::ExponentZero);
    }
    let Some(modulus) = Modulus::new(&modulus) else {
        return Err(Pkcs1Error::ModulusNotOdd);
    };
    let modulus_bits = modulus.value.bits();
    if signature.len() != modulus_bits.div_ceil(8) {
        return Err(Pkcs1Error::SignatureLength);
    }
    let value = Integer::from_be_bytes(signature).ok_or(Pkcs1Error::ModulusTooLarge)?;
    // Section 5.2.2 step 1: "If the signature representative s is not between 0 and n - 1,
    // output "signature representative out of range" and stop."
    if !value.less_than(&modulus.value) {
        return Err(Pkcs1Error::SignatureNotReduced);
    }
    Ok((modpow(&value, &exponent, &modulus), modulus_bits))
}

/// RFC 8017 section 9.2's `EMSA-PKCS1-v1_5-ENCODE`, for an encoded message of `length` octets.
///
/// Step 5 gives the block as `EM = 0x00 || 0x01 || PS || 0x00 || T`, where `T` is the DER encoding of `DigestInfo ::= SEQUENCE { digestAlgorithm
/// AlgorithmIdentifier, digest OCTET STRING }` and `PS` is `0xFF` repeated to fill the block —
/// "[t]he length of PS will be at least 8 octets" (step 4). The algorithm identifier's parameters
/// are written as an explicit `NULL` rather than omitted because RFC 8017 Appendix A.2.4 requires
/// it: "the parameters field associated with this OID in a value of type `AlgorithmIdentifier` SHALL
/// have a value of type NULL".
///
/// # Errors
///
/// [`Pkcs1Error::ModulusTooShortForDigest`] where section 9.2 step 3's `emLen < tLen + 11` holds — a
/// modulus with no room for at least eight octets of padding.
fn encode(digest: Digest, message_digest: &[u8], length: usize) -> Result<Vec<u8>, Pkcs1Error> {
    let oid = digest.oid();
    // AlgorithmIdentifier ::= SEQUENCE { OBJECT IDENTIFIER, NULL }, then the digest as an OCTET
    // STRING, then both inside one SEQUENCE. Every length here is under 128, so X.690's short
    // form is the only one reachable and the encoder needs no long form.
    let algorithm = sequence(&[der_value(0x06, oid), der_value(0x05, &[])].concat())?;
    let info = sequence(&[algorithm, der_value(0x04, message_digest)].concat())?;
    // section 9.2 step 3, verbatim in the error's documentation: three octets for `0x00 0x01 … 0x00`
    // plus at least eight of padding.
    let padding = length
        .checked_sub(info.len())
        .and_then(|room| room.checked_sub(3))
        .filter(|&padding| padding >= 8)
        .ok_or(Pkcs1Error::ModulusTooShortForDigest)?;
    let mut out = Vec::with_capacity(length);
    out.push(0x00);
    out.push(0x01);
    out.resize(padding.saturating_add(2), 0xFF);
    out.push(0x00);
    out.extend_from_slice(&info);
    Ok(out)
}

/// One DER tag-length-value in X.690's short length form.
///
/// Only ever called on contents under 128 octets — an object identifier, a `NULL`, a digest, and
/// the two `SEQUENCE`s around them — so the long form is unreachable and [`sequence`] is where
/// that is checked rather than assumed.
fn der_value(identifier: u8, contents: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(contents.len().saturating_add(2));
    out.push(identifier);
    out.push(u8::try_from(contents.len().min(127)).unwrap_or(127));
    out.extend_from_slice(contents);
    out
}

/// A `SEQUENCE` around already-encoded members, refusing the long length form.
///
/// # Errors
///
/// [`Pkcs1Error::ModulusTooShortForDigest`] where the contents would need X.690's long form. A
/// `DigestInfo` for the widest digest here is 83 octets, so this is unreachable for the six
/// algorithms [`Digest`] names; it is a bound rather than a case.
fn sequence(contents: &[u8]) -> Result<Vec<u8>, Pkcs1Error> {
    if contents.len() > 127 {
        return Err(Pkcs1Error::ModulusTooShortForDigest);
    }
    Ok(der_value(0x30, contents))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{MAX_EXPONENT_BITS, MAX_MODULUS_BITS, Pkcs1Error, PublicKey, encode, verify};
    use crate::cms::Digest;

    /// A 2048-bit RSA modulus, and below it a signature made with the matching private key.
    ///
    /// **A test vector, generated once and pasted in — not an oracle.** `CLAUDE.md` principle 5
    /// forbids treating another implementation's output as the definition of correct, and this is
    /// not that: RFC 8017 defines `s^e mod n` and section 9.2's block, and what a vector does is pin that
    /// *these* two hundred lines compute them. Nothing about the vector decides what is correct;
    /// it decides only whether the arithmetic here is the arithmetic the RFC states, which is a
    /// question a hand-checked small case cannot settle at 2048 bits.
    pub(crate) const MODULUS: &str = "\
        c56a39fbe4fd00ac43c8080e81b5b2a314c57647dd5317854109d621e44713ca\
        fabd7fbe5275f933f5956fd158c8fe6dab374475949366675f4feff22689459f\
        d676925b9b55a1dec6274debe37905a3f843d322bf4495164ec6e626f8c0f198\
        f538d93e9ab8be31250ce1af107a53415c663ddcefd8cef220613c58e1a9870e\
        a2f67576e85d6457019c9b86422b3df59a664089e0d5a9f97f921940eab4d951\
        32a1b19870635d6372e4275c06d39f8943d6a971c46fa86199e5acbc24ed0a9f\
        7c8aa50a5e57ef9df109c0bac20be6022abede06cc603b1bed0e4d08e7c836af\
        378d310edb7a752f2a9541f6da47b06291daaa0e25ff4b29f6e2fc926f4789b7";

    /// The PKCS #1 v1.5 signature of `SHA-256(b"the signed bytes")` under the key above.
    const SIGNATURE: &str = "\
        a8051d60075059a973c9845268a3c7848a60e36a1904180dd95d4226dad15bea\
        4050dd59907bbb865d915611e55ae1fa1017a71fe084b90695f6ed5ca75fdc91\
        7caf751663b994ed3f6f7cac36729a87b237668eec74aeb6040a682ff5c87e75\
        1763e17a5d010383187a73af9ee5e8e4f890802882baba7dbb0873847dfa55a1\
        db7bd5998ddd9456b79eb269184ac578e1451d29ea8748f6b8d2bfe4c9ec834f\
        9c9f917089d8210aec49c8538e72f05ed93fe4d4b9145acd94db022a4d16b93e\
        7ca648f59a45bcc30786e72cec2ce3147700cd6cebf2e48ff8eb1c777802003b\
        7869fdaa3f4f11773f78f2002bf1a66c019e62946c3c7aed327940e7e0121204";

    /// Hexadecimal to bytes, for the two constants above.
    pub(crate) fn hex(text: &str) -> Vec<u8> {
        text.as_bytes()
            .chunks(2)
            .filter_map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
            .collect()
    }

    /// The block RFC 8017 section 9.2 describes, checked against the clause's own description of it.
    #[test]
    fn the_encoded_block_is_the_one_rfc_8017_describes() {
        let digest = Digest::Sha256.compute(&[b"abc"]);
        let block = encode(Digest::Sha256, &digest, 256).expect("room for it");
        assert_eq!(block.len(), 256);
        assert_eq!(block.first(), Some(&0x00));
        assert_eq!(block.get(1), Some(&0x01));
        // `DigestInfo` for SHA-256 is 51 octets — a `SEQUENCE` around SHA-256's nine-octet
        // identifier with a `NULL` and a 32-octet `OCTET STRING` — so PS runs from index 2 to 204.
        assert!(
            block
                .get(2..204)
                .is_some_and(|run| run.iter().all(|&byte| byte == 0xFF)),
            "PS is 0xFF throughout"
        );
        assert_eq!(block.get(204), Some(&0x00), "the separator after PS");
        let info = block.get(205..).unwrap_or(&[]);
        assert_eq!(info.len(), 51, "and DigestInfo fills the rest of the block");
        assert_eq!(info.first(), Some(&0x30), "a DigestInfo SEQUENCE follows");
        assert!(info.ends_with(&digest), "ending in the digest itself");
        assert_eq!(
            encode(Digest::Sha256, &digest, 50),
            Err(Pkcs1Error::ModulusTooShortForDigest),
            "section 9.2 step 3's emLen < tLen + 11"
        );
    }

    /// The whole of [`verify`], on a real 2048-bit key and a real signature.
    #[test]
    fn a_signature_verifies_under_the_key_that_made_it_and_under_no_other() {
        let modulus = hex(MODULUS);
        let signature = hex(SIGNATURE);
        let key = PublicKey {
            modulus: &modulus,
            exponent: &[0x01, 0x00, 0x01],
        };
        let digest = Digest::Sha256.compute(&[b"the signed bytes"]);
        assert_eq!(verify(key, &signature, Digest::Sha256, &digest), Ok(true));

        // One bit of the message, and it no longer verifies.
        let other = Digest::Sha256.compute(&[b"the signed byteS"]);
        assert_eq!(verify(key, &signature, Digest::Sha256, &other), Ok(false));

        // One bit of the signature, likewise.
        let mut tampered = signature.clone();
        if let Some(last) = tampered.last_mut() {
            *last ^= 1;
        }
        assert_eq!(
            verify(key, &tampered, Digest::Sha256, &digest),
            Ok(false),
            "a signature is not its neighbour"
        );

        // And the same digest under the wrong algorithm identifier: the recovered block carries
        // SHA-256's object identifier, so SHA-512's DigestInfo cannot equal it.
        assert_eq!(
            verify(key, &signature, Digest::Sha512, &digest),
            Ok(false),
            "the algorithm is inside the block that is compared"
        );

        // A different exponent recovers a different number.
        assert_eq!(
            verify(
                PublicKey {
                    modulus: &modulus,
                    exponent: &[0x03],
                },
                &signature,
                Digest::Sha256,
                &digest,
            ),
            Ok(false)
        );
    }

    /// Every budget and every malformed shape, refused by name rather than by running out.
    #[test]
    fn the_budgets_are_reported_rather_than_reached() {
        let digest = Digest::Sha256.compute(&[b"abc"]);
        let modulus = hex(MODULUS);
        let long = vec![0u8; modulus.len()];

        let mut even = modulus.clone();
        if let Some(last) = even.last_mut() {
            *last = 0x02;
        }
        assert_eq!(
            verify(
                PublicKey {
                    modulus: &even,
                    exponent: &[0x01, 0x00, 0x01]
                },
                &long,
                Digest::Sha256,
                &digest
            ),
            Err(Pkcs1Error::ModulusNotOdd)
        );
        assert_eq!(
            verify(
                PublicKey {
                    modulus: &modulus,
                    exponent: &[0x01, 0x00, 0x01]
                },
                &[0u8; 8],
                Digest::Sha256,
                &digest
            ),
            Err(Pkcs1Error::SignatureLength)
        );
        assert_eq!(
            verify(
                PublicKey {
                    modulus: &vec![0xFFu8; (MAX_MODULUS_BITS / 8) + 1],
                    exponent: &[0x03]
                },
                &[],
                Digest::Sha256,
                &digest
            ),
            Err(Pkcs1Error::ModulusTooLarge)
        );
        assert_eq!(
            verify(
                PublicKey {
                    modulus: &modulus,
                    exponent: &[0xFFu8; (MAX_EXPONENT_BITS / 8) + 1]
                },
                &long,
                Digest::Sha256,
                &digest
            ),
            Err(Pkcs1Error::ExponentTooLarge)
        );
        assert_eq!(
            verify(
                PublicKey {
                    modulus: &modulus,
                    exponent: &[0x00]
                },
                &long,
                Digest::Sha256,
                &digest
            ),
            Err(Pkcs1Error::ExponentZero)
        );
        assert_eq!(
            verify(
                PublicKey {
                    modulus: &modulus,
                    exponent: &[0x03]
                },
                &vec![0xFFu8; modulus.len()],
                Digest::Sha256,
                &digest
            ),
            Err(Pkcs1Error::SignatureNotReduced)
        );
        // A signature of zero recovers zero, which no encoded block can be.
        assert_eq!(
            verify(
                PublicKey {
                    modulus: &modulus,
                    exponent: &[0x01, 0x00, 0x01]
                },
                &long,
                Digest::Sha256,
                &digest
            ),
            Ok(false)
        );
    }

    /// A key's stated width is its modulus's significant bits, not its byte string's length.
    #[test]
    fn a_keys_width_is_its_modulus_significant_bits() {
        assert_eq!(
            PublicKey {
                modulus: &[0x00, 0xFF, 0xFF],
                exponent: &[0x03]
            }
            .bits(),
            16
        );
        assert_eq!(
            PublicKey {
                modulus: &[0x01, 0x00],
                exponent: &[0x03]
            }
            .bits(),
            9
        );
        assert_eq!(
            PublicKey {
                modulus: &hex(MODULUS),
                exponent: &[0x03]
            }
            .bits(),
            2048
        );
    }
}
