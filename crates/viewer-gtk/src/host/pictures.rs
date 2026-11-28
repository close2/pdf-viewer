//! A clip's pictures in the GTK window: each picture converted on the processor by
//! `viewer_host::video::Pictures`, held up as a texture in a widget over the screen annotation
//! Table 294's `/W` 3 names — "\[t]he rectangle occupied by the screen annotation" — and looked at
//! again on a glib timeout at the next picture's time (RFC 0015 section 6, round 6; ADR 3545).
//!
//! **Why the processor.** This window's pages reach the screen as textures GTK draws, read back
//! where the graphics device drew them; a picture converted on the device would be read back the
//! same way, so the conversion is the oracle's own, which costs no read-back (ADR 3529).
//!
//! **What wakes the window.** The decoding thread's wake, at each picture it queues, invokes GTK's
//! main context; a one-shot timeout at [`viewer_host::video::Pictures::next_look`] shows a picture
//! queued early at its time, and ends a pass once its last picture has been up for its interval.
//! Only the newest timeout acts, so a wake and a timeout never multiply the looks.

use std::cell::RefCell;
use std::rc::Weak;
use std::time::Instant;

use gtk4::glib;
use gtk4::prelude::*;
use viewer_core::{Answer, Query};
use viewer_host::native_window::NativeWindow;

use super::Host;

thread_local! {
    /// The windows that have shown a clip, on GTK's own thread: `sounds`' `HEARING` for the
    /// decoding thread's wake, for its reason.
    static SHOWING: RefCell<Vec<Weak<RefCell<Host>>>> = const { RefCell::new(Vec::new()) };
}

/// The wake for the video player `me` makes at its first clip: GTK's main context, invoked from
/// the decoding thread.
pub(super) fn wake(me: Weak<RefCell<Host>>) -> viewer_host::sound::Wake {
    SHOWING.with(|showing| {
        let mut showing = showing.borrow_mut();
        showing.retain(|window| window.strong_count() > 0);
        showing.push(me);
    });
    Box::new(|| glib::MainContext::default().invoke(seen))
}

/// What the wake runs on GTK's thread: one look at each window's pictures.
fn seen() {
    let windows: Vec<_> =
        SHOWING.with(|showing| showing.borrow().iter().filter_map(Weak::upgrade).collect());
    for window in windows {
        look_or_later(&window);
    }
}

/// A look at `window`'s pictures, or — where the window is inside a turn of its own — the same a
/// moment later, so that a picture's look is never lost to a borrow.
fn look_or_later(window: &std::rc::Rc<RefCell<Host>>) {
    if let Ok(mut host) = window.try_borrow_mut() {
        host.look_at_the_pictures();
        return;
    }
    let again = std::rc::Rc::downgrade(window);
    glib::timeout_add_local_once(RETRY, move || {
        if let Some(window) = again.upgrade() {
            look_or_later(&window);
        }
    });
}

/// How soon a look the window was too busy for is tried again: well inside a picture's interval.
const RETRY: std::time::Duration = std::time::Duration::from_millis(5);

/// The widget the pictures are held up in, and the look the newest timeout belongs to.
#[derive(Debug, Default)]
pub(super) struct HeldUp {
    /// Made at the first picture, in the page layer.
    widget: Option<gtk4::Picture>,
    /// The controller bar over the rectangle's bottom, made at the first clip that asks for one
    /// (Table 307's `/ShowControls`, ADR 3595).
    bar: Option<gtk4::Picture>,
    /// Table 306's posters retrieved from their movie files, one widget each, with the movie
    /// annotation it shows and the part of the picture its texture was made of (ADR 3609).
    posters: Vec<HeldPoster>,
    /// Counts the timeouts armed; one whose number is not this one's is stale.
    armed: u64,
    /// The window of its own a clip that Table 294's `/W` 0 or 1 asks for is shown in, while one
    /// is in hand (ADR 3587).
    pub(super) media_window: Option<super::media_window::MediaWindow>,
}

/// One poster's widget, and what its texture was made from, so that a placement that moves it
/// makes no new texture.
#[derive(Debug)]
struct HeldPoster {
    document: viewer_core::DocumentId,
    annotation: pdf_syntax::ObjectId,
    part: [u32; 4],
    widget: gtk4::Picture,
}

impl HeldUp {
    /// The posters held up and shown, for a test.
    #[cfg(test)]
    pub(super) fn posters_shown(&self) -> usize {
        self.posters
            .iter()
            .filter(|poster| poster.widget.is_visible())
            .count()
    }

    /// The widget, for a test that reads the picture it holds.
    #[cfg(test)]
    pub(super) fn widget(&self) -> Option<&gtk4::Picture> {
        self.widget.as_ref()
    }
}

