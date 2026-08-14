//! RSASSA-PSS verification: the *other* padding of Table 260's RSA family.
//!
//! ISO 32000-2's Table 260 names the family by key size and no padding at all — "RSA Algorithm
//! Support | Up to 1024-bit (PDF 1.3) Up to 2048-bit (PDF 1.5) Up to 4096-bit (PDF 1.5)" — and
//! RFC 8017 defines two signature schemes over that one key type: RSASSA-PKCS1-v1_5, which is
//! [`crate::pkcs1`], and RSASSA-PSS, which is this module. Six of the 811 signatures in the
//! `SafeDocs` population state `id-RSASSA-PSS` — twice ECDSA's share, and until the
//! four-hundred-and-eighty-seventh session the commonest thing this program declined (ADR 0322;
//! the census is `examples/signature_algorithm_census.rs` and is meant to be re-run, not quoted).
//!
//! # Two schemes, two modules, deliberately
//!
//! `id-RSASSA-PSS` (`1.2.840.113549.1.1.10`) sits in the same `pkcs-1` arc as `rsaEncryption`
//! and states a different padding, so a reader that matched the arc would verify the wrong
//! construction. The two stay separate: [`crate::pkcs1`] is RFC 8017 sections 8.2 and 9.2,
//! this module is its sections 8.1 and 9.1 with Appendix B.2.1's MGF1, and the one thing they
//! share is the thing the RFC itself shares between them — section 5.2.2's `RSAVP1` primitive,
//! `m = s^e mod n`, which section 8.1.2 step 2.b and section 8.2.2 step 2.b both invoke by name
//! ([`crate::pkcs1::rsavp1`]).
//!
//! # What makes PSS different to verify
//!
//! RSASSA-PKCS1-v1_5 is deterministic, so [`crate::pkcs1`] re-encodes the expected block and
//! compares whole octet strings. PSS is salted: the signer folded a random salt into the encoded
//! message, so there is no block to predict and the verifier must instead run RFC 8017 section
//! 9.1.2's `EMSA-PSS-VERIFY` — unmask the salt with the mask generation function and check that
//! `H = Hash(padding || mHash || salt)` comes out equal. The construction's own checks are what
//! keep a forgery out: the trailer octet (step 4), the zero padding and the `0x01` separator
//! (step 10), and the final hash comparison (step 14). A mistake here is still safe in the closed
//! direction — a wrong mask, salt length or hash produces "inconsistent", never a false
//! "consistent" — because the last step compares a hash this module computed itself.
//!
//! # Where the parameters come from
//!
//! Everything the scheme is parameterised by arrives in the file, inside the `signatureAlgorithm`
//! `AlgorithmIdentifier`. RFC 8017 Appendix A.2.3: "[t]he parameters field associated with this
//! OID in a value of type `AlgorithmIdentifier` SHALL have a value of type `RSASSA-PSS-params`":
//!
//! ```text
//! RSASSA-PSS-params ::= SEQUENCE {
//!     hashAlgorithm      [0] HashAlgorithm      DEFAULT sha1,
//!     maskGenAlgorithm   [1] MaskGenAlgorithm   DEFAULT mgf1SHA1,
//!     saltLength         [2] INTEGER            DEFAULT 20,
//!     trailerField       [3] TrailerField       DEFAULT trailerFieldBC
//! }
//! ```
//!
//! [`parameters`] reads that over [`crate::der`]. The hashes this program accepts here are the
//! ones Appendix A.2.1's `OAEP-PSSDigestAlgorithms` set names that [`Digest`] already computes —
//! SHA-1, SHA-256, SHA-384 and SHA-512 — and every refusal carries the identifier the file
//! stated, never a silence: a hash outside that set, a mask generation function other than
//! Appendix B.2.1's MGF1 (the whole of `PKCS1MGFAlgorithms` "for this version"), or a trailer
//! field other than the 1 that Appendix A.2.3 says "SHALL be 1 for this version of the document,
//! which represents the trailer field with hexadecimal value 0xbc". ADR 0322 has the decision.
//!
//! There is no arithmetic of this module's own: the modular exponentiation is
//! [`crate::bigint`]'s — `crypto-bigint` behind this crate's budgets since ADR 0331 — and ADR
//! 0229's argument still holds: a signature verification has no secret in it, so nothing here
//! runs in constant time and nothing needs to.

use crate::cms::Digest;
use crate::der::{INTEGER, SEQUENCE, Value};
use crate::pkcs1::{Pkcs1Error, PublicKey, rsavp1};

/// RFC 8017 Appendix A.2.3's `id-RSASSA-PSS`, `1.2.840.113549.1.1.10` — "{ pkcs-1 10 }".
pub const ID_RSASSA_PSS: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0A];

/// RFC 8017 Appendix B.2.1's `id-mgf1`, `1.2.840.113549.1.1.8` — "{ pkcs-1 8 }".
pub const ID_MGF1: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x08];

