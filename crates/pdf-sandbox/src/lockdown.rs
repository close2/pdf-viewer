//! Confining the calling process, irreversibly.
//!
//! Three mechanisms, applied in this order because each one removes the ability to apply
//! the next:
//!
//! 1. **Resource limits.** `RLIMIT_AS` bounds the address space, so a decompression bomb
//!    fails an allocation instead of taking the machine's memory. `RLIMIT_NOFILE` bounds
//!    descriptors, and `RLIMIT_FSIZE` of zero means a file that somehow got opened still
//!    cannot be written.
//! 2. **Landlock**, which makes the filesystem unreachable by path: the ruleset is created
//!    handling every access right the kernel offers and *no rules are added*, so nothing is
//!    permitted anywhere.
//! 3. **seccomp-BPF**, an allow-list of system call numbers. Anything not on it kills the
//!    process. This is the load-bearing layer: `openat` and `socket` are simply not
//!    reachable, so there is no filesystem and no network irrespective of what any path or
//!    address would have permitted.
//!
//! # Why both Landlock and seccomp
//!
//! They fail differently. seccomp is decided by system call number and cannot see
//! arguments that live in memory, so it can say "no `openat` at all" but not "`openat` only
//! under this directory". Landlock is decided by path and stays correct if a later version
//! of this worker genuinely needs one file. Today seccomp alone would be enough; the day
//! that changes is the day the second layer starts carrying weight, and adding it then
//! would be adding it after the code that needs it.
//!
//! # What is deliberately *not* required
//!
//! Landlock is best-effort and its achieved level is reported rather than demanded.
//! A kernel built without it, or booted with it out of the LSM list, is a deployment this
//! worker should still run under — because the property that matters, no filesystem and no
//! network, is enforced by seccomp either way. seccomp is not best-effort: if the filter
//! cannot be installed, [`apply`] fails and the caller must not proceed.
//!
//! # This confines a thread, not a process
//!
//! Both Landlock and seccomp attach to the calling thread and to threads it creates
//! afterwards. The seccomp filter is installed with `TSYNC` so that it reaches threads that
//! already exist, but the Landlock domain is not synchronised that way. The worker is
//! therefore single-threaded by construction, and [`apply`] is called before it does
//! anything else.

use landlock::{
    ABI, Access, AccessFs, AccessNet, CompatLevel, Compatible as _, RulesetAttr as _,
    RulesetStatus, Scope,
};
use seccompiler::{SeccompAction, SeccompFilter, TargetArch};

/// Ceiling on the worker's address space, in bytes.
///
/// A bilevel page at 600 dpi is 35 megabytes as one byte per pixel and the decoders hold a
/// handful of such planes at once, so a gigabyte is roughly thirty times the largest
/// legitimate working set. It is a bound against a hostile file, not a budget anyone should
/// ever reach: crossing it aborts the worker, which the parent reports as an undecodable
/// image.
const ADDRESS_SPACE_LIMIT: u64 = 1 << 30;

/// Ceiling on open descriptors.
///
/// The worker inherits three and opens none. Eight leaves room for the runtime to do
/// something ordinary without leaving room to do something interesting.
const DESCRIPTOR_LIMIT: u64 = 8;

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
    /// The kernel does not support Landlock, or it is not enabled.
    Unavailable,
}

/// What confinement was achieved.
///
/// Reported rather than assumed: the parent process logs it, and a test asserts the
/// strongest level on a kernel that can provide it. Everything in here that is not
/// [`LandlockLevel::Enforced`] is a weakening of defence in depth and never of the
/// no-filesystem, no-network property, which seccomp enforces unconditionally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Confinement {
    /// How thoroughly Landlock could be applied.
    pub landlock: LandlockLevel,
    /// The address-space ceiling that was installed, in bytes.
    pub address_space_limit: u64,
}

