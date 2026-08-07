//! Confining the calling process, irreversibly — and saying so where a platform cannot.
//!
//! The mechanism is Linux's and lives in [`crate::lockdown_linux`]: resource limits, a
//! Landlock domain that permits nothing, and a seccomp-BPF allow-list. This module is the
//! front door: the vocabulary for *what was achieved*, and [`apply`], which on a platform
//! with none of those three installs nothing and returns a [`Confinement`] that says so.
//!
//! # Why a platform without a filter still gets a worker
//!
//! Decided by the project owner in the three-hundred-and-fifteenth session, and it is a
//! narrowing of principle 3 rather than an abandonment of it. The crate's own documentation
//! lists three reasons the boundary exists, and only the first two are about the isolation
//! being *enforced by the kernel*:
//!
//! - **Resource exhaustion** — `RLIMIT_AS` is Linux's here. Off Linux the ceiling is gone, and
//!   what is left of it is `crate::decode`'s hand-placed `MAX_PIXELS` and `MAX_SAMPLES`, which
//!   are a discipline rather than a fact.
//! - **Panics** — this survives untouched, and it is a *process* property rather than a kernel
//!   one. Release builds abort on panic; a decoder that panics in a worker costs one image and
//!   the page draws around it, on every platform equally.
//! - **What comes later** — the protocol stays narrow everywhere, which is the half of that
//!   argument a platform cannot take away.
//!
//! **The thing that must not happen is a sandbox that silently does nothing**, which is what
//! the `compile_error!` this module replaced was defending against. So the level reached is
//! carried in the worker's handshake, [`Confinement::is_enforced`] answers it in one call, and
//! `viewer-ui` prints what it did not get. A caller that believed itself confined and was not
//! is the failure this design exists to prevent; a caller that is told, in the same sentence
//! every other refusal in this program uses, is not that failure.

/// What the confined process is going to do, which is what decides the allow-list.
///
/// **A profile rather than a set of options.** Which system calls a program needs is not a
/// preference a caller should be able to widen a little at a time; it is a property of the work,
/// found by running that work under `strace` and reading what appeared. So there are two, each
/// named for a program in this workspace, and adding a third means measuring a third.
///
/// The difference between them is exactly two things — threads, and how much address space —
/// and both follow from the second doing more than decode one image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Profile {
    /// One image, decoded on one thread: `pdf-sandbox-worker`.
    ///
    /// The narrower of the two, and the default because a caller who has not thought about it
    /// should get the list that permits less. It creates no thread, so `clone` is absent.
    #[default]
    Decoder,
    /// A document, its interpretation and its rasterisation: `pdf-view-worker`.
    ///
    /// Wider in two ways, both measured rather than assumed (ADR 0218):
    ///
    /// - **Threads.** `render-cpu` draws a page on every core, so `clone3`, `rseq`,
    ///   `set_robust_list` and `sched_getaffinity` are on the list. A thread is not a new
    ///   program: `execve`, `execveat`, `fork` and `vfork` stay off, so what this permits is
    ///   another thread of the code that is already confined and nothing else.
    /// - **Address space.** A rasteriser holds a page's pixels, and `viewer_core`'s own budget
    ///   for one is 2²⁸ pixels — a gibibyte of RGBA before the document, the display list or a
    ///   glyph cache is counted.
    Interpreter,
}

/// How thoroughly Landlock could be applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandlockLevel {
    /// Every access right this build knows about is denied by the kernel.
    Enforced,
    /// The kernel supports Landlock but not every right this build asked for.
    ///
    /// The rights it does support are denied. A right the kernel does not know is a right
    /// it does not implement, so nothing is permitted that would otherwise have been
    /// denied — the gap is in *future* protections, not current ones.
    Partial,
    /// The kernel does not support Landlock, it is not enabled, or this platform has no
    /// such interface at all.
    Unavailable,
}

/// Whether the worker's system calls are restricted to an allow-list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemCalls {
    /// seccomp-BPF is installed: anything outside the allow-list kills the process.
    ///
    /// This is the layer that makes "no filesystem and no network" true irrespective of what
    /// any path or address would have permitted.
    Filtered,
    /// No filter. The worker may call anything the operating system offers it.
    ///
    /// Off Linux, where seccomp-BPF does not exist. What remains is the process boundary
    /// itself, which is not nothing — see this module's header — but it is not this.
    Unfiltered,
}

/// What confinement was achieved.
///
/// Reported rather than assumed: the parent process logs it, and a test asserts the
/// strongest level on a kernel that can provide it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Confinement {
    /// How thoroughly Landlock could be applied.
    pub landlock: LandlockLevel,
    /// The address-space ceiling that was installed, in bytes, or 0 where there is none.
    pub address_space_limit: u64,
    /// Whether the system-call allow-list is in force.
    pub system_calls: SystemCalls,
}

impl Confinement {
    /// Whether the kernel is enforcing the properties this crate exists to provide.
    ///
    /// True only where the system-call filter is installed, because that is the layer the
    /// no-filesystem, no-network property rests on. Landlock is defence in depth and its
    /// absence weakens rather than removes; a missing filter removes.
    #[must_use]
    pub fn is_enforced(&self) -> bool {
        self.system_calls == SystemCalls::Filtered
    }

