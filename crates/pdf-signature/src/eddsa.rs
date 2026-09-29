//! `EdDSA` signature verification: the row ISO/TS 32002 adds to Table 260, on both its curves.
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
//! # Two curves behind one seam, and one of them is this tree's own
//!
//! [`Curve`] is ISO/TS 32002 Table 4's pair. Ed25519 is `ed25519-dalek`'s. **Ed448 is computed by a
//! private module of this crate** (`ed448.rs`), because no stable, reviewed package carries its
//! signature scheme: `ed448-goldilocks` has it only on a pre-release line, which this tree does not
//! take. The project owner's answer A170 makes that module a stopgap with a stated exit — the day a
//! stable, reviewed crate covers the curve, the swap is decided on `doc/stack.md`'s terms — and
//! [`verify`]'s one `match` on [`Curve`] is the seam the swap replaces one arm of. ADR 1386.
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
//! **One refusal sits beside the equation, and it is about the key rather than the signature**:
//! a key of small order is [`EdDsaError::SmallOrderKey`] on both curves. No key pair RFC 8032
//! generates has one, and under one the equation holds for a signature anybody can write for any
//! message — the `x509` fuzz target found exactly that for Ed448 (ADR 1386). A signature under a
//! real key is judged by the specification's equation and nothing else.
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

use crate::ed448;
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

/// RFC 8410 section 3's `id-Ed448`, `1.3.101.113` — ISO/TS 32002 Table 4's other curve, and the
/// same double duty as [`ID_ED25519`].
pub const ID_ED448: ObjectIdentifier = rfc8410::ID_ED_448;

/// RFC 8032 section 5.1's `b / 8`: an Ed25519 public key is 32 octets.
const ED25519_KEY_OCTETS: usize = 32;

/// RFC 8032 section 5.1.6's signature, `R ‖ S`: two of those, so 64 octets.
const ED25519_SIGNATURE_OCTETS: usize = 2 * ED25519_KEY_OCTETS;

/// One of ISO/TS 32002 Table 4's two Edwards curves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Curve {
    /// RFC 8032 section 5.1's Ed25519, `id-Ed25519`.
    Ed25519,
    /// RFC 8032 section 5.2's Ed448, `id-Ed448`.
    Ed448,
}

impl Curve {
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
            Self::Ed448 => 8 * ed448::KEY_OCTETS,
        }
    }
}

/// An `EdDSA` public key, as far as verifying one needs.
///
/// `subjectPublicKey`'s octets, which RFC 8410 section 4 puts there raw rather than wrapped in a
/// structure, and the curve the certificate's algorithm identifier named. Whether the octets
/// decompress to a point is [`verify`]'s question, because it is arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicKey<'a> {
    /// Which of the two curves the certificate's `algorithm` stated.
    pub curve: Curve,
    /// The octets of the compressed Edwards point: `b / 8` of them.
    pub key: &'a [u8],
}

/// What stopped an `EdDSA` signature from being verified — a statement about the file, in every case.
///
/// None of these is "the signature is bad". A signature that is read and found not to match is
/// [`Ok(false)`](verify); these are the cases where the arithmetic never ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum EdDsaError {
    /// `subjectPublicKey` is not the curve's `b / 8` octets, or is not a point.
    ///
    /// RFC 8032 section 5.1.7 and section 5.2.7 make both the same refusal: a public key that does
    /// not decode to a point is a signature that is invalid.
    #[error("the certificate's EdDSA public key is not a point of its curve's width")]
    MalformedKey,
    /// The signature value is not `2 · b / 8` octets — 64 for Ed25519, 114 for Ed448.
    ///
    /// Its length is fixed by the curve, so this is the whole of the budget this module needs.
    #[error("the signature value is not the width its curve fixes")]
    MalformedSignature,
    /// `subjectPublicKey` decodes to a point of small order — one the cofactor multiplies to the
    /// neutral point.
    ///
    /// RFC 8032 section 5.1.5 and section 5.2.5 make a public key `[s]B`, which is never such a
    /// point, so no key generation the RFC describes produces one. Under one, the verification
    /// equation holds for a signature anybody can write down for any message — a small-order `R`
    /// and `S = 0` — so answering "verified" would say nothing about any signer. It is refused by
    /// name rather than verified or failed (ADR 1386).
    #[error("the certificate's EdDSA public key is a point of small order, which no key pair has")]
    SmallOrderKey,
}

