//! §7.10.2's `/Order 3`: the cubic spline a Type 0 function interpolates its samples by.
//!
//! ISO 32000-2 §7.10.2, Table 39:
//!
//! > The order of interpolation between samples. Valid values shall be 1 and 3, specifying linear
//! > and cubic spline interpolation, respectively.
//!
//! The clause names the kind of interpolation and states no spline, so which cubic spline is a
//! documented choice (ADR 1636). The choice is the **not-a-knot** cubic spline, extended to many
//! inputs as a tensor product:
//!
//! - **It is a spline in the word's numerical sense**: piecewise cubic, twice continuously
//!   differentiable, and through every sample. A local cubic such as Catmull-Rom is once
//!   differentiable and is a different interpolant, a Hermite curve with estimated tangents.
//! - **The end condition is the one the clause's own threshold describes.** "If Size is less than 4,
//!   cubic spline interpolation is not possible and Order 3 shall be ignored if specified." A
//!   natural spline exists through two or three samples, so four is not where it becomes possible;
//!   the not-a-knot spline needs four, because its first and last two pieces are each one cubic and
//!   a cubic is fixed by four values. Through exactly four samples it is the cubic through them.
//! - **It reproduces every cubic exactly**, at and between the samples, in the end pieces as well as
//!   the middle ones. A natural spline sets the second derivative to zero at both ends, which is an
//!   end condition no sample stated, and bends the end pieces of any function whose curvature there
//!   is not zero.
//!
//! The cost is the one every C2 spline has: a value between two samples depends on every sample on
//! its line rather than on the nearest few, so the coefficients are solved once per function, at
//! parse time, and evaluation reads only the four nearest of them per input.
//!
//! # How it is held
//!
//! A spline through samples spaced one index apart is a sum of uniform cubic B-splines, one per
//! knot plus one beyond each end, so a line of `n` samples becomes `n + 2` coefficients and a value
//! is a weighted sum of the four around it. The not-a-knot interpolant lies in that space, and its
//! coefficients come from its second derivatives `M` at the samples: `c[j] = y[j] - M[j] / 6`, with
//! one coefficient beyond each end fixed by the second derivative there. Widening each input
//! dimension in turn gives the tensor product, because each widening is linear and they commute.
//!
//! Interpolating the decoded samples rather than the raw ones is the same answer as the clause's
//! order (interpolate, then decode): `/Decode` is affine and the spline's weights sum to one.

/// The coefficient table of a tensor-product not-a-knot spline through `samples`.
///
/// `samples` is laid out as §7.10.2 lays a sample table out, the first input varying fastest, with
/// `outputs` values per sample innermost. Every entry of `size` is at least four, which the caller
/// has checked under the clause's own sentence. The result is laid out the same way over
/// `size[k] + 2` coefficients per dimension; [`coefficient_count`] is its length.
pub(super) fn coefficients(samples: &[f32], size: &[usize], outputs: usize) -> Vec<f32> {
    let mut table: Vec<f64> = samples.iter().copied().map(f64::from).collect();
    let mut extent: Vec<usize> = size.to_vec();
    for axis in 0..extent.len() {
        let line = extent.get(axis).copied().unwrap_or(0);
        // How far apart two samples on one line are, and how many lines there are beyond it.
        let stride = extent
            .iter()
            .take(axis)
            .fold(outputs, |acc, n| acc.saturating_mul(*n));
        let beyond = extent
            .iter()
            .skip(axis.saturating_add(1))
            .fold(1usize, |acc, n| acc.saturating_mul(*n));
        let widened_line = line.saturating_add(2);
        let mut widened = vec![0.0f64; stride.saturating_mul(widened_line).saturating_mul(beyond)];
        let mut values = vec![0.0f64; line];
        let mut out = vec![0.0f64; widened_line];
        for outer in 0..beyond {
            for inner in 0..stride {
                for (j, value) in values.iter_mut().enumerate() {
                    let at = inner.saturating_add(
                        stride.saturating_mul(j.saturating_add(line.saturating_mul(outer))),
                    );
                    *value = table.get(at).copied().unwrap_or(0.0);
                }
                widen_line(&values, &mut out);
                for (p, value) in out.iter().enumerate() {
                    let at = inner.saturating_add(
                        stride.saturating_mul(p.saturating_add(widened_line.saturating_mul(outer))),
                    );
                    if let Some(slot) = widened.get_mut(at) {
                        *slot = *value;
                    }
                }
            }
        }
        table = widened;
        if let Some(slot) = extent.get_mut(axis) {
            *slot = widened_line;
        }
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the coefficients are computed in f64 for the solve and held in f32 as the \
                  samples are"
    )]
    table.into_iter().map(|value| value as f32).collect()
}

/// How many coefficients [`coefficients`] returns for a table of `size` with `outputs` values per
/// sample, or `None` where the count overflows.
pub(super) fn coefficient_count(size: &[usize], outputs: usize) -> Option<usize> {
    size.iter()
        .try_fold(outputs, |acc, n| acc.checked_mul(n.checked_add(2)?))
}

