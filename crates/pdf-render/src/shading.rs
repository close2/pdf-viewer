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
    ///
    /// Shared rather than owned because one shading object is commonly painted many times —
    /// a pattern filling every cell of a chart, or an `sh` inside a form invoked once per
    /// data point — and each of those is the same colours under a different transform.
    /// `bug1721218_reduced.pdf` paints 3576 of them from three function objects, which is
    /// why this is an `Arc` and `pdf_model::shading::Cache` exists: building the kind again
    /// per use was 14% of that page (ADR 0069).
    pub kind: Arc<ShadingKind>,
    /// Maps the shading's own coordinates into the space the command is drawn in.
    ///
    /// Separate from the drawn path's transform because they are genuinely different: a
    /// shading used as a pattern is positioned by the *pattern* matrix relative to the
    /// page, not by the transform in force when the path was filled.
    pub transform: Transform,
}

impl Shading {
    /// Whether every colour this shading can paint is fully opaque.
    ///
    /// Asked for the same reason as [`crate::Image::is_opaque`]: §11.4.6's knockout differs
    /// from ordinary compositing only where the upper object is not opaque, and a shading's
    /// alpha lives in its colours rather than in a single field a caller can read.
    ///
    /// A shading that does not extend leaves part of its region unpainted, which is a shape
    /// of zero rather than an opacity, so it is not what this answers.
    #[must_use]
    pub fn is_opaque(&self) -> bool {
        let opaque = |colour: &Color| colour.a >= 1.0;
        match self.kind.as_ref() {
            ShadingKind::Axial { ramp, .. } | ShadingKind::Radial { ramp, .. } => {
                ramp.stops.iter().all(|stop| opaque(&stop.colour))
            }
            ShadingKind::Sampled { pixels, .. } => pixels.iter().all(opaque),
            ShadingKind::Mesh { triangles } => triangles
                .iter()
                .all(|triangle| triangle.colours.iter().all(opaque)),
        }
    }