impl Host {
    /// [`NativeWindow::look_at_the_pictures`]: the picture that is due taken and held up, every
    /// sentence the player said said, and the next look armed.
    pub(super) fn show_the_pictures(&mut self) {
        let now = Instant::now();
        let mut looked = self.state.pictures.look(now);
        for said in &looked.said {
            self.say(said.sentence());
        }
        // Table 344's snapshot is drawn while a clip associated with its annotation is in hand
        // (ADR 3889).
        for command in std::mem::take(&mut looked.told) {
            self.dispatch(command);
        }
        self.keep_the_media_window();
        // A sentence is a change of state — paused, showing again — which the controller bar's
        // button shows, so the bar is drawn again with it.
        if looked.changed || !looked.said.is_empty() {
            self.hold_up_the_picture();
        }
        self.arm_the_next_look(now);
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

    /// Puts the picture held up over its screen annotation — where the clip's document is the one
    /// in front and the annotation is on a page now placed — and takes it down otherwise. Called at
    /// each new picture and each time the pages are placed again, so the picture moves with them.
    pub(super) fn hold_up_the_picture(&mut self) {
        if self.shown.media_window.is_some() {
            self.hold_up_in_the_media_window();
        }
        // Under the playing picture and its bar, which are held up after them.
        self.hold_up_the_posters();
        let screen = self
            .state
            .pictures
            .picture()
            .filter(|(_, document, _)| *document == self.state.documents.focused())
            .and_then(
                |(raster, _, screen)| match self.state.viewer.query(Query::Screen(screen)) {
                    Answer::Screen(quad) => Some((raster, quad)),
                    _ => None,
                },
            );
        // The bar goes over the picture, so it is held up after it.
        let screen_quad = screen.map(|(_, quad)| quad);
        let placed = screen
            // Fitted by the clip's Table 291 `/F` (ADR 3553): the part of the picture shown, over
            // the part of the rectangle it covers.
            .and_then(|(raster, quad)| {
                let placed = self.state.pictures.placed(quad)?;
                Some((crate::page::texture(&placed.part_of(raster)), placed.quad))
            });
        let Some((texture, quad)) = placed else {
            if let Some(widget) = &self.shown.widget {
                widget.set_paintable(gtk4::gdk::Paintable::NONE);
                widget.set_visible(false);
            }
            self.hold_up_the_bar(None);
            return;
        };
        let texture = match texture {
            Ok(texture) => texture,
            Err(refused) => {
                self.say(&format!("a clip's picture was not shown: {refused}"));
                return;
            }
        };
        let fixed = self.ui.fixed.clone();
        let widget = self.shown.widget.get_or_insert_with(|| {
            let widget = gtk4::Picture::new();
            widget.set_content_fit(gtk4::ContentFit::Fill);
            widget.set_can_shrink(false);
            widget.set_can_target(false);
            fixed.put(&widget, 0.0, 0.0);
            widget
        });
        // Over every page picture, which the placement adds to the same layer as it needs them.
        if fixed.last_child().as_ref() != Some(widget.upcast_ref()) {
            widget.insert_after(&fixed, fixed.last_child().as_ref());
        }
        let (width, height) = (texture.width(), texture.height());
        widget.set_paintable(Some(&texture));
        widget.set_size_request(width, height);
        widget.set_visible(true);
        fixed.set_child_transform(widget, Some(&placement(quad, (width, height), self.scale)));
        self.hold_up_the_bar(screen_quad);
    }

    /// Puts the controller bar over the bottom of `screen`, the clip's rectangle, where the clip in
    /// hand asks for one, and takes it down otherwise (ADR 3595). Drawn again with each picture, so
    /// its time and track move as the clip plays.
    fn hold_up_the_bar(&mut self, screen: Option<[f32; 8]>) {
        let bar = screen.and_then(|quad| self.state.pictures.controller_bar(quad, Instant::now()));
        let Some((raster, quad)) = bar else {
            if let Some(widget) = &self.shown.bar {
                widget.set_visible(false);
            }
            return;
        };
        let texture = match crate::page::texture(&raster) {
            Ok(texture) => texture,
            Err(refused) => {
                self.say(&format!("a clip's controller bar was not shown: {refused}"));
                return;
            }
        };
        let fixed = self.ui.fixed.clone();
        let widget = self.shown.bar.get_or_insert_with(|| {
            let widget = gtk4::Picture::new();
            widget.set_content_fit(gtk4::ContentFit::Fill);
            widget.set_can_shrink(false);
            widget.set_can_target(false);
            fixed.put(&widget, 0.0, 0.0);
            widget
        });
        if fixed.last_child().as_ref() != Some(widget.upcast_ref()) {
            widget.insert_after(&fixed, fixed.last_child().as_ref());
        }
        let (width, height) = (texture.width(), texture.height());
        widget.set_paintable(Some(&texture));
        widget.set_size_request(width, height);
        widget.set_visible(true);
        fixed.set_child_transform(widget, Some(&placement(quad, (width, height), self.scale)));
    }

    /// A press at `at`, device pixels of the viewport, on the controller bar of the clip whose
    /// picture is held up: the bar's button pauses or resumes it, and the press is the bar's
    /// (Table 307's `/ShowControls`, Table 291's `/C`; ADR 3610). Answers whether it was.
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
            // A paused clip has no next picture to draw its bar again with.
            self.hold_up_the_picture();
        }
        pressed
    }

    /// Puts each poster over its movie annotation, where its document is the one in front and the
    /// annotation is on a page now placed, and takes the rest down (ADR 3609). A poster's texture
    /// is made when it first goes up and again only where the part of it shown changes.
    fn hold_up_the_posters(&mut self) {
        let focused = self.state.documents.focused();
        let mut wanted = Vec::new();
        let mut refusals = Vec::new();
        for poster in self.state.pictures.posters() {
            if poster.document != focused {
                continue;
            }
            let Answer::Screen(quad) = self.state.viewer.query(Query::Screen(poster.annotation))
            else {
                continue;
            };
            let Some(placed) = poster.placed(quad) else {
                continue;
            };
            let part = [placed.left, placed.top, placed.right, placed.bottom];
            let made = self.shown.posters.iter().any(|held| {
                held.document == poster.document
                    && held.annotation == poster.annotation
                    && held.part == part
            });
            let texture = if made {
                None
            } else {
                match crate::page::texture(&placed.part_of(poster.raster)) {
                    Ok(texture) => Some(texture),
                    Err(refused) => {
                        refusals.push(format!("a movie's poster was not shown: {refused}"));
                        continue;
                    }
                }
            };
            wanted.push((
                poster.document,
                poster.annotation,
                part,
                placed.quad,
                texture,
            ));
        }
        for refused in &refusals {
            self.say(refused);
        }
        let fixed = self.ui.fixed.clone();
        let mut kept = Vec::new();
        for (document, annotation, part, quad, texture) in wanted {
            let at = self
                .shown
                .posters
                .iter()
                .position(|held| held.document == document && held.annotation == annotation);
            let mut held = if let Some(at) = at {
                self.shown.posters.swap_remove(at)
            } else {
                let widget = gtk4::Picture::new();
                widget.set_content_fit(gtk4::ContentFit::Fill);
                widget.set_can_shrink(false);
                widget.set_can_target(false);
                fixed.put(&widget, 0.0, 0.0);
                HeldPoster {
                    document,
                    annotation,
                    part,
                    widget,
                }
            };
            if let Some(texture) = texture {
                held.widget.set_paintable(Some(&texture));
                held.widget
                    .set_size_request(texture.width(), texture.height());
                held.part = part;
            }
            if fixed.last_child().as_ref() != Some(held.widget.upcast_ref()) {
                held.widget
                    .insert_after(&fixed, fixed.last_child().as_ref());
            }
            let size = (held.widget.width_request(), held.widget.height_request());
            held.widget.set_visible(true);
            fixed.set_child_transform(&held.widget, Some(&placement(quad, size, self.scale)));
            kept.push(held);
        }
        // A poster whose movie plays, whose page left the screen or whose document closed.
        for gone in self.shown.posters.drain(..) {
            fixed.remove(&gone.widget);
        }
        self.shown.posters = kept;
    }

    /// Arms a one-shot look at the player's next time, where it has one; the one armed before is
    /// left to fire and do nothing.
    fn arm_the_next_look(&mut self, now: Instant) {
        self.shown.armed = self.shown.armed.wrapping_add(1);
        let Some(at) = self.state.pictures.next_look(now) else {
            return;
        };
        let armed = self.shown.armed;
        let me = self.me.clone();
        glib::timeout_add_local_once(at.saturating_duration_since(now), move || {
            let Some(window) = me.upgrade() else {
                return;
            };
            let current = window
                .try_borrow()
                .map_or(true, |host| host.shown.armed == armed);
            if current {
                look_or_later(&window);
            }
        });
    }
}

