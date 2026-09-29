//! RFC 5639 section 3.7's brainpoolP512r1, as a curve type [`crate::ecdsa`] can verify on.
//!
//! ISO 32000-2 §12.8.3.1 introduces ECDSA "as defined by Internet RFC 5480", and ISO/TS 32002
//! Table 3, appended to that subclause, names six curves for it; five of them are packages. **This one has no
//! package on crates.io at all**, so the project owner's answer A170 (`doc/questions/A170`) is that
//! the tree states it itself, as a stopgap: private to this crate, reached only through
//! [`crate::ecdsa::verify`]'s named-curve seam, re-exported nowhere, and advertised nowhere. ADR
//! 1385 is the decision and its measurement.
//!
//! # The swap condition
//!
//! In A170's own terms: **the day a stable, reviewed crate covers the curve, the swap is decided on
//! `doc/stack.md`'s terms.** The module is shaped so that the swap is one line — [`BrainpoolP512r1`]
//! implements exactly the traits `bp256::BrainpoolP256r1` and `bp384::BrainpoolP384r1` implement,
//! so `ecdsa.rs`'s arm names a package's type instead of this one and this file is deleted.
//!
//! # What is this tree's, and what is still reviewed code
//!
//! **The arithmetic is not written here.** The two curve packages beside it are built from two
//! frames their supplier publishes separately — `primefield`, a prime field over `crypto-bigint`'s
//! Montgomery form, and `primeorder`, the complete addition formulas for a prime-order short
//! Weierstrass curve (Renes, Costello and Batina's) — and a curve package is those two frames plus
//! its constants. This module is the same construction with RFC 5639 section 3.7's constants, so
//! what is this tree's own is:
//!
//! - **the six numbers** `p`, `A`, `B`, `x`, `y` and `q`, transcribed from RFC 5639 section 3.7 and
//!   checked against it by the tests below — the base point on the curve, `q` times it the
//!   identity, and RFC 7027 appendix A.3's key-agreement vectors reproduced;
//! - **two quadratic non-residues**, `2` modulo `p` and `7` modulo `q`, which `primefield` asks of
//!   every field. They are the smallest non-residues, found by Euler's criterion and checked by
//!   the frame's own test. The frame names them *multiplicative generators*, and its own test
//!   checks only the non-residue property, for every curve it serves: whether either generates the
//!   whole multiplicative group would need `p − 1` factored, and nothing on a verification path
//!   depends on it — ECDSA verification never takes a square root, and the one decoding that does
//!   (a compressed SEC1 point) needs only `p ≡ 3 (mod 4)`, which RFC 5639's prime satisfies;
//! - **the trait wiring**, which is what makes this a curve `ecdsa`'s generic verification accepts.
//!
//! # Constant time, and why it does not bind here
//!
//! `crypto-bigint`'s Montgomery arithmetic and `primeorder`'s complete formulas are constant-time;
//! `primeorder`'s scalar multiplication under [`mul_backend::VariableOnly`] is **not**, and neither
//! is `ecdsa`'s verification, which runs `u1·G + u2·Q` in variable time on purpose. That is the
//! right side of the trade for the same reason [`crate::ecdsa`] states: a verifier holds no secret.
//! The key, `r`, `s` and the digest all came out of a file a stranger wrote; a timing channel on
//! them leaks what is already public. The danger in a verifier is *wrong* arithmetic, which is why
//! the arithmetic is the frames' and the numbers are checked against both RFCs.

use ecdsa::elliptic_curve::bigint::{Odd, U512};
use ecdsa::elliptic_curve::consts::U64;
use ecdsa::elliptic_curve::ff::PrimeField;
use ecdsa::elliptic_curve::ops::BatchInvert;
use ecdsa::elliptic_curve::scalar::{FromUintUnchecked, IsHigh};
use ecdsa::elliptic_curve::subtle::{Choice, ConstantTimeGreater as _};
use ecdsa::elliptic_curve::{self, CurveArithmetic, PrimeCurveArithmetic, hazmat::FieldArithmetic};
use primeorder::{PrimeCurveParams, mul_backend, point_arithmetic, wnaf};

/// RFC 5639 section 3.7's `p`, the field prime.
const MODULUS_HEX: &str = "aadd9db8dbe9c48b3fd4e6ae33c9fc07cb308db3b3c9d20ed6639cca703308717d4d9b009bc66842aecda12ae6a380e62881ff2f2d82c68528aa6056583a48f3";

