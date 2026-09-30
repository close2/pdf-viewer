# 1435 — A clip rectangle meets a mark as a set, and a differing page is held to a bound

Status: accepted. Session 1300. Amends ADR 0355's "quorra has not" (the mark-side product at a
clip rectangle, `doc/QUORRA_FEEDBACK.md` section 24); supersedes nothing. Context: ISO 32000-2
§7.4.1, §8.5.4, §8.7.3.1 (Table 74), §10.7.2, §10.7.4; traps 13, 26, 65; habit 53; A76's rule
that raster's expected values are not read out of `render-cpu`.
Code: `raster/crates/raster-gpu/src/shaders/{coverage,image,shading,function_lane}.wgsl`
(`shape_at`), `crates/render-raster/tests/corpus.rs` (`Held`, `DIFFERS_AT_THE_EDGES`,
`DIFFERS_IN_SHAPE`, `Outcome::Differs`, `hold`, `NOT_COMPARABLE`'s note),
`fuzz/fuzz_targets/revocation.rs`.
Tests: `raster/crates/raster-gpu/tests/clip_meets_a_mark_as_a_set.rs`.

## 1. The clip rectangle, from the clause

§10.7.4: "Subsequent painting operations shall affect a region that is the intersection of the set
of pixels defined by the clipping region with the set of pixels for the region to be painted." In
one pixel the area of `S ∩ C` lies in `[max(0, s + c − 1), min(s, c)]`, and equals `min(s, c)`
wherever one set contains the other there. Four shaders returned `s × c`, which is below the area
wherever the two edges coincide: `issue19083.pdf`'s border rule, `0.5 0.5 124.2502 19 re s` under
its `/BBox`, kept 0.75² of its top row (112, where the clause's value is 64) and 0.25² of its left
column (239 for 191). The same paragraph fixes the direction of any error left: "[t]he area covered
by painted pixels shall always be at least as large as the area of the original shape." So:

- where the lane holds **both rectangles** (an axis-preserving image, an analytic coverage
  rectangle) they are intersected before the cell overlap is taken, and the area is exact;
- where it holds a **coverage byte**, which says how much of the pixel and not where, the byte
  meets the clip by `min`: exact at a coincident or nested edge, the upper bound elsewhere.

The fixture holds five pixels from that arithmetic; four fail under the product (watched, trap 13),
the fifth — a right-angle crossing on the image lane — is the control where the product was right.
Corpus gate at 1×: 959 / 2 / 6 / 7 became 960 / 1 / 6 / 7, `issue19083.pdf` the only name to move.

## 2. The page still differing is held to a bound, derived

`issue2177.pdf` was read against a per-pixel reference built from the page's geometry alone:
each mark's area intersected with its clip inside `[i, i+1) × [j, j+1)`, composited in paint order,
sampled 128 × 128 a pixel (0.016 of 255 from 64 × 64). Oracle 0.71 of 255 from it, raster 2.05;
page ink 13013.8 reference, 13030.3 oracle, 12933.9 raster. Two causes, each measured:

- **Interior: raster's flattening.** Chords 0.25 device pixel from the curve, inscribed. Pre-
  flattening the rings at 1/256 takes raster to 0.39 of the reference and the page mean from 1.94 to
  0.68. §10.7.2 permits the tolerance; §10.7.4's "at least as large" is the side it misses. Raster's
  `src/raster/flatten.rs` is another round's file, so it is an ask (feedback section 59), not a fix.
- **Worst tile (32, 224), one pixel tall**: the pattern's cells meet the circle's own path as a
  clip, a residue raster multiplies; there the oracle is 8.5 heavy and raster 7.2 light of the
  reference. Table 74's `/TilingType 2` latitude ("may vary by as much as 1 device pixel") is taken
  by neither lane.

So the list now carries a `Held { mean, worst_tile }` per page — measured 1.9385 / 11.54, held at
1.94 / 11.6 — and a page that grows worse on its list fails (planted at 1.90: it failed, naming it).

## 3. The refusals and the incomparable, decided without moving anything

- The six refused at 1× are four `REFUSED_BEFORE_THE_SCENE` (vocabulary raster lacks, section 43)
  and two the device refuses: the cycle's four million commands (a budget doing its job,
  `doc/todo/49`) and `issue19517.pdf`'s 12608 × 16806 target past the adapter's 16384 per side (a
  capability, not a budget). At 4× `issue12810` (609086160) exceeds the 268435456 scene-byte budget
  for a **whole-page** target of 71.7 megapixels, and `issue1905` and `issue9418` meet the coverage
  sheet's 16384-texel ceiling (a capability); a window's
  viewport at 4× asks a fraction of that. The budget (raster's brief section 5, `max_frame_bytes`)
  is the right instrument and it is not moved, so no census is owed. A refused frame is reported
  out loud — `quorra-confined`'s `frame_landed` prints the refusal and the pages falling back.
- The seven not comparable are the document's limits, not a lane's or the instrument's: two will
  not open, four have no page tree a reader reaches, and `Brotli-Prototype-FileA.pdf` compresses
  its page tree with a filter §7.4.1's Table 6 does not name.

## 4. The revocation target reached past the name comparison

It asked every input about one certificate whose issuer and serial no input reaches, so section
6.3.3's walk stopped at the issuer comparison and the search's matching branch never ran. It now
also asks, per list, the serials the list itself carries (a listed serial found or refused, never
"not listed") and a certificate the list names — reaching the signature, which no key it holds
verifies, so `Good` stays forbidden — and, per OCSP response, the first single response's serial.
Coverage figures are in the round's record.
