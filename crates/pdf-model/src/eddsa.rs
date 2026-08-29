//! `EdDSA` signature verification: the row ISO/TS 32002 adds to Table 260, on Ed25519.
//!
//! ISO 32000-2's Table 260 has three algorithm families and none of them is this one. ISO/TS
//! 32002:2022 section 5.1.2 adds a fourth row to that table, and ISO/TS 32002 Table 2 spells that row's
//! first column "IETF RFC 8032, Edwards-curve Digital Signature Algorithm (EdDSA) (PDF 2.x) using
//! the Ed25519 or Ed448 elliptic curves" — for `adbe.pkcs7.detached`, `ETSI.CAdES.detached` and
//! `ETSI.RFC3161`, and **No** for the other two `/SubFilter` values. ISO/TS 32002 Table 4, in
//! section 5.1.3, pairs Ed25519 with SHA512 and Ed448 with SHAKE256.
//!
//! **Every sentence quoted from that document is in prose rather than in a blockquote**, for the
//! reason `cms::Digest` records: `tools/conformance` checks a rustdoc blockquote verbatim against
//! `doc/md/`'s ISO 32000-2, and words from another document would be unattributable there. The
//! quotation marks still mean verbatim.
//!
//! # This is not a fourth curve of the same shape as [`crate::ecdsa`]'s
//!
//! It is a different group law and a different construction: an Edwards curve, a signature that is
//! `R ‖ S` as fixed-width octets rather than two DER integers, a key that is a compressed point
//! rather than a SEC1 encoding, and — the difference that reaches this crate's shape — **a
//! signature over the message itself rather than over a digest of it**. RFC 8032's Ed25519 hashes
//! internally with SHA-512; there is no `e` to hand it. So [`verify`] takes the signed bytes,
//! where [`crate::ecdsa::verify`] and [`crate::pkcs1::verify`] take a digest.
//!
//! ISO/TS 32002 Table 4's "SHA512" is therefore not a parameter of this verification. It is what RFC 5652's
//! `digestAlgorithm` must state — the digest of the content, which question 1's `message-digest`
//! attribute carries — and ISO/TS 32002 section 5.1.4 is what ties the two together: "[i]f the
//! signedAttrs field is present in the SignerInfo field for the signer, then the same message
//! digest algorithm shall be used to compute both the digest of the SignedData encapContentInfo
//! eContent and the digest of the DER-encoded signedAttrs passed to the signature algorithm."
//!
//! # Ed448 is named and not computed
//!
//! ISO/TS 32002 Table 4's second curve has no verification here. `ed448-goldilocks`'s stable line carries the
//! field arithmetic without the signature scheme and sits on an older random-number stack, and its
//! 0.14 line is a pre-release, which this tree does not take (measured 2026-08-23; ADR 0532). A
//! certificate stating `id-Ed448` reaches a reader as that identifier — [`ID_ED448`] is the
//! constant it is recognised by — rather than as a silence.
//!
//! # Which of RFC 8032's two verification equations, and why the looser one is the right one
//!
//! RFC 8032 section 5.1.7 states the check and then states that a stricter one is optional:
//! "Check the group equation \[8\]\[S\]B = \[8\]R + \[8\]\[k\]A'. It's sufficient, but not
//! required, to instead check \[S\]B = R + \[k\]A'." `ed25519-dalek` offers both — the first as
//! `verify`/`multipart_verify`, the second plus a small-order rejection as `verify_strict`, whose
//! own documentation calls itself "technically non-RFC8032 compliant".
//!
//! **This module takes the specification's**, which is `multipart_verify`. Principle 5 decides it:
//! the stricter check would refuse a signature RFC 8032 says is valid, and a viewer that reported
//! a conforming signature as failing would be wrong about the file. The malleability the strict
//! form closes is not a forgery — it lets somebody holding a valid signature produce a second
//! value over the *same* message under the *same* key, which changes nothing a document signature
//! asserts. Both forms reject an unreduced `S`, which is the check RFC 8032 makes mandatory.
//!
//! # There is no secret here either
//!
//! ADR 0229's argument, unchanged: the key, the signature and the message all came out of a file a
//! stranger wrote. What the dependency is taken for is arithmetic that has been reviewed, which is
//! ADR 0331's owner decision, and not side-channel resistance that nothing here needs.

#![expect(
    clippy::doc_markdown,
    reason = "ISO/TS 32002's sentences are quoted verbatim and its ASN.1 names are camel case \
              throughout; a quotation with backticks added to please a lint is no longer a \
              quotation (the same reasoning `pdf_syntax::filter::Delimiting` records)"
)]

use const_oid::ObjectIdentifier;
use const_oid::db::rfc8410;
use ed25519_dalek::VerifyingKey;
use ed25519_dalek::ed25519::Signature;
use ed25519_dalek::ed25519::signature::MultipartVerifier as _;

