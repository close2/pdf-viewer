//! An unsmoothed image drawn by the sample each device pixel's centre maps back to on the page —
//! ISO 32000-2 §10.7.4, ADR 2100.
//!
//! > The position of the centre of such a pixel -in other words, the point whose coordinate values
//! > have fractional parts of one-half -shall be mapped back into source space to determine how to
//! > colour the pixel.
//!
//! The pixel is the page's: §10.7.4 puts pixel boundaries on the integers of device space, and how
//! a page is divided into strips, or a clip's rows into a band, is this program's choice. Handed
//! the image as a pattern, `tiny-skia` maps a centre from the first row of the surface it draws
//! into, in `f32`, and a strip's first row moves that sum into another binade — so where a centre
//! lay within a rounding of a sample boundary, a strip took the neighbouring sample and a pixel
//! moved by a whole level of 255 (`bug1799927.pdf` at four times, 212 pixels of one row of
//! stencils). Here the centre is stated in the page's own device coordinates and the mark's
//! matrix and the page's are composed and inverted in `f64`, so the sample a pixel takes is a
//! function of where the pixel is on the page; the library is then handed one sample per device
//! pixel at whole pixels, where its sampler cannot change one.

use pdf_render::Transform;

use crate::surface::ToDevice;
use crate::{CpuRasterError, scan};

/// Where an image is placed: its own matrix, and the map from page space onto the surface.
#[derive(Debug, Clone, Copy)]
pub(super) struct Placement {
    /// The command's matrix, which carries the unit square onto page space.
    pub(super) mark: Transform,
    /// Page space onto the surface's device grid.
    pub(super) to_device: ToDevice,
}

/// How the samples are put onto the surface: the three paint settings `draw_image` decides.
#[derive(Debug, Clone, Copy)]
pub(super) struct Drawn {
    /// The command's constant opacity.
    pub(super) alpha: f32,
    /// The blend mode the library composites with.
    pub(super) blend: tiny_skia::BlendMode,
    /// Whether the square's edge is anti-aliased.
    pub(super) anti_alias: bool,
}

/// Draws `samples` — premultiplied, one pixel a sample — onto `pixmap` by §10.7.4's pixel centres,
/// through the unit square under `placement`.
pub(super) fn draw(
    pixmap: &mut tiny_skia::PixmapMut<'_>,
    samples: &tiny_skia::Pixmap,
    placement: Placement,
    drawn: Drawn,
    clip: scan::Clip<'_>,
) -> Result<(), CpuRasterError> {
    let Some(on_page) = Affine::on_page(placement) else {
        return Ok(());
    };
    let Some(region) = Region::reached(on_page, placement.to_device.top, pixmap) else {
        return Ok(());
    };
    let Some(centred) = sampled(samples, on_page, region) else {
        return Err(CpuRasterError::Allocation {
            width: region.width(),
            height: region.height(),
        });
    };
    let paint = tiny_skia::Paint {
        shader: tiny_skia::Pattern::new(
            centred.as_ref(),
            tiny_skia::SpreadMode::Pad,
            tiny_skia::FilterQuality::Nearest,
            drawn.alpha.clamp(0.0, 1.0),
            region.on_surface(),
        ),
        blend_mode: drawn.blend,
        anti_alias: drawn.anti_alias,
        force_hq_pipeline: crate::HIGH_PRECISION_PIPELINE,
        ..tiny_skia::Paint::default()
    };
    // The square is carried onto the surface here rather than by the converter, so that the
    // paint's transform is the device's own; the converter maps a point by the same `map_points`
    // either way, so the region it covers is the one it covered when it was handed the matrix.
    let at = crate::convert::transform(placement.to_device.of(placement.mark));
    let Some(square) = super::unit_square()?.transform(at) else {
        return Ok(());
    };
    scan::fill(
        pixmap,
        &square,
        &paint,
        tiny_skia::FillRule::Winding,
        tiny_skia::Transform::identity(),
        clip,
    );
    Ok(())
}

