//! ISO 32000-2 §12.4.4.1's clock: when a host ticks, and what a frame looks like while it does.
//!
//! # What the clause asks of a clock
//!
//! §12.4.4.1 states two durations and one relationship between them. The page's is
//!
//! > The Dur entry in the page object specifies the page's display duration (also called its
//! > advance timing): the maximum length of time, in seconds, that the page shall be displayed
//! > before the presentation automatically advances to the next page.
//!
//! with the silence spelled out beside it —
//!
//! > If no Dur entry is specified in the page object, the page shall not advance automatically.
//!
//! — and NOTE 1's other half, "[t]he user can advance the page manually before the specified time
//! has expired", which is a key press rather than a clock. Table 164 states the transition's
//! duration as `/D`, "[t]he duration of the transition effect, in seconds. Default value: 1", and
//! §12.4.4.1's own EXAMPLE fixes how the two compose:
//!
//! > The following example shows the presentation parameters for a page to be displayed for 5
//! > seconds. Before the page is displayed, there is a 3.5-second transition in which two vertical
//! > lines sweep outward from the centre to the edges of the page.
//!
//! **Before the page is displayed.** So a transition is not part of the display duration it
//! introduces, and a clock that ran through one would spend the arriving page's `/Dur` on the
//! effect that brings it in. That is the one arithmetic rule in this module, and it is the
//! standard's rather than a convenience.
//!
//! # Why the decision is here and not in a host
//!
//! `viewer-core` has no clock — that is rule 3 of `doc/ui-boundary.md`, and it is what makes
//! [`viewer_core::Command::Tick`] exist at all. So *something* outside it has to decide how often
//! to look at a wall clock, what a tick carries, when a transition begins and when it has run out.
//! None of those four is a toolkit's question: `glib::timeout_add_local`, `QTimer` and winit's
//! `ControlFlow::WaitUntil` differ in every letter and agree about every one of the four. The
//! third copy is where two hosts stop agreeing, which is the test [`crate`] states for what belongs
//! in it — [`crate::arrangement`] and [`crate::presentation`] are the precedents.
//!
//! **And it needed no new message on the boundary.** [`viewer_core::Command::Tick`] has carried
//! milliseconds since ADR 0135 and [`viewer_core::Event::Transition`] has named a transition since
//! ADR 0230; what was missing was not a channel but two hosts driving the one that existed.
//!
//! # What is chosen rather than derived
//!
//! - **[`Clock::RESTING`] is a tenth of a second.** The clause counts in seconds and
//!   [`viewer_core::Command::Tick`] carries the milliseconds that really passed rather than an
//!   assumed step, so this decides only *when an advance is noticed* — a tenth of a second against
//!   a duration a document states in whole ones. Not every frame: between transitions a slide show
//!   has nothing to draw, and a window that woke sixty times a second to add nothing would be
//!   spending a processor on a still page, which `CLAUDE.md`'s principle 2 forbids.
//! - **[`Clock::ANIMATING`] is a sixtieth**, and it is a floor rather than a target. A host with a
//!   better clock than a timer — winit's window has the display's own cadence — uses that instead
//!   and never asks for this one.
//! - **Progress through a transition is linear in time**, which is `viewer_core::transition`'s
//!   choice already: Table 164 states a duration and no curve.
//! - **A clock exists only while a presentation is running.** There is no paused state and no
//!   "off" variant: a host holds an [`Option<Clock>`] and drops it when the mode ends, so a
//!   program that is not presenting has no timer armed at all rather than one that wakes to
//!   discover it has nothing to do.

use std::time::{Duration, Instant};

use pdf_model::navigation::Transition;
use pdf_render::{DisplayList, DisplayListError, Image, Rect, TargetSpec, Transform};

/// One transition, mid-flight: what it is, when it began, and the two pages it is between.
#[derive(Debug)]
struct Playing {
    /// Table 164's style, duration and direction, as the page arrived at states them.
    transition: Transition,
    /// When the first frame of it was drawn.
    began: Instant,
    /// The page being left, rasterised for the whole viewport.
    outgoing: Image,
    /// The page being moved to, at the same size.
    incoming: Image,
}

/// §12.4.4's presentation clock, while one is running.
///
/// One value per window rather than per document: §12.4.4.1's `/Dur` is a property of the page
/// being *shown*, and a window shows one document at a time.
#[derive(Debug)]
pub struct Clock {
    /// When the clock was last read, so that a tick carries the milliseconds that really passed.
    ticked: Instant,
    /// The transition being drawn, where one is in flight.
    playing: Option<Playing>,
}

impl Clock {
    /// How long a host waits between ticks while nothing is being animated.
    pub const RESTING: Duration = Duration::from_millis(100);

    /// How long it waits between frames of a transition, where it has no better clock to use.
    pub const ANIMATING: Duration = Duration::from_millis(16);

    /// A clock for a presentation that has just begun.
    #[must_use]
    pub fn started(now: Instant) -> Self {
        Self {
            ticked: now,
            playing: None,
        }
    }

    /// How long the host should wait before asking this clock again.
    #[must_use]
    pub fn interval(&self) -> Duration {
        if self.playing.is_some() {
            Self::ANIMATING
        } else {
            Self::RESTING
        }
    }

