//! Confining the calling process, irreversibly — and saying so where a platform cannot.
//!
//! Two platforms have a mechanism. Linux's lives in `crate::lockdown_linux`: resource limits, a
//! Landlock domain that permits nothing, and a seccomp-BPF allow-list. macOS's lives in
//! `crate::lockdown_macos`: a Seatbelt profile that denies every operation it does not name, the
//! filesystem and the network included, with the memory ceiling held from outside by the process
//! that started the worker (`crate::watch`, ADR 2864). This module is the front door: the
//! vocabulary for *what was achieved*, and [`apply`], which on a platform with neither installs
//! nothing and returns a [`Confinement`] that says so.
//!
//! # Why a platform without a filter still gets a worker
//!
//! Decided by the project owner (ADR 0194), and it is a narrowing of principle 3 rather than an
//! abandonment of it; since ADR 2864 the platform it describes is Windows alone. The crate's own
//! documentation
//! lists three reasons the boundary exists, and only the first two are about the isolation
//! being *enforced by the kernel*:
//!
//! - **Resource exhaustion** — `RLIMIT_AS` is Linux's here, and macOS's is the starting
//!   process's watch. Where neither exists the ceiling is gone, and
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
/// found by running that work under `strace` and reading what appeared. So there are three, each
/// named for a program in this workspace, and the third was measured as the first two were (ADR
/// 1608).
///
/// **The difference between the first two is more than threads and address space** (ADR 0218):
/// ADR 0812 adds `recvmsg` and `pread64`, which are neither, and ADR 0888 the interpreter's one
/// argument-narrowed rule. What they have in common is the honest
/// heading and is why the profile is not a set of options: every one of them follows from the
/// second profile being handed **a descriptor** and being asked to draw a page with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Profile {
    /// One image, decoded on one thread: `pdf-sandbox-worker`.
    ///
    /// The narrower of the two profiles that decode or draw — [`Profile::Script`] is narrower
    /// still, sized against the one engine it confines — and the default because a caller who has
    /// not thought about it should get the list that permits less. It creates no thread, so
    /// `clone` is absent.
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
    /// - **The document's descriptor**, which ADR 0812 hands across: `recvmsg` is how it arrives
    ///   and `pread64` is how it is read, and since ADR 0888 `fcntl` **narrowed by argument to
    ///   `F_GETFD`** is how it is given back — the standard library asks that question inside
    ///   `OwnedFd::drop`, so a worker forbidden it can be handed a descriptor and can never
    ///   close one. It is the only conditional rule of its list, and
    ///   `lockdown_linux::PERMITTED_INTERPRETER_NARROWED` is what it costs and what it does not.
    Interpreter,
    /// A document's scripts, run by an ECMAScript engine on one thread: `pdf-script-worker`.
    ///
    /// The narrowest of the three, because what it runs is the most programmable input this tree
    /// reads and it needs the least to run it (ADR 1608). **No thread**: `clone`, `clone3` and the
    /// per-thread registrations are absent. **No descriptor**: it reads its frames with `read`, so
    /// a descriptor sent beside one is closed by the kernel rather than received — `recvmsg`,
    /// `pread64`, `fcntl` and even `close` are absent — and `RLIMIT_NOFILE` is zero, so none can be
    /// made either. **No executable memory**: `mmap` is admitted only where the protection it asks
    /// for leaves out `PROT_EXEC`, and `mprotect` not at all, so the heap grows and nothing in it can
    /// be run. And an address-space ceiling in the tens of megabytes, sized against the engine's own
    /// construction and the budgets one run is held to (ADR 1609).
    Script,
}

/// Ceiling on a decoder's address space, in bytes.
///
/// A bilevel page at 600 dpi is 35 megabytes as one byte per pixel and the decoders hold a
/// handful of such planes at once, so a gigabyte is roughly thirty times the largest
/// legitimate working set. It is a bound against a hostile file, not a budget anyone should
/// ever reach: crossing it aborts the worker, which the parent reports as an undecodable
/// image.
#[cfg(any(target_os = "linux", target_os = "macos"))]
const ADDRESS_SPACE_LIMIT: u64 = 1 << 30;

/// Ceiling on an interpreter's address space, in bytes.
///
/// Four times the decoder's, and the multiplier is derived rather than chosen. A rasteriser is
/// asked for a whole page at a whole magnification, and `viewer_core`'s own `MAX_PIXELS` bounds
/// one at 2²⁸ pixels — which is [`ADDRESS_SPACE_LIMIT`] exactly, in RGBA, for the raster alone.
/// Beside it live the document's bytes, the display list, the glyph outlines and a rasteriser's
/// scratch sheet on every core, so a ceiling equal to the pixel budget would refuse pages the
/// viewer permits. Four gibibytes leaves room for all of that and is still a bound: a document
/// that reaches it has asked for more than any page needs.
#[cfg(any(target_os = "linux", target_os = "macos"))]
const INTERPRETER_ADDRESS_SPACE_LIMIT: u64 = 4 << 30;

