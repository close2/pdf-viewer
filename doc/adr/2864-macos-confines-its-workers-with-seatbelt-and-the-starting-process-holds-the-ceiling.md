# 2864 — macOS confines its workers with Seatbelt, and the starting process holds the ceiling

Status: **accepted and built** in `darwin-sandbox` (new: `enter`, `usage`), `pdf-sandbox`
(`lockdown_macos.rs`, `watch.rs`, `lockdown.rs`: `SystemCalls::Seatbelt`, `ceiling`) and
`confined-transport` (`host.rs`, `greeting.rs`).
Carries out `doc/todo/35`'s macOS half. Narrows ADR 0194 to Windows.
Clauses: none. Principle 3, not the standard.

## Context

ADR 0194 let the workers ship unconfined off Linux and said so in the first line of every run.
`doc/todo/35` named what macOS owed: a mechanism for *no filesystem, no network*, and an answer to
whether `setrlimit(RLIMIT_AS)` is enforced there, "the first thing to measure rather than assume".

The project owner chose the route, and the reason is the architecture rather than the platform:
> ich finde die anpassung b - im prozess selbst insofern besser, weil es zum linux way passt

and, of the memory ceiling, that it should not reshape the abstraction:
> ich will nicht die abstraktion anpassen müssen

## What was measured, on macOS 26.6.2 (Apple M5 Pro)

- **`RLIMIT_AS` and `RLIMIT_DATA` are not a ceiling here.** `setrlimit` is refused for both, and
  two gibibytes written after the attempt were allocated and touched without complaint. A process
  on macOS cannot bound its own memory; the todo's question has that answer.
- **A Seatbelt profile of `(deny default)` holds.** A process in it is refused a read of
  `/etc/hosts` and of the user's home directory with `EPERM`, and cannot resolve or reach a host.
- **Applied at `exec` it costs a start; applied in the worker it costs nothing measurable.** The
  `sandbox-exec` route — the profile applied by the starting process, before the program loads —
  needed Apple's `bsd.sb` imported for `dyld` to load the program at all, and cost about 9 ms per
  worker start: 4 ms to start `sandbox-exec`, 4 ms to compile `bsd.sb`. A render's own time was
  unchanged (574 thumbnails of a 13 MB report: 897 ms either way). Applied by the worker after it
  is loaded, the profile needs no import and nothing to load.

## Decision

1. **The worker confines itself, as on Linux.** `pdf_sandbox::lockdown::apply_for` gains a macOS
   branch, `lockdown_macos::apply`, which enters `(version 1)(deny default)` with `sandbox_init(3)`
   before the worker reads a byte. No worker, no spawn site and no protocol learns a new step. The
   `sandbox-exec` route was declined: every place that starts a worker would have had to wrap it,
   which inverts the arrangement Linux has, and it pays a start per worker for nothing the
   in-process route lacks.
2. **One profile for all three `Profile`s.** None of them asks the kernel for anything a profile
   governs once it is running: everything a worker reads arrives over a descriptor it already holds,
   and an open descriptor's reads and writes are not operations a profile is asked about. What
   distinguishes the profiles on Linux — threads, descriptors received, executable memory — is what
   Seatbelt cannot see, and that is said in `SystemCalls::Seatbelt`'s documentation rather than
   implied by a shared name.
3. **The two foreign calls live in `darwin-sandbox`.** `pdf-sandbox` reads untrusted bytes and stays
   `forbid(unsafe_code)`; `sandbox_init` has no binding in any crate this tree depends on, so the
   new crate is to macOS what `landlock` and `seccompiler` are to Linux: it reads no PDF byte, and
   each `unsafe` block is one call with its contract beside it. `sandbox_init` is marked deprecated
   since OS X 10.8 in favour of the App Sandbox; the App Sandbox attaches to a signed bundle when it
   is built and cannot confine a helper started with a pipe, and the kernel module under both is the
   one browsers still confine their renderers with. A future signed `.app` with XPC services is the
   point to revisit that.
4. **The memory ceiling is held by the starting process** (`pdf_sandbox::watch`). The worker reports
   its profile's ceiling in the greeting as it does on Linux; the spawner starts a `Watch` on the
   worker with that number before sending it anything. A thread reads the worker's physical
   footprint with `proc_pid_rusage` every 20 ms and kills it past the ceiling, and the worker's death
   is then reported through the path every caller already has, with the watch's sentence in front.
   The abstraction does not change: `Confinement::address_space_limit` carries the same number on
   both platforms, and its documentation now says who holds it.
5. **`SystemCalls` gains `Seatbelt`**, carried as byte 2 in both greetings, so that a Seatbelt worker
   is never reported as a seccomp one (`doc/todo/35`'s own requirement). `is_enforced` is true for
   it, and a Seatbelt worker with its ceiling owes no shortfall sentence: Landlock is a layer under
   seccomp, and a platform without either has nothing of it to report missing.
   `ENFORCED_BY_THIS_BUILD` is true on macOS, so `quorra`'s first-line note is Windows's alone.

## Consequences

- **The watch is a sampling interval late.** Linux refuses the allocation that crosses the ceiling;
  macOS's worker is killed after its footprint is seen past it, up to 20 ms of writing later. The
  bound is the same number; the overshoot is what a bound held from outside costs.
- **The watch is built not to kill another process.** It ends when the worker is gone or its start
  time differs from the first reading's, and its owners end it before they reap the worker, so an
  identifier the system has given to a new process is not acted on. The one window left is between
  a reading and the kill after it.
- **The watch fails closed.** A first reading refused for any reason but the worker being gone is an
  error the spawner reports instead of starting the worker unwatched; a later one kills the worker
  and names the reason, so a worker is never reported as held to a ceiling nobody is reading.
- **On macOS the ceiling is now a footprint**, and the confined viewer's and the virtual file
  system's message budgets are computed from it with an address-space baseline of zero, since
  `/proc` does not exist there. The largest message falls from 2 GiB to about 1.4 GiB, as on Linux;
  the sentences that name the ceiling still call it an address-space ceiling.
- **A confined macOS worker decodes in process on its own thread**, as a confined Linux one does
  (`lockdown::CONFINED`): Seatbelt would let the deadline thread start, and the same path on both
  platforms keeps the deadline the starting process's.
- **What a macOS worker can still do that a Linux one cannot**: start a thread, ask about a
  descriptor, make memory executable. None of them reaches a file or the network, which is the
  property this crate exists for.
- **Not done here**: closing descriptors a worker inherited by accident (`pdf-sandbox-worker`'s
  `close_range`, ADR 1945, is Linux's; the leak it answers is GLib's, not seen on macOS), and
  Windows (`doc/todo/35`).

## Checked

- `cargo test --release -p darwin-sandbox`: a footprint reading sees 64 MiB written; a reaped
  process is an error.
- `cargo test --release -p pdf-sandbox --test confinement` on macOS: a confined probe is refused
  `/etc/hosts` and a socket with `EPERM`, the same file opening unconfined; a worker reports `Seatbelt` and its ceiling, for the ordinary and the
  whole-resolution decoder; a process writing memory past a 256 MiB watch is killed by `SIGKILL` and
  the watch names the footprint. With the profile changed to `(allow default)` the file and network
  probes fail, so they test the profile and not the probe.
- `quorra-transform render` of the 574-page report through its confined worker: same pages, no
  "not confined" note.