/// RFC 8410 section 3's `id-Ed25519`, `1.3.101.112`.
///
/// The identifier a certificate's `subjectPublicKeyInfo` states for one of ISO/TS 32002 Table 4's
/// keys, and the
/// one a `SignerInfo` states as its `signatureAlgorithm` for one of its signatures — RFC 8419
/// makes them the same number, which is why one constant serves both.
pub const ID_ED25519: ObjectIdentifier = rfc8410::ID_ED_25519;

/// RFC 8410 section 3's `id-Ed448`, `1.3.101.113` — ISO/TS 32002 Table 4's other curve, recognised and
/// refused.
pub const ID_ED448: ObjectIdentifier = rfc8410::ID_ED_448;

/// RFC 8032 section 5.1's `b / 8`: an Ed25519 public key is 32 octets.
const KEY_OCTETS: usize = 32;

/// RFC 8032 section 5.1.6's signature, `R ‖ S`: two of those, so 64 octets.
const SIGNATURE_OCTETS: usize = 2 * KEY_OCTETS;

/// An Ed25519 public key, as far as verifying one needs.
///
/// `subjectPublicKey`'s octets, which RFC 8410 section 4 puts there raw rather than wrapped in a
/// structure. Whether they decompress to a point is [`verify`]'s question, because it is
/// arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicKey<'a> {
    /// The 32 octets of the compressed Edwards point.
    pub key: &'a [u8],
}

/// What stopped an `EdDSA` signature from being verified — a statement about the file, in every case.
///
/// None of these is "the signature is bad". A signature that is read and found not to match is
/// [`Ok(false)`](verify); these are the cases where the arithmetic never ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum EdDsaError {
    /// `subjectPublicKey` is not 32 octets, or is not a point.
    ///
    /// RFC 8032 section 5.1.7 makes both the same refusal: "Decode the public key A as point A'.
    /// If any of the decodings fail … the signature is invalid."
    #[error("the certificate's Ed25519 public key is not 32 octets of a point")]
    MalformedKey,
    /// The signature value is not 64 octets.
    ///
    /// Its length is fixed by the curve, so this is the whole of the budget this module needs.
    #[error("the signature value is not 64 octets")]
    MalformedSignature,
}

/// Verifies an Ed25519 signature over the bytes it was made on.
///
/// `message` is what RFC 5652 section 5.4 says the signature is over, in parts — the DER-encoded
/// `signedAttrs`, the encapsulated content, or the `/ByteRange`'s two halves — and it is passed
/// through as parts rather than joined, so that a signature over a whole document costs no copy of
/// it.
///
/// # Errors
///
/// An [`EdDsaError`] naming what stopped the check. A signature that is checked and does not match
/// is `Ok(false)`, and so is one whose `S` is not reduced, which RFC 8032 section 5.1.7 requires be
/// rejected.
pub fn verify(key: PublicKey<'_>, signature: &[u8], message: &[&[u8]]) -> Result<bool, EdDsaError> {
    let key: &[u8; KEY_OCTETS] = key.key.try_into().map_err(|_| EdDsaError::MalformedKey)?;
    let key = VerifyingKey::from_bytes(key).map_err(|_| EdDsaError::MalformedKey)?;
    let signature: &[u8; SIGNATURE_OCTETS] = signature
        .try_into()
        .map_err(|_| EdDsaError::MalformedSignature)?;
    let signature = Signature::from_bytes(signature);
    Ok(key.multipart_verify(message, &signature).is_ok())
}

/// A key, a certificate and a signature built once with `openssl` and pasted in.
///
/// **Test vectors rather than oracles**, on the footing ADR 0314 states and [`crate::ecdsa`]'s
/// fixtures repeat: the corpus contains no `EdDSA` signature at all, so a hand-made pair is the only
/// witness there is, and what it pins is that this module walks the encodings and hands the
/// dependency the right octets.
///
/// ```sh
/// openssl genpkey -algorithm ed25519 -out key.pem
/// openssl req -x509 -key key.pem -days 3650 -subj /CN=pdf-viewer -outform der -out cert.der
/// printf 'the signed bytes' | openssl pkeyutl -sign -inkey key.pem -rawin -out sig.bin
/// ```
#[cfg(test)]
pub(crate) mod fixtures {
    /// A self-signed Ed25519 certificate.
    pub(crate) const ED25519_CERTIFICATE: &str = "\
        308201593082010ba00302010202143601154d87c49ef6dbfa4d13caa5f81304\
        4ef50b300506032b657030223120301e06035504030c177064662d7669657765\
        7220656432353531392074657374301e170d3236303832333136343332305a17\
        0d3336303832303136343332305a30223120301e06035504030c177064662d76\
        696577657220656432353531392074657374302a300506032b65700321001c02\
        7ffe568ba8f2b72e2a801b102a036276716b3cdca74cd9f443f090bb03c1a353\
        3051301d0603551d0e0416041413182ecad54eaf3223653693c2a05ee82473ab\
        30301f0603551d2304183016801413182ecad54eaf3223653693c2a05ee82473\
        ab30300f0603551d130101ff040530030101ff300506032b6570034100816e21\
        4bb82bb8c6b25cd70ec95e715626862f4f3fae58466f838c6853a178393089c9\
        bcd2b11e37d551f1a122300c00352b307b59fe1339198ec00551ae410a";

