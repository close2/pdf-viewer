//! The stand-in drawn on the processor: the layers resampled into one picture on the presenting
//! thread, for the window with no graphics device (ADR 0461).

use pdf_render::{Color, Medium, Raster, RasterFormat, Transform};

/// One colour of the medium as the four bytes a raster holds it in.
///
/// The same rounding `crate::compose::compose` uses on its way to the window, so that the
/// pixels a stand-in puts outside the picture it holds are the pixels the frame after it will
/// put there — a revealed edge that changed shade when the true frame landed would be this host
/// telling a person something happened.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "each channel is clamped to 0..=1 and scaled to 0..=255 before the cast"
)]
fn bytes_of(colour: Color) -> [u8; 4] {
    [colour.r, colour.g, colour.b, colour.a]
        .map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// The window's own pixels, on the surface that has no device to hold them.
///
/// **The processor's base, and it costs nothing to obtain** (`doc/todo/37`, ADR 0457). The device
/// path's base is a texture the render thread drew and never gave up; this one is the raster
/// `crate::compose::compose_pages` produced, which `quorra::composer` already had in its hand
/// on the way to the window. There is no capture to arrange and no readback to price.
///
/// **It is kept in step with [`Stale::settled`] by being written in the same breath**, which is
/// the arrangement ADR 0385 arrived at the hard way: a base and the record of what it is a picture
/// of, updated by two different events, parted company and the window showed nothing for want of
/// pixels it was holding. `Composer::collect` adopts both or neither.
///
/// [`Stale::settled`]: super::Stale::settled
#[derive(Debug, Default)]
pub struct Canvas {
    /// The last frame this window drew, in the window's own extent.
    held: Option<Raster>,
}

impl Canvas {
    /// Adopts the frame that has just been drawn as the base the next stand-in resamples.
    ///
    /// The pages under it are not kept: the raster handed here is the whole window's, chrome
    /// excluded, exactly as the device path's page texture is.
    pub fn keep(&mut self, frame: Raster) {
        self.held = Some(frame);
    }

    /// The frame on hand, for the tick that presents it unchanged.
    ///
    /// A rendering of the view being asked for goes to the window as it is: there is nothing to
    /// resample, and walking every pixel of the window to copy it would be this surface paying for
    /// a transform the other one gets from a textured quad.
    #[must_use]
    pub fn frame(&self) -> Option<&Raster> {
        self.held.as_ref()
    }

    /// Draws the layers a stand-in is made of into one window raster, bottom upwards.
    ///
    /// **The same four layers `crate::renderer` puts on the other surface, in the same order**, and
    /// that is what makes one gesture look like one program rather than two: the medium
    /// everywhere, then the retained pages each under its own placement, then the base over them —
    /// the chrome being the presenter's and composited after this.
    ///
    /// - `base` is [`Stale::reproject`]'s answer, and maps a texel of the picture held to the pixel
    ///   of the window this view puts it at. `None` is a picture drawn from the retained pages
    ///   alone: a page turn, a `GoTo`, a resize, a zoom in a column.
    /// - `under` is one raster and one placement per retained page — **one placement per page
    ///   rather than one for the picture**, which is what lets this layer answer the cases the one
    ///   over it refuses ([`Proxies::placements`]).
    ///
    /// `None` where neither layer had anything to draw, which is a picture the caller must not
    /// present: a window that has not been drawn to is not the same thing as a window somebody
    /// emptied.
    ///
    /// **Bilinear, because the device path is** (`crate::renderer`'s `ImageFilter::Linear`): a
    /// blur is what an approximation should look like, where squares of four device pixels look
    /// like a rendering decision somebody made.
    ///
    /// **What no layer covers gets the window's medium**, never page white — asserting that the
    /// page is blank there is the plausible-looking lie principle 1 is about (ADR 0378), and it is
    /// what the device path's bottom layer paints for the same reason.
    ///
    /// [`Stale::reproject`]: super::Stale::reproject
    /// [`Proxies::placements`]: super::Proxies::placements
    #[must_use]
    pub fn stand_in(
        &self,
        extent: (u32, u32),
        base: Option<Transform>,
        under: &[(&Raster, Transform)],
    ) -> Option<Raster> {
        let held = self.held.as_ref().zip(base);
        if held.is_none() && under.is_empty() {
            return None;
        }
        let (width, height) = extent;
        let pixels = (width as usize).checked_mul(height as usize)?;
        let mut picture = Raster {
            width,
            height,
            format: RasterFormat::Rgba8,
            data: bytes_of(Medium::WINDOW.surround).repeat(pixels),
        };
        // The blurrier layer first, so that the sharp one is drawn over it wherever it has pixels
        // — the ordering `crate::renderer`'s layer list states, and the reason neither layer has
        // to know about the other.
        for (layer, placement) in under {
            paint(&mut picture, layer, *placement);
        }
        if let Some((layer, placement)) = held {
            paint(&mut picture, layer, placement);
        }
        Some(picture)
    }
}

/// Draws one layer's texels into `picture` under `placement`, leaving what it does not cover.
///
/// `placement` maps a texel of `layer` to a pixel of `picture`, so the map wanted here is its
/// **inverse** — every destination pixel asks which texel it is a sample of — and a placement that
/// does not invert is one `crate::stale` has already refused ([`Refusal::NoPlacement`]) rather than
/// a case to be silent about.
///
/// **Only the pixels the layer reaches are visited**, which is what makes a picture of several
/// layers cost what its layers cover rather than a window per layer: the four corners of the layer
/// under an affine bound its image exactly, because an affine image of a rectangle is a
/// parallelogram and a parallelogram is inside the box of its vertices.
///
/// [`Refusal::NoPlacement`]: super::Refusal::NoPlacement
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a raster dimension, capped at pdf_render::MAX_EXTENT = 2^24, which f32 represents \
              exactly; each bound is clamped into 0..width or 0..height before its cast"
)]
fn paint(picture: &mut Raster, layer: &Raster, placement: Transform) {
    match layer.format {
        // Exhaustive rather than checked, on ADR 0247's argument: the arithmetic below is written
        // for four straight-alpha bytes per pixel, and a second format must fail to compile here
        // rather than be refused at runtime.
        RasterFormat::Rgba8 => {}
    }
    let Some(back) = placement.invert() else {
        return;
    };
    let (span, rows) = (layer.width as f32, layer.height as f32);
    let corners = [(0.0, 0.0), (span, 0.0), (0.0, rows), (span, rows)]
        .map(|(x, y)| placement.apply(pdf_render::Point::new(x, y)));
    let least = |read: fn(&pdf_render::Point) -> f32, limit: u32| -> u32 {
        let found = corners.iter().map(read).fold(f32::INFINITY, f32::min);
        found.max(0.0).min(limit as f32) as u32
    };
    let most = |read: fn(&pdf_render::Point) -> f32, limit: u32| -> u32 {
        let found = corners.iter().map(read).fold(f32::NEG_INFINITY, f32::max);
        (found.max(0.0).min(limit as f32).ceil()) as u32
    };
    let (left, right) = (
        least(|point| point.x, picture.width),
        most(|point| point.x, picture.width),
    );
    let (top, bottom) = (
        least(|point| point.y, picture.height),
        most(|point| point.y, picture.height),
    );
    let stride = (picture.width as usize).saturating_mul(4);
    let (from_byte, to_byte) = (
        (top as usize).saturating_mul(stride),
        (bottom as usize).saturating_mul(stride),
    );
    let Some(reached) = picture.data.get_mut(from_byte..to_byte) else {
        return;
    };
    // **The rows are shared among the processor's cores** (ADR 1832). The resample runs on the
    // thread that presents, and rule 4 refuses a stand-in whose resample would not be up a refresh
    // before the true frame: on one core a 2500 × 1600 window resampled in 130–185 ms against
    // frames of 30–100 ms, so a zoom of an ordinary page was refused at every other notch and the
    // window held the previous picture unmoved — the owner's "does not resize previous frames".
    // Each row is independent of every other, so the split changes no byte; the cost is a scoped
    // thread a band, a few tens of microseconds against a resample of tens of milliseconds.
    let bands = std::thread::available_parallelism()
        .map_or(1, std::num::NonZeroUsize::get)
        .min(RESAMPLE_BANDS);
    let rows_in_a_band = (bottom.saturating_sub(top) as usize).div_ceil(bands).max(1);
    std::thread::scope(|scope| {
        for (band, lines) in reached
            .chunks_mut(rows_in_a_band.saturating_mul(stride).max(1))
            .enumerate()
        {
            let row = (top as usize).saturating_add(band.saturating_mul(rows_in_a_band));
            scope.spawn(move || {
                for (offset, line) in lines.chunks_exact_mut(stride).enumerate() {
                    paint_row(
                        line,
                        layer,
                        &back,
                        row.saturating_add(offset) as f32,
                        (left, right),
                        (span, rows),
                    );
                }
            });
        }
    });
}

