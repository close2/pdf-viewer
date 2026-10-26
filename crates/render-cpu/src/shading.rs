//! Turning a resolved shading into a `tiny-skia` shader.
//!
//! Three of the four shading kinds map onto something `tiny-skia` implements natively, so
//! they are handed over rather than rasterised here: axial becomes a linear gradient,
//! radial a two-circle radial gradient — `tiny-skia` takes both radii, which is exactly
//! PDF's model — and a sampled shading becomes a pattern.
//!
//! Mesh shadings have no native equivalent and are drawn by the caller as triangles.
//!
//! # `Extend` has no direct equivalent, and needs one
//!
//! PDF's `/Extend` says whether the shading continues past each end. Where it does not,
//! *nothing is painted* there — which is not the same as painting the end colour, and is
//! the difference between a band across part of a shape and a wash over all of it.
//!
//! `tiny-skia`'s spread modes cannot express it: `Pad` paints the end colour forever,
//! `Repeat` and `Reflect` tile. So a non-extended end gets a fully transparent stop at the
//! same position as the ramp's own end stop — a hard stop, carrying the end's colour so no
//! fringe appears — and `Pad` then repeats *transparency* beyond it, which is precisely what
//! `/Extend false` asks for. `tiny-skia` keeps a hard stop at 0 or 1 intact under `Pad` by
//! evaluating its general gradient stage on the unclamped parameter.
//!
//! **Every other stop stays where the ramp put it.** ISO 32000-2 §8.7.4.5.3 gives the point
//! at `t` from its projection onto the axis alone, so a stop moved along the ramp is a colour
//! moved along the page, by a distance that grows with the axis and with the scale; a stripe
//! boundary that falls on a whole device row must be drawn on that row at every scale (ADR
//! 1387).

use pdf_render::{Color, Point, Ramp, Shading, ShadingKind, Transform};

use crate::surface::Placed;

/// Builds a shader for a shading, or `None` for kinds the caller must draw itself.
///
/// `page_to_path` maps page space into the space the path being drawn is stated in.
///
/// # Which space a paint is positioned in
///
/// `tiny-skia` *post-concatenates the drawing transform onto the paint's shader* —
/// `Pixmap::fill_path` and `Pixmap::stroke_path` both do it — so the transform handed to
/// a gradient or pattern is read in the path's own space, not the device's. Handing it a
/// device-space transform therefore applies the device transform twice.
///
/// That is not a hypothetical. It shipped: a gradient came out mirrored about the page's
/// horizontal centre at a scale of 1.0, where the y-flip happens to be its own inverse so
/// the second application cancels the geometry but not the flip, and displaced by a
/// scale-dependent amount at every other scale. A test in this crate pins a gradient's
/// value at two scales, because one scale cannot see it.
///
/// So the shading's transform, which maps its own space to *page* space, is carried the
/// rest of the way into the path's space here, and `tiny-skia` completes the journey to
/// the device. A pattern stays positioned relative to the page whatever the graphics
/// state held at fill time, which is what the specification asks for.
///
/// A sampled shading becomes a pattern, and `tiny-skia` patterns *borrow* their pixels, so
/// the caller lends somewhere to keep them. Everything else leaves `scratch` untouched.
///
/// `page_to_device` maps page space onto the device and `target` is that device's extent in
/// pixels. Both are read only by the sampled kind, whose colours are produced at the grid the
/// device turns out to want and over the block of it the target can sample
/// (`Shading::sampled_at`); the gradients position themselves in the path's space and never
/// ask how large they are drawn.
pub(crate) fn shader<'a>(
    shading: &Shading,
    page_to_path: Transform,
    page_to_device: Transform,
    target: (u32, u32),
    scratch: &'a mut Option<tiny_skia::Pixmap>,
) -> Option<tiny_skia::Shader<'a>> {
    let transform = crate::convert::transform(shading.transform.then(page_to_path));

    // Table 77's `/Background` is a colour this shading answers outside its own bounds, and no
    // `tiny-skia` shader can say so: a gradient's spread modes pad, repeat or reflect, and a
    // pattern's pad the grid's own edge. [`fill_with_background`] is the answer, and it is a
    // *fill*'s door — so a shading arriving here with one is a stroke, and refusing it loudly
    // is what keeps the shortfall a report rather than a picture missing its wash (trap 5).
    if shading.background.is_some() {
        return None;
    }

    match shading.kind.as_ref() {
        ShadingKind::Axial {
            start,
            end,
            ramp,
            extend,
        } => tiny_skia::LinearGradient::new(
            tiny_skia::Point::from_xy(start.x, start.y),
            tiny_skia::Point::from_xy(end.x, end.y),
            stops(ramp, *extend),
            tiny_skia::SpreadMode::Pad,
            transform,
        ),
        ShadingKind::Radial {
            start,
            start_radius,
            end,
            end_radius,
            ramp,
            extend,
        } => tiny_skia::RadialGradient::new(
            tiny_skia::Point::from_xy(start.x, start.y),
            *start_radius,
            tiny_skia::Point::from_xy(end.x, end.y),
            *end_radius,
            stops(ramp, *extend),
            tiny_skia::SpreadMode::Pad,
            transform,
        ),
        ShadingKind::Sampled { .. } => {
            sampled_shader(shading, transform, page_to_device, target, scratch)
        }
        // Meshes carry a colour — or §8.7.4.5.5's parametric value — per triangle corner,
        // which no shader can express; `fill_mesh` rasterises them instead. A kind added
        // later lands here too, and returning None makes the caller report it rather than
        // draw nothing.
        _ => None,
    }
}

