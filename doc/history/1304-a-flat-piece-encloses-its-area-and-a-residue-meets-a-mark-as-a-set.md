# 1304 — A flat piece encloses its area, and a residue clip meets a mark as a set

Raster, from the clauses alone: feedback section 59's two asks and ADR 1431's slivers. ADRs 1443,
1444. No question. `crates/render-cpu/` was not opened (A76); every expected value is a closed form.

**Flattening (ADR 1443).** One chord on the curve was 19.7 levels light per unit of rim past r = 6.
Eight candidates measured on circles against Green's closed form; built: each flat piece is two chords
through the midpoint of its inner controls (exact area at the thirds, second order elsewhere, ≤ 0.04
levels a unit of rim, 2.0–2.9 per pixel), division-free, mirrored in the compute lane. Round joins/caps
pair their chords through radius `r·(a/2)/sin(a/2)`, same vertex count. `issue2177.pdf` against a
rebuilt geometric reference (reproduces 1300's 0.71 / 2.05 as 0.64 / 2.00): raster 0.197; against the
oracle 1.9385 / 11.54 → 0.4930 / 6.42, so it agrees and `DIFFERS_AT_THE_EDGES` is empty.

**Residue (ADR 1444).** `min` on the CPU residue and `image.wgsl`. 64-gon over 99 rectangles, 48
doubly-fractional pixels: product 15.4 mean, −44 to +43; `min` +22, never below. Worst tile: the
flattening alone 7.42 → 1.09, then `min` 1.35.

**Slivers.** A cut through a vertex another piece holds one ulp away (and zero-area fragments).
Dropped where oriented area ≤ `m·f32::EPSILON` × box perimeter, `f64`; rings fixture 2 of 461 → 0.

**Corpus, digests against HEAD** (`PDFVIEWER_RASTER_TIMES` now writes digest and mean; HEAD and
HEAD-plus-this-round in separate trees): 1× 645 moved, 599 toward the oracle, 33 equal, 13 away — glyph
pages whose ink ladder moved toward the set (`pr12564` 60238 → 60424, limit ~60420). 4× 641 moved, 635
toward, 0 away. Gates on the worktree: 1× 961/0/6/7, 4× 958/0/8/8, gpu 960/1 (`bug1743245`, HEAD's,
2.6131 → 2.6102), compute 960/0/7 (`issue1905`'s refusal is HEAD's); 1-vs-N 0 on all four.

**Cost** (callgrind, one frame_budget round, M Ir): page 101 862 → 913, `issue14415` 1418 → 2126,
`bug1743245` 7100 → 7213, `personwithdog` 934 → 937. Removed on the way: a `hypot` per fragment in the
sliver test (+14% on `bug1743245`) and doubled arc vertices (+7%).

**Left: the stroker's hairpin.** A stroke test one halving coarser keeps `issue14415` at +3.9% but on
`inks.pdf` at 4× (4 w, round caps and joins) draws a piece 21 px from a centreline of half-width 10:
the tail `…(1172.34, 909.50) (1170.65, 910.31) (1170.15, 910.96) (1172.94, 911.12)` with the curve's
arriving tangent `(10.2, 0)` at the last point. Not built until that is answered. And the exact `S ∩ C`
in doubly-fractional rim pixels (needs the clip's polylines beside its region).

**Gates.** rustfmt (own files, via stdin) 0; clippy raster-gpu + render-raster 0; nextest 727 pass;
conformance 0; corpus as above; headless_gpu 39 pass; raster_golden 0 (unchanged — no CPU pixel);
launch_path 0. Proposed trap: a flattening's error is read per unit of rim against the curve's own
closed-form area (Green), never against πr² or another renderer — the one-chord loss hid at 19.7.
