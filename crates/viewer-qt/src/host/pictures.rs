//! A clip's pictures in the Qt window: each picture converted on the processor by
//! `viewer_host::video::Pictures`, fitted by the clip's Table 291 `/F` (ADR 3553), and handed to
//! C++ through the frame sink the pages already cross — one frame more, after the pages, over the
//! screen annotation Table 294's `/W` 3 names — "\[t]he rectangle occupied by the screen
//! annotation" (RFC 0015 section 6, round 7; ADR 3562).
//!
//! **Why the frame sink, and no bridge function of its own.** The sink is the one hand-written
//! `unsafe extern "C++"` block this crate has (`tests/unsafe_position.rs`, A979), and it already
//! copies a checked RGBA raster into an image C++ owns and paints at a rectangle. A picture is such
//! a raster: appended last, it is painted over the pages, and placed afresh from the screen
//! annotation's quadrilateral at each paint, so it moves with them when they scroll or zoom.
//!
//! **What wakes the window.** Nothing from the decoding thread: a wake into `QApplication::exec`
//! would cost this crate a second hand-written `unsafe` token, `Host::plays_sounds`'s reason. The
//! timer `Host::drawing_wait` arms looks at the clip's next time instead
//! ([`viewer_host::video::Pictures::next_look_unwoken`]): `viewer-gtk`'s timeout's look, with the
//! decoding thread's queue polled in place of its wake.

use std::time::{Duration, Instant};

use viewer_core::{Answer, Query};

use super::Host;

/// A picture as the frame sink takes it: its samples, where its top-left sits in the viewport and
/// the extent it is shown at, both in device pixels.
pub(super) struct Shown<'a> {
    /// The samples, the whole picture or the part `/F` shows.
    pub(super) raster: std::borrow::Cow<'a, pdf_render::Raster>,
    /// The top-left corner of the bounding box of the quadrilateral the part covers.
    pub(super) origin: (f32, f32),
    /// That box's width and height.
    pub(super) extent: (u32, u32),
}

impl Host {
    /// [`viewer_host::native_window::NativeWindow::look_at_the_pictures`]: the picture that is due
    /// taken, every sentence the player said said, and the page area painted again where the
    /// picture changed. The next look is the timer's, at [`Self::picture_wait`].
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
            self.update.frame = true;
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

    /// How long until the clip's next look, for the timer `Self::drawing_wait` arms; `None` while
    /// no clip is running.
    pub(super) fn picture_wait(&self) -> Option<Duration> {
        let now = Instant::now();
        self.state
            .pictures
            .next_look_unwoken(now)
            .map(|at| at.saturating_duration_since(now))
    }

    /// The controller bar over the bottom of the clip's rectangle, where the clip in hand asks for
    /// one — Table 307's `/ShowControls` (ADR 3595) — placed as the picture is, or `None`.
    pub(super) fn shown_bar(&self) -> Option<Shown<'static>> {
        let (_, document, screen) = self.state.pictures.picture()?;
        if document != self.state.documents.focused() {
            return None;
        }
        let Answer::Screen(quad) = self.state.viewer.query(Query::Screen(screen)) else {
            return None;
        };
        let (raster, covered) = self.state.pictures.controller_bar(quad, Instant::now())?;
        let (origin, extent) = viewer_host::video::Placed {
            left: 0,
            top: 0,
            right: raster.width,
            bottom: raster.height,
            quad: covered,
            quarter_turns: 0,
        }
        .bounding_box();
        Some(Shown {
            raster: std::borrow::Cow::Owned(raster),
            origin,
            extent,
        })
    }

    /// A press at `at`, device pixels of the viewport, on the controller bar of the clip whose
    /// picture is shown: the bar's button pauses or resumes it, and the press is the bar's (Table
    /// 307's `/ShowControls`, Table 291's `/C`; ADR 3610). Answers whether it was.
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
            // A paused clip has no next picture to paint its bar again with.
            self.update.frame = true;
        }
        pressed
    }

    /// Table 306's posters retrieved from their movie files, each placed over its movie annotation
    /// — where its document is the one in front and the annotation is on a page now placed — as the
    /// picture is, and asked at each paint for its reason (ADR 3609).
    pub(super) fn shown_posters(&self) -> Vec<Shown<'_>> {
        let focused = self.state.documents.focused();
        self.state
            .pictures
            .posters()
            .filter(|poster| poster.document == focused)
            .filter_map(|poster| {
                let Answer::Screen(quad) =
                    self.state.viewer.query(Query::Screen(poster.annotation))
                else {
                    return None;
                };
                let placed = poster.placed(quad)?;
                let raster = if placed.is_whole(poster.raster) {
                    std::borrow::Cow::Borrowed(poster.raster)
                } else {
                    std::borrow::Cow::Owned(placed.part_of(poster.raster))
                };
                let (origin, extent) = placed.bounding_box();
                Some(Shown {
                    raster,
                    origin,
                    extent,
                })
            })
            .collect()
    }

    /// The picture held up, placed over its screen annotation — where the clip's document is the
    /// one in front and the annotation is on a page now placed — or `None`.
    ///
    /// Asked at each paint, so the picture follows the pages. The part `/F` shows is copied out
    /// only where it is not the whole picture: meet, fill and the default show all of it.
    pub(super) fn shown_picture(&self) -> Option<Shown<'_>> {
        let (raster, document, screen) = self.state.pictures.picture()?;
        if document != self.state.documents.focused() {
            return None;
        }
        let Answer::Screen(quad) = self.state.viewer.query(Query::Screen(screen)) else {
            return None;
        };
        let placed = self.state.pictures.placed(quad)?;
        let whole = placed.is_whole(raster);
        let raster = if whole {
            std::borrow::Cow::Borrowed(raster)
        } else {
            std::borrow::Cow::Owned(placed.part_of(raster))
        };
        let (origin, extent) = placed.bounding_box();
        Some(Shown {
            raster,
            origin,
            extent,
        })
    }
}

/// The bounding box of `quad`, in device pixels of the viewport
/// ([`viewer_host::video::Placed::bounding_box`]).
#[cfg(test)]
pub(super) fn bounding_box(quad: [f32; 8]) -> ((f32, f32), (u32, u32)) {
    viewer_host::video::Placed {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
        quad,
        quarter_turns: 0,
    }
    .bounding_box()
}
