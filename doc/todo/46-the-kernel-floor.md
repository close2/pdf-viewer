# The kernel floor: the zoom step is the compute kernels, and the floor is the walk, not the registers

Status: **open — re-priced.** The last structural idea this file carried,
flatten-from-quadratics, was built whole and declined on its numbers in session 843
(quorra ADR 0096, this tree's ADR 0770). What remains has the step's magnitude only as
*designs*, each with its own argument still owed; nothing here is an evening's
experiment any more.
Priority: 46 — performance, measured.
Corpus: the owner's `tmp/Entwurf.pdf` (58 009 fills, the worst page by a wide margin);
`doc/ISO_32000-2_sponsored_EC3.pdf` dense text at high magnification is the second witness.
Code: `render-lib/crates/quorra-gpu/src/compute.rs` (the count → scan → emit → deposit
chain and its WGSL); the CPU mirror it is held to is `render-lib`'s port of
`pdf-render`'s `fill.rs` arithmetic.
Instrument: the compute lane's own pass timestamps (quorra's `ComputeQueries`), read off
any traced run — `ZOOM_FRAME_COVERAGE=compute` on `examples/zoom_frame` (ADR 0767's
knob); and since session 843 `PDFVIEWER_QUORRA_COVERAGE=compute` points the corpus gate
at this lane.

## Where the time is

Re-measured in session 843 (ADR 0770), consistent with rounds 840 and earlier: an
Entwurf moved-view step is 62–67 ms on the 890M — kernels 42–47 (count 12.5–19.9,
emit ~16.5, deposit ~13.5, the emit/deposit split taken by rescoping the coverage query
for one sitting), host encode 9–10 record-replayed, residency+records ~4, transfer ~5.

## What was tried and declined, so nobody re-buys it

Quorra ADR 0092 built and measured the first two, ADR 0096 the third; all three were
reverted with numbers:

- **Partition by `has_cubics`** — refuted by the populations: Entwurf is 58 000/58 000
  all-cubic tiles, dense text 7 426/8 470. Glyphs are curves; there is no straight-segment
  fast path worth dispatching.
- **Closed-form subdivision bound instead of the exact count** — the bounds pass was 3.5×
  cheaper than counting, and the frame was *worse*: deposit paid ~6 ms for bound-slack
  sparsity in the edge buffer. Stage B then took the only shape that removes the sync and
  the sparsity together (their ADR 0095).
- **Flatten from the resident quadratic forms** — this file's own last idea, built end to
  end: the arena carried `push_cubic`'s quadratics, the kernel flattened them with a
  closed-form chord count and **no per-thread stack**, and the Cpu↔Compute contract was
  restated as a derived band under quorra ADR 0082. The count pass fell 30–45% (13.5–19.9
  → 9.8 ms) and **the step did not move** — the ~170 scalars of per-thread state quorra
  ADR 0091 suspected were not the wall. Costs of keeping it: +100–130 ms on the first
  compute frame of the worst page (conversion at residency plus a third more arena to
  upload), +4% resident arena, and the byte-for-byte lane identity — the sharpest canary
  either tree owns — traded for a band, for a step change inside run-to-run noise. On the
  pdf.js corpus at the gate's scale the divergence population was unchanged to the page
  (932 agree / 22 differ / 3 refused, same lists, means within ±0.03). Quorra ADR 0096
  carries the table and the one transferable lesson: **ADR 0044's relative flatness
  tolerance is the cubic's, never a fragment's** — read per quad it over-flattened by
  ~1.7× the edges and +28% of emit+deposit.

## Where the floor actually is

The three declines triangulate it: the hot passes are bound by the **serial per-tile
arena walk** (one thread per tile streaming ~85 MB of segments, twice — count 12.5 ms
with nearly zero arithmetic in it) and by **edge traffic** (emit writes ~64 MB of edges,
~16.5 ms; deposit re-reads them per row, ~13.5 ms). Ideas with the step's magnitude are
therefore structural and quorra-side, each a design of its own: fewer bytes walked (a
packed arena format), parallelism inside a tile (segments across threads with a
device-side scan), fewer edges deposited (tile-local runs). Quorra ADR 0091's two stage-B
design debts (a scan-computable seat; a refusal that keeps its name without a sync) are
still open and still gate the other road, device-resident records
([`47-the-encode-term.md`](47-the-encode-term.md)).

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
