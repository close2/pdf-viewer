# `encode` on more than one thread — an ask, with its own ceiling stated

> **Answered, built, taken and turned on.** `doc/QUORRA_ENCODE_THREADS_ANSWER.md` is the reply,
> quorra's ADR 0054 is their design, and it is in this tree at pin `619ef3b4`. **The body below
> stays as it was written** — it is the ask, and an ask that is edited after the answer stops being
> evidence of what was asked. What changed on this side is one thing and it is ADR 0377: the number
> the answer left to us is chosen once, in `render_quorra::options()`, from
> `std::thread::available_parallelism`, measured by `crates/render-quorra/examples/encode_threads.rs`
> on the very document §1 is about. Three of this document's own statements were checked rather than
> believed — the determinism of §5 (all four corpus lanes, at one thread against twenty-four,
> identical), the untouched cold start of §4 (the launch table's `graphics device` row does not
> move), and the ceiling of §6 (the zoom step is 608 → 295 ms and the fit view 938 → 314, a stall
> becoming a step and not sixty frames a second). §7's fallback plan was not needed. §8's two
> censuses are still owed, and the second is now the more interesting of the two: it is the number
> that says how much of this corpus gets the drawing's factor and how much gets `artwork`'s.

Written 2026-08-15 from **this** side, against quorra at `a64a9084`. It is a request for one
thing, it is the fourth item of the `encode` conversation your ADR 0023 opened and your
`QUORRA_API_2026_08_15.md` §4 last moved, and — unlike `doc/QUORRA_FUNCTION_PAINT.md` — **it does
not claim to deliver a frame.** §6 says what it does deliver and §7 says what happens if the
answer is no. `doc/QUORRA_FEEDBACK.md` remains the standing document; this is separate because it
is a design question with a cost attached rather than a defect.

## 1. The document, and the one number

The project owner's own file: one page, 49.7 MB, one content stream inflating to 141 MiB,
3 185 295 operators, **58 009 display commands — 58 003 fills, six strokes, no text, no images, no
groups, and not one command under a clip or a soft mask**, carrying **3 011 879 path segments**
between them, 51.9 per fill. It is a
geological cross-section exported by Inkscape as tens of thousands of filled polygons. Your
`QUORRA_RETAINED_FRAME.md` §2 already quotes its trace back to us; this is the same document with
your own instrument switched on.

`Xvfb`, llvmpipe, 900×1100, three scripted sessions, the frame each one ends on — the whole page at
the fit view, 58 009 commands, nothing culled, the magnification new:

| | ms | share of the frame |
|---|---:|---:|
| our display-list-to-`Scene` walk | 15.8 | 2.5% |
| **`Timings::encode`** | **475.9** | **74.4%** |
| `Timings::upload` | 65.4 | 10.2% |
| `Timings::execute` (your timestamp queries) | 29.1 | 4.5% |
| the remainder inside `Device::render` | ~52 | ~8.1% |
| **the frame** | **639.8** | |

The other two sessions: 660.0 and 661.9 ms. **The adapter is 4.5% of it.** Everything else is one
host thread, and three quarters of the whole is `encode`.

## 2. `Options::instrument_encode`, used for the first time here, and what it found

With the switch on — which costs what your `instrument.rs` says it costs, 512.8 ms against 475.9
on this frame, so read the shares:

| phase | ms | share of `encode` |
|---|---:|---:|
| **`encode: geometry`** | **406.3** | **79.2%** |
| `encode: recording` | 82.9 | 16.2% |
| `encode: staging` | 23.6 | 4.6% |

**So 59% of the whole frame is your scanline rasteriser turning 3.0 M path segments into 58 003
coverage tiles, on one thread.** Your ADR 0023 was built for exactly this question and this is the
first time this tree has asked it; the answer is not the one either side would have guessed from
the 3.86 µs/command fit, because that fit came from pages of glyphs against an atlas and this page
has no atlas in it at all.

**One control, so the phase is not merely named but identified.** The fourth frame of each session
zooms *back* to the magnification the second frame drew — the same commands at the same transform,
in a scene `FrameSlot` rebuilt from nothing, `encode_source` reading `Encoded` on both:

| the same view, twice in one session | frame | `encode` |
|---|---:|---:|
| first draw, no instrument | 629.7 | 483.8 |
| **second draw, no instrument** | **140.0** | **90.6** |

and the second draw subdivided, in the instrumented session: `encode` 93.8 of which **geometry
1.7**, staging 0.2, **recording 91.8**.

Your tile cache working exactly as designed — and also the proof that the 406 ms is coverage
rasterisation and nothing else, since `recording` does not move.

**One thing that pair does not establish**, offered because you will see it if you reproduce this:
the *fifth* frame returns to the fit view the *first* frame drew and pays full geometry again. Three
intervening magnifications is evidently enough to lose it. We did not chase whether that is the
atlas's capacity or a transform differing where we did not look, and we are not reporting it as a
defect — but if a *policy* of keeping the two or three magnifications a person moves between
resident is cheap on your side, it is worth 4.5× on the gesture that matters and it would come
before any of §4.

## 3. Why the device is not the answer here, measured rather than assumed

Before asking for threads we tested the obvious alternative, because
`doc/quorra-gpu-coverage.md`'s headline sentence is *"nothing in it depends on the
magnification"*: we forced `Coverage::Gpu` for a whole session.

