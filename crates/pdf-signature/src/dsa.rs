//! DSA signature verification: Table 260's second algorithm family.
//!
//! ISO 32000-2's Table 260 gives a PDF signature three algorithm families, and this is the one
//! between the two the four-hundred-and-seventy-ninth session found on either side of it:
//!
//! | | `adbe.pkcs7.detached`, `ETSI.CAdES.detached` or `ETSI.RFC3161` | `adbe.pkcs7.sha1` | `adbe.x509.rsa_sha1` |
//! |---|---|---|---|
//! | RSA Algorithm Support | Up to 1024/2048/4096-bit | See `adbe.pkcs7.detached` | See `adbe.pkcs7.detached` |
//! | DSA Algorithm Support | Up to 4096-bits (PDF 1.6) | See `adbe.pkcs7.detached` | **No** |
//! | ECDSA Algorithm Support | ANSI X9.62 (PDF 2.0) | No | No |
//!
//! **That "No" is load-bearing and is why this module is only ever reached through CMS.** Table
//! 260 forbids DSA for §12.8.3.2's `adbe.x509.rsa_sha1`, so a `/Cert` holding a DSA key is a file
//! departing from the table rather than a case to handle — [`crate::signature::Signature`] reports
//! such a key by its object identifier and verifies nothing. §12.8.3.2's own sentence that "[t]he
//! PKCS #1 standard supports several public-key cryptographic algorithms and digest methods,
//! including RSA encryption, DSA signatures" is about what the *standard* supports, and Table 260
//! is what says which of them this `/SubFilter` may carry.
//!
//! # Where the algorithm comes from, since ISO 32000-2 does not state it
//!
//! Table 260 names DSA and states a key size and nothing else, so the arithmetic is FIPS 186-4's.
//! Section 4.7, "DSA Signature Verification and Validation", states it in full, and this module is
//! those steps in order; every one of its sentences quoted below was read out of the published
//! standard rather than recalled. Two documents supply the encodings ISO 32000-2 does not:
//! RFC 3279 sections 2.2.2 and 2.3.2 for `Dss-Sig-Value`, `Dss-Parms` and `DSAPublicKey`, and RFC
//! 5758 section 3.1 for the two SHA-2 identifiers it adds.
//!
//! **FIPS 186-5 withdrew DSA for *signing* and kept it for exactly what this is.** Its section 4:
//! "This standard no longer approves the DSA for digital signature generation. However, the DSA
//! may be used to verify signatures generated prior to the implementation date of this standard".
//! A viewer is a verifier, and documents signed before that date do not stop existing.
//!
//! # There is no secret here either
//!
//! ADR 0229's argument for writing RSA in the tree carries over unchanged, and it is what makes
//! this module small: `p`, `q`, `g`, `y`, `r` and `s` all came out of a file a stranger wrote, none
//! of them is ours and none is private, so the side-channel class of defect that ADR 0031 takes
//! reviewed implementations for has nothing to act on. Nothing here runs in constant time.
//!
//! What it does *not* carry over is the direction of a mistake. RSA verification compares two
//! whole encoded blocks and a mistake produces "does not verify"; DSA compares two numbers modulo
//! `q`, and the checks that keep a forgery out are FIPS 186-4's own step 1 — `0 < r' < q` and
//! `0 < s' < q` — which is why it is the first thing [`verify`] does and why the test below feeds
//! it `r = 0`, `s = 0`, `r = q` and `s = q` one at a time.
//!
//! # Why the parameters need no table and a curve's do
//!
//! Every number this module computes with is in the file: `p`, `q` and `g` are the certificate's
//! own `Dss-Parms`. An elliptic-curve verification instead needs the domain parameters of a curve
//! the certificate only *names*, and those are in no document this tree holds — which was ADR
//! 0314's argument for stopping there. **It stopped deciding anything when the owner accepted
//! reviewed arithmetic as a dependency** (ADR 0331): reviewed constants in a curve package stand
//! on the same footing, and [`crate::ecdsa`] is that family since the six-hundred-and-eighty-ninth
//! session (ADR 0532). The contrast is still worth stating, because it is why *this* module needs
//! no dependency at all.

use crate::bigint::{Integer, MAX_BITS, Modulus, modpow, significant_bits};
use crate::der::{INTEGER, Reader, SEQUENCE};

