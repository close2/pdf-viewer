//! The Linux mechanism: resource limits, a Landlock domain, and a seccomp-BPF allow-list.
//!
//! [`crate::lockdown`] is the front door and holds the vocabulary; this is what it calls where
//! the three interfaces exist. Everything here was written against Linux and is compiled only
//! there — the dependencies are Linux-only in `Cargo.toml` for the same reason.
//!
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
//! already exist, but the Landlock domain is not synchronised that way. So the rule for every
//! caller is the same one: **[`apply`] runs before any thread is made**, and a thread made
//! afterwards inherits both.
//!
//! The decoder worker keeps that trivially by being single-threaded. The interpreter worker
//! ([`Profile::Interpreter`]) does make threads — `render-cpu` draws a page on every core — and
//! keeps it because `rayon`'s pool is built on its first use, which is inside a render, which is
//! after the confinement. A caller that warmed a thread pool first would have threads outside
//! the Landlock domain and inside the seccomp filter, which is the weaker of the two arrangements
//! and not one this crate offers.

use crate::lockdown::{Confinement, LandlockLevel, LockdownError, Profile, SystemCalls};
use landlock::{
    ABI, Access, AccessFs, AccessNet, CompatLevel, Compatible as _, RulesetAttr as _,
    RulesetStatus, Scope,
};
use seccompiler::{SeccompAction, SeccompFilter, TargetArch};

/// Ceiling on a decoder's address space, in bytes.
///
/// A bilevel page at 600 dpi is 35 megabytes as one byte per pixel and the decoders hold a
/// handful of such planes at once, so a gigabyte is roughly thirty times the largest
/// legitimate working set. It is a bound against a hostile file, not a budget anyone should
/// ever reach: crossing it aborts the worker, which the parent reports as an undecodable
/// image.
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
const INTERPRETER_ADDRESS_SPACE_LIMIT: u64 = 4 << 30;

/// Ceiling on open descriptors.
///
/// The worker inherits three and opens none. Eight leaves room for the runtime to do
/// something ordinary without leaving room to do something interesting.
///
/// **The interpreter is also *handed* one per open document** (ADR 0812): the document arrives
/// as a descriptor beside `Command::Open`, which the kernel enters into the worker's table
/// under this same ceiling — so five documents may be open in a confined viewer at once, and a
/// sixth's descriptor is dropped by the kernel with `MSG_CTRUNC` set, which the worker reads
/// as a refusal by name rather than as a document. The number is not raised for it: a
/// document that is closed gives its descriptor back, and a viewer holding six documents in
/// one confined process is not a shape any host on that boundary has.
const DESCRIPTOR_LIMIT: u64 = 8;

/// Confines the calling thread. There is no way to undo this.
///
/// # Errors
///
/// Returns [`LockdownError`] if a limit or the seccomp filter could not be installed. A
/// caller that gets an error must not continue with the work it intended to confine.
pub(crate) fn apply(profile: Profile) -> Result<Confinement, LockdownError> {
    let address_space_limit = match profile {
        Profile::Decoder => ADDRESS_SPACE_LIMIT,
        Profile::Interpreter => INTERPRETER_ADDRESS_SPACE_LIMIT,
    };
    limit_resources(address_space_limit)?;
    let landlock = deny_filesystem_and_network();
    restrict_system_calls(profile)?;
    Ok(Confinement {
        landlock,
        address_space_limit,
        system_calls: SystemCalls::Filtered,
    })
}