/// Verifies an `EdDSA` signature over the bytes it was made on, on the curve the key states.
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
    match key.curve {
        Curve::Ed25519 => verify_ed25519(key.key, signature, message),
        // RFC 8419 section 1 excludes HashEdDSA from CMS and uses no context string, so the one
        // Ed448 a document can carry is PureEdDSA's.
        Curve::Ed448 => ed448::verify(key.key, signature, message),
    }
}

/// The Ed25519 arm, over `ed25519-dalek`'s specification-conforming `multipart_verify`.
fn verify_ed25519(key: &[u8], signature: &[u8], message: &[&[u8]]) -> Result<bool, EdDsaError> {
    let key: &[u8; ED25519_KEY_OCTETS] = key.try_into().map_err(|_| EdDsaError::MalformedKey)?;
    let key = VerifyingKey::from_bytes(key).map_err(|_| EdDsaError::MalformedKey)?;
    if key.is_weak() {
        return Err(EdDsaError::SmallOrderKey);
    }
    let signature: &[u8; ED25519_SIGNATURE_OCTETS] = signature
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
/// openssl genpkey -algorithm ed25519 -out key.pem                # and -algorithm ED448
/// openssl req -x509 -key key.pem -days 3650 -subj /CN=quorra -outform der -out cert.der
/// printf 'the signed bytes' | openssl pkeyutl -sign -inkey key.pem -rawin -out sig.bin
/// ```
///
/// The Ed448 pair is a second implementation's output checked by this tree's own arithmetic;
/// what fixes that arithmetic is RFC 8032 section 7.4's vectors, in `ed448.rs`.
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

    /// A self-signed Ed448 certificate, its own signature Ed448 too.
    pub(crate) const ED448_CERTIFICATE: &str = "\
        308201a030820120a00302010202144b99d2b6ca489f6be3db3dc7555f212c08\
        b3e7ed300506032b65713020311e301c06035504030c157064662d7669657765\
        722065643434382074657374301e170d3236303932383232303032315a170d33\
        36303932353232303032315a3020311e301c06035504030c157064662d766965\
        77657220656434343820746573743043300506032b6571033a008497a6a716b7\
        c7d8bdeb1091ec6e7578598ebf9cd8ba5cf363c2ca212b6304024f223a3091ae\
        bd8c3bab77ddfff11c64a28e0c8324480e9e00a3533051301d0603551d0e0416\
        0414f9f056b240f995f8c0501e1df3e5852f73c9226b301f0603551d23041830\
        168014f9f056b240f995f8c0501e1df3e5852f73c9226b300f0603551d130101\
        ff040530030101ff300506032b6571037300e3c7b7cb8e36db066261be383012\
        1c624f800238e683844535e0441eef3489418245bca24ec6e645f3a0de8d0e96\
        5229f7437e012edb60840006efe2af7d3dbbcc6895e3ac883fea5b4c5ae379f9\
        47ed609a9de5ce953a8afe9a7b9e1ea46672f27e813eb08d679c4a3c65a409be\
        33173600";

    /// `R ‖ S` over `b"the signed bytes"` under the Ed448 key above: PureEdDSA, empty context.
    pub(crate) const ED448_SIGNATURE: &str = "\
        670d82dfb3cd7017ca12d5ffe54f9374022c8c2d894a8eecc5b5b80d58be76f6\
        32b68bc2cd6e90898a1578d7c85274e240f1a22a34c9ef2500ff9e3aeb15d330\
        3aa6cbcc5a0bb23733f373911901e94b084986962c6a455d35781bd7344242f4\
        4ca4a6c5e1370e0ffe0d89a09332a7ca3900";
}

#[cfg(test)]
mod tests {
    use super::fixtures::{
        ED448_CERTIFICATE, ED448_SIGNATURE, ED25519_CERTIFICATE, ED25519_SIGNATURE,
    };
    use super::{Curve, EdDsaError, verify};
    use crate::ecdsa::fixtures::{MESSAGE, hex};
    use crate::x509::{self, PublicKey};

