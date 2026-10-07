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
//!    fails an allocation instead of taking the machine's memory, and `RLIMIT_FSIZE` of zero
//!    means a file that somehow got opened still cannot be written.
//! 2. **Landlock**, which makes the filesystem unreachable by path: the ruleset is created
//!    handling every access right the kernel offers and *no rules are added*, so nothing is
//!    permitted anywhere. Then `RLIMIT_NOFILE`, which bounds descriptors, and comes after
//!    Landlock because the ruleset is itself a descriptor and the script worker's ceiling is
//!    none (ADR 1608).
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
//! The decoder worker and the script worker keep that trivially by being single-threaded — the
//! script worker's filter does not even admit a thread. The interpreter worker
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
use seccompiler::{
    SeccompAction, SeccompCmpArgLen, SeccompCmpOp, SeccompCondition, SeccompFilter, SeccompRule,
    TargetArch,
};

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
const SCRIPT_ADDRESS_SPACE_LIMIT: u64 = 96 << 20;

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

/// Ceiling on a script worker's descriptors: none beyond the three it inherits, which this limit
/// does not close.
///
/// `RLIMIT_NOFILE` bounds the number a *new* descriptor may take, and every one the kernel would
/// hand out is at least zero, so a ceiling of zero refuses every new descriptor whatever the
/// process tries — the filter's kill for the calls that make one, and this for any the filter
/// would ever let through (ADR 1608).
const SCRIPT_DESCRIPTOR_LIMIT: u64 = 0;

/// Confines the calling thread. There is no way to undo this.
///
/// # Errors
///
/// Returns [`LockdownError`] if a limit or the seccomp filter could not be installed. A
/// caller that gets an error must not continue with the work it intended to confine.
/// `wider` is a ceiling a caller sized for a stated budget, which is taken only where it is above
/// the profile's own (`crate::lockdown::apply_for_decoder_within`).
pub(crate) fn apply(profile: Profile, wider: Option<u64>) -> Result<Confinement, LockdownError> {
    let (own, descriptors) = match profile {
        Profile::Decoder => (ADDRESS_SPACE_LIMIT, DESCRIPTOR_LIMIT),
        Profile::Interpreter => (INTERPRETER_ADDRESS_SPACE_LIMIT, DESCRIPTOR_LIMIT),
        Profile::Script => (SCRIPT_ADDRESS_SPACE_LIMIT, SCRIPT_DESCRIPTOR_LIMIT),
    };
    let address_space_limit = wider.map_or(own, |wider| wider.max(own));
    limit_resources(address_space_limit)?;
    let landlock = deny_filesystem_and_network();
    // After Landlock, because building its domain takes a descriptor — the ruleset is one — and a
    // script worker's ceiling of none would have refused it (ADR 1608).
    limit_descriptors(descriptors)?;
    restrict_system_calls(profile)?;
    crate::lockdown::CONFINED.store(true, std::sync::atomic::Ordering::Release);
    Ok(Confinement {
        landlock,
        address_space_limit,
        system_calls: SystemCalls::Filtered,
    })
}

/// Installs the address-space and file-size ceilings.
fn limit_resources(address_space_limit: u64) -> Result<(), LockdownError> {
    for (resource, name, value) in [
        (
            rustix::process::Resource::As,
            "RLIMIT_AS",
            address_space_limit,
        ),
        (rustix::process::Resource::Fsize, "RLIMIT_FSIZE", 0),
    ] {
        fix_limit(resource, name, value)?;
    }
    Ok(())
}

/// Installs the descriptor ceiling.
fn limit_descriptors(descriptors: u64) -> Result<(), LockdownError> {
    fix_limit(
        rustix::process::Resource::Nofile,
        "RLIMIT_NOFILE",
        descriptors,
    )
}