/// The most bands one resample is split into: the cores past it buy a resample nothing a person
/// could see and take them from the frame being drawn beside it.
const RESAMPLE_BANDS: usize = 8;

/// One row of [`paint`]: every pixel from `left` to `right` that samples inside the layer.
#[expect(
    clippy::cast_precision_loss,
    reason = "a column of a raster capped at pdf_render::MAX_EXTENT = 2^24, which f32 represents \
              exactly"
)]
fn paint_row(
    line: &mut [u8],
    layer: &Raster,
    back: &Transform,
    row: f32,
    (left, right): (u32, u32),
    (span, rows): (f32, f32),
) {
    // Stepped along the row rather than transformed per pixel: an affine of `(column + 1, row)`
    // differs from one of `(column, row)` by exactly `(a, b)`, so a row is one `apply` and two
    // additions a pixel instead of four multiplications and two additions. It is the same
    // arithmetic to the last bit for a translation and a scale, which is every placement this
    // program composes, and within an `f32` rounding of it for one with rotation in it. **It is
    // not where the time goes** and is not claimed to be: this loop with the sampling taken out of
    // it is 0.13 ms of a resample that is tens, so what it saves is inside the noise and what
    // justifies it is that it is also shorter to read.
    let mut at = back.apply(pdf_render::Point::new(left as f32 + 0.5, row + 0.5));
    for column in left..right {
        if at.x >= 0.0 && at.x < span && at.y >= 0.0 && at.y < rows {
            let from = (column as usize).saturating_mul(4);
            if let Some(pixel) = line.get_mut(from..from.saturating_add(4)) {
                pixel.copy_from_slice(&bilinear(layer, at.x - 0.5, at.y - 0.5));
            }
        }
        at.x += back.a;
        at.y += back.b;
    }
}