    /// One sentence naming what is *not* enforced, or `None` where everything is.
    ///
    /// For a host with somewhere to print it. Written here rather than in the host because
    /// what a weakened confinement gives up is this crate's knowledge, not a window's.
    #[must_use]
    pub fn shortfall(&self) -> Option<String> {
        let mut missing = Vec::new();
        if self.system_calls == SystemCalls::Unfiltered {
            missing.push("no system-call filter, so the worker's filesystem and network are the process's own");
        }
        if self.address_space_limit == 0 {
            missing.push(
                "no address-space ceiling, so a decompression bomb is bounded only by this machine",
            );
        }
        if self.landlock != LandlockLevel::Enforced {
            missing.push("no Landlock domain");
        }
        if missing.is_empty() {
            return None;
        }
        Some(format!(
            "the image decoder runs in a separate process, which still contains a panic and a \
             crash, but it is not confined: {}",
            missing.join("; ")
        ))
    }

    /// The confinement of a platform that has none of the three mechanisms.
    pub(crate) const NONE: Self = Self {
        landlock: LandlockLevel::Unavailable,
        address_space_limit: 0,
        system_calls: SystemCalls::Unfiltered,
    };
}

/// Why a process could not be confined.
///
/// Every variant is Linux's, because Linux is where the mechanisms are. On another platform
/// [`apply`] installs nothing and cannot fail, which is what an enum with no reachable
/// variants says in the type system rather than in a comment.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LockdownError {
    /// A resource limit could not be installed.
    #[cfg(target_os = "linux")]
    #[error("setting {resource} failed: {source}")]
    Rlimit {
        /// Which limit.
        resource: &'static str,
        /// The system error.
        #[source]
        source: std::io::Error,
    },
    /// The Landlock ruleset could not be built, which is a programming error rather than a
    /// kernel one — an unsupported kernel yields [`LandlockLevel::Unavailable`] instead.
    #[cfg(target_os = "linux")]
    #[error("building the Landlock ruleset failed: {0}")]
    Landlock(#[from] landlock::RulesetError),
    /// This architecture has no seccomp back end in `seccompiler`.
    #[cfg(target_os = "linux")]
    #[error("no seccomp filter can be built for the {0} architecture")]
    UnknownArchitecture(String),
    /// The seccomp filter could not be compiled from the allow-list.
    #[cfg(target_os = "linux")]
    #[error("compiling the seccomp filter failed: {0}")]
    SeccompCompile(#[from] seccompiler::BackendError),
    /// The seccomp filter could not be installed.
    #[cfg(target_os = "linux")]
    #[error("installing the seccomp filter failed: {0}")]
    Seccomp(#[from] seccompiler::Error),
}

/// Whether this build has a confinement to install at all.
///
/// A property of the *build* rather than of a worker, and therefore knowable before anything
/// is spawned — which is what a host needs, because `CLAUDE.md`'s startup rules forbid starting
/// a worker to find out. `false` off Linux, where [`apply`] installs nothing.
///
/// [`Confinement::is_enforced`] is the same question asked of a worker that exists, and is the
/// one to trust: a kernel can refuse what a build offers.
pub const ENFORCED_BY_THIS_BUILD: bool = cfg!(target_os = "linux");

/// Confines the calling thread. There is no way to undo this.
///
/// # Errors
///
/// Returns [`LockdownError`] if a limit or the seccomp filter could not be installed. A
/// caller that gets an error must not continue with the work it intended to confine. On a
/// platform with no confinement to install this cannot fail, and answers
/// [`Confinement::NONE`].
///
/// `#[must_use]` because off Linux the error type has no reachable variants, so the compiler
/// would let a caller drop the whole answer — including the part that says what was *not*
/// enforced, which is the one thing this crate promises never to lose quietly.
#[must_use = "the confinement reached is what a host has to report"]
pub fn apply() -> Result<Confinement, LockdownError> {
    apply_for(Profile::Decoder)
}

/// Confines the calling thread for one kind of work. There is no way to undo this.
///
/// [`apply`] is this with [`Profile::Decoder`], which is what `pdf-sandbox-worker` wants and what
/// this crate meant by "confined" before there was a second program to confine.
///
/// # Errors
///
/// Returns [`LockdownError`] if a limit or the seccomp filter could not be installed. A caller
/// that gets an error must not continue with the work it intended to confine. On a platform with
/// no confinement to install this cannot fail, and answers [`Confinement::NONE`].
///
/// `#[must_use]` for [`apply`]'s reason: off Linux the error type has no reachable variants, so
/// the compiler would let a caller drop the part of the answer that says what was *not* enforced.
#[must_use = "the confinement reached is what a host has to report"]
pub fn apply_for(profile: Profile) -> Result<Confinement, LockdownError> {
    #[cfg(target_os = "linux")]
    {
        crate::lockdown_linux::apply(profile)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = profile;
        Ok(Confinement::NONE)
    }
}

#[cfg(test)]
mod tests {
    use super::{Confinement, LandlockLevel, SystemCalls};

    #[test]
    fn a_confinement_that_enforces_nothing_says_all_three_things() {
        let sentence = Confinement::NONE.shortfall().expect("a shortfall");
        assert!(sentence.contains("system-call filter"), "{sentence}");
        assert!(sentence.contains("address-space"), "{sentence}");
        assert!(sentence.contains("Landlock"), "{sentence}");
        assert!(!Confinement::NONE.is_enforced());
    }

    /// A worker with the filter and the ceiling is enforced whatever Landlock managed.
    ///
    /// The distinction the crate rests on: seccomp carries the no-filesystem, no-network
    /// property on its own, and Landlock is depth. So a kernel without Landlock still
    /// reports an enforced confinement, and still says what it did not get.
    #[test]
    fn landlock_is_depth_and_seccomp_is_the_property() {
        let partial = Confinement {
            landlock: LandlockLevel::Unavailable,
            address_space_limit: 1 << 30,
            system_calls: SystemCalls::Filtered,
        };
        assert!(partial.is_enforced());
        assert!(
            partial
                .shortfall()
                .is_some_and(|sentence| sentence.contains("Landlock"))
        );
    }
}
