//! A clip's pictures in this window, over the screen annotation Table 294's `/W` 3 names — "\[t]he
//! rectangle occupied by the screen annotation" (ISO 32000-2 §13.2.6.1) — through
//! `viewer_host::video::VideoPlayer` (RFC 0015 section 6, rounds 5 and 6; ADRs 3538, 3545).
//!
//! **Where a picture is converted.** On the graphics device where the window draws on one: the
//! planes go to `viewer_device`'s window, which converts them into a texture it keeps and draws as
//! a layer under the chrome at every present, so nothing is read back. On a window the processor
//! draws, the oracle's own conversion makes a raster, drawn as the lowest of the overlays.
//!
//! **When.** `about_to_wait` asks for the picture that is due before the loop rests, and rests no
//! longer than the next picture's time; the decoding thread wakes the loop at each picture it
//! queues. Nothing here waits for the decoder, so a slow frame drops a picture rather than delays
//! the window.

use std::sync::Arc;
use std::time::Instant;

use pdf_render::{BlendMode, Command as DrawCommand, ImageSource, SampleAlpha, Size, Transform};
use viewer_core::{Answer, Query};
use viewer_host::video::{Seen, Shown, VideoPlayer};

use crate::app::App;

impl App {
    /// Shows `shown`'s pictures in place of the clip in hand, following `sound` — the clip's sound
    /// as the sound player was handed it — making the player, its thread and its worker started at
    /// this first clip, never at the launch, with the loop's waker.
    pub(crate) fn start_the_pictures(
        &mut self,
        shown: Shown,
        sound: Option<viewer_host::sound::Listening>,
    ) {
        let waker = self.waker.clone();
        let video = self.video.get_or_insert_with(|| {
            VideoPlayer::new(Box::new(move || {
                if let Some(waker) = &waker {
                    let _ = waker.send_event(());
                }
            }))
        });
        video.play_beside(shown, sound);
        self.show_the_pictures();
    }

    /// Takes the picture that is due and puts it up, says what the player said, and answers when
    /// the loop should look again. Never waits.
    pub(crate) fn show_the_pictures(&mut self) -> Option<Instant> {
        let now = Instant::now();
        let video = self.video.as_mut()?;
        let frame = video.due(now);
        let mut said = video.seen();
        let holds = video.holds_a_picture();
        // A clip in a window of its own is drawn there by the processor (ADR 3587).
        let windowed = video.window().is_some();
        let mut changed = false;
        if let Some(frame) = frame {
            let placed = self.picture_placed();
            let device = if windowed { None } else { self.device_window() };
            let shown = match device {
                Some(window) => frame
                    .planes()
                    .map_err(|refused| refused.to_string())
                    .and_then(|planes| {
                        window
                            .show_video(&planes, &frame.conversion, placed)
                            .map_err(|refused| refused.to_string())
                    })
                    .map(drop),
                None => viewer_host::video::on_the_processor(&frame)
                    .map(|raster| self.picture = Some(raster))
                    .map_err(|refused| refused.to_string()),
            };
            changed = true;
            if let Err(refused) = shown {
                if let Some(video) = self.video.as_mut() {
                    video.stop();
                    said.extend(video.seen());
                }
                said.push(Seen::Refused(format!(
                    "a clip's picture was not shown, because {refused}"
                )));
            }
        }
        let holds = holds
            && self
                .video
                .as_ref()
                .is_some_and(VideoPlayer::holds_a_picture);
        if !holds {
            changed |= self.take_the_picture_down();
        }
        for seen in &said {
            println!("note: {}", seen.sentence());
        }
        if changed && windowed {
            self.present_the_media_window();
        } else if changed {
            self.redraw();
        }
        self.video.as_ref()?.next_due(now)
    }

    /// Takes the picture off the window, and answers whether one was there.
    fn take_the_picture_down(&mut self) -> bool {
        let mut was = self.picture.take().is_some();
        if let Some(window) = self.device_window()
            && window.shows_video()
        {
            window.clear_video();
            was = true;
        }
        was
    }

    /// Escape: the clip in hand stops and its picture comes down; answers whether there was one.
    pub(crate) fn stop_the_pictures(&mut self) -> bool {
        let Some(video) = self.video.as_mut().filter(|video| video.in_hand()) else {
            return false;
        };
        video.stop();
        self.show_the_pictures();
        true
    }

    /// Space: a clip held at its start (Table 291's `/A` false) or paused shows again; answers
    /// whether there was one.
    pub(crate) fn resume_the_pictures(&mut self) -> bool {
        let resumed = self.video.as_mut().is_some_and(VideoPlayer::resume);
        if resumed {
            self.show_the_pictures();
        }
        resumed
    }

