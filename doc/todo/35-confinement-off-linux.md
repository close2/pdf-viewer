# Confinement on macOS and Windows

Status: **open, and no longer blocking an executable.** The viewer builds and ships on all three
platforms since the three-hundred-and-fifteenth session (ADR 0194); what is owed here is the
confinement itself on the two that have none.
Priority: 35 — capability. Nothing is wrong here; something does not exist.
Corpus: — (this is not a question about any document)
Clauses: none. Principle 3, not the standard.
Code: `crates/pdf-sandbox/src/lockdown.rs`, `crates/pdf-sandbox/src/lockdown_linux.rs`,
`.github/workflows/ci.yml`'s `platforms` and `snapshot` jobs.

**Read [`_image-codecs-and-the-sandbox.md`](_image-codecs-and-the-sandbox.md) first if the
thought is "remove the codecs and the problem goes away".** It does not: all three decoders are
already pure safe Rust with no dependencies and compile on every platform, and the sandbox exists
for resource exhaustion and panic isolation, which are as true off Linux as on it.

## What is true today

`pdf-sandbox` compiles everywhere. On Linux it confines the worker with seccomp-BPF, Landlock and
`RLIMIT_AS`; on macOS and Windows it installs **nothing** and says so — `Confinement::shortfall`
words it, the worker's handshake carries it, and `pdf-viewer` prints it in its first line.

What those two platforms still get, and it is not nothing:

- **The worker process**, so a decoder panic costs one image rather than the viewer — release
  builds are `panic = "abort"`, and this is the reason that matters most in practice.
- **The request deadline**, which on Windows is a reader thread and a channel timeout because
  `poll` is POSIX. On a platform with no address-space ceiling this is the *only* bound left on a
  hostile file's decode, which is why it was worth the thread.
- **`decode.rs`'s `MAX_PIXELS` and `MAX_SAMPLES`**, which are a discipline rather than a fact.

What they do not get: a system-call filter, so the worker's filesystem and network are the
process's own; and an address-space ceiling, so a decompression bomb is bounded only by the
machine.

## What is owed: option 1, per platform

The three ways out are in ADR 0194's context; option 3 was taken. Option 1 is what closes this
file, and it is two independent pieces of work.

**Windows.** A *job object* gives the address-space ceiling (`JOB_OBJECT_LIMIT_PROCESS_MEMORY`),
and AppContainer or `SetProcessMitigationPolicy` gives the rest — no filesystem, no network,
restricted tokens. Both are `windows-sys` calls, and **the thing to settle first is
`#![forbid(unsafe_code)]`**: `landlock`, `seccompiler` and `rustix` each wrap the raw calls safely,
and the Windows equivalents would have to be found or written to the same standard rather than
reached for with an `unsafe` block. That constraint is why this crate builds unconfined on Windows
today rather than half-confined.

**macOS.** The App Sandbox, with `sandbox_init` deprecated and entitlements the supported route —
which means a signed, bundled application, so this is the option that reaches furthest into how the
program is packaged. `setrlimit(RLIMIT_AS)` exists and would be the cheap half; whether it is
*enforced* on modern macOS is the first thing to measure rather than assume.

## What a round taking either owes

- A test that fails on a kernel that should confine and cannot, in the shape
  `a_confined_process_cannot_open_a_file` already has. The four Linux tests are
  `cfg(target_os = "linux")` and each new platform adds its own rather than widening those: the
  question "does the *kernel* refuse this" is asked differently everywhere.
- A `SystemCalls` variant or a wider `Confinement`, so that "confined by a job object" is not
  reported as "confined by seccomp". The vocabulary is deliberately small today because there is
  one mechanism.
- The startup sentence in `pdf-viewer` narrows or goes away, and it is the visible half: a person
  running the Windows build is told what this build cannot enforce, and that sentence is a promise
  to keep accurate.