/// Sets one resource limit's soft and hard values to `value`, so that neither can be raised.
fn fix_limit(
    resource: rustix::process::Resource,
    name: &'static str,
    value: u64,
) -> Result<(), LockdownError> {
    let fixed = rustix::process::Rlimit {
        current: Some(value),
        maximum: Some(value),
    };
    rustix::process::setrlimit(resource, fixed).map_err(|error| LockdownError::Rlimit {
        resource: name,
        source: error.into(),
    })
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
/// `ioctl`, `fcntl`. There is no path from decoding an image to any of them — the last of those
/// because a decoder is handed no descriptor and so never gives one back, which is what
/// [`PERMITTED_INTERPRETER_NARROWED`] is about and why it is not on this list.
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

/// The one system call an interpreter may make **for a single value of a single argument**, and
/// the only entry in this file that is narrower than a number.
///
/// `fcntl(fd, F_GETFD)`, and nothing else `fcntl` can do. It is here because of what an
/// interpreter is *given*: ADR 0812 hands the worker a descriptor per document opened on disk, and
/// `Command::Close` drops it — `std::os::fd::OwnedFd::drop` asks `fcntl(fd, F_GETFD)` before
/// `close`, under `core::ub_checks::check_library_ub()`, to catch a double close. There is no way
/// to close a descriptor from safe Rust without that question being asked, so a worker that is
/// handed a descriptor and forbidden `fcntl` can never give it back. ADR 0880 section 6 found it
/// and declined to fix it (trap 32); ADR 0888 is the decision and prices what else was
/// available. In short: the two alternatives both **leak**, and leaking is arithmetic rather than
/// taste — [`DESCRIPTOR_LIMIT`] is 8, three are inherited, so the fifth document a viewer opens
/// and closes would be the last it could open at all.
///
/// **What admitting it costs, stated rather than asserted.** `F_GETFD` reads the close-on-exec
/// flag of a descriptor the process already holds. It takes no path, opens nothing, creates
/// nothing, and changes nothing; the most a hostile interpreter learns from it is which of its
/// eight descriptor numbers are open, which `close` and `read` already tell it. Every other
/// command of the same call is refused with the same kill as before — `F_SETFD` cannot clear
/// close-on-exec (which would matter if `execve` were reachable, and it is not), `F_DUPFD` and
/// `F_DUPFD_CLOEXEC` cannot manufacture a descriptor, `F_SETFL` cannot turn a blocking read into a
/// spin, and `F_GETFL`, the locking commands and `F_ADD_SEALS` are all as absent as they were.
/// `a_confined_interpreter_can_close_a_descriptor_it_was_handed` and
/// `a_confined_interpreter_cannot_set_a_descriptors_flags` are the two directions, and
/// `pdf-sandbox`'s own `a_confined_decoder_cannot_ask_about_a_descriptor_at_all` is the third: the
/// **decoder** list did not move, because a decoder is handed no descriptor and needs none.
///
/// This is `doc/todo/61`'s rule surviving rather than bending. That item forbids answering an
/// *environment probe* with a permission, and this is not one: nothing is being asked about the
/// machine, and the fix is not "the worker wanted something, so the filter grew". It is the
/// standard library checking a precondition on a resource the worker legitimately owns, at the
/// one moment it gives that resource back.
const PERMITTED_INTERPRETER_NARROWED: i64 = libc::SYS_fcntl;

/// Which argument of [`PERMITTED_INTERPRETER_NARROWED`] the condition reads, counted from zero.
///
/// `fcntl(int fd, int cmd, ...)`, so the command is argument 1.
const FCNTL_COMMAND_ARGUMENT: u8 = 1;

/// `F_GETFD`, the only command [`PERMITTED_INTERPRETER_NARROWED`] permits.
///
/// A literal rather than a cast of `libc::F_GETFD`, because a security-relevant constant should
/// not depend on a sign conversion to be right; the assertion below is what keeps the two equal,
/// and it is a compile error rather than a test if they ever differ.
const F_GETFD_COMMAND: u64 = 1;
const _: () = assert!(
    libc::F_GETFD == 1,
    "F_GETFD_COMMAND is the value this platform's F_GETFD has"
);

/// What a script worker is permitted, in place of [`PERMITTED`] rather than beside it.
///
/// Found as the other two lists were: `strace -ff` over `pdf-script-worker`, debug and release,
/// running `crates/pdf-script-worker/tests/end_to_end.rs` — the engine's library from `Math.random`
/// to `JSON`, `RegExp`, typed arrays and `AF*` calls, a run the wall budget stops, one the deadline
/// kills, one the ceiling aborts and one that overflows the parser's stack — and every call it
/// issued after its filter was installed is here (ADR 1608):
///
/// - `read`, `write` — its frames, and its standard error, which is a pipe the host reads.
/// - `brk`, `munmap`, `mremap` — the allocator, which on one thread keeps one arena and grows it
///   with `brk`; and `mmap`, which is not on this list but under
///   [`PERMITTED_SCRIPT_UNLESS_EXECUTABLE`]'s condition.
/// - `madvise` — not issued by the traces, and kept because `glibc` reaches it on the allocator's
///   own pages under a `GLIBC_TUNABLES` the host's environment passes through; it touches nothing
///   but memory this process already holds.
/// - `getrandom` — `Math.random`'s seed.
/// - `clock_gettime` — the wall budget's clock between slices; the vDSO's, so absent from the
///   traces, and kept for a machine without one, as the decoder keeps it.
/// - `exit_group` — leaving when the host closes its end.
/// - `sigaltstack` — the standard library's teardown on that clean exit, which disables the
///   alternate signal stack it installed for reporting an overflowed stack and unmaps it; without
///   it a worker whose host had closed its end died by `SIGSYS` on the way out, its last act a core
///   dump (ADR 1627). It changes where this process's own signal handler runs and reaches nothing
///   outside it.
/// - `rt_sigprocmask`, `rt_sigaction`, `rt_sigreturn`, `getpid`, `gettid`, `tgkill` — the abort
///   path: the address-space ceiling's failed allocation and the standard library's report of an
///   overflowed stack both end in `abort`, and without these the process would die by `SIGSYS` from
///   this filter instead, naming the wrong cause (trap 18).
///
/// **Absent, and each absence is the profile**: `clone`, `clone3` and every per-thread call — one
/// thread; `recvmsg`, `pread64`, `fcntl` and even `close` — no descriptor is received, read or let
/// go, because the worker reads its frames with `read` and holds only the three it was started
/// with; `mprotect` — the single arena never changes a mapping's protection, so nothing can be made
/// executable after it was written; `futex` and `sched_yield` — nothing to contend with; and, as
/// for every profile, `openat`, `socket`, `execve` and the rest.
const PERMITTED_SCRIPT: &[i64] = &[
    libc::SYS_read,
    libc::SYS_write,
    libc::SYS_brk,
    libc::SYS_munmap,
    libc::SYS_mremap,
    libc::SYS_madvise,
    libc::SYS_getrandom,
    libc::SYS_clock_gettime,
    libc::SYS_exit_group,
    libc::SYS_sigaltstack,
    libc::SYS_rt_sigprocmask,
    libc::SYS_rt_sigaction,
    libc::SYS_rt_sigreturn,
    libc::SYS_getpid,
    libc::SYS_gettid,
    libc::SYS_tgkill,
];

/// The one call a script worker may make **only where the protection asked for leaves out
/// `PROT_EXEC`**: `mmap`, whose protection is argument 2.
///
/// The allocator maps every block past its threshold, and never needs the mapping executable: the
/// engine is an interpreter and compiles nothing to machine code. So the condition is a mask rather
/// than a value — any protection whose `PROT_EXEC` bit is clear is admitted, and one with it set is
/// the filter's kill. With `mprotect` absent, that closes both routes from a corrupted heap to code
/// of the attacker's own: no new mapping is executable and no written one can be made so (ADR
/// 1608).
const PERMITTED_SCRIPT_UNLESS_EXECUTABLE: i64 = libc::SYS_mmap;

/// Which argument of [`PERMITTED_SCRIPT_UNLESS_EXECUTABLE`] holds the protection.
///
/// `mmap(addr, length, prot, flags, fd, offset)`.
const PROTECTION_ARGUMENT: u8 = 2;

/// `PROT_EXEC`, the bit [`PERMITTED_SCRIPT_UNLESS_EXECUTABLE`] refuses.
///
/// A literal for [`F_GETFD_COMMAND`]'s reason, held to the platform's by the assertion beneath.
const PROT_EXEC_BIT: u64 = 4;
const _: () = assert!(
    libc::PROT_EXEC == 4,
    "PROT_EXEC_BIT is the value this platform's PROT_EXEC has"
);

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

    let (base, extra): (&[i64], &[i64]) = match profile {
        Profile::Decoder => (PERMITTED, &[]),
        Profile::Interpreter => (PERMITTED, PERMITTED_INTERPRETER_EXTRA),
        Profile::Script => (PERMITTED_SCRIPT, &[]),
    };
    // An empty rule vector means "this call, unconditionally", which is what every entry of
    // these lists is: each is permitted for every argument the worker could pass.
    let mut rules: std::collections::BTreeMap<i64, Vec<SeccompRule>> = base
        .iter()
        .chain(extra)
        .map(|number| (*number, Vec::new()))
        .collect();
    if profile == Profile::Script {
        // [`PERMITTED_SCRIPT_UNLESS_EXECUTABLE`]'s condition: the protection masked by
        // `PROT_EXEC` is zero. `Dword` for the reason the `fcntl` rule below gives: the kernel
        // reads `prot` as flags in its low 32 bits, and the executable bit is among them.
        rules.insert(
            PERMITTED_SCRIPT_UNLESS_EXECUTABLE,
            vec![SeccompRule::new(vec![SeccompCondition::new(
                PROTECTION_ARGUMENT,
                SeccompCmpArgLen::Dword,
                SeccompCmpOp::MaskedEq(PROT_EXEC_BIT),
                0,
            )?])?],
        );
    }
    if profile == Profile::Interpreter {
        // The one exception, and [`PERMITTED_INTERPRETER_NARROWED`] is why. A non-empty rule
        // vector matches only where its conditions hold, so `fcntl` reaches the match action —
        // `Allow` — for `F_GETFD` and falls to the mismatch action — `KillProcess` — for every
        // other command, exactly as it did when the number was on no list at all.
        //
        // **`Dword`, not `Qword`, and the reason is the kernel's declaration rather than a
        // preference.** `SYSCALL_DEFINE3(fcntl, unsigned int fd, unsigned int cmd, unsigned long
        // arg)`: the command the kernel dispatches on is 32 bits, so comparing 32 bits is
        // comparing what the kernel will act on. A 64-bit comparison would refuse a request whose
        // upper half is dirty and whose lower half the kernel would read as `F_GETFD` — stricter,
        // and therefore safe, but it would be a filter that disagrees with the call it is
        // filtering, which is how an argument-narrowed rule becomes wrong somewhere else.
        rules.insert(
            PERMITTED_INTERPRETER_NARROWED,
            vec![SeccompRule::new(vec![SeccompCondition::new(
                FCNTL_COMMAND_ARGUMENT,
                SeccompCmpArgLen::Dword,
                SeccompCmpOp::Eq,
                F_GETFD_COMMAND,
            )?])?],
        );
    }

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