    /// Returns this shading with every colour's alpha scaled by `alpha`.
    ///
    /// A shading is the one paint whose colours are not a single [`Color`] the caller can
    /// modify, so a constant alpha has to reach *every* colour it carries or reach none of
    /// them. ISO 32000-2 §11.6.4.4 makes it every: the alpha constants are a property of the
    /// graphics state applied to "all other painting operations" — a path filled with a
    /// shading pattern and an `sh` alike — rather than of the colour being painted.
    ///
    /// The clone is deliberate and paid for only where `alpha` is below 1: shadings are
    /// shared behind an `Arc` because one pattern commonly paints many paths, and a
    /// half-transparent fill of that pattern is a different paint from an opaque one.
    #[must_use]
    pub fn with_alpha(&self, alpha: f32) -> Self {
        let scale = |colour: &Color| Color {
            a: colour.a * alpha,
            ..*colour
        };
        let ramp = |ramp: &Ramp| Ramp {
            stops: ramp
                .stops
                .iter()
                .map(|stop| Stop {
                    at: stop.at,
                    colour: scale(&stop.colour),
                })
                .collect(),
        };
        let kind = match self.kind.as_ref() {
            ShadingKind::Axial {
                start,
                end,
                ramp: colours,
                extend,
            } => ShadingKind::Axial {
                start: *start,
                end: *end,
                ramp: ramp(colours),
                extend: *extend,
            },
            ShadingKind::Radial {
                start,
                start_radius,
                end,
                end_radius,
                ramp: colours,
                extend,
            } => ShadingKind::Radial {
                start: *start,
                start_radius: *start_radius,
                end: *end,
                end_radius: *end_radius,
                ramp: ramp(colours),
                extend: *extend,
            },
            ShadingKind::Sampled {
                domain,
                width,
                height,
                pixels,
            } => ShadingKind::Sampled {
                domain: *domain,
                width: *width,
                height: *height,
                pixels: pixels.iter().map(scale).collect(),
            },
            ShadingKind::Mesh { triangles } => ShadingKind::Mesh {
                triangles: triangles
                    .iter()
                    .map(|triangle| Triangle {
                        points: triangle.points,
                        colours: triangle.colours.map(|colour| scale(&colour)),
                    })
                    .collect(),
            },
        };
        Self {
            kind: Arc::new(kind),
            transform: self.transform,
        }
    }
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

/// A colour ramp: positions along a shading's parameter, each with a colour.
///
/// PDF states a shading's colours as a function of one parameter. Sampling it is what lets a
/// backend hand the result to a gradient implementation, which is how both rasterisers draw
/// these natively and quickly.
///
/// # Why the positions are carried rather than implied
///
/// An evenly spaced array cannot express a *step*. §8.7.4.5.3's colour at a point is whatever
/// the function says, and a type 3 stitching function with two equal `/Bounds` says green up to
/// one point and blue after it — a discontinuity, exactly representable by two stops at the
/// same position and not representable at all by samples that are averaged between.
/// `issue10572.pdf` is the page that made the difference visible: 24 hard stripes drawn as
/// seven-pixel gradients, because 256 even samples over an 1800-unit axis put a sample every
/// seven pixels and every step landed inside one interval.
#[derive(Debug, Clone, PartialEq)]
pub struct Ramp {
    /// The stops, in ascending position order, spanning `0.0..=1.0`. Never empty.
    pub stops: Arc<[Stop]>,
}

/// How far a dropped stop may sit from the line its neighbours draw, per channel.
///
/// Both rasterisers interpolate a gradient linearly between consecutive stops and deliver eight
/// bits per channel, so a stop within **half a level** of the line through the stops either side
/// of it produces the same byte whether it is there or not. `1.0 / 512.0` is that half level in
/// the `0.0..=1.0` this crate's [`Color`] uses.
///
/// It is a *lossless* bound rather than a quality knob: raising it would start changing pixels,
/// which is why it is stated as a fraction of a level and not as a tolerance.
const COLLINEAR: f32 = 1.0 / 512.0;

/// Drops the stops a rasteriser would have computed anyway.
///
/// [`Ramp::sample_across`] samples a colour function at [`Ramp::RESOLUTION`] positions because
/// that is the resolution at which a *function* has to be believed. What a gradient needs is
/// something else: the positions where the colour stops being a straight line. A shading whose
/// function is an exponential with `/N 1` — which is most of them, and every `/FunctionType 2`
/// interpolation between two colours — is one straight line and needs **two** stops, not 256.
///
/// The cost of the difference is not in building the ramp. `tiny-skia` walks a gradient's stop
/// list per pixel batch, so 256 stops is 128 times the search 2 stops is, and on
/// `bug1721218_reduced.pdf` `tiny_skia::pipeline::lowp::gradient` was **68% of a 144 G
/// instruction page**. Vello's shader does the same walk on the GPU.
///
/// The rule is exact rather than approximate: a stop is dropped only where every dropped stop
/// lies within [`COLLINEAR`] of the line the surviving neighbours draw, and both backends
/// interpolate linearly between those neighbours — so the colour a rasteriser computes at every
/// position is the same to eight bits. Two stops at one position, which is how
/// [`Ramp::sample_across`] expresses a discontinuity, are never collapsed into one: a vertical
/// segment fails the test at once.
fn simplify(stops: &[Stop]) -> Vec<Stop> {
    let Some(first) = stops.first().copied() else {
        return Vec::new();
    };
    let mut out = vec![first];
    let mut anchor = 0usize;
    let mut index = 1usize;
    while index < stops.len() {
        // Extend the run while every stop between the anchor and the candidate lies on the
        // line the two of them draw. Checking *all* of them, rather than only the one being
        // dropped, is what stops the error accumulating over a long run.
        let mut end = index;
        while end.saturating_add(1) < stops.len() {
            let next = end.saturating_add(1);
            let (Some(&start), Some(&finish)) = (stops.get(anchor), stops.get(next)) else {
                break;
            };
            let straight = stops
                .get(anchor.saturating_add(1)..next)
                .unwrap_or_default()
                .iter()
                .all(|middle| on_the_line(start, finish, *middle));
            if !straight {
                break;
            }
            end = next;
        }
        if let Some(&keep) = stops.get(end) {
            out.push(keep);
        }
        anchor = end;
        index = end.saturating_add(1);
    }
    out
}

/// Whether `middle` is what linear interpolation between `start` and `finish` would give.
fn on_the_line(start: Stop, finish: Stop, middle: Stop) -> bool {
    let span = finish.at - start.at;
    // A zero-width span is a discontinuity, and a stop inside one has no line to lie on. A
    // NaN position is neither, and `<=` answers false for it, which is the same refusal.
    if !span.is_finite() || span <= 0.0 {
        return false;
    }
    let fraction = ((middle.at - start.at) / span).clamp(0.0, 1.0);
    let between = |a: f32, b: f32| a + (b - a) * fraction;
    (middle.colour.r - between(start.colour.r, finish.colour.r)).abs() <= COLLINEAR
        && (middle.colour.g - between(start.colour.g, finish.colour.g)).abs() <= COLLINEAR
        && (middle.colour.b - between(start.colour.b, finish.colour.b)).abs() <= COLLINEAR
        && (middle.colour.a - between(start.colour.a, finish.colour.a)).abs() <= COLLINEAR
}

/// One entry of a [`Ramp`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stop {
    /// Where along the shading's parameter this colour applies, in `0.0..=1.0`.
    pub at: f32,
    /// The colour there.
    pub colour: Color,
}

