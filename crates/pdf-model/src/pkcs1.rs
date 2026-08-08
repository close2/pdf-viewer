//! RSASSA-PKCS1-v1_5 verification: the arithmetic behind a signature's *second* question.
//!
//! ISO 32000-2 §12.8.3.2 is titled "PKCS #1 signatures" and says what the standard expects of a
//! processor that verifies one:
//!
//! > The PKCS #1 standard supports several public-key cryptographic algorithms and digest
//! > methods, including RSA encryption, DSA signatures, and SHA-1 and MD5 digests.
//!
//! and Table 260 states the sizes: "RSA Algorithm Support | Up to 1024-bit (PDF 1.3) Up to
//! 2048-bit (PDF 1.5) Up to 4096-bit (PDF 1.5)". This module is the RSA half of that, and the
//! other two algorithms Table 260 names — DSA and ECDSA — are **not** here and are reported by
//! name rather than skipped ([`crate::signature::Authenticity`]). ADR 0229 has the decision.
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
//! # Why this is in the tree rather than a dependency
//!
//! ADR 0229 argues it against the `rsa` crate, and the short form is that the argument ADR
//! 0031 made for taking the *ciphers* has no instance here: **there is no secret.** A signature
//! verification is `s^e mod n` over three numbers a stranger wrote into the file, none of them
//! ours and none of them private, so the class of defect wide review buys protection from —
//! timing and cache side channels on key material — has nothing to act on. Nothing in this module
//! is written to run in constant time and nothing needs to be.
//!
//! What is left is integer arithmetic against published test vectors, under this crate's
//! `#![forbid(unsafe_code)]`, with every loop bounded by a constant rather than by the file.
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

use crate::cms::Digest;

/// The widest modulus this module will exponentiate, in bits.
///
/// Table 260's ceiling is 4096 ("Up to 4096-bit (PDF 1.5)"), and this is twice it: a key larger
/// than the standard describes is a file to report rather than a file to refuse silently, and one
/// larger than *this* is refused with [`Pkcs1Error::ModulusTooLarge`]. The cost of the headroom is
/// one kilobyte of stack per big integer.
pub const MAX_MODULUS_BITS: usize = 8192;

/// The largest public exponent this module will raise a signature to, in bits.
///
/// [`modpow`] costs one modular squaring per bit of the exponent, so this is a *time* budget on
/// arithmetic whose inputs come out of the file. The exponents real keys use are 3 and 65537, and
/// this leaves room for eight times as many bits as the larger of them needs while keeping the
/// worst case at 512 modular multiplications.
pub const MAX_EXPONENT_BITS: usize = 256;

/// How many 64-bit limbs [`MAX_MODULUS_BITS`] is.
const MAX_LIMBS: usize = MAX_MODULUS_BITS / 64;

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
        let leading = self
            .modulus
            .iter()
            .take_while(|&&byte| byte == 0)
            .count()
            .min(self.modulus.len());
        let Some(rest) = self.modulus.get(leading..) else {
            return 0;
        };
        let Some(&first) = rest.first() else {
            return 0;
        };
        // Whole octets after the leading one, plus that one's significant bits.
        rest.len()
            .saturating_sub(1)
            .saturating_mul(8)
            .saturating_add(8usize.saturating_sub(first.leading_zeros() as usize))
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
    // RFC 8017 section 8.2.2 step 1, and the reason the length is checked rather than the value: `k` is
    // the modulus's length in octets, which is what `i2osp` will produce, so a signature of any
    // other length cannot equal the encoded block whatever it contains.
    let length = key.modulus.len().saturating_sub(
        key.modulus
            .iter()
            .take_while(|&&byte| byte == 0)
            .count()
            .min(key.modulus.len()),
    );
    if signature.len() != length {
        return Err(Pkcs1Error::SignatureLength);
    }
    let value = Integer::from_be_bytes(signature).ok_or(Pkcs1Error::ModulusTooLarge)?;
    if !value.less_than(&modulus.value) {
        return Err(Pkcs1Error::SignatureNotReduced);
    }
    let expected = encode(digest, message_digest, length)?;
    let recovered = modpow(&value, &exponent, &modulus).be_bytes(length);
    Ok(recovered == expected)
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