/// Ceiling on a script worker's address space, in bytes.
///
/// Sized from measurements rather than chosen (ADR 1609). What the worker may hold whatever it runs
/// comes to about 43 MiB in a debug build: its image at start (17.6 MiB; 12.4 in release) and the
/// engine one run constructs (0.4), as measured, and three bounds of the worker's own — the frames
/// of one run and their decoded copies (8), the scripts it holds (8) and a main-thread stack grown
/// to its 8 MiB. Beside that sits what a script makes:
/// the largest allocation `pdf_script::Budget` admits at face value, a 16 MiB `ArrayBuffer`, and
/// as much again for the script's own objects. Ninety-six mebibytes holds both. A call whose
/// transient cost is many times the size its budget names — `padStart` at the string budget peaked
/// 96 MiB above start, `join` at the element budget 150 — and growth through an operator, which no
/// per-call budget sees, meet it here instead, and the worker aborts with the allocator's sentence
/// on the pipe its host reads.
#[cfg(any(target_os = "linux", target_os = "macos"))]
const SCRIPT_ADDRESS_SPACE_LIMIT: u64 = 96 << 20;

/// The ceiling a profile's process is held to, in bytes: the profile's own, or `wider` where a
/// caller sized one for a stated budget and it is the larger (`apply_for_decoder_within`).
///
/// One function for both platforms, because the number is a property of the *work* and not of the
/// mechanism that holds a process to it: Linux installs it as `RLIMIT_AS` inside the worker, macOS
/// has no ceiling a process can set on itself and is held to it by the process that started it
/// (`crate::watch`, ADR 2864).
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) fn ceiling(profile: Profile, wider: Option<u64>) -> u64 {
    let own = match profile {
        Profile::Decoder => ADDRESS_SPACE_LIMIT,
        Profile::Interpreter => INTERPRETER_ADDRESS_SPACE_LIMIT,
        Profile::Script => SCRIPT_ADDRESS_SPACE_LIMIT,
    };
    wider.map_or(own, |wider| wider.max(own))
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
    /// A Seatbelt profile is in force: every operation it does not name is refused by the kernel.
    ///
    /// macOS's mechanism (ADR 2864). It is decided per *operation* — reading a file, opening a
    /// socket, looking up a service — rather than per system-call number, so it says nothing about
    /// threads or executable memory, which seccomp can refuse and a profile cannot; what it carries
    /// is the property this crate exists for, no filesystem and no network, and the kernel answers
    /// a refused operation with `EPERM` rather than killing the process.
    Seatbelt,
    /// No filter. The worker may call anything the operating system offers it.
    ///
    /// On Windows, where neither seccomp-BPF nor Seatbelt exists. What remains is the process boundary
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
    /// The memory ceiling the worker is held to, in bytes, or 0 where there is none.
    ///
    /// On Linux the kernel holds it inside the worker as `RLIMIT_AS`, and an allocation past it
    /// fails. On macOS no process can set one on itself, so the process that started the worker
    /// samples its physical footprint and kills it past this number (`crate::watch`, ADR 2864):
    /// the same bound, held from outside and a sampling interval late.
    pub address_space_limit: u64,
    /// Whether the system-call allow-list is in force.
    pub system_calls: SystemCalls,
}

