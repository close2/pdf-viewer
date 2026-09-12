//! Modular arithmetic over numbers a stranger wrote into a file — `crypto-bigint`, behind
//! this crate's own budgets.
//!
//! **This module knows nothing about signatures.** It is the integer arithmetic two of them need:
//! [`crate::pkcs1`]'s `s^e mod n` (RFC 8017 section 5.2.2) and [`crate::dsa`]'s `g^u1 y^u2 mod p mod q`
//! (FIPS 186-4 section 4.7). It lived inside `pkcs1` until the four-hundred-and-seventy-ninth
//! session and moved out when a second caller arrived (ADR 0314).
//!
//! # The arithmetic is a dependency's, and that is a decision
//!
//! Until the four-hundred-and-ninety-sixth session this module *was* the arithmetic: a fixed-size
//! limb array, Montgomery multiplication and square-and-multiply, written in tree on ADR 0229's
//! argument that a verification has no secret to leak. The project owner decided otherwise after
//! reading ADR 0314, and ADR 0331 carries the reasoning: `RustCrypto`'s `crypto-bigint` is the
//! reviewed implementation of exactly these operations, it sits on the same supplier line as every
//! cipher and digest this tree already takes, and what review buys a *verifier* is not
//! side-channel resistance but a second set of eyes on carry propagation — the one class of defect
//! a wrong-arithmetic test vector can miss and a forgery can hit.
//!
//! What stays this module's is everything that is a statement about a PDF file rather than about
//! mathematics: the conversion between the file's big-endian octet strings and integers, the
//! [`MAX_BITS`] budget both callers restate, and the refusal shapes (an even or trivial modulus,
//! a number too wide to hold). Every multiplication, exponentiation, reduction and inversion below
//! is one call into `crypto-bigint`.
//!
//! # There is no secret here
//!
//! Nothing in this module needs to run in constant time: every number it touches is public — a
//! modulus, a public key, a signature value, a digest — and all of them came out of a file anyone
//! can read (ADR 0229's argument, which survives the port). The `_vartime` spellings below say so
//! deliberately: `crypto-bigint` offers both, and taking the constant-time form would claim a
//! property nothing here relies on.

use crypto_bigint::modular::{BoxedMontyForm, BoxedMontyParams};
use crypto_bigint::{BoxedUint, Odd, Resize};

/// The widest number this module holds, in bits.
///
/// Twice Table 260's largest key ("Up to 4096-bit (PDF 1.5)"), so that a key beyond the standard
/// is reported by name rather than refused by running out of room, and one beyond this is refused
/// by name too. Both callers restate it as their own budget: [`crate::pkcs1::MAX_MODULUS_BITS`]
/// and [`crate::dsa::MAX_MODULUS_BITS`]. The budget is this program's, not the dependency's:
/// `crypto-bigint`'s heap-allocated integers would hold whatever a hostile file wrote, and the
/// point of the bound is that work stays a constant of this module rather than a number out of
/// the document.
pub(crate) const MAX_BITS: usize = 8192;

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

/// A big unsigned integer of at most [`MAX_BITS`] bits.
///
/// A [`BoxedUint`] whose precision was chosen from the encoded value's own significant octets, so
/// a 2048-bit key computes at 2048 bits rather than at the budget's ceiling. The precision is a
/// public fact about a public number; see the module documentation for why nothing here is
/// constant-time.
#[derive(Clone)]
pub(crate) struct Integer {
    value: BoxedUint,
}

impl std::fmt::Debug for Integer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Integer({} bits)", self.bits())
    }
}

impl Integer {
    /// Zero.
    #[cfg(test)]
    pub(crate) fn zero() -> Self {
        Self {
            value: BoxedUint::zero_with_precision(64),
        }
    }