/// A big unsigned integer of at most [`MAX_LIMBS`] limbs, least significant first.
///
/// Fixed size rather than a `Vec`: the bound is [`MAX_MODULUS_BITS`] and a fixed array makes
/// every loop's trip count a constant of this module rather than a number out of the file, which
/// is what `CLAUDE.md` principle 3 asks of arithmetic over untrusted input.
#[derive(Clone, Copy)]
struct Integer {
    limbs: [u64; MAX_LIMBS],
}

impl std::fmt::Debug for Integer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Integer({} bits)", self.bits())
    }
}

impl Integer {
    /// Zero.
    fn zero() -> Self {
        Self {
            limbs: [0; MAX_LIMBS],
        }
    }

    /// A big-endian byte string as an integer, or `None` where it needs more than [`MAX_LIMBS`].
    ///
    /// Leading zero octets are ignored, which is what makes an X.509 `INTEGER`'s sign octet
    /// harmless here: RFC 5280's serial numbers and moduli are written with a leading `0x00` when
    /// the high bit is set, and the value is the same number either way.
    fn from_be_bytes(bytes: &[u8]) -> Option<Self> {
        let leading = bytes
            .iter()
            .take_while(|&&byte| byte == 0)
            .count()
            .min(bytes.len());
        let significant = bytes.get(leading..).unwrap_or(&[]);
        if significant.len().div_ceil(8) > MAX_LIMBS {
            return None;
        }
        let mut out = Self::zero();
        for (index, byte) in significant.iter().rev().enumerate() {
            let limb = index / 8;
            let shift = u32::try_from(index % 8).unwrap_or(0).saturating_mul(8);
            if let Some(slot) = out.limbs.get_mut(limb) {
                *slot |= u64::from(*byte) << shift;
            }
        }
        Some(out)
    }

    /// The integer as exactly `length` big-endian octets, zero-padded on the left.
    ///
    /// Truncating on the left where the value needs more, which cannot happen for a value reduced
    /// modulo an `n` of that many octets — and which would produce a block that fails the
    /// comparison rather than one that passes it.
    fn be_bytes(&self, length: usize) -> Vec<u8> {
        let mut out = vec![0u8; length];
        for index in 0..length {
            let limb = index / 8;
            let shift = u32::try_from(index % 8).unwrap_or(0).saturating_mul(8);
            let byte = self.limbs.get(limb).map_or(0, |value| {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "one octet of a limb is wanted, and the shift selects it"
                )]
                {
                    (value >> shift) as u8
                }
            });
            if let Some(slot) = out
                .len()
                .checked_sub(1)
                .and_then(|last| last.checked_sub(index))
                .and_then(|at| out.get_mut(at))
            {
                *slot = byte;
            }
        }
        out
    }

    /// Whether every limb is zero.
    fn is_zero(&self) -> bool {
        self.limbs.iter().all(|&limb| limb == 0)
    }

    /// The number of significant bits.
    fn bits(&self) -> usize {
        for index in (0..MAX_LIMBS).rev() {
            let limb = self.limbs.get(index).copied().unwrap_or(0);
            if limb != 0 {
                return index
                    .saturating_mul(64)
                    .saturating_add(64usize.saturating_sub(limb.leading_zeros() as usize));
            }
        }
        0
    }

    /// Bit `index`, counting from the least significant.
    fn bit(&self, index: usize) -> bool {
        let shift = u32::try_from(index % 64).unwrap_or(0);
        self.limbs
            .get(index / 64)
            .is_some_and(|limb| (limb >> shift) & 1 == 1)
    }

    /// Whether this is strictly less than `other`, comparing whole arrays.
    ///
    /// The comparison walks all [`MAX_LIMBS`] limbs rather than the used length, so it does not
    /// depend on either value's `length` field being normalised.
    fn less_than(&self, other: &Self) -> bool {
        for index in (0..MAX_LIMBS).rev() {
            let mine = self.limbs.get(index).copied().unwrap_or(0);
            let theirs = other.limbs.get(index).copied().unwrap_or(0);
            if mine != theirs {
                return mine < theirs;
            }
        }
        false
    }
}

