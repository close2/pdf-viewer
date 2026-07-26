# Project: PDF Viewer

A PDF viewer in Rust, targeting Acrobat-class fidelity, with the goal of being the
noticeably fastest PDF viewer available — and clean enough to be taught from.

## Non-negotiable principles

These four are stated by the project owner and override convenience, velocity, and any
default habit. When a suggestion conflicts with one of these, say so explicitly rather
than quietly compromising.

### 1. Quality first — no shortcuts

- No placeholder implementations, no `todo!()` left in merged code, no "we'll fix it
  later" paths. If something cannot be done properly now, it is not started now.
- No silent error swallowing. Every error is typed, propagated, and handled somewhere
  deliberate. No `unwrap()` outside tests and provably-infallible cases (and then with a
  comment naming why it cannot fail).
- Every public item documented, with `#![warn(missing_docs)]` enforced.
- `clippy::pedantic` clean. Warnings are errors in CI.
- If a shortcut is genuinely the right call, it is documented as a deliberate decision
  with its cost written down — never taken silently.

### 2. Fast — including startup

- Performance is a feature owned from day one, not an optimization phase later.
- Parallelism (rayon) and GPU offload (vello/wgpu) are used wherever they genuinely help.
- "Genuinely" is decided by measurement, never by assumption. For an interactive viewer,
  *latency* usually matters more than throughput — a parallel path that improves
  throughput while worsening time-to-first-page is a regression.
- Perf gates run in CI: cold open, time-to-first-page, page-turn latency, memory
  high-water. A regression fails the build.
- Remember where the time actually goes: parsing, xref resolution, and font loading
  usually dominate time-to-first-page — not rasterization.

#### Startup time is a first-class requirement

Launch latency is what a user judges the program by before it has rendered anything, and
it is the easiest thing to lose gradually to unnoticed initialisation. Rules:

- **Nothing eager.** No system font enumeration, no full page-tree walk, no
  configuration or recent-file scanning, no thumbnail generation on the launch path.
  Anything not needed to show page one is deferred until first use (`OnceLock`, not
  startup).
- **No parsed data at startup.** The Arlington-generated tables are compiled-in `static`
  data, so the object model costs zero parse time at launch. Any future data resource
  follows the same rule.
- **Incremental parsing.** Opening a document reads the trailer and the objects page one
  needs — not the whole file. A 500-page document must open no slower than a 5-page one.
- **GPU initialisation stays off the critical path.** Creating a wgpu device and
  compiling pipelines costs tens to hundreds of milliseconds, largely in the driver.
  Page one therefore renders on the CPU backend while the GPU initialises on another
  thread, and the GPU takes over once ready. This is a second, independent reason the
  CPU backend exists — it is not only the correctness oracle, it is the startup path.
- **No heavy runtime.** No async runtime unless something genuinely requires one; a
  thread pool for rasterisation is not a reason to pull in one.
- Cold-start and time-to-first-page are CI gates with numbers attached, measured with a
  cold page cache. Targets are set once Spike A gives a real baseline, rather than
  invented now.

### 3. Secure from the start

- Untrusted input never reaches unsafe code. `#![forbid(unsafe_code)]` on every crate
  that touches PDF bytes; this is compiler-enforced, not a convention.
- Multi-process sandbox: renderer runs unprivileged under seccomp-BPF + Landlock, with no
  filesystem and no network access.
- Memory safety is not enough. Explicit memory and time budgets guard against
  decompression bombs, xref cycles, and pathological content — Rust does not prevent
  resource exhaustion.
- Fuzzing from the first parser commit. Every crasher found becomes a permanent
  regression test.
- Any C dependency (notably JBIG2 / JPEG2000, both historically severe attack surfaces)
  must be isolated in the sandboxed process and justified in writing.

### 4. Exemplary — a project others can learn from

The aim is for this codebase to be worth showing to students as an example of how to
build an application cleanly.

- Architecture is legible: clear layer boundaries, no circular dependencies, each crate
  with one stated responsibility.
- Names say what things are. Comments say *why*, never *what*.
- Every non-obvious decision gets an ADR in `doc/adr/` — the reasoning matters as much as
  the result.
- Prefer the clear construction over the clever one.

### On the tension between 2 and 4

The project owner has noted that speed and exemplary clarity partly conflict. They
conflict less than they appear to, and the resolution is a rule:

**An optimization must be justified by a benchmark and explained by a comment.**

Clean architecture at the top, optimized code in measured hot spots, with every
optimization carrying (a) the benchmark number that justifies it and (b) a comment
explaining what it buys and what it costs in readability. Optimized code that is
*explained* teaches more than naive code does — a student learns both the technique and
the discipline of proving it was needed. What is forbidden is unexplained cleverness and
speculative optimization of code nobody measured.

Where the conflict is real and unresolvable, clarity wins in cold paths, speed wins in
hot paths, and the choice is written down.

## Stack

| Area | Choice |
|---|---|
| Language | Rust |
| Rasterizer | `tiny-skia` first (oracle + startup path), `vello` on wgpu second — behind one trait |
| Fonts | `skrifa` (+ Type1/Type3 handled in-tree) |
| Windowing | `winit` |
| Dialogs | `ashpd` (XDG desktop portal — native KDE dialogs, any toolkit) |
| Accessibility | `AccessKit` (AT-SPI on Linux) |
| Parallelism | `rayon` |
| Deflate | `flate2` with `zlib-rs` backend (pure Rust, ~C speed) |
| Spec model | Arlington PDF Model → generated validation layer |

**Not used:** `rustybuzz`. PDF content streams carry already-positioned glyphs; shaping
them again would move glyphs away from where the document specifies. It may return later,
scoped strictly to text *we* generate (annotations, form fields with non-embedded fonts).

## Working agreements

- You are running as your own user.  Obviously not a real sandbox, but you do not need to ask
  before deleting files,...   You are not able to modify global config or install anything globally.
  Evaluate if installing something globally by asking the human or creating a user local
  copy / installation automatically is the better choice.
- If a proposed fix looks wrong for this setup, say so instead of running it.
- Verify claims by running them. Report failures with their output; never assert that
  something works without having checked.

## Environment notes

- Arch Linux. GPU: AMD Strix (Radeon 880M/890M, RDNA 3.5) — RADV. Session: X11.
- Claude Code may run as user `AI` via `sudo -u AI`, reaching this tree through the
  `coders` group. That user has no X authority cookie, so GUI windows cannot be opened
  from such a session — use headless lavapipe for verification, and hand interactive runs
  to the user.
- KDE Frameworks 6 packages on Arch have no `kf6-` prefix (`kio`, `kconfig`, `ki18n`).
