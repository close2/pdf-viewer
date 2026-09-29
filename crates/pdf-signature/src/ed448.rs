//! RFC 8032 section 5.2's Ed448, verification only, as [`crate::eddsa`] reaches it.
//!
//! ISO/TS 32002 section 5.1.2 adds an `EdDSA` row to ISO 32000-2 §12.8.3.1's Table 260, and its
//! Table 4 pairs Ed448 with SHAKE256 as the second of that row's two curves.
//! `ed448-goldilocks` carries the signature scheme only on a pre-release line, which this tree does
//! not take, so the project owner's answer A170 (`doc/questions/A170`) is that the tree computes it
//! itself, as a stopgap: private to this crate, reached only through [`crate::eddsa::verify`]'s
//! named-curve seam, re-exported nowhere and advertised nowhere. ADR 1386 is the decision.
//!
//! # The swap condition
//!
//! In A170's own terms: **the day a stable, reviewed crate covers the curve, the swap is decided on
//! `doc/stack.md`'s terms.** The seam is [`verify`]'s signature — a key's octets, a signature's
//! octets and the message in parts, answering `Ok(bool)` or an [`EdDsaError`] — which is exactly
//! what the Ed25519 arm of [`crate::eddsa::verify`] hands `ed25519-dalek`. A crate's verifier goes
//! behind the same arm and this file is deleted; the vectors below move to the arm's tests.
//!
//! # What is here
//!
//! RFC 8032 section 5.2 in the order the section states it, and nothing a signer needs:
//!
//! - **the field** GF(p), `p = 2^448 − 2^224 − 1` (Table 2), as `crypto-bigint`'s constant-modulus
//!   Montgomery form — the reviewed floor ADR 0331 put under every other verification here, so no
//!   multiplication, reduction or exponentiation is written in this file;
//! - **the untwisted Edwards curve** `x² + y² = 1 + d·x²·y²`, `d = −39081` (Table 2), in section
//!   5.2.4's projective coordinates with its complete addition and doubling formulas;
//! - **section 5.2.3's decoding**, every check it names: `y ≥ p` fails, a `u/v` with no square
//!   root fails, and `x = 0` with the sign bit set fails;
//! - **section 5.2.7's verification**, with `H(x) = SHAKE256(dom4(F, C) ‖ x, 114)`.
//!
//! **Which of section 5.2.7's two equations.** The section's check is the cofactored group
//! equation, `[4][S]B = [4]R + [4][k]A'`, and it permits the uncofactored `[S]B = R + [k]A'` in
//! its place without requiring it; [`crate::eddsa`] takes the specification's own equation for
//! Ed25519 on the argument that the stricter one would refuse a signature the RFC calls valid, and
//! this module takes it for the same reason. (RFC text is paraphrased here, never quoted: the
//! quotation gate reads quotation marks as ISO 32000-2's.)
//!
//! **SHAKE256 is the `shake` package's**, the one [`crate::cms::Digest::Shake256`] is computed
//! with. It is called directly rather than through that type because the two uses squeeze
//! different lengths: a CMS digest is 64 octets (RFC 8702 section 3.1) and section 5.2's `H` is
//! 114.
//!
//! # Constant time, and why it does not bind here
//!
//! `crypto-bigint`'s field operations are constant-time. The scalar multiplication here is not —
//! it is double-and-add over the scalar's bits — and neither are the decoding's comparisons. That
//! is deliberate and is ADR 0229's argument: a verifier holds no secret. `A`, `R`, `S` and the
//! message all came out of a file a stranger wrote, and a timing channel on them leaks what anyone
//! holding the file already has. The danger in a verifier is *wrong* arithmetic, which is why the
//! arithmetic is reviewed code and the construction is held to every vector RFC 8032 section 7.4
//! and section 7.5 print.

use crypto_bigint::modular::ConstMontyForm;
use crypto_bigint::{Encoding as _, NonZero, U448, U512, U1024};
use shake::{ExtendableOutput as _, Update as _, XofReader as _};

use crate::eddsa::EdDsaError;

/// RFC 8032 section 5.2's `p`, big-endian hexadecimal: `2^448 − 2^224 − 1`.
const PRIME_HEX: &str = "fffffffffffffffffffffffffffffffffffffffffffffffffffffffeffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