/// Builds the gradient stops for a ramp, honouring `/Extend`.
///
/// The ramp's stops are handed over at their own positions, and a non-extended end adds a
/// transparent stop at the same position as the end it closes. See the note at the top of
/// this module for why nothing else may move.
fn stops(ramp: &Ramp, extend: (bool, bool)) -> Vec<tiny_skia::GradientStop> {
    let mut stops: Vec<tiny_skia::GradientStop> =
        Vec::with_capacity(ramp.stops.len().saturating_add(2));

    if !extend.0 {
        stops.push(transparent_stop(0.0, ramp.colour_at(0.0)));
    }
    for stop in ramp.stops.iter() {
        stops.push(tiny_skia::GradientStop::new(
            stop.at.clamp(0.0, 1.0),
            crate::convert::color(stop.colour),
        ));
    }
    if !extend.1 {
        stops.push(transparent_stop(1.0, ramp.colour_at(1.0)));
    }
    stops
}

/// A stop that paints nothing, keeping the neighbouring colour so no fringe appears.
fn transparent_stop(position: f32, colour: Color) -> tiny_skia::GradientStop {
    tiny_skia::GradientStop::new(position, crate::convert::color(Color { a: 0.0, ..colour }))
}

/// A colour component in `0.0..=1.0` as the eight-bit level nearest its exact value, half away
/// from zero — ADR 2311's rule, which the device's `nearest_level` writes (ADR 2325) — so that a
/// cell of the oracle's grid and the device's function lane are asked for the same byte (ADR 2340).
///
/// The `f32` product with 255 is not that value: it rounds a component a hair below a half level
/// onto the half, and the rounding after it then goes up — `0.7` is 178.499 997 levels and its
/// `f32` product is 178.5. Twice the exact level is taken instead, in `f64`, where a 24-bit
/// significand times 510 needs 33 bits of 53 and is therefore exact; the cast truncates it, which is
/// the floor of a value that is not negative, and the level is half of that floor rounded up —
/// `(⌊510 c⌋ + 1) / 2`, the device's own construction (ADR 2325 §2). The one exact half in the
/// range is `0.5`, which is 128; a NaN clamps to NaN and the cast makes it level 0.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the component is clamped to 0..=1, so twice its level is at most 510 and the level \
              at most 255; the first cast is the floor of a value that is not negative"
)]
pub(crate) fn nearest_level(component: f32) -> u8 {
    let doubled = (f64::from(component.clamp(0.0, 1.0)) * 510.0) as u32;
    doubled.div_ceil(2) as u8
}

/// Builds a pattern of an axial or radial ramp's own levels, one cell per device pixel, for a fill
/// whose shape is `path` (ISO 32000-2 §8.7.4.5.3, §8.7.4.5.4).
///
/// The clause gives a point the colour function's answer at the `t` of that point, and the oracle
/// computes it here rather than handing the ramp to `tiny-skia` as a gradient: each pixel centre of
/// the page's grid is taken back into the shading's space, its `t` is the projection onto the axis
/// or [`pdf_render::blend_parameter`]'s greatest admissible circle, and the cell holds the level
/// nearest the ramp's colour there — [`nearest_level`], the byte the device's shading lane writes
/// for an opaque paint (ADRs 2310, 2325). A gradient interpolates in the library and is stored by
/// its high-precision pipeline as the `f32` product with 255 rounded to nearest-even, which breaks
/// a component a hair below a half level upwards half the time, and nothing in this tree sees the
/// value before that store (ADR 2356).
///
/// The cells are on the page's own grid, for [`fill_radial`]'s reason: a window's or a strip's
/// pixel is evaluated at the centre the page's is (ADR 2148). They cover the shape's device bounds
/// on this surface, and the pattern is positioned in the path's space — the surface's matrix
/// `shape.at` inverted, since every converter `draw_fill` reaches moves a paint by it (trap 2) — so
/// each device pixel reads its own cell, and the shape's edge, its marks and its bands stay the
/// converters' they were.
///
/// `None` where the ramp is not one this draws — a cone, which [`fill_radial`] draws first; an
/// axis of no length, a geometry `tiny-skia` refuses as a gradient, or a ramp below full alpha,
/// each of which keeps the gradient's answer; a shading with Table 77's `/Background`, which is
/// [`fill_with_background`]'s — or where the shape covers no pixel of this surface.
pub(crate) fn ramp_shader<'a>(
    shading: &Shading,
    path: &tiny_skia::Path,
    (shape, onto_page): (Placed, Placed),
    target: (u32, u32),
    scratch: &'a mut Option<tiny_skia::Pixmap>,
) -> Option<tiny_skia::Shader<'a>> {
    if shading.background.is_some() {
        return None;
    }
    let sweep = Sweep::of(shading)?;
    // A translucent ramp — a constant alpha below 1 folded into its stops — is the gradient's: the
    // library multiplies the colour by its alpha in `f32` and the blend rounds the pixel once, which
    // is how the device hands a paint below full alpha over (ADR 2350), where a cell would round the
    // product first and the blend a second time (trap 257).
    if sweep.ramp().stops.iter().any(|stop| stop.colour.a < 1.0) {
        return None;
    }
    let mut cells = RampCells::of(sweep.ramp());
    let grid = OnSurface::page(onto_page);
    let (left, top, right, bottom) = grid.bounds(path, shape, target)?;
    let to_shading = grid.matrix.invert()?;
    let onto_path = shape.at.invert()?;
    let mut pixmap = tiny_skia::Pixmap::new(right.checked_sub(left)?, bottom.checked_sub(top)?)?;
    let width = pixmap.width() as usize;
    for (row, cells_of_row) in pixmap.pixels_mut().chunks_exact_mut(width).enumerate() {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a row of a raster bounded by the target, whose extent `MAX_EXTENT` keeps \
                      under 2^24, so each coordinate is exact in f32"
        )]
        let y = (top as usize).saturating_add(row) as f32 + 0.5;
        for (column, destination) in cells_of_row.iter_mut().enumerate() {
            #[expect(
                clippy::cast_precision_loss,
                reason = "a column of the same raster, exact in f32 for the same reason"
            )]
            let x = (left as usize).saturating_add(column) as f32 + 0.5;
            if let Some(t) = sweep.at(to_shading.apply(Point { x, y })) {
                *destination = cells.at(t);
            }
        }
    }
    let (x, y) = grid.onto_surface(i32::try_from(left).ok()?, i32::try_from(top).ok()?);
    #[expect(
        clippy::cast_precision_loss,
        reason = "a device pixel index is far below f32's exact integer limit"
    )]
    let onto_surface = Transform::translate(x as f32, y as f32);
    Some(tiny_skia::Pattern::new(
        scratch.insert(pixmap).as_ref(),
        tiny_skia::SpreadMode::Pad,
        // One cell per device pixel, placed at whole pixels: no sample is interpolated.
        tiny_skia::FilterQuality::Nearest,
        1.0,
        crate::convert::transform(onto_surface.then(onto_path)),
    ))
}