/// RFC 5639 section 3.7's `q`, the order of the base point — and the group's, since `h = 1`.
const ORDER_HEX: &str = "aadd9db8dbe9c48b3fd4e6ae33c9fc07cb308db3b3c9d20ed6639cca70330870553e5c414ca92619418661197fac10471db1d381085ddaddb58796829ca90069";

/// The same `q` as a number, which is how `elliptic_curve::Curve` states an order.
const ORDER: Odd<U512> = Odd::<U512>::from_be_hex(ORDER_HEX);

/// brainpoolP512r1, RFC 5639 section 3.7's verifiably pseudo-random 512-bit curve.
///
/// A type rather than a value, as each curve package's is: [`crate::ecdsa`]'s one generic
/// verification is instantiated on it, which is what keeps the security-critical path written
/// once for six curves. `pub` because the frames' trait implementations name it in public
/// positions; the module is private, so nothing outside this crate can reach it.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, PartialOrd, Ord)]
pub struct BrainpoolP512r1;

impl elliptic_curve::Curve for BrainpoolP512r1 {
    /// A 512-bit prime is 64 octets exactly, so SEC1 needs no padding octet here.
    type FieldBytesSize = U64;
    type Uint = U512;
    const ORDER: Odd<U512> = ORDER;
}

impl elliptic_curve::PrimeCurve for BrainpoolP512r1 {}

impl elliptic_curve::point::PointCompression for BrainpoolP512r1 {
    const COMPRESS_POINTS: bool = false;
}

impl ecdsa::EcdsaCurve for BrainpoolP512r1 {
    /// ANSI X9.62 admits either `s` or `n − s`; only a signer would choose, and a verifier accepts
    /// both, as `bp384`'s own declaration does.
    const NORMALIZE_S: bool = false;
}

/// The field modulo `p`, over `crypto-bigint`'s Montgomery form.
///
/// A module of its own because the frame's macros declare items with fixed names and expect the
/// `subtle` and `ff` names in scope, and this is where both are bounded: nothing here leaves
/// `pdf-signature`.
mod field {
    use super::{MODULUS_HEX, U512};
    use ecdsa::elliptic_curve::ff::PrimeField;
    use ecdsa::elliptic_curve::subtle::{Choice, ConstantTimeEq, CtOption};

    primefield::monty_field_params! {
        name: FieldParams,
        modulus: MODULUS_HEX,
        uint: U512,
        byte_order: primefield::ByteOrder::BigEndian,
        multiplicative_generator: 2,
        doc: "Montgomery parameters for brainpoolP512r1's field prime, RFC 5639 section 3.7's p"
    }

    primefield::monty_field_element! {
        name: FieldElement,
        params: FieldParams,
        uint: U512,
        doc: "An element of brainpoolP512r1's base field, modulo RFC 5639 section 3.7's p"
    }

    primefield::monty_field_arithmetic! {
        name: FieldElement,
        params: FieldParams,
        uint: U512
    }
}

/// The scalars modulo `q`, the same construction over the group order.
mod scalar {
    use super::{ORDER_HEX, U512};
    use ecdsa::elliptic_curve::ff::PrimeField;
    use ecdsa::elliptic_curve::subtle::{Choice, ConstantTimeEq, CtOption};

    primefield::monty_field_params! {
        name: ScalarParams,
        modulus: ORDER_HEX,
        uint: U512,
        byte_order: primefield::ByteOrder::BigEndian,
        multiplicative_generator: 7,
        doc: "Montgomery parameters for brainpoolP512r1's group order, RFC 5639 section 3.7's q"
    }

    primefield::monty_field_element! {
        name: Scalar,
        params: ScalarParams,
        uint: U512,
        doc: "A scalar of brainpoolP512r1, modulo RFC 5639 section 3.7's q"
    }

    primefield::monty_field_arithmetic! {
        name: Scalar,
        params: ScalarParams,
        uint: U512
    }

    primefield::monty_field_reduce! {
        name: Scalar,
        params: ScalarParams,
        uint: U512,
    }
}

pub(crate) use field::FieldElement;
pub(crate) use scalar::Scalar;

impl BatchInvert for FieldElement {}

elliptic_curve::scalar_impls!(BrainpoolP512r1, Scalar);

wnaf::impl_wnaf_size_for_scalar!(Scalar);

impl AsRef<Scalar> for Scalar {
    fn as_ref(&self) -> &Scalar {
        self
    }
}

impl FromUintUnchecked for Scalar {
    type Uint = U512;

    fn from_uint_unchecked(uint: Self::Uint) -> Self {
        Self::from_uint_unchecked(uint)
    }
}

impl IsHigh for Scalar {
    fn is_high(&self) -> Choice {
        const HALF_ORDER: U512 = ORDER.as_ref().shr_vartime(1);
        self.to_canonical().ct_gt(&HALF_ORDER)
    }
}

