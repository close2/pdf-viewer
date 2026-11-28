//! A clip's pictures in a window driven from C: each picture converted on the processor by
//! `viewer_host::video::Pictures`, fitted by the clip's Table 291 `/F` (ADR 3553), and read by the
//! host through `quorra_window_picture` and `quorra_window_picture_copy` — over the screen
//! annotation Table 294's `/W` 3 names, "\[t]he rectangle occupied by the screen annotation" (RFC
//! 0015 section 6, round 8; ADR 3578).
//!
//! **An RGBA buffer and not a texture handle.** This window draws its pages on the processor and
//! holds no graphics device, so it has no texture to hand over; a device object crossing the
//! boundary would also be one the host could free, resize or use on another thread while this
//! library still held it, and nothing in C says so. The picture crosses as the pages do: its facts
//! read, a buffer the host owns sized from them, one copy into it.
//!
//! **What says a picture changed.** `QUORRA_REPAINT_PICTURE` in a `QUORRA_OWED_REPAINT` record: a
//! picture came up, a new one is due, it moved with the pages, or it came down. The pixels are not
//! digested as a page's are (`repainted`'s construction): a clip's picture changes up to sixty
//! times a second, and each new one is known new where it is taken, so it is counted instead.
//!
//! **What times it.** The host's own timer, as Qt's: nothing wakes the host from the decoding
//! thread, because a callback into a C caller is what this boundary refuses (ADR 0247), so
//! `quorra_window_interval` answers the clip's next time through
//! [`viewer_host::video::Pictures::next_look_unwoken`] and `quorra_window_tick` looks.

use std::time::{Duration, Instant};

use viewer_core::{Answer, Query};
use viewer_host::native_window::NativeWindow;

use super::Window;
use crate::Status;

/// A picture as the host reads it: the part `/F` shows, where it goes and at what extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PictureInfo {
    /// The part's width in samples.
    pub width: u32,
    /// The part's height in samples.
    pub height: u32,
    /// The pixel layout.
    pub format: pdf_render::RasterFormat,
    /// How many bytes [`Window::picture_copy`] writes.
    pub bytes: usize,
    /// The top-left corner of the bounding box of the quadrilateral the part covers, in device
    /// pixels of the viewport, as bits so that the record compares equal to itself.
    origin: (u32, u32),
    /// That box's width and height, in device pixels.
    pub extent: (u32, u32),
}

impl PictureInfo {
    /// Where the part's top-left corner is shown, in device pixels of the viewport.
    #[must_use]
    pub const fn origin(&self) -> (f32, f32) {
        (f32::from_bits(self.origin.0), f32::from_bits(self.origin.1))
    }
}

/// The picture as the caller was last told of it, by `QUORRA_REPAINT_PICTURE`.
pub(super) type Told = Option<(u64, PictureInfo)>;

impl Window {
    /// [`NativeWindow::look_at_the_pictures`]: the picture that is due taken, every sentence the
    /// player said said, and a repaint owed where the picture changed.
    pub(super) fn show_the_pictures(&mut self) {
        let looked = self.state.pictures.look(Instant::now());
        for said in &looked.said {
            self.say(said.sentence());
        }
        // Table 344's snapshot is drawn while a clip associated with its annotation is in hand
        // (ADR 3889).
        for command in looked.told {
            self.dispatch(command);
        }
        if looked.changed {
            self.picture_count = self.picture_count.wrapping_add(1);
            self.owe_the_picture();
        }
    }

    /// Escape: the clip in hand stops and its picture comes down; answers whether there was one.
    pub(super) fn stop_the_pictures(&mut self) -> bool {
        let stopped = self.state.pictures.stop();
        if stopped {
            self.show_the_pictures();
        }
        stopped
    }

    /// Space: a clip held at its start or paused shows again; answers whether there was one.
    pub(super) fn resume_the_pictures(&mut self) -> bool {
        let resumed = self.state.pictures.resume();
        if resumed {
            self.show_the_pictures();
        }
        resumed
    }

    /// How long until the clip's next look, for [`Window::interval`]; `None` while no clip runs.
    pub(super) fn picture_wait(&self) -> Option<Duration> {
        let now = Instant::now();
        self.state
            .pictures
            .next_look_unwoken(now)
            .map(|at| at.saturating_duration_since(now))
    }

    /// The picture held up, where the clip's document is the one in front and its screen
    /// annotation is on a page now placed, and where `/F` places it — asked at each call, so it
    /// follows a scroll or a zoom.
    fn placed_picture(&self) -> Option<(&pdf_render::Raster, viewer_host::video::Placed)> {
        self.placed_clip().or_else(|| self.placed_poster())
    }

    /// The first of Table 306's posters placed now, where no clip's picture is shown: what the
    /// picture calls hand a C host of a movie that is not playing (ADR 3609). One at a time, as
    /// the calls have one picture to hand; a second poster on the screen at once is not handed.
    fn placed_poster(&self) -> Option<(&pdf_render::Raster, viewer_host::video::Placed)> {
        let focused = self.state.documents.focused();
        self.state
            .pictures
            .posters()
            .filter(|poster| poster.document == focused)
            .find_map(|poster| {
                let Answer::Screen(quad) =
                    self.state.viewer.query(Query::Screen(poster.annotation))
                else {
                    return None;
                };
                Some((poster.raster, poster.placed(quad)?))
            })
    }