/// What [`ramp_shader`] reads a pixel's `t` from: an axial shading's axis or a radial shading's two
/// circles, each with its ramp and `/Extend`.
enum Sweep<'a> {
    /// §8.7.4.5.3: the projection onto the axis from `start` along `axis`, `length` its square.
    Axial {
        start: Point,
        axis: (f32, f32),
        length: f32,
        extend: (bool, bool),
        ramp: &'a Ramp,
    },
    /// §8.7.4.5.4, for two circles one of which contains the other.
    Radial(pdf_render::Radial<'a>),
}

impl<'a> Sweep<'a> {
    /// The sweep a shading states, or `None` for every shading [`ramp_shader`] leaves alone.
    ///
    /// A geometry `tiny-skia` would refuse as a gradient is left to the gradient's own refusal, so
    /// that it is reported as it was rather than drawn by a second reading of it.
    fn of(shading: &'a Shading) -> Option<Self> {
        match shading.kind.as_ref() {
            ShadingKind::Axial {
                start,
                end,
                ramp,
                extend,
            } => {
                let axis = (end.x - start.x, end.y - start.y);
                let length = axis.0.mul_add(axis.0, axis.1 * axis.1);
                (length > 0.0 && length.is_finite()).then_some(Self::Axial {
                    start: *start,
                    axis,
                    length,
                    extend: *extend,
                    ramp,
                })
            }
            ShadingKind::Radial {
                start,
                start_radius,
                end,
                end_radius,
                ramp,
                extend,
            } => {
                tiny_skia::RadialGradient::new(
                    tiny_skia::Point::from_xy(start.x, start.y),
                    *start_radius,
                    tiny_skia::Point::from_xy(end.x, end.y),
                    *end_radius,
                    stops(ramp, *extend),
                    tiny_skia::SpreadMode::Pad,
                    tiny_skia::Transform::identity(),
                )?;
                (!is_a_cone(*start, *start_radius, *end, *end_radius)).then_some(Self::Radial(
                    pdf_render::Radial {
                        start: *start,
                        start_radius: *start_radius,
                        end: *end,
                        end_radius: *end_radius,
                        ramp,
                        extend: *extend,
                    },
                ))
            }
            _ => None,
        }
    }

    /// The ramp the sweep reads.
    fn ramp(&self) -> &'a Ramp {
        match self {
            Self::Axial { ramp, .. } => ramp,
            Self::Radial(radial) => radial.ramp,
        }
    }

    /// The `t` of a point in the shading's own space, clamped into `0.0..=1.0`, or `None` where
    /// `/Extend` leaves the point unpainted, which paints nothing rather than the end colour —
    /// ISO 32000-2 §8.7.4.5.3, for a point before the axis's start:
    ///
    /// > For x ′ < 0, if the first element of the Extend array is true , then t = t 0 ; otherwise,
    /// > t is undefined and the point shall be left unpainted.
    ///
    /// and §8.7.4.5.4's blend circles, through [`pdf_render::blend_parameter`], for a radial one.
    fn at(&self, point: Point) -> Option<f32> {
        match self {
            Self::Axial {
                start,
                axis,
                length,
                extend,
                ..
            } => {
                let along = (point.x - start.x) * axis.0 + (point.y - start.y) * axis.1;
                let t = along / length;
                if !t.is_finite() || (t < 0.0 && !extend.0) || (t > 1.0 && !extend.1) {
                    return None;
                }
                Some(t.clamp(0.0, 1.0))
            }
            Self::Radial(radial) => pdf_render::blend_parameter(
                point,
                radial.start,
                radial.start_radius,
                radial.end,
                radial.end_radius,
                radial.extend,
            )
            .map(|s| s.clamp(0.0, 1.0)),
        }
    }
}

