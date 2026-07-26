//! Smooth colour transitions, resolved to a device-independent form.
//!
//! PDF defines seven shading types. They arrive here with their colour spaces already
//! resolved to RGB and their functions already evaluated, for the same reason [`Color`]
//! does: colour management belongs in one place, upstream, so that the backends cannot
//! disagree about it.
//!
//! What survives that resolution is *geometry* — where a colour transition starts and
//! ends — plus the colours it passes through. That is what a backend needs and all it
//! needs.
//!
//! # Why the types collapse into four
//!
//! The specification's seven types describe three different things. Axial (2) and radial
//! (3) are the two the underlying rasterisers implement natively, so they stay distinct.
//! Function-based shadings (1) are an arbitrary function of two variables and reduce to a
//! grid of samples. The four mesh types (4, 5, 6, 7) all describe the same thing — patches
//! of smoothly varying colour — and differ only in how the file writes them down; Coons
//! and tensor patches are subdivided into the triangles that types 4 and 5 give directly,
//! so all four arrive here as triangles.
//!
//! Nothing is lost by that grouping except the name of the type, which no backend needs.

use std::sync::Arc;

use crate::geom::{Point, Transform};
use crate::paint::Color;

/// A colour transition, together with the space it is defined in.
#[derive(Debug, Clone, PartialEq)]
pub struct Shading {
    /// The geometry of the transition, in the shading's own coordinates.
    pub kind: ShadingKind,
    /// Maps the shading's own coordinates into the space the command is drawn in.
    ///
    /// Separate from the drawn path's transform because they are genuinely different: a
    /// shading used as a pattern is positioned by the *pattern* matrix relative to the
    /// page, not by the transform in force when the path was filled.
    pub transform: Transform,
}

/// The geometry of a colour transition.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ShadingKind {
    /// Colour varies along a line, perpendicular to it (PDF type 2).
    Axial {
        /// Where the ramp's first colour sits.
        start: Point,
        /// Where its last colour sits.
        end: Point,
        /// The colours passed through.
        ramp: Ramp,
        /// Whether the shading continues beyond `start` and beyond `end`.
        ///
        /// Where it does not, nothing is painted there at all — which is not the same as
        /// painting the end colour, and is the difference between a band and a wash.
        extend: (bool, bool),
    },
    /// Colour varies between two circles (PDF type 3).
    Radial {
        /// Centre of the circle carrying the ramp's first colour.
        start: Point,
        /// Its radius.
        start_radius: f32,
        /// Centre of the circle carrying the ramp's last colour.
        end: Point,
        /// Its radius.
        end_radius: f32,
        /// The colours passed through.
        ramp: Ramp,
        /// Whether the shading continues beyond each circle.
        extend: (bool, bool),
    },
    /// Colour is an arbitrary function of position, sampled on a grid (PDF type 1).
    ///
    /// Reduced to samples because the display list must not hold a PDF function: the
    /// function machinery lives above this crate, and a display list has to be plain data
    /// so it can cross a process boundary. The grid is generous enough that the
    /// interpolation between samples is not visible at ordinary magnifications, and this
    /// is the one place in the display list where resolution is baked in.
    Sampled {
        /// The rectangle the samples cover, as `[x0, x1, y0, y1]`.
        domain: [f32; 4],
        /// Samples across the domain.
        width: u32,
        /// Samples down the domain.
        height: u32,
        /// Row-major samples, `width * height` of them.
        pixels: Arc<[Color]>,
    },
    /// Colour varies smoothly across triangles (PDF types 4, 5, 6 and 7).
    Mesh {
        /// The triangles, each carrying a colour per corner.
        triangles: Arc<[Triangle]>,
    },
}

/// A colour ramp, sampled at even intervals.
///
/// PDF states a shading's colours as a function of one parameter. Sampling it evenly is
/// what lets a backend hand the result to a gradient implementation, which is how both
/// rasterisers draw these natively and quickly.
#[derive(Debug, Clone, PartialEq)]
pub struct Ramp {
    /// Colours from the start of the transition to its end. Never empty.
    pub colours: Arc<[Color]>,
}

impl Ramp {
    /// How many samples a ramp carries.
    ///
    /// Enough that a full-page gradient has more samples than it has pixels of gradient
    /// axis at ordinary magnifications, so the sampling is not what limits fidelity. The
    /// cost is one array of this length per shading, built once.
    pub const RESOLUTION: usize = 256;

