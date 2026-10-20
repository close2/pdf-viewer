//! A viewer window's frames: the graphics device on a thread of its own, the lane each frame is
//! drawn in, the processor's composition, and what the window shows while a frame is drawn.
//!
//! **A library because more than one window needs it.** `quorra` drew through this code from
//! inside its own binary, and a toolkit window that wants a device of its own (ADR 2049's G1 and
//! Q1, RFC 0010's N3a) would otherwise write the chooser a second time — and the third copy of a
//! function is where hosts stop agreeing (`doc/todo/30`). ADR 2052 is the move and its argument.
//!
//! | module | what it is |
//! |---|---|
//! | [`renderer`] | the device on a thread of its own, and the finished window frames that cross back |
//! | [`lane`] | which coverage lane a frame is drawn with, and what a swapchain state means |
//! | [`stale`] | what the window puts up while the frame of the view asked for is being drawn |
//! | [`compose`] | Table 29's arrangement composed on the processor, for either kind of window |
//!
//! # What a host supplies
//!
//! **Three small things, and none of them names a toolkit.** A window's event loop is woken by a
//! [`Wake`]; the stand-in policy says what it declined through [`FrameLines`] and arms the clock
//! that asks for the real frame through [`FrameClock`]. The surface is not one of them: a host
//! makes its `QuorraWindowRenderer` from whatever its window converts to a
//! `wgpu::SurfaceTarget` and hands the device to [`renderer::Window::split`], so no window type is
//! named here (ADR 2048's rule for a platform handle, ADR 2052).

#![forbid(unsafe_code)]

pub mod compose;
pub mod lane;
pub mod renderer;
pub mod stale;

/// What the render thread calls so that the window's event loop comes round to a frame that has
/// landed, rather than at whatever wakes it next (ADR 1926).
///
/// **A host's loop may rest in a wait**, and a frame finished on another thread is invisible to
/// it until something wakes it; `quorra` wakes winit's loop with a user event, and a toolkit would
/// post to its main context. A wake sent to a loop that has exited is nothing to tell, so the
/// call returns nothing.
///
/// **Cloned rather than shared**, because each render thread takes one of its own and a host's
/// waker need not be `Sync` — winit's proxy is not on every platform it supports.
pub struct Wake(Box<dyn Again>);

/// A wake that can make another of itself, which is what lets [`Wake`] be cloned without asking
/// the host's waker to be shared between threads.
trait Again: Send {
    /// Wakes the host's loop.
    fn wake(&self);
    /// Another wake for the same loop.
    fn again(&self) -> Box<dyn Again>;
}

impl<F: Fn() + Send + Clone + 'static> Again for F {
    fn wake(&self) {
        self();
    }

    fn again(&self) -> Box<dyn Again> {
        Box::new(self.clone())
    }
}

impl Wake {
    /// The wake that calls `wake` on the render thread whenever a frame or a look has landed.
    #[must_use]
    pub fn new(wake: impl Fn() + Send + Clone + 'static) -> Self {
        Self(Box::new(wake))
    }

    /// Wakes the host's loop.
    pub fn wake(&self) {
        self.0.wake();
    }
}

impl Clone for Wake {
    fn clone(&self) -> Self {
        Self(self.0.again())
    }
}

impl std::fmt::Debug for Wake {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Wake").finish_non_exhaustive()
    }
}

/// Where the stand-in policy says what it decided about a frame, as one line of a host's trace.
///
/// `quorra`'s `--trace frames` is the one there is; the policy asks whether anybody is listening
/// no more than it did when it named that trace itself, so a host whose trace is off says nothing.
pub trait FrameLines: Copy {
    /// Says `what`, or nothing where the host's frame trace is off.
    fn say(self, what: std::fmt::Arguments<'_>);
}

/// The clock that asks for the frame replacing a stand-in, which is `doc/todo/37`'s rule 1.
///
/// The host owns the clock because it owns the refresh it ticks on; the policy only says that the
/// real frame is owed, and from when (`stale::MustFollow`).
pub trait FrameClock {
    /// Records that a frame is owed from `now`, so that the clock asks for it on its next tick.
    fn owed(&mut self, now: std::time::Instant);
}