/// The widest `p` this module will exponentiate, in bits.
///
/// Table 260's ceiling is 4096 ("DSA Algorithm Support | Up to 4096-bits (PDF 1.6)") and this is
/// twice it, on [`crate::pkcs1::MAX_MODULUS_BITS`]'s reasoning: a key beyond the standard is a
/// file to report rather than one to refuse silently, and one beyond this is
/// [`DsaError::ModulusTooLarge`].
pub const MAX_MODULUS_BITS: usize = MAX_BITS;

/// The widest `q` this module will work modulo, in bits.
///
/// **A time budget rather than an opinion about keys.** Two of the three exponentiations below run
/// with an exponent below `q`, and each costs one modular squaring per bit of it, so an unbounded
/// `q` is unbounded work over a number a stranger chose. FIPS 186-4 section 4.2 gives the four
/// approved pairs of `(L, N)` — (1024, 160), (2048, 224), (2048, 256) and (3072, 256) — so the
/// largest `q` the standard admits is 256 bits and this is twice it. Beyond it is
/// [`DsaError::SubgroupTooLarge`].
///
/// **What the two budgets leave as a worst case**, because a bound nobody has multiplied out is a
/// hope: two exponentiations of 512 bits each is 2048 modular multiplications at `p`'s width, so a
/// key at both ceilings costs about four times what [`crate::pkcs1`]'s does at its own — tens of
/// milliseconds, once, on the document's own thread and never on the launch path.
pub const MAX_SUBGROUP_BITS: usize = 512;

/// RFC 3279 section 2.3.2's `id-dsa`, `1.2.840.10040.4.1`.
///
/// ```text
/// id-dsa OBJECT IDENTIFIER ::= { iso(1) member-body(2) us(840) x9-57(10040) x9cm(4) 1 }
/// ```
///
/// This is what a certificate's `subjectPublicKeyInfo` states for a DSA key, and what some
/// producers state as a `SignerInfo`'s `signatureAlgorithm` as well.
pub const ID_DSA: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x38, 0x04, 0x01];

/// RFC 3279 section 2.2.2's `id-dsa-with-sha1`, `1.2.840.10040.4.3`.
pub const ID_DSA_WITH_SHA1: &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x38, 0x04, 0x03];

/// The `id-dsa-with-sha2` arc, `2.16.840.1.101.3.4.3` — one identifier per digest.
///
/// RFC 5758 section 3.1 assigns `1` to SHA-224 and `2` to SHA-256. The NIST Computer Security
/// Objects Register, which is the authority that assigns this arc, carries `3` and `4` for SHA-384
/// and SHA-512 and `5` to `8` for the SHA-3 family — and Table 260 permits SHA-384 and SHA-512 for
/// this `/SubFilter`, so a conforming file can state either.
///
/// **Recognising an identifier is not computing its digest**: which function was used is
/// `SignerInfo`'s own `digestAlgorithm`, read by [`crate::cms::Digest`], and a signature naming one
/// this program cannot compute is reported by that number instead of this one.
const ID_DSA_WITH_SHA2: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x03];

/// Whether an object identifier names a DSA signature or a DSA key.
#[must_use]
pub fn is_dsa(oid: &[u8]) -> bool {
    if oid == ID_DSA || oid == ID_DSA_WITH_SHA1 {
        return true;
    }
    // The arc plus exactly one more component, which is the digest. A prefix test over a longer
    // tail would accept identifiers nobody has assigned.
    matches!(oid.split_at_checked(ID_DSA_WITH_SHA2.len()),
        Some((arc, [_])) if arc == ID_DSA_WITH_SHA2)
}

