//! Table 147's `/FitWindow` and `/CenterWindow` as one answer to a window's own measured extents,
//! for the windows that place themselves by request — `viewer-qt` and the C window's caller
//! (ADRs 1429, 2308).
//!
//! **What is shared is the decision, and what is a toolkit's is the act.** A window states its
//! inner size, what its frame adds, the screen's available area and whether the platform lets a
//! program place its own window at all; the answer is one resize, one move, or nothing. The
//! arithmetic is [`crate::fitted`]'s and [`crate::centred`]'s, which `viewer-gtk` uses too; GTK is
//! not a caller here because GTK 4 has no frame to measure and no move to make, so its fit is a
//! default size and its centring a sentence.
//!
//! **A window that does not fit yet is asked to resize and nothing else**, and the next frame
//! measures again, because the first frame is drawn before the chrome this document hid has left
//! the viewport ([`crate::Presenting::place`] bounds the attempts). A window that fits is moved to
//! the centre where that was asked — or, where the platform places every window itself (a
//! Wayland compositor), is left where it is with the reason said on the terminal and named in the
//! title, ADR 1831's shape for a request this program could not carry out.

use viewer_core::{Answer, Query};

use super::NativeWindow;

/// A window's extents as its toolkit measured them, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extents {
    /// The window's inner width and height: everything inside its frame.
    pub window: (i32, i32),
    /// What its frame adds to the width and to the height.
    pub frame: (i32, i32),
    /// The available area of the screen it is on — left, top, width, height — with a width or
    /// height of zero or less for a window the platform has put on no screen yet.
    pub screen: (i32, i32, i32, i32),
    /// Whether the platform lets a program move its own window: `false` on Wayland, where the
    /// compositor places every window and a request to move one is ignored.
    pub positions: bool,
}

/// What a window does for Table 147, from [`NativeWindow::place_the_window`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// Resize the inner window to this extent, and ask again at the next frame.
    Resize {
        /// The inner width wanted.
        width: i32,
        /// The inner height wanted.
        height: i32,
    },
    /// The window fits: move its outer top-left corner here, and nothing more is owed.
    Move {
        /// The outer left edge wanted.
        x: i32,
        /// The outer top edge wanted.
        y: i32,
    },
    /// The window fits and is not to be moved, or cannot be: nothing more is owed.
    Placed,
}

/// The sentence a window whose platform places it says where `/CenterWindow` asked for the
/// centre, once, at the frame the fit is paid.
pub const NOT_CENTRED: &str = "this document asks for its window in the centre of the screen \
                               (§12.2's /CenterWindow); this platform places every window itself \
                               and lets no program move its own, so it stays where it was put";

/// What the title says after the caption while that request stands unmet.
pub const NOT_CENTRED_TITLE: &str = "not centred: the platform places this window";

/// [`NativeWindow::place_the_window`]'s body.
pub(super) fn place<W: NativeWindow + ?Sized>(
    window: &mut W,
    scale: f32,
    extents: Extents,
) -> Option<Placement> {
    let Extents {
        window: (window_width, window_height),
        frame: (frame_width, frame_height),
        screen: (screen_left, screen_top, screen_width, screen_height),
        positions,
    } = extents;
    let state = window.state_mut();
    let page = match state.viewer.query(Query::View) {
        Answer::View(viewing) => viewing.page,
        _ => 0,
    };
    let placing = state.presenting.place(page)?;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a window's extent and position in logical pixels, held to its screen"
    )]
    let whole = |value: f64| value.round() as i32;
    // A window the platform could not put on a screen has no screen to fit to or to centre on.
    let screen = (screen_width > 0 && screen_height > 0).then_some((
        f64::from(screen_left),
        f64::from(screen_top),
        f64::from(screen_width),
        f64::from(screen_height),
    ));
    let inner = (f64::from(window_width), f64::from(window_height));
    if placing.fit
        && let Answer::Geometry(geometry) = state.viewer.query(Query::PageGeometry(page))
    {
        let scale = f64::from(scale);
        let wanted = crate::fitted(
            inner,
            (
                f64::from(state.viewport.0) / scale,
                f64::from(state.viewport.1) / scale,
            ),
            (
                f64::from(geometry.width) / scale,
                f64::from(geometry.height) / scale,
            ),
            screen.map(|(_, _, width, height)| {
                (
                    width - f64::from(frame_width),
                    height - f64::from(frame_height),
                )
            }),
        );
        if (wanted.0 - inner.0).abs() > 1.0 || (wanted.1 - inner.1).abs() > 1.0 {
            return Some(Placement::Resize {
                width: whole(wanted.0),
                height: whole(wanted.1),
            });
        }
    }
    state.presenting.placed();
    if !placing.centre {
        return Some(Placement::Placed);
    }
    if !positions {
        state.showing.uncentred = true;
        window.say(NOT_CENTRED);
        window.retitle();
        return Some(Placement::Placed);
    }
    Some(screen.map_or(Placement::Placed, |screen| {
        let (x, y) = crate::centred(
            (
                inner.0 + f64::from(frame_width),
                inner.1 + f64::from(frame_height),
            ),
            screen,
        );
        Placement::Move {
            x: whole(x),
            y: whole(y),
        }
    }))
}
