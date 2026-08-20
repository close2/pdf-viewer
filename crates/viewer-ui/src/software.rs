//! The window's pixels, written by the processor, with no graphics device anywhere.
//!
//! **This exists so that `--cpu` can mean what it says.** Until the
//! three-hundred-and-eighty-fourth session the flag chose which rasteriser drew the page and
//! nothing else: the presenter still created a `wgpu::Instance`, still selected an adapter and
//! still made a device, and the processor's raster was handed *back* to that device as one image
//! because a working surface was the only path pixels took to the screen. A driver that faults
//! while it is being loaded therefore took the process down under `--cpu` exactly as it did
//! without it, which is what the project owner hit on a Windows machine with Intel graphics
//! (`doc/QUORRA_FEEDBACK.md` section 12, and ADR 0221 for this half of it).
//!
//! So there are two ways a frame reaches the window, and this is the second. It is deliberately
//! the smaller of the two in every way — one full-window buffer, one source-over per overlay, one
//! copy to the window — because the page it draws was already rasterised on the processor and the
//! difference in cost is not where this path is slow.
//!
//! # What it is not
//!
//! It is not a second renderer. Everything it draws comes from [`CpuRasterizer`], which is this
//! project's correctness oracle, through the same display lists the graphics device is handed.
//! What is here is compositing — §11.3.6's simple case, straight alpha and no blend mode, over
//! lists that are already in the window's own pixels — and a copy.

use std::num::NonZeroU32;
use std::sync::Arc;

use pdf_render::{
    Color, DisplayList, Raster, RasterFormat, Rasterizer as _, TargetSpec, Transform,
};
use render_cpu::{CpuRasterError, CpuRasterizer};
use winit::window::Window;

/// Why a frame could not reach the window without a graphics device.
#[derive(Debug, thiserror::Error)]
pub enum SoftwareError {
    /// This platform, or this build, has no software path onto a window.
    ///
    /// The X11 and Wayland backends are compiled in here and the DRM/KMS one is not, so on Linux
    /// this is a session neither protocol answers for. See the manifest's note on the feature set.
    #[error("no software surface for this window: {0}")]
    Surface(#[from] softbuffer::SoftBufferError),
    /// An overlay could not be drawn on the processor.
    #[error("an overlay could not be drawn: {0}")]
    Overlay(#[from] CpuRasterError),
    /// A page of Table 29's arrangement could not be drawn on the processor.
    ///
    /// Named apart from [`Self::Overlay`] although both are the same rasteriser's refusal, because
    /// the two are different sentences to a person: a page that will not draw is a fact about the
    /// document, and an overlay that will not draw is a defect in this host's own chrome.
    #[error("page {page} could not be drawn: {problem}")]
    Page {
        /// Which page of the arrangement, counting from one as a reader does.
        page: usize,
        /// What the processor said.
        problem: CpuRasterError,
    },
    /// The window has no extent — minimised, or between a resize and its first frame.
    ///
    /// Not an error the caller reports: there is nothing to present *to*, which is different
    /// from failing to present.
    #[error("the window has no extent")]
    NoExtent,
    /// The raster offered is not the size the surface was configured for.
    ///
    /// A defect in this host rather than a fact about the machine — the page is rasterised at
    /// the window's own size — so it says both numbers rather than silently drawing part of a
    /// frame.
    #[error("a {given_width}x{given_height} raster for a {width}x{height} window")]
    WrongSize {
        /// The raster's width.
        given_width: u32,
        /// The raster's height.
        given_height: u32,
        /// The window's width.
        width: u32,
        /// The window's height.
        height: u32,
    },
}

/// A window that is written to by the processor.
///
/// Held instead of a `QuorraWindowRenderer`, never beside one: the whole point is that a run
/// holding one of these has opened no driver.
pub struct SoftwareSurface {
    /// Kept for its lifetime rather than read again. The backends take what they need from it
    /// when the surface is made, and dropping it here would be relying on that.
    _context: softbuffer::Context<Arc<Window>>,
    surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
}

impl std::fmt::Debug for SoftwareSurface {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("SoftwareSurface").finish()
    }
}

impl SoftwareSurface {
    /// Takes a software surface on `window`.
    ///
    /// # Errors
    ///
    /// [`SoftwareError::Surface`] where the platform has no software path onto a window, which
    /// is a fact about the session rather than about this document.
    pub fn new(window: Arc<Window>) -> Result<Self, SoftwareError> {
        let context = softbuffer::Context::new(Arc::clone(&window))?;
        let surface = softbuffer::Surface::new(&context, window)?;
        Ok(Self {
            _context: context,
            surface,
        })
    }

