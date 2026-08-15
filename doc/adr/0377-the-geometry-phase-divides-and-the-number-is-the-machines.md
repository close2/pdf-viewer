# 0377 — The geometry phase divides, and the number is the machine's

**Status.** Accepted. Session 542, an adoption round: it takes a release, turns one field on, and
proves the property that field threatens.

## Context

Session 533 measured the project owner's `tmp/Entwurf.pdf` — one page, 58 009 display commands,
3.0 M path segments, no text, no images, not one clip — and found that **59 % of a zoom frame was
one thread turning those segments into 58 003 coverage tiles** (ADR 0368). It priced every remedy
on this side of the boundary, found none worth building, and asked upstream for the one that was
not ours to build: `doc/QUORRA_ENCODE_THREADS.md`, an ask with its own ceiling stated — geometry to
*zero* still leaves about 235 ms of that frame.

quorra built it. `doc/QUORRA_ENCODE_THREADS_ANSWER.md` is their answer and their ADR 0054 is the
design: a `std::thread::scope` entered and left inside `Device::render`, no dependency and no
`unsafe`, everything an order depends on drained before it acts. **The API is one field —
`Options::encode_threads` — and its default is 1**, which upstream is explicit is *a permission
rather than a preference*: only a host knows whether it has a pool of its own, a seccomp policy
that forbids a thread, or a launch path a pool would land on.

So three questions arrive here, and only the first is a bump:

1. take the release;
2. **what number**, and **where does the decision live**;
3. **is the frame still the same frame** — upstream ran our corpus at 1 thread against 8 and
   reported every per-page line identical. That is a claim about *our* gate, and a claim about our
   gate is ours to reproduce rather than to accept.

## Decision

**`render_quorra::options()` is the one place this host's number is chosen, and the number is
[`std::thread::available_parallelism`].** Every constructor in `render-quorra` spreads it —
`QuorraRasterizer::new_headless`, `new_headless_software`, `QuorraPresenter::new` and
`with_instance` — and every caller in this tree that wanted quorra's defaults now writes
`..render_quorra::options()`. `with_options` stays exactly what it was: the escape hatch for a
caller who means something else.

**Not a constant.** Upstream reported a round of theirs where 24 threads read *worse* than 8 at
load 25–33 and declined to publish a crossover; this tree's own ADR 0260 found `rayon` over pages
stopping at about eight. A number written into this file would be this laptop's, and
`available_parallelism` is the call that already answers the question a cgroup quota and an
affinity mask were asked — a build moved to a smaller machine gets that machine's answer without
anyone editing anything.

**Not a flag.** `doc/todo/49`'s rule is that a flag must have a right default and exist for the
person who knows their document, and there is nothing here for a person to type: the number is the
machine's, and the one caller that genuinely needs a different one is a *gate*, not a person.
`crates/render-quorra/tests/corpus.rs` therefore reads `PDFVIEWER_QUORRA_ENCODE_THREADS` — the
same shape as its two existing knobs — so that the corpus can be run at 1 and at 24 and the two
compared. **What would revive the flag** is a person needing a single-threaded *window*: a machine
where the ladder below inverts, or a driver that misbehaves under a busy encode. `--encode-threads`
is where it would go, the instrument that would justify it is checked in, and nothing asks for it
today.

**And it is a host decision in the sense `CLAUDE.md` means**: the policy is asked once, in a place
a host can reach, rather than hard-coded where no caller can see it. Upstream's third reason for
the permission — a confined worker that cannot spawn — does not bind this tree today for a reason
worth writing down rather than assuming: **nothing under `viewer-confined` builds a quorra device
at all** (it draws with `render-cpu`), and `pdf_sandbox::lockdown::Profile::Interpreter` permits
`clone3` and `sched_getaffinity` because `render-cpu` already draws on every core. A future
confined path that *did* hold a device would call `with_options` and say 1, which is the shape this
decision leaves open.