    /// Whether a transition is being drawn, so that a host knows the window is not at rest.
    #[must_use]
    pub const fn animating(&self) -> bool {
        self.playing.is_some()
    }

    /// The milliseconds [`viewer_core::Command::Tick`] should carry, or `None` for nothing to say.
    ///
    /// **`None` while a transition is being drawn**, and the clock restarts rather than pausing:
    /// §12.4.4.1's EXAMPLE puts the transition *before* the page is displayed, so the page's own
    /// `/Dur` begins when the effect that introduces it ends. `None` also for a call so soon after
    /// the last one that no whole millisecond has passed, because a tick of zero would tell
    /// `viewer-core` that time did not move.
    pub fn tick(&mut self, now: Instant) -> Option<u32> {
        if self.playing.is_some() {
            self.ticked = now;
            return None;
        }
        let elapsed = now.saturating_duration_since(self.ticked);
        self.ticked = now;
        let millis = u32::try_from(elapsed.as_millis()).unwrap_or(u32::MAX);
        (millis > 0).then_some(millis)
    }

    /// Whether `viewer_core::transition` shapes frames for this style at all.
    ///
    /// Five of Table 164's twelve are shaped by nothing: four are reported by name, each because
    /// the table describes it with a quantity it does not state, and the fifth is `R`, which is
    /// the cut the table defines and so has nothing to report — `viewer_core::transition` owns
    /// that list and the core has already said which by the time an
    /// [`viewer_core::Event::Transition`] arrives, so a host asks this instead of repeating it.
    #[must_use]
    pub fn shapes(transition: &Transition, viewport: Rect) -> bool {
        viewer_core::transition::frame(transition, viewport, 0.0).is_some()
    }

    /// Starts Table 164's transition between two pages already rasterised for the viewport.
    ///
    /// The two images are taken **once per transition and never per frame**: a
    /// [`pdf_render::Image`] holds its samples behind an `Arc`, and a frame is two draws of them.
    /// A host that re-rasterised each frame would pay a page's interpretation sixty times a second
    /// for the length of the effect.
    pub fn begin(
        &mut self,
        transition: Transition,
        outgoing: Image,
        incoming: Image,
        now: Instant,
    ) {
        self.playing = Some(Playing {
            transition,
            began: now,
            outgoing,
            incoming,
        });
    }

    /// The frame of the transition in flight, or `None` when there is none to draw.
    ///
    /// `None` covers three states a host treats alike — nothing is playing, the effect has run for
    /// its whole `/D`, or the style is one this program does not shape — and in all three what a
    /// host shows is the page itself, which is the transition's own end state.
    ///
    /// Ends the transition at Table 164's `/D`, which is the one place time is compared against
    /// the document's own number: the fraction handed to `viewer_core::transition::frame` is
    /// elapsed over duration, linear, because the table states a duration and no curve.
    ///
    /// # Errors
    ///
    /// [`DisplayListError`] where the frame's own commands would not build. Not reachable from a
    /// frame — the largest one adds four clips — and returned rather than swallowed for the reason
    /// every refusal in this tree is. The transition ends, so a window cannot be left holding a
    /// picture it could not draw.
    pub fn frame(
        &mut self,
        viewport: Rect,
        now: Instant,
    ) -> Result<Option<DisplayList>, DisplayListError> {
        let Some(playing) = self.playing.as_ref() else {
            return Ok(None);
        };
        let elapsed = now.saturating_duration_since(playing.began).as_secs_f32();
        let progress = if playing.transition.duration > 0.0 {
            elapsed / playing.transition.duration
        } else {
            1.0
        };
        if progress >= 1.0 {
            self.ended(now);
            return Ok(None);
        }
        let Some(shaped) = viewer_core::transition::frame(&playing.transition, viewport, progress)
        else {
            self.ended(now);
            return Ok(None);
        };
        match shaped.draw(viewport, &playing.outgoing, &playing.incoming) {
            Ok(list) => Ok(Some(list)),
            Err(problem) => {
                self.ended(now);
                Err(problem)
            }
        }
    }

    /// Puts the clock back on the page, which is where §12.4.4.1's EXAMPLE says it starts.
    fn ended(&mut self, now: Instant) {
        self.playing = None;
        self.ticked = now;
    }
}