impl Ramp {
    /// How many samples a ramp carries.
    ///
    /// Enough that a full-page gradient has more samples than it has pixels of gradient
    /// axis at ordinary magnifications, so the sampling is not what limits fidelity. The
    /// cost is one array of this length per shading, built once.
    pub const RESOLUTION: usize = 256;

    /// Builds a ramp by sampling a colour function evenly over `0.0..=1.0`.
    ///
    /// The function is called [`Self::RESOLUTION`] times and never afterwards, which is
    /// what keeps PDF functions out of the display list.
    #[must_use]
    pub fn sample(colour_at: impl FnMut(f32) -> Color) -> Self {
        Self::sample_across(&[], colour_at)
    }

    /// The same, with positions at which the function is known to be discontinuous.
    ///
    /// `breaks` are positions in `0.0..=1.0` — for a PDF shading, a type 3 function's
    /// `/Bounds` mapped onto the shading's own domain. Each gets **two** stops at the same
    /// position, taken from just below and just above it, which is how a gradient expresses a
    /// step; the samples between two breaks are spread evenly over the interval, so a ramp
    /// with no breaks is exactly what [`Self::sample`] used to build.
    ///
    /// The total number of stops stays near [`Self::RESOLUTION`]: a function with many bounds
    /// gets fewer samples inside each interval rather than more stops overall, because the
    /// stops are what a rasteriser walks per pixel batch.
    #[must_use]
    pub fn sample_across(breaks: &[f32], mut colour_at: impl FnMut(f32) -> Color) -> Self {
        /// How far either side of a break the two stops are sampled.
        ///
        /// Small enough that a continuous function's two values are the same colour to eight
        /// bits, and large enough to be a distinct `f32` anywhere in the unit interval.
        const NUDGE: f32 = 1e-5;

        let mut edges: Vec<f32> = breaks
            .iter()
            .copied()
            .filter(|at| at.is_finite() && *at > 0.0 && *at < 1.0)
            .collect();
        edges.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        edges.dedup();

        // Samples per interval, so that the whole ramp is about RESOLUTION stops however many
        // intervals there are. Three is the floor: the ends of the interval and its middle.
        let intervals = edges.len().saturating_add(1);
        let per_interval = Self::RESOLUTION.checked_div(intervals).unwrap_or(3).max(3);

        let mut stops: Vec<Stop> = Vec::with_capacity(Self::RESOLUTION.saturating_add(8));
        let mut low = 0.0f32;
        for index in 0..intervals {
            let high = edges.get(index).copied().unwrap_or(1.0);
            let span = high - low;
            for step in 0..per_interval {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "step and per_interval are bounded by RESOLUTION"
                )]
                let fraction = step as f32 / (per_interval.saturating_sub(1).max(1)) as f32;
                let at = low + span * fraction;
                // The last sample of an interval belongs to the function *below* the break,
                // and the first of the next to the function above it: two stops at one
                // position, which is the step.
                let sampled = if step.saturating_add(1) == per_interval && high < 1.0 {
                    (high - NUDGE).max(low)
                } else if step == 0 && low > 0.0 {
                    (low + NUDGE).min(high)
                } else {
                    at
                };
                stops.push(Stop {
                    at: at.clamp(0.0, 1.0),
                    colour: colour_at(sampled),
                });
            }
            low = high;
        }

        Self {
            stops: simplify(&stops).into(),
        }
    }

    /// Returns the colour at a position in `0.0..=1.0`, interpolating between stops.
    #[must_use]
    pub fn colour_at(&self, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        let Some(first) = self.stops.first() else {
            return Color::BLACK;
        };
        if t <= first.at {
            return first.colour;
        }
        let mut previous = *first;
        for stop in self.stops.iter().skip(1) {
            if t <= stop.at {
                let span = stop.at - previous.at;
                let fraction = if span > 0.0 {
                    (t - previous.at) / span
                } else {
                    1.0
                };
                let (a, b) = (previous.colour, stop.colour);
                return Color {
                    r: a.r + (b.r - a.r) * fraction,
                    g: a.g + (b.g - a.g) * fraction,
                    b: a.b + (b.b - a.b) * fraction,
                    a: a.a + (b.a - a.a) * fraction,
                };
            }
            previous = *stop;
        }
        previous.colour
    }
}