/// Installs the resource ceilings.
fn limit_resources(address_space_limit: u64) -> Result<(), LockdownError> {
    use rustix::process::{Resource, Rlimit, setrlimit};

    let fixed = |value: u64| Rlimit {
        current: Some(value),
        maximum: Some(value),
    };
    for (resource, name, value) in [
        (Resource::As, "RLIMIT_AS", address_space_limit),
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

/// What an interpreter is permitted beyond [`PERMITTED`].
///
/// Found the same way that list was: `strace -f -c` over a real page of a real document
/// interpreted and rasterised — `pdf-model`'s `render_at` example on `doc/PDF20_AN001-BPC.pdf`
/// — and every call it issued after process start is either here or above. The first four
/// entries are one fact: **`render-cpu` draws a page on every core**.
///
/// - `clone3` — the thread. `clone` is beside it because `glibc` falls back to it on a kernel
///   that answers `ENOSYS`, and a worker that died on such a kernel would look like a defect in
///   this program rather than in the list.
/// - `rseq`, `set_robust_list` — per-thread registrations `glibc` performs inside a new thread
///   before it runs anything of ours.
/// - `sched_getaffinity` — `std::thread::available_parallelism`, which is how the pool learns
///   how many threads to make.
///
/// **And two more since the document stopped crossing as bytes** (ADR 0812), each on the
/// same evidence — `strace -f` over `pdf-view-worker` opening a document handed to it as a
/// descriptor — and each a fact about a descriptor the worker already holds rather than about
/// anything it could reach:
///
/// - `recvmsg` — how the descriptor arrives. The host makes the socket pair and gives the
///   worker one end as its standard input, so the worker reads its frames from a socket it did
///   not create and cannot create: `socket`, `socketpair`, `connect`, `bind` and `accept` stay
///   off the list, and `a_confined_interpreter_cannot_reach_the_network` still holds. What
///   `recvmsg` admits over `read` is exactly the ancillary data — a descriptor the host chose
///   to send — on that one socket.
/// - `pread64` — how the document is read. `pdf_syntax::FileBytes` reads a file on disk where
///   its offsets point, through `FileExt::read_at`, which is one positional read and moves no
///   cursor. It reads a descriptor the worker holds; on the three it inherits, a pipe and a
///   socket, it is `ESPIPE`. **Not `fstat`, `statx`, `lseek` or `openat`**: the file's length
///   crosses beside the descriptor rather than being asked for, because `statx` takes a path
///   and admitting it would let a confined process ask whether `/etc/passwd` exists, and
///   `std::fs::File::read_to_end` asks both `statx` and `lseek` — which is why every read the
///   worker makes goes through `pdf_syntax`'s own positional reader and none through
///   `File::read`. `a_confined_interpreter_cannot_stat_a_descriptor_it_holds` pins that shape.
///
/// **What is deliberately still absent is what makes this a thread and not a program**:
/// `execve`, `execveat`, `fork` and `vfork` are on no list here, so nothing confined can start
/// anything. That is also why the interpreter decodes its own images rather than spawning
/// `pdf-sandbox-worker` for them (ADR 0218) — it could not, and a filter that let it would have
/// given the confined process the one capability the confinement is for.
const PERMITTED_INTERPRETER_EXTRA: &[i64] = &[
    libc::SYS_clone3,
    libc::SYS_clone,
    libc::SYS_rseq,
    libc::SYS_set_robust_list,
    libc::SYS_sched_getaffinity,
    libc::SYS_recvmsg,
    libc::SYS_pread64,
];

/// Installs the seccomp-BPF allow-list.
///
/// The mismatch action is `KillProcess` rather than `Errno`: a worker that reaches a
/// forbidden call is a worker doing something no image decode requires, and the loud answer
/// is the one that reaches the parent as an unmistakable signal rather than as an error
/// return the code might paper over. It also means an exploit's first step, not its tenth,
/// is what ends the process.
fn restrict_system_calls(profile: Profile) -> Result<(), LockdownError> {
    let architecture = TargetArch::try_from(std::env::consts::ARCH)
        .map_err(|_| LockdownError::UnknownArchitecture(std::env::consts::ARCH.to_owned()))?;

    let extra: &[i64] = match profile {
        Profile::Decoder => &[],
        Profile::Interpreter => PERMITTED_INTERPRETER_EXTRA,
    };
    // An empty rule vector means "this call, unconditionally". Conditions exist for
    // narrowing by argument; nothing here needs one, because every permitted call is
    // permitted for every argument the worker could pass.
    let rules = PERMITTED
        .iter()
        .chain(extra)
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