/// Why a process could not be confined.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LockdownError {
    /// A resource limit could not be installed.
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
    #[error("building the Landlock ruleset failed: {0}")]
    Landlock(#[from] landlock::RulesetError),
    /// This architecture has no seccomp back end in `seccompiler`.
    #[error("no seccomp filter can be built for the {0} architecture")]
    UnknownArchitecture(String),
    /// The seccomp filter could not be compiled from the allow-list.
    #[error("compiling the seccomp filter failed: {0}")]
    SeccompCompile(#[from] seccompiler::BackendError),
    /// The seccomp filter could not be installed.
    #[error("installing the seccomp filter failed: {0}")]
    Seccomp(#[from] seccompiler::Error),
}

/// Confines the calling thread. There is no way to undo this.
///
/// # Errors
///
/// Returns [`LockdownError`] if a limit or the seccomp filter could not be installed. A
/// caller that gets an error must not continue with the work it intended to confine.
pub fn apply() -> Result<Confinement, LockdownError> {
    limit_resources()?;
    let landlock = deny_filesystem_and_network();
    restrict_system_calls()?;
    Ok(Confinement {
        landlock,
        address_space_limit: ADDRESS_SPACE_LIMIT,
    })
}

/// Installs the resource ceilings.
fn limit_resources() -> Result<(), LockdownError> {
    use rustix::process::{Resource, Rlimit, setrlimit};

    let fixed = |value: u64| Rlimit {
        current: Some(value),
        maximum: Some(value),
    };
    for (resource, name, value) in [
        (Resource::As, "RLIMIT_AS", ADDRESS_SPACE_LIMIT),
        (Resource::Nofile, "RLIMIT_NOFILE", DESCRIPTOR_LIMIT),
        (Resource::Fsize, "RLIMIT_FSIZE", 0),
    ] {
        setrlimit(resource, fixed(value)).map_err(|error| LockdownError::Rlimit {
            resource: name,
            source: error.into(),
        })?;
    }
    Ok(())
}

/// Creates a Landlock domain that permits nothing.
///
/// A ruleset *handles* the access rights it names and then permits only what its rules
/// allow. No rules are added, so every handled right is denied everywhere. Compatibility is
/// best effort, and the level actually reached is returned rather than swallowed.
fn deny_filesystem_and_network() -> LandlockLevel {
    // The highest ABI this code was written against. Naming it explicitly rather than
    // asking the kernel keeps the request deterministic: the same binary asks for the same
    // rights everywhere, and the *answer* is what varies.
    const TARGET: ABI = ABI::V6;

    let built = landlock::Ruleset::default()
        .set_compatibility(CompatLevel::BestEffort)
        .handle_access(AccessFs::from_all(TARGET))
        .and_then(|ruleset| ruleset.handle_access(AccessNet::from_all(TARGET)))
        .and_then(|ruleset| ruleset.scope(Scope::from_all(TARGET)))
        .and_then(landlock::Ruleset::create)
        .and_then(landlock::RulesetCreated::restrict_self);

    match built {
        Ok(status) => match status.ruleset {
            RulesetStatus::FullyEnforced => LandlockLevel::Enforced,
            RulesetStatus::PartiallyEnforced => LandlockLevel::Partial,
            RulesetStatus::NotEnforced => LandlockLevel::Unavailable,
        },
        // A ruleset this crate builds from constants cannot be malformed, so an error here
        // means the kernel refused the whole mechanism. That is the same situation as
        // `NotEnforced` and is reported the same way; seccomp still runs.
        Err(_) => LandlockLevel::Unavailable,
    }
}

/// The system calls the worker is permitted to make once confined.
///
/// This list was not written from intuition. It is what a worker actually issues, found by
/// running it under `strace` and adding what appeared, and every entry has a reason:
///
/// - `read`, `write`, `close` — the request and response pipes, and stderr for diagnostics.
/// - `mmap`, `munmap`, `mremap`, `mprotect`, `brk`, `madvise` — the allocator. A decoder
///   allocates plane buffers per image, so these are hot rather than incidental.
/// - `futex`, `sched_yield` — lock and allocator arbitration inside the runtime.
/// - `exit`, `exit_group`, `rt_sigreturn`, `restart_syscall` — leaving, and returning from
///   a signal handler.
/// - `rt_sigaction`, `rt_sigprocmask`, `sigaltstack`, `getpid`, `gettid`, `tgkill` — the
///   abort path. Without these a failed allocation would die by `SIGSYS` from this very
///   filter, which reports the wrong cause; with them it dies by `SIGABRT` and the parent
///   can say what happened.
/// - `getrandom` — the standard library seeds a hash map's keys from it on first use.
/// - `clock_gettime` — usually served from the vDSO without a system call at all, but not
///   on every machine, and a decoder that measures its own progress must not die for it.
///
/// Notably absent: `openat`, `socket`, `connect`, `execve`, `clone`, `ptrace`, `prctl`,
/// `ioctl`. There is no path from decoding an image to any of them.
/// The type is `i64` because that is what `seccompiler` keys a filter by. On a 32-bit
/// target `libc::SYS_*` is an `i32` and this will not compile — which is the right outcome,
/// because the rest of the list would need reviewing for that architecture anyway.
const PERMITTED: &[i64] = &[
    libc::SYS_read,
    libc::SYS_write,
    libc::SYS_close,
    libc::SYS_mmap,
    libc::SYS_munmap,
    libc::SYS_mremap,
    libc::SYS_mprotect,
    libc::SYS_brk,
    libc::SYS_madvise,
    libc::SYS_futex,
    libc::SYS_sched_yield,
    libc::SYS_exit,
    libc::SYS_exit_group,
    libc::SYS_rt_sigreturn,
    libc::SYS_restart_syscall,
    libc::SYS_rt_sigaction,
    libc::SYS_rt_sigprocmask,
    libc::SYS_sigaltstack,
    libc::SYS_getpid,
    libc::SYS_gettid,
    libc::SYS_tgkill,
    libc::SYS_getrandom,
    libc::SYS_clock_gettime,
];

/// Installs the seccomp-BPF allow-list.
///
/// The mismatch action is `KillProcess` rather than `Errno`: a worker that reaches a
/// forbidden call is a worker doing something no image decode requires, and the loud answer
/// is the one that reaches the parent as an unmistakable signal rather than as an error
/// return the code might paper over. It also means an exploit's first step, not its tenth,
/// is what ends the process.
fn restrict_system_calls() -> Result<(), LockdownError> {
    let architecture = TargetArch::try_from(std::env::consts::ARCH)
        .map_err(|_| LockdownError::UnknownArchitecture(std::env::consts::ARCH.to_owned()))?;

    // An empty rule vector means "this call, unconditionally". Conditions exist for
    // narrowing by argument; nothing here needs one, because every permitted call is
    // permitted for every argument the worker could pass.
    let rules = PERMITTED
        .iter()
        .map(|number| (*number, Vec::new()))
        .collect();

    let filter = SeccompFilter::new(
        rules,
        SeccompAction::KillProcess,
        SeccompAction::Allow,
        architecture,
    )?;
    let program: seccompiler::BpfProgram = filter.try_into()?;
    // `TSYNC`, so that a process which already has threads is covered rather than only the
    // thread that asked. The worker has none; a future caller that confines itself later
    // would, and the surprising version of this function is the one that quietly protects a
    // single thread.
    seccompiler::apply_filter_all_threads(&program)?;
    Ok(())
}
