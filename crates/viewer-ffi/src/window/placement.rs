//! Table 147's `/FitWindow` and `/CenterWindow`, answered to the caller's own window extents after
//! a `QUORRA_OWED_PLACE` record (ADR 2308).
//!
//! The decision is `viewer_host::native_window::NativeWindow::place_the_window`'s, the one
//! `viewer-qt` takes too; what is the caller's is measuring its window and carrying the answer out
//! — a resize, then a move once the window fits — because only the caller's toolkit can.

use viewer_host::native_window::{Extents, NativeWindow as _, Placement};

use super::Window;

/// The caller's window as `quorra_window_place` reads it, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct WindowExtents {
    /// The window's inner width: everything inside its frame.
    pub window_width: i32,
    /// Its inner height.
    pub window_height: i32,
    /// What its frame adds to the width.
    pub frame_width: i32,
    /// What its frame adds to the height.
    pub frame_height: i32,
    /// The left edge of the available area of the screen it is on.
    pub screen_left: i32,
    /// That area's top edge.
    pub screen_top: i32,
    /// That area's width, zero or less where the window is on no screen yet.
    pub screen_width: i32,
    /// That area's height, zero or less where the window is on no screen yet.
    pub screen_height: i32,
    /// One where the platform lets a program move its own window, zero where it places every
    /// window itself.
    pub positions: u32,
}

/// What `quorra_window_place` answers: a flag before each pair, in logical pixels.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(C)]
pub struct WindowPlacement {
    /// One where the inner window is to be resized to `width` by `height`, and asked again after
    /// the next `QUORRA_OWED_PLACE`.
    pub resize: u32,
    /// The inner width wanted.
    pub width: i32,
    /// The inner height wanted.
    pub height: i32,
    /// One where the window's outer top-left corner is to be moved to `x`, `y`.
    pub move_to: u32,
    /// The outer left edge wanted.
    pub x: i32,
    /// The outer top edge wanted.
    pub y: i32,
}

impl Window {
    /// Whether the window presents full screen now: Table 29's `FullScreen`, which the caller
    /// shows by its platform's own full-screen presentation, title bar and frame taken with it.
    #[must_use]
    pub const fn full_screen(&self) -> bool {
        self.state.presenting.full_screen()
    }

    /// Table 147's placement for the window `extents` describes: `None` where none is owed, and a
    /// placement of zeros once the window fits and nothing is left to move.
    pub fn place(&mut self, extents: WindowExtents) -> Option<WindowPlacement> {
        let WindowExtents {
            window_width,
            window_height,
            frame_width,
            frame_height,
            screen_left,
            screen_top,
            screen_width,
            screen_height,
            positions,
        } = extents;
        let extents = Extents {
            window: (window_width, window_height),
            frame: (frame_width, frame_height),
            screen: (screen_left, screen_top, screen_width, screen_height),
            positions: positions != 0,
        };
        let placement = self.place_the_window(self.scale, extents)?;
        Some(match placement {
            Placement::Resize { width, height } => WindowPlacement {
                resize: 1,
                width,
                height,
                ..WindowPlacement::default()
            },
            Placement::Move { x, y } => WindowPlacement {
                move_to: 1,
                x,
                y,
                ..WindowPlacement::default()
            },
            Placement::Placed => WindowPlacement::default(),
        })
    }
}
