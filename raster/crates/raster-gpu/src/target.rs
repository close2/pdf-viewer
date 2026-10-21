//! Three kinds of target, because there are three kinds of host.
//!
//! # Why three, and why this is the first milestone's work
//!
//! Section 6.1 of the brief measured an offscreen frame and found that **between 55% and 92%
//! of it is paid before any of the page is drawn** — 3.48 ms of 6.34 at 1×, 26.73 of
//! 29.13 at 4×. Most of that scales with *bytes*: 26.7 ms for a 32 MB target is about
//! 1.2 GB/s, which is what a mapped readback and a demultiply cost.
//!
//! **A window presenting to a swapchain does not pay the readback at all.** That is why
//! the brief asks for three target kinds rather than one, and why the ranking it gives
//! puts the surface and texture paths first: they delete the largest single item in the
//! frame.
//!
//! # The rule that holds for all three
//!
//! **Render onto transparency, always.** ISO 32000-2 §11.4.7 makes the page group
//! *isolated*, and a finished page is composited onto the medium by the caller after we
//! return it — painting the medium first is a different picture. Every target is
//! cleared to transparent before the scene draws, including a `Texture` the host owns.
//! A non-isolated group inside the page (§11.4.4, ADR 0019) seeds its own buffer from
//! its backdrop, never from the target: the page's own initial backdrop stays
//! transparent, which is what §11.4.7 requires of it.

/// Where a frame's pixels go.
#[derive(Debug)]
pub enum Target<'a> {
    /// Tier 1: the caller wants the pixels. The frame carries a
    /// [`Raster`](crate::frame::Raster) — straight-alpha RGBA8, converted once at this
    /// boundary (section 3 of the brief).
    ///
    /// This is the correctness oracle's path and the expensive one: it pays the
    /// copy-out, the map, and the demultiply, and [`Timings::readback`] prices exactly
    /// that. Brief section 6.1's numbers say this cost dominates an offscreen frame, which is why
    /// the other two tiers exist.
    ///
    /// [`Timings::readback`]: crate::frame::Timings::readback
    Readback,
    /// Tier 1 with the caller's backdrop put under the frame on the device: the pixels
    /// [`Readback`](Self::Readback) would hand back, composited over one opaque colour before
    /// they are copied out (the caller's `doc/adr/2150`).
    ///
    /// **Still onto transparency**: the scene draws exactly as it does for `Readback`, and the
    /// backdrop goes under the *finished* frame, which is where ISO 32000-2 §11.4.7 puts the
    /// medium. What moves is only where that composite runs. Every pixel then comes back
    /// opaque, so straight and premultiplied alpha are the same bytes and the host converts
    /// nothing; [`Timings::readback`] is the copy, the map and the row copy alone.
    ///
    /// [`Timings::readback`]: crate::frame::Timings::readback
    ReadbackOnto(Backdrop),
    /// Tier 2: draw into the surface the device was constructed with
    /// ([`Device::for_surface`]) and present. No readback.
    ///
    /// [`Device::for_surface`]: crate::device::Device::for_surface
    Surface,
    /// Tier 3: a texture the host owns and composites itself. No readback.
    ///
    /// The texture must be 2D, single-sampled, `Rgba8Unorm`, sized exactly to the
    /// viewport, and created with `RENDER_ATTACHMENT` usage on this same device; a
    /// texture that is not is refused with an error naming which requirement failed,
    /// before anything is drawn. Its previous contents are replaced: the device clears
    /// it to transparent, because a page renders onto transparency, always.
    Texture(&'a wgpu::Texture),
}

impl Target<'_> {
    /// The backdrop a [`Target::ReadbackOnto`] frame is put onto, and `None` for every other
    /// target.
    pub(crate) fn backdrop(&self) -> Option<&Backdrop> {
        match self {
            Self::ReadbackOnto(backdrop) => Some(backdrop),
            Self::Readback | Self::Surface | Self::Texture(_) => None,
        }
    }
}

/// What [`Target::ReadbackOnto`] puts under a finished frame.
///
/// The arithmetic is the caller's, stated in `doc/adr/2150` and run by `onto.wgsl`: each
/// premultiplied channel is first held to its own alpha — the bytes a straight-alpha readback
/// premultiplied again would be, which `doc/adr/2150` checks over every pair — then a pixel
/// outside [`Self::keep`] is cleared, and the colour goes under by premultiplied source-over in
/// eight bits, `c + ⌊(b·(255 − α) + 127) / 255⌋`, which leaves every alpha at 255.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Backdrop {
    /// The colour under the frame — red, green, blue — and opaque, which is what lets the
    /// frame skip the conversion to straight alpha altogether.
    pub colour: [u8; 3],
    /// The pixels that keep what the frame drew, as `[left, top, right, bottom)` in whole
    /// pixels of the target; every pixel outside shows the colour alone. `None` keeps every
    /// pixel.
    pub keep: Option<[u32; 4]>,
}