    /// A big-endian byte string as an integer, or `None` where it needs more than [`MAX_BITS`].
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
        if significant.len() > MAX_BITS / 8 {
            return None;
        }
        let precision = u32::try_from(significant.len())
            .ok()?
            .saturating_mul(8)
            .max(64);
        // Cannot fail: the slice fits the precision by construction, and `from_be_slice` rounds
        // the precision up to a whole limb itself.
        BoxedUint::from_be_slice(significant, precision)
            .ok()
            .map(|value| Self { value })
    }

    /// The integer as exactly `length` big-endian octets, zero-padded on the left.
    ///
    /// Truncating on the left where the value needs more, which cannot happen for a value reduced
    /// modulo an `n` of that many octets — and which would produce a block that fails the
    /// comparison rather than one that passes it.
    pub(crate) fn be_bytes(&self, length: usize) -> Vec<u8> {
        let raw = self.value.to_be_bytes();
        let mut out = vec![0u8; length];
        let take = raw.len().min(length);
        let source = raw.get(raw.len().saturating_sub(take)..).unwrap_or(&[]);
        if let Some(slot) = out.get_mut(length.saturating_sub(take)..) {
            slot.copy_from_slice(source);
        }
        out
    }

    /// Whether the value is zero.
    pub(crate) fn is_zero(&self) -> bool {
        self.value.is_zero().into()
    }

    /// The number of significant bits.
    pub(crate) fn bits(&self) -> usize {
        usize::try_from(self.value.bits()).unwrap_or(usize::MAX)
    }

    /// Whether this is strictly less than `other`.
    ///
    /// `crypto-bigint` compares across differing precisions, so neither value's width needs
    /// normalising first.
    pub(crate) fn less_than(&self, other: &Self) -> bool {
        self.value < other.value
    }

    /// Whether the two are the same number, whatever their precisions.
    pub(crate) fn equals(&self, other: &Self) -> bool {
        self.value.cmp_vartime(&other.value) == std::cmp::Ordering::Equal
    }

    /// `self >> bits`, losing the low bits — how a digest wider than a DSA `q` is truncated.
    ///
    /// The unbounded form, so a shift as wide as the value produces zero rather than wrapping the
    /// shift amount the way `wrapping_shr` would.
    pub(crate) fn shifted_right(&self, bits: usize) -> Self {
        Self {
            value: self
                .value
                .unbounded_shr_vartime(u32::try_from(bits).unwrap_or(u32::MAX)),
        }
    }
}

/// An odd modulus greater than one, holding the Montgomery parameters `crypto-bigint` computes
/// once per modulus.
pub(crate) struct Modulus {
    /// The modulus itself, which callers compare signature values against.
    pub(crate) value: Integer,
    /// The precomputed Montgomery form parameters, shared by every operation below.
    params: BoxedMontyParams,
}

impl Modulus {
    /// An odd modulus greater than one, or `None`.
    ///
    /// Montgomery reduction needs `n` odd — it works modulo a power of two, where an even `n` has
    /// no inverse. RFC 8017 section 3.1 makes an RSA modulus a product of odd primes and FIPS
    /// 186-4 section 4.1 makes DSA's `p` and `q` prime, so this refuses nothing a real key could
    /// be; both callers report the refusal by name rather than working around it.
    pub(crate) fn new(value: &Integer) -> Option<Self> {
        if value.bits() < 2 {
            return None;
        }
        let odd = Odd::new(value.value.clone()).into_option()?;
        Some(Self {
            // `new_vartime` rather than `new`: the modulus is public (module documentation).
            params: BoxedMontyParams::new_vartime(odd),
            value: value.clone(),
        })
    }

    /// The modulus's precision, which every Montgomery-form operand must share.
    fn precision(&self) -> u32 {
        self.params.bits_precision()
    }

    /// `value` at the modulus's precision, as the Montgomery form the parameters expect.
    ///
    /// Callers hand values already below `n`, so the narrowing resize drops nothing; a caller
    /// that broke that contract is caught by `try_resize` and reduced first, which is the honest
    /// repair rather than a silent wrong answer.
    fn form(&self, value: &Integer) -> BoxedMontyForm {
        let at_precision = Resize::try_resize(&value.value, self.precision())
            .unwrap_or_else(|| self.reduce(value).value.resize(self.precision()));
        BoxedMontyForm::new(at_precision, &self.params)
    }

    /// `value mod n`, for a `value` of any size.
    ///
    /// FIPS 186-4 section 4.7 needs exactly this once — `v = ((g^u1 y^u2) mod p) mod q` reduces a
    /// `p`-sized number by a much smaller `q` — and both callers use it to bring a file's numbers
    /// below their modulus before exponentiating.
    pub(crate) fn reduce(&self, value: &Integer) -> Integer {
        let divisor = self.params.modulus().as_nz_ref();
        Integer {
            value: value.value.rem_vartime(divisor),
        }
    }