    /// `R ‖ S` over `b"the signed bytes"` under the key above.
    pub(crate) const ED25519_SIGNATURE: &str = "\
        684a4550bb5c95ffac4c04f5067d59a0e9c572da92d175b4d996c1a18fd63d5d\
        fa81f9b4a28c26fe4f159d6259822126ad19f6f6e104fc4d11ee374ff5fc8407";
}

#[cfg(test)]
mod tests {
    use super::fixtures::{ED25519_CERTIFICATE, ED25519_SIGNATURE};
    use super::{EdDsaError, ID_ED448, verify};
    use crate::ecdsa::fixtures::{MESSAGE, hex};
    use crate::x509::{self, PublicKey};

    /// The key out of the certificate, for the one fixture that carries an Ed25519 one.
    fn key_of(certificate: &[u8]) -> super::PublicKey<'_> {
        let certificate = x509::parse(certificate).expect("a certificate");
        match certificate.public_key {
            PublicKey::Ed25519(key) => key,
            other => panic!("this certificate states id-Ed25519, not {other:?}"),
        }
    }

    /// The whole path: a signature made with a key verifies under the certificate's, and stops
    /// when one bit of the message moves.
    #[test]
    fn a_signature_verifies_under_its_own_certificates_key() {
        let certificate = hex(ED25519_CERTIFICATE);
        let key = key_of(&certificate);
        let signature = hex(ED25519_SIGNATURE);
        assert_eq!(verify(key, &signature, &[MESSAGE]), Ok(true));
        let mut moved = MESSAGE.to_vec();
        moved[0] ^= 0x01;
        assert_eq!(verify(key, &signature, &[moved.as_slice()]), Ok(false));
    }

    /// The message is verified in parts, and the parts are the message.
    ///
    /// The one property that could go wrong silently in the multipart form: a verifier that hashed
    /// the parts with anything between them, or in the wrong order, would still answer `false` for
    /// a wrong message and `true` for nothing at all. So the same bytes split differently must
    /// verify, and the same bytes in the other order must not.
    #[test]
    fn the_parts_are_hashed_as_one_message_in_the_order_given() {
        let certificate = hex(ED25519_CERTIFICATE);
        let key = key_of(&certificate);
        let signature = hex(ED25519_SIGNATURE);
        let (head, tail) = MESSAGE.split_at(4);
        assert_eq!(verify(key, &signature, &[head, tail]), Ok(true));
        assert_eq!(verify(key, &signature, &[tail, head]), Ok(false));
    }

    /// A signature or a key that is not the width the curve fixes, named rather than padded.
    #[test]
    fn a_value_of_the_wrong_width_is_refused_by_name() {
        let certificate = hex(ED25519_CERTIFICATE);
        let key = key_of(&certificate);
        let signature = hex(ED25519_SIGNATURE);
        assert_eq!(
            verify(key, &signature[..63], &[MESSAGE]),
            Err(EdDsaError::MalformedSignature)
        );
        assert_eq!(
            verify(super::PublicKey { key: &[0u8; 31] }, &signature, &[MESSAGE]),
            Err(EdDsaError::MalformedKey)
        );
    }

    /// One bit of the signature moved is a signature that does not verify, not one that panics.
    #[test]
    fn a_signature_one_bit_away_does_not_verify() {
        let certificate = hex(ED25519_CERTIFICATE);
        let key = key_of(&certificate);
        for index in [0usize, 31, 32, 63] {
            let mut signature = hex(ED25519_SIGNATURE);
            signature[index] ^= 0x01;
            assert_eq!(
                verify(key, &signature, &[MESSAGE]),
                Ok(false),
                "octet {index}"
            );
        }
    }

    /// ISO/TS 32002 Table 4's other curve, recognised by its number so that a report can name
    /// it.
    #[test]
    fn ed448_is_a_number_this_program_can_print() {
        assert_eq!(
            x509::dotted(ID_ED448.as_bytes()).as_deref(),
            Some(ID_ED448.to_string().as_str())
        );
        assert_ne!(ID_ED448, super::ID_ED25519);
    }
}
