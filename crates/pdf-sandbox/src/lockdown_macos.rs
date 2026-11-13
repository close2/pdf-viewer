//! The macOS mechanism: a Seatbelt profile that denies every operation it does not name.
//!
//! [`crate::lockdown`] is the front door and holds the vocabulary; this is what it calls on macOS.
//! The profile is entered with `sandbox_init(3)` through `darwin_sandbox::enter`, in the worker,
//! before it reads a byte — the place `crate::lockdown_linux` installs seccomp — so every worker
//! confines itself the same way on both platforms and nothing about how it is started changes
//! (ADR 2864).
//!
//! # What the profile says
//!
//! `(deny default)` and nothing else. A worker is handed everything it reads — the image bytes,
//! the document's descriptor, a face the host opened for it — over descriptors it already holds,
//! and reading and writing an open descriptor is not an operation a profile is asked about. So
//! there is nothing to allow: no path, no socket, no service lookup. The profile is entered after
//! the program and its libraries are loaded, which is why it needs none of the reads `dyld` makes
//! at start-up, which a profile applied at `exec` would (the `sandbox-exec` route ADR 2864 weighed
//! and declined).
//!
//! # What it does not say, and Linux's filter does
//!
//! Seatbelt decides by operation, not by system-call number. It cannot refuse a thread, a call to
//! `fcntl` or memory made executable, which the script profile's seccomp list refuses on Linux
//! (ADR 1608); [`crate::lockdown::SystemCalls::Seatbelt`] says so, and the property it does carry —
//! no filesystem and no network — is the one this crate exists for. Nor can a process give itself
//! a memory ceiling here: `setrlimit(RLIMIT_AS)` is refused and an allocation past the number it
//! asked for still succeeds, measured on macOS 26. The ceiling is therefore reported in the
//! [`Confinement`] for the starting process to hold, which `crate::watch` does.

use crate::lockdown::{Confinement, LandlockLevel, LockdownError, Profile, SystemCalls};

/// The profile every worker enters: every operation refused, and none named.
///
/// One text for all three [`Profile`]s, because none of them asks the kernel for anything a
/// profile governs once it is running; what differs between them on Linux — threads, descriptors
/// received, executable memory — is what Seatbelt cannot see. The profiles stay distinct where
/// they still mean something here: the ceiling each is held to.
const DENY_EVERYTHING: &str = "(version 1)\n(deny default)\n";

/// Confines the calling process. There is no way to undo this.
///
/// # Errors
///
/// [`LockdownError::Seatbelt`] if the profile was not entered. A caller that gets an error must not
/// continue with the work it intended to confine.
pub(crate) fn apply(profile: Profile, wider: Option<u64>) -> Result<Confinement, LockdownError> {
    darwin_sandbox::enter(DENY_EVERYTHING)?;
    crate::lockdown::CONFINED.store(true, std::sync::atomic::Ordering::Release);
    Ok(Confinement {
        landlock: LandlockLevel::Unavailable,
        address_space_limit: crate::lockdown::ceiling(profile, wider),
        system_calls: SystemCalls::Seatbelt,
    })
}