    /// Draws `page` under `overlays` and puts the result on the window.
    ///
    /// `page` must already be the window's size: the caller rasterises it through a
    /// [`TargetSpec`] whose extent is the window's and whose transform carries the page's
    /// placement, which is the same spec the graphics device is given.
    ///
    /// # Errors
    ///
    /// [`SoftwareError::NoExtent`] for a window with no pixels, [`SoftwareError::WrongSize`] for
    /// a raster that is not the window's, [`SoftwareError::Overlay`] where an overlay would not
    /// draw, and [`SoftwareError::Surface`] where the platform refused the buffer or the copy.
    pub fn present(
        &mut self,
        page: &Raster,
        overlays: &[&DisplayList],
    ) -> Result<(), SoftwareError> {
        let (Some(width), Some(height)) =
            (NonZeroU32::new(page.width), NonZeroU32::new(page.height))
        else {
            return Err(SoftwareError::NoExtent);
        };
        self.surface.resize(width, height)?;
        let mut buffer = self.surface.buffer_mut()?;
        if buffer.len() != pixels(page) {
            // `resize` succeeded, so this is the platform's buffer disagreeing with the size it
            // was just given — reported rather than trusted, because the alternative is writing
            // a partial frame and calling it a present.
            return Err(SoftwareError::WrongSize {
                given_width: page.width,
                given_height: page.height,
                width: width.get(),
                height: height.get(),
            });
        }

        let composed = compose(page, overlays)?;
        // softbuffer's pixel is `00000000RRRRRRRRGGGGGGGGBBBBBBBB` on every platform it
        // supports, so the window is opaque by construction and the page's alpha — which
        // `impose_on_medium` has already resolved against the medium — is dropped here.
        for (pixel, rgba) in buffer.iter_mut().zip(composed.data.chunks_exact(4)) {
            *pixel = u32::from_be_bytes([0, rgba[0], rgba[1], rgba[2]]);
        }
        buffer.present()?;
        Ok(())
    }
}

/// How many pixels a raster holds: the count of whole RGBA groups in its bytes.
///
/// Counted through `chunks_exact` rather than divided, so there is no arithmetic to be wrong
/// about and a truncated buffer answers with what it actually holds.
fn pixels(raster: &Raster) -> usize {
    raster.data.chunks_exact(4).len()
}

/// Draws every page of Table 29's arrangement into one window-sized raster, in the order given.
///
/// ISO 32000-2 §7.7.2's `OneColumn` and the four two-page values put several pages in one window,
/// and each arrives with its own [`TargetSpec`] whose extent is the window's and whose transform
/// carries that page's placement — the same specs the graphics device is handed. So the first
/// page is drawn as it always was, onto the medium, and every page after it onto transparency and
/// composited over what is there. The pages do not overlap, so the order is the arrangement's
/// rather than a compositing decision.
///
/// **The same shape as [`compose`], and for the same reason**: [`CpuRasterizer`] answers with a
/// new raster rather than drawing into one, so a page costs a window's worth of pixels and a
/// source-over. There are at most `viewer_core`'s bound of them, and this is the path that is
/// already the slow one.
///
/// # Errors
///
/// [`SoftwareError::NoExtent`] where the arrangement places no page at all, and
/// [`SoftwareError::Page`] where one would not rasterise.
pub fn compose_pages(pages: &[(&DisplayList, TargetSpec)]) -> Result<Raster, SoftwareError> {
    /// Which page of the arrangement refused, counting from one.
    fn refused(index: usize) -> impl Fn(CpuRasterError) -> SoftwareError {
        move |problem| SoftwareError::Page {
            page: index.saturating_add(1),
            problem,
        }
    }
    let Some(((first, target), rest)) = pages.split_first() else {
        return Err(SoftwareError::NoExtent);
    };
    let mut composed = CpuRasterizer::new()
        .rasterize(first, *target)
        .map_err(refused(0))?;
    match composed.format {
        RasterFormat::Rgba8 => {}
    }
    for (index, (list, placed)) in rest.iter().enumerate() {
        // Transparent for the same reason an overlay is: this is a layer over what is already
        // drawn, and the medium under it has been painted once by the page above.
        let over = CpuRasterizer::new()
            .with_background(Color::TRANSPARENT)
            .rasterize(list, *placed)
            .map_err(refused(index.saturating_add(1)))?;
        match over.format {
            RasterFormat::Rgba8 => {}
        }
        source_over(&mut composed, &over);
    }
    Ok(composed)
}

/// Draws every overlay over `page`, in order, and answers the result.
///
/// The overlays are window-pixel display lists — selection quads, the sidebar, the modal card —
/// so each is rasterised at the identity transform into the page's own extent and composited.
/// Separate from [`SoftwareSurface::present`] and public for one reason: it is the half of the
/// software path that can be tested without a window, and it is the half with the arithmetic in
/// it.
///
/// **A copy per overlay, deliberately.** [`CpuRasterizer`] answers with a new raster rather than
/// drawing into one, so the alternative is a second entry point into the oracle for the sake of a
/// path that is already the slow one. There are between zero and six of these lists on a frame.
///
/// # Errors
///
/// [`SoftwareError::Overlay`] where an overlay would not rasterise, which is a defect in this
/// host's own chrome rather than anything about the document.
pub fn compose(page: &Raster, overlays: &[&DisplayList]) -> Result<Raster, SoftwareError> {
    // Exhaustive, and that is what ADR 0247 bought: this function's arithmetic is written for
    // four straight-alpha bytes per pixel, and a second `RasterFormat` now fails to compile here
    // rather than being refused at runtime. Both operands are checked because both are rasters —
    // the page from the worker and each overlay from `CpuRasterizer` below.
    match page.format {
        RasterFormat::Rgba8 => {}
    }
    let mut composed = page.clone();
    for list in overlays {
        let spec = TargetSpec {
            width: page.width,
            height: page.height,
            transform: Transform::IDENTITY,
        };
        // Transparent rather than white: this is a layer over the page, and a white background
        // would erase it. `Color::TRANSPARENT` is what `CpuRasterizer::with_background`'s own
        // documentation names for exactly this.
        let over = CpuRasterizer::new()
            .with_background(Color::TRANSPARENT)
            .rasterize(list, spec)?;
        match over.format {
            RasterFormat::Rgba8 => {}
        }
        source_over(&mut composed, &over);
    }
    Ok(composed)
}

/// Composites `top` onto `under` in place: source-over, straight alpha, no blend mode.
///
/// §11.3.6's formula with the group machinery removed, which is what is left when both operands
/// are already device pixels at the same extent and neither is in a transparency group. Both
/// rasters are [`RasterFormat::Rgba8`], which [`compose`] has checked — this function is private
/// so that the check has exactly one caller to be true of.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "each channel is a ratio of two bytes scaled back to 0..=255 and clamped there \
              before the cast, so neither truncation nor a negative value is reachable"
)]
fn source_over(under: &mut Raster, top: &Raster) {
    for (dst, src) in under.data.chunks_exact_mut(4).zip(top.data.chunks_exact(4)) {
        let source_alpha = f32::from(src[3]) / 255.0;
        if source_alpha <= 0.0 {
            continue;
        }
        if source_alpha >= 1.0 {
            dst.copy_from_slice(src);
            continue;
        }
        let under_alpha = f32::from(dst[3]) / 255.0;
        let alpha = source_alpha + under_alpha * (1.0 - source_alpha);
        if alpha <= 0.0 {
            dst.copy_from_slice(&[0, 0, 0, 0]);
            continue;
        }
        for channel in 0..3 {
            let source = f32::from(src[channel]) / 255.0;
            let below = f32::from(dst[channel]) / 255.0;
            let value =
                (source * source_alpha + below * under_alpha * (1.0 - source_alpha)) / alpha;
            dst[channel] = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
        dst[3] = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pdf_render::{
        BlendMode, Color, Command, DisplayList, Paint, Path, PathCommand, Point, Raster,
        RasterFormat, Size,
    };

    use super::compose;

    /// A window-sized opaque page, one colour throughout.
    fn page(width: u32, height: u32, colour: [u8; 4]) -> Raster {
        Raster {
            width,
            height,
            format: RasterFormat::Rgba8,
            data: colour.repeat((width as usize).saturating_mul(height as usize)),
        }
    }

    /// An overlay that fills the left half of the window with `paint`.
    fn left_half(width: u32, height: u32, paint: Color) -> DisplayList {
        #[expect(
            clippy::cast_precision_loss,
            reason = "test extents are tens of pixels"
        )]
        let mut list = DisplayList::new(Size::new(width as f32, height as f32));
        let mut path = Path::new();
        #[expect(
            clippy::cast_precision_loss,
            reason = "test extents are tens of pixels"
        )]
        let (half, tall) = ((width / 2) as f32, height as f32);
        path.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
        path.push(PathCommand::LineTo(Point::new(half, 0.0)));
        path.push(PathCommand::LineTo(Point::new(half, tall)));
        path.push(PathCommand::LineTo(Point::new(0.0, tall)));
        path.push(PathCommand::Close);
        list.push(Command::Fill {
            path: Arc::new(path),
            transform: pdf_render::Transform::IDENTITY,
            paint: Paint::Solid(paint),
            fill_rule: pdf_render::FillRule::NonZero,
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        });
        list
    }

    /// No overlays is the page itself: the identity of the composition, which is what a
    /// `--cpu` run with no sidebar and no selection presents.
    #[test]
    fn no_overlays_leaves_the_page_alone() {
        let under = page(8, 4, [200, 100, 50, 255]);
        let composed = compose(&under, &[]).expect("compose");
        assert_eq!(composed, under);
    }

    /// An opaque overlay replaces what it covers and leaves the rest.
    #[test]
    fn an_opaque_overlay_covers_the_page() {
        let under = page(8, 4, [255, 255, 255, 255]);
        let list = left_half(8, 4, Color::BLACK);
        let composed = compose(&under, &[&list]).expect("compose");
        assert_eq!(&composed.data[0..4], &[0, 0, 0, 255], "the covered half");
        assert_eq!(
            &composed.data[7 * 4..8 * 4],
            &[255, 255, 255, 255],
            "the half nothing was drawn on"
        );
    }

    /// A half-transparent overlay is a half-way colour, and the result stays opaque —
    /// the property that makes the 0RGB copy to the window lossless.
    #[test]
    fn a_translucent_overlay_mixes_and_stays_opaque() {
        let under = page(8, 4, [255, 255, 255, 255]);
        let list = left_half(
            8,
            4,
            Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.5,
            },
        );
        let composed = compose(&under, &[&list]).expect("compose");
        assert_eq!(composed.data[3], 255, "an opaque page stays opaque");
        assert!(
            (100..=160).contains(&composed.data[0]),
            "half of white over black is a middle grey, not {}",
            composed.data[0]
        );
    }

    /// Table 29's column on the processor: two pages, each under its own placement, in one
    /// window-sized raster.
    ///
    /// The property that matters is that the **second page reaches the window at all** — a
    /// composition that drew the first and stopped would be the tier-2 host drawing one page of a
    /// column, which is exactly what ADR 0442 was for. Each list here fills the whole target under
    /// its own transform, so the second covers the first and the result is the second's colour.
    #[test]
    fn every_page_of_the_arrangement_reaches_the_raster() {
        let first = left_half(8, 4, Color::BLACK);
        let second = left_half(
            8,
            4,
            Color {
                r: 0.0,
                g: 0.0,
                b: 1.0,
                a: 1.0,
            },
        );
        // The second page placed four pixels to the right of the first, which is what a column's
        // second entry is: the same list at a different origin.
        let whole = pdf_render::TargetSpec {
            width: 8,
            height: 4,
            transform: pdf_render::Transform::IDENTITY,
        };
        let moved = pdf_render::TargetSpec {
            transform: pdf_render::Transform::translate(4.0, 0.0),
            ..whole
        };
        let composed =
            super::compose_pages(&[(&first, whole), (&second, moved)]).expect("two pages");
        assert_eq!(&composed.data[0..4], &[0, 0, 0, 255], "the first page");
        assert_eq!(
            &composed.data[5 * 4..6 * 4],
            &[0, 0, 255, 255],
            "the second page, at the origin the arrangement gave it"
        );
    }

    /// An arrangement that places no page has nothing to compose, and says so rather than
    /// answering with an empty raster a caller would present.
    #[test]
    fn no_pages_is_not_a_frame() {
        assert!(matches!(
            super::compose_pages(&[]),
            Err(super::SoftwareError::NoExtent)
        ));
    }

    /// Overlays are drawn in the order they are given: the modal card is over the sidebar
    /// because it is later in the list, and nothing else decides that.
    #[test]
    fn later_overlays_are_drawn_over_earlier_ones() {
        let under = page(8, 4, [255, 255, 255, 255]);
        let first = left_half(8, 4, Color::BLACK);
        let second = left_half(
            8,
            4,
            Color {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        );
        let composed = compose(&under, &[&first, &second]).expect("compose");
        assert_eq!(&composed.data[0..4], &[255, 0, 0, 255]);
    }
}
