//! Fixed-size modular arithmetic over numbers a stranger wrote into a file.
//!
//! **This module knows nothing about signatures.** It is the integer arithmetic two of them need:
//! [`crate::pkcs1`]'s `s^e mod n` (RFC 8017 section 5.2.2) and [`crate::dsa`]'s `g^u1 y^u2 mod p mod q`
//! (FIPS 186-4 section 4.7). It lived inside `pkcs1` until the four-hundred-and-seventy-ninth
//! session and moved out when a second caller arrived, rather than being reached into from one
//! module by another — ADR 0314.
//!
//! # What the shape buys, and it is a security property rather than a style
//!
//! Every value is [`MAX_LIMBS`] limbs whatever the file says, so **every loop's trip count is a
//! constant of this module** and none is a number out of the document. `CLAUDE.md` principle 3
//! asks exactly that of arithmetic over untrusted input: memory safety does not bound work, and a
//! `Vec`-backed integer would let a modulus in a hostile file choose how long a verification takes.
//! What it costs is one kilobyte of stack per integer.
//!
//! **There is no division anywhere in this module.** Entering the Montgomery domain is repeated
//! doubling, every reduction is one pass of the multiplication itself, and [`Modulus::reduce`] —
//! the one operation that looks like a division — is a shift-and-subtract over the value's bits.
//!
//! # There is no secret here
//!
//! Nothing in this module runs in constant time and nothing needs to: every number it touches is
//! public — a modulus, a public key, a signature value, a digest — and all of them came out of a
//! file anyone can read. ADR 0229 has the argument in full; it is why this arithmetic is in the
//! tree at all rather than taken from a dependency.

/// The widest number this module holds, in bits.
///
/// Twice Table 260's largest key ("Up to 4096-bit (PDF 1.5)"), so that a key beyond the standard
/// is reported by name rather than refused by running out of room, and one beyond this is refused
/// by name too. Both callers restate it as their own budget: [`crate::pkcs1::MAX_MODULUS_BITS`]
/// and [`crate::dsa::MAX_MODULUS_BITS`].
pub(crate) const MAX_BITS: usize = 8192;

/// How many 64-bit limbs [`MAX_BITS`] is.
pub(crate) const MAX_LIMBS: usize = MAX_BITS / 64;

/// The significant bits of a big-endian byte string, ignoring leading zero octets.
///
/// This is what a person means by "a 2048-bit key": the modulus's own width, not the length of the
/// string an encoder wrote it in — an X.509 `INTEGER` carries a leading `0x00` whenever the high
/// bit is set, and that octet is spelling rather than value.
pub(crate) fn significant_bits(bytes: &[u8]) -> usize {
    let leading = bytes
        .iter()
        .take_while(|&&byte| byte == 0)
        .count()
        .min(bytes.len());
    let Some(rest) = bytes.get(leading..) else {
        return 0;
    };
    let Some(&first) = rest.first() else {
        return 0;
    };
    rest.len()
        .saturating_sub(1)
        .saturating_mul(8)
        .saturating_add(8usize.saturating_sub(first.leading_zeros() as usize))
}

/// A big unsigned integer of at most [`MAX_LIMBS`] limbs, least significant first.
#[derive(Clone, Copy)]
pub(crate) struct Integer {
    pub(crate) limbs: [u64; MAX_LIMBS],
}

impl std::fmt::Debug for Integer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Integer({} bits)", self.bits())
    }
}

impl Integer {
    /// Zero.
    pub(crate) fn zero() -> Self {
        Self {
            limbs: [0; MAX_LIMBS],
        }
    }

    /// One.
    pub(crate) fn one() -> Self {
        let mut out = Self::zero();
        if let Some(slot) = out.limbs.first_mut() {
            *slot = 1;
        }
        out
    }