/// What stopped a DSA signature from being verified — a statement about the file, in every case.
///
/// None of these is "the signature is bad". A signature that is read, exponentiated and found not
/// to match is [`Ok(false)`](verify), and so is one FIPS 186-4 section 4.7 step 1 rejects; these
/// are the cases where the arithmetic never ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum DsaError {
    /// `p` is wider than [`MAX_MODULUS_BITS`].
    #[error("the signer's DSA modulus p is wider than {MAX_MODULUS_BITS} bits")]
    ModulusTooLarge,
    /// `q` is wider than [`MAX_SUBGROUP_BITS`].
    #[error("the signer's DSA subgroup order q is wider than {MAX_SUBGROUP_BITS} bits")]
    SubgroupTooLarge,
    /// `p` is zero, one, or even.
    ///
    /// FIPS 186-4 section 4.1 makes `p` "a prime modulus", so an even one is not a DSA modulus at
    /// all — and the Montgomery reduction [`crate::bigint`] uses exists only for an odd one.
    #[error("the signer's DSA modulus p is not an odd number greater than one")]
    ModulusNotOdd,
    /// `q` is zero, one, or even, for the same two reasons.
    #[error("the signer's DSA subgroup order q is not an odd number greater than one")]
    SubgroupNotOdd,
    /// `q` is not smaller than `p`.
    ///
    /// FIPS 186-4 section 4.1 makes `q` "a prime divisor of (p − 1)", which cannot be as large as
    /// `p`. A file stating otherwise has not written a DSA key, and the reduction of `v` modulo `q`
    /// that step 2 ends with would be meaningless.
    #[error("the signer's DSA subgroup order q is not smaller than the modulus p")]
    SubgroupNotSmaller,
    /// `g` or `y` is wider than [`MAX_MODULUS_BITS`].
    ///
    /// Separate from [`Self::ModulusTooLarge`] because it is a different field of the same key, and
    /// reporting a wide generator as a wide modulus would send a reader to the wrong number.
    #[error("the signer's DSA generator or public value is wider than {MAX_MODULUS_BITS} bits")]
    KeyValueTooLarge,
    /// The signature value is not RFC 3279 section 2.2.2's `Dss-Sig-Value`.
    #[error("the signature is not a DER Dss-Sig-Value of two integers")]
    SignatureMalformed,
    /// `r` or `s` is wider than [`MAX_MODULUS_BITS`].
    ///
    /// A signature value merely larger than `q` is not this: FIPS 186-4 section 4.7 step 1 rejects
    /// one as invalid, which is [`Ok(false)`](verify). This is a number too wide to hold at all.
    #[error("the signature's integers are wider than {MAX_MODULUS_BITS} bits")]
    SignatureTooLarge,
}

/// The public half of a DSA key: RFC 3279's `Dss-Parms` and the `y` beside it.
///
/// All four are big-endian, exactly as the certificate writes them, and all four are borrowed:
/// nothing here owns or copies a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicKey<'a> {
    /// `p`, the prime modulus.
    pub p: &'a [u8],
    /// `q`, the prime divisor of `p - 1`.
    pub q: &'a [u8],
    /// `g`, the generator of the order-`q` subgroup.
    pub g: &'a [u8],
    /// `y`, the public key itself — RFC 3279 section 2.3.2's `DSAPublicKey ::= INTEGER`.
    pub y: &'a [u8],
}

impl PublicKey<'_> {
    /// The key's width in bits, which is `p`'s — the number Table 260 puts its ceiling on.
    #[must_use]
    pub fn bits(&self) -> usize {
        significant_bits(self.p)
    }

    /// `N` in FIPS 186-4's notation: the bit length of `q`, which decides how much of a digest is
    /// used.
    #[must_use]
    pub fn subgroup_bits(&self) -> usize {
        significant_bits(self.q)
    }
}

/// RFC 3279 section 2.2.2's `Dss-Sig-Value ::= SEQUENCE { r INTEGER, s INTEGER }`, as two slices.
///
/// That clause is where the two-number shape comes from: "When signing, the DSA algorithm
/// generates two values. These values are commonly referred to as r and s. To easily transfer
/// these two values as one signature, they SHALL be ASN.1 encoded using the following ASN.1
/// structure".
fn signature_values(signature: &[u8]) -> Result<(&[u8], &[u8]), DsaError> {
    let mut reader = Reader::new(signature).map_err(|_| DsaError::SignatureMalformed)?;
    let value = reader
        .next_value()
        .map_err(|_| DsaError::SignatureMalformed)?
        .ok_or(DsaError::SignatureMalformed)?;
    if value.identifier != SEQUENCE {
        return Err(DsaError::SignatureMalformed);
    }
    let mut members = value.children().map_err(|_| DsaError::SignatureMalformed)?;
    let mut next = || -> Result<&[u8], DsaError> {
        let member = members
            .next_value()
            .map_err(|_| DsaError::SignatureMalformed)?
            .ok_or(DsaError::SignatureMalformed)?;
        if member.identifier != INTEGER {
            return Err(DsaError::SignatureMalformed);
        }
        Ok(member.contents)
    };
    let r = next()?;
    let s = next()?;
    Ok((r, s))
}