/// A ramp's cells by `t`: [`Ramp::colour_at`]'s colour, as [`level_of_colour`]'s cell, found
/// without walking the stops.
///
/// A cell per device pixel asks a ramp for a colour sixteen million times on a 4x page, and a ramp
/// sampled from a type 0 function holds hundreds of stops (ADR 2356 prices it on `issue14165.pdf`).
/// So the stop is found by halving, after the interval the last `t` fell in is tried, and a `t`
/// equal to the last one — every pixel of a row across a vertical axis — takes the last cell. Each
/// finds the stop the walk finds, the first after the first whose position is not below `t`,
/// wherever the positions ascend, which is what [`Ramp::stops`] states; a ramp whose positions do
/// not ascend is walked, so the answer is the walk's either way, and
/// `a_ramps_cells_are_the_walks_colours` holds the two equal bit for bit.
struct RampCells<'a> {
    /// The ramp read.
    ramp: &'a Ramp,
    /// Whether its positions ascend, so that halving finds the walk's stop.
    ascending: bool,
    /// The stop after the first that the last `t` interpolated towards, counted from the second.
    hint: usize,
    /// The last `t`'s bits and its cell.
    last: Option<(u32, tiny_skia::PremultipliedColorU8)>,
}

impl<'a> RampCells<'a> {
    /// The cells of `ramp`.
    fn of(ramp: &'a Ramp) -> Self {
        Self {
            ramp,
            ascending: ramp.stops.windows(2).all(|pair| pair[0].at <= pair[1].at),
            hint: 0,
            last: None,
        }
    }

    /// The cell at `t`.
    fn at(&mut self, t: f32) -> tiny_skia::PremultipliedColorU8 {
        if let Some((bits, cell)) = self.last
            && bits == t.to_bits()
        {
            return cell;
        }
        let cell = level_of_colour(self.colour(t));
        self.last = Some((t.to_bits(), cell));
        cell
    }

    /// The colour at `t`, as [`Ramp::colour_at`] answers it.
    fn colour(&mut self, t: f32) -> Color {
        let stops = &self.ramp.stops;
        let t = t.clamp(0.0, 1.0);
        let (Some(first), true) = (stops.first(), self.ascending && !t.is_nan()) else {
            return self.ramp.colour_at(t);
        };
        if t <= first.at {
            return first.colour;
        }
        let rest = stops.get(1..).unwrap_or_default();
        let below = |index: usize| rest.get(index).is_some_and(|stop| stop.at < t);
        // The number of stops after the first whose position is below `t`: the hint where the
        // stop before it is below and its own is not, which over ascending positions is that
        // number and no other.
        let index = if self.hint.checked_sub(1).is_none_or(below) && !below(self.hint) {
            self.hint
        } else {
            rest.partition_point(|stop| stop.at < t)
        };
        self.hint = index;
        // `index` counts from the second stop, so the stop it names is `index + 1` and the one
        // before it `index`; past the last, the last stop's colour is the walk's answer too.
        let (Some(previous), Some(stop)) = (stops.get(index), rest.get(index)) else {
            return stops.last().map_or(first.colour, |last| last.colour);
        };
        let span = stop.at - previous.at;
        let fraction = if span > 0.0 {
            (t - previous.at) / span
        } else {
            1.0
        };
        let (a, b) = (previous.colour, stop.colour);
        Color {
            r: a.r + (b.r - a.r) * fraction,
            g: a.g + (b.g - a.g) * fraction,
            b: a.b + (b.b - a.b) * fraction,
            a: a.a + (b.a - a.a) * fraction,
        }
    }
}

/// A straight-alpha colour as the premultiplied cell [`nearest_level`] makes of it, channel by
/// channel, as [`sampled_shader`] writes a cell of its grid.
fn level_of_colour(colour: Color) -> tiny_skia::PremultipliedColorU8 {
    let alpha = colour.a.clamp(0.0, 1.0);
    let channel = |value: f32| nearest_level(value.clamp(0.0, 1.0) * alpha);
    // A premultiplied channel is never above its alpha, and the level of a smaller fraction is
    // never the larger, so `from_rgba`'s refusal of a channel past its alpha cannot fire.
    tiny_skia::PremultipliedColorU8::from_rgba(
        channel(colour.r),
        channel(colour.g),
        channel(colour.b),
        nearest_level(alpha),
    )
    .unwrap_or(tiny_skia::PremultipliedColorU8::TRANSPARENT)
}