## The measurement that chose the number

`crates/render-quorra/examples/encode_threads.rs`, new here and the instrument for this and for
any later round that doubts it. **A cold device per sample** — quorra's tile cache answers a second
frame at the same transform from the atlas, which is exactly what made ADR 0368's fourth frame
140 ms instead of 640, so a ladder walked on one device measures the cache from the second rung on.
**Round-robin rounds, and the statistic is the minimum**, because this machine is shared: a thread
count measured under load is a measurement of the load.

The owner's document at its fit view (900 × 256, 58 030 commands encoded), RADV, minima of five
round-robin rounds, `Timings::encode` as `FrameCost` reports it:

| threads | quiet (load 3.8) | busy (load 10 → 16) | oversubscribed (load 22 → 33) |
|---:|---:|---:|---:|
| 1 | 467.2 ms | 849.8 ms | 1376.0 ms |
| 2 | 329.6 | 539.8 | 1091.1 |
| 4 | 241.3 | 340.2 | 782.2 |
| 8 | 221.0 | 315.5 | 667.3 |
| 16 | 152.3 | 280.3 | 575.1 |
| 24 | **150.6** | **251.8** | **458.7** |
| | **3.10×** | **3.37×** | **3.00×** |

Three readings, and the third is the one that decided:

- **The curve is monotone at every load this machine could be put under.** The inversion upstream
  saw did not reproduce here — at load 22 → 33, with more spinners than the machine has threads,
  24 is still the fastest column and by the largest margin of the three. Their caution was right to
  publish and this is the answer to it on this machine, which is the only place it could be
  answered.
- **The knee is at about 16 and the last eight threads are free rather than valuable** when the
  machine is quiet (152.3 → 150.6). They stop being free the moment anything else is running, which
  is the argument against writing 16 into the code.
- **The frame is not the encode**, and the ceiling session 533 stated itself holds: the quiet
  frame's minimum falls 994.8 → 629.4 ms, a third rather than a sixth, because recording, upload,
  execute and this tree's own scene walk are untouched. A stall becoming a step is what was
  claimed and it is what arrived.

## Determinism, reproduced here rather than accepted

Upstream ran our corpus at 1 thread against 8 and reported every per-page line identical. This
round ran **all four lanes twice** — `cpu` and `gpu` coverage, scale 1 and scale 4, at 24 threads
and at 1 — and compared the verdict lines character by character.

| | at 24 threads | at 1 thread | judged lines |
|---|---|---|---:|
| scale 1, `cpu` | 931 / 23 / 2 / 18 | identical | 26 |
| scale 1, `gpu` | 929 / 25 / 2 / 18 | identical | 28 |
| scale 4, `cpu` | 936 / 10 / 5 / 23 | identical | 16 |
| scale 4, `gpu` | 937 / 9 / 5 / 23 | identical | 15 |

*(agree / differ / refused / not comparable. The judged lines are every `differs:` and `refused:`
line with its mean, worst tile, differing fraction and similarity, plus the verdict counts —
compared character by character with the clock stripped out of the one line that carries one.)*

**And the release moved nothing either**: those four rows are the same four ADR 0367 recorded at
`a64a9084`, so 32 commits of upstream — a whole function-paint implementation among them — arrive
here as no pixel at all, which is what a paint nothing in this tree emits should do.

**`REFUSED_AT_FOUR` did not move**, which is the ratchet the ask itself named as the one a parallel
phase would break if it changed what a frame commits.

**One number moved that is not a verdict, and the control says what it was.** The scale-1 `cpu`
lane took 98.1 s at 24 threads against 40.5 s at 1 — and in the same run the *CPU oracle*, which
never touches quorra, took 9.13 s against 3.06. Both sides scaled by the same factor, on a machine
carrying a parallel round: it is the load, not the threads. What the corpus does say about threads
is the thing upstream's floor predicts — over 956 pages that are mostly a page of text, the number
buys nothing measurable in either direction, because a page below 4 096 queued segments never
enters the pool.

