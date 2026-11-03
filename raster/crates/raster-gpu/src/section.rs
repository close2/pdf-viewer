//! Device sections: when this crate begins and ends work on the graphics device, said to a hook
//! a host or a harness installs (the caller's ADR 2664).
//!
//! A machine whose graphics adapter is shared between processes — two test suites, a gate and a
//! neighbour's measurement — needs each process to hold the adapter only while it is actually
//! using it. A [`Device`](crate::Device) lives as long as a window does, so its lifetime is the
//! wrong span; what is the right span is known only here, where the device is brought up and
//! each frame is submitted and waited for. This module is the seam: a [`SectionHook`] installed
//! once per process is told [`SectionHook::begin`] and [`SectionHook::end`] around
//!
//! - **bring-up** ([`DeviceWork::BringUp`]): from the moment the adapter is chosen until the
//!   constructor returns — `request_device`, the samplers and query sets, the warm-up thread's
//!   start. Adapter enumeration runs before it, because the adapter's type is what lets a hook
//!   tell a hardware adapter from a processor's;
//! - **each frame** ([`DeviceWork::Frame`]): a public call that records, submits and, where the
//!   target is read back or presented, waits — [`Device::render`](crate::Device::render) and its
//!   siblings, [`Device::submit`](crate::Device::submit) and [`Device::collect`](crate::Device::collect)
//!   each on its own, and [`Presenter::present`](crate::Presenter::present);
//! - **an upload** ([`DeviceWork::Upload`]): a resident resource written to the device outside a
//!   frame.
//!
//! The warm-up thread's pipeline compiles are not a section: they are the driver's compiler on a
//! processor, and they submit nothing to the adapter.
//!
//! **Nobody installs a hook in a shipped host**, and with none installed a section is one atomic
//! load of an empty [`OnceLock`]: the launch path pays nothing for the seam's existence (the
//! caller's CLAUDE.md principle 2). The hook is what a test or a gate installs; the lock it takes,
//! and the file that lock is on, are the harness's and never this crate's, because a machine path
//! has no place in a library (the caller's ADR 2656).
//!
//! **Sections may nest and may overlap across threads** — a harness holds an outer section around
//! a whole timed sequence and every frame inside it opens one of its own; a host's device thread
//! renders while another thread uploads. A hook that serialises against other processes counts its
//! holders rather than locking twice.

use std::sync::OnceLock;

/// What a section is for, so that a hook that reports its holds can say what each one was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceWork {
    /// A device's construction, from the adapter's choice to the constructor's return.
    BringUp,
    /// One frame's recording, submission and, where the caller waits for it, completion.
    Frame,
    /// A resident resource written to the device outside a frame.
    Upload,
}

/// One section, as a hook is told of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Section {
    /// What the section is for.
    pub work: DeviceWork,
    /// The kind of adapter the device is on. [`wgpu::DeviceType::Cpu`] is a processor drawing
    /// for a software driver (lavapipe, llvmpipe, WARP), which shares no adapter with anybody.
    pub device_type: wgpu::DeviceType,
}

/// What a host or a harness installs to be told when device work begins and ends.
///
/// `begin` may block — a hook that waits for another process's hold does so here — and `end` is
/// called exactly once for every `begin`, on the same thread, including when the section's work
/// returned an error or unwound.
pub trait SectionHook: Send + Sync {
    /// Device work described by `section` is about to start.
    fn begin(&self, section: Section);
    /// The device work `begin` was told of has finished.
    fn end(&self, section: Section);
}

static HOOK: OnceLock<Box<dyn SectionHook>> = OnceLock::new();

/// Installs the process's hook. One per process, for the process's lifetime: a hook that could be
/// swapped mid-section would be told an `end` it never had a `begin` for.
///
/// # Errors
///
/// Returns the hook back when one is installed already.
pub fn install_section_hook(hook: Box<dyn SectionHook>) -> Result<(), Box<dyn SectionHook>> {
    HOOK.set(hook)
}

/// Whether a hook is installed, for a harness that installs its own only where nobody else has.
#[must_use]
pub fn section_hook_installed() -> bool {
    HOOK.get().is_some()
}

/// A section in progress: `begin` was called when it was made, and `end` is called when it drops.
pub(crate) struct Open {
    hook: Option<&'static dyn SectionHook>,
    section: Section,
}

impl Open {
    /// Tells the installed hook, if any, that `work` on an adapter of `device_type` begins.
    pub(crate) fn begin(work: DeviceWork, device_type: wgpu::DeviceType) -> Self {
        let section = Section { work, device_type };
        let hook = HOOK.get().map(Box::as_ref);
        if let Some(hook) = hook {
            hook.begin(section);
        }
        Self { hook, section }
    }
}

impl Drop for Open {
    fn drop(&mut self) {
        if let Some(hook) = self.hook {
            hook.end(self.section);
        }
    }
}
