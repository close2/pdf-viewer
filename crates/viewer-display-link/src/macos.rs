//! `NSView`'s `CADisplayLink` (macOS 14): a callback on the main run loop at every refresh of
//! the display the view is on, following the view when the window moves between displays.
#![allow(
    unsafe_code,
    reason = "Objective-C messages are unsafe calls; each block says what it relies on"
)]

use std::cell::Cell;
use std::rc::Rc;

use objc2::rc::Retained;
use objc2::runtime::{NSObject, NSObjectProtocol};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::NSView;
use objc2_foundation::{NSRunLoop, NSRunLoopCommonModes};
use objc2_quartz_core::{CADisplayLink, CAFrameRateRange};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::{IDLE_BEATS, Unavailable};

/// What the link's callback and the window share. Main thread only, so cells suffice.
pub(crate) struct Shared {
    /// Refreshes seen.
    pub(crate) beats: Cell<u64>,
    /// The last refresh the window was woken for.
    pub(crate) woken: Cell<u64>,
    /// Whether a frame is wanted at the next refresh.
    wanted: Cell<bool>,
    /// Refreshes in a row nobody wanted a frame for.
    idle: Cell<u32>,
    /// One refresh, in seconds, as the link last stated it.
    pub(crate) period: Cell<f64>,
    /// What makes the window's event loop come round.
    wake: Box<dyn Fn() + Send>,
}

define_class!(
    /// The object a `CADisplayLink` calls: Objective-C takes a target and a selector, not a
    /// closure.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "QuorraDisplayLinkTarget"]
    #[ivars = Rc<Shared>]
    struct Target;

    impl Target {
        #[unsafe(method(beat:))]
        fn beat(&self, link: &CADisplayLink) {
            let shared = self.ivars();
            let beats = shared.beats.get().saturating_add(1);
            shared.beats.set(beats);
            let period = link.targetTimestamp() - link.timestamp();
            shared.period.set(period);
            if shared.wanted.get() {
                shared.idle.set(0);
                shared.woken.set(beats);
                (shared.wake)();
                return;
            }
            let idle = shared.idle.get().saturating_add(1);
            shared.idle.set(idle);
            if idle >= IDLE_BEATS {
                link.setPaused(true);
            }
        }
    }

    unsafe impl NSObjectProtocol for Target {}
);

impl Target {
    fn new(mtm: MainThreadMarker, shared: Rc<Shared>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(shared);
        // SAFETY: `NSObject`'s `init` on a freshly allocated instance whose ivars are set.
        unsafe { msg_send![super(this), init] }
    }
}

/// A running or paused link, and what it shares with its callback.
pub(crate) struct Link {
    link: Retained<CADisplayLink>,
    pub(crate) shared: Rc<Shared>,
}

impl Link {
    pub(crate) fn on(
        window: &impl HasWindowHandle,
        wake: Box<dyn Fn() + Send>,
    ) -> Result<Self, Unavailable> {
        let mtm = MainThreadMarker::new().ok_or(Unavailable::NotMainThread)?;
        let handle = window.window_handle().map_err(|_| Unavailable::NotNative)?;
        let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
            return Err(Unavailable::NotNative);
        };
        // SAFETY: an AppKit window handle's `ns_view` is a valid `NSView` for as long as the
        // handle borrows the window, which outlives this call; the reference is not kept.
        let view: &NSView = unsafe { appkit.ns_view.cast::<NSView>().as_ref() };
        let selector = sel!(displayLinkWithTarget:selector:);
        if !view.respondsToSelector(selector) {
            return Err(Unavailable::TooOld);
        }
        let shared = Rc::new(Shared {
            beats: Cell::new(0),
            woken: Cell::new(0),
            wanted: Cell::new(false),
            idle: Cell::new(0),
            period: Cell::new(0.0),
            wake,
        });
        let target = Target::new(mtm, Rc::clone(&shared));
        // SAFETY: `target` is a `Target`, which implements `beat:` taking the link, and the
        // selector names that method; the link retains its target. Checked above that the view
        // answers the message, which is macOS 14's.
        let link = unsafe { view.displayLinkWithTarget_selector(&target, sel!(beat:)) };
        // ProMotion lowers a link's rate unless the link asks for the panel's top rate, and a
        // minimum of 60 lets it settle there; a display slower than this clamps it to its own.
        link.setPreferredFrameRateRange(CAFrameRateRange {
            minimum: 80.0,
            maximum: 120.0,
            preferred: 120.0,
        });
        link.setPaused(true);
        // SAFETY: the main run loop, from the main thread (`mtm` above). The common modes, so
        // the link also runs while AppKit tracks a resize or a menu.
        unsafe {
            link.addToRunLoop_forMode(&NSRunLoop::mainRunLoop(), NSRunLoopCommonModes);
        }
        Ok(Self { link, shared })
    }

    pub(crate) fn running(&self) -> bool {
        !self.link.isPaused()
    }

    pub(crate) fn listen(&self) {
        // Activity, whether or not a frame is wanted at the next refresh: a window drawing on its
        // input's events never wants one, and a link that counted those refreshes as idle paused
        // in the middle of a gesture and took a refresh or two to start again.
        self.shared.idle.set(0);
        if self.link.isPaused() {
            self.link.setPaused(false);
        }
    }

    pub(crate) fn want(&self, wanted: bool) {
        self.shared.wanted.set(wanted);
        if wanted && self.link.isPaused() {
            self.shared.idle.set(0);
            self.link.setPaused(false);
        }
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        // Takes the link off the run loop and releases its target, which holds `shared`.
        self.link.invalidate();
    }
}