crypto_bigint::const_monty_params!(
    Prime,
    U448,
    PRIME_HEX,
    "RFC 8032 section 5.2's field prime, `2^448 − 2^224 − 1`."
);

/// An element of GF(p), in Montgomery form.
type Element = ConstMontyForm<Prime, { U448::LIMBS }>;

/// `p` as a 512-bit number, for section 5.2.3 step 1's comparison of a 456-bit encoding with `p`.
const PRIME_WIDE: U512 = U448::from_be_hex(PRIME_HEX).resize();

/// RFC 8032 section 5.2's `L`, the order of the base point: `2^446 −
/// 13818066809895115352007386748515426880336692474882178609894547503885`.
const ORDER: U448 = U448::from_be_hex(
    "3fffffffffffffffffffffffffffffffffffffffffffffffffffffff7cca23e9c44edb49aed63690216cc2728dc58f552378c292ab5844f3",
);

/// `(p − 3) / 4`, the exponent section 5.2.3 step 2 raises `u^5·v^3` to — one exponentiation doing
/// the inversion of `v` and the square root together.
const ROOT_EXPONENT: U448 = U448::from_be_hex(
    "3fffffffffffffffffffffffffffffffffffffffffffffffffffffffbfffffffffffffffffffffffffffffffffffffffffffffffffffffff",
);

/// Section 5.2's `B`, Table 2's `X(P)`, as hexadecimal of the decimal the table prints.
const BASE_X: U448 = U448::from_be_hex(
    "4f1970c66bed0ded221d15a622bf36da9e146570470f1767ea6de324a3d3a46412ae1af72ab66511433b80e18b00938e2626a82bc70cc05e",
);

/// Section 5.2's `B`, Table 2's `Y(P)`.
const BASE_Y: U448 = U448::from_be_hex(
    "693f46716eb6bc248876203756c9c7624bea73736ca3984087789c1e05a0c2d73ad3ff1ce67c39c4fdbd132c4ed7c8ad9808795bf230fa14",
);

/// Section 5.2's `b / 8`: a key, and each half of a signature, is 57 octets.
pub(crate) const KEY_OCTETS: usize = 57;

/// Section 5.2.6's signature, `R ‖ S`: two of those.
pub(crate) const SIGNATURE_OCTETS: usize = 2 * KEY_OCTETS;

/// Section 5.2's `H` output, `SHAKE256(…, 114)`.
const HASH_OCTETS: usize = 114;

/// Section 5.2's `d = −39081`, as an element of the field.
fn curve_d() -> Element {
    Element::new(&U448::from_u32(39_081)).neg()
}

/// A point in section 5.2.4's projective coordinates, `x = X/Z`, `y = Y/Z`.
#[derive(Clone, Copy, Debug)]
struct Point {
    x: Element,
    y: Element,
    z: Element,
}

impl Point {
    /// Section 5.2.4's neutral point, `(0, 1)`.
    fn identity() -> Self {
        Self {
            x: Element::ZERO,
            y: Element::ONE,
            z: Element::ONE,
        }
    }

    /// Section 5.2's base point `B`.
    fn base() -> Self {
        Self {
            x: Element::new(&BASE_X),
            y: Element::new(&BASE_Y),
            z: Element::ONE,
        }
    }