/// Builds a pattern shader over a sampled shading's grid.
///
/// The grid is the device's: `Shading::sampled_at` derives it from how many device pixels
/// the domain covers under `page_to_device`, so zooming re-resolves the function rather than
/// magnifying a raster fixed when the display list was built. `target` is the extent of the
/// pixmap being drawn into, and what it buys is the block: past the magnification at which a
/// page fits, most of the domain is off the target and every cell of it used to be evaluated
/// anyway (ADR 0408).
///
/// **`SpreadMode::Pad` still pads the domain's own edge colours.** Where the block stops
/// short of the domain it stops at least one cell outside the target, so the padding a
/// clipped block repeats is padding no pixel of this target reads; where the target reaches
/// the domain's edge the block does too, and the colour repeated is the one that was
/// repeated before.
fn sampled_shader<'a>(
    shading: &Shading,
    transform: tiny_skia::Transform,
    page_to_device: Transform,
    target: (u32, u32),
    scratch: &'a mut Option<tiny_skia::Pixmap>,
) -> Option<tiny_skia::Shader<'a>> {
    let grid = shading.sampled_at(page_to_device, target)?;
    let [x0, x1, y0, y1] = grid.covers;
    if x1 - x0 == 0.0 || y1 - y0 == 0.0 {
        return None;
    }

    let mut pixmap = tiny_skia::Pixmap::new(grid.width, grid.height)?;
    for (destination, colour) in pixmap.pixels_mut().iter_mut().zip(grid.pixels.iter()) {
        // Each cell holds the level nearest its own value, the byte the device's function lane
        // writes for the same answer (ADR 2340); `to_color_u8` rounds the `f32` product with 255,
        // which breaks a component a hair below a half level upwards.
        let premultiplied = crate::convert::color(*colour).premultiply();
        let level = nearest_level;
        // A premultiplied channel is never above its alpha, and the level of a smaller fraction
        // is never the larger, so `from_rgba`'s refusal of a channel past its alpha cannot fire.
        *destination = tiny_skia::PremultipliedColorU8::from_rgba(
            level(premultiplied.red()),
            level(premultiplied.green()),
            level(premultiplied.blue()),
            level(premultiplied.alpha()),
        )
        .unwrap_or(tiny_skia::PremultipliedColorU8::TRANSPARENT);
    }

    // The cells cover `grid.covers`, so the pattern is scaled from pixel coordinates onto
    // that rectangle — which is the whole domain wherever nothing was clipped away — and then
    // carried into the shading's own space.
    #[expect(
        clippy::cast_precision_loss,
        reason = "grid dimensions are bounded well inside f32's exact integer range"
    )]
    let per_cell = Transform::scale(1.0 / grid.width as f32, 1.0 / grid.height as f32);
    let to_domain = crate::convert::transform(per_cell.then(grid.onto_shading()));

    Some(tiny_skia::Pattern::new(
        scratch.insert(pixmap).as_ref(),
        tiny_skia::SpreadMode::Pad,
        tiny_skia::FilterQuality::Bilinear,
        1.0,
        transform.pre_concat(to_domain),
    ))
}

/// Draws a mesh shading's triangles, clipped to a path (ISO 32000-2 §8.7.4.5.5).
///
/// The mesh is rasterised once by [`pdf_render::MeshRaster`] — the clause's own linear
/// interpolation, evaluated at each device pixel's centre, over the triangles the file states
/// or over the tessellation the page's matrix says §8.7.4.5.7's patches need (ADR 1217) — and the
/// result is drawn as an image confined to the shape. So the *colour* is `pdf-render`'s, identically on both
/// backends, and the *edge* is `tiny-skia`'s, antialiased as every other fill's is.
///
/// A triangle is not subdivided into flat pieces grown to close their seams; `MeshRaster` says
/// why (ADR 0051).
#[expect(
    clippy::too_many_arguments,
    reason = "these are the parameters `fill_path` itself takes, threaded through one \
              level; bundling them into a struct would only move the list"
)]
pub(crate) fn fill_mesh(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    path: &tiny_skia::Path,
    triangles: &[pdf_render::Triangle],
    patches: Option<&pdf_render::PatchMesh>,
    ramp: Option<&Ramp>,
    mesh_to_device: Placed,
    fill_rule: tiny_skia::FillRule,
    shape: Placed,
    clip: crate::scan::Clip<'_>,
    blend: tiny_skia::BlendMode,
    anti_alias: bool,
) {
    // Evaluated at the page's own pixel centres over the surface's part of the page, the triangles
    // mapped at the page's magnitude, so a window's or a strip's raster is the page's there; the
    // shape's edge is the page's as well (ADRs 2148, 2167).
    let grid = OnSurface::page(mesh_to_device);
    let (left, top) = grid.corner;
    let (Some(right), Some(bottom)) = (
        left.checked_add(pixmap.width()),
        top.checked_add(pixmap.height()),
    ) else {
        return;
    };
    let Some(raster) = pdf_render::MeshRaster::build_within(
        triangles,
        patches,
        ramp,
        grid.matrix,
        (left, top, right, bottom),
    ) else {
        return;
    };
    fill_with_raster(
        pixmap,
        (path, shape),
        &raster.image,
        grid.onto_surface(raster.left, raster.top),
        fill_rule,
        clip,
        blend,
        anti_alias,
    );
}