/// A mesh shading rasterised into device pixels, ISO 32000-2 §8.7.4.5.5.
///
/// # Why a raster rather than a pile of flat triangles
///
/// §8.7.4.5.5 states it in one sentence, of which the load-bearing half is:
///
/// > The colour at each vertex of the triangles is specified, and a technique known as
/// > Gouraud interpolation is used to colour the interiors.
///
/// Neither `tiny-skia` nor Vello has a Gouraud primitive, so both backends used to subdivide
/// a triangle until its corner colours agreed to within 1/512 and then fill the piece flat.
/// That produced three defects at once, and `issue2948.pdf` showed all three: a visible
/// lattice where the flat pieces meet, a *bias* — a piece takes the mean of its corners,
/// which on a ramp is not the colour at any of its pixels — and, because two abutting
/// antialiased edges do not sum to full coverage, seams that had to be closed by growing
/// every piece by 0.8 pixels, which is itself a departure nobody could derive.
///
/// Rasterising the mesh once, at device resolution, removes all three. A pixel's colour is
/// the clause's own interpolation at that pixel's centre; adjacent triangles tile exactly
/// under point sampling, so there are no seams to repair; and the arithmetic is the same on
/// both backends because it is this function.
///
/// # What is given up, and to what
///
/// The mesh's own outer boundary is point-sampled and therefore *not* antialiased. In every
/// case that matters it does not show: a mesh is painted through the path being filled, and
/// that path's edge is antialiased by the backend as it always was — so the hard edge appears
/// only where a mesh ends *inside* its own shape, which is a mesh that does not cover the
/// region its document asked it to fill. That is a real, small departure and it buys the
/// removal of a larger one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshRaster {
    /// Device x of the raster's first column.
    pub left: i32,
    /// Device y of the raster's first row.
    pub top: i32,
    /// Straight-alpha RGBA8 samples, one per device pixel.
    pub image: crate::Image,
}

