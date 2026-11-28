//! Table 291's `/F 3`, SMIL's `scroll`: "The media shall not be scaled. A scrolling user interface
//! shall be provided if the media rectangle is wider or taller than the play rectangle." (ISO
//! 32000-2 §13.2.5; ADR 3901.)
//!
//! **The interface is the wheel over the play rectangle** — a choice, since the table names the
//! requirement and not its form. Every window that shows pictures already reads its wheel (or a
//! touchpad's two-finger travel, which the toolkits deliver as the same event) as a distance in
//! pixels; over the rectangle of a picture that overflows it, that distance moves the part of the
//! picture shown instead of the page, one sample a pixel as the picture is unscaled, and it stops
//! at each edge of the picture. A drag would compete with a press on the page's own annotations
//! and the controller bar, and a drawn scroll bar would cover the picture it scrolls in a rectangle
//! that may be a few dozen pixels across; the wheel costs neither. A picture that fits takes no
//! wheel: "if the media rectangle is wider or taller than the play rectangle" is the condition, so
//! no overflow, no interface.
//!
//! The geometry is [`super::placed`]'s, the part shown moved by the samples scrolled: the window
//! draws [`Placed`] as it draws every fit, so the device layer, the processor window, GTK, Qt and
//! the C ABI's window scroll through the one construction that places the picture.

use super::Placed;

/// [`super::placed_turned`] with an `/F 3` picture's part shown begun `scrolled` samples from its
/// top-left corner — across its rows and down its columns, as the picture holds them — held so the
/// part stays inside the picture. Every other fit, and a picture that fits its rectangle, is placed
/// as [`super::placed_turned`] places it, whatever `scrolled` says.
#[must_use]
pub fn placed_scrolled(
    fit: pdf_model::media::Fit,
    size: (u32, u32),
    quad: [f32; 8],
    quarter_turns: u8,
    scrolled: (u32, u32),
) -> Option<Placed> {
    let mut placed = super::placed_turned(fit, size, quad, quarter_turns)?;
    if fit == pdf_model::media::Fit::Scroll {
        let (across, down) = placed.overflow(size);
        let (left, top) = (scrolled.0.min(across), scrolled.1.min(down));
        placed.left = left;
        placed.right = placed.right.saturating_add(left);
        placed.top = top;
        placed.bottom = placed.bottom.saturating_add(top);
    }
    Some(placed)
}

impl Placed {
    /// How many samples of a picture of `size` the part shown leaves out, across and down: the
    /// farthest the part can be scrolled. `(0, 0)` where the picture fits.
    #[must_use]
    pub const fn overflow(&self, size: (u32, u32)) -> (u32, u32) {
        (
            size.0.saturating_sub(self.right.saturating_sub(self.left)),
            size.1.saturating_sub(self.bottom.saturating_sub(self.top)),
        )
    }

    /// A distance `by` in window pixels — the wheel's, positive to the right and down, the way
    /// the view moves over the content — as a distance in the picture's samples: through the
    /// inverse of [`Self::affine`]'s linear part, so a turned or mirrored picture scrolls along
    /// its own rows and columns. `None` where the placement has no extent to invert.
    #[must_use]
    pub fn in_samples(&self, by: (f32, f32)) -> Option<(f64, f64)> {
        let [row_x, row_y, column_x, column_y, _, _] = self.affine().map(f64::from);
        let determinant = row_x.mul_add(column_y, -(row_y * column_x));
        if determinant.abs() < f64::EPSILON {
            return None;
        }
        let (across, down) = (f64::from(by.0), f64::from(by.1));
        Some((
            column_y.mul_add(across, -(column_x * down)) / determinant,
            row_x.mul_add(down, -(row_y * across)) / determinant,
        ))
    }

    /// Whether `point`, in window pixels, falls on the play rectangle `quad` the part is placed in.
    #[must_use]
    pub fn within(quad: [f32; 8], point: (f32, f32)) -> bool {
        let [x0, y0, x1, y1, x2, y2, x3, y3] = quad;
        let corners = [(x0, y0), (x1, y1), (x2, y2), (x3, y3)];
        let edges = corners.iter().zip(corners.iter().cycle().skip(1));
        // A convex quadrilateral holds a point where the point is on the same side of all four
        // edges; the corners may run either way round, so the sign is the first edge's.
        let mut side = 0.0_f32;
        for (&(from_x, from_y), &(to_x, to_y)) in edges {
            let cross =
                (to_x - from_x).mul_add(point.1 - from_y, -((to_y - from_y) * (point.0 - from_x)));
            if cross != 0.0 {
                if side != 0.0 && cross.signum() != side.signum() {
                    return false;
                }
                side = cross;
            }
        }
        true
    }
}

/// Where an `/F 3` picture is scrolled to, in samples from its top-left: kept fractional, so a
/// touchpad's travel of less than a pixel an event is not lost, and read whole by the placement.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct Scrolled {
    across: f64,
    down: f64,
}

impl Scrolled {
    /// The part's first column and row, to the nearest sample.
    pub(super) fn whole(self) -> (u32, u32) {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "held between zero and a picture's side by `moved`, far inside u32's range"
        )]
        let whole = |value: f64| value.round().max(0.0) as u32;
        (whole(self.across), whole(self.down))
    }

    /// Moved `by` samples, held between the picture's top-left and the farthest `overflow` lets
    /// the part go.
    pub(super) fn moved(self, by: (f64, f64), overflow: (u32, u32)) -> Self {
        Self {
            across: (self.across + by.0).clamp(0.0, f64::from(overflow.0)),
            down: (self.down + by.1).clamp(0.0, f64::from(overflow.1)),
        }
    }
}