    /// Section 5.2.4's addition, complete on this curve: it holds for any two valid points,
    /// including equal ones and the identity, so no case is branched on.
    #[expect(
        clippy::many_single_char_names,
        reason = "the names are section 5.2.4's own, so the formulas read against the RFC line for line"
    )]
    fn add(&self, other: &Self) -> Self {
        let a = self.z.mul(&other.z);
        let b = a.square();
        let c = self.x.mul(&other.x);
        let d = self.y.mul(&other.y);
        let e = curve_d().mul(&c).mul(&d);
        let f = b.sub(&e);
        let g = b.add(&e);
        let h = self.x.add(&self.y).mul(&other.x.add(&other.y));
        Self {
            x: a.mul(&f).mul(&h.sub(&c).sub(&d)),
            y: a.mul(&g).mul(&d.sub(&c)),
            z: f.mul(&g),
        }
    }

    /// Section 5.2.4's doubling formulas.
    #[expect(
        clippy::many_single_char_names,
        reason = "the names are section 5.2.4's own, so the formulas read against the RFC line for line"
    )]
    fn double(&self) -> Self {
        let b = self.x.add(&self.y).square();
        let c = self.x.square();
        let d = self.y.square();
        let e = c.add(&d);
        let h = self.z.square();
        let j = e.sub(&h.double());
        Self {
            x: b.sub(&e).mul(&j),
            y: e.mul(&c.sub(&d)),
            z: e.mul(&j),
        }
    }

    /// `[scalar]self`, by double-and-add from the most significant bit.
    ///
    /// Variable time in the scalar's bits, which are public: see the module documentation.
    fn times(&self, scalar: &U448) -> Self {
        let mut sum = Self::identity();
        for bit in (0..U448::BITS).rev() {
            sum = sum.double();
            if scalar.bit_vartime(bit) {
                sum = sum.add(self);
            }
        }
        sum
    }

    /// `[4]self`, section 5.2's cofactor `2^c` with `c = 2`.
    fn times_cofactor(&self) -> Self {
        self.double().double()
    }

    /// Whether two projective points are the same affine point: `X1·Z2 = X2·Z1` and `Y1·Z2 =
    /// Y2·Z1`. `Z` is never zero for a point these formulas produce from valid inputs.
    fn same_as(&self, other: &Self) -> bool {
        self.x.mul(&other.z) == other.x.mul(&self.z) && self.y.mul(&other.z) == other.y.mul(&self.z)
    }

    /// Section 5.2.3's decoding of a 57-octet string, or `None` where any of its checks fails.
    fn decode(octets: &[u8; KEY_OCTETS]) -> Option<Self> {
        // Step 1: little-endian, bit 455 is `x_0`, and clearing it leaves `y`.
        let mut wide = [0u8; 64];
        wide.get_mut(..KEY_OCTETS)?.copy_from_slice(octets);
        let x_0 = wide
            .get(KEY_OCTETS - 1)
            .is_some_and(|&last| last & 0x80 != 0);
        if let Some(last) = wide.get_mut(KEY_OCTETS - 1) {
            *last &= 0x7F;
        }
        let y = U512::from_le_bytes(wide.into());
        if y >= PRIME_WIDE {
            return None;
        }
        let y = Element::new(&y.resize::<{ U448::LIMBS }>());
        // Step 2: `u = y² − 1`, `v = d·y² − 1`, and `x = u³·v·(u⁵·v³)^((p−3)/4)`.
        let y_squared = y.square();
        let u = y_squared.sub(&Element::ONE);
        let v = curve_d().mul(&y_squared).sub(&Element::ONE);
        let u_cubed = u.square().mul(&u);
        let v_cubed = v.square().mul(&v);
        let x = u_cubed.mul(&v).mul(
            &u_cubed
                .mul(&u)
                .mul(&u)
                .mul(&v_cubed)
                .pow_vartime(&ROOT_EXPONENT),
        );
        // Step 3: the candidate is a root only where `v·x² = u`.
        if v.mul(&x.square()) != u {
            return None;
        }
        // Step 4: `x = 0` has no negative, and otherwise the sign bit picks the root.
        let canonical = x.retrieve();
        if canonical == U448::ZERO && x_0 {
            return None;
        }
        let x = if canonical.bit_vartime(0) == x_0 {
            x
        } else {
            x.neg()
        };
        Some(Self {
            x,
            y,
            z: Element::ONE,
        })
    }
}

/// Verifies a `PureEdDSA` Ed448 signature — RFC 8419's only use of the curve in CMS — over the bytes
/// it was made on.
///
/// Section 5.2's Ed448 row: `phflag` 0 and the empty context, which is what RFC 8419 section 1
/// fixes for CMS — pure mode only, and no context string.
///
/// # Errors
///
/// [`EdDsaError::MalformedKey`] where the key is not 57 octets or does not decode, and
/// [`EdDsaError::MalformedSignature`] where the signature is not 114. A signature whose `R` does
/// not decode or whose `S` is not below `L` is `Ok(false)`: section 5.2.7 step 1 makes both an
/// invalid signature, which is an answer about the signature rather than a failure to reach one.
pub(crate) fn verify(key: &[u8], signature: &[u8], message: &[&[u8]]) -> Result<bool, EdDsaError> {
    verify_with(key, signature, 0, &[], message)
}