impl Confinement {
    /// Whether the kernel is enforcing the properties this crate exists to provide.
    ///
    /// True only where the system-call filter or the Seatbelt profile is installed, because that
    /// is the layer the no-filesystem, no-network property rests on. Landlock is defence in depth
    /// and its absence weakens rather than removes; a missing filter removes.
    #[must_use]
    pub fn is_enforced(&self) -> bool {
        matches!(
            self.system_calls,
            SystemCalls::Filtered | SystemCalls::Seatbelt
        )
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
        // Landlock is a Linux mechanism and a layer *under* the filter: its absence is worth a
        // word where seccomp is the filter, and none where Seatbelt is, which has no such layer.
        if self.system_calls != SystemCalls::Seatbelt && self.landlock != LandlockLevel::Enforced {
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

    /// The confinement of a platform that has no mechanism: Windows.
    pub(crate) const NONE: Self = Self {
        landlock: LandlockLevel::Unavailable,
        address_space_limit: 0,
        system_calls: SystemCalls::Unfiltered,
    };
}

/// Why a process could not be confined.
///
/// Every variant is Linux's or macOS's, because those are where the mechanisms are. On Windows
/// [`apply`] installs nothing and cannot fail, which is what an enum with no reachable variants
/// says in the type system rather than in a comment.
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
    /// The Seatbelt profile was not accepted.
    #[cfg(target_os = "macos")]
    #[error("entering the Seatbelt profile failed: {0}")]
    Seatbelt(#[from] darwin_sandbox::EnterError),
}

/// Whether this build has a confinement to install at all.
///
/// A property of the *build* rather than of a worker, and therefore knowable before anything
/// is spawned — which is what a host needs, because `CLAUDE.md`'s startup rules forbid starting
/// a worker to find out. `false` on Windows, where [`apply`] installs nothing.
///
/// [`Confinement::is_enforced`] is the same question asked of a worker that exists, and is the
/// one to trust: a kernel can refuse what a build offers.
pub const ENFORCED_BY_THIS_BUILD: bool = cfg!(any(target_os = "linux", target_os = "macos"));

/// Confines the calling thread. There is no way to undo this.
///
/// # Errors
///
/// Returns [`LockdownError`] if a limit or the seccomp filter could not be installed. A
/// caller that gets an error must not continue with the work it intended to confine. On a
/// platform with no confinement to install this cannot fail, and answers
/// `Confinement::NONE`.
///
/// `#[must_use]` because on Windows the error type has no reachable variants, so the compiler
/// would let a caller drop the whole answer — including the part that says what was *not*
/// enforced, which is the one thing this crate promises never to lose quietly.
#[must_use = "the confinement reached is what a host has to report"]
pub fn apply() -> Result<Confinement, LockdownError> {
    apply_for(Profile::Decoder)
}

/// Whether this process has installed its system-call filter or entered its Seatbelt profile.
///
/// Set once, by the confinement, and never cleared — neither can be removed. Read by
/// [`crate::decode()`], whose in-process deadline starts a thread: a confined process must not
/// take that path, because the filter kills it (ADR 1447). A Seatbelt profile would let the thread
/// start, and takes the same path anyway, so that a confined process decodes the same way on both
/// platforms and its deadline is the one its starting process holds.
pub(crate) static CONFINED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Whether this process is behind its own system-call filter or Seatbelt profile ([`apply_for`]
/// has succeeded).
#[must_use]
pub fn is_confined() -> bool {
    CONFINED.load(std::sync::atomic::Ordering::Acquire)
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
/// no confinement to install this cannot fail, and answers `Confinement::NONE`.
///
/// `#[must_use]` for [`apply`]'s reason: on Windows the error type has no reachable variants, so
/// the compiler would let a caller drop the part of the answer that says what was *not* enforced.
#[must_use = "the confinement reached is what a host has to report"]
pub fn apply_for(profile: Profile) -> Result<Confinement, LockdownError> {
    #[cfg(target_os = "linux")]
    {
        crate::lockdown_linux::apply(profile, None)
    }
    #[cfg(target_os = "macos")]
    {
        crate::lockdown_macos::apply(profile, None)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = profile;
        Ok(Confinement::NONE)
    }
}

/// Confines a decoder whose address-space ceiling is `address_space` bytes rather than the
/// profile's own, never lower than it.
///
/// The one caller is a worker started for a writer's full-resolution decodes
/// ([`crate::Sandbox::whole`]): the ceiling is sized from the sample budget the operator
/// stated, so the bound is still explicit and still the kernel's (ADR 1333). The system-call
/// list is [`Profile::Decoder`]'s unchanged.
///
/// # Errors
///
/// As [`apply_for`].
#[must_use = "the confinement reached is what a host has to report"]
pub(crate) fn apply_for_decoder_within(address_space: u64) -> Result<Confinement, LockdownError> {
    #[cfg(target_os = "linux")]
    {
        crate::lockdown_linux::apply(Profile::Decoder, Some(address_space))
    }
    #[cfg(target_os = "macos")]
    {
        crate::lockdown_macos::apply(Profile::Decoder, Some(address_space))
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = address_space;
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

    /// A worker in a Seatbelt profile with its ceiling is enforced and owes no sentence: Landlock
    /// is a layer under seccomp, and a platform with neither has nothing of it to report missing.
    #[test]
    fn seatbelt_with_a_ceiling_is_enforced_and_says_nothing() {
        let seatbelt = Confinement {
            landlock: LandlockLevel::Unavailable,
            address_space_limit: 1 << 30,
            system_calls: SystemCalls::Seatbelt,
        };
        assert!(seatbelt.is_enforced());
        assert_eq!(seatbelt.shortfall(), None);
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