/// `a * b + c + d`, as `(high, low)`.
///
/// The bound is what makes every carry in this module fit: with all four at `u64::MAX` the sum is
/// `(2^64 - 1)^2 + 2 * (2^64 - 1) = 2^128 - 1`, so a `u128` holds it exactly and no intermediate
/// wraps. The wrapping spellings are therefore descriptions of arithmetic that cannot wrap, and
/// not permissions for it to.
fn multiply_accumulate(a: u64, b: u64, c: u64, d: u64) -> (u64, u64) {
    let wide = u128::from(a)
        .wrapping_mul(u128::from(b))
        .wrapping_add(u128::from(c))
        .wrapping_add(u128::from(d));
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the two halves of a 128-bit product are wanted, and each is taken exactly"
    )]
    {
        ((wide >> 64) as u64, wide as u64)
    }
}

/// An odd modulus, with the constant Montgomery reduction needs.
struct Modulus {
    value: Integer,
    /// `-n^-1 mod 2^64`, the quantity each reduction step multiplies by.
    inverse: u64,
    /// How many limbs the modulus occupies, which is every loop's trip count below.
    limbs: usize,
}

impl Modulus {
    /// An odd modulus greater than one, or `None`.
    ///
    /// Montgomery reduction needs `n` odd — it works modulo `2^64` in each step, and an even `n`
    /// has no inverse there. RFC 8017 section 3.1 makes an RSA modulus a product of odd primes, so this
    /// refuses nothing a real key could be.
    fn new(value: &Integer) -> Option<Self> {
        let low = value.limbs.first().copied().unwrap_or(0);
        if low & 1 == 0 || value.bits() < 2 {
            return None;
        }
        // Newton's iteration for the inverse modulo `2^64`: `x` doubles its correct bits each
        // step, so five steps take one correct bit (odd numbers are their own inverse mod 2) to
        // sixty-four. Every operation is deliberately modulo `2^64`, which is what wrapping is.
        let mut inverse = 1u64;
        for _ in 0..6 {
            inverse = inverse.wrapping_mul(2u64.wrapping_sub(low.wrapping_mul(inverse)));
        }
        Some(Self {
            limbs: value.bits().div_ceil(64).min(MAX_LIMBS),
            inverse: inverse.wrapping_neg(),
            value: *value,
        })
    }

