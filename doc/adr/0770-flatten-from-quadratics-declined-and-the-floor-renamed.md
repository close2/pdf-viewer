# 0770 — Flatten-from-quadratics declined on its numbers, and the kernel floor renamed

Status: accepted — a measurement round with a priced refusal; the one change shipped in
this tree is an instrument knob.
Context: `doc/todo/46-the-kernel-floor.md`, which named flatten-from-quadratics the only
untried structural idea with the moved-view step's magnitude and argued its determinism
statement both ways; quorra's ADRs 0082 (the owner's relaxed contract), 0091 (the
register-pressure suspicion and stage B's two design debts), 0092 (two kernel
experiments declined), 0095 (stage B as one submission); this tree's ADR 0767 (the lane
the window takes, and the decomposition this round re-reproduced).
Code: quorra ADR 0096 in `/home/cl/projects/render-lib` (the experiment, its table, and
the revert); `crates/render-quorra/tests/corpus.rs` (`PDFVIEWER_QUORRA_COVERAGE` now
accepts `compute`).

## 1. What was built, where, and how it was held honest

The experiment was implemented whole in quorra's `compute.rs` — the resident arena
carrying `push_cubic`'s quadratic conversion (the GPU triangle lane's own geometry,
within their stated Loop–Blinn bound), the kernel flattening each quadratic with a
closed-form chord count and **no per-thread stack** where the cubic recursion held ~170
scalars — with the Cpu↔Compute identity restated as a derived band under their ADR 0082
(option (a) of `doc/todo/46`'s argument: the CPU oracle unchanged, the divergence
confined to a curve boundary's own neighbourhood, one summation step elsewhere). Their
whole 632-test suite was green on both adapters, with the byte-for-byte claim kept on
straight geometry, before any number was taken.

## 2. The numbers (890M, headless, Entwurf p1, ADR 0767's sequence, one interleaved sitting, load < 1.1)

| arm | warm step (ms) | count | emit+deposit | first compute frame |
|---|---|---|---|---|
| cubic kernel (quorra `5fb011a`, the pin) | **62.0–66.7** | 13.2–19.9 | 27.8–29.9 | 341–343 |
| quads, tolerance read per quad | 65.9–66.1 | **9.7–9.9** | 35.7–36.6 | 454–476 |
| quads, tolerance read per source cubic | 62.5–63.7 | 12.4–13.0 | 29.8–30.5 | 444–450 |

One further split, taken by rescoping the coverage timestamp to the emit pass for a
sitting: **emit ≈ 16.5 ms, deposit ≈ 13.5**. And the corpus-scale control, run through
the new gate knob on this machine both ways: **the divergence population is unchanged to
the page** — 957 compared, 932 agree, 22 differ, 3 refused, the same page lists, per-page
means within ±0.03.

## 3. The decision, and what it refutes

**Declined and reverted in quorra; nothing about the kernels ships from this round.**
Removing the whole per-thread stack — not shrinking it — moved the count pass 30–45% and
the step not at all, so the occupancy suspicion their ADR 0091 named is measured and
refuted: the passes are bound by the serial per-tile arena walk (~85 MB streamed twice by
one thread per tile) and by edge traffic (~64 MB written by emit, re-read per row by the
deposit). Keeping the change would have bought a step difference inside run-to-run noise
at the price of +100–130 ms on the first compute frame of the worst page, +4% of resident
arena, and the byte-for-byte lane identity — the sharpest canary either tree owns.
`doc/todo/46` is re-pointed at the real floor; what remains with the step's magnitude are
quorra-side designs (a packed arena, intra-tile parallelism, tile-local edge runs), each
owing its own argument. The transferable lesson is priced in quorra ADR 0096: a relative
flatness tolerance is the *curve's*, and read off a fragment of the curve it
over-flattens by the square root of the split — ~1.7× the edges, +28% of emit+deposit.

## 4. The knob, and what its first run showed

`PDFVIEWER_QUORRA_COVERAGE=compute` points the render-quorra corpus gate at the lane
`surface::lane_for` takes for **every moved view on a real adapter** (ADR 0700) — a lane
no corpus-scale instrument in this tree could name, trap 12b's shape exactly as ADR 0767
found it for the measurement knob. Like the other lane knobs it turns the ratchets off
and says so. Its first survey (this machine's 890M, scale 1): 932 agree, 22 differ, 3
refused of 957 — and one of the refusals is worth a name: `issue1905.pdf` refuses on the
compute lane for frame bytes (272 MB against the 256 MB budget), so a drag on that page
draws through the CPU fallback by design. Nothing ratchets on any of this; the command is
the record.

## Held by

Quorra ADR 0096 (the experiment's full table and revert, committed there); the corpus
gate knob and its doc comment in `tests/corpus.rs`; `doc/todo/46` restated around the
measured floor. The quorra pin is unchanged (`5fb011a`), so no shipped pixel path moved
in this tree.
