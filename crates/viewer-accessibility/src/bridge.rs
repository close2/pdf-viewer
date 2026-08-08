//! The adapter that puts the tree on AT-SPI, and the two platforms that have none.
//!
//! # Nothing eager, and where the runtime is
//!
//! `accesskit_unix` needs an executor, because `zbus` is asynchronous. It puts one on **a thread
//! of its own**, created the first time an adapter is: the thread connects to the session bus,
//! watches `org.a11y.Status.IsEnabled`, and leaves every adapter *inactive* — publishing nothing,
//! asking this program for nothing — until an assistive technology is actually there.
//!
//! What this crate adds to that is the one rule `CLAUDE.md`'s startup section makes
//! non-negotiable: **the adapter is not created on the launch path at all**. [`Bridge::new`] is
//! called after the first frame has been presented, so a D-Bus connection is never in front of
//! page one. ADR 0214 has the measurement.
//!
//! # What the other two platforms say
//!
//! macOS and Windows have AccessKit adapters and this program does not use them, because nothing
//! here can test one. That is a shortfall rather than an absence, and it is *named* —
//! [`Bridge::shortfall`] answers with a sentence a host prints, exactly as
//! `pdf_sandbox::Confinement::shortfall` does for the two platforms with no kernel confinement
//! (ADR 0194). A build that quietly did nothing would be the failure that precedent exists to
//! prevent.

use std::sync::{Arc, Mutex, mpsc};

use accesskit::TreeUpdate;

/// What an assistive technology asked this program to do.
///
/// Carried out by nobody yet, and that is a bounded gap rather than a silence: the tree this
/// crate publishes declares **no** actions on any node, so a conforming client has nothing to
/// request. Anything that arrives anyway is handed to the host, which says so out loud.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Requested {
    /// Which node it was asked of.
    pub node: accesskit::NodeId,
    /// What was asked.
    pub action: accesskit::Action,
}

/// The bridge between this program's tree and the platform's.
///
/// Holds the current tree so that the platform can ask for it from another thread —
/// `accesskit::ActivationHandler::request_initial_tree` is called on the adapter's own thread,
/// which is why the tree is behind a lock rather than rebuilt on demand.
#[derive(Debug)]
pub struct Bridge {
    /// The whole tree as it stands, which is what an attaching client is given.
    current: Arc<Mutex<Option<TreeUpdate>>>,
    /// What clients have asked for and nobody has carried out.
    asked: mpsc::Receiver<Requested>,
    /// The platform adapter, where this platform has one.
    #[cfg(target_os = "linux")]
    adapter: accesskit_unix::Adapter,
    /// Kept so that [`Bridge::requested`] answers with an empty list rather than a disconnected
    /// channel on a platform with no adapter to fill it.
    #[cfg(not(target_os = "linux"))]
    _asking: mpsc::Sender<Requested>,
}

/// Answers the platform's request for a tree with whatever the host last published.
#[cfg(target_os = "linux")]
struct Activation {
    /// The same lock [`Bridge::current`] holds.
    current: Arc<Mutex<Option<TreeUpdate>>>,
}

#[cfg(target_os = "linux")]
impl accesskit::ActivationHandler for Activation {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        // A poisoned lock means a panic while a tree was being stored, which under this
        // workspace's `panic = "abort"` cannot happen — and if it somehow did, answering `None`
        // is what the trait documents for "not ready yet": the adapter waits and asks again.
        self.current.lock().ok().and_then(|held| held.clone())
    }
}

/// Hands a client's request to the host rather than dropping it.
#[cfg(target_os = "linux")]
struct Actions {
    /// The host's end of the queue.
    asking: mpsc::Sender<Requested>,
}

#[cfg(target_os = "linux")]
impl accesskit::ActionHandler for Actions {
    fn do_action(&mut self, request: accesskit::ActionRequest) {
        // The host has gone away: the program is shutting down and there is nobody to tell.
        let _ = self.asking.send(Requested {
            node: request.target_node,
            action: request.action,
        });
    }
}

/// Notices that the last assistive technology went away.
#[cfg(target_os = "linux")]
struct Deactivation;

#[cfg(target_os = "linux")]
impl accesskit::DeactivationHandler for Deactivation {
    fn deactivate_accessibility(&mut self) {
        // Nothing to release: the tree this crate holds is one page's worth of plain data, and
        // rebuilding it costs one page turn's work. The adapter will ask for it again.
    }
}