/// Whether `signature` is a DSA signature over `message_digest` under `key`.
///
/// `signature` is the `SignerInfo`'s signature octets, which RFC 3279 makes a DER `Dss-Sig-Value`.
/// `message_digest` is the digest the *caller* computed over whatever RFC 5652 says is signed —
/// this module never hashes anything, so that nothing here can disagree with
/// [`crate::cms::Digest`] about what was hashed.
///
/// The steps are FIPS 186-4 section 4.7's, in its order and with its names:
///
/// ```text
/// 1. The verifier shall check that 0 < r' < q and 0 < s' < q; if either condition is
///    violated, the signature shall be rejected as invalid.
/// 2. If the two conditions in step 1 are satisfied, the verifier computes the following:
///      w  = (s')^-1 mod q
///      z  = the leftmost min(N, outlen) bits of Hash(M')
///      u1 = (zw) mod q
///      u2 = ((r')w) mod q
///      v  = (((g)^u1 (y)^u2) mod p) mod q
/// 3. If v = r', then the signature is verified.
/// ```
///
/// `z` is taken as an integer by Appendix C.2.1's rule — most significant bit first — and the
/// truncation is a shift rather than a byte slice, so that an `N` which is not a whole number of
/// octets takes the bits the standard says rather than the octets that contain them.
///
/// Its step 5 — "the verifier shall have assurances as specified in Section 3.3" — is a signature's
/// *third* question and this program answers none of it. [`crate::signature::Authenticity`] is
/// where that is said out loud rather than implied by a return value.
///
/// # Errors
///
/// A [`DsaError`] where the key or the signature is outside this module's budgets or is not shaped
/// like a DSA one at all. A signature that is well-formed and simply does not verify is
/// `Ok(false)`, and the two are kept apart because only the second says anything about the file's
/// honesty.
#[expect(
    clippy::many_single_char_names,
    reason = "p, q, g, y, r, s, w, z and v are FIPS 186-4 section 4.7's own names for these nine \
              quantities, and a reader checking this against the standard needs them spelled the \
              way the standard spells them"
)]
pub fn verify(
    key: PublicKey<'_>,
    signature: &[u8],
    message_digest: &[u8],
) -> Result<bool, DsaError> {
    let p = Integer::from_be_bytes(key.p).ok_or(DsaError::ModulusTooLarge)?;
    let q = Integer::from_be_bytes(key.q).ok_or(DsaError::SubgroupTooLarge)?;
    if key.subgroup_bits() > MAX_SUBGROUP_BITS {
        return Err(DsaError::SubgroupTooLarge);
    }
    if !q.less_than(&p) {
        return Err(DsaError::SubgroupNotSmaller);
    }
    let p = Modulus::new(&p).ok_or(DsaError::ModulusNotOdd)?;
    let q = Modulus::new(&q).ok_or(DsaError::SubgroupNotOdd)?;

    let (r_bytes, s_bytes) = signature_values(signature)?;
    let r = Integer::from_be_bytes(r_bytes).ok_or(DsaError::SignatureTooLarge)?;
    let s = Integer::from_be_bytes(s_bytes).ok_or(DsaError::SignatureTooLarge)?;
    // Step 1. Both bounds, both ends, and a violated one is `Ok(false)` rather than an error: the
    // standard's word is "rejected as invalid", which is a verdict on the signature.
    if r.is_zero() || s.is_zero() || !r.less_than(&q.value) || !s.less_than(&q.value) {
        return Ok(false);
    }

    // Step 2. `w = s^-1 mod q` — FIPS 186-4 Appendix C.1 states the extended Euclidean algorithm
    // and admits "an algorithm that produces an equivalent result"; `Modulus::invert` is
    // `crypto-bigint`'s, which is one.
    let Some(w) = q.invert(&s) else {
        return Ok(false);
    };
    let z =
        truncated_digest(message_digest, key.subgroup_bits()).ok_or(DsaError::SubgroupTooLarge)?;
    let z = q.reduce(&z);
    let u1 = q.multiply_reduced(&z, &w);
    let u2 = q.multiply_reduced(&r, &w);
    // `v = ((g^u1 y^u2) mod p) mod q`. `g` and `y` are reduced first because they are numbers a
    // file wrote and the exponentiation's domain conversion needs a base already below `p`.
    let g = p.reduce(&Integer::from_be_bytes(key.g).ok_or(DsaError::KeyValueTooLarge)?);
    let y = p.reduce(&Integer::from_be_bytes(key.y).ok_or(DsaError::KeyValueTooLarge)?);
    let product = p.multiply_reduced(&modpow(&g, &u1, &p), &modpow(&y, &u2, &p));
    let v = q.reduce(&product);

    // Step 3.
    Ok(v.equals(&r))
}

