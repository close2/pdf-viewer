//! The two precisions the accumulation runs in: `f32` for a mark, `f64` for a clip's residue
//! (ADR 1491).
//!
//! A mark is filled once, over its own tile, and its bytes are the compute lane's bytes too
//! (ADR 0080): that port runs the same arithmetic on the device, where `f64` is not available,
//! so a mark keeps `f32`. A clip's residue is the one coverage this crate fills over two
//! different regions and must answer alike in both — the chain's own region, which every mark
//! under it then crops, or the asking mark's tile, when the region is declined (ADR 0049). In
//! `f32` the two disagree by one level wherever the set's area sits within a few units in the
//! last place of a rounding boundary, because a region crosses the columns left of a tile one
//! deposit at a time where the tile takes them as one deposit at its border. In `f64` from the
//! same `f32` points both are the area to far below a level, which is
//! [`area_in_pixel`](super::super::area_in_pixel)'s precision contract applied to the fill
//! that feeds it.
//!
//! The generic code is the `f32` code with its type named: every operation below is the
//! operation the fill made before, so a mark's bytes do not move.

use std::cmp::Ordering;
use std::fmt::Debug;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub};

/// A floating-point type the accumulation can run in.
pub(crate) trait Real:
    Copy
    + Debug
    + Default
    + PartialOrd
    + Sum
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Neg<Output = Self>
    + AddAssign
{
    /// `0`.
    const ZERO: Self;
    /// `1`.
    const ONE: Self;
    /// `1/2`.
    const HALF: Self;
    /// `255`, the byte a full pixel quantises to.
    const LEVELS: Self;

    /// A coordinate of a flattened point, widened exactly.
    fn of(value: f32) -> Self;
    /// A device row or column, as the region's corner states it.
    fn of_corner(value: i32) -> Self;
    /// A region's width or height, or a column of it.
    fn of_count(value: usize) -> Self;
    /// The largest whole number not above `self`.
    #[must_use]
    fn floor(self) -> Self;
    /// The smallest whole number not below `self`.
    #[must_use]
    fn ceil(self) -> Self;
    /// The nearest whole number, half away from zero.
    #[must_use]
    fn round(self) -> Self;
    /// The magnitude.
    #[must_use]
    fn abs(self) -> Self;
    /// The least non-negative remainder of division by `divisor`.
    #[must_use]
    fn rem_euclid(self, divisor: Self) -> Self;
    /// The smaller of two values, as the type's own `min` takes it.
    #[must_use]
    fn min(self, other: Self) -> Self;
    /// The larger of two values, as the type's own `max` takes it.
    #[must_use]
    fn max(self, other: Self) -> Self;
    /// `self` held to `lo ..= hi`.
    #[must_use]
    fn clamp(self, lo: Self, hi: Self) -> Self;
    /// Whether `self` is neither infinite nor NaN.
    fn is_finite(self) -> bool;
    /// A non-negative whole number below `usize::MAX`, as a column or row index.
    fn index(self) -> usize;
    /// A coverage in `0 ..= 255`, as a byte.
    fn byte(self) -> u8;
    /// IEEE 754's total order, as the type's own `total_cmp` takes it.
    fn total_cmp(&self, other: &Self) -> Ordering;
}

/// The body every method shares, for one primitive float type.
macro_rules! real {
    ($t:ty, $corner:expr) => {
        impl Real for $t {
            const ZERO: Self = 0.0;
            const ONE: Self = 1.0;
            const HALF: Self = 0.5;
            const LEVELS: Self = 255.0;

            fn of(value: f32) -> Self {
                Self::from(value)
            }
            fn of_corner(value: i32) -> Self {
                $corner(value)
            }
            #[expect(clippy::cast_precision_loss)] // a region's extent, bounded by the budget
            fn of_count(value: usize) -> Self {
                value as Self
            }
            fn floor(self) -> Self {
                <$t>::floor(self)
            }
            fn ceil(self) -> Self {
                <$t>::ceil(self)
            }
            fn round(self) -> Self {
                <$t>::round(self)
            }
            fn abs(self) -> Self {
                <$t>::abs(self)
            }
            fn rem_euclid(self, divisor: Self) -> Self {
                <$t>::rem_euclid(self, divisor)
            }
            fn min(self, other: Self) -> Self {
                <$t>::min(self, other)
            }
            fn max(self, other: Self) -> Self {
                <$t>::max(self, other)
            }
            fn clamp(self, lo: Self, hi: Self) -> Self {
                <$t>::clamp(self, lo, hi)
            }
            fn is_finite(self) -> bool {
                <$t>::is_finite(self)
            }
            #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            // callers floor and clamp first
            fn index(self) -> usize {
                self as usize
            }
            #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            // a rounded coverage in 0..=255
            fn byte(self) -> u8 {
                self as u8
            }
            fn total_cmp(&self, other: &Self) -> Ordering {
                <$t>::total_cmp(self, other)
            }
        }
    };
}

/// A device corner as `f32`, as the fill always stated it: exact below `2^24`, which a
/// viewport's corner is.
#[expect(clippy::cast_precision_loss)] // a corner bounded by target limits
fn corner_f32(value: i32) -> f32 {
    value as f32
}

real!(f32, corner_f32);
real!(f64, f64::from);
