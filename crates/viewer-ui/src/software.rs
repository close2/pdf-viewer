//! The window's pixels, written by the processor, with no graphics device anywhere.
//!
//! **This exists so that `--cpu` can mean what it says.** A flag that chose only the rasteriser
//! would still create a `wgpu::Instance`, select an adapter and make a device, and hand the
//! processor's raster *back* to that device as one image, because a working surface would be the
//! only path pixels took to the screen. A driver that faults while it is being loaded would then
//! take the process down under `--cpu` exactly as without it, which is what the project owner hit
//! on a Windows machine with Intel graphics (`doc/QUORRA_FEEDBACK.md` section 12, and ADR 0221 for
//! this half of it).
//!
//! So there are two ways a frame reaches the window, and this is the second. It is deliberately
//! the smaller of the two in every way — one full-window buffer, one source-over per overlay, one
//! copy to the window — because the page it draws was already rasterised on the processor and the
//! difference in cost is not where this path is slow.
//!
//! # What it is not
//!
//! It is not a second renderer. Everything it draws comes from [`render_cpu::CpuRasterizer`], which
//! is this project's correctness oracle, through the same display lists the graphics device is
//! handed. What is here is a copy; the compositing before it — §11.3.6's simple case, straight
//! alpha and no blend mode, over lists already in the window's own pixels — is
//! [`viewer_device::compose`]'s, which the device's processor lane draws with too (ADR 2052).

use std::num::NonZeroU32;
use std::sync::Arc;

use pdf_render::{DisplayList, Raster};
use render_cpu::CpuRasterError;
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
    /// [`pdf_render::TargetSpec`] whose extent is the window's and whose transform carries the page's
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

// The processor's composition, which the device's processor lane draws with as well, lives in
// `viewer-device` (ADR 2052); it is named here too so that a window with no device reads it where
// it always has.
pub use viewer_device::compose::{
    Assembly, ComposeError, Looks, Seen, compose, compose_pages, compose_pages_interruptibly,
    compose_pages_watched, over_the_ground, surround, was_interrupted,
};

impl From<ComposeError> for SoftwareError {
    /// Each refusal of composition as the variant of the same name and the same sentence.
    fn from(refusal: ComposeError) -> Self {
        match refusal {
            ComposeError::Overlay(problem) => Self::Overlay(problem),
            ComposeError::Page { page, problem } => Self::Page { page, problem },
            ComposeError::NoExtent => Self::NoExtent,
        }
    }
}
