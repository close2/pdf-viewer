//! CPU coverage rasterisation: the one producer of coverage bytes for the glyph and
//! path lanes.
//!
//! # Why the CPU, and why our own (ADR 0008)
//!
//! The glyph atlas (M4) and the general path lane (M5) both need an R8 coverage
//! image of an outline. Rasterising it on the CPU makes the bytes **identical on
//! every adapter** — coverage becomes immune to the driver-owned float→unorm
//! conversion that ADR 0006 measured, leaving only the final blend inside the ±1
//! bound — and it needs no compute pipeline on the startup path. Writing our own
//! (rather than depending on `tiny-skia`) keeps the dependency graph as `deny.toml`
//! wants it and keeps the arithmetic *stated*: every rule below is a documented
//! choice a test can derive expectations from, because ISO 32000-2 does not define
//! anti-aliasing (ADR 0005 records that silence).
//!
//! # The definition of coverage
//!
//! A pixel's coverage is the area of the set ISO 32000-2 §8.5.3.3's rule declares inside,
//! within the pixel (§10.7.4 scan-converts after "all 'insideness' computations have been
//! performed"; ADR 1389). Flattened edges deposit exact signed trapezoid areas into an
//! accumulation grid; a left-to-right prefix sum recovers the average winding `w` per pixel;
//! and where the winding in the pixel takes at most two neighbouring values — every pixel
//! of a fill whose subpaths neither cross nor nest the same way round — the rule applied
//! to the average is that area:
//! - non-zero rule (§8.5.3.3.2): `min(|w|, 1)`,
//! - even-odd rule (§8.5.3.3.3): `1 − |1 − (w mod 2)|`, the triangle fold.
//!
//! The other pixels — where a path's portions overlap, cross, or wind against one another
//! — are recomputed from the set itself, band by band (`fill::exact`).
//!
//! Curves flatten by recursive midpoint subdivision to a stated tolerance — the tighter
//! of [`FLATTEN_TOLERANCE`](flatten::FLATTEN_TOLERANCE) and
//! [`RELATIVE_FLATTEN_TOLERANCE`](flatten::RELATIVE_FLATTEN_TOLERANCE) of the curve's
//! own size (ADR 0044); strokes expand to closed polygons (§8.4.3's caps and joins) and
//! fill non-zero. Quantisation to a byte is `round(cov × 255)`.
//!
//! # The three parts, and the order a mark passes through them
//!
//! Those three sentences are three clauses and three files, and a mark drawn on the CPU
//! passes through them in one order: **segments → polylines → polygons → bytes**.
//! rustdoc inlines a re-export from a private module (ADR 0051 section 1), so this table is
//! the only place that structure survives into the documentation:
//!
//! | Module | Its one thing |
//! |---|---|
//! | [`mod@flatten`] | an outline's segments, under a transform, as device-space polylines — and how finely (§10.7.2, ADR 0044) |
//! | [`stroke`] | §8.4.3's stroke expanded into closed polygons for filling non-zero: polylines in, polylines out, no coverage anywhere |
//! | [`fill`] | polylines into coverage bytes over a region: the accumulation grid, §8.5.3.3's two rules, and ADR 0049's cut at the border |
//!
//! The dependency runs one way — [`stroke`] and [`fill`] each take what [`mod@flatten`]
//! produces, and neither knows about the other. **A defect in one of the three is a
//! defect in one clause**, which is what the split is for: the three arithmetic defects
//! this code has had were `stroke::direction`'s (a length that left `f32` at both ends),
//! `fill::accumulate_edge`'s (a slope that left it at one) and `fill::deposit_slab`'s
//! (a slab smeared across the border instead of cut at it, ADR 0049). Each is one
//! function of one part, and none could have been found by reading another.

mod fill;
mod flatten;
pub(crate) mod reduce;
mod stroke;

pub(crate) use fill::{CoverageMask, Rule, fill_mask, fill_mask_settled, winds_two_values};
pub(crate) use flatten::{DeviceTransform, Polyline, flatten, polyline_bounds};
pub(crate) use stroke::{resolve_width, stroke_polylines};

#[cfg(test)]
mod tests;
