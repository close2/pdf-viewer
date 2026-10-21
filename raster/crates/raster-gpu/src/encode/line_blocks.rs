//! Conservative bounds for consecutive resident line instances. Selection changes only draw
//! ranges: the original instance bytes, their order, and the shader's arithmetic remain intact.

use super::instance::{LINE_INSTANCE_BYTES, LINE_INSTANCE_STRIDE, LineView};

/// Instances covered by one bound: the table costs one 32-byte bound per 15,360 stream bytes.
pub(super) const BLOCK: u32 = 256;

/// The resident stream's spatial bounds, shared by all its kept encodes.
#[derive(Debug, PartialEq)]
pub(super) struct LineBlocks {
    bounds: Vec<Bounds>,
}

impl LineBlocks {
    /// Allocation charged before construction, including the vector and its Arc counters.
    pub(super) fn allocation_bytes(stream_bytes: usize) -> u64 {
        let instances = (stream_bytes as u64).saturating_div(LINE_INSTANCE_STRIDE);
        instances
            .div_ceil(u64::from(BLOCK))
            .saturating_mul(size_of::<Bounds>() as u64)
            .saturating_add(size_of::<Self>() as u64)
            .saturating_add(2_u64.saturating_mul(size_of::<usize>() as u64))
    }

    /// Build only after the caller has admitted `allocation_bytes` against its frame budget.
    pub(super) fn of(stream: &[u8]) -> Self {
        let stride = LINE_INSTANCE_BYTES;
        let bounds = stream
            .chunks(stride.saturating_mul(BLOCK as usize))
            .map(|block| {
                block
                    .chunks_exact(stride)
                    .map(Bounds::of)
                    .reduce(Bounds::union)
                    .unwrap_or(Bounds::UNKNOWN)
            })
            .collect();
        Self { bounds }
    }

    /// Whether the block containing `instance` might reach this view. An absent bound keeps it.
    pub(super) fn admitted(&self, instance: u32, view: LineView) -> bool {
        self.bounds
            .get(instance.saturating_div(BLOCK) as usize)
            .is_none_or(|bound| bound.admitted(view))
    }

    /// Allocation held by this table, including its shared owner.
    pub(super) fn bytes(&self) -> u64 {
        (self.bounds.capacity() as u64)
            .saturating_mul(size_of::<Bounds>() as u64)
            .saturating_add(size_of::<Self>() as u64)
            .saturating_add(2_u64.saturating_mul(size_of::<usize>() as u64))
    }
}

/// Endpoint extrema and maximum scaled/fixed pen radii. Adjusted pens contribute to both.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Bounds {
    low: [f32; 2],
    high: [f32; 2],
    scaled: [f32; 2],
    fixed: [f32; 2],
}

impl Bounds {
    const UNKNOWN: Self = Self {
        low: [f32::NEG_INFINITY; 2],
        high: [f32::INFINITY; 2],
        scaled: [f32::INFINITY; 2],
        fixed: [f32::INFINITY; 2],
    };

    /// The first seven little-endian floats are endpoints, pen radii, and packed cap/resolution.
    fn of(instance: &[u8]) -> Self {
        let mut fields = [0.0_f32; 7];
        for (field, bytes) in fields.iter_mut().zip(instance.chunks_exact(4)) {
            *field = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        }
        if fields.iter().any(|value| !value.is_finite()) {
            return Self::UNKNOWN;
        }
        let [x0, y0, x1, y1, hx, hy, packed] = fields;
        let fixed = packed >= 6.0;
        Self {
            low: [x0.min(x1), y0.min(y1)],
            high: [x0.max(x1), y0.max(y1)],
            scaled: if fixed { [0.0; 2] } else { [hx, hy] },
            fixed: if fixed {
                [hx, hy]
            } else if packed >= 3.0 {
                [0.5; 2]
            } else {
                [0.0; 2]
            },
        }
    }

    fn union(mut self, other: Self) -> Self {
        for axis in 0..2 {
            self.low[axis] = self.low[axis].min(other.low[axis]);
            self.high[axis] = self.high[axis].max(other.high[axis]);
            self.scaled[axis] = self.scaled[axis].max(other.scaled[axis]);
            self.fixed[axis] = self.fixed[axis].max(other.fixed[axis]);
        }
        self
    }

    /// Test only the viewport. The shader still maps and intersects the instance's own clip.
    fn admitted(self, LineView(view): LineView) -> bool {
        let mut outside = false;
        let axes = [
            [view[0], view[2], view[4], view[6]],
            [view[1], view[3], view[5], view[7]],
        ];
        for (axis, values) in axes.into_iter().enumerate() {
            let [a, b, offset, extent] = values.map(f64::from);
            let x = [a * f64::from(self.low[0]), a * f64::from(self.high[0])];
            let y = [b * f64::from(self.low[1]), b * f64::from(self.high[1])];
            let half = (a.abs() * f64::from(self.scaled[0]) + b.abs() * f64::from(self.scaled[1]))
                .max(f64::from(self.fixed[axis]));
            let reach = half * f64::from(1.4143_f32);
            let magnitude =
                x[0].abs().max(x[1].abs()) + y[0].abs().max(y[1].abs()) + offset.abs() + reach;
            // Endpoint mapping, pen mapping, reach multiplication and expansion each use a
            // short f32 chain in line.wgsl. Sixteen epsilons of the sum of magnitudes bounds
            // its rounding under fused or separate operations; the absolute term also covers
            // flushed subnormals. Keep uncertain/overflowing arithmetic, in either axis.
            let guard =
                magnitude * (16.0 * f64::from(f32::EPSILON)) + f64::from(f32::MIN_POSITIVE) * 16.0;
            let low = x[0].min(x[1]) + y[0].min(y[1]) + offset - reach - guard;
            let high = x[0].max(x[1]) + y[0].max(y[1]) + offset + reach + guard;
            if !low.is_finite() || !high.is_finite() || magnitude > f64::from(f32::MAX) {
                return true;
            }
            outside |= high < 0.0 || low > extent;
        }
        !outside
    }
}

#[cfg(test)]
#[path = "../../tests/common/line_bounds.rs"]
mod tests;