/// Section 5.2.7 under any `dom4(F, C)`.
///
/// `phflag` is `F` and `context` is `C`; a caller verifying Ed448ph hands in `PH(M)` as the
/// message. Only [`verify`]'s Ed448 is reached from a document; the other two variants are here so
/// that RFC 8032 section 7.4 and section 7.5's vectors test the same function a document does.
fn verify_with(
    key: &[u8],
    signature: &[u8],
    phflag: u8,
    context: &[u8],
    message: &[&[u8]],
) -> Result<bool, EdDsaError> {
    let key: &[u8; KEY_OCTETS] = key.try_into().map_err(|_| EdDsaError::MalformedKey)?;
    let a = Point::decode(key).ok_or(EdDsaError::MalformedKey)?;
    // A key no section 5.2.5 key pair has, under which every message has a signature anybody can
    // write: see `EdDsaError::SmallOrderKey`.
    if a.times_cofactor().same_as(&Point::identity()) {
        return Err(EdDsaError::SmallOrderKey);
    }
    let signature: &[u8; SIGNATURE_OCTETS] = signature
        .try_into()
        .map_err(|_| EdDsaError::MalformedSignature)?;
    let (r_octets, s_octets) = signature.split_at(KEY_OCTETS);
    // Section 5.2 bounds a context at 255 octets. A longer one is a context no signer could have
    // used, so no signature verifies under it.
    let Ok(context_length) = u8::try_from(context.len()) else {
        return Ok(false);
    };
    // Step 1: `R` decodes as a point and `S` is an integer at least 0 and below `L`.
    let Some(r) = r_octets.try_into().ok().and_then(Point::decode) else {
        return Ok(false);
    };
    let mut wide = [0u8; 64];
    let Some(low) = wide.get_mut(..KEY_OCTETS) else {
        return Ok(false);
    };
    low.copy_from_slice(s_octets);
    let s = U512::from_le_bytes(wide.into());
    if s >= ORDER.resize() {
        return Ok(false);
    }
    let s = s.resize::<{ U448::LIMBS }>();
    // Step 2: `k = SHAKE256(dom4(F, C) ‖ R ‖ A ‖ PH(M), 114)`, little-endian, reduced modulo `L`.
    let mut hasher = shake::Shake256::default();
    hasher.update(b"SigEd448");
    hasher.update(&[phflag, context_length]);
    hasher.update(context);
    hasher.update(r_octets);
    hasher.update(key);
    for part in message {
        hasher.update(part);
    }
    let mut digest = [0u8; 128];
    if let Some(squeezed) = digest.get_mut(..HASH_OCTETS) {
        hasher.finalize_xof().read(squeezed);
    }
    let order = NonZero::new(ORDER)
        .into_option()
        .ok_or(EdDsaError::MalformedKey)?;
    let k = U1024::from_le_bytes(digest.into()).rem_vartime(&order);
    // Step 3: `[4][S]B = [4]R + [4][k]A'`.
    let left = Point::base().times(&s).times_cofactor();
    let right = r.add(&a.times(&k)).times_cofactor();
    Ok(left.same_as(&right))
}

#[cfg(test)]
mod vectors;

#[cfg(test)]
mod tests {
    use super::{BASE_X, BASE_Y, ORDER, PRIME_HEX, Point, U448, curve_d, verify, verify_with};
    use crate::eddsa::EdDsaError;
    use crypto_bigint::modular::ConstMontyForm;
    use shake::{ExtendableOutput as _, Update as _, XofReader as _};

    use super::vectors::{ED448, ED448PH, Vector, hex};

    /// A decimal string as a number, for checking a transcription against the table's own digits.
    fn decimal(digits: &str) -> U448 {
        digits.chars().fold(U448::ZERO, |sum, digit| {
            let digit = digit.to_digit(10).expect("a decimal digit");
            sum.wrapping_mul(&U448::from_u8(10))
                .wrapping_add(&U448::from_u32(digit))
        })
    }