/// Draws a shading washed with ISO 32000-2 §8.7.4.3 Table 77's `/Background`, clipped to a path.
///
/// The third member of [`fill_mesh`]'s and [`fill_radial`]'s family and the general one:
/// [`pdf_render::ShadingRaster`] evaluates the shading at each device pixel's centre and answers
/// the background where the shading's own geometry answers nothing, so the wash and the gradient
/// are one raster, drawn through the shape's antialiased edge as a single painting operation.
/// That is §11.6.7's construction — the pattern's implicit group filled with the background
/// before the `sh` — rather than Table 77's NOTE 1, which states the two-operation equivalence
/// for the *opaque* imaging model only.
///
/// The region is the shape's own device bounds rather than the target's, for [`fill_radial`]'s
/// reason: the wash covers "the area to be painted", which is the path, and a page-sized raster
/// per shading is a cost the shape already rules out.
#[expect(
    clippy::too_many_arguments,
    reason = "these are the parameters `fill_path` itself takes, threaded through one \
              level; bundling them into a struct would only move the list"
)]
pub(crate) fn fill_with_background(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    path: &tiny_skia::Path,
    shading: &Shading,
    page_to_device: Placed,
    fill_rule: tiny_skia::FillRule,
    shape: Placed,
    clip: crate::scan::Clip<'_>,
    blend: tiny_skia::BlendMode,
    anti_alias: bool,
) {
    let target = (pixmap.width(), pixmap.height());
    // A sampled shading's colours are a grid of cells `Shading::sampled_at` crops to the target
    // it is handed, and on the page's grid that target would be every pixel above and left of the
    // surface as well; it keeps the surface's grid. Every other kind is evaluated at the page's
    // own pixel centres over the surface's part of the page (ADR 2148).
    let grid = if matches!(shading.kind.as_ref(), ShadingKind::Sampled { .. }) {
        OnSurface::of(page_to_device.at)
    } else {
        OnSurface::page(page_to_device)
    };
    let Some(within) = grid.bounds(path, shape, target) else {
        return;
    };
    let Some(raster) = pdf_render::ShadingRaster::build(shading, grid.matrix, within, target)
    else {
        return;
    };
    fill_with_raster(
        pixmap,
        (path, shape),
        &raster.image,
        grid.onto_surface(raster.left, raster.top),
        fill_rule,
        clip,
        blend,
        anti_alias,
    );
}

/// The grid a shading's raster is evaluated on, and where the surface lies on it.
///
/// The page's, with the surface's corner, wherever the raster can be bounded to the surface's part
/// of the page: then a window's or a strip's pixel takes the value the page's takes, because each
/// is evaluated at the same centre under the same matrix (ADR 2148). The surface's own, at a
/// corner of nothing, where it cannot.
#[derive(Debug, Clone, Copy)]
struct OnSurface {
    /// The shading's coordinates onto the grid.
    matrix: Transform,
    /// The surface's first column and row on that grid.
    corner: (u32, u32),
}

impl OnSurface {
    /// The surface's own grid, under `matrix`.
    fn of(matrix: Transform) -> Self {
        Self {
            matrix,
            corner: (0, 0),
        }
    }

    /// The page's grid, as `placed` states it.
    fn page(placed: Placed) -> Self {
        Self {
            matrix: placed.page,
            corner: placed.corner,
        }
    }

    /// The pixels of this grid the shape covers that lie on a surface `target` big, as
    /// `(left, top, right, bottom)`.
    fn bounds(
        self,
        path: &tiny_skia::Path,
        shape: Placed,
        target: (u32, u32),
    ) -> Option<(u32, u32, u32, u32)> {
        if self.corner == (0, 0) {
            return device_bounds(path, crate::convert::transform(shape.at), target);
        }
        let (left, top) = self.corner;
        let (right, bottom) = (left.checked_add(target.0)?, top.checked_add(target.1)?);
        let (x0, y0, x1, y1) =
            device_bounds(path, crate::convert::transform(shape.page), (right, bottom))?;
        Some((x0.max(left), y0.max(top), x1, y1))
    }

    /// A raster's corner on this grid, on the surface.
    fn onto_surface(self, left: i32, top: i32) -> (i32, i32) {
        let corner = (
            i32::try_from(self.corner.0).unwrap_or(i32::MAX),
            i32::try_from(self.corner.1).unwrap_or(i32::MAX),
        );
        (left.saturating_sub(corner.0), top.saturating_sub(corner.1))
    }
}

/// The device pixels a shape covers, clamped to the target.
///
/// Half a pixel of margin on each side, because a pixel is sampled at its centre and a shape
/// ending at x = 10.0 still covers the sample at 9.5 — `MeshRaster::build`'s own margin, and the
/// same bound the sibling backends compute.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "each cast is between a device pixel index and its coordinate, both bounded \
              by the target's extent"
)]
fn device_bounds(
    shape: &tiny_skia::Path,
    shape_transform: tiny_skia::Transform,
    (width, height): (u32, u32),
) -> Option<(u32, u32, u32, u32)> {
    let bounds = shape.clone().transform(shape_transform)?.bounds();
    Some((
        (bounds.left() - 0.5).floor().max(0.0) as u32,
        (bounds.top() - 0.5).floor().max(0.0) as u32,
        (bounds.right() + 0.5).ceil().max(0.0).min(width as f32) as u32,
        (bounds.bottom() + 0.5).ceil().max(0.0).min(height as f32) as u32,
    ))
}