/// A point in affine coordinates, `primeorder`'s generic one on this curve.
pub(crate) type AffinePoint = primeorder::AffinePoint<BrainpoolP512r1>;

/// A point in projective coordinates, where the group law runs.
pub(crate) type ProjectivePoint = primeorder::ProjectivePoint<BrainpoolP512r1>;

impl CurveArithmetic for BrainpoolP512r1 {
    type AffinePoint = AffinePoint;
    type ProjectivePoint = ProjectivePoint;
    type Scalar = Scalar;
}

impl FieldArithmetic for BrainpoolP512r1 {
    type FieldElement = FieldElement;
}

impl PrimeCurveArithmetic for BrainpoolP512r1 {
    type CurveGroup = ProjectivePoint;
}

impl PrimeCurveParams for BrainpoolP512r1 {
    /// `A` is neither zero nor `−3`, so the generic complete formulas are the ones that apply.
    type PointArithmetic = point_arithmetic::EquationAIsGeneric;
    /// Variable time, and deliberately so: see the module documentation.
    type Backend = mul_backend::VariableOnly;

    /// RFC 5639 section 3.7's `A`.
    const EQUATION_A: FieldElement = FieldElement::from_hex_vartime(
        "7830a3318b603b89e2327145ac234cc594cbdd8d3df91610a83441caea9863bc2ded5d5aa8253aa10a2ef1c98b9ac8b57f1117a72bf2c7b9e7c1ac4d77fc94ca",
    );
    /// RFC 5639 section 3.7's `B`.
    const EQUATION_B: FieldElement = FieldElement::from_hex_vartime(
        "3df91610a83441caea9863bc2ded5d5aa8253aa10a2ef1c98b9ac8b57f1117a72bf2c7b9e7c1ac4d77fc94cadc083e67984050b75ebae5dd2809bd638016f723",
    );
    /// RFC 5639 section 3.7's base point `G = (x, y)`.
    const GENERATOR: (FieldElement, FieldElement) = (
        FieldElement::from_hex_vartime(
            "81aee4bdd82ed9645a21322e9c4c6a9385ed9f70b5d916c1b43b62eef4d0098eff3b1f78e2d0d48d50d1687b93b97d5f7c6d5047406a5e688b352209bcb9f822",
        ),
        FieldElement::from_hex_vartime(
            "7dde385d566332ecc0eabfa9cf7822fdf209f70024a57b1aa000c55b881f8111b2dcde494a5f485e5bca4bd88a2763aed1ca2b2fa8f0540678cd1e0f3ad80892",
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::{AffinePoint, BrainpoolP512r1, FieldElement, ProjectivePoint, Scalar, U512};
    use ecdsa::elliptic_curve::PrimeField as _;
    use ecdsa::elliptic_curve::group::Group as _;
    use ecdsa::elliptic_curve::sec1::{FromSec1Point as _, ToSec1Point as _};
    use primeorder::PrimeCurveParams as _;

    primefield::test_primefield!(FieldElement, U512);

    /// The frame's own test of a field's constants, a second time for the scalar field.
    mod scalar_field {
        use super::super::{Scalar, U512};
        primefield::test_primefield!(Scalar, U512);
    }

    /// A scalar out of RFC 7027's hexadecimal, which is big-endian and may drop leading zeros.
    fn scalar(hex: &str) -> Scalar {
        let padded = format!("{hex:0>128}");
        Scalar::from_repr(bytes::<64>(&padded).into()).expect("a scalar below q")
    }

    /// A point out of RFC 7027's two coordinates, through SEC1's uncompressed encoding — the
    /// path a certificate's key takes, so the vector exercises the decoding as well.
    fn point(x: &str, y: &str) -> ProjectivePoint {
        let encoded = bytes::<129>(&format!("04{x:0>128}{y:0>128}"));
        let point = AffinePoint::from_sec1_bytes(&encoded).expect("a point on the curve");
        point.into()
    }

    fn bytes<const N: usize>(hex: &str) -> [u8; N] {
        let mut out = [0u8; N];
        for (octet, pair) in out.iter_mut().zip(hex.as_bytes().chunks(2)) {
            *octet = u8::from_str_radix(std::str::from_utf8(pair).expect("ascii"), 16)
                .expect("hexadecimal");
        }
        out
    }

    /// RFC 5639 section 3.7's `G` satisfies its `y² = x³ + Ax + B`.
    #[test]
    fn the_base_point_is_on_the_curve_its_section_states() {
        let (x, y) = BrainpoolP512r1::GENERATOR;
        let right = x.square() * x + BrainpoolP512r1::EQUATION_A * x + BrainpoolP512r1::EQUATION_B;
        assert_eq!(y.square(), right);
    }

    /// `q · G` is the identity and `(q − 1) · G` is `−G`: the order the section states is the
    /// base point's order, which is the property ECDSA's `s⁻¹` modulo `q` rests on.
    #[test]
    fn the_order_the_section_states_is_the_base_points() {
        let generator = ProjectivePoint::generator();
        let minus_one = -Scalar::ONE;
        assert_eq!(generator * minus_one, -generator);
        assert_eq!(
            generator * minus_one + generator,
            ProjectivePoint::identity()
        );
    }

    /// RFC 7027 appendix A.3's key agreement, reproduced: each private key times `G` is its
    /// public key, and each private key times the other's public key is the same shared point.
    ///
    /// Taken from IETF RFC 7027 (Merkle and Lochter, 2013) unmodified but for line joins, under the
    /// IETF Trust's Legal Provisions section 3.c.iii; `doc/third-party-data.md` records it. These
    /// are the one set of published vectors for this curve among the texts this round could
    /// hold — RFC 5639 prints parameters and no computations, and BSI TR-03111's examples are not
    /// held. Scalar multiplication is the whole of ECDSA verification's group arithmetic, so a
    /// vector that fixes `d · G` and `d · Q` fixes what verification computes with.
    #[test]
    fn rfc_7027_appendix_a3s_key_agreement_is_reproduced() {
        let d_a = scalar(
            "16302FF0DBBB5A8D733DAB7141C1B45ACBC8715939677F6A56850A38BD87BD59B09E80279609FF333EB9D4C061231FB26F92EEB04982A5F1D1764CAD57665422",
        );
        let q_a = point(
            "0A420517E406AAC0ACDCE90FCD71487718D3B953EFD7FBEC5F7F27E28C6149999397E91E029E06457DB2D3E640668B392C2A7E737A7F0BF04436D11640FD09FD",
            "72E6882E8DB28AAD36237CD25D580DB23783961C8DC52DFA2EC138AD472A0FCEF3887CF62B623B2A87DE5C588301EA3E5FC269B373B60724F5E82A6AD147FDE7",
        );
        let d_b = scalar(
            "230E18E1BCC88A362FA54E4EA3902009292F7F8033624FD471B5D8ACE49D12CFABBC19963DAB8E2F1EBA00BFFB29E4D72D13F2224562F405CB80503666B25429",
        );
        let q_b = point(
            "9D45F66DE5D67E2E6DB6E93A59CE0BB48106097FF78A081DE781CDB31FCE8CCBAAEA8DD4320C4119F1E9CD437A2EAB3731FA9668AB268D871DEDA55A5473199F",
            "2FDC313095BCDD5FB3A91636F07A959C8E86B5636A1E930E8396049CB481961D365CC11453A06C719835475B12CB52FC3C383BCE35E27EF194512B71876285FA",
        );
        let z = point(
            "A7927098655F1F9976FA50A9D566865DC530331846381C87256BAF3226244B76D36403C024D7BBF0AA0803EAFF405D3D24F11A9B5C0BEF679FE1454B21C4CD1F",
            "7DB71C3DEF63212841C463E881BDCF055523BD368240E6C3143BD8DEF8B3B3223B95E0F53082FF5E412F4222537A43DF1C6D25729DDB51620A832BE6A26680A2",
        );
        let generator = ProjectivePoint::generator();
        assert_eq!(generator * d_a, q_a, "d_A · G = Q_A");
        assert_eq!(generator * d_b, q_b, "d_B · G = Q_B");
        assert_eq!(q_b * d_a, z, "d_A · Q_B = Z");
        assert_eq!(q_a * d_b, z, "d_B · Q_A = Z");
        // The same point comes back out of SEC1 as went in, so the encoding is not what agreed.
        let encoded = z.to_affine().to_sec1_point(false);
        assert_eq!(
            ProjectivePoint::from(AffinePoint::from_sec1_bytes(encoded.as_bytes()).expect("Z")),
            z
        );
    }

    /// A point one unit off the curve is refused by the decoding, not carried into arithmetic.
    #[test]
    fn a_point_off_the_curve_does_not_decode() {
        let (x, y) = BrainpoolP512r1::GENERATOR;
        let mut encoded = vec![0x04];
        encoded.extend_from_slice(&x.to_repr());
        encoded.extend_from_slice(&(y + FieldElement::ONE).to_repr());
        assert!(AffinePoint::from_sec1_bytes(&encoded).is_err());
    }
}