/// The unit square onto the page's device grid, in `f64`: `x' = a·x + c·y + e`,
/// `y' = b·x + d·y + f`, as [`Transform`] states it.
#[derive(Debug, Clone, Copy)]
struct Affine {
    linear: [f64; 4],
    offset: [f64; 2],
}

impl Affine {
    /// The mark's matrix followed by the page's — `Transform::then`'s product, taken in `f64` from
    /// the two matrices' own entries so that no rounding of the composition is the strip's either.
    /// `None` for a matrix with no inverse, which carries the square onto a line or a point.
    fn on_page(placement: Placement) -> Option<Self> {
        let wide = |t: Transform| [t.a, t.b, t.c, t.d, t.e, t.f].map(f64::from);
        let [a1, b1, c1, d1, e1, f1] = wide(placement.mark);
        let [a2, b2, c2, d2, e2, f2] = wide(placement.to_device.page);
        let composed = Self {
            linear: [
                a1 * a2 + b1 * c2,
                a1 * b2 + b1 * d2,
                c1 * a2 + d1 * c2,
                c1 * b2 + d1 * d2,
            ],
            offset: [e1 * a2 + f1 * c2 + e2, e1 * b2 + f1 * d2 + f2],
        };
        let determinant = composed.determinant();
        (determinant != 0.0 && determinant.is_finite()).then_some(composed)
    }

    fn determinant(self) -> f64 {
        let [a, b, c, d] = self.linear;
        a * d - b * c
    }

    /// A unit-square point onto the page's device grid.
    fn apply(self, point: (f64, f64)) -> (f64, f64) {
        let [a, b, c, d] = self.linear;
        (
            a * point.0 + c * point.1 + self.offset[0],
            b * point.0 + d * point.1 + self.offset[1],
        )
    }
}

/// The pixels of a surface the square can reach, in page columns and page rows.
#[derive(Debug, Clone, Copy)]
struct Region {
    columns: (f64, f64),
    rows: (f64, f64),
    /// The surface's first page row.
    top: f64,
}

impl Region {
    /// The square's device box on the page, a pixel wider on every side for the rounding of the
    /// corners the converter maps in `f32`, clamped to `pixmap`, which holds page rows from `top`
    /// on at the page's full width — `None` where that leaves no pixel.
    fn reached(on_page: Affine, top: u32, pixmap: &tiny_skia::PixmapMut<'_>) -> Option<Self> {
        let corners = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)].map(|at| on_page.apply(at));
        let (mut low, mut high) = (corners[0], corners[0]);
        for (x, y) in corners {
            low = (low.0.min(x), low.1.min(y));
            high = (high.0.max(x), high.1.max(y));
        }
        let top = f64::from(top);
        let columns = (
            (low.0.floor() - 1.0).max(0.0),
            (high.0.ceil() + 1.0).min(f64::from(pixmap.width())),
        );
        let rows = (
            (low.1.floor() - 1.0).max(top),
            (high.1.ceil() + 1.0).min(top + f64::from(pixmap.height())),
        );
        (columns.0 < columns.1 && rows.0 < rows.1).then_some(Self { columns, rows, top })
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a whole, positive count of the surface's own columns, which `rasterize` holds \
                  under MAX_EXTENT = 2^24"
    )]
    fn width(self) -> u32 {
        (self.columns.1 - self.columns.0) as u32
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a whole, positive count of the surface's own rows, as the width"
    )]
    fn height(self) -> u32 {
        (self.rows.1 - self.rows.0) as u32
    }

    /// Where the region's first pixel lies on the surface, as the paint's transform.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a whole pixel index below MAX_EXTENT = 2^24, exact in f32"
    )]
    fn on_surface(self) -> tiny_skia::Transform {
        tiny_skia::Transform::from_translate(self.columns.0 as f32, (self.rows.0 - self.top) as f32)
    }

    /// The `count` pixel centres from `from` on, as offsets from `origin`.
    fn centres(from: f64, count: u32, origin: f64) -> impl Iterator<Item = f64> {
        (0..count).map(move |step| from + f64::from(step) + 0.5 - origin)
    }
}