/// Whether a radial shading's geometry is one §8.7.4.5.4 decides and a gradient cannot.
///
/// The quadratic whose roots are the blend circles through a point has leading coefficient
/// `|c1 − c0|² − (r1 − r0)²`, and its **sign is the whole question**:
///
/// - **Negative** — the centres are closer together than the radii differ, so one circle
///   contains the other: §8.7.4.5.4's NOTE 2 sphere. Exactly one blend circle passes through
///   any point. `|p − c(s)| − r(s)` is convex in `s` and runs from `+∞` to `−∞`, so it is
///   monotone and has one root; the other root of the squared equation is where
///   `|p − c(s)| = −r(s)`, which is not a circle. There is nothing to choose between, so a
///   two-point conical gradient cannot pick the wrong one.
/// - **Zero** — internally tangent, one root, the same argument.
/// - **Positive** — NOTE 3's cone. Now the convex function runs to `+∞` in both directions
///   and can cross zero twice, so a point can lie on two blend circles and the clause's
///   "greatest value of s" has work to do — including the case where the *greater* root is
///   one `/Extend` refuses and the answer is the lesser. That is what no gradient expresses.
///
/// So this is not a tuned threshold. It is the exact condition under which the clause's
/// tie-breaking rule can change a pixel, which is why the exact evaluation is paid for
/// there and nowhere else.
#[must_use]
pub(crate) fn is_a_cone(start: Point, start_radius: f32, end: Point, end_radius: f32) -> bool {
    let (dx, dy, dr) = (end.x - start.x, end.y - start.y, end_radius - start_radius);
    dr.mul_add(-dr, dx.mul_add(dx, dy * dy)) > 0.0
}

/// Draws a radial shading exactly, clipped to a path (ISO 32000-2 §8.7.4.5.4).
///
/// The counterpart of [`fill_mesh`] one shading type over, and for the same reason: the
/// clause states an algorithm the rasteriser's native primitive does not implement, so
/// [`pdf_render::RadialRaster`] evaluates it at each device pixel's centre and the result is
/// drawn as an image confined to the shape. [`pdf_render::blend_parameter`] is the algorithm
/// and [`is_a_cone`] is when it is needed.
///
/// The raster covers the shape's own device bounds rather than the target's, because an
/// extended radial covers everything: `radial_gradients.pdf` puts twenty-four of them on one
/// page, and a page-sized raster apiece is a cost the shape already rules out.
#[expect(
    clippy::too_many_arguments,
    reason = "these are the parameters `fill_path` itself takes, threaded through one \
              level; bundling them into a struct would only move the list"
)]
pub(crate) fn fill_radial(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    path: &tiny_skia::Path,
    radial: pdf_render::Radial<'_>,
    to_device: Placed,
    fill_rule: tiny_skia::FillRule,
    shape: Placed,
    clip: crate::scan::Clip<'_>,
    blend: tiny_skia::BlendMode,
    anti_alias: bool,
) -> bool {
    // Evaluated at the page's own pixel centres over the surface's part of the page (ADR 2148).
    let grid = OnSurface::page(to_device);
    let Some(within) = grid.bounds(path, shape, (pixmap.width(), pixmap.height())) else {
        return false;
    };
    let Some(raster) = pdf_render::RadialRaster::build(radial, grid.matrix, within) else {
        return false;
    };
    fill_with_raster(
        pixmap,
        (path, shape),
        &raster.image,
        grid.onto_surface(raster.left, raster.top),
        fill_rule,
        clip,
        blend,
        anti_alias,
    );
    true
}

/// Draws a device-resolution raster through a shape, at whole device pixels.
///
/// Shared by [`fill_mesh`] and [`fill_radial`] because the two differ only in what fills the
/// raster: both evaluate a clause in `pdf-render` so that the backends cannot disagree about
/// the colour, and both then need the *edge* to be the shape's, antialiased as every other
/// fill's is.
#[expect(
    clippy::too_many_arguments,
    reason = "these are the parameters `fill_path` itself takes, threaded through one \
              level; bundling them into a struct would only move the list"
)]
fn fill_with_raster(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    (path, shape): (&tiny_skia::Path, Placed),
    image: &pdf_render::Image,
    (left, top): (i32, i32),
    fill_rule: tiny_skia::FillRule,
    clip: crate::scan::Clip<'_>,
    blend: tiny_skia::BlendMode,
    anti_alias: bool,
) {
    let Some(mut samples) = tiny_skia::Pixmap::new(image.width, image.height) else {
        return;
    };
    // `tiny-skia` pixmaps are premultiplied and `Image` is straight alpha, the same boundary
    // conversion `CpuRasterizer::draw_image` makes.
    for (target, source) in samples
        .pixels_mut()
        .iter_mut()
        .zip(image.data.chunks_exact(4))
    {
        let alpha = source[3];
        *target = tiny_skia::PremultipliedColorU8::from_rgba(
            crate::compose::premultiply(source[0], alpha),
            crate::compose::premultiply(source[1], alpha),
            crate::compose::premultiply(source[2], alpha),
            alpha,
        )
        .unwrap_or(tiny_skia::PremultipliedColorU8::TRANSPARENT);
    }

    let paint = tiny_skia::Paint {
        shader: tiny_skia::Pattern::new(
            samples.as_ref(),
            tiny_skia::SpreadMode::Pad,
            // The raster is already at device resolution and is placed at whole pixels, so
            // no sample is ever interpolated and the filter cannot change a pixel.
            tiny_skia::FilterQuality::Nearest,
            1.0,
            tiny_skia::Transform::from_translate(
                // Both are device pixel indices, bounded by `MAX_EXTENT` at 2^24, so the
                // conversion is exact.
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "a device pixel index is far below f32's exact integer limit"
                )]
                {
                    left as f32
                },
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "a device pixel index is far below f32's exact integer limit"
                )]
                {
                    top as f32
                },
            ),
        ),
        blend_mode: blend,
        anti_alias,
        force_hq_pipeline: crate::HIGH_PRECISION_PIPELINE,
        ..tiny_skia::Paint::default()
    };
    // The shape carries its own transform; the pattern's is in the surface's *device* space,
    // which is what a paint is read in — see `shader` above, and trap 2 — so it is moved by no
    // matrix, and the shape is measured on the page's grid and written at the surface's corner
    // (ADR 2148).
    crate::scan::fill_positioned(pixmap, path, &paint, fill_rule, shape, clip);
}

