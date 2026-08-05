# ADR 0194 — Three platforms, one confinement

Status: accepted, 2026-08-05 (session 315). **Decided by the project owner**, who asked for macOS
and Windows executables and said the sandbox may be absent on them.

## Context

`pdf-sandbox` carried a `compile_error!` for every platform that is not Linux:

> There is deliberately no fallback: a sandbox that silently does nothing on another platform is
> worse than no sandbox, because the code above it would keep handing untrusted input to a decoder
> while believing it was contained.

`viewer-ui` depends on it, so the viewer did not build on macOS or Windows and the snapshot release
shipped Linux only (ADR 0188). `doc/todo/35` wrote down three ways out and recorded that the owner
had chosen the first — build the real confinement for each platform — which is the largest of the
three and had not started.

**The owner's question, and the correction it needs.** The request came with a premise: *the
sandbox is only for DOS protection — memory exhaustion*. That is two thirds right, and the missing
third is what decides this ADR. `pdf-sandbox`'s own documentation lists three reasons the boundary
exists:

1. **Resource exhaustion** — the premise. `RLIMIT_AS` on a separate process is a fact where
   `decode.rs`'s hand-placed `MAX_PIXELS` and `MAX_SAMPLES` are a discipline.
2. **Panics** — release builds are `panic = "abort"`, so a slice index a decoder got wrong on one
   malformed file takes the viewer down *with the document open*. In a worker it costs one image
   and the page draws around it.
3. **Insurance** against a codec that one day has to be linked from C.

Reason 2 is a property of the **process**, not of the kernel — and every platform has processes.
What is *not* on the list is memory corruption: all three decoders are pure safe Rust with
`#![forbid(unsafe_code)]` and no dependencies (`doc/todo/_image-codecs-and-the-sandbox.md` §1),
which is why `--no-sandbox` can exist at all.

## Decision

**Build everywhere. Keep the worker process everywhere. Install the kernel confinement where it
exists, and say so in the first line where it does not.**

This is `doc/todo/35`'s option 3 — "build everywhere with a weaker confinement" — taken as the
owner directed, and the file said it "would need its own ADR arguing that a process boundary
without a filter is worth having". This is that argument:

- **The panic isolation is untouched.** It is the reason a corpus of 974 hostile-ish documents does
  not take the viewer down, and it costs a process rather than a kernel interface.
- **The deadline is untouched, and on Windows it had to be rebuilt to stay so.** A request is
  bounded by `REQUEST_TIMEOUT`, which on unix is `poll` on the worker's pipe. Windows has no
  equivalent that reaches a `ChildStdout`, so the pipe moves onto a reader thread and the timeout
  is the channel's. **On a platform with no address-space ceiling, that deadline is the only bound
  left on a hostile file's decode**, which is why it was worth a thread rather than dropping.
- **What is lost is named, in three places.** `Confinement` gained a `SystemCalls` field, the
  worker's handshake carries it (magic `PDFSBX02`), `Confinement::shortfall` words it as one
  sentence, and `pdf-viewer` prints at startup that this build has no kernel confinement. The
  `compile_error!` was defending against a caller that believed itself confined and was not; a
  caller that is *told* is not that failure.
- **`#![forbid(unsafe_code)]` survives.** That is the constraint that makes option 1 expensive —
  a Windows job object or macOS's `sandbox_init` would have to be reached through a safe wrapper
  as `landlock`, `seccompiler` and `rustix` are on Linux — and it is why the platform without one
  gets *no* confinement rather than an `unsafe` block.

The dependencies moved with the code: `landlock`, `seccompiler` and `libc` are
`[target.'cfg(target_os = "linux")'.dependencies]` and `rustix` is `cfg(unix)`, because a `cfg`
inside the source would still make every other platform *build* `seccompiler`, which is exactly
what stopped compiling for Windows.

## Consequences

- **The snapshot release is three archives**: x86_64 Linux, aarch64 macOS, x86_64 Windows. The
  workflow's `snapshot` job became a matrix and a separate `publish-snapshot` job, because three
  jobs racing to delete and recreate a moving tag would let the loser publish the older commit's
  binaries under the newer one's name.
- **A `platforms` job builds and tests on macOS and Windows runners on every push**, so the code
  behind the `cfg` cannot rot. It runs `pdf-sandbox`'s and `pdf-render`'s tests — the ones needing
  neither a display nor the specification documents, which are a repository secret.
- **Four of `pdf-sandbox`'s tests are `cfg(target_os = "linux")`**, and gated rather than skipped:
  they ask whether a *kernel* refuses an `openat` and a `socket`, and a platform with no seccomp
  has no such question to answer. A test that passed by doing nothing would be worse than one that
  is not there.
- **`doc/todo/35` is not closed.** What it now owes is option 1 for each platform — a job object
  and AppContainer on Windows, the App Sandbox on macOS — and the file says what each would cost.
  This ADR narrows principle 3 on two platforms and writes the narrowing down; it does not decide
  that the narrowing is permanent.
- **Untested here, and the CI job is the instrument.** This machine has no MSVC toolchain and no
  macOS, so what was verified locally is `cargo check` and `cargo clippy` for both targets across
  the workspace's binaries. Linking, running and drawing a page on either platform is what the new
  jobs will say — and a build that fails there fails a push to `main` rather than a release.

## What the first run found, and what it says about the check

Both new jobs **built** and then failed their test step, on two things this machine had not been
asked:

- **`RUSTFLAGS: -D warnings` is CI's and was not the cross-check's.** Three constants in
  `tests/confinement.rs` — the probe's environment variable and its two exit codes — belong to the
  four tests that are now `cfg(target_os = "linux")`, so off Linux they are dead code, so they are
  *errors* under the workspace's own lint policy as CI applies it. A local `cargo check --target`
  says nothing about that. **A cross-target check that does not set `RUSTFLAGS` is checking a
  different build from the one CI runs**, and the handover's "Verify it" now spells the command out.
- **`is_file` on a path with no `.exe` is false.** `worker_program()` searches beside the running
  executable for `WORKER_PROGRAM`, and Windows spells that file `pdf-sandbox-worker.exe` — so the
  search would have found nothing, and the viewer would have refused every JBIG2 and JPEG 2000
  image while reporting a missing worker. `worker_file_name()` appends `std::env::consts::EXE_SUFFIX`,
  and the constant now documents that it is the name *without* it. The compiler could not have
  found this one; the Windows runner's test step would have, one failure later.