/// FIPS 186-4 section 4.7 step 2's `z`, as an integer: "the leftmost min(N, outlen) bits of
/// Hash(M′ )", where `N` is the bit length of `q` and `outlen` the hash function's.
///
/// Appendix C.2.1 fixes how those bits become a number — "the first bit of a sequence corresponds
/// to the most significant bit of the corresponding integer" — so the digest is read big-endian
/// and the *low* `outlen - min(N, outlen)` bits are shifted away. Every `(L, N)` pair the standard
/// approves makes `N` a multiple of eight, so the shift is zero in practice; it is written for the
/// general case because a file states `q` and this program does not get to assume its width.
///
/// `None` where the digest is wider than [`crate::bigint`] holds, which no hash function this
/// program computes can produce.
fn truncated_digest(message_digest: &[u8], subgroup_bits: usize) -> Option<Integer> {
    let outlen = message_digest.len().saturating_mul(8);
    let value = Integer::from_be_bytes(message_digest)?;
    let drop = outlen.saturating_sub(subgroup_bits.min(outlen));
    Some(value.shifted_right(drop))
}

/// A DSA key and a signature made with it, built once with `openssl` and pasted in.
///
/// **Test vectors rather than oracles**, on the footing `pkcs1`'s constants are on and for a
/// sharper reason: FIPS 186-4 defines the arithmetic and RFC 3279 the encoding, and what a vector
/// pins is that *these* lines compute them. Nothing about the vector decides what is correct.
///
/// **A fixture rather than a corpus document, and the population is why.** 67 460 documents were
/// read for this round — the 974 of `doc/pdf.js`, the 275 under `doc/corpora`, and all 66 211 of
/// the `SafeDocs` crawl — and they hold 811 signature dictionaries between them. **Not one names a
/// DSA key or a DSA signature algorithm.** `CLAUDE.md`'s trap 8 is exactly this case: a corpus
/// finds what documents contain and not what the standard says, and a hand-built pair is the only
/// witness a requirement with no file behind it can have.
#[cfg(test)]
pub(crate) mod fixtures {
    /// A self-signed 2048-bit DSA certificate, `dsa-with-SHA256`, over the key below.
    ///
    /// `openssl dsaparam 2048`, `openssl gendsa`, `openssl req -x509 -sha256`.
    pub(crate) const CERTIFICATE: &str = "\
        3082047830820425a0030201020214274a70a668212a187021ba9e224680d8d5\
        f8fb14300b0609608648016503040302301e311c301a06035504030c13706466\
        2d766965776572206473612074657374301e170d323630383133313433383432\
        5a170d3336303831303134333834325a301e311c301a06035504030c13706466\
        2d766965776572206473612074657374308203443082023606072a8648ce3804\
        01308202290282010100d9d123085131364d0bcbb4049269167efee82595e73a\
        6e7098d31a254edf704826ad20158da54a0166020f860d6f3cf2b26fcbd3510c\
        888782207a3710a7f47f7de61df5afed7901e83b1b06def80b3816332b4c0bb5\
        fb15ffebc04a9baadb30b9edf10001f8b28406b5b0ea01d248fb1b6c7c30f41b\
        09953140ca5fe32516c28de90096b7c05e2324c88e70e42a27778d79283f93e9\
        97fa89cd852fab4de8cd719a2c7c60d9dba3bdf64568b910d7e310e3a55e80c6\
        a0c4827a1a126f144f4491006f56a9dc7855391129853290ef0f82cf00adc17c\
        f746f7b46bc7790bcda54790262b67f5d5091fe1319faa2fd253ea6cbe589879\
        53ead84fe62c5a5e3cd7021d00c469479a7eef6cefe349e556c5151625521bad\
        cf21548f1f4520041d028201010089132814febe89b908c6b950782fa9527e0e\
        66f1a56e3043255d72158c1ecba76cad1cb27c126db7afb90e11783ae032a613\
        ad5b05276cd5e961f395399e36a2a902e356d8f77d35f27f2d06a2b756321248\
        fcc2cdb8e8f5ca7bc34f5dd4b8252aededf29134148df9cf831911c2dba66619\
        606153bc5fede786d809b6eafd4c2474d4db789a452b0c677bf00011b60a1945\
        1a2de1f48b067635e7ee063758bf7f49712bc375cb7cb2b9d28d57347a20099c\
        04c4e46f6084ae8fc4aacb20597eaa45fb149ca7777fbf7cf07a0801bd12e769\
        b3c2c96eaac4572ecf4ad26dbb4cae8506cb88eaaa370154349dda8374204c6b\
        ef7624097376c6d5bcae796bde8703820106000282010100b0ffafff3c64e1c5\
        189a61abc335ff334c058746c1e65d7acdb480f81cdec2535a07aa0ddfa61344\
        fb399dfc252eac355fc950ea75ac2e186ab05701f08040444bb4edde9876b6a3\
        6ffd2daa23d550ee1a48c27a6c195ee99a8b719bf8681dd0e6b6e70a529c66d2\
        77b8c25588f63b21a354cba03d14739d706f0f6d880d0c74ac7c9971f2db6218\
        734d05f9ff23328eb6ab46b182c7f3d7e51ee6bdcf150a543e659b4b5958b8ee\
        a04c2816cda7cd2e1c3078409235dd055c528a0bc696c1f9be5233fc45357f8d\
        fccc68f61e1552146715b604faf2832f675963ca84112edbb5e9060fce6f8f7b\
        64f6596b8caf9dd6da5bfd759910b4498e8a5159a78767bda3533051301d0603\
        551d0e0416041454ce36e81e6d47d3d13bccd56e79d8e86c11fc3d301f060355\
        1d2304183016801454ce36e81e6d47d3d13bccd56e79d8e86c11fc3d300f0603\
        551d130101ff040530030101ff300b0609608648016503040302034000303d02\
        1c1d712d27114f632c2ab7ba3888892f19cd9ea0ca959fcfc3573f06fb021d00\
        ae3bf3ba86f2b138872e46c4c33ccbf4e23cee97902486f775657737";