/// Where one page's own display list goes when it is drawn as a transition's face.
///
/// Both faces of a frame are "the page rasterised for the whole viewport", which
/// `viewer_core::transition::Frame::draw` states as its own precondition — so a host takes the
/// [`TargetSpec`] the core asked for, which is the page's own extent, and composes the placement
/// the viewport gives it. Doing it anywhere else would let two hosts disagree about where the last
/// frame of a transition leaves the page, which is exactly the seam a person sees.
#[must_use]
pub fn face_target(page: TargetSpec, origin: (f32, f32), viewport: (u32, u32)) -> TargetSpec {
    TargetSpec {
        width: viewport.0,
        height: viewport.1,
        transform: page
            .transform
            .then(Transform::translate(origin.0, origin.1)),
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use pdf_model::navigation::{Dimension, Direction, Motion, Style, Transition};
    use pdf_render::{Image, Point, Rect};

    use super::{Clock, face_target};

    /// A viewport big enough for a frame to have a middle.
    fn viewport() -> Rect {
        Rect::from_corners(Point::new(0.0, 0.0), Point::new(200.0, 100.0))
    }

    /// One page's pixels, at the viewport's size, in the format `drawable` produces.
    fn face() -> Image {
        Image {
            width: 200,
            height: 100,
            data: vec![0u8; 200 * 100 * 4].into(),
            interpolate: false,
        }
    }

    /// Table 164's style over two seconds, with the table's own defaults on every other entry.
    fn over_two_seconds(style: Style) -> Transition {
        Transition {
            style,
            duration: 2.0,
            dimension: Dimension::Horizontal,
            motion: Motion::Inward,
            direction: Direction::Degrees(0.0),
            scale: 1.0,
            opaque: false,
        }
    }

    /// `Wipe`, which is one of the seven styles this program shapes.
    fn wipe() -> Transition {
        over_two_seconds(Style::Wipe)
    }

    /// §12.4.4.1: a tick carries the milliseconds that really passed, and a call inside the same
    /// instant carries nothing at all rather than a zero.
    #[test]
    fn a_tick_carries_the_time_that_passed() {
        let start = Instant::now();
        let mut clock = Clock::started(start);
        assert_eq!(clock.tick(start), None, "no time has passed");
        assert_eq!(
            clock.tick(start + Duration::from_millis(250)),
            Some(250),
            "a quarter of a second"
        );
        assert_eq!(
            clock.tick(start + Duration::from_millis(400)),
            Some(150),
            "since the last tick, not since the start"
        );
    }

    /// §12.4.4.1's EXAMPLE: "[b]efore the page is displayed, there is a 3.5-second transition",
    /// so the transition is not part of the display duration it introduces and the page's own
    /// `/Dur` starts when the effect ends.
    #[test]
    fn the_clock_is_held_while_a_transition_is_drawn() {
        let start = Instant::now();
        let mut clock = Clock::started(start);
        clock.begin(wipe(), face(), face(), start);
        assert!(clock.animating());
        assert_eq!(
            clock.tick(start + Duration::from_secs(1)),
            None,
            "a tick here would spend the arriving page's /Dur on its own transition"
        );
        let ended = start + Duration::from_secs(3);
        assert!(
            clock
                .frame(viewport(), ended)
                .expect("a wipe's frame builds")
                .is_none(),
            "two seconds of /D have run out"
        );
        assert!(!clock.animating());
        assert_eq!(
            clock.tick(ended + Duration::from_millis(500)),
            Some(500),
            "the page's own duration begins where the transition ended"
        );
    }

    /// Table 164's `/D` decides the last frame, and the fraction between is linear.
    #[test]
    fn a_frame_is_drawn_until_the_duration_runs_out() {
        let start = Instant::now();
        let mut clock = Clock::started(start);
        clock.begin(wipe(), face(), face(), start);
        assert!(
            clock
                .frame(viewport(), start + Duration::from_secs(1))
                .expect("a wipe's frame builds")
                .is_some(),
            "half way through two seconds"
        );
        assert!(
            clock
                .frame(viewport(), start + Duration::from_secs(2))
                .expect("a wipe's frame builds")
                .is_none(),
            "/D has elapsed exactly"
        );
    }

    /// A host with no better clock animates at [`Clock::ANIMATING`] and rests at
    /// [`Clock::RESTING`], which is the whole of what an interval decides.
    #[test]
    fn the_interval_follows_whether_anything_is_moving() {
        let start = Instant::now();
        let mut clock = Clock::started(start);
        assert_eq!(clock.interval(), Clock::RESTING);
        clock.begin(wipe(), face(), face(), start);
        assert_eq!(clock.interval(), Clock::ANIMATING);
    }

    /// Five of Table 164's twelve styles are shaped by nothing — four reported by name and `R`,
    /// which is the cut — so a host asks before it rasterises two pages for a transition nobody
    /// will see.
    #[test]
    fn a_style_this_program_does_not_shape_is_refused_before_the_pages_are_taken() {
        assert!(Clock::shapes(&wipe(), viewport()));
        assert!(!Clock::shapes(
            &over_two_seconds(Style::Dissolve),
            viewport()
        ));
    }

    /// A face is the page's own list drawn into the whole viewport at the placement the viewport
    /// gives it, which is what `Frame::draw` is documented to expect of both of them.
    #[test]
    fn a_face_is_the_page_placed_in_the_whole_viewport() {
        let page = pdf_render::TargetSpec {
            width: 40,
            height: 20,
            transform: pdf_render::Transform::IDENTITY,
        };
        let composed = face_target(page, (12.0, 5.0), (200, 100));
        assert_eq!((composed.width, composed.height), (200, 100));
        let placed = composed.transform.apply(Point::new(0.0, 0.0));
        assert!((placed.x - 12.0).abs() < f32::EPSILON);
        assert!((placed.y - 5.0).abs() < f32::EPSILON);
    }
}