    /// `a * b * R^-1 mod n`, where `R` is `2^(64 * limbs)` — one Montgomery multiplication.
    ///
    /// The coarsely-integrated operand scanning form: each pass over `b`'s limbs multiplies,
    /// accumulates and reduces by one limb, so the accumulator never grows past `limbs + 2` and
    /// no division is performed anywhere in this module.
    fn multiply(&self, a: &Integer, b: &Integer) -> Integer {
        let limbs = self.limbs;
        let mut t = [0u64; MAX_LIMBS + 2];
        for index in 0..limbs {
            let multiplier = b.limbs.get(index).copied().unwrap_or(0);
            let mut carry = 0u64;
            for place in 0..limbs {
                let (high, low) = multiply_accumulate(
                    a.limbs.get(place).copied().unwrap_or(0),
                    multiplier,
                    t.get(place).copied().unwrap_or(0),
                    carry,
                );
                if let Some(slot) = t.get_mut(place) {
                    *slot = low;
                }
                carry = high;
            }
            let (sum, overflow) = t.get(limbs).copied().unwrap_or(0).overflowing_add(carry);
            if let Some(slot) = t.get_mut(limbs) {
                *slot = sum;
            }
            if let Some(slot) = t.get_mut(limbs.saturating_add(1)) {
                *slot = u64::from(overflow);
            }
            // One limb of the reduction: adding `m * n` clears `t[0]`, which is what makes the
            // whole accumulator shift down by a limb without a division.
            let m = t.first().copied().unwrap_or(0).wrapping_mul(self.inverse);
            let (mut carry, _) = multiply_accumulate(
                m,
                self.value.limbs.first().copied().unwrap_or(0),
                t.first().copied().unwrap_or(0),
                0,
            );
            for place in 1..limbs {
                let (high, low) = multiply_accumulate(
                    m,
                    self.value.limbs.get(place).copied().unwrap_or(0),
                    t.get(place).copied().unwrap_or(0),
                    carry,
                );
                if let Some(slot) = t.get_mut(place.saturating_sub(1)) {
                    *slot = low;
                }
                carry = high;
            }
            let (sum, overflow) = t.get(limbs).copied().unwrap_or(0).overflowing_add(carry);
            if let Some(slot) = t.get_mut(limbs.saturating_sub(1)) {
                *slot = sum;
            }
            let top = t
                .get(limbs.saturating_add(1))
                .copied()
                .unwrap_or(0)
                .wrapping_add(u64::from(overflow));
            if let Some(slot) = t.get_mut(limbs) {
                *slot = top;
            }
        }
        let mut out = Integer::zero();
        for place in 0..limbs {
            if let (Some(slot), Some(&value)) = (out.limbs.get_mut(place), t.get(place)) {
                *slot = value;
            }
        }
        // The result is below `2 * n`, so at most one subtraction brings it below `n`. The extra
        // limb `t[limbs]` carries the case where it does not fit in `limbs` limbs at all.
        if t.get(limbs).copied().unwrap_or(0) != 0 || !out.less_than(&self.value) {
            self.subtract(&mut out);
        }
        out
    }

    /// `value -= n`, in place, wrapping is not reachable because the caller has compared first.
    fn subtract(&self, value: &mut Integer) {
        let mut borrow = 0u64;
        for place in 0..self.limbs {
            let mine = value.limbs.get(place).copied().unwrap_or(0);
            let theirs = self.value.limbs.get(place).copied().unwrap_or(0);
            let (first, under) = mine.overflowing_sub(theirs);
            let (result, again) = first.overflowing_sub(borrow);
            if let Some(slot) = value.limbs.get_mut(place) {
                *slot = result;
            }
            borrow = u64::from(under || again);
        }
    }

    /// `value = 2 * value mod n`, in place.
    ///
    /// The one operation that needs no multiplication, and the whole of how a value enters the
    /// Montgomery domain: doubling `64 * limbs` times multiplies by `R`.
    fn double(&self, value: &mut Integer) {
        let mut carry = 0u64;
        for place in 0..self.limbs {
            let limb = value.limbs.get(place).copied().unwrap_or(0);
            if let Some(slot) = value.limbs.get_mut(place) {
                *slot = (limb << 1) | carry;
            }
            carry = limb >> 63;
        }
        // A carry out of the top means the doubled value is at least `2^(64 * limbs)`, which is
        // larger than `n`, so the subtraction is owed whatever the comparison says.
        if carry != 0 || !value.less_than(&self.value) {
            self.subtract(value);
        }
    }

    /// `value * R mod n` — the Montgomery form of a value already reduced modulo `n`.
    fn to_montgomery(&self, value: &Integer) -> Integer {
        let mut out = *value;
        for _ in 0..self.limbs.saturating_mul(64) {
            self.double(&mut out);
        }
        out
    }
}