#[cfg(test)]
mod tests {
    use pdf_render::{Color, Ramp, Stop};

    use super::{RampCells, nearest_level};

    /// ADR 2356: [`RampCells`] answers what [`Ramp::colour_at`]'s walk answers, bit for bit — over
    /// ramps with a hard step, a run of three stops at one position and ends short of 0 and 1, at
    /// every stop, an ulp either side of it and between, asked in an order that leaves the hint
    /// wrong as often as right — and walks a ramp whose positions do not ascend.
    #[test]
    fn a_ramps_cells_are_the_walks_colours() {
        let colour = |v: f32| Color::rgba(v, 1.0 - v, v * 0.5, 1.0);
        let ramp = |positions: &[f32]| Ramp {
            stops: positions
                .iter()
                .enumerate()
                .map(|(index, &at)| Stop {
                    at,
                    colour: colour(f32::from(u8::try_from(index).unwrap_or(0)) / 7.0),
                })
                .collect::<Vec<_>>()
                .into(),
        };
        let ramps = [
            ramp(&[0.0, 1.0]),
            ramp(&[0.0, 0.25, 0.25, 0.6, 1.0]),
            ramp(&[0.1, 0.4, 0.4, 0.4, 0.9]),
            ramp(&[0.0, 0.5, 0.3, 1.0]),
            ramp(&[0.5]),
        ];
        for ramp in &ramps {
            let mut asked: Vec<f32> = Vec::new();
            for stop in ramp.stops.iter() {
                asked.extend([stop.at.next_down(), stop.at, stop.at.next_up()]);
            }
            asked.extend((0..=64_u8).map(|step| f32::from(step) / 64.0));
            asked.extend([-0.5, 1.5, f32::NAN]);
            let mut cells = RampCells::of(ramp);
            for &t in asked.iter().chain(asked.iter().rev()) {
                let (walked, halved) = (ramp.colour_at(t), cells.colour(t));
                assert_eq!(
                    [walked.r, walked.g, walked.b, walked.a].map(f32::to_bits),
                    [halved.r, halved.g, halved.b, halved.a].map(f32::to_bits),
                    "t = {t} over {:?}",
                    ramp.stops.iter().map(|stop| stop.at).collect::<Vec<_>>()
                );
            }
        }
    }

    /// ADR 2340: every level `k` below 255 at the `f32` nearest `(k + ½) / 255` and at the `f32`
    /// either side of it — the 765 greys ADR 2325 proved the device's `nearest_level` with — is
    /// the level nearest its exact value, half away from zero. The exact value is `f64`'s
    /// product, which holds a 24-bit significand times 255 without rounding; the count beside it
    /// is the calibration (trap 13), that the fixture reaches greys `to_color_u8`'s `f32` form breaks.
    #[test]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the construction this test calibrates against: a component of 0..=1 times 255, \
                  rounded, and in range by construction"
    )]
    fn a_component_is_the_level_nearest_its_exact_value() {
        let mut broken_by_the_f32_product = 0_u32;
        for k in 0..255_u16 {
            let nearest = ((f32::from(k) + 0.5) / 255.0).to_bits();
            for bits in [
                nearest.saturating_sub(1),
                nearest,
                nearest.saturating_add(1),
            ] {
                let component = f32::from_bits(bits);
                let exact = f64::from(component) * 255.0;
                let level = nearest_level(component);
                assert!(
                    f64::from(level) - 0.5 <= exact && exact < f64::from(level) + 0.5,
                    "{component} is {exact} levels and was written as {level}"
                );
                if (component * 255.0 + 0.5) as u8 != level {
                    broken_by_the_f32_product = broken_by_the_f32_product.saturating_add(1);
                }
            }
        }
        assert!(
            broken_by_the_f32_product > 0,
            "the fixture reaches the greys `to_color_u8`'s f32 product rounds onto a half"
        );
        for (component, level) in [
            (0.0, 0),
            (0.3, 77),
            (0.5, 128),
            (0.7, 178),
            (1.0, 255),
            (-1.0, 0),
            (2.0, 255),
            (f32::NAN, 0),
        ] {
            assert_eq!(nearest_level(component), level, "{component}");
        }
    }
}