    /// Table 2's decimal `B` and `L` are the hexadecimal constants above, digit for digit.
    #[test]
    fn the_constants_are_the_tables_own_numbers() {
        assert_eq!(
            decimal(
                "224580040295924300187604334099896036246789641632564134246125461686950415467406032909029192869357953282578032075146446173674602635247710"
            ),
            BASE_X
        );
        assert_eq!(
            decimal(
                "298819210078481492676017930443930673437544040154080242095928241372331506189835876003536878655418784733982303233503462500531545062832660"
            ),
            BASE_Y
        );
        let two_to_446 = U448::ONE.shl_vartime(446);
        assert_eq!(
            two_to_446.wrapping_sub(&decimal(
                "13818066809895115352007386748515426880336692474882178609894547503885"
            )),
            ORDER
        );
        let prime = U448::from_be_hex(PRIME_HEX);
        assert_eq!(
            // `U448::MAX` is `2^448 − 1`.
            U448::MAX.wrapping_sub(&U448::ONE.shl_vartime(224)),
            prime,
            "2^448 − 2^224 − 1"
        );
    }

    /// `B` satisfies `x² + y² = 1 + d·x²·y²`, and `[L]B` is the neutral point.
    #[test]
    fn the_base_point_is_on_the_curve_and_has_the_order_the_table_states() {
        let base = Point::base();
        let (x2, y2) = (base.x.square(), base.y.square());
        assert_eq!(
            x2.add(&y2),
            ConstMontyForm::ONE.add(&curve_d().mul(&x2).mul(&y2))
        );
        assert!(base.times(&ORDER).same_as(&Point::identity()));
        assert!(
            !base
                .times(&ORDER.wrapping_sub(&U448::ONE))
                .same_as(&Point::identity())
        );
    }

    fn check(vector: &Vector, phflag: u8) -> Result<bool, EdDsaError> {
        let message = hex(vector.message);
        let key = hex(vector.public_key);
        let signature = hex(vector.signature);
        let context = hex(vector.context);
        if phflag == 0 {
            verify_with(&key, &signature, 0, &context, &[&message])
        } else {
            // Section 5.2: Ed448ph's `PH` is `SHAKE256(x, 64)`.
            let mut hasher = shake::Shake256::default();
            hasher.update(&message);
            let mut prehash = [0u8; 64];
            hasher.finalize_xof().read(&mut prehash);
            verify_with(&key, &signature, 1, &context, &[&prehash])
        }
    }

    /// Every vector RFC 8032 section 7.4 prints verifies.
    #[test]
    fn every_section_7_4_vector_verifies() {
        for vector in ED448 {
            assert_eq!(check(vector, 0), Ok(true), "{}", vector.name);
        }
    }

    /// Both of section 7.5's Ed448ph vectors verify through the same function, with `F = 1`.
    #[test]
    fn every_section_7_5_vector_verifies() {
        for vector in ED448PH {
            assert_eq!(check(vector, 1), Ok(true), "{}", vector.name);
            assert_eq!(check(vector, 0), Ok(false), "{} is not Ed448", vector.name);
        }
    }

    /// The document path: an empty context and `F = 0`, reached through [`verify`].
    #[test]
    fn the_context_free_vectors_verify_through_the_document_path() {
        for vector in ED448.iter().filter(|vector| vector.context.is_empty()) {
            let key = hex(vector.public_key);
            let signature = hex(vector.signature);
            let message = hex(vector.message);
            assert_eq!(
                verify(&key, &signature, &[&message]),
                Ok(true),
                "{}",
                vector.name
            );
            let (head, tail) = message.split_at(message.len() / 2);
            assert_eq!(
                verify(&key, &signature, &[head, tail]),
                Ok(true),
                "{} in two parts",
                vector.name
            );
        }
    }