/// One line of `n >= 4` samples as the `n + 2` B-spline coefficients of its not-a-knot spline.
///
/// The second derivatives come from the interpolation conditions at the interior samples,
/// `M[j-1] + 4 M[j] + M[j+1] = 6 (y[j+1] - 2 y[j] + y[j-1])`, with not-a-knot's two extra
/// conditions — the third derivative continuous at the second sample and at the second-to-last,
/// `M[0] = 2 M[1] - M[2]` and its mirror. Substituted into the first and last equations they leave
/// `M[1]` and `M[n-2]` equal to the second differences there, and the equations between are a
/// tridiagonal system with diagonal 4 and off-diagonal 1: strictly diagonally dominant, so
/// elimination without pivoting is stable.
fn widen_line(y: &[f64], out: &mut [f64]) {
    let n = y.len();
    let at = |j: usize| y.get(j).copied().unwrap_or(0.0);
    let second_difference =
        |j: usize| at(j.saturating_add(1)) - 2.0 * at(j) + at(j.saturating_sub(1));
    let mut m = vec![0.0f64; n];
    let last_interior = n.saturating_sub(2);
    if let Some(slot) = m.get_mut(1) {
        *slot = second_difference(1);
    }
    if let Some(slot) = m.get_mut(last_interior) {
        *slot = second_difference(last_interior);
    }
    // The unknowns M[2] ..= M[n-3]; none for four samples, where both ends are already fixed.
    let first = 2usize;
    let unknowns = n.saturating_sub(4);
    if unknowns > 0 {
        let mut diagonal = vec![4.0f64; unknowns];
        let mut rhs: Vec<f64> = (0..unknowns)
            .map(|k| 6.0 * second_difference(first.saturating_add(k)))
            .collect();
        if let Some(value) = rhs.first_mut() {
            *value -= m.get(1).copied().unwrap_or(0.0);
        }
        if let Some(value) = rhs.last_mut() {
            *value -= m.get(last_interior).copied().unwrap_or(0.0);
        }
        // Thomas's algorithm: forward elimination, then back substitution.
        for k in 1..unknowns {
            let ratio = 1.0 / diagonal.get(k.saturating_sub(1)).copied().unwrap_or(4.0);
            if let Some(d) = diagonal.get_mut(k) {
                *d -= ratio;
            }
            let previous = rhs.get(k.saturating_sub(1)).copied().unwrap_or(0.0);
            if let Some(r) = rhs.get_mut(k) {
                *r -= ratio * previous;
            }
        }
        let mut next = 0.0f64;
        for k in (0..unknowns).rev() {
            let value = (rhs.get(k).copied().unwrap_or(0.0)
                - if k.saturating_add(1) < unknowns {
                    next
                } else {
                    0.0
                })
                / diagonal.get(k).copied().unwrap_or(4.0);
            if let Some(slot) = m.get_mut(first.saturating_add(k)) {
                *slot = value;
            }
            next = value;
        }
    }
    let m_at = |j: usize| m.get(j).copied().unwrap_or(0.0);
    let end = n.saturating_sub(1);
    let start = 2.0 * m_at(1) - m_at(2);
    let mirrored = 2.0 * m_at(last_interior) - m_at(n.saturating_sub(3));
    if let Some(slot) = m.get_mut(0) {
        *slot = start;
    }
    if let Some(slot) = m.get_mut(end) {
        *slot = mirrored;
    }

    // `out[p]` is the coefficient of the B-spline centred on sample `p - 1`.
    for j in 0..n {
        if let Some(slot) = out.get_mut(j.saturating_add(1)) {
            *slot = at(j) - m.get(j).copied().unwrap_or(0.0) / 6.0;
        }
    }
    let c = |p: usize| out.get(p).copied().unwrap_or(0.0);
    let before = m.first().copied().unwrap_or(0.0) + 2.0 * c(1) - c(2);
    let after = m.get(end).copied().unwrap_or(0.0) + 2.0 * c(n) - c(n.saturating_sub(1));
    if let Some(slot) = out.first_mut() {
        *slot = before;
    }
    if let Some(slot) = out.get_mut(n.saturating_add(1)) {
        *slot = after;
    }
}

/// The four uniform cubic B-spline weights at `t` in `[0, 1]` of the cell they span.
///
/// They sum to one for every `t`, which is what makes the spline reproduce a constant and lets
/// `/Decode` be applied before interpolation rather than after.
pub(super) fn weights(t: f32) -> [f32; 4] {
    let s = 1.0 - t;
    let t2 = t * t;
    let t3 = t2 * t;
    [
        s * s * s / 6.0,
        (3.0 * t3 - 6.0 * t2 + 4.0) / 6.0,
        (-3.0 * t3 + 3.0 * t2 + 3.0 * t + 1.0) / 6.0,
        t3 / 6.0,
    ]
}
