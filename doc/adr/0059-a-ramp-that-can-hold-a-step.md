# ADR 0059 — A ramp that can hold a step

Status: accepted, 2026-07-31.

## Context

`issue10572.pdf` was top of the ratio-ranked unexplained list at 2.55. It is twenty-four hard
stripes of green and blue, and poppler, ghostscript and `hayro` draw every boundary as one
pixel. We drew each as a **seven-pixel gradient**.

The file says exactly what it wants, twice over. Its axial shading's `/Function` is a type 3
stitching function over `[-6, 6]` with eleven bounds, and each of its twelve sub-functions is
*itself* a type 3 with `/Bounds [0.5 0.5]` — a subdomain of zero width. That is how a producer
writes a discontinuity: constant green below the point, constant blue above it, and a middle
sub-function that no input ever reaches.

Seven pixels is not a coincidence. `Ramp` held 256 colours sampled at even intervals, the
shading's axis is 1800 units long, and 1800 ÷ 256 ≈ 7. Every step landed inside one sampling
interval and the two backends interpolated across it, because that is what a gradient does
between two stops.

## Decision

**A ramp carries a position per stop**, not just a colour:

```rust
pub struct Stop { pub at: f32, pub colour: Color }
pub struct Ramp { pub stops: Arc<[Stop]> }
```

Both backends already built their gradient stops *with* positions, computing them from the index
and the length; they now read `stop.at` instead. The change is three lines in each.

**`Function::breakpoints` reports where the standard allows a jump.** §7.10.4's type 3 function
stitches sub-functions across subdomains, and its `/Bounds` are the only places a 1-input
function may be discontinuous — types 0, 2 and 4 declare nothing of the kind. Nested stitching
functions are followed, with each inner bound mapped back through `/Encode` and the subdomain it
belongs to, because a jump inside a sub-function is a jump of the whole. That mapping is the
inverse of what `eval_stitching` applies on the way in, so the two cannot drift apart without a
test failing.

**`Ramp::sample_across(breaks, f)` puts two stops at each break** — one sampled just below it and
one just above — and spreads the remaining samples evenly inside each interval. Two stops at one
position is precisely how a gradient expresses a step, in `tiny-skia` and in Vello alike.

The total stop count stays near `RESOLUTION`: a function with many bounds gets fewer samples per
interval rather than more stops overall, because the stops are what a rasteriser walks per pixel
batch (the forty-sixth session's profile has `tiny_skia::pipeline::lowp::gradient` at 29.7% of a
shading-heavy page). This change is therefore neutral on that cost by construction, and the
corpus gate's wall time is unchanged.

## Why not simply sample more finely

Because it does not fix it. Doubling the resolution halves the width of the smear; it never
reaches zero, and the error is a *step* rendered as a ramp rather than a small colour error. At
4096 samples the smear on this page would still be half a pixel and every shading in every
document would carry sixteen times the stops. The clause states where the function jumps, and a
reader that has read the clause does not need to guess.

## Consequences

- 821 pages agree with the reference consensus and 76 are contradicted, from 820 and 77.
  `CONTRADICTED_UNEXPLAINED` is 30.
- `Ramp::colour_at` now searches for the bracketing stops instead of indexing. It is called twice
  per gradient — for the transparent cut-off stops at a non-extended end — so a linear scan over
  256 entries is not on any hot path.
- The representation is now the one a *smaller* ramp would need. Nothing takes advantage of that
  yet: a two-stop linear gradient is still sampled at 256 points, and reducing that is a
  measurable optimisation for another session with the profile in hand.