    /// One bit moved anywhere — message, `R`, `S`, key or context — and the vector no longer
    /// verifies. A key that stops decoding is a named refusal; anything else is `Ok(false)`.
    #[test]
    fn a_vector_one_bit_away_does_not_verify() {
        for vector in ED448 {
            let key = hex(vector.public_key);
            let signature = hex(vector.signature);
            let message = hex(vector.message);
            let context = hex(vector.context);
            if !message.is_empty() {
                let mut moved = message.clone();
                moved[0] ^= 0x01;
                assert_eq!(
                    verify_with(&key, &signature, 0, &context, &[&moved]),
                    Ok(false),
                    "{}: message",
                    vector.name
                );
            }
            for index in [0, 28, 56, 57, 85, 112] {
                let mut moved = signature.clone();
                moved[index] ^= 0x01;
                assert_eq!(
                    verify_with(&key, &moved, 0, &context, &[&message]),
                    Ok(false),
                    "{}: signature octet {index}",
                    vector.name
                );
            }
            for index in [0, 28, 55] {
                let mut moved = key.clone();
                moved[index] ^= 0x01;
                assert_ne!(
                    verify_with(&moved, &signature, 0, &context, &[&message]),
                    Ok(true),
                    "{}: key octet {index}",
                    vector.name
                );
            }
            let mut other = context.clone();
            other.push(0);
            assert_eq!(
                verify_with(&key, &signature, 0, &other, &[&message]),
                Ok(false),
                "{}: context",
                vector.name
            );
        }
    }

    /// Section 5.2.7 step 1: `S` not below `L` is an invalid signature, even where `S − L` would
    /// verify — the check that keeps a second signature from being made out of a first.
    #[test]
    fn an_s_not_below_the_order_is_refused_even_where_the_group_would_accept_it() {
        let vector = &ED448[0];
        let key = hex(vector.public_key);
        let mut signature = hex(vector.signature);
        let s = crypto_bigint::U512::from_le_slice(&[&signature[57..], &[0u8; 7][..]].concat());
        let pushed = s.wrapping_add(&ORDER.resize());
        signature[57..].copy_from_slice(&pushed.to_le_bytes()[..57]);
        assert_eq!(
            verify_with(&key, &signature, 0, &[], &[&hex(vector.message)]),
            Ok(false)
        );
    }

    /// Section 5.2.3's three failures, each on its own.
    #[test]
    fn a_key_section_5_2_3_refuses_is_named() {
        // `y ≥ p`: `p` itself, little-endian, sign bit clear.
        let mut at_prime = U448::from_be_hex(PRIME_HEX).to_le_bytes().to_vec();
        at_prime.push(0);
        // `y = 1` is the neutral point, `x = 0`; with the sign bit set there is no such `x`.
        let mut negative_zero = vec![0u8; 57];
        negative_zero[0] = 1;
        negative_zero[56] = 0x80;
        // `y = 2`: `u/v = 3 / (4d − 1)` is not a square modulo `p` (checked by Euler's criterion
        // when this vector was chosen), so no `x` exists.
        let mut no_root = vec![0u8; 57];
        no_root[0] = 2;
        // A width other than 57.
        let short = vec![0u8; 56];
        let signature = hex(ED448[0].signature);
        for key in [at_prime, negative_zero, no_root, short] {
            assert_eq!(
                verify(&key, &signature, &[b""]),
                Err(EdDsaError::MalformedKey),
                "{key:02x?}"
            );
        }
        // `y = 1` with the sign bit clear *is* a point — the neutral one — and `y = 0` is `(±1, 0)`,
        // of order 4: both decode, and both are keys no key pair has. Under the second, the
        // all-zero signature (`R = (1, 0)`, `S = 0`) satisfies the cofactored equation for any
        // message, which is what the `x509` fuzz target found and why these are refused.
        let mut neutral = vec![0u8; 57];
        neutral[0] = 1;
        let order_four = vec![0u8; 57];
        for key in [neutral, order_four] {
            assert_eq!(
                verify(&key, &[0u8; 114], &[b"any message at all"]),
                Err(EdDsaError::SmallOrderKey),
                "{key:02x?}"
            );
        }
        assert_eq!(
            verify(&hex(ED448[0].public_key), &signature[..113], &[b""]),
            Err(EdDsaError::MalformedSignature)
        );
    }
}
