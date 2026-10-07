# 1608 — The script worker is a third confined program, with the narrowest profile

Status: accepted and **built**. Session 1386. Builds on RFC 0008 sections 6.2 and 11 item 4,
accepted by the owner in `doc/questions/A193`; ADR 1590 (Boa, and what its budgets cannot bound);
ADR 1602 (one realm per document). ADR 1609 is what crosses to it and how it is supervised.
Context: `CLAUDE.md` principle 3 (untrusted input never reaches unsafe code unconfined; budgets
against pathological input); traps 18, 31, 32 and 83.
Code: `crates/pdf-script-worker/` (new), `crates/pdf-sandbox/src/lockdown.rs` (`Profile::Script`),
`crates/pdf-sandbox/src/lockdown_linux.rs` (`PERMITTED_SCRIPT`, the `mmap` rule, the ceilings),
`crates/pdf-script/src/engine/mod.rs` (the idle module loader), `tools/conformance/tests/batch.rs`.
Tests: `crates/pdf-script-worker/tests/end_to_end.rs`, `crates/pdf-sandbox/tests/confinement.rs`'s
`a_confined_script_worker_reaches_none_of_what_its_profile_denies`.

## 1. A program of its own, on the shared wire

`pdf-script-worker` is the third program on `confined-transport` (ADR 0846), beside
`pdf-view-worker` and `pdf-vfs-worker`, with its own eight-byte magic (`PDFSCW01`) so that a host
started on the wrong program refuses it at the greeting. Not inside the view worker, for RFC 0008
section 6.2's reason: a script that spins under the rasteriser's ceiling takes the page with it.
The binary needs the engine and is built only with `--features engine`; the host half of the crate
(the wire and `ScriptWorker`) builds without it and links no engine.

**It is not installed until a host supplies a level for scripts.** No window does yet (RFC 0008
section 11 item 3 (e)), so an installed worker would put an engine beside `quorra` that nothing
starts. `tools/conformance/tests/batch.rs` reads a `[[bin]]` that states `required-features` as a
program a person is not handed, with this ADR as the reason; the round that wires the first level
installs it with `--features pdf-script-worker/engine` and the expectation follows.

## 2. `Profile::Script`, measured, and why each call is on it

`strace -ff` over the worker, debug and release, running `end_to_end.rs` — the engine's library
from `Math.random` to `JSON`, `RegExp`, typed arrays and `AF*` calls, a run the wall budget stops,
one the deadline kills, one the ceiling aborts and one that overflows the parser's stack. After the
filter was installed the workers issued `brk`, `read`, `write`, `mmap`, `mremap`, `munmap`,
`getrandom` and the abort path (`rt_sigprocmask`, `getpid`, `gettid`, `tgkill`), and nothing else:

| kept | why |
|---|---|
| `read`, `write` | the frames, and standard error, a pipe the host reads |
| `brk`, `munmap`, `mremap`, `mmap` without `PROT_EXEC` | **Boa's allocator is the global allocator**: `boa_gc` boxes every cell through `glibc`'s `malloc`, which on one thread keeps one arena, grows it with `brk` and maps a block past its threshold |
| `madvise` | not issued; kept because `glibc` reaches it on its own pages under a `GLIBC_TUNABLES` the host's environment passes through, and it touches nothing else |
| `getrandom` | `Math.random`'s seed |
| `clock_gettime` | **Boa's time source is not the machine's**: a realm's `Date` reads the moment the request carries (ADR 1602), so the only clock read is the wall budget's `Instant` between slices, served by the vDSO; kept, as the decoder keeps it, for a machine without one |
| `exit_group` | leaving when the host closes its end |
| the abort path, and `rt_sigaction`, `rt_sigreturn` | the ceiling's failed allocation and the standard library's report of an overflowed stack both end in `abort`; without these the filter's `SIGSYS` would name the wrong cause (trap 18) |

**Absent, and each absence is the profile:** `clone`, `clone3`, `rseq`, `set_robust_list`,
`sched_getaffinity` (one thread; the filter does not admit a second); `recvmsg`, `pread64`, `fcntl`
and `close` (no descriptor: the worker reads its frames with `read`, so a descriptor sent beside a
frame is discarded by the kernel rather than received, and it never lets go of one it holds, so
trap 32 cannot arise); `mprotect` (one arena never changes a mapping's protection); `futex`,
`sched_yield`. **`mmap` is the one argument-narrowed rule**: `prot & PROT_EXEC == 0`, so no mapping
is executable and, with `mprotect` absent, no written one can be made so. `RLIMIT_NOFILE` is zero.

## 3. Three findings the measurement made

- **Landlock needs a descriptor.** With `RLIMIT_NOFILE` zero set beside the other limits the ruleset
  could not be created and the greeting reported no Landlock domain. The descriptor ceiling is now
  set after Landlock, for every profile, and the greeting reports `Enforced`.
- **Boa's default module loader reads the file system.** `Context::builder().build()` constructs a
  `SimpleModuleLoader` over `.`, whose `canonicalize` is a `realpath` and a `getcwd` — trap 31's
  shape, and every confined worker died of it at its first run. A realm is now built with Boa's
  `IdleModuleLoader`, which refuses every module and reads nothing; a document's scripts import
  nothing.
- **A probe in a test binary is not the worker.** An allocation probe run on the test harness's
  thread met `mprotect` in a second arena and was killed; the worker, on one thread, never issues
  it. So `confinement.rs` holds the denials and `end_to_end.rs`, in the worker itself, holds what
  is admitted.

What no probe reaches: `mmap` asked for `PROT_EXEC` directly, for which no safe call exists; the
rule stands on the filter's text, and the `mprotect` probe covers making written memory executable.