## What it buys in the window, on the document that asked

`tmp/Entwurf.pdf` under `Xvfb` at 900 × 1100 on llvmpipe, the release binary, ADR 0368's own
script — settle, two magnifications up, two back down — with the arms alternated **A A B B A**
around one rebuild each way, the *only* variable being the field this ADR turns on. The frames are
identified by their cull counts, which are deterministic and reproduce ADR 0368's exactly (8763 and
17986 at the two magnifications), so the rows below are the same frames in every run.

| the frame | 1 thread | 24 threads |
|---|---|---|
| first magnification (8763 culled) | 608.2, 618.5 ms | **295.0, 331.6, 372.7** |
| second magnification (17986 culled) | 514.6, 701.6 | **274.7, 314.6, 322.3** |
| the magnification drawn before (8763 again) | 268.7, 176.2 | **131.0, 136.8, 168.2** |
| back to the fit view (nothing culled) | 937.8, 799.6 | **314.1, 380.0, 420.3** |

**The structure is unchanged and the shares are what moved.** `host` is 0.0 on every frame of every
run, `scene` is 14–22 ms whatever the thread count, `settle` is 1–2 ms, 40 resources are uploaded
and the cull counts are identical — the whole of the difference is inside `device`, which is where
the phase that divides lives. **A zoom step that was a stall is a step**, which is exactly the claim
session 533 made for it and no more: the third row is the one with little geometry left to divide
(quorra's tile cache already holds those tiles) and it moves least.

### The launch table did not move, and that was the claim to check

Upstream says nothing is built at device construction. The launch table says so from this side —
`graphics device` is the row that would carry a pool if one existed:

| | 1 thread | 24 threads |
|---|---|---|
| `graphics device` (its own step) | +30.4, +27.9 ms | +35.3, +22.8, +30.9 |
| `interpreted, 58009 cmd` | +698.5, +757.2 | +754.6, +739.6, +752.7 |
| `first present`, absolute | 1793.9, 2002.8 | 1511.9, 1573.9, 1750.6 |

The device row's two ranges overlap and neither is ordered: the cold-start gate is untouched, as
predicted. The launch *total* does improve, and the reason is not the launch path — the first frame
is a frame like any other and its encode divides too.

## What it cost, written down

- **A queue holds every tile in flight where the walk held one**, bounded upstream at 1/64 of
  `max_frame_bytes` as a batching granularity rather than a capacity (their ADR 0054). It is their
  budget, and this tree's evidence for it is the refusal column above: unmoved at both scales.
- **`recording` is now the largest phase of the owner's page.** Upstream measured 132 ms of encode
  with geometry at 47 on their machine; ADR 0023's "revisit when" is closer than it was, and it is
  the next thing to ask for on that document rather than anything on this side.
- **A page of curve clips gets much less than this page did.** Residue-clipped marks stay serial by
  design, and upstream's `artwork` archetype moves 1.2× where the drawing moves 6.6×. **We cannot
  say how much of our corpus is which**, and the number that would say is one of the two censuses
  this tree still owes them (`QUORRA_FEEDBACK.md` §25.3's `(clip_residue_regions,
  clip_residue_tiles)` distribution). That is now the most interesting of the two.

## What this round deliberately did not do

- **`Timings::phases` on `FrameCost`.** Upstream's §7 notes it has been public since their ADR
  0023 and that making it a `FrameCost` field is entirely ours. It is a probe a round adds when it
  has a question, and this round's questions were answered by `encode` alone.
- **The fifth-frame tile-cache loss** session 533 saw and declined to report as a defect.
  `Counters::atlas_repacked` — wired here in session 532 — is the instrument for it, and it stays a
  later round's.

[`std::thread::available_parallelism`]: https://doc.rust-lang.org/std/thread/fn.available_parallelism.html