/// The `RSASSA-PSS-params` a signature states, as far as this program acts on them.
///
/// The trailer field is not here because it admits one value: [`parameters`] refuses anything but
/// RFC 8017 Appendix A.2.3's 1, so a `Parameters` that exists is one whose trailer octet is
/// `0xbc` and [`verify`] checks for exactly that octet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parameters {
    /// `hashAlgorithm` — what digests the message *and* the padded block.
    ///
    /// RFC 8017 section 9.1.2 uses one `Hash` for both `mHash` (step 2) and `H'` (step 13), so
    /// the caller computes the message digest with this and [`verify`] computes the block's with
    /// it too.
    pub hash: Digest,
    /// The hash MGF1 is based on — `maskGenAlgorithm`'s own parameter.
    ///
    /// Appendix A.2.3 recommends it "be the same as the one identified by hashAlgorithm" and does
    /// not require it, so it is carried separately rather than assumed equal.
    pub mgf_hash: Digest,
    /// `saltLength`, "the octet length of the salt".
    ///
    /// Not bounded here: RFC 8017 section 9.1.2 step 3 makes an over-long salt "inconsistent" —
    /// a verification failure, not a refusal — and every use of it in [`verify`] is a saturating
    /// comparison against the encoded message's own length, so a preposterous value costs
    /// nothing before it fails.
    pub salt_length: usize,
}