    /// The clip's picture held up, placed as [`Self::placed_picture`] says.
    fn placed_clip(&self) -> Option<(&pdf_render::Raster, viewer_host::video::Placed)> {
        let (raster, document, screen) = self.state.pictures.picture()?;
        if document != self.state.documents.focused() {
            return None;
        }
        let Answer::Screen(quad) = self.state.viewer.query(Query::Screen(screen)) else {
            return None;
        };
        let placed = self.state.pictures.placed(quad)?;
        Some((raster, placed))
    }

    /// A press at `at`, device pixels of the viewport, on the controller bar of the clip whose
    /// picture is shown — the bar `quorra_window_controller`'s facts describe, laid over the bottom
    /// [`viewer_host::video::BAR_HEIGHT`] pixels of the picture's rectangle: its button, the first
    /// [`viewer_host::video::BAR_HEIGHT`] pixels from the left, pauses or resumes the clip, and the
    /// press is the bar's (Table 307's `/ShowControls`, Table 291's `/C`; ADR 3610). Answers
    /// whether it was. A host reaches it through `quorra_window_pointer`, as every press.
    pub(super) fn press_the_bar(&mut self, at: (f32, f32)) -> bool {
        let Some((_, document, screen)) = self.state.pictures.picture() else {
            return false;
        };
        if document != self.state.documents.focused() {
            return false;
        }
        let Answer::Screen(quad) = self.state.viewer.query(Query::Screen(screen)) else {
            return false;
        };
        let pressed = self.state.pictures.press_over(quad, at);
        if pressed {
            self.show_the_pictures();
            // The bar's button changed, which a paused clip's next picture will not say.
            self.owe_picture_repaint();
        }
        pressed
    }

    /// The wheel's travel `by` at `at`, device pixels of the viewport, over the clip whose picture
    /// is shown: where it is Table 291's `/F 3` and overflows its rectangle, the travel scrolls the
    /// part shown and is the picture's, not the page's (ADR 3901). Answers whether it was. A host
    /// reaches it through `quorra_window_wheel`.
    pub(super) fn scroll_the_picture(&mut self, at: (f32, f32), by: (f32, f32)) -> bool {
        let Some((_, document, screen)) = self.state.pictures.picture() else {
            return false;
        };
        if document != self.state.documents.focused() {
            return false;
        }
        let Answer::Screen(quad) = self.state.viewer.query(Query::Screen(screen)) else {
            return false;
        };
        let scrolled = self.state.pictures.scroll_over(quad, at, by);
        if scrolled {
            // The part shown moved, which a paused clip's next picture will not say.
            self.owe_picture_repaint();
        }
        scrolled
    }

    /// The controller bar's three facts for the clip whose picture is shown, where it asks for one:
    /// `quorra_window_controller` (Table 307's `/ShowControls`, ADR 3595).
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no picture is shown or the clip asks for no bar.
    pub fn controller(&self) -> Result<viewer_host::video::Controller, Status> {
        self.placed_clip().ok_or(Status::NoAnswer)?;
        self.state
            .pictures
            .controller(Instant::now())
            .ok_or(Status::NoAnswer)
    }

    /// The picture shown over its screen annotation now: `quorra_window_picture`.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no picture is shown: no clip runs, its document is not in
    /// front, or its annotation is on no page now placed.
    pub fn picture(&self) -> Result<PictureInfo, Status> {
        let (raster, placed) = self.placed_picture().ok_or(Status::NoAnswer)?;
        let (width, height) = if placed.is_whole(raster) {
            (raster.width, raster.height)
        } else {
            (
                placed.right.min(raster.width).saturating_sub(placed.left),
                placed.bottom.min(raster.height).saturating_sub(placed.top),
            )
        };
        let ((x, y), extent) = placed.bounding_box();
        // At most 1920 by 1080 samples, the worker's bound, so the product cannot wrap; it is
        // checked all the same rather than argued.
        let bytes = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|samples| samples.checked_mul(4))
            .and_then(|bytes| usize::try_from(bytes).ok())
            .ok_or(Status::NoAnswer)?;
        Ok(PictureInfo {
            width,
            height,
            format: raster.format,
            bytes,
            origin: (x.to_bits(), y.to_bits()),
            extent,
        })
    }

    /// Copies the picture shown into a buffer the caller owns: `quorra_window_picture_copy`. The
    /// part `/F` shows is cut out only where it is not the whole picture.
    ///
    /// # Errors
    ///
    /// [`Status::NoAnswer`] where no picture is shown, and [`Status::BufferTooSmall`] where `into`
    /// is shorter than it, writing nothing.
    pub fn picture_copy(&self, into: &mut [u8]) -> Result<usize, Status> {
        let (raster, placed) = self.placed_picture().ok_or(Status::NoAnswer)?;
        let part;
        let shown = if placed.is_whole(raster) {
            raster
        } else {
            part = placed.part_of(raster);
            &part
        };
        let room = into
            .get_mut(..shown.data.len())
            .ok_or(Status::BufferTooSmall)?;
        room.copy_from_slice(&shown.data);
        Ok(shown.data.len())
    }

    /// The picture as the caller would read it now, with the count of pictures taken, so that a
    /// new picture at the same place is told apart from the last.
    pub(super) fn picture_now(&self) -> Told {
        self.picture().ok().map(|info| (self.picture_count, info))
    }

    /// Owes a repaint saying only that the picture changed, where it did since the caller was
    /// last told: a new picture is a repaint of the picture alone, and the pages are not digested
    /// for it.
    fn owe_the_picture(&mut self) {
        let now = self.picture_now();
        if self.shown.painted.picture_told(now) {
            self.owe_picture_repaint();
        }
    }
}
