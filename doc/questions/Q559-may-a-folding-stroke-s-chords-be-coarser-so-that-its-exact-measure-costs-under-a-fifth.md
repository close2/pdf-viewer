# Q559 — May a folding stroke's chords be coarser, so that its exact measure costs under a fifth?

Source: session 1664, ADR 2164, on `doc/questions/A538` ("agreed with wait"): the exact measure is
routed once it costs under a fifth more on `inks.pdf`, `rotated_ink.pdf` and `issue14415.pdf`.

## Why it needs the owner

The levers inside the measure do not reach the fifth (ADR 2164 §2–§3): the cost is the pairs of
pieces that overlap, and a fold's pieces are the bands of chords flattened to a sixty-fourth of a
pixel, chosen against the sixteenths' step (ADR 1348). What reaches it is coarser chords — §10.7.2
leaves the flatness to the processor ("PDF processors may choose to ignore any flatness tolerance
specified within a PDF file"), and §10.7.4 measures the flattened set. But the round's correctness
gate, the paperclip of veraPDF's `6-3-1-t01-fail-f` against the stroke drawn sixteen times finer,
compares with the true curve, so coarser chords move it by the flattening's own error:

`callgrind_rasterise`, page 1, three draws, scale 1, one thread; HEAD draws the sixteenths at 1/64.

| chords | `inks.pdf` | `rotated_ink.pdf` | `issue14415.pdf` | paperclip pixel (ladder) |
|---|---|---|---|---|
| HEAD, sixteenths | 283 228 522 | 16 958 381 | 985 610 733 | 255 (242.3) |
| 1/64, exact | +145.3% | +45.6% | +16.8% | 243 (242.3) |
| 1/32, exact | +55.2% | +22.5% | +10.4% | 245 (242.3) |
| 1/24, exact | +29.9% | +14.9% | +8.9% | 245 (242.3) |
| 1/20, exact | +14.5% | +7.7% | +7.8% | 245 (242.4) |
| 1/16, exact | +2.7% | +4.1% | +6.4% | 247 (242.5) |

At the pool's own width the strips each find a mark's boundary again: at 1/20 that is +21.8%,
+63.6% and +6.9%, and bringing those under a fifth is a second lever — one boundary per mark shared
by the strips that cross it — which this round did not build.

## What the tree does meanwhile

The sixteenths at 1/64, as before: no pixel moves. The exact measure is the patch at
`/home/AI/r1664-folding-union/folding-union-boundary.patch` (applies to `66c6ef23`, with its ladder
test); the chords are one constant in `crates/render-cpu/src/stroke/folding.rs`, `FOLD_FLATNESS`.

## Recommendation

**Take chords at 1/20 of a pixel with the exact measure, and restate the gate as the flattened
set's.** Every page is then under the fifth by the instrument ADR 2122 priced with, and every rim
pixel is nearer the curve than today: the paperclip reads 245 against the curve's 242.4 where the
sixteenths read 255, and a pixel is bounded by the chords' own error, √2/20 of its area (18 levels),
where the sixteenths' band reaches 50 (ADR 2106 §4). The gate would then hold the measure to the
flattened set — the pieces flattened at the page's scale, drawn sixteen times finer — and the
paperclip to the flattening's bound against the curve. A round building it shares one boundary
between the strips that cross a mark before routing, so that the twenty-four-thread figures are under
the fifth too. If the answer is instead "keep the sixty-fourth", the measure waits on that second
lever and on a cheaper construction ADR 2164 §4 does not yet know.
