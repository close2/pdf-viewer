# 0387 — The forty uploads that moved none of the bytes, and the hull nobody read

**Status.** Accepted. Session 552, an attribution round: it takes a frame apart, removes the one
thing in it this tree owns and does not need, and hands two questions back with numbers on them.

## Context

ADR 0368 attributed a *zoom* frame on the project owner's `tmp/Entwurf.pdf` — one page, 58 009
display commands, 3.0 M path segments, no text, no images, not one clip — and found 59 % of it in
quorra's coverage rasterisation on one thread. ADR 0377 took upstream's answer and turned
`encode_threads` on at this machine's own parallelism. What the owner's trace shows now is a
different frame, and it is the one this round was given:

| the owner's `tmp/trace2.entwurf.txt`, medians of 15 frames | ms | share |
|---|---:|---:|
| **frame** | **272.0** | |
| `scene` — this tree's display-list walk | 12.6 | 4.6 % |
| `device` | 258.4 | 95.0 % |
| — `encode` | 128.9 | 47.4 % |
| — `transfer` | 64.3 | 23.6 % |
| — `execute` (the adapter's own timestamps) | **0.2** | **0.07 %** |
| — `elsewhere` (a bound, not a duration) | 62.6 | 23.0 % |
| `settle` | 0.9 | 0.3 % |
| resources handed over per encoded frame | **40** | |

Every millisecond of that is a millisecond ADR 0378's reprojection has to cover, so the surest
route to a correct frame every refresh is a cheaper frame. Three of those rows were this round's
subject: the forty uploads that appear to cost 64.3 ms, the 62.6 ms nobody has named, and the
12.6 ms of walk that is ours outright.

## The instrument, and why the numbers below are the owner's machine

**`crates/render-quorra/examples/zoom_frame.rs`**, new here. Two frames, one device, one page: the
first places the page at a scale and fills every cache, the second places the *same* display list —
the same `Arc`, so every outline is a hit — at a new one. That second frame is the population the
owner's trace is full of and the one nothing in this tree had ever drawn in isolation:
`examples/encode_threads.rs` brings a *cold* device up per sample on purpose, and a cold device
measures exactly what a magnification does not pay.

**It runs on the AMD Radeon 890M (RADV STRIX1) with no window and no compositor**, which is the
adapter the owner's traces were taken on. That is worth stating plainly because
`doc/environment.md` says to hand a run on the real GPU to the user, and it is right about a
*window*: `render_quorra::options()` names no adapter, so a headless device lands on the same GPU
without an X authority cookie or a session. So — unlike ADR 0368's and every earlier round's — **the
absolute numbers below are the owner's hardware and not llvmpipe.** What they are still not is the
owner's *window*: no swapchain, no vsync, and a readback where a surface would present.

The machine is shared, so the statistic is the minimum of interleaved rounds and the load average
is printed at both ends of every run.

## 1. The forty uploads move none of the sixty-four milliseconds

**They are two different quantities and the trace only ever printed one of them.**
`FrameCost::uploads` — the frame line's `up` — is what `render-quorra`'s caches handed the device:
outlines, images, ramps and §7.10.5 programs, counted where a lookup misses (`crate::cache`).
`transfer` is `quorra_gpu::Timings::upload`, which quorra documents as *"preparing and scheduling
CPU→GPU transfers for this frame"* — its own encoded scene. Nothing said so on one line, and a
reader with one number and a duration beside it will pair them.

quorra has counted the bytes since its ADR 0227 and `FrameCost::bytes_uploaded` has carried them
across this boundary ever since. **No host in this tree had ever read the field.** Read now:

| frame | resources handed over (`up`) | bytes quorra staged |
|---|---:|---:|
| first draw, 2133 × 607 | **58 029** | 6 898 596 |
| the zoom step, 2667 × 758 | **40** | **8 475 012** |

**More bytes, from fourteen hundred times fewer resource uploads.** The two do not correlate at
all, in either direction, and that settles the row: the 64.3 ms is not the forty and nothing this
tree stops handing over would shorten it. It is quorra's coverage tiles, its instance streams and
its atlas, staged for the device on every frame that encodes — which is every frame of a
magnification by construction.

**And the rate is the interesting part.** 8.5 MB in a phase that costs tens of milliseconds is
about 65 MB/s on an *integrated* adapter, where a memory-to-memory copy is two orders of magnitude
faster than that. So `transfer` is not transferring; it is the *preparing* half of quorra's own
description of the phase. That is not ours to fix and it is now a question with a denominator on
it — `doc/QUORRA_FEEDBACK.md` §29.

The number is in the viewer's own summary now, on the line under the upload count, saying in its
own words that it is not what `up` counts.

## 2. `elsewhere` is host time nobody times, and the two spans quorra names are microseconds

quorra's documentation of `Timings` tells a caller to subtract `host_total` and *"read what is left
against the `"target acquire"` and `"present"` entries of `phases`"*. `render-quorra` had been
dropping `Timings::phases` on the floor since it was written, so nobody here had ever looked. It is
carried across now — `QuorraPresenter::last_phases`, filled into a buffer the presenter keeps, so
it costs a memcpy of a handful of entries and no allocation after the first frame.

On the owner's adapter, on a zoom frame whose `elsewhere` is over a hundred milliseconds:

| phase | ms |
|---|---:|
| `target acquire` | **0.035** |
| `present` | **0.001** |
| `content pass` — the GPU's own timestamp for the drawing | **0.62** |

**The two entries quorra names account for three hundredths of one per cent of the remainder**, so
reading `elsewhere` against them answers nothing, and `doc/todo/45` §3's retraction — that it is a
bound rather than a duration — stands for a sharper reason than the clock disagreement it was
written for. What is actually in it is legible from `quorra-gpu`'s own source and is *host* time
inside `Device::render`:

- **`compose::submit_and_wait` is measured and then thrown away.** `run_frame` returns it as
  `execute_wall`, and `timing::read_pass` reports the adapter's timestamp *instead* wherever
  timestamp queries exist. The number is already taken; it simply does not survive.
- **`record_content` — building the wgpu command buffer — is timed by nothing at all.** For a page
  placing 58 003 coverage tiles that is where a host-side hundred milliseconds would live.

The frame line's own comment used to name the remainder as "acquiring the swapchain texture,
presenting it, and reading the timestamp queries back". That was a claim about a measurement nobody
had taken, and it is now corrected in place with the measurement beside it. **`execute` at 0.2 ms
of a 272 ms frame is the sentence that matters**: on a page this size the graphics device does
about a thousandth of the work, and everything a person waits for is one host thread.

**One thing found on the way, and it is ours rather than quorra's.** `Timings` has a *fourth*
measured phase — `readback` — and `FrameCost` did not carry it. It is zero for every frame that
goes to a window, which is why it went unnoticed for twenty-six sessions; it is not zero for
`QuorraRasterizer::rasterize_frame`, which is every corpus and oracle page and this round's own
instrument, and there a multi-megabyte copy was landing in the remainder a caller computes by
subtracting the other three. `FrameCost::readback` exists now. A phase the library measures and a
host drops is a phase the host will attribute to something else.

## 3. `scene` — and the one thing in it this tree computes and discards

`Encoder::fill` took the device-pixel window of every fill before doing anything else:

```rust
let within = self.device_pixels(path, transform);
```

**That value has exactly one consumer**: `Encoder::radial_cone`, which is reached only when the
paint is a `Paint::Shading` *and* the shading is a §8.7.4.5.4 cone. On the owner's drawing 58 003
of 58 009 commands are opaque fills and not one paint is a shading, so the window was computed
58 003 times a frame and read zero times. It was taken eagerly for a stated reason — so that the
three `emit_fill` calls under §10.7.4's collapsed-subpath split share one conservative bound rather
than measuring three — and that reason is kept: what is passed down now is the whole `Path`, and
the window is asked inside the branch that reads it.

**The cost was not the arithmetic.** `Path::bounds` memoises an untransformed hull and maps it
where the transform preserves axes — but the *first* call for a path walks every control point, and
nothing else on this page ever asks a path for its hull. So the eager window was paying a walk of
all 3.0 M path segments, once, for an answer nothing read.

**callgrind, `ZOOM_FRAME_ENCODE_THREADS=1`, one open plus two frames of the witness:**

| | before | after | delta |
|---|---:|---:|---:|
| **whole program** | **27 132 909 847** | **26 865 611 146** | **−267 298 701 (−0.99 %)** |
| `<pdf_render::geom::Path>::walked` | 227 021 102 | **0** | −227 021 102 |
| `<render_quorra::scene::Encoder>::commands`, self | 25 957 897 | 13 023 472 | −12 934 425 |
| `<pdf_render::geom::Path>::bounds` | 12 470 645 | **0** | −12 470 645 |
| the `OnceLock<Option<Rect>>` that memoised the hull | 2 262 117 | **0** | −2 262 117 |
| `<render_quorra::scene::Encoder>::emit_fill`, self | 8 239 266 | 8 007 174 | −232 092 |

**Wall clock, the same adapter, ten interleaved rounds an arm at load 2.8 → 3.2** — the `scene`
phase of the zoom frame, in ms:

| | samples | minimum | median |
|---|---|---:|---:|
| before | 14.0 15.6 11.6 11.9 11.2 13.2 12.1 13.3 15.4 13.1 | 11.2 | 13.15 |
| after | 10.7 12.1 9.8 8.9 14.0 12.0 10.0 9.8 9.3 12.3 | **8.9** | **10.35** |

**−20.5 % on the minimum, −21.3 % on the median**, and the two agreeing is what makes a
twenty-per-cent claim on a shared machine worth stating at all.

**The first frame is not claimed.** 227 M of the 267 M is the hull walk, which happens once per
path and therefore on the frame that builds — `first scene built` on the launch table. The
instruction count says it is a third of that frame's walk; the wall clock could not separate it
from the machine's own drift across a ten-round pass, and a number that needs a quiet machine is a
number the owner's next launch trace should produce rather than one this round should assert.

### Why the pixels cannot move, and what proves it anyway

The value's only consumer receives exactly what it received before, from the same path under the
same transform, computed by the same function. Nothing else in the change touches a drawing
decision: `readback`, `phases` and the summary line are reporting.

That is an argument, so it is not the proof. The proof is the gates, which are in
`doc/history/552-…` with their own output — the two quorra lanes in particular, since a page that
draws a cone is exactly what would move, and the corpus carries several.

## 4. `encode`, subdivided from a host for the first time without a patch

ADR 0368 read quorra's three encode phases once, through a probe it removed, and `doc/todo/45` §3
left open "whether it becomes a `FrameCost` field … to the round that next needs it". This round
needed it, and the answer is that it is not a `FrameCost` field: `FrameCost` is `Copy` and the
phases are a `Vec`. `last_phases()` beside `last_frame()` is the shape, and
`ZOOM_FRAME_ENCODE_PHASES=1` is how the instrument asks for the subdivision that costs a few per
cent to have.

The zoom frame, real adapter, minima of five rounds, load 22.1 → 21.1 — **the instrument costs a few
per cent of `encode` to have, so these are shares rather than absolutes**:

| phase | ms | share of `encode` |
|---|---:|---:|
| **`encode: recording`** | **94.1** | **43.8 %** |
| `encode: geometry` | 77.2 | 35.9 % |
| `encode: staging` | 43.5 | 20.2 % |

**Upstream's `QUORRA_ENCODE_THREADS_ANSWER.md` §6 is reproduced**: recording *"is now the largest
phase of your page"*, and it is, on this adapter and on the page the claim was made about. ADR 0368
measured geometry at **79.2 %** of `encode` on one thread; it is 35.9 % now, and the phase that
overtook it is the serial one their §6 names.

**A three-round pass earlier in the same session read the opposite ordering** — geometry 46.4 %,
recording 36.5 % — and the difference is the statistic rather than the machine: geometry is what
divides across 24 threads, so it is what a shared machine's contention inflates, and three rounds
find a worse minimum than five. Recorded because it is the trap upstream warned about in their own
currency — *"we are not publishing a crossover as a constant, and neither should you"* — arriving
one level down, in an *ordering* rather than a thread count.

## Decision

- **Take the lazy window.** It is a removal rather than an optimisation — a computation whose result
  was discarded — and it is 20.5 % of the phase this tree owns outright.
- **Read the two counters quorra was already sending.** `bytes_uploaded` in the viewer's summary,
  `Timings::phases` through `last_phases()`, and `readback` in `FrameCost`. None of the three is new
  information crossing the boundary; all three were being dropped on this side, and each of them was
  making a different row of the frame table mean something it does not.
- **Keep `zoom_frame.rs`.** ADR 0368's probes were named and removed and the next round rebuilt one
  of them; this is the instrument for the population every remaining question in `doc/todo/44` and
  `doc/todo/45` is about, and it costs nothing to keep.
- **Change nothing about `transfer`, `elsewhere` or `encode`.** All three are quorra's, and this
  round's contribution to them is a measurement with a denominator rather than a change.

### What was declined, with its number

- **Batching the forty uploads away.** They are the chrome's, rebuilt with fresh `Arc`s every frame
  by construction (`Overlays::of`, and ADR 0351's `Retained::overlays` holds them by value for that
  reason). Removing them would take 40 of 8 475 012 bytes off `transfer` — a rate of nothing — and
  would cost the reuse key its correctness.
- **Anything about `elsewhere` on this side.** It is inside `Device::render`, so no host code runs
  during it. Its subdivision is upstream's, and §29 is the ask.
- **Turning `instrument_encode` on for the window.** It costs a few per cent of `encode` (ADR 0368:
  512.8 ms against 475.9), and `encode` is 47 % of the frame. A person's window must not pay to be
  measured; an instrument may.

## Consequences

- `doc/todo/45` §3's open question about `Timings::phases` is closed, and its `elsewhere` row now
  has a measurement under it instead of an inference.
- `doc/todo/44` §5's table gains the row it never had: `execute` on the owner's own adapter is 0.07 %
  of a zoom frame, so every remaining lever on this document is a host-thread lever.
- The frame line's `elsewhere` comment carried a false enumeration of its own contents for
  twenty-four sessions. It is corrected in place, which is the only useful form of that repair.
- `doc/QUORRA_FEEDBACK.md` §29 carries the three measurements upstream: the transfer rate, the
  unnamed host span with its two candidate causes named from their own source, and the encode
  ordering under load.
