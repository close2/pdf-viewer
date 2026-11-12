//! The display's own refresh as a window's clock.
//!
//! A window that presents on a timer of its own presents *beside* the display rather than with
//! it: a timer started at the last present and stepped by the stated period keeps a phase that
//! has nothing to do with when the panel refreshes, and a present made the moment an input event
//! arrives has none at all. Measured on a 120 Hz panel (ADR 2849), the intervals between presents
//! spread from 8 to 17 ms with a third of them a refresh late, while every frame took a little
//! over half a refresh to draw — the frames were fast enough and started at the wrong moments.
//!
//! **A display link is the platform telling the program when a refresh is**, which is the one
//! fact a timer cannot know. This crate wraps it as a counter and a wake: [`DisplayLink::beats`]
//! counts refreshes, and the wake handed to [`DisplayLink::on`] is called on each refresh a frame
//! is wanted for. What to draw and whether to present stay the window's; this only says *when*.
//!
//! # Where there is one
//!
//! | platform | source | here |
//! |---|---|---|
//! | macOS 14 and later | `NSView`'s `CADisplayLink`, which follows the view between displays | yes |
//! | macOS before 14 | `CVDisplayLink`, deprecated, on a thread of its own | no: [`Unavailable::TooOld`] |
//! | everything else | — | no: [`Unavailable::Platform`] |
//!
//! A window that is answered [`Unavailable`] keeps its own clock, which is what every platform
//! did before this crate; the answer is a reason to say in a trace, not a failure.
//!
//! # Power
//!
//! A link that runs costs a wake a refresh. **It runs only while a frame is wanted**, and pauses
//! itself after [`IDLE_BEATS`] refreshes that nobody wanted a frame for, so a window nobody is
//! touching is not woken at all — the same rule the window's own clock keeps.
#![deny(unsafe_code)]

use std::fmt;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as platform;

/// Every platform this crate has no display link for: a link that cannot be made, so that the
/// type and its methods are the same everywhere and only [`DisplayLink::on`]'s answer differs.
#[cfg(not(target_os = "macos"))]
mod platform {
    use crate::Unavailable;

    /// No value of this type exists, so every method below is unreachable by construction.
    pub(crate) enum Link {}

    impl Link {
        pub(crate) fn on(
            window: &impl raw_window_handle::HasWindowHandle,
            wake: Box<dyn Fn() + Send>,
        ) -> Result<Self, Unavailable> {
            let _ = (window, wake);
            Err(Unavailable::Platform)
        }

        pub(crate) fn beats(&self) -> u64 {
            match *self {}
        }

        pub(crate) fn answered(&self) -> u64 {
            match *self {}
        }

        pub(crate) fn running(&self) -> bool {
            match *self {}
        }

        pub(crate) fn want(&self, _wanted: bool) {
            match *self {}
        }

        pub(crate) fn listen(&self) {
            match *self {}
        }

        pub(crate) fn period(&self) -> Option<std::time::Duration> {
            match *self {}
        }
    }
}

/// How many refreshes in a row nobody wanted a frame for before the link pauses itself.
///
/// More than one, because a gesture's events do not arrive on every refresh and a link paused
/// between two of them would have to be started again for the next — a pause and a resume a
/// frame is two property writes, but the first refresh after a resume is the platform's to
/// time and not ours. Few enough that a window put down rests within a twentieth of a second.
pub const IDLE_BEATS: u32 = 6;

/// Why a window keeps its own clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unavailable {
    /// This platform's display link is not one this crate speaks.
    Platform,
    /// The window's handle is not the platform's native one.
    NotNative,
    /// The system is older than the display link this crate uses (macOS 14).
    TooOld,
    /// A display link can only be made on the thread that runs the window's event loop.
    NotMainThread,
}

impl fmt::Display for Unavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Platform => "this platform has no display link this program speaks",
            Self::NotNative => "the window's handle is not the platform's own",
            Self::TooOld => "the system predates NSView's display link (macOS 14)",
            Self::NotMainThread => "a display link is made on the main thread only",
        })
    }
}

impl std::error::Error for Unavailable {}

/// The display's refresh, as a count and a wake.
///
/// Main-thread only, like the window it is attached to. Dropping it stops the link.
pub struct DisplayLink {
    inner: platform::Link,
}

impl fmt::Debug for DisplayLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DisplayLink")
            .field("beats", &self.beats())
            .field("running", &self.running())
            .finish_non_exhaustive()
    }
}

impl DisplayLink {
    /// A link on the display `window` is on, calling `wake` on every refresh a frame is
    /// [wanted](Self::want) for.
    ///
    /// `wake` runs outside the window's event handler — on macOS inside the main run loop, and
    /// on a platform whose refresh is waited for on a thread of its own, on that thread — so what
    /// it may do is make that handler come round: `winit`'s `EventLoopProxy::send_event` is the
    /// intended body, and `Send` because a refresh need not be reported on the window's thread.
    ///
    /// The link starts paused.
    ///
    /// # Errors
    ///
    /// [`Unavailable`] where this platform, this system or this handle has no link this crate
    /// speaks; the window then keeps its own clock.
    pub fn on(
        window: &impl raw_window_handle::HasWindowHandle,
        wake: impl Fn() + Send + 'static,
    ) -> Result<Self, Unavailable> {
        platform::Link::on(window, Box::new(wake)).map(|inner| Self { inner })
    }

    /// How many refreshes the link has seen since it was made. Paused refreshes are not seen.
    #[must_use]
    pub fn beats(&self) -> u64 {
        self.inner.beats()
    }

    /// The last refresh that a frame was wanted for when it came, counted as [`Self::beats`]
    /// counts: the refresh the window was woken for.
    #[must_use]
    pub fn answered(&self) -> u64 {
        self.inner.answered()
    }

    /// Whether the link is counting refreshes now rather than paused.
    #[must_use]
    pub fn running(&self) -> bool {
        self.inner.running()
    }

    /// Says whether a frame is wanted at the next refresh. Wanting one starts a paused link.
    pub fn want(&self, wanted: bool) {
        self.inner.want(wanted);
    }

    /// Says the window is active: starts a paused link counting refreshes, and restarts its
    /// count towards pausing, without wanting a frame at the next refresh.
    ///
    /// For a window that has just presented: the input that follows a present comes at the
    /// display's pace, and it can only be told which refresh it landed in by a link that is
    /// counting. Unwanted, the link pauses itself again after [`IDLE_BEATS`].
    pub fn listen(&self) {
        self.inner.listen();
    }

    /// One refresh of the display the window is on now, as the link last stated it.
    #[must_use]
    pub fn period(&self) -> Option<std::time::Duration> {
        self.inner.period()
    }
}