/// What stopped `RSASSA-PSS-params` from yielding [`Parameters`] this program can verify under.
///
/// Every variant that involves an algorithm carries the object identifier the file wrote, so the
/// refusal reaches a person as a number they can check rather than as a shrug —
/// [`crate::signature::Authenticity`] is the channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterProblem<'a> {
    /// The parameters are absent, or are not RFC 8017 Appendix A.2.3's `RSASSA-PSS-params`.
    ///
    /// Absence is included deliberately: that appendix says the parameters field "SHALL have a
    /// value of type RSASSA-PSS-params", so a `SignerInfo` stating `id-RSASSA-PSS` with no
    /// parameters at all has not said which hash, mask or salt length it means, and defaulting
    /// on its behalf would be this program guessing three algorithms at once.
    Malformed,
    /// `hashAlgorithm` (or MGF1's underlying hash) names a digest this program does not compute.
    HashNotComputed(&'a [u8]),
    /// `hashAlgorithm` (or MGF1's underlying hash) is computed by this program but is outside
    /// RFC 8017 Appendix A.2.1's `OAEP-PSSDigestAlgorithms` set — MD5 or RIPEMD-160, which
    /// [`Digest`] has for Table 256's sake and the PSS scheme does not admit.
    HashNotAdmitted(&'a [u8]),
    /// `maskGenAlgorithm` is not `id-mgf1`, which is the whole of `PKCS1MGFAlgorithms`:
    /// Appendix A.2.1 says the set "for this version SHALL consist of id-mgf1".
    MaskGenerationNotMgf1(&'a [u8]),
    /// `trailerField` is not 1 — Appendix A.2.3: "[i]t SHALL be 1 for this version of the
    /// document".
    TrailerFieldNotOne,
}

/// Reads RFC 8017 Appendix A.2.3's `RSASSA-PSS-params` out of an `AlgorithmIdentifier`'s
/// parameters value.
///
/// The four members are `[0]` to `[3]`, each explicit (the RFC's ASN.1 module is `DEFINITIONS
/// EXPLICIT TAGS`), each optional with the appendix's defaults: SHA-1, MGF1 with SHA-1, a salt
/// of 20 octets and trailer field 1.
///
/// # Errors
///
/// A [`ParameterProblem`] naming what could not be acted on; nothing is guessed at.
pub fn parameters(value: Option<Value<'_>>) -> Result<Parameters, ParameterProblem<'_>> {
    let Some(value) = value else {
        return Err(ParameterProblem::Malformed);
    };
    if value.identifier != SEQUENCE {
        return Err(ParameterProblem::Malformed);
    }
    let mut hash = None;
    let mut mgf_hash = None;
    let mut salt_length: usize = 20;
    let mut members = value.children().map_err(|_| ParameterProblem::Malformed)?;
    while let Some(member) = members
        .next_value()
        .map_err(|_| ParameterProblem::Malformed)?
    {
        if member.is_context(0) {
            hash = Some(pss_digest(algorithm_identifier(member)?)?);
        } else if member.is_context(1) {
            // MaskGenAlgorithm ::= AlgorithmIdentifier { OID id-mgf1, parameters HashAlgorithm }.
            let mgf = explicit_child(member)?;
            let mut parts = mgf.children().map_err(|_| ParameterProblem::Malformed)?;
            let oid = parts
                .next_value()
                .map_err(|_| ParameterProblem::Malformed)?
                .and_then(|value| value.object_identifier())
                .ok_or(ParameterProblem::Malformed)?;
            if oid != ID_MGF1 {
                return Err(ParameterProblem::MaskGenerationNotMgf1(oid));
            }
            let inner = parts
                .next_value()
                .map_err(|_| ParameterProblem::Malformed)?
                .ok_or(ParameterProblem::Malformed)?;
            mgf_hash = Some(pss_digest(digest_identifier(inner)?)?);
        } else if member.is_context(2) {
            salt_length = unsigned_integer(explicit_child(member)?)?;
        } else if member.is_context(3) {
            // TrailerField ::= INTEGER { trailerFieldBC(1) }, and 1 is the only value the RFC
            // gives it: "[o]ther trailer fields (including the trailer field HashID || 0xcc in
            // IEEE 1363a) are not supported in this document" (Appendix A.2.3).
            if unsigned_integer(explicit_child(member)?)? != 1 {
                return Err(ParameterProblem::TrailerFieldNotOne);
            }
        }
        // An unrecognised tag is skipped rather than refused: X.680's extensibility is how the
        // RFC's own "future expansion" markers arrive, and every field this scheme needs has
        // been read or defaulted.
    }
    Ok(Parameters {
        hash: hash.unwrap_or(Digest::Sha1),
        mgf_hash: mgf_hash.unwrap_or(Digest::Sha1),
        salt_length,
    })
}

/// The one value inside an explicit context tag.
fn explicit_child(member: Value<'_>) -> Result<Value<'_>, ParameterProblem<'static>> {
    member
        .children()
        .ok()
        .and_then(|mut children| children.next_value().ok().flatten())
        .ok_or(ParameterProblem::Malformed)
}

/// The digest identifier out of an explicit tag holding an `AlgorithmIdentifier`.
fn algorithm_identifier(member: Value<'_>) -> Result<&[u8], ParameterProblem<'_>> {
    digest_identifier(explicit_child(member)?)
}

/// The object identifier inside one `AlgorithmIdentifier ::= SEQUENCE { OID, parameters }`.
fn digest_identifier(algorithm: Value<'_>) -> Result<&[u8], ParameterProblem<'_>> {
    if algorithm.identifier != SEQUENCE {
        return Err(ParameterProblem::Malformed);
    }
    algorithm
        .children()
        .ok()
        .and_then(|mut children| children.next_value().ok().flatten())
        .and_then(|value| value.object_identifier())
        .ok_or(ParameterProblem::Malformed)
}

/// A digest identifier as one of the hashes this scheme admits and this program computes.
///
/// RFC 8017 Appendix A.2.1's `OAEP-PSSDigestAlgorithms` names SHA-1, SHA-224, SHA-256, SHA-384,
/// SHA-512, SHA-512/224 and SHA-512/256; [`Digest`] computes four of those seven, and its other
/// two members — MD5 and RIPEMD-160, which exist for Table 256's sake — are refused as *not
/// admitted* rather than as unknown, because "this program does not compute it" would be false
/// of them.
fn pss_digest(oid: &[u8]) -> Result<Digest, ParameterProblem<'_>> {
    match Digest::from_oid(oid) {
        Some(digest @ (Digest::Sha1 | Digest::Sha256 | Digest::Sha384 | Digest::Sha512)) => {
            Ok(digest)
        }
        Some(Digest::Md5 | Digest::Ripemd160) => Err(ParameterProblem::HashNotAdmitted(oid)),
        None => Err(ParameterProblem::HashNotComputed(oid)),
    }
}

/// One `INTEGER`'s value as a `usize`, refusing a negative one and saturating a huge one.
///
/// Saturation rather than refusal for width is deliberate: the only integer read here is a salt
/// length, and RFC 8017 section 9.1.2 step 3 makes an over-long salt a verification failure
/// rather than a malformed file — `usize::MAX` fails that step exactly as the stated number
/// would.
fn unsigned_integer(value: Value<'_>) -> Result<usize, ParameterProblem<'static>> {
    if value.identifier != INTEGER {
        return Err(ParameterProblem::Malformed);
    }
    if value
        .contents
        .first()
        .is_some_and(|&first| first & 0x80 != 0)
    {
        // X.690's INTEGER is two's complement, so a set top bit is a negative number, and no
        // octet length is negative.
        return Err(ParameterProblem::Malformed);
    }
    let mut out: usize = 0;
    for &octet in value.contents {
        out = out
            .checked_mul(256)
            .and_then(|shifted| shifted.checked_add(usize::from(octet)))
            .unwrap_or(usize::MAX);
    }
    Ok(out)
}

/// Whether `signature` is an RSASSA-PSS signature over `message_digest` under `key`.
///
/// `message_digest` is section 9.1.2 step 2's `mHash` — the [`Parameters::hash`] digest the
/// *caller* computed over whatever RFC 5652 says is signed, so that nothing here can disagree
/// with [`crate::cms::Digest`] about what was hashed. The steps are RFC 8017 section 8.1.2's:
/// the length check, `RSAVP1`, `I2OSP` at emLen — "emLen = \ceil ((modBits - 1)/8) octets, where
/// modBits is the length in bits of the RSA modulus n", and its own note that "emLen will be one
/// less than k if modBits - 1 is divisible by 8 and equal to k otherwise" — and then section
/// 9.1.2's `EMSA-PSS-VERIFY` over the result.
///
/// # Errors
///
/// A [`Pkcs1Error`] where the key or the signature is outside [`crate::pkcs1`]'s budgets or is
/// not shaped like an RSA value at all — the schemes share section 5.2.2's primitive and
/// therefore its refusals. A signature that is well-formed and simply does not verify is
/// `Ok(false)`, and the two are kept apart because only the second says anything about the
/// file's honesty.
pub fn verify(
    key: PublicKey<'_>,
    signature: &[u8],
    parameters: Parameters,
    message_digest: &[u8],
) -> Result<bool, Pkcs1Error> {
    let (message, modulus_bits) = rsavp1(key, signature)?;
    // Section 8.1.2 step 2.c. `modulus_bits` is at least 2 — `Modulus::new` refused anything
    // smaller inside `rsavp1` — so `em_bits` is at least 1 and `em_len` at least 1.
    let em_bits = modulus_bits.saturating_sub(1);
    let em_len = em_bits.div_ceil(8);
    if message.bits() > em_len.saturating_mul(8) {
        // "If I2OSP outputs "integer too large", output "invalid signature" and stop."
        return Ok(false);
    }
    let encoded = message.be_bytes(em_len);
    Ok(emsa_pss_verify(
        parameters,
        message_digest,
        &encoded,
        em_bits,
    ))
}

/// RFC 8017 section 9.1.2's `EMSA-PSS-VERIFY (M, EM, emBits)`, steps 3 to 14.
///
/// Steps 1 and 2 are the caller's: `mHash` arrives already computed, which skips step 1's input
/// limitation (no octet string this program can hold approaches `2^61`) and *is* step 2. Every
/// "output "inconsistent" and stop" is `false` here — a verification failure, never a panic and
/// never a report — and the arithmetic mirrors the RFC's own names so it can be checked line by
/// line: `emLen`, `maskedDB`, `H`, `dbMask`, `DB`, `salt`, `M'`, `H'`.
fn emsa_pss_verify(
    parameters: Parameters,
    message_digest: &[u8],
    encoded: &[u8],
    em_bits: usize,
) -> bool {
    let hash_length = message_digest.len();
    let em_len = encoded.len();
    let salt_length = parameters.salt_length;
    // Step 3: "If emLen < hLen + sLen + 2, output "inconsistent" and stop." Saturating, so a
    // salt length near `usize::MAX` fails here rather than wrapping.
    if em_len < hash_length.saturating_add(salt_length).saturating_add(2) {
        return false;
    }
    // Step 4: "If the rightmost octet of EM does not have hexadecimal value 0xbc ...".
    if encoded.last() != Some(&0xBC) {
        return false;
    }
    // Step 5: "Let maskedDB be the leftmost emLen - hLen - 1 octets of EM, and let H be the next
    // hLen octets." Step 3 established emLen ≥ hLen + 2, so both subtractions are covered.
    let db_len = em_len.saturating_sub(hash_length).saturating_sub(1);
    let Some(masked_db) = encoded.get(..db_len) else {
        return false;
    };
    let Some(h) = encoded.get(db_len..em_len.saturating_sub(1)) else {
        return false;
    };
    // Step 6: "If the leftmost 8emLen - emBits bits of the leftmost octet in maskedDB are not
    // all equal to zero ...". The difference is 0..=7 by em_len's construction.
    let unused_bits = em_len.saturating_mul(8).saturating_sub(em_bits).min(8);
    let unused = u32::try_from(unused_bits).unwrap_or(8);
    if unused > 0
        && masked_db
            .first()
            .is_some_and(|&first| first.checked_shr(8u32.saturating_sub(unused)).unwrap_or(0) != 0)
    {
        return false;
    }
    // Step 7: "Let dbMask = MGF(H, emLen - hLen - 1)." Step 8: "Let DB = maskedDB \xor dbMask."
    let mask = mgf1(parameters.mgf_hash, h, db_len);
    let mut db: Vec<u8> = masked_db
        .iter()
        .zip(&mask)
        .map(|(&masked, &mask)| masked ^ mask)
        .collect();
    // Step 9: "Set the leftmost 8emLen - emBits bits of the leftmost octet in DB to zero."
    if let Some(first) = db.first_mut() {
        *first &= 0xFFu8.checked_shr(unused).unwrap_or(0);
    }
    // Step 10: "If the emLen - hLen - sLen - 2 leftmost octets of DB are not zero or if the
    // octet at position emLen - hLen - sLen - 1 (the leftmost position is "position 1") does not
    // have hexadecimal value 0x01 ...".
    let padding = db_len.saturating_sub(salt_length).saturating_sub(1);
    let zeros_hold = db
        .get(..padding)
        .is_some_and(|zeros| zeros.iter().all(|&octet| octet == 0));
    if !zeros_hold || db.get(padding) != Some(&0x01) {
        return false;
    }
    // Step 11: "Let salt be the last sLen octets of DB."
    let Some(salt) = db.get(db.len().saturating_sub(salt_length)..) else {
        return false;
    };
    // Steps 12 and 13: "M' = (0x)00 00 00 00 00 00 00 00 || mHash || salt", "Let H' = Hash(M')".
    let h_prime = parameters.hash.compute(&[&[0u8; 8], message_digest, salt]);
    // Step 14: "If H = H', output "consistent"." There is no secret to protect, so this is an
    // ordinary comparison (ADR 0229's argument, module documentation).
    h == h_prime
}

/// RFC 8017 Appendix B.2.1's MGF1: `T = T || Hash(mgfSeed || C)` for a four-octet counter `C`
/// from zero, "[o]utput the leading maskLen octets of T".
///
/// Step 1's `maskLen > 2^32 hLen` refusal is unreachable: the one caller asks for at most an
/// encoded message's width, and [`crate::bigint::MAX_BITS`] caps that at a kilobyte.
fn mgf1(hash: Digest, seed: &[u8], mask_length: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(mask_length.saturating_add(64));
    let mut counter: u32 = 0;
    while out.len() < mask_length {
        out.extend_from_slice(&hash.compute(&[seed, &counter.to_be_bytes()]));
        counter = counter.saturating_add(1);
    }
    out.truncate(mask_length);
    out
}

#[cfg(test)]
pub(crate) mod fixtures {
    //! An RSA key and PSS signatures made with it, built once with `openssl` and pasted in.
    //!
    //! **Test vectors rather than oracles**, on `dsa::fixtures`' footing: RFC 8017 defines the
    //! scheme and what a vector pins is that *these* lines compute it — a question a hand-checked
    //! small case cannot settle at 2048 bits, because the salt makes every signature distinct.
    //! The corpus cannot stand in for a *positive* case either: its six PSS signatures are real
    //! ones whose private keys nobody here holds, so a signature over bytes this tree chose needs
    //! a key this tree made. (`openssl genpkey`, `req -x509`, `dgst -sigopt
    //! rsa_padding_mode:pss`, once; the census example measures the six real ones.)

    /// A 2048-bit RSA modulus; the public exponent is 65537.
    pub(crate) const MODULUS: &str = "\
        ed76553d95ecefd520b15824746f298fe2b9bfe48a3fb0bf57e7f3a932425af5\
        ffc38810b6eea0225b20693f19486d512208445af9bba4d47f904bd505ccd39e\
        98230c9998abe7c3bb0522d6b772a7a364e66fa3799736194ba68ecf429ff9fc\
        8c7c6bb12d0a595c8719c04348f41d379c6263742afed866a06d73760761070f\
        6a9eec2ad67852671763264a171485ff9e3c2363304ee585199452ef263f8a65\
        d17b5f9e8f7acc1f13c38e354979b1a4e9ad48c9a695c78105a469851fee6f55\
        8f79e046cbe7ad00ecd127d20ca4393e1c62b995a651db91897286d0c5cff400\
        2e7c7a744e7ef8e291dfcd7aeee77ecb5c6123d01034dcff0ab9478c3f238235";

    /// PSS over `SHA-256(b"the signed bytes")`: SHA-256, MGF1 with SHA-256, a 32-octet salt.
    pub(crate) const SIGNATURE_SHA256_SALT32: &str = "\
        e6d8ae07db1e28bdec0ce0b23f6d9e1b5d963fd545136fd29d0c9a60996ad172\
        5b5ddee801ff1d725c9d844f0d6c22ea9afb5135dcb40f36201b5c87e2980492\
        546174faaf6e70ec0e758b6c94fb971381abd80f3982817f5ddb0ace5e979aff\
        d4f37b1b3e0f2ed51453bee1c65ea8c9a199cdbdfad9519b17069943eb1138d2\
        bf70a2242a108d001b3d48fafe5f21648b1e3b6fe111374500b17758a373b4d6\
        96901baee879d06a1d1cc17a88860a34051a3bcb7ed61c84da644f88162ef73f\
        ec0b98833c6b7cc4af26c63dddba33ad041986e6471692691fa3c74f4001d573\
        5dde979df581afe411fcafef2bb2fbeeea5f02f70cde66d1a78d01d885edf65f";

    /// The same message under RFC 8017 Appendix A.2.3's defaults: SHA-1, MGF1 with SHA-1, 20.
    pub(crate) const SIGNATURE_SHA1_SALT20: &str = "\
        b3da72d0adae0ae3b64a127728c69836d90dd0347483e5232013392880986821\
        9def90d032823300de11babffa68a95b68cfc26eb36c1fea95d856a339900e3c\
        9f634bc94720a2a080c8d56b3926c56d50020cc83086c2cdee7bbe6542639aea\
        10aa22ea72d23d06cd6f4e1cd4d21f9ce1957929c744c07ee3b46f28e11e1fc8\
        f4902bd0c0f8f2022933eb7481469420cf8743fb83283d697e9a3c7de2588024\
        232c6a0e576a9b8722dd8bfb7ae61474d7b7f3ab6d8a95df9dfe7fa247b78837\
        e162a20478d6606acb146a5cc59d11dc2ad93b9545f35e826141546c8e2c7a71\
        97badac939b95d3ce7e1474cdf610e8f5c84112a097654fb338230e2939ded6e";

    /// And under SHA-512 throughout, with a 64-octet salt — the widest digest Table 260 names.
    pub(crate) const SIGNATURE_SHA512_SALT64: &str = "\
        a540dbe303421243d8336af71ac461f48b0055babfac4d802b5514496da4a370\
        94fcc846e6017e50eabb3470f1f083216e0705f0443d244cfae9b9bf4d601906\
        e3e1aacfb5dbb5824dbf1018a3363cf01417169970e515f2a59780702a622e53\
        d316d19c6bbe51ee9938faaeb7f0a4693d7fdb48e24a26c0897c20fe0dbc50e6\
        7fbed4f02dacabac14dbb59796f8d1444a25cfd8e9a12e6320069169f0201302\
        cd80b13afcc2d088dc9fd110b853dd27eddfe43ace6e6890b7fa0a494a76cd8f\
        08a32c26d6d3c742a893e10be35af08bcc725560d27d5e5f06c29f34f7ae3077\
        3004fa0d2e9adc0e26ed638460c21794229fb32bbe04adea44673682966fef75";

    /// A self-signed certificate over the key above, `sha256WithRSAEncryption` outside and
    /// `rsaEncryption` in its `subjectPublicKeyInfo` — which is what every one of the six real
    /// PSS signers' certificates states, measured by the census before this module was written.
    pub(crate) const CERTIFICATE: &str = "\
        3082031d30820205a0030201020214099194e1c90421460669d6c14d64e63103\
        c90a92300d06092a864886f70d01010b0500301e311c301a06035504030c1370\
        64662d766965776572207073732074657374301e170d32363038313332323537\
        32315a170d3336303831303232353732315a301e311c301a06035504030c1370\
        64662d76696577657220707373207465737430820122300d06092a864886f70d\
        01010105000382010f003082010a0282010100ed76553d95ecefd520b1582474\
        6f298fe2b9bfe48a3fb0bf57e7f3a932425af5ffc38810b6eea0225b20693f19\
        486d512208445af9bba4d47f904bd505ccd39e98230c9998abe7c3bb0522d6b7\
        72a7a364e66fa3799736194ba68ecf429ff9fc8c7c6bb12d0a595c8719c04348\
        f41d379c6263742afed866a06d73760761070f6a9eec2ad67852671763264a17\
        1485ff9e3c2363304ee585199452ef263f8a65d17b5f9e8f7acc1f13c38e3549\
        79b1a4e9ad48c9a695c78105a469851fee6f558f79e046cbe7ad00ecd127d20c\
        a4393e1c62b995a651db91897286d0c5cff4002e7c7a744e7ef8e291dfcd7aee\
        e77ecb5c6123d01034dcff0ab9478c3f2382350203010001a3533051301d0603\
        551d0e04160414d98a648d62fe2ba9919b8578eefb1e5bfafe1e8f301f060355\
        1d23041830168014d98a648d62fe2ba9919b8578eefb1e5bfafe1e8f300f0603\
        551d130101ff040530030101ff300d06092a864886f70d01010b050003820101\
        00c1108c58da8b6952739c5360ef4ed75d426cd85669f62ff813ca2a8b0fcd47\
        6575685c7dbaed6a8516f0a5dde05ddef02b57e810d59ad8ec4833facfd10cf5\
        7966b9d792f52f794f60895d1ec5ac0818e50f3b850aad2beb076d706d7fc099\
        ede64d872b69234a6a44b6add629ec2a0395cf811ba954701f8b47b89e3a6f8a\
        68b60b0243f2f5c03af85b33b0c1405d9d7b137d3f82ed864b71711f20a17a40\
        fa975df78f659bf137a425b18de95696a93dcb62a259370a6f8eeb91ea5ab845\
        c4b38d8e92a032d42ad5f9eb7f3330853e500c59a12ae0cbddc1ce3539da7903\
        e2111c43bec20a2479bb1e0cc913b2c769737552230309c50fe2636aec2fea90\
        da";

    /// Hexadecimal to bytes, for the constants above.
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
        MODULUS, SIGNATURE_SHA1_SALT20, SIGNATURE_SHA256_SALT32, SIGNATURE_SHA512_SALT64, hex,
    };
    use super::{
        ID_MGF1, ID_RSASSA_PSS, ParameterProblem, Parameters, emsa_pss_verify, mgf1, parameters,
        verify,
    };
    use crate::cms::Digest;
    use crate::der::Reader;
    use crate::pkcs1::PublicKey;

    /// The fixture key.
    fn key(modulus: &[u8]) -> PublicKey<'_> {
        PublicKey {
            modulus,
            exponent: &[0x01, 0x00, 0x01],
        }
    }

    /// The three openssl vectors, each under its own parameters — and under nobody else's.
    ///
    /// A wrong salt length, a wrong mask hash and a wrong message must all come out
    /// "inconsistent": each is one of RFC 8017 section 9.1.2's own failure exits, and a verifier
    /// missing any one of them would accept things nobody signed.
    #[test]
    fn a_pss_signature_verifies_under_its_stated_parameters_and_no_others() {
        let modulus = hex(MODULUS);
        let key = key(&modulus);
        let digest = Digest::Sha256.compute(&[b"the signed bytes"]);
        let stated = Parameters {
            hash: Digest::Sha256,
            mgf_hash: Digest::Sha256,
            salt_length: 32,
        };
        let signature = hex(SIGNATURE_SHA256_SALT32);
        assert_eq!(verify(key, &signature, stated, &digest), Ok(true));

        // One bit of the message, and it no longer verifies (step 14's hash comparison).
        let other = Digest::Sha256.compute(&[b"the signed byteS"]);
        assert_eq!(verify(key, &signature, stated, &other), Ok(false));

        // One bit of the signature, likewise.
        let mut tampered = signature.clone();
        if let Some(last) = tampered.last_mut() {
            *last ^= 1;
        }
        assert_eq!(verify(key, &tampered, stated, &digest), Ok(false));

        // A salt length other than the signer's fails step 10's separator check.
        for salt_length in [0, 20, 31, 33, usize::MAX] {
            assert_eq!(
                verify(
                    key,
                    &signature,
                    Parameters {
                        salt_length,
                        ..stated
                    },
                    &digest,
                ),
                Ok(false),
                "salt length {salt_length} is not the signer's 32"
            );
        }

        // A different MGF1 hash unmasks a different DB.
        assert_eq!(
            verify(
                key,
                &signature,
                Parameters {
                    mgf_hash: Digest::Sha512,
                    ..stated
                },
                &digest,
            ),
            Ok(false)
        );
    }

    /// Appendix A.2.3's defaults — SHA-1, MGF1 with SHA-1, a 20-octet salt — are a real profile.
    #[test]
    fn the_default_parameters_verify_a_signature_made_under_them() {
        let modulus = hex(MODULUS);
        let key = key(&modulus);
        let defaults = parameters(first_value(&[0x30, 0x00])).expect("an empty SEQUENCE defaults");
        assert_eq!(
            defaults,
            Parameters {
                hash: Digest::Sha1,
                mgf_hash: Digest::Sha1,
                salt_length: 20,
            }
        );
        let digest = Digest::Sha1.compute(&[b"the signed bytes"]);
        assert_eq!(
            verify(key, &hex(SIGNATURE_SHA1_SALT20), defaults, &digest),
            Ok(true)
        );

        // And the widest digest the tables name, under its own parameters.
        let wide = Parameters {
            hash: Digest::Sha512,
            mgf_hash: Digest::Sha512,
            salt_length: 64,
        };
        let digest = Digest::Sha512.compute(&[b"the signed bytes"]);
        assert_eq!(
            verify(key, &hex(SIGNATURE_SHA512_SALT64), wide, &digest),
            Ok(true)
        );
    }

    /// One value with the given identifier and contents, in X.690's short length form.
    fn primitive(identifier: u8, contents: &[u8]) -> Vec<u8> {
        let mut out = vec![identifier];
        assert!(contents.len() < 128, "the fixtures here are short");
        out.push(u8::try_from(contents.len()).unwrap_or(0));
        out.extend_from_slice(contents);
        out
    }

    /// A constructed value around already-encoded children.
    fn tagged(identifier: u8, children: &[Vec<u8>]) -> Vec<u8> {
        primitive(identifier, &children.concat())
    }

    /// An `AlgorithmIdentifier` for a digest, parameters omitted.
    fn algorithm(oid: &[u8]) -> Vec<u8> {
        tagged(0x30, &[primitive(0x06, oid)])
    }

    /// The first value of an encoding, for handing a fixture to [`parameters`].
    fn first_value(bytes: &[u8]) -> Option<crate::der::Value<'_>> {
        Reader::new(bytes).ok()?.next_value().ok()?
    }

    /// RFC 8017 Appendix A.2.3's structure, read member by member — and each refusal named.
    #[test]
    fn the_parameters_are_read_as_rfc_8017_defines_them() {
        // The identifiers this module acts on, pinned to their dotted forms once.
        assert_eq!(
            crate::x509::dotted(ID_RSASSA_PSS).as_deref(),
            Some("1.2.840.113549.1.1.10")
        );
        assert_eq!(
            crate::x509::dotted(ID_MGF1).as_deref(),
            Some("1.2.840.113549.1.1.8")
        );

        let sha256 = Digest::Sha256.oid();
        let stated = tagged(
            0x30,
            &[
                tagged(0xA0, &[algorithm(sha256)]),
                tagged(
                    0xA1,
                    &[tagged(0x30, &[primitive(0x06, ID_MGF1), algorithm(sha256)])],
                ),
                tagged(0xA2, &[primitive(0x02, &[32])]),
                tagged(0xA3, &[primitive(0x02, &[1])]),
            ],
        );
        assert_eq!(
            parameters(first_value(&stated)),
            Ok(Parameters {
                hash: Digest::Sha256,
                mgf_hash: Digest::Sha256,
                salt_length: 32,
            })
        );

        // Absent parameters are refused rather than defaulted: Appendix A.2.3's "SHALL have a
        // value of type RSASSA-PSS-params" is a requirement on the file.
        assert_eq!(parameters(None), Err(ParameterProblem::Malformed));
        // And so is a value of the wrong type, which is what a producer writing NULL states.
        assert_eq!(
            parameters(first_value(&[0x05, 0x00])),
            Err(ParameterProblem::Malformed)
        );

        // A trailer field other than 1 is outside the RFC and says so.
        let two = tagged(0x30, &[tagged(0xA3, &[primitive(0x02, &[2])])]);
        assert_eq!(
            parameters(first_value(&two)),
            Err(ParameterProblem::TrailerFieldNotOne)
        );

        // A mask generation function other than MGF1 is named by the identifier the file wrote.
        let not_mgf1 = tagged(
            0x30,
            &[tagged(
                0xA1,
                &[tagged(0x30, &[primitive(0x06, &[0x2A, 0x03])])],
            )],
        );
        assert_eq!(
            parameters(first_value(&not_mgf1)),
            Err(ParameterProblem::MaskGenerationNotMgf1(&[0x2A, 0x03]))
        );

        // MD5 is a digest this program computes and the PSS scheme does not admit; SHA3-256 is
        // one it does not compute at all. The two refusals are kept apart because they say two
        // different things about this program.
        let md5 = tagged(0x30, &[tagged(0xA0, &[algorithm(Digest::Md5.oid())])]);
        assert_eq!(
            parameters(first_value(&md5)),
            Err(ParameterProblem::HashNotAdmitted(Digest::Md5.oid()))
        );
        let sha3 = [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x08];
        let unknown = tagged(0x30, &[tagged(0xA0, &[algorithm(&sha3)])]);
        assert_eq!(
            parameters(first_value(&unknown)),
            Err(ParameterProblem::HashNotComputed(&sha3))
        );

        // A negative salt length is not an octet length.
        let negative = tagged(0x30, &[tagged(0xA2, &[primitive(0x02, &[0xFF])])]);
        assert_eq!(
            parameters(first_value(&negative)),
            Err(ParameterProblem::Malformed)
        );
    }

    /// RFC 8017 section 9.1.1's *encode* operation, written here so the verifier can be checked
    /// against the RFC's own construction rather than only against `openssl`'s output.
    ///
    /// Steps 5 to 12 of EMSA-PSS-ENCODE, with the salt supplied by the test: `M' = 00 x8 ||
    /// mHash || salt`, `H = Hash(M')`, `DB = PS || 0x01 || salt`, `maskedDB = DB xor MGF(H,
    /// emLen - hLen - 1)` with the spare top bits cleared, `EM = maskedDB || H || 0xbc`.
    fn emsa_pss_encode(
        hash: Digest,
        message_digest: &[u8],
        salt: &[u8],
        em_bits: usize,
    ) -> Vec<u8> {
        let h_len = message_digest.len();
        let em_len = em_bits.div_ceil(8);
        let h = hash.compute(&[&[0u8; 8], message_digest, salt]);
        let padding = em_len
            .saturating_sub(h_len)
            .saturating_sub(salt.len())
            .saturating_sub(2);
        let mut db = vec![0u8; padding];
        db.push(0x01);
        db.extend_from_slice(salt);
        let mask = mgf1(hash, &h, db.len());
        let mut masked: Vec<u8> = db.iter().zip(&mask).map(|(&a, &b)| a ^ b).collect();
        let unused = u32::try_from(em_len.saturating_mul(8).saturating_sub(em_bits)).unwrap_or(0);
        if let (Some(first), true) = (masked.first_mut(), unused > 0) {
            *first &= 0xFFu8.checked_shr(unused).unwrap_or(0);
        }
        masked.extend_from_slice(&h);
        masked.push(0xBC);
        masked
    }

    /// The verifier against the RFC's own construction, at both widths section 8.1.2 names.
    ///
    /// `emBits = 2047` is what every 2048-bit key produces — one spare bit in the leftmost
    /// octet, so steps 6 and 9 do real work — and `emBits = 2048` is the case the note in step
    /// 2.c describes, "emLen will be one less than k if modBits - 1 is divisible by 8", which
    /// `openssl` will not make a key for (it rounds a 2049-bit request down) and which this
    /// construction reaches directly.
    #[test]
    fn the_verifier_accepts_the_rfcs_own_construction_and_rejects_its_neighbours() {
        let digest = Digest::Sha256.compute(&[b"the signed bytes"]);
        let salt = [0xA5u8; 32];
        for em_bits in [2047usize, 2048] {
            let parameters = Parameters {
                hash: Digest::Sha256,
                mgf_hash: Digest::Sha256,
                salt_length: salt.len(),
            };
            let encoded = emsa_pss_encode(Digest::Sha256, &digest, &salt, em_bits);
            assert!(
                emsa_pss_verify(parameters, &digest, &encoded, em_bits),
                "the RFC's encode operation is consistent at emBits {em_bits}"
            );

            // Step 4: the trailer octet.
            let mut wrong_trailer = encoded.clone();
            if let Some(last) = wrong_trailer.last_mut() {
                *last = 0xCC;
            }
            assert!(!emsa_pss_verify(
                parameters,
                &digest,
                &wrong_trailer,
                em_bits
            ));

            // Step 6: a spare top bit that is not zero, where there is one.
            if em_bits % 8 != 0 {
                let mut wrong_top = encoded.clone();
                if let Some(first) = wrong_top.first_mut() {
                    *first |= 0x80;
                }
                assert!(!emsa_pss_verify(parameters, &digest, &wrong_top, em_bits));
            }

            // Step 3: a salt the encoded message has no room for.
            let cramped = Parameters {
                salt_length: 300,
                ..parameters
            };
            assert!(!emsa_pss_verify(cramped, &digest, &encoded, em_bits));
        }
    }
}