    /// Builds a ramp by sampling a colour function over `0.0..=1.0`.
    ///
    /// The function is called [`Self::RESOLUTION`] times and never afterwards, which is
    /// what keeps PDF functions out of the display list.
    #[must_use]
    pub fn sample(mut colour_at: impl FnMut(f32) -> Color) -> Self {
        #[expect(
            clippy::cast_precision_loss,
            reason = "RESOLUTION is a small constant, exactly representable"
        )]
        let last = (Self::RESOLUTION.saturating_sub(1)) as f32;
        let colours: Vec<Color> = (0..Self::RESOLUTION)
            .map(|index| {
                #[expect(clippy::cast_precision_loss, reason = "index is bounded by RESOLUTION")]
                let t = index as f32 / last;
                colour_at(t)
            })
            .collect();
        Self {
            colours: colours.into(),
        }
    }

    /// Returns the colour at a position in `0.0..=1.0`, interpolating between samples.
    #[must_use]
    pub fn colour_at(&self, t: f32) -> Color {
        let last = self.colours.len().saturating_sub(1);
        if last == 0 {
            return self.colours.first().copied().unwrap_or(Color::BLACK);
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "the ramp length is RESOLUTION, a small constant"
        )]
        let scaled = t.clamp(0.0, 1.0) * last as f32;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "scaled is clamped to 0..=last"
        )]
        let low = scaled.floor() as usize;
        let high = low.saturating_add(1).min(last);
        let fraction = scaled - scaled.floor();
        let a = self.colours.get(low).copied().unwrap_or(Color::BLACK);
        let b = self.colours.get(high).copied().unwrap_or(a);
        Color {
            r: a.r + (b.r - a.r) * fraction,
            g: a.g + (b.g - a.g) * fraction,
            b: a.b + (b.b - a.b) * fraction,
            a: a.a + (b.a - a.a) * fraction,
        }
    }
}

/// One triangle of a mesh shading, with a colour at each corner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Triangle {
    /// The corners, in the shading's own coordinates.
    pub points: [Point; 3],
    /// The colour at each corner, in the same order.
    pub colours: [Color; 3],
}

impl Triangle {
    /// Whether the three corner colours are close enough to draw as one flat colour.
    ///
    /// Backends that cannot interpolate colour across a triangle subdivide until this
    /// holds. The threshold is below what an eight-bit channel can represent, so the
    /// result is indistinguishable from true interpolation once quantised.
    #[must_use]
    pub fn is_flat(&self, tolerance: f32) -> bool {
        let [first, second, third] = self.colours;
        let spread = |get: fn(&Color) -> f32| {
            let (x, y, z) = (get(&first), get(&second), get(&third));
            x.max(y).max(z) - x.min(y).min(z)
        };
        spread(|colour| colour.r) <= tolerance
            && spread(|colour| colour.g) <= tolerance
            && spread(|colour| colour.b) <= tolerance
            && spread(|colour| colour.a) <= tolerance
    }

    /// Returns the average of the three corner colours.
    #[must_use]
    pub fn average_colour(&self) -> Color {
        let [first, second, third] = self.colours;
        Color {
            r: (first.r + second.r + third.r) / 3.0,
            g: (first.g + second.g + third.g) / 3.0,
            b: (first.b + second.b + third.b) / 3.0,
            a: (first.a + second.a + third.a) / 3.0,
        }
    }

    /// Splits the triangle into four by halving each edge.
    ///
    /// Four rather than two so that subdivision stays symmetric: splitting on one edge
    /// repeatedly produces slivers, which rasterise with visible seams.
    #[must_use]
    pub fn subdivide(&self) -> [Self; 4] {
        let midpoint = |start: Point, end: Point| {
            Point::new(f32::midpoint(start.x, end.x), f32::midpoint(start.y, end.y))
        };
        let blend = |start: Color, end: Color| Color {
            r: f32::midpoint(start.r, end.r),
            g: f32::midpoint(start.g, end.g),
            b: f32::midpoint(start.b, end.b),
            a: f32::midpoint(start.a, end.a),
        };
        let [p0, p1, p2] = self.points;
        let [c0, c1, c2] = self.colours;
        let (m01, m12, m20) = (midpoint(p0, p1), midpoint(p1, p2), midpoint(p2, p0));
        let (b01, b12, b20) = (blend(c0, c1), blend(c1, c2), blend(c2, c0));

        [
            Self {
                points: [p0, m01, m20],
                colours: [c0, b01, b20],
            },
            Self {
                points: [m01, p1, m12],
                colours: [b01, c1, b12],
            },
            Self {
                points: [m20, m12, p2],
                colours: [b20, b12, c2],
            },
            Self {
                points: [m01, m12, m20],
                colours: [b01, b12, b20],
            },
        ]
    }
}