    /// `a * b mod n` for two values already below `n`.
    pub(crate) fn multiply_reduced(&self, a: &Integer, b: &Integer) -> Integer {
        // Montgomery-form multiplication cannot overflow — the product is reduced modulo `n` by
        // construction — so the operator the lint sees is not integer arithmetic at all.
        #[expect(
            clippy::arithmetic_side_effects,
            reason = "BoxedMontyForm's Mul is a modular multiplication and cannot wrap"
        )]
        Integer {
            value: (self.form(a) * self.form(b)).retrieve(),
        }
    }

    /// `value^-1 mod n`, or `None` where no inverse exists.
    ///
    /// FIPS 186-4 Appendix C.1 states the extended Euclidean algorithm and admits "an algorithm
    /// that produces an equivalent result"; `crypto-bigint`'s inversion is one. Where the previous
    /// in-tree arithmetic answered a non-invertible value with a number that was simply not the
    /// inverse — safe, because `v` then fails to equal `r'` — this answers `None`, and the one
    /// caller treats that as "does not verify", which is the same verdict said sooner.
    pub(crate) fn invert(&self, value: &Integer) -> Option<Integer> {
        self.form(value)
            .invert_vartime()
            .into_option()
            .map(|inverse| Integer {
                value: inverse.retrieve(),
            })
    }
}

/// `base^exponent mod n`.
///
/// One `crypto-bigint` exponentiation. The trip count is the exponent's bit length, which every
/// caller bounds before arriving here — an unbounded exponent is unbounded work over a number a
/// stranger chose. `base` need not be below `n`; [`Modulus::form`] reduces one that is not.
pub(crate) fn modpow(base: &Integer, exponent: &Integer, modulus: &Modulus) -> Integer {
    Integer {
        value: modulus.form(base).pow(&exponent.value).retrieve(),
    }
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
    /// because Montgomery reduction has no inverse modulo a power of two for it. The first draft
    /// of this test used 1000 and the refusal is what it found.
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

    /// `x^3` reached two ways at 2048 bits, which is where a mapping bug would live if there were
    /// one: the cube by one exponentiation against the square multiplied back by the base.
    #[test]
    fn a_wide_modulus_exponentiates_consistently() {
        let bytes = wide_modulus();
        let modulus =
            Modulus::new(&Integer::from_be_bytes(&bytes).expect("fits")).expect("odd and large");
        let base = modulus.reduce(&Integer::from_be_bytes(&[0x9Au8; 250]).expect("fits"));
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
        let by_hand = modulus.multiply_reduced(&squared, &base);
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
        // And a value far wider than the modulus: 250 octets of `0xFF` is `2^2000 - 1`. The
        // independent answer comes from the *other* operation in this module — `2^2000 mod 1001`,
        // less one — so agreement is between two constructions rather than between this one and
        // itself.
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

    /// An inverse, checked by multiplying back — and refused for zero.
    #[test]
    fn an_inverse_multiplies_back_to_one() {
        // 1009 is prime, so every non-zero residue has an inverse.
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

    /// A modulus wider than the budget cannot be built, and an even one cannot either.
    #[test]
    fn the_bound_and_the_oddness_are_both_refusals() {
        assert!(Integer::from_be_bytes(&vec![0xFFu8; (MAX_BITS / 8) + 1]).is_none());
        assert!(Integer::from_be_bytes(&vec![0xFFu8; MAX_BITS / 8]).is_some());
        let even = Integer::from_be_bytes(&[0x10, 0x00]).expect("small");
        assert!(Modulus::new(&even).is_none(), "Montgomery needs an odd n");
        assert!(
            Modulus::new(&Integer::from_be_bytes(&[0x01]).expect("small")).is_none(),
            "and n > 1"
        );
    }

    /// A right shift keeps the high bits and zeroes out at the value's own width.
    #[test]
    fn a_shift_drops_the_low_bits_and_bottoms_out_at_zero() {
        let value = Integer::from_be_bytes(&[0x12, 0x34]).expect("small");
        assert_eq!(value.shifted_right(4).be_bytes(2), [0x01, 0x23]);
        assert_eq!(value.shifted_right(16).be_bytes(2), [0x00, 0x00]);
        assert!(value.shifted_right(4096).is_zero(), "far past the width");
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