/// One bilinear sample of `raster`, at a point in the space where texel centres are the integers.
///
/// The caller has already established that the *pixel* centre is inside the picture; the four
/// texels around it need not be, and the indices are clamped to the edge — which is what a
/// clamped sampler does and what the half-texel border either side of the picture is. It is at
/// most half a texel of edge repetition on a picture that is a deliberate approximation.
///
/// # There is no `floor` and no `round` in here, and that is worth six libm calls a pixel
///
/// **Measured, and measured as a *ratio* on purpose** (ADR 0457). `x86-64` without `SSE4.1` has no
/// rounding instruction, so `f32::floor` and `f32::round` are calls into `libm` — two for the
/// texel and four for the channels, six per pixel and 4.8 million on an 800 × 1000 window.
/// Truncation *is* `floor` for a non-negative value and `+ 0.5` then truncation *is* `round` for
/// one already inside `0..=255`, and the `max` above and the convexity below are what establish
/// both. Timing the two forms **alternately in one process**, 25 samples each and the best of
/// them, an 800 × 1000 resample is **1.3 to 1.8 times dearer with the calls** — 48.3 against 35.3
/// on a scroll, 55.4 against 41.8 at the identity, 157.1 against 86.1 on a zoom.
///
/// The absolute figures are worthless and are quoted only so that the ratio's samples are legible:
/// the machine was running three other rounds' gates at a load average of over 70 on 24 cores, and
/// this kernel is memory-bound. That is exactly why the ratio was taken by interleaving rather
/// than by two runs — and why **there is no constant here at all**: rule 4 measures what a
/// resample costs on the machine it is running on ([`Stale::resampling`]), so a slower or faster
/// one changes the policy's answer rather than falsifying a number in this file.
///
/// **Fixed-point weights were tried in the same way and rejected**, which is the other half of
/// `CLAUDE.md`'s rule about optimisation: interpolating in `u32` with the weights quantised to
/// 1/256 was 30.4 against 33.3, 30.4 against 26.8 and 37.8 against 33.9 — no gain outside the
/// noise — and it cost two levels of 255 of accuracy and a paragraph of justification. The float
/// form is what is here.
///
/// [`Stale::resampling`]: super::Stale::resampling
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "each index is clamped into 0..width or 0..height before the cast, and each channel \
              is a convex combination of four bytes and so is already inside 0..=255"
)]
fn bilinear(raster: &Raster, x: f32, y: f32) -> [u8; 4] {
    // At least zero, so that the cast below truncates towards the same side `floor` rounds to. A
    // point between the picture's edge and the first texel *centre* belongs to that texel, which
    // is the clamped sampler's answer and is what this `max` says.
    let (x, y) = (x.max(0.0), y.max(0.0));
    let (left, top) = (x as usize, y as usize);
    #[expect(
        clippy::cast_precision_loss,
        reason = "a raster index, far inside f32's exact integer range"
    )]
    let (fraction_x, fraction_y) = (x - left as f32, y - top as f32);
    // The last texel of each axis. A raster with no pixels holds no sample and the caller's
    // bounds test cannot pass on one, so the saturating subtraction never underflows into a
    // meaningful index.
    let (last_x, last_y) = (
        raster.width.saturating_sub(1) as usize,
        raster.height.saturating_sub(1) as usize,
    );
    let (x0, y0) = (left.min(last_x), top.min(last_y));
    let (x1, y1) = (
        left.saturating_add(1).min(last_x),
        top.saturating_add(1).min(last_y),
    );
    let texel = |column: usize, row: usize| -> [u8; 4] {
        let start = row
            .saturating_mul(raster.width as usize)
            .saturating_add(column)
            .saturating_mul(4);
        raster
            .data
            .get(start..start.saturating_add(4))
            .and_then(|texel| <[u8; 4]>::try_from(texel).ok())
            .unwrap_or([0, 0, 0, 0])
    };
    let (top_left, top_right) = (texel(x0, y0), texel(x1, y0));
    let (bottom_left, bottom_right) = (texel(x0, y1), texel(x1, y1));
    // The weights in eighths of a byte rather than in floating point, which is the second half of
    // this function's measurement and the larger half. A float weight means a **float-to-integer
    // cast per channel** on the way out, and Rust's is saturating — a compare, a select and a NaN
    // test apiece, four times a pixel, which no compiler can turn into anything cheaper without
    // being told the value is already in range and having no way to be told. Fixed point pays two
    // casts a pixel instead of four and leaves the rest as integer multiplies a compiler can put
    // four at a time into a register.
    //
    // What it costs is under two levels of 255 at the worst pixel: quantising each weight to
    // 1/256 moves the interpolation by at most `|left − right| / 256` per axis, and a channel's
    // two texels differ by at most 255. That is invisible, and it is on a picture whose *other*
    // error is a whole-texture resample of the wrong view.
    let mut sample = [0_u8; 4];
    for (channel, out) in sample.iter_mut().enumerate() {
        let above = f32::from(top_left[channel])
            + fraction_x * (f32::from(top_right[channel]) - f32::from(top_left[channel]));
        let below = f32::from(bottom_left[channel])
            + fraction_x * (f32::from(bottom_right[channel]) - f32::from(bottom_left[channel]));
        // A convex combination of four values in `0..=255` is in `0..=255`, so the clamp is the
        // arithmetic's rather than a guard, and `+ 0.5` with truncation is what `round` does to a
        // non-negative value. See this function's own note for what the call cost.
        *out = ((above + fraction_y * (below - above)).clamp(0.0, 255.0) + 0.5) as u8;
    }
    sample
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A window of pixels, one colour, for the resample to move about.
    fn canvas(width: u32, height: u32, colour: [u8; 4]) -> Canvas {
        let mut canvas = Canvas::default();
        canvas.keep(Raster {
            width,
            height,
            format: RasterFormat::Rgba8,
            data: colour.repeat((width as usize).saturating_mul(height as usize)),
        });
        canvas
    }

    /// One raster of a colour, as a layer under a stand-in.
    fn sheet(width: u32, height: u32, colour: [u8; 4]) -> Raster {
        Raster {
            width,
            height,
            format: RasterFormat::Rgba8,
            data: colour.repeat((width as usize).saturating_mul(height as usize)),
        }
    }

    /// No layers at all is not a picture, and the caller must not present one: a window that has
    /// not been drawn to is not the same thing as a window somebody emptied.
    #[test]
    fn a_stand_in_of_no_layers_is_not_a_picture() {
        assert!(
            Canvas::default()
                .stand_in((8, 4), Some(Transform::IDENTITY), &[])
                .is_none(),
            "a base asked for with no frame held is nothing to draw"
        );
        assert!(
            canvas(8, 4, [1, 2, 3, 255])
                .stand_in((8, 4), None, &[])
                .is_none(),
            "and neither is a frame held with no layer naming it"
        );
    }

    /// **The processor's stand-in is the same picture the device's is**, which is the property
    /// that keeps one gesture from looking like two programs: at the identity it is the frame
    /// itself, unchanged, because a bilinear sample at texel centres is the texel.
    #[test]
    fn a_resample_at_the_identity_is_the_picture_it_holds() {
        let held = canvas(8, 4, [200, 100, 50, 255]);
        let picture = held
            .stand_in((8, 4), Some(Transform::IDENTITY), &[])
            .expect("a frame is held");
        assert_eq!(picture.width, 8);
        assert_eq!(picture.height, 4);
        assert_eq!(picture.data, [200, 100, 50, 255].repeat(32));
    }

    /// A translation moves the pixels by it, and **what the picture held has nothing to say about
    /// gets the window's medium** — never page white, which would assert that the page is blank
    /// there (ADR 0378). The revealed edge is the case the device path's bottom layer paints.
    #[test]
    fn a_scroll_moves_the_pixels_and_the_revealed_edge_is_the_medium() {
        let held = canvas(4, 4, [255, 255, 255, 255]);
        // The view moved up by two pixels: every texel is drawn two rows higher, and the bottom
        // two rows of the window are area the picture held does not cover.
        let picture = held
            .stand_in((4, 4), Some(Transform::translate(0.0, -2.0)), &[])
            .expect("a frame is held");
        let row = |index: usize| &picture.data[index * 4 * 4..(index + 1) * 4 * 4];
        assert_eq!(row(0), [255, 255, 255, 255].repeat(4), "still the page");
        let medium = bytes_of(Medium::WINDOW.surround);
        assert_eq!(row(3), medium.repeat(4), "the edge this scroll revealed");
        assert_ne!(
            medium,
            [255, 255, 255, 255],
            "or the case is not being made"
        );
    }

    /// **The revealed edge is filled by the retained page, and the base still wins where it has
    /// pixels** — the four-layer order `crate::renderer` puts on the other surface, drawn by the
    /// processor for the window that has no device (ADR 0461).
    ///
    /// The scroll of the case above, with a whole page under it: the rows the base has moved off
    /// show the page rather than the medium, and the rows it still covers are unchanged. That is
    /// what makes the two layers complementary rather than competing.
    #[test]
    fn a_retained_page_fills_what_the_moved_base_no_longer_covers() {
        let held = canvas(4, 4, [255, 255, 255, 255]);
        let page = sheet(4, 4, [10, 20, 30, 255]);
        let picture = held
            .stand_in(
                (4, 4),
                Some(Transform::translate(0.0, -2.0)),
                &[(&page, Transform::IDENTITY)],
            )
            .expect("a frame is held");
        let row = |index: usize| &picture.data[index * 4 * 4..(index + 1) * 4 * 4];
        assert_eq!(
            row(0),
            [255, 255, 255, 255].repeat(4),
            "the sharp layer is drawn over the blurred one wherever it has pixels"
        );
        assert_eq!(
            row(3),
            [10, 20, 30, 255].repeat(4),
            "and the edge the scroll revealed is the retained page rather than the medium"
        );
    }

    /// A picture made of the retained pages alone — a page turn, where nothing about the outgoing
    /// page's pixels is true of the incoming one at any placement.
    ///
    /// The page is placed at half scale in the corner of the window, so the assertion is also
    /// about the thing a layered composition can get wrong: what the layer does *not* cover is
    /// left alone rather than cleared.
    #[test]
    fn a_page_turn_is_drawn_from_the_retained_page_and_the_medium() {
        let held = canvas(4, 4, [255, 255, 255, 255]);
        let page = sheet(2, 2, [10, 20, 30, 255]);
        let picture = held
            .stand_in((4, 4), None, &[(&page, Transform::IDENTITY)])
            .expect("a retained page is a picture");
        let at = |column: usize, row: usize| {
            let start = (row * 4 + column) * 4;
            &picture.data[start..start + 4]
        };
        assert_eq!(at(0, 0), [10, 20, 30, 255], "the page, where it is placed");
        assert_eq!(at(1, 1), [10, 20, 30, 255], "its far corner");
        let medium = bytes_of(Medium::WINDOW.surround);
        assert_eq!(at(3, 3), medium, "and the window beside it");
    }
}