    /// The DER `Dss-Sig-Value` of `SHA-256(b"the signed bytes")` under that key.
    pub(crate) const SIGNATURE: &str = "\
        303c021c6cee1705fab9ed9986c89cb23d5b5a2136d2c0463f297da450834a35\
        021c5a69787ef200460e3540fad38883458b1863b7f2121bf093492afa66";
}

#[cfg(test)]
mod tests {
    use super::{DsaError, ID_DSA, ID_DSA_WITH_SHA1, MAX_SUBGROUP_BITS, PublicKey, is_dsa, verify};
    use crate::cms::Digest;
    use crate::x509::{self, dotted};

    /// The key out of the fixture certificate, which is the only place a test gets one.
    fn key(bytes: &[u8]) -> x509::Certificate<'_> {
        x509::parse(bytes).expect("a certificate")
    }

    /// The whole of [`verify`], on a real 2048-bit key and a real signature.
    #[test]
    fn a_signature_verifies_under_the_key_that_made_it_and_under_no_other() {
        let certificate = x509::fixtures::hex(super::fixtures::CERTIFICATE);
        let certificate = key(&certificate);
        let x509::PublicKey::Dsa(key) = certificate.public_key else {
            panic!("this certificate states id-dsa");
        };
        assert_eq!(key.bits(), 2048, "Table 260's ceiling is on p");
        // FIPS 186-4 section 4.2's `(L, N) = (2048, 224)`, and the pair is worth stating: SHA-256
        // is 256 bits wide, so this vector is one where `z` is a *truncation* rather than the
        // whole digest. A verifier that skipped step 2's `min(N, outlen)` passes on (2048, 256)
        // and fails here.
        assert_eq!(key.subgroup_bits(), 224, "FIPS 186-4 section 4.2's N");
        let signature = x509::fixtures::hex(super::fixtures::SIGNATURE);
        let digest = Digest::Sha256.compute(&[b"the signed bytes"]);
        assert_eq!(verify(key, &signature, &digest), Ok(true));

        // One bit of the message, and it no longer verifies.
        let other = Digest::Sha256.compute(&[b"the signed byteS"]);
        assert_eq!(verify(key, &signature, &other), Ok(false));

        // One bit of `s`, likewise — the last octet of the encoding is `s`'s least significant.
        let mut tampered = signature.clone();
        if let Some(last) = tampered.last_mut() {
            *last ^= 1;
        }
        assert_eq!(
            verify(key, &tampered, &digest),
            Ok(false),
            "a signature is not its neighbour"
        );

        // And under a key that is the same but for its `y`, which is what a wrong signer is.
        let mut other_y = key.y.to_vec();
        if let Some(last) = other_y.last_mut() {
            *last ^= 1;
        }
        assert_eq!(
            verify(PublicKey { y: &other_y, ..key }, &signature, &digest),
            Ok(false)
        );
    }

    /// FIPS 186-4 section 4.7 step 1, at both ends of both bounds.
    ///
    /// **This is the step that keeps a forgery out**, and it is the one a verifier can most easily
    /// leave out and still pass every positive test: `r = 0` with `s = 0` makes `v` computable and
    /// meaningless. Each of the four is built by re-encoding the fixture's `Dss-Sig-Value` with one
    /// value replaced.
    #[test]
    fn a_signature_outside_the_subgroup_is_rejected_by_step_one() {
        let certificate = x509::fixtures::hex(super::fixtures::CERTIFICATE);
        let certificate = key(&certificate);
        let x509::PublicKey::Dsa(key) = certificate.public_key else {
            panic!("id-dsa");
        };
        let digest = Digest::Sha256.compute(&[b"the signed bytes"]);
        let signature = x509::fixtures::hex(super::fixtures::SIGNATURE);
        let (r, s) = super::signature_values(&signature).expect("two integers");
        let q = key.q.to_vec();
        for (r, s, why) in [
            (vec![0x00], s.to_vec(), "r = 0"),
            (r.to_vec(), vec![0x00], "s = 0"),
            (q.clone(), s.to_vec(), "r = q"),
            (r.to_vec(), q.clone(), "s = q"),
        ] {
            assert_eq!(
                verify(key, &encode(&r, &s), &digest),
                Ok(false),
                "step 1 rejects {why}"
            );
        }
        // And the real pair still verifies once re-encoded by the same builder, so that the four
        // above fail for their own reason rather than because the builder is wrong.
        assert_eq!(verify(key, &encode(r, s), &digest), Ok(true));
    }

    /// A `Dss-Sig-Value` around two big-endian integers, for the test above.
    fn encode(r: &[u8], s: &[u8]) -> Vec<u8> {
        let mut body = Vec::new();
        for value in [r, s] {
            body.push(0x02);
            body.push(u8::try_from(value.len()).unwrap_or(0));
            body.extend_from_slice(value);
        }
        let mut out = vec![0x30];
        if body.len() < 128 {
            out.push(u8::try_from(body.len()).unwrap_or(0));
        } else {
            out.push(0x81);
            out.push(u8::try_from(body.len()).unwrap_or(0));
        }
        out.extend_from_slice(&body);
        out
    }

    /// Every budget and every malformed shape, refused by name rather than by running out.
    #[test]
    fn the_budgets_are_reported_rather_than_reached() {
        let certificate = x509::fixtures::hex(super::fixtures::CERTIFICATE);
        let certificate = key(&certificate);
        let x509::PublicKey::Dsa(key) = certificate.public_key else {
            panic!("id-dsa");
        };
        let signature = x509::fixtures::hex(super::fixtures::SIGNATURE);
        let digest = Digest::Sha256.compute(&[b"the signed bytes"]);

        let huge = vec![0xFFu8; (MAX_SUBGROUP_BITS / 8) + 1];
        assert_eq!(
            verify(PublicKey { q: &huge, ..key }, &signature, &digest),
            Err(DsaError::SubgroupTooLarge)
        );
        assert_eq!(
            verify(PublicKey { p: &[0x04], ..key }, &signature, &digest),
            Err(DsaError::SubgroupNotSmaller),
            "q must divide p - 1, so it cannot reach p"
        );
        let mut even = key.p.to_vec();
        if let Some(last) = even.last_mut() {
            *last &= 0xFE;
        }
        assert_eq!(
            verify(PublicKey { p: &even, ..key }, &signature, &digest),
            Err(DsaError::ModulusNotOdd)
        );
        let mut even = key.q.to_vec();
        if let Some(last) = even.last_mut() {
            *last &= 0xFE;
        }
        assert_eq!(
            verify(PublicKey { q: &even, ..key }, &signature, &digest),
            Err(DsaError::SubgroupNotOdd)
        );
        for malformed in [
            &[][..],
            &[0x04, 0x01, 0x00],
            &[0x30, 0x00],
            &[0x30, 0x03, 0x02, 0x01, 0x01],
        ] {
            assert_eq!(
                verify(key, malformed, &digest),
                Err(DsaError::SignatureMalformed),
                "a Dss-Sig-Value is a SEQUENCE of two INTEGERs"
            );
        }
    }

    /// The identifiers, decoded by this tree's own reader rather than trusted as written.
    ///
    /// A constant written as octets is a claim about a number, and `x509::dotted` is what checks
    /// it — the same discipline `x509`'s own test applies to `rsaEncryption`.
    #[test]
    fn the_object_identifiers_are_the_numbers_the_documents_assign() {
        assert_eq!(dotted(ID_DSA).as_deref(), Some("1.2.840.10040.4.1"));
        assert_eq!(
            dotted(ID_DSA_WITH_SHA1).as_deref(),
            Some("1.2.840.10040.4.3")
        );
        assert!(is_dsa(ID_DSA));
        assert!(is_dsa(ID_DSA_WITH_SHA1));
        // RFC 5758 section 3.1's two, and the CSOR's SHA-384 and SHA-512 beside them.
        for (last, expected) in [
            (1, "2.16.840.1.101.3.4.3.1"),
            (2, "2.16.840.1.101.3.4.3.2"),
            (3, "2.16.840.1.101.3.4.3.3"),
            (4, "2.16.840.1.101.3.4.3.4"),
        ] {
            let oid = [super::ID_DSA_WITH_SHA2, &[last]].concat();
            assert_eq!(dotted(&oid).as_deref(), Some(expected));
            assert!(is_dsa(&oid));
        }
        // The arc itself is not a signature algorithm, and neither is one component further on.
        assert!(!is_dsa(super::ID_DSA_WITH_SHA2));
        assert!(!is_dsa(&[super::ID_DSA_WITH_SHA2, &[1, 1]].concat()));
        // RSA and ECDSA are not DSA, which a prefix test over `1.2.840` would have said they were.
        assert!(!is_dsa(&[
            0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x01
        ]));
        assert!(!is_dsa(&[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03, 0x02]));
        assert!(!is_dsa(&[]));
    }

    /// FIPS 186-4 section 4.7's `z`, including the case no approved `(L, N)` pair produces.
    #[test]
    fn a_digest_wider_than_q_keeps_its_leftmost_bits() {
        // outlen 32, N 16: the leftmost two octets, which is a whole-octet shift.
        let value = super::truncated_digest(&[0x12, 0x34, 0x56, 0x78], 16).expect("fits");
        assert_eq!(value.be_bytes(2), [0x12, 0x34]);
        // N wider than the digest takes all of it — min(N, outlen).
        let value = super::truncated_digest(&[0x12, 0x34], 256).expect("fits");
        assert_eq!(value.be_bytes(2), [0x12, 0x34]);
        // And an N that is not a whole number of octets shifts by bits, which is the case
        // Appendix C.2.1 decides and a byte slice would get wrong: the leftmost 12 bits of
        // 0x1234 are 0x123.
        let value = super::truncated_digest(&[0x12, 0x34], 12).expect("fits");
        assert_eq!(value.be_bytes(2), [0x01, 0x23]);
    }
}