/// `base^exponent mod n`, left to right over the exponent's bits.
///
/// Square-and-multiply in the Montgomery domain, so the only reduction anywhere is
/// [`Modulus::multiply`]'s. The trip count is the exponent's bit length, which [`verify`] has
/// already bounded by [`MAX_EXPONENT_BITS`].
fn modpow(base: &Integer, exponent: &Integer, modulus: &Modulus) -> Integer {
    let mut one = Integer::zero();
    if let Some(slot) = one.limbs.first_mut() {
        *slot = 1;
    }
    let mut accumulator = modulus.to_montgomery(&one);
    let multiplier = modulus.to_montgomery(base);
    let bits = exponent.bits();
    for index in (0..bits).rev() {
        accumulator = modulus.multiply(&accumulator, &accumulator);
        if exponent.bit(index) {
            accumulator = modulus.multiply(&accumulator, &multiplier);
        }
    }
    // Multiplying by one in the Montgomery domain is the conversion back out of it.
    modulus.multiply(&accumulator, &one)
}

#[cfg(test)]
mod tests {
    use super::{
        Integer, MAX_EXPONENT_BITS, MAX_MODULUS_BITS, Modulus, Pkcs1Error, PublicKey, encode,
        modpow, verify,
    };
    use crate::cms::Digest;

    /// A 2048-bit RSA modulus, and below it a signature made with the matching private key.
    ///
    /// **A test vector, generated once and pasted in — not an oracle.** `CLAUDE.md` principle 5
    /// forbids treating another implementation's output as the definition of correct, and this is
    /// not that: RFC 8017 defines `s^e mod n` and section 9.2's block, and what a vector does is pin that
    /// *these* two hundred lines compute them. Nothing about the vector decides what is correct;
    /// it decides only whether the arithmetic here is the arithmetic the RFC states, which is a
    /// question a hand-checked small case cannot settle at 2048 bits.
    const MODULUS: &str = "\
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

    /// `a^b mod m` on numbers small enough to check by hand or with a calculator.
    ///
    /// Every modulus here is odd, which is not a convenience: [`Modulus::new`] refuses an even one
    /// because Montgomery reduction has no inverse modulo `2^64` for it, and RFC 8017 section 3.1 makes
    /// an RSA modulus a product of odd primes. The first draft of this test used 1000 and the
    /// refusal is what it found.
    #[test]
    fn modular_exponentiation_agrees_with_arithmetic() {
        let cases: [(u64, u64, u64, u64); 5] = [
            (2, 10, 1001, 23),
            (3, 0, 7, 1),
            (5, 3, 13, 8),
            (7, 65537, 9, 4),
            (123_456_789, 3, 1_000_000_007, 350_575_129),
        ];
        for (base, exponent, modulus, expected) in cases {
            let reduced =
                Modulus::new(&Integer::from_be_bytes(&modulus.to_be_bytes()).expect("small"))
                    .expect("odd");
            let result = modpow(
                &Integer::from_be_bytes(&base.to_be_bytes()).expect("small"),
                &Integer::from_be_bytes(&exponent.to_be_bytes()).expect("small"),
                &reduced,
            );
            assert_eq!(
                result.be_bytes(8),
                expected.to_be_bytes(),
                "{base}^{exponent} mod {modulus}"
            );
        }
    }

    /// `x^3` reached two ways at 2048 bits, which is where a carry bug lives if there is one.
    #[test]
    fn a_wide_modulus_exponentiates_consistently() {
        let bytes = hex(MODULUS);
        let modulus =
            Modulus::new(&Integer::from_be_bytes(&bytes).expect("fits")).expect("odd and large");
        let mut base = Integer::from_be_bytes(&[0x9Au8; 250]).expect("fits");
        while !base.less_than(&modulus.value) {
            modulus.subtract(&mut base);
        }
        let one = Integer::from_be_bytes(&[1]).expect("small");
        let squared = modpow(
            &base,
            &Integer::from_be_bytes(&[2]).expect("small"),
            &modulus,
        );
        let cubed = modpow(
            &base,
            &Integer::from_be_bytes(&[3]).expect("small"),
            &modulus,
        );
        let by_hand = modulus.multiply(
            &modulus.multiply(
                &modulus.to_montgomery(&squared),
                &modulus.to_montgomery(&base),
            ),
            &one,
        );
        assert_eq!(
            cubed.be_bytes(256),
            by_hand.be_bytes(256),
            "x^3 is x^2 times x, however it is reached"
        );
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