/// The texture of `size` laid over the bounding box of `quad` — the part of the rectangle the fitted
/// picture covers — device pixels of the viewport, in the page layer's logical units at `scale`:
/// [`super::stand_in::place`]'s construction. A page turned by a quarter keeps its rectangle's box,
/// and the picture fills it upright.
fn placement(quad: [f32; 8], size: (i32, i32), scale: f64) -> gtk4::gsk::Transform {
    let xs = [quad[0], quad[2], quad[4], quad[6]];
    let ys = [quad[1], quad[3], quad[5], quad[7]];
    let left = xs.iter().copied().fold(f32::INFINITY, f32::min);
    let right = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let top = ys.iter().copied().fold(f32::INFINITY, f32::min);
    let bottom = ys.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "viewport coordinates and texture extents are far inside f32's range"
    )]
    let (at, fill) = (
        gtk4::graphene::Point::new(
            (f64::from(left) / scale) as f32,
            (f64::from(top) / scale) as f32,
        ),
        (
            (f64::from(right - left) / f64::from(size.0.max(1)) / scale) as f32,
            (f64::from(bottom - top) / f64::from(size.1.max(1)) / scale) as f32,
        ),
    );
    gtk4::gsk::Transform::new()
        .translate(&at)
        .scale(fill.0, fill.1)
}
