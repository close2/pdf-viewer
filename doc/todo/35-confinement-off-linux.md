# Confinement on macOS and Windows, and the executables that wait on it

Status: **owed**, and it is the only thing between this tree and a three-platform release.
Priority: 35 — capability. Nothing is wrong here; something does not exist.
Corpus: — (this is not a question about any document)
Clauses: none. Principle 3, not the standard.
Code: `crates/pdf-sandbox/src/lockdown.rs`, `crates/pdf-sandbox/src/lib.rs`'s `compile_error!`,
`.github/workflows/ci.yml`'s `snapshot` job.

## What is true today

`pdf-sandbox` confines the JBIG2 and JPEG 2000 decoders to an unprivileged process with seccomp-BPF
and Landlock. Both are Linux interfaces, and the crate refuses to compile anywhere else with the
argument in the message:

> There is deliberately no fallback: a sandbox that silently does nothing on another platform is
> worse than no sandbox, because the code above it would keep handing untrusted input to a decoder
> while believing it was contained.

`viewer-ui` depends on it, so **the viewer does not build on macOS or Windows**. Measured rather
than assumed in the three-hundred-and-eleventh session: `cargo check -p viewer-ui --target
x86_64-pc-windows-msvc` fails inside `seccompiler`, on eight system-call constants `libc` does not
define off Linux. The snapshot release therefore ships Linux only (ADR 0188).

## The three ways out, and none of them is wrong

The project owner chose the first in session 311. The other two stay written down because either
could be taken later, and because a gap nobody argued about turns into an oversight.

1. **Build the real confinement for each platform.** Windows: a job object for the address-space
   limit, and AppContainer or `SetProcessMitigationPolicy` for the rest. macOS: the App Sandbox,
   with `sandbox_init` deprecated and entitlements the supported route — which means a signed,
   bundled application, so this is the option that reaches furthest into how the program is
   packaged. **The thing to settle first is `#![forbid(unsafe_code)]`**: `pdf-sandbox` has it
   today because `landlock`, `seccompiler` and `rustix` each wrap the raw calls safely, and the
   Windows and macOS equivalents would have to be found or written to the same standard rather
   than reached for with `unsafe` blocks.
2. **Build everywhere, refuse the codecs off Linux.** `cfg`-gate the Linux-only dependencies,
   keep the worker protocol, and have a JBIG2 or JPEG 2000 request on an unconfined platform
   report exactly as a missing worker already does. Nothing about principle 3 weakens; two image
   formats are unavailable there until option 1 lands. **This is the cheapest honest option** and
   the one to take if a macOS or Windows executable is wanted before the confinement exists.
3. **Build everywhere with a weaker confinement.** Separate process and an address-space limit
   (job object, `setrlimit`), no syscall filter, no filesystem restriction, and the crate says at
   runtime what it is not doing. Three fully-functional platforms, and a documented departure from
   principle 3 that would need its own ADR arguing that a process boundary without a filter is
   worth having.

## What taking any of them costs in the workflow

One line. `.github/workflows/ci.yml`'s `snapshot` job builds a single target because there is a
single target; it becomes a matrix over `runs-on` and an archive name the day the second one
compiles. The packaging step is already written to name its target rather than assume it.