impl Bridge {
    /// Brings the platform adapter up.
    ///
    /// **Not on the launch path.** Called after the first frame is on the screen; on Linux it
    /// spawns `accesskit_unix`'s thread, which connects to the session bus and publishes nothing
    /// until an assistive technology is present.
    #[must_use]
    pub fn new() -> Self {
        let current = Arc::new(Mutex::new(None));
        let (asking, asked) = mpsc::channel();
        #[cfg(target_os = "linux")]
        {
            let adapter = accesskit_unix::Adapter::new(
                Activation {
                    current: Arc::clone(&current),
                },
                Actions {
                    asking: asking.clone(),
                },
                Deactivation,
            );
            drop(asking);
            Self {
                current,
                asked,
                adapter,
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            // No adapter, and therefore no handlers to give one: what this platform does instead
            // is `shortfall`, which says so in a sentence the host prints. The tree is still
            // built and still stored, so the half of this crate that maps §14.8.4 onto AccessKit
            // compiles and is tested here exactly as it is on Linux.
            Self {
                current,
                asked,
                _asking: asking,
            }
        }
    }

    /// What this build cannot do, in a sentence a host prints.
    ///
    /// `None` on Linux, where AT-SPI is reached through `accesskit_unix`. `Some` on macOS and
    /// Windows, which have AccessKit adapters this program does not use — the same shape
    /// `pdf_sandbox::Confinement::shortfall` takes, and for the same reason: a platform that
    /// silently exposed nothing would look identical to one where the bridge is broken.
    #[must_use]
    pub fn shortfall() -> Option<&'static str> {
        #[cfg(target_os = "linux")]
        {
            None
        }
        #[cfg(not(target_os = "linux"))]
        {
            Some(
                "this build has no accessibility bridge — AccessKit's macOS and Windows adapters \
                 exist and are not wired in here, so a screen reader sees the window and not the \
                 page's structure. ISO 32000-2 §14.7's tree is still read and is still what \
                 `Query::AccessibilityTree` answers with",
            )
        }
    }

    /// Whether this build has an adapter that needs to be told where the window is.
    ///
    /// **A separate question from [`Self::shortfall`], and the reason is that the two will stop
    /// agreeing.** AT-SPI reports a node's bounds in *screen* coordinates, so an X11 host has to
    /// hand `accesskit_unix` the window's own place before any node is where it says it is.
    /// AccessKit's Windows and macOS adapters take a window *handle* and let the platform do
    /// that arithmetic, so when `doc/todo/31` wires them in, `shortfall` will answer `None` there
    /// while this still answers `false`. A host that had asked "is there a bridge" would then
    /// start paying for a position nobody wants.
    ///
    /// **What it is worth**: the two calls a host makes to satisfy this — winit's
    /// `outer_position` and `inner_position` — are **synchronous X11 round trips**, measured at
    /// **1.8 to 3.2 ms together** on this machine over twenty page turns (ADR 0228). That was
    /// being paid on every page turn, on every platform, including the two where the result was
    /// dropped by the `#[cfg]` in [`Self::placed`].
    #[must_use]
    pub const fn wants_window_bounds() -> bool {
        cfg!(target_os = "linux")
    }

    /// Publishes a page.
    ///
    /// Stores the tree for a client that attaches later, and gives it to the platform now if one
    /// is already attached. `update_if_active` does nothing at all while no assistive technology
    /// is present, which is what makes calling this on every page turn free.
    pub fn publish(&mut self, view: &crate::PageView) {
        let update = crate::tree::build(view);
        if let Ok(mut held) = self.current.lock() {
            *held = Some(update.clone());
        }
        #[cfg(target_os = "linux")]
        self.adapter.update_if_active(|| update);
        #[cfg(not(target_os = "linux"))]
        drop(update);
    }

    /// Tells the platform where the window is, which is what X11 needs to place a node.
    ///
    /// AT-SPI reports a node's position in *screen* coordinates and this crate's bounds are in
    /// the window's, so the adapter needs the window's own place to add them. Under Wayland an
    /// application cannot learn its own position and `accesskit_unix` says so; this is called
    /// where the host knows, and skipped where it does not.
    #[cfg_attr(
        not(target_os = "linux"),
        expect(
            unused_variables,
            reason = "the platforms with no adapter have nowhere to put this, and \
                      `shortfall` is what says so"
        )
    )]
    pub fn placed(&mut self, outer: (f32, f32, f32, f32), inner: (f32, f32, f32, f32)) {
        #[cfg(target_os = "linux")]
        self.adapter
            .set_root_window_bounds(rect(outer), rect(inner));
    }

    /// What clients have asked for since this was last called.
    ///
    /// Drained rather than kept: a host prints them, which is the whole of what this program does
    /// with an action request today.
    pub fn requested(&mut self) -> Vec<Requested> {
        self.asked.try_iter().collect()
    }
}

impl Default for Bridge {
    fn default() -> Self {
        Self::new()
    }
}

/// A rectangle in the platform's own type.
#[cfg(target_os = "linux")]
fn rect(bounds: (f32, f32, f32, f32)) -> accesskit::Rect {
    accesskit::Rect {
        x0: f64::from(bounds.0),
        y0: f64::from(bounds.1),
        x1: f64::from(bounds.2),
        y1: f64::from(bounds.3),
    }
}

#[cfg(test)]
mod tests {
    use super::Bridge;

    /// A build with no adapter has nowhere to put a window's place on the screen.
    ///
    /// The two questions are separate on purpose — see [`Bridge::wants_window_bounds`] — and
    /// they are separate in one direction only: an adapter may not need the bounds, but the
    /// absence of an adapter certainly does not need them. A host pays two synchronous X11
    /// round trips to answer this, so the implication is worth a guard rather than a comment:
    /// `doc/todo/31` will make `shortfall` answer `None` on two more platforms, and if
    /// `wants_window_bounds` were ever written as its negation this would be the thing that
    /// noticed.
    #[test]
    fn a_build_with_no_bridge_wants_no_window_bounds() {
        assert!(
            Bridge::shortfall().is_none() || !Bridge::wants_window_bounds(),
            "a platform with no adapter asked to be told where the window is"
        );
    }
}