impl MeshRaster {
    /// Rasterises `triangles` into the part of a `width` by `height` target they cover.
    ///
    /// `to_device` carries the mesh's own coordinates onto the target. Returns `None` when
    /// the mesh covers no pixel of it, which a clipped-away or degenerate mesh does.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "every cast below is between a device pixel index and its coordinate, both                   bounded by the target's extent, which `MAX_EXTENT` keeps under 2^24"
    )]
    pub fn build(
        triangles: &[Triangle],
        to_device: Transform,
        width: u32,
        height: u32,
    ) -> Option<Self> {
        if triangles.is_empty() || width == 0 || height == 0 {
            return None;
        }
        let device: Vec<Triangle> = triangles
            .iter()
            .map(|triangle| Triangle {
                points: triangle.points.map(|point| to_device.apply(point)),
                colours: triangle.colours,
            })
            .collect();

        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for triangle in &device {
            for point in triangle.points {
                if !point.x.is_finite() || !point.y.is_finite() {
                    return None;
                }
                x0 = x0.min(point.x);
                y0 = y0.min(point.y);
                x1 = x1.max(point.x);
                y1 = y1.max(point.y);
            }
        }
        // Half a pixel of margin on each side, because a pixel is sampled at its centre and a
        // triangle ending at x = 10.0 still covers the sample at 9.5.
        let left = (x0 - 0.5).floor().max(0.0) as u32;
        let top = (y0 - 0.5).floor().max(0.0) as u32;
        let right = (x1 + 0.5).ceil().max(0.0).min(width as f32) as u32;
        let bottom = (y1 + 0.5).ceil().max(0.0).min(height as f32) as u32;
        let (span, rows) = (right.checked_sub(left)?, bottom.checked_sub(top)?);
        if span == 0 || rows == 0 {
            return None;
        }

        let mut data = vec![
            0u8;
            (span as usize)
                .saturating_mul(rows as usize)
                .saturating_mul(4)
        ];
        for triangle in &device {
            triangle.paint(&mut data, left, top, span, rows);
        }

        Some(Self {
            left: i32::try_from(left).ok()?,
            top: i32::try_from(top).ok()?,
            image: crate::Image {
                width: span,
                height: rows,
                data: data.into(),
                // Nearest sampling: the raster is already at device resolution and is drawn
                // at 1:1, so no filter can be reached — and asking for one would let a
                // backend blur the mesh against the transparent pixels outside it.
                interpolate: false,
            },
        })
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

    /// Whether the triangle is too small on the device for subdivision to change a pixel.
    ///
    /// The points must already be in device space, which is where both backends subdivide.
    ///
    /// # Why this is a correctness statement and not an approximation
    ///
    /// [`Self::is_flat`] asks whether the *colours* are close enough to fill flat, and a
    /// triangle whose corners differ keeps splitting until they are — up to `4^6` pieces,
    /// however small it is on screen. But a triangle that covers less than a pixel cannot
    /// display a gradient: the output raster has one sample there, and every sub-triangle's
    /// average would land in the same one. Splitting it produces four fills that composite
    /// to the colour the single fill already had.
    ///
    /// So this is not a quality-for-speed trade. It is the observation that beyond a certain
    /// size the extra work has no representable effect, and both backends stop at the same
    /// point because the criterion lives here rather than in either of them.
    ///
    /// # What it is worth
    ///
    /// `tensor-allflags-withfunction.pdf` filled 22.0 G instructions' worth of tiny paths
    /// through `tiny-skia`, half of it inside the fill machinery and its per-path pipeline
    /// compilation. `personwithdog.pdf` took 17.3 seconds to rasterise a page whose display
    /// list has eighteen commands.
    #[must_use]
    pub fn is_subpixel(&self) -> bool {
        let [first, second, third] = self.points;
        let extent = |get: fn(&Point) -> f32| {
            let (x, y, z) = (get(&first), get(&second), get(&third));
            x.max(y).max(z) - x.min(y).min(z)
        };
        // One pixel in *both* axes: a long thin sliver spans several samples along its
        // length and its colour still varies across them, so a bounding box test has to
        // require smallness in each direction rather than in area.
        extent(|point| point.x) <= 1.0 && extent(|point| point.y) <= 1.0
    }

    /// Paints this triangle into a device-resolution buffer by §8.7.4.5.5's interpolation.
    ///
    /// The buffer's first pixel is device `(left, top)`. A pixel belongs to the triangle when
    /// its *centre* does — no antialiasing and no partial coverage — which is what makes two
    /// triangles sharing an edge tile exactly: every sample falls on one side or the other,
    /// and a sample exactly on the edge is claimed by both, the later one winning. Gaps are
    /// what a mesh cannot have; a one-pixel overlap between two nearly equal colours is
    /// invisible, which is the same property the old subdivision relied on.
    ///
    /// The interpolation itself is barycentric, which is the linear interpolation the clause
    /// asks for written in the coordinates that make it one expression per channel.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "indices and coordinates of device pixels, bounded by the caller's extent"
    )]
    #[expect(
        clippy::many_single_char_names,
        reason = "the clause's own notation for a triangle and its barycentric weights"
    )]
    fn paint(&self, data: &mut [u8], left: u32, top: u32, span: u32, rows: u32) {
        let [a, b, c] = self.points;
        // Twice the signed area. Zero means the three corners are collinear, so the triangle
        // covers nothing and has no interior to interpolate over.
        let area = (b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y);
        if area == 0.0 || !area.is_finite() {
            return;
        }

        let extent = |get: fn(&Point) -> f32| {
            let (p, q, r) = (get(&a), get(&b), get(&c));
            (p.min(q).min(r), p.max(q).max(r))
        };
        let (x0, x1) = extent(|point| point.x);
        let (y0, y1) = extent(|point| point.y);
        let first_x = ((x0 - 0.5).floor().max(0.0) as u32).saturating_sub(left);
        let first_y = ((y0 - 0.5).floor().max(0.0) as u32).saturating_sub(top);
        let last_x = (((x1 + 0.5).ceil().max(0.0) as u32).saturating_sub(left)).min(span);
        let last_y = (((y1 + 0.5).ceil().max(0.0) as u32).saturating_sub(top)).min(rows);

        for row in first_y..last_y {
            let y = top.saturating_add(row) as f32 + 0.5;
            for column in first_x..last_x {
                let x = left.saturating_add(column) as f32 + 0.5;
                // Barycentric coordinates, scaled by `area` so no division is needed to test
                // the sign — and divided once each only for a pixel that is inside.
                let w0 = (b.x - x) * (c.y - y) - (c.x - x) * (b.y - y);
                let w1 = (c.x - x) * (a.y - y) - (a.x - x) * (c.y - y);
                let w2 = (a.x - x) * (b.y - y) - (b.x - x) * (a.y - y);
                let inside = if area > 0.0 {
                    w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0
                } else {
                    w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0
                };
                if !inside {
                    continue;
                }
                let (u, v, w) = (w0 / area, w1 / area, w2 / area);
                let [ca, cb, cc] = self.colours;
                let mix = |get: fn(&Color) -> f32| {
                    (u * get(&ca) + v * get(&cb) + w * get(&cc)).clamp(0.0, 1.0)
                };
                let at = ((row as usize)
                    .saturating_mul(span as usize)
                    .saturating_add(column as usize))
                .saturating_mul(4);
                let Some(pixel) = data.get_mut(at..at.saturating_add(4)) else {
                    continue;
                };
                for (slot, value) in pixel.iter_mut().zip([
                    mix(|colour| colour.r),
                    mix(|colour| colour.g),
                    mix(|colour| colour.b),
                    mix(|colour| colour.a),
                ]) {
                    // Rounded rather than truncated, so a corner's own colour round-trips.
                    *slot = (value * 255.0 + 0.5) as u8;
                }
            }
        }
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

#[cfg(test)]
mod tests {
    use super::{Ramp, Triangle};
    use crate::{Color, Point};

    /// A break makes a step, and a ramp without one averages across it.
    ///
    /// The function here is green below 0.5 and blue above it, which is what a type 3
    /// stitching function with two equal `/Bounds` states. Sampled evenly, the midpoint of the
    /// ramp is a blend of the two; sampled across the break, every position is one colour or
    /// the other and the two stops that share position 0.5 are what carry the jump.
    #[test]
    fn a_break_is_a_step_and_not_a_gradient() {
        let step = |t: f32| {
            if t < 0.5 {
                Color {
                    r: 0.0,
                    g: 1.0,
                    b: 0.0,
                    a: 1.0,
                }
            } else {
                Color {
                    r: 0.0,
                    g: 0.0,
                    b: 1.0,
                    a: 1.0,
                }
            }
        };

        let even = Ramp::sample(step);
        let across = Ramp::sample_across(&[0.5], step);

        // Just to one side of the break, an evenly sampled ramp is already part-way to the
        // other colour; a ramp with the break in it is not.
        let near = 0.5 - 0.001;
        assert!(
            even.colour_at(near).b > 0.05,
            "an even ramp bleeds blue below the step: {:?}",
            even.colour_at(near)
        );
        assert!(
            across.colour_at(near).b < 0.001,
            "a ramp sampled across the break does not: {:?}",
            across.colour_at(near)
        );
        assert!(across.colour_at(0.5 + 0.001).g < 0.001);

        // Two stops share the break's position, which is what a gradient needs to draw a step.
        let at_break = across
            .stops
            .iter()
            .filter(|stop| (stop.at - 0.5).abs() < 1e-6)
            .count();
        assert_eq!(at_break, 2, "the step is two stops at one position");
    }

    /// A ramp with no breaks spans the whole interval, and a straight one is two stops.
    ///
    /// The length assertion used to be `RESOLUTION`, which was a statement about how the ramp
    /// is *built* rather than about what it says. `simplify` drops every stop a rasteriser
    /// would have computed anyway, so a linear function — every `/FunctionType 2` with `/N 1`,
    /// which is most shadings in most documents — comes out as its two endpoints. That is the
    /// whole of ADR 0068, seen from the smallest possible case.
    #[test]
    fn an_unbroken_ramp_spans_the_whole_interval() {
        let ramp = Ramp::sample(|t| Color {
            r: t,
            g: t,
            b: t,
            a: 1.0,
        });
        assert_eq!(ramp.stops.len(), 2, "a straight line needs its two ends");
        assert!(
            ramp.stops
                .first()
                .is_some_and(|stop| stop.at.abs() < f32::EPSILON)
        );
        assert!(
            ramp.stops
                .last()
                .is_some_and(|stop| (stop.at - 1.0).abs() < f32::EPSILON)
        );
        assert!((ramp.colour_at(0.5).r - 0.5).abs() < 0.01);
    }

    /// A triangle with the given device-space extents and three distinct corner colours, so
    /// that [`Triangle::is_flat`] never short-circuits the size question.
    fn spanning(width: f32, height: f32) -> Triangle {
        Triangle {
            points: [
                Point::new(0.0, 0.0),
                Point::new(width, 0.0),
                Point::new(0.0, height),
            ],
            colours: [
                Color::rgb(0.0, 0.0, 0.0),
                Color::rgb(1.0, 0.0, 0.0),
                Color::rgb(0.0, 1.0, 0.0),
            ],
        }
    }

    #[test]
    fn a_triangle_spanning_pixels_is_not_subpixel() {
        assert!(!spanning(10.0, 10.0).is_subpixel());
    }

    #[test]
    fn a_triangle_inside_one_pixel_is_subpixel() {
        assert!(spanning(0.5, 0.5).is_subpixel());
        // Exactly one pixel across counts: a triangle that spans from one sample's position
        // to the next still has only one sample inside it.
        assert!(spanning(1.0, 1.0).is_subpixel());
    }

    /// The rule is smallness in *each* axis, not small area, and this is the case that
    /// distinguishes them.
    ///
    /// A sliver twenty pixels long and a tenth of a pixel tall covers two square pixels of
    /// area but crosses twenty samples along its length, and its colour varies across them.
    /// Stopping subdivision there would draw a twenty-pixel streak in one flat colour.
    #[test]
    fn a_long_thin_sliver_is_not_subpixel() {
        assert!(!spanning(20.0, 0.1).is_subpixel());
        assert!(!spanning(0.1, 20.0).is_subpixel());
    }

    /// Subdivision stops on either condition, so a triangle can be worth subdividing for its
    /// size and not for its colour. Pinning both directions keeps the two independent.
    #[test]
    fn size_and_colour_are_separate_questions() {
        let large_and_flat = Triangle {
            points: spanning(10.0, 10.0).points,
            colours: [Color::rgb(0.5, 0.5, 0.5); 3],
        };
        assert!(large_and_flat.is_flat(1.0 / 512.0));
        assert!(!large_and_flat.is_subpixel());

        let tiny_and_varied = spanning(0.25, 0.25);
        assert!(!tiny_and_varied.is_flat(1.0 / 512.0));
        assert!(tiny_and_varied.is_subpixel());
    }
}
