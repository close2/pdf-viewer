//! Rendered frames supplied by a caller and copied back through the C boundary.

use super::{FrameInfo, Session};
use crate::events::Events;
use crate::status::Status;
use pdf_render::Raster;
use viewer_core::{Answer, Command, Query, RenderRequest, Rendered};

impl Session {
    /// Hands back the pixels a request asked for.
    ///
    /// The request carries the token, which is how a token that is opaque in Rust stays out of
    /// the header entirely: a caller returns what it was given. A stale one is dropped by the
    /// viewer, which is what stops a page turned mid-render from being overwritten by the frame
    /// the previous page produced.
    #[must_use]
    pub fn render_ready_raster(&mut self, request: &RenderRequest, raster: Raster) -> Events {
        self.handle(Command::RenderReady {
            token: request.token,
            rendered: Rendered::Raster(raster),
        })
    }

    /// Says that a request could not be drawn, and why.
    ///
    /// Reported rather than swallowed, which is [`Rendered::Failed`]'s own reason: a viewer that
    /// silently shows the previous page when a render fails is telling a person something false
    /// about the document.
    #[must_use]
    pub fn render_ready_failed(&mut self, request: &RenderRequest, why: String) -> Events {
        self.handle(Command::RenderReady {
            token: request.token,
            rendered: Rendered::Failed(why),
        })
    }

    /// What the viewer is holding, without the pixels: enough to size a buffer for them.
    ///
    /// The first half of C's two-call idiom, and the reason no pointer into the viewer is ever
    /// handed out. Answers the page, the extent, the pixel layout, how many bytes
    /// [`Self::frame_copy`] will write, and where the raster's top-left corner sits.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where the viewer holds no frame — before the first render, and for a
    /// tier-2 host that presented rather than handing pixels back.
    pub fn frame_info(&self, frame: usize) -> Result<FrameInfo, Status> {
        match self.viewer.query(Query::Frame) {
            Answer::Frame(frames) => frames.get(frame).map_or(Err(Status::NoAnswer), |frame| {
                Ok(FrameInfo {
                    page: frame.page,
                    width: frame.raster.width,
                    height: frame.raster.height,
                    format: frame.raster.format,
                    bytes: frame.raster.data.len(),
                    origin: frame.origin,
                })
            }),
            _ => Err(Status::NoAnswer),
        }
    }

    /// How many frames the viewer is holding — Table 29's arrangement, counted.
    ///
    /// **The entry point a C caller could not have deduced**, and the reason it exists rather than
    /// a changed `quorra_frame_info` alone: a C consumer cannot fail to compile, so a `/PageLayout`
    /// putting a second page on the screen has to be something a caller *asks* about. Zero for a
    /// tier-2 host, which hands no pixels back, and one for the `SinglePage` every document that
    /// says nothing about it opens in.
    #[must_use]
    pub fn frame_count(&self) -> usize {
        match self.viewer.query(Query::Frame) {
            Answer::Frame(frames) => frames.len(),
            _ => 0,
        }
    }

    /// Copies the frame into a buffer the caller owns.
    ///
    /// **One copy, which is what `doc/ui-boundary.md` prices tier 1 at** — the same copy
    /// `gdk::MemoryTexture` and `QImage` make, into a buffer the caller already has because it is
    /// about to upload it. Lending a pointer instead would save the copy and hand a C program a
    /// lifetime it has no way to keep.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where the viewer holds no frame, and [`Status::BufferTooSmall`] where
    /// `into` is shorter than [`FrameInfo::bytes`] said.
    pub fn frame_copy(&self, frame: usize, into: &mut [u8]) -> Result<usize, Status> {
        let Answer::Frame(frames) = self.viewer.query(Query::Frame) else {
            return Err(Status::NoAnswer);
        };
        let Some(frame) = frames.get(frame) else {
            return Err(Status::NoAnswer);
        };
        let data = &frame.raster.data;
        let Some(room) = into.get_mut(..data.len()) else {
            return Err(Status::BufferTooSmall);
        };
        room.copy_from_slice(data);
        Ok(data.len())
    }

    /// Say that the focused document's first frame is on the screen, so that its open sequence
    /// runs: RFC 0008 section 6.5 step 1, after the first present and never before it (ADR 1602).
    ///
    /// **Call it when [`Self::report`] is called**, once the reader has their page. It runs once a
    /// document, and what it says comes back on [`viewer_core::Event::Reported`] with no page.
    #[must_use]
    pub fn presented(&mut self) -> Events {
        self.handle(Command::Presented)
    }

    /// What this caller does with a page past [`viewer_core::MAX_PIXELS`].
    #[must_use]
    pub fn oversize(&mut self, how: viewer_core::Oversize) -> Events {
        self.handle(Command::Oversize(how))
    }

    /// Says the caller took a request's list and holds the page itself, which is
    /// [`viewer_core::Oversize::Listed`]'s answer; the request stays the caller's to draw windows
    /// of.
    #[must_use]
    pub fn render_ready_listed(&mut self, request: &RenderRequest) -> Events {
        self.handle(Command::RenderReady {
            token: request.token,
            rendered: Rendered::Listed,
        })
    }
}