    /// Where the picture goes: the screen annotation's quadrilateral in window pixels — the core's
    /// answer in the viewport's, moved past the panel — where the clip's document is the one in
    /// front and the annotation is on a page now placed; `None` otherwise.
    pub(crate) fn picture_quad(&self) -> Option<[f32; 8]> {
        let (document, screen) = self.video.as_ref()?.screen()?;
        if document != self.documents.focused() {
            return None;
        }
        let Answer::Screen(mut quad) = self.viewer.query(Query::Screen(screen)) else {
            return None;
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "a panel width in pixels, which is hundreds"
        )]
        let edge = self.inset() as f32;
        for x in quad.iter_mut().step_by(2) {
            *x += edge;
        }
        Some(quad)
    }

    /// Moves the device's picture with the pages, at each frame: the view may have scrolled or
    /// zoomed since the picture was put up, and a paused clip has no next picture to move it.
    pub(crate) fn place_the_picture(&mut self) {
        let placed = self.picture_placed();
        if let Some(window) = self.device_window()
            && window.shows_video()
        {
            window.place_video(placed);
        }
    }

    /// The picture fitted to its screen annotation by Table 291's `/F` (ADR 3553): the part of it
    /// shown and the quadrilateral that part covers, in window pixels.
    pub(crate) fn picture_placed(&self) -> Option<viewer_host::video::Placed> {
        let quad = self.picture_quad()?;
        self.video.as_ref()?.placed(quad)
    }

    /// The processor's picture as the lowest overlay, over its screen annotation — the overlays'
    /// pixels are the window's, `width` by `height`.
    pub(crate) fn picture_list(&self, width: u32, height: u32) -> Option<pdf_render::DisplayList> {
        let raster = self.picture.as_ref()?;
        let placed = self.picture_placed()?;
        let part = placed.part_of(raster);
        #[expect(
            clippy::cast_precision_loss,
            reason = "window dimensions are far below f32's exact integer range"
        )]
        let mut list = pdf_render::DisplayList::new(Size::new(width as f32, height as f32));
        list.push(DrawCommand::Image {
            image: ImageSource::Decoded(pdf_render::Image {
                width: part.width,
                height: part.height,
                data: Arc::from(part.data.as_slice()),
                // A picture is drawn at the rectangle's size, larger or smaller than its own;
                // smoothing it is this window's choice, as the device's sampler makes it.
                interpolate: true,
                sample_alpha: SampleAlpha::Shape,
            }),
            transform: unit_square_onto(placed.quad),
            alpha: 1.0,
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        });
        Some(list)
    }
}

impl App {
    /// The controller bar over the bottom of the clip's rectangle, where the clip in hand asks for
    /// one — Table 307's `/ShowControls` (ADR 3595) — drawn by the processor and laid over the
    /// picture on either kind of window, the overlays' pixels being the window's.
    pub(crate) fn controller_list(
        &self,
        width: u32,
        height: u32,
    ) -> Option<pdf_render::DisplayList> {
        let quad = self.picture_quad()?;
        let (bar, covered) = self.video.as_ref()?.controller_bar(quad, Instant::now())?;
        #[expect(
            clippy::cast_precision_loss,
            reason = "window dimensions are far below f32's exact integer range"
        )]
        let mut list = pdf_render::DisplayList::new(Size::new(width as f32, height as f32));
        list.push(DrawCommand::Image {
            image: ImageSource::Decoded(pdf_render::Image {
                width: bar.width,
                height: bar.height,
                data: Arc::from(bar.data.as_slice()),
                // Drawn at its own size, pixel for pixel.
                interpolate: false,
                sample_alpha: SampleAlpha::Shape,
            }),
            transform: unit_square_onto(covered),
            alpha: 1.0,
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
        });
        Some(list)
    }
}

/// The image's unit square onto `quad`, corners clockwise from its top-left in pixels with y
/// downwards: §8.9.5's image space puts an image's first row at the unit square's `y = 1`, so that
/// edge goes to the quadrilateral's first side and the square's origin to its last corner.
fn unit_square_onto(quad: [f32; 8]) -> Transform {
    let [x0, y0, x1, y1, _, _, x3, y3] = quad;
    Transform {
        a: x1 - x0,
        b: y1 - y0,
        c: x0 - x3,
        d: y0 - y3,
        e: x3,
        f: y3,
    }
}

#[cfg(test)]
mod tests {
    use pdf_render::{Raster, RasterFormat};

    use super::unit_square_onto;

    /// A two-by-two picture over the window's upper-left four pixels: its first row is the top of
    /// the rectangle and its first column the left (§8.9.5's image space), so each window pixel
    /// shows the sample at the same row and column.
    #[test]
    fn a_picture_is_drawn_upright_over_its_rectangle() {
        let samples: [[u8; 4]; 4] = [
            [255, 0, 0, 255],
            [0, 255, 0, 255],
            [0, 0, 255, 255],
            [255, 255, 0, 255],
        ];
        let mut list = pdf_render::DisplayList::new(pdf_render::Size::new(4.0, 4.0));
        list.push(pdf_render::Command::Image {
            image: pdf_render::ImageSource::Decoded(pdf_render::Image {
                width: 2,
                height: 2,
                data: samples.concat().into(),
                interpolate: false,
                sample_alpha: pdf_render::SampleAlpha::Shape,
            }),
            transform: unit_square_onto([0.0, 0.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0]),
            alpha: 1.0,
            clip: None,
            mask: None,
            blend: pdf_render::BlendMode::Normal,
        });
        let page = Raster {
            width: 4,
            height: 4,
            format: RasterFormat::Rgba8,
            data: [255, 255, 255, 255].repeat(16),
        };
        let composed = viewer_ui::software::compose(&page, &[&list]).expect("compose");
        let at = |x: usize, y: usize| &composed.data[(y * 4 + x) * 4..(y * 4 + x) * 4 + 3];
        assert_eq!(
            at(0, 0),
            &samples[0][..3],
            "the first sample at the top left"
        );
        assert_eq!(at(1, 0), &samples[1][..3], "the second along the top");
        assert_eq!(at(0, 1), &samples[2][..3], "the second row below the first");
        assert_eq!(at(1, 1), &samples[3][..3]);
        assert_eq!(at(3, 3), &[255, 255, 255], "the page outside the rectangle");
    }
}