    /// The key out of the certificate, for a fixture that carries an Edwards one.
    fn key_of(certificate: &[u8]) -> super::PublicKey<'_> {
        let certificate = x509::parse(certificate).expect("a certificate");
        match certificate.public_key {
            PublicKey::EdDsa(key) => key,
            other => panic!("this certificate states an EdDSA key, not {other:?}"),
        }
    }

    /// Each curve's fixture: the certificate, the signature over [`MESSAGE`], and the curve.
    fn each_curve() -> [(Vec<u8>, Vec<u8>, Curve); 2] {
        [
            (
                hex(ED25519_CERTIFICATE),
                hex(ED25519_SIGNATURE),
                Curve::Ed25519,
            ),
            (hex(ED448_CERTIFICATE), hex(ED448_SIGNATURE), Curve::Ed448),
        ]
    }

    /// The whole path, on each curve: a signature made with a key verifies under the
    /// certificate's, and stops when one bit of the message moves.
    #[test]
    fn a_signature_verifies_under_its_own_certificates_key() {
        for (certificate, signature, curve) in each_curve() {
            let key = key_of(&certificate);
            assert_eq!(key.curve, curve);
            assert_eq!(key.key.len() * 8, curve.bits(), "{}", curve.name());
            assert_eq!(
                verify(key, &signature, &[MESSAGE]),
                Ok(true),
                "{}",
                curve.name()
            );
            let mut moved = MESSAGE.to_vec();
            moved[0] ^= 0x01;
            assert_eq!(
                verify(key, &signature, &[moved.as_slice()]),
                Ok(false),
                "{}",
                curve.name()
            );
        }
    }

    /// The message is verified in parts, and the parts are the message.
    ///
    /// The one property that could go wrong silently in the multipart form: a verifier that hashed
    /// the parts with anything between them, or in the wrong order, would still answer `false` for
    /// a wrong message and `true` for nothing at all. So the same bytes split differently must
    /// verify, and the same bytes in the other order must not.
    #[test]
    fn the_parts_are_hashed_as_one_message_in_the_order_given() {
        for (certificate, signature, curve) in each_curve() {
            let key = key_of(&certificate);
            let (head, tail) = MESSAGE.split_at(4);
            assert_eq!(
                verify(key, &signature, &[head, tail]),
                Ok(true),
                "{}",
                curve.name()
            );
            assert_eq!(
                verify(key, &signature, &[tail, head]),
                Ok(false),
                "{}",
                curve.name()
            );
        }
    }

    /// A signature or a key that is not the width the curve fixes, named rather than padded.
    #[test]
    fn a_value_of_the_wrong_width_is_refused_by_name() {
        for (certificate, signature, curve) in each_curve() {
            let key = key_of(&certificate);
            assert_eq!(
                verify(key, &signature[..signature.len() - 1], &[MESSAGE]),
                Err(EdDsaError::MalformedSignature),
                "{}",
                curve.name()
            );
            let short = super::PublicKey {
                curve,
                key: &key.key[..key.key.len() - 1],
            };
            assert_eq!(
                verify(short, &signature, &[MESSAGE]),
                Err(EdDsaError::MalformedKey),
                "{}",
                curve.name()
            );
        }
    }

    /// One bit of the signature moved is a signature that does not verify, not one that panics.
    #[test]
    fn a_signature_one_bit_away_does_not_verify() {
        for (certificate, signature, curve) in each_curve() {
            let key = key_of(&certificate);
            let half = signature.len() / 2;
            for index in [0, half - 1, half, signature.len() - 1] {
                let mut moved = signature.clone();
                moved[index] ^= 0x01;
                assert_eq!(
                    verify(key, &moved, &[MESSAGE]),
                    Ok(false),
                    "{} octet {index}",
                    curve.name()
                );
            }
        }
    }

    /// A key of small order is refused by name on either curve, never verified: under the
    /// neutral point, `R` the neutral point and `S = 0` satisfy the equation for every message.
    #[test]
    fn a_key_of_small_order_is_refused_on_either_curve() {
        for (curve, width) in [(Curve::Ed25519, 32), (Curve::Ed448, 57)] {
            // `y = 1`, sign bit clear: the neutral point's encoding on both curves.
            let mut neutral = vec![0u8; width];
            neutral[0] = 1;
            let signature = [neutral.clone(), vec![0u8; width]].concat();
            let key = super::PublicKey {
                curve,
                key: &neutral,
            };
            assert_eq!(
                verify(key, &signature, &[b"a message nobody signed"]),
                Err(EdDsaError::SmallOrderKey),
                "{}",
                curve.name()
            );
        }
    }

    /// The `x509` fuzz target's crasher, byte for byte: the Ed448 fixture certificate with its key
    /// mutated to 57 zero octets, `(±1, 0)`, of order 4. Under it the target's all-zero signature
    /// satisfied the cofactored equation over a message nobody signed. `CLAUDE.md` principle 3
    /// makes a crasher a permanent test, and the bytes are here because `fuzz/artifacts` is not
    /// carried; libFuzzer named it `crash-60910642d68fa22e963bf5af9bebafdc2504926d`.
    #[test]
    fn the_fuzzers_order_four_ed448_key_is_refused() {
        const CRASHER: &str = "\
            308201a030820120a00302010202144b99d2b6ca489f6be3db3dc7555f212c08b3e7ed300506032b\
            65713020311e301c06035504030c157064662d76696577656b2065643434382074657374301e170d\
            3236303932383232303032315a170d3336303932353232303032315a3020311e301c06035504030c\
            157064662d76696577657220656434343820746573743043300506032b6571033a00000000000000\
            00000000000000000000000000000000000000000000000000000000000000000000000000000000\
            00000000000000000000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000008497a6a716b7c7d8bdeb\
            1091ec6e75788e9c59d8babf5c308035800102000000000000302300000000000000fff11c64a28e\
            0c8324480e9e00a3533051301d0603551d0e04160414f9f056b240f995f8c0501e1df3e5852f73c9\
            226b301f0603551d23041830168014f9f056b240f995f840501e1df3e5852f73c9226b300f060355\
            1d130101ff040530030101ff300506032b6571037300e3c7b7cb8e36db066261be3830121c624f80\
            0238e683844535e0441eef3489418245bca24ec6e645f3a0de8d0e965229f7437e012edb60840006\
            efe2af7d3dbbcc6895e3ac883fea5b4c5ae379f947ed609a9de5ce953a8afe9a7b9e1ea46672f27e\
            813eb08d679c4a3c65a409be33173600";
        let crasher = hex(CRASHER);
        assert_eq!(crasher.len(), 536, "the case the fuzz run found");
        let key = key_of(&crasher);
        assert_eq!(key.curve, Curve::Ed448);
        assert_eq!(
            verify(key, &[0u8; 114], &[b"a message nobody signed"]),
            Err(EdDsaError::SmallOrderKey)
        );
    }

    /// A key read as the other curve's is refused rather than verified: the curve is the key's,
    /// and the widths alone keep the two apart.
    #[test]
    fn a_key_is_verified_only_on_its_own_curve() {
        let [(ed25519, ed25519_signature, _), (ed448, ed448_signature, _)] = each_curve();
        let mut crossed = key_of(&ed448);
        crossed.curve = Curve::Ed25519;
        assert_eq!(
            verify(crossed, &ed448_signature, &[MESSAGE]),
            Err(EdDsaError::MalformedKey)
        );
        let mut crossed = key_of(&ed25519);
        crossed.curve = Curve::Ed448;
        assert_eq!(
            verify(crossed, &ed25519_signature, &[MESSAGE]),
            Err(EdDsaError::MalformedKey)
        );
    }

    /// The Ed448 certificate's own signature — `tbsCertificate` signed with `id-Ed448` — verifies
    /// through the path a certification path takes (RFC 5280 section 4.1.1.3).
    #[test]
    fn a_self_signed_ed448_certificate_verifies_its_own_signature() {
        let certificate = hex(ED448_CERTIFICATE);
        let parsed = x509::parse(&certificate).expect("a certificate");
        assert_eq!(
            x509::verify_signature(
                parsed.tbs,
                parsed.signature_algorithm,
                None,
                parsed.signature,
                parsed.public_key,
            ),
            Ok(true)
        );
    }

    /// Both identifiers read back as the digits a person sees, and each names its own curve.
    #[test]
    fn both_curves_are_numbers_this_program_can_print() {
        for curve in [Curve::Ed25519, Curve::Ed448] {
            let oid = curve.oid();
            assert_eq!(
                x509::dotted(oid.as_bytes()).as_deref(),
                Some(oid.to_string().as_str())
            );
            assert_eq!(Curve::of(oid.as_bytes()), Some(curve));
        }
        assert_ne!(super::ID_ED448, super::ID_ED25519);
    }
}