/// The sample each pixel of `region` takes, as a pixmap of the region's size.
///
/// A centre that maps beyond the samples takes the nearest edge sample, as the library's `Pad`
/// does: the square's anti-aliased edge reaches pixels whose centres lie just outside it.
fn sampled(
    samples: &tiny_skia::Pixmap,
    on_page: Affine,
    region: Region,
) -> Option<tiny_skia::Pixmap> {
    let (width, height) = (region.width(), region.height());
    let mut centred = tiny_skia::Pixmap::new(width, height)?;
    let [a, b, c, d] = on_page.linear;
    let [left, top] = on_page.offset;
    let determinant = on_page.determinant();
    let (across, down) = (f64::from(samples.width()), f64::from(samples.height()));
    // A coordinate on the sample grid to the index of the sample holding it. §8.9.4 gives each
    // sample one square unit with its upper-left corner at its own indices, and states no side for
    // a point on the edge two samples share; it is taken as the sample whose corner it is, as
    // §10.7.4's pixel is identified by its corner of minimum coordinates — so a coordinate is
    // truncated once it is known to be non-negative, which is the floor without the library call
    // `floor` is on a processor with no rounding instruction, and one beyond the last sample takes
    // the last.
    let index = |at: f64, count: f64| -> usize {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to the grid's own indices first, and a NaN, which a finite inverse \
                      does not produce, is 0"
        )]
        let whole = at.clamp(0.0, (count - 1.0).max(0.0)) as u32;
        whole as usize
    };
    let source = samples.pixels();
    let stride = samples.width() as usize;
    let lines = centred
        .pixels_mut()
        .chunks_exact_mut(width as usize)
        .zip(Region::centres(region.rows.0, height, top));
    // A centre's offset from the square's origin, `(x, y)`, goes onto the sample grid by the
    // placement's inverse and then §8.9.4's `[1/w 0 0 −1/h 0 1]` undone, the first sample row at
    // the square's top.
    //
    // An image whose sample columns run along the device's columns or along its rows — every
    // placement a page states without turning it by other than a quarter — takes one of its two
    // indices from the centre's `x` alone and the other from its `y` alone, so each is asked once
    // per column and once per row, by the one entry of the matrix that scales it and with that
    // division last: an image drawn at one sample a pixel from a half-pixel origin puts every
    // centre exactly on a sample's edge, and `x · w / a` computes such a centre exactly because the
    // product before the division is exact, where dividing first and multiplying back is not.
    let turned = a == 0.0 && d == 0.0;
    if turned || (b == 0.0 && c == 0.0) {
        let along_x: Vec<usize> = Region::centres(region.columns.0, width, left)
            .map(|x| {
                if turned {
                    index(down - x * down / c, down).saturating_mul(stride)
                } else {
                    index(x * across / a, across)
                }
            })
            .collect();
        for (line, y) in lines {
            let along_y = if turned {
                index(y * across / b, across)
            } else {
                index(down - y * down / d, down).saturating_mul(stride)
            };
            for (pixel, at) in line.iter_mut().zip(&along_x) {
                if let Some(sample) = source.get(along_y.saturating_add(*at)) {
                    *pixel = *sample;
                }
            }
        }
    } else {
        // Any other placement is asked per pixel, of the inverse's four entries divided once:
        // a turned or sheared grid has no centre on a sample's edge that a page states exactly.
        let (per_x, per_y) = (
            [d * across / determinant, b * down / determinant],
            [-c * across / determinant, -a * down / determinant],
        );
        for (line, y) in lines {
            let start = [per_y[0] * y, down + per_y[1] * y];
            for (pixel, x) in line
                .iter_mut()
                .zip(Region::centres(region.columns.0, width, left))
            {
                let at = index(start[1] + per_x[1] * x, down)
                    .saturating_mul(stride)
                    .saturating_add(index(start[0] + per_x[0] * x, across));
                if let Some(sample) = source.get(at) {
                    *pixel = *sample;
                }
            }
        }
    }
    Some(centred)
}
