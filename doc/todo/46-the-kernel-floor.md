# The kernel floor: the zoom step is now the compute kernels and nothing else

Status: **open — the last structural idea on the table, and the only remaining item with a
real payoff.** Everything the host used to add on top of the kernels is gone: quorra's stage
B (their ADR 0095) made the compute lane one submission with no mid-frame readback, and the
frame's trace shows zero stall spans.
Priority: 46 — performance, measured.
Corpus: the owner's `tmp/Entwurf.pdf` (58 009 fills, the worst page by a wide margin);
`doc/ISO_32000-2_sponsored_EC3.pdf` dense text at high magnification is the second witness.
Code: `render-lib/crates/quorra-gpu/src/compute.rs` (the count → scan → emit → deposit
chain and its WGSL); the CPU mirror it is held to is `render-lib`'s port of
`pdf-render`'s `fill.rs` arithmetic.
Instrument: the compute lane's own pass timestamps (quorra's `ComputeQueries`, in the tree
since their timestamps commit), read off any traced run — the numbers below are the 890M's,
recorded beside quorra ADR 0095, and are re-measured rather than trusted.

## Where the time is

After stage B, an Entwurf zoom step is 53–66 ms headless on the 890M and decomposes as:
encode 7–10 (host — [`47-the-encode-term.md`](47-the-encode-term.md)), upload 4–5, and
**"content beyond pass" 39–49, which is the GPU chain itself: count 12–18 ms, emit+deposit
24–28 ms**. The kernels' costs move together (their ADR 0092's finding), so this file is one
item and not two.

## What was tried and declined, so nobody re-buys it

Quorra ADR 0092 built and measured both cheap experiments and reverted them with numbers:

- **Partition by `has_cubics`** — refuted by the populations: Entwurf is 58 000/58 000
  all-cubic tiles, dense text 7 426/8 470. Glyphs are curves; there is no straight-segment
  fast path worth dispatching.
- **Closed-form subdivision bound instead of the exact count** — the bounds pass was 3.5×
  cheaper than counting, and the frame was *worse*: deposit paid ~6 ms for bound-slack
  sparsity in the edge buffer. Stage B then took the only shape that removes the sync and
  the sparsity together, which was that ADR's conclusion.

## The untried idea, and what it costs

Their ADR 0091 named the occupancy problem: `flatten_cubic` holds ~170 scalars of
per-thread state, which caps how many warps a CU can keep in flight through both hot
passes. The one structural idea not yet built: **flatten from the resident quadratic
forms instead of from cubics.** Every outline already carries its quadratic conversion on
the device (their ADR 0075's deferral put it there); a quadratic flattens with roughly half
the per-thread state and a shallower subdivision, through both count and emit.

The cost is the determinism statement, and it has to be argued before the round starts:
the compute lane's edges are currently the CPU mirror's bit-for-bit, and ADR 0094 holds
Cpu↔Compute to one coverage step. Flattening from quadratics changes the emitted geometry
— the quadratic forms are themselves an approximation with Loop–Blinn's stated bound — so
the round either (a) states a new bound under the owner's relaxed contract (quorra ADR
0082: close best effort with stated bounds is enough), or (b) moves the CPU lane to the
same quadratic source so the lanes keep agreeing to a step. Option (b) also changes the
oracle's pixels and therefore touches every gate; option (a) is cheaper and the contract
explicitly permits it. Either way `tests/lane_bound.rs` is the gate that has to keep
holding, possibly at a restated bound.

Plausible payoff if occupancy is the wall it appears to be: the Entwurf step from ~55 ms
toward the 30s. Nothing else left in this arc has that magnitude.

## Two small remainders beside it, named here rather than given files

- **First-frame capacity heuristic.** The persistent edges buffer starts from a guess
  (`tiles × 32`), so a cold first compute frame can pay one grow-and-rerun of the whole
  frame. Harmless in steady state — a steady zoom pays zero — and the forced-growth gate
  (`capacity_growth.rs` under the `sabotage-capacity` feature) holds the road. Tunable
  only if a first-frame number ever comes to matter.
- **The hybrid's admission race** (quorra ADR 0093's own note): a queued other-key job can
  still beat an admitted tile into the atlas, and the tile falls through at commit exactly
  as before the probe. Documented, benign, no measured cost; recorded so it reads as a
  decision rather than an oversight.