**Nothing moved.** `encode: geometry` 418.5 ms against the CPU lane's 406.3, the frame 732.7
against 660.0. And your own code says why, which is the part worth writing down: `take_gpu_lane`
requires `tile area ≥ triangles × 3 × WindingVertex::STRIDE`, and a 52-segment outline is at least
five kilobytes of vertices against a tile of **about three device pixels** at this page's fit view.
Your other two conditions are both satisfied here — the caller asked, and there is no residue clip
anywhere on the page — so the refusal is the triangle test and nothing else. Your ADR 0026's
refusal, the page that asked for 821 MB of vertices, is this page's whole population.

**The rule is right and we are not asking you to change it.** It is the reason the ask below is
about threads rather than about a second lane: this shape of page is what the CPU lane exists for,
and the CPU lane is one thread.

## 4. What we would ask for

**Divide `encode` across more than one thread**, and `encode: geometry` first, because it is 79% of
the phase and because the work is independent by construction: each command's coverage tile is a
function of its own polylines, its own fill rule and its own resolved clip, and nothing in it reads
another command's result.

`quorra-gpu` links `quorra-scene`, `thiserror`, `wgpu` and `pollster`, and the only
`thread::spawn` in the crate is the background pipeline compile. So this is an ask for a dependency
as much as for a change, and that is a decision only your side can take.

We are **not** asking for:

- a parallel `Device`, or any thread-safety on your public API. One frame, on one caller thread,
  entering a pool inside `Device::render` and leaving it before the call returns is the whole
  shape;
- parallelism in `recording` or `staging` first. They are 16% and 5%, and the sheet packing and the
  instance order are exactly the parts where an order matters;
- a thread pool built at construction. `CLAUDE.md`'s launch rules bind our side of the boundary,
  and a pool that exists before the first page needs it would show up in our cold-start gate;
- anything on the frame path that a small page pays for. Our own ADR 0228 divided an image
  resampler across `rayon` and had to put a measured floor under it for exactly this reason.

## 5. What it must not cost

**Determinism, in two directions, and the second is the one a thread pool threatens.**

- **Across adapters.** Our CI compares RADV against lavapipe byte for byte and your
  `doc/quorra-gpu-coverage.md` §2 states that promise deliberately. Nothing here touches it.
- **Across thread counts.** A frame drawn on 24 threads must be the same bytes as the same frame
  drawn on one. Coverage tiles are independent, so this is a property a design can *have* rather
  than approximate — but only if the sheet reservation and the instance stream are assigned before
  or after the parallel phase and never inside it. `crates/render-quorra/tests/corpus.rs` holds
  four lanes of our corpus to equality and would find a violation; we would rather you found it in
  your own gate.

**And the refusals must not move.** `REFUSED_AT_FOUR` is held by name to equality in this tree
(`QUORRA_FEEDBACK.md` §22.4). A parallel phase that changed how much scratch a frame commits would
fail that gate, correctly.

## 6. What it buys, and the ceiling we are stating ourselves

If `encode: geometry` went to **zero** — not to a twenty-fourth, to zero — this frame is still
**about 235 ms**: `recording` 83, `upload` 65, `execute` 29, the remainder ~52, our own `scene` 16.

So the honest claim is not sixty frames a second. It is:

- a zoom step of roughly **640 ms → 250–300 ms** on this machine, which is the difference between a
  stall and a step;
- and the same factor on every page whose marks are paths rather than glyphs, which is every
  drawing, map, plan and chart in a corpus rather than one file.

**We would rather say that now than have you build it and find we had oversold it.** If your side
prices the work above that, the answer is no and this document has done its job.

## 7. What happens if the answer is no

Nothing on our side blocks. Three things are already true and stay true:

- **A still window replays** — your ADR 0048 and our ADR 0351 — at 21.5 to 34.7 ms a frame here,
  with `encode` at zero. That was the case the owner's trace was full of and it is taken.
- **A repeat magnification costs 135 ms rather than 640**, because of your tile cache. A person who
  zooms out to where they were pays a seventh.
- **The rest is ours and we have priced it**: our own ADR 0368 enumerates a page-space scene under
  `Viewport`'s root affine (worth our `scene` phase, 2.4% of a zoom frame), batching by paint state
  (worth 1.0% of the commands on this document — 590 of 58 009 — and a loss once the merged
  bounding boxes reach your tiles), damage (zero: your `encode` never reads it), and dropping
  sub-pixel marks (forbidden outright by ISO 32000-2 §10.7.4's *"no shape ever disappears"*, even
  though 48.1% of this page's commands are under a device pixel at the fit view). None of them is
  worth building, and we are not building them.

What we would then do instead is decouple the gesture from the frame on our side — show the frame
already on the device while the next magnification is built — which needs nothing from you and
which we have not designed.

## 8. What we owe you, unchanged

Both of `QUORRA_API_2026_08_15.md` §6's asks are still open and still ours: the rectangular-fill
census and the `(clip_residue_regions, clip_residue_tiles)` distribution, one walk over our corpus
rather than two. This document does not displace them, and it did not do either — this page's
polygons were counted, not classified, and its residue count is zero for the trivial reason that it
states no clip at all.

**And one small thing that cost this round an hour**, offered as an observation rather than a
request: `render-quorra` reads `Timings` and drops `Timings::phases` on the floor, so the
subdivision your ADR 0023 built cannot be seen from a host of ours without a patch. That is our
omission and not your API's. If you would like the subdivision to be *routinely* visible from
here, say so and it becomes a `FrameCost` field; if not, it stays a probe a round adds when it has
a question, which is what it was this time.