    /// A big-endian byte string as an integer, or `None` where it needs more than [`MAX_LIMBS`].
    ///
    /// Leading zero octets are ignored, which is what makes an X.509 `INTEGER`'s sign octet
    /// harmless here: RFC 5280's serial numbers and moduli are written with a leading `0x00` when
    /// the high bit is set, and the value is the same number either way.
    pub(crate) fn from_be_bytes(bytes: &[u8]) -> Option<Self> {
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
    pub(crate) fn be_bytes(&self, length: usize) -> Vec<u8> {
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
    pub(crate) fn is_zero(&self) -> bool {
        self.limbs.iter().all(|&limb| limb == 0)
    }

    /// The number of significant bits.
    pub(crate) fn bits(&self) -> usize {
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
    pub(crate) fn bit(&self, index: usize) -> bool {
        let shift = u32::try_from(index % 64).unwrap_or(0);
        self.limbs
            .get(index / 64)
            .is_some_and(|limb| (limb >> shift) & 1 == 1)
    }

    /// Whether this is strictly less than `other`, comparing whole arrays.
    ///
    /// The comparison walks all [`MAX_LIMBS`] limbs rather than the used length, so it does not
    /// depend on either value's width being normalised.
    pub(crate) fn less_than(&self, other: &Self) -> bool {
        for index in (0..MAX_LIMBS).rev() {
            let mine = self.limbs.get(index).copied().unwrap_or(0);
            let theirs = other.limbs.get(index).copied().unwrap_or(0);
            if mine != theirs {
                return mine < theirs;
            }
        }
        false
    }

    /// Whether the two are the same number.
    pub(crate) fn equals(&self, other: &Self) -> bool {
        self.limbs == other.limbs
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
pub(crate) struct Modulus {
    pub(crate) value: Integer,
    /// `-n^-1 mod 2^64`, the quantity each reduction step multiplies by.
    inverse: u64,
    /// How many limbs the modulus occupies, which is every loop's trip count below.
    limbs: usize,
}

impl Modulus {
    /// An odd modulus greater than one, or `None`.
    ///
    /// Montgomery reduction needs `n` odd — it works modulo `2^64` in each step, and an even `n`
    /// has no inverse there. RFC 8017 section 3.1 makes an RSA modulus a product of odd primes and
    /// FIPS 186-4 section 4.1 makes DSA's `p` and `q` prime, so this refuses nothing a real key
    /// could be; both callers report the refusal by name rather than working around it.
    pub(crate) fn new(value: &Integer) -> Option<Self> {
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
    pub(crate) fn multiply(&self, a: &Integer, b: &Integer) -> Integer {
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

    /// `value -= n`, in place; wrapping is not reachable because the caller has compared first.
    pub(crate) fn subtract(&self, value: &mut Integer) {
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

    /// `value = 2 * value mod n`, in place, for a `value` already below `n`.
    ///
    /// The one operation that needs no multiplication, and the whole of how a value enters the
    /// Montgomery domain: doubling `64 * limbs` times multiplies by `R`.
    pub(crate) fn double(&self, value: &mut Integer) {
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

    /// `value = value + 1 mod n`, in place, for a `value` already below `n`.
    ///
    /// One conditional subtraction is enough: `value < n` makes `value + 1` at most `n`.
    fn increment(&self, value: &mut Integer) {
        let mut carry = 1u64;
        for place in 0..self.limbs {
            let limb = value.limbs.get(place).copied().unwrap_or(0);
            let (sum, overflow) = limb.overflowing_add(carry);
            if let Some(slot) = value.limbs.get_mut(place) {
                *slot = sum;
            }
            carry = u64::from(overflow);
            if carry == 0 {
                break;
            }
        }
        if carry != 0 || !value.less_than(&self.value) {
            self.subtract(value);
        }
    }

    /// `value * R mod n` — the Montgomery form of a value already reduced modulo `n`.
    pub(crate) fn to_montgomery(&self, value: &Integer) -> Integer {
        let mut out = *value;
        for _ in 0..self.limbs.saturating_mul(64) {
            self.double(&mut out);
        }
        out
    }

    /// `value mod n`, for a `value` of any size — the one place a division would ordinarily be.
    ///
    /// Shift-and-subtract from the top bit down, which is the schoolbook long division with the
    /// quotient thrown away: the running remainder is always below `n`, so doubling it and adding
    /// the next bit needs at most one subtraction each. FIPS 186-4 section 4.7 needs exactly this
    /// once — `v = ((g^u1 y^u2) mod p) mod q` reduces a `p`-sized number by a much smaller `q` —
    /// and the cost is one pass per bit of `value`, on a path that runs once per signature.
    pub(crate) fn reduce(&self, value: &Integer) -> Integer {
        let mut out = Integer::zero();
        for index in (0..value.bits()).rev() {
            self.double(&mut out);
            if value.bit(index) {
                self.increment(&mut out);
            }
        }
        out
    }

    /// `a * b mod n` for two values already below `n`.
    ///
    /// Montgomery multiplication divides by `R` once, so putting one operand *into* the domain
    /// first cancels it exactly: `MontMul(aR, b) = a b R R^-1 = a b`.
    pub(crate) fn multiply_reduced(&self, a: &Integer, b: &Integer) -> Integer {
        self.multiply(&self.to_montgomery(a), b)
    }

    /// `n - 2`, which is the exponent Fermat's little theorem inverts by.
    ///
    /// `n` is odd and at least three here — [`Modulus::new`] refuses anything smaller — so the
    /// subtraction cannot borrow past the top.
    fn minus_two(&self) -> Integer {
        let mut out = self.value;
        let mut borrow = 2u64;
        for place in 0..self.limbs {
            let limb = out.limbs.get(place).copied().unwrap_or(0);
            let (result, under) = limb.overflowing_sub(borrow);
            if let Some(slot) = out.limbs.get_mut(place) {
                *slot = result;
            }
            borrow = u64::from(under);
            if borrow == 0 {
                break;
            }
        }
        out
    }

    /// `value^-1 mod n`, **where `n` is prime** — Fermat's little theorem, `value^(n-2)`.
    ///
    /// FIPS 186-4 Appendix C.1 states the extended Euclidean algorithm for this and admits "an
    /// algorithm that produces an equivalent result"; this is one, and it is the one that needs no
    /// division. What it costs is a primality assumption, and that assumption is safe *here* in
    /// the only direction that matters: `q` is prime in any real DSA key, and for a `q` that is
    /// not, `value^(q-2)` is simply not the inverse, `v` is not `r'`, and the signature does not
    /// verify. The mistake is closed rather than open.
    ///
    /// `value` must already be below `n`, which is [`modpow`]'s precondition and not a new one.
    ///
    /// `None` for zero, which has no inverse under any modulus.
    pub(crate) fn invert(&self, value: &Integer) -> Option<Integer> {
        if value.is_zero() {
            return None;
        }
        Some(modpow(value, &self.minus_two(), self))
    }
}

/// `base^exponent mod n`, left to right over the exponent's bits.
///
/// Square-and-multiply in the Montgomery domain, so the only reduction anywhere is
/// [`Modulus::multiply`]'s. The trip count is the exponent's bit length, which every caller bounds
/// before arriving here — an unbounded exponent is unbounded work over a number a stranger chose.
/// `base` must already be below `n`; [`Modulus::reduce`] is how a caller makes sure of it.
pub(crate) fn modpow(base: &Integer, exponent: &Integer, modulus: &Modulus) -> Integer {
    let one = Integer::one();
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
    use super::{Integer, MAX_BITS, Modulus, modpow, significant_bits};

    /// A 2048-bit modulus, borrowed from `pkcs1`'s test vector so that the wide cases below run on
    /// a number a real key uses rather than on one this test invented.
    fn wide_modulus() -> Vec<u8> {
        crate::pkcs1::tests::hex(crate::pkcs1::tests::MODULUS)
    }

    /// `a^b mod m` on numbers small enough to check by hand or with a calculator.
    ///
    /// Every modulus here is odd, which is not a convenience: [`Modulus::new`] refuses an even one
    /// because Montgomery reduction has no inverse modulo `2^64` for it. The first draft of this
    /// test used 1000 and the refusal is what it found.
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
        let bytes = wide_modulus();
        let modulus =
            Modulus::new(&Integer::from_be_bytes(&bytes).expect("fits")).expect("odd and large");
        let mut base = Integer::from_be_bytes(&[0x9Au8; 250]).expect("fits");
        while !base.less_than(&modulus.value) {
            modulus.subtract(&mut base);
        }
        let one = Integer::one();
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

    /// [`Modulus::reduce`] against arithmetic anyone can check, including a value far above `n`.
    #[test]
    fn a_value_larger_than_the_modulus_is_reduced_by_it() {
        let modulus = Modulus::new(&Integer::from_be_bytes(&1001u64.to_be_bytes()).expect("small"))
            .expect("odd");
        for value in [0u64, 1, 1000, 1001, 1002, 123_456_789, u64::MAX] {
            let reduced = modulus.reduce(&Integer::from_be_bytes(&value.to_be_bytes()).expect(""));
            assert_eq!(
                reduced.be_bytes(8),
                (value % 1001).to_be_bytes(),
                "{value} mod 1001"
            );
        }
        // And a value far wider than the modulus: 250 octets of `0xFF` is `2^2000 - 1`, so the
        // shift-and-subtract runs two thousand times. The independent answer comes from the
        // *other* operation in this module — `2^2000 mod 1001`, less one — so agreement is
        // between two constructions rather than between this one and itself.
        let reduced = modulus.reduce(&Integer::from_be_bytes(&[0xFFu8; 250]).expect("fits"));
        assert!(reduced.less_than(&modulus.value), "a remainder is below n");
        let power = modpow(
            &Integer::from_be_bytes(&[2]).expect("small"),
            &Integer::from_be_bytes(&2000u64.to_be_bytes()).expect("small"),
            &modulus,
        );
        let as_u64 = |value: &Integer| {
            u64::from_be_bytes(
                value
                    .be_bytes(8)
                    .try_into()
                    .expect("eight octets by construction"),
            )
        };
        assert_eq!(
            as_u64(&reduced),
            (as_u64(&power) + 1000) % 1001,
            "(2^2000 - 1) mod 1001"
        );
    }

    /// Fermat's inverse, checked by multiplying back — and refused for zero.
    #[test]
    fn an_inverse_multiplies_back_to_one() {
        // 1009 is prime, which is what Fermat's little theorem needs.
        let modulus = Modulus::new(&Integer::from_be_bytes(&1009u64.to_be_bytes()).expect("small"))
            .expect("odd");
        for value in [1u64, 2, 3, 500, 1008] {
            let integer = Integer::from_be_bytes(&value.to_be_bytes()).expect("small");
            let inverse = modulus.invert(&integer).expect("non-zero");
            let product = modulus.multiply_reduced(&integer, &inverse);
            assert_eq!(
                product.be_bytes(8),
                1u64.to_be_bytes(),
                "{value} times its inverse mod 1009"
            );
        }
        assert!(
            modulus.invert(&Integer::zero()).is_none(),
            "zero has no inverse"
        );
    }

    /// A modulus wider than the array cannot be built, and an even one cannot either.
    #[test]
    fn the_bound_and_the_oddness_are_both_refusals() {
        assert!(Integer::from_be_bytes(&vec![0xFFu8; (MAX_BITS / 8) + 1]).is_none());
        assert!(Integer::from_be_bytes(&vec![0xFFu8; MAX_BITS / 8]).is_some());
        let even = Integer::from_be_bytes(&[0x10, 0x00]).expect("small");
        assert!(Modulus::new(&even).is_none(), "Montgomery needs an odd n");
        assert!(Modulus::new(&Integer::one()).is_none(), "and n > 1");
    }

    /// A width is the number's, not its encoding's.
    #[test]
    fn significant_bits_ignore_the_encodings_leading_zeros() {
        assert_eq!(significant_bits(&[]), 0);
        assert_eq!(significant_bits(&[0x00]), 0);
        assert_eq!(significant_bits(&[0x00, 0xFF, 0xFF]), 16);
        assert_eq!(significant_bits(&[0x01, 0x00]), 9);
        assert_eq!(significant_bits(&wide_modulus()), 2048);
    }
}
