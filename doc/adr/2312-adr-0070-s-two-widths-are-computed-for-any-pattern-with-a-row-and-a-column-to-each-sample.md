# 2312 — ADR 0070's two widths are computed for any pattern with a row and a column to each sample

Status: implemented. Session 1739, 2026-10-24, the pixels slot of batch one hundred and thirteen,
under the project owner's switch to code improvements and bug fixes of 2026-10-14. Clause: ISO
32000-2 §10.7.4 (the row stays `departed`), read with §10.7.1's NOTE that the scan-conversion
algorithm is not defined by PDF. Context: raster's ADRs 0070 and 0076; ADR 2296. Code:
`raster/crates/raster-gpu/src/winding/pattern.rs` (new), `encode/thin.rs`'s `vanishing_width`.
Amends raster's ADR 0070 (where its threshold comes from) and ADR 2296 section 3 (which computed
the widths for lattices only).

## 1. The premise, checked

ADR 2296 left a searched sixteen-row table unshipped because "its band width a lattice's closed
form cannot give". It held: `SampleLattice::widest_empty_band` is `1/√m` for the lattice's
shortest vector, a formula that exists only where the points are a lattice. The brief's
hypothesis — that the widths of an arbitrary pattern are computable over its point pairs — holds
in the form below, which needs no lattice structure at all.

## 2. The two widths, for any pattern of the lane's shape

The lane's shape: `n` samples, sample `i` in column `i` and row `rows[i]` for a permutation
`rows` of `0..n`, repeated in every pixel. `SamplePattern` holds one.

- **The widest empty band.** A band along a direction of irrational slope meets the pattern's
  copies densely, so it is never empty at any width; only directions of primitive integer vectors
  `(p, q)` count. Along one, the plane's copies project onto the normal with period `1/|(p, q)|`
  (in units of `1/n`, the projections `p·row − q·column` repeat modulo `n` because `p` and `q`
  share no factor), so the widest empty band in that direction is the widest gap between the `n`
  projections modulo the period, over `n·|(p, q)|`. It is at most `1/|(p, q)|`, and the
  horizontal direction already gives `1/n`, so no vector longer than `n` can beat it: the search
  over directions is finite, and it skips a direction once `1/|(p, q)|` cannot beat the widest
  found.
- **The widest empty square**, open, axis-aligned. Slid left and then down until it touches a
  sample on each of those sides, a largest one has its corner at one sample's column and
  another's row — `n²` corners — and grows from each until it meets the first sample above and to
  the right, in the pixel or its neighbours.

Both are re-derived in the module's tests by brute forces that prune nothing — every integer
direction up to the pixel's width, and every corner against the eight neighbouring pixels — over
the lane's pattern at each admitted count and over every rank-1 lattice of every generator at
each, and on the lattices they agree with ADR 2296's closed forms to an `f32`'s rounding. Two
hand cases hold the arithmetic: four samples on the diagonal give `1/√2` and `3/4`.

## 3. ADR 0070's argument, re-derived on them

ADR 0070 keeps a mark on the processor lane when its thin axis — the narrower side of its device
box, or a stroke's own width — is below the width at which the lane could miss it. The argument
uses nothing of the lattice: a box with both sides at least the widest empty square holds a
sample, and a band at least the widest empty band holds one at any angle, for any point set
whatever. So the threshold is `SamplePattern::vanishing_width`, the larger of the two, computed
from the pattern the lane draws with, and `encode/thin.rs` keeps each count's once.

What the two readings do not cover is unchanged: a mark short along its length and turned, whose
box and width both reach the threshold, is bounded by neither, for the lattice as for any other
pattern — ADR 0070's 45° fill is that residual and this ADR neither narrows nor widens it.

**A pattern the lane ships is held to neither width wider than its lattice's.** That is the
constraint ADR 2313's search runs under: the threshold cannot rise, so no mark the lattice drew on
the device is sent back to the processor by the change of pattern.

## 4. Alternatives

- **Keep the widths a lattice property and ship only lattices**: ADR 2296's position, which
  leaves every non-lattice pattern unmeasurable — including the one that does better on the arms.
- **A third width for short turned marks** — the widest empty square at any angle: it would
  narrow the residual of section 3, and it is a change to ADR 0070's condition rather than to the
  pattern, so it is not taken here.

## Revisit when

- ADR 0070's condition gains a reading of a mark's length, which would let the third width of
  section 4 decide something.
