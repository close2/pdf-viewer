# 1310 — A point inside a curve is no corner, and the exact residue meet is priced

Raster, from the clauses alone: the hairpin and the exact `S ∩ C` ADRs 1443 and 1444 left. ADRs 1455,
1456. No question. `crates/render-cpu/` was not opened (A76); every expected value is the set's.

**The hairpin (ADR 1455).** Reproduced on `inks.pdf` at 4× with the coarser stroke test by printing
every piece reaching past `hw + 0.75` of its chords: one, a **miter** join (vertex, two outer corners,
tip at 21.3 px), at the third-last point of one cubic's flattening, turning 124°. The stroke joins by
miter, not round as 1304's record said; the ratio 2.1 is under the limit of 10, so §8.4.3.5's bevel
does not fire. §8.4.3.4: join styles are "significant only at points where consecutive segments of a
path connect at an angle" — a curve is one segment, so a join there is wrong at all. `Centre` now
marks points inside one curve (between its recorded leaving and arriving directions) and joins them
round, which is §8.4.3.2's disc. Fixture `inside_a_curve.rs` (the tail as one curve and as three
lines, per pixel against the distance set sampled 32 × 32): failed on 13 wedge pixels before, holds
after (worst 31.9 levels on arc rims, ink 99155 vs 99161).

**Corpus**, digests with and without in one binary: 1× 36 moved, 4 closer to the oracle, 31 equal,
1 further (`rotated_ink` 0.0351 → 0.0352, whose ink ladder moves toward its 8× limit at every rung);
4× 45 moved, 4 closer, 41 equal, 0 further.

**Coarser flattening re-measured.** `inks.pdf` 4× now agrees under it, but on stroked rings against
the annulus the rim error goes 1.7–3.4 → 4.6–9.0 levels a pixel (worst 15.6 → 45.2). Declined.

**Exact `S ∩ C` (ADR 1456), priced.** Needs the mark's polylines at the fan-out's commit, the clip's
beside its cached region (both in `encode/` files of 1311's) and a two-winding slab sweep; built on
the walk's path alone it would break one-vs-many. At 1× the whole corpus has 13 340 doubly-fractional
residue pixels, interval width 43.5 levels mean, 2 275 pixels of area in all. `min` stays.

**Gates.** rustfmt (own files) 0; clippy raster-gpu + render-raster 0; nextest 731 pass; conformance
0; corpus 1× 961/0/6/7 and 4× 958/0/8/8, one-vs-many 0; headless_gpu 39 pass; raster_golden 0
(unchanged, CPU backend); launch_path 0. Proposed habit: before calling a stroke piece wrong, ask whether the point it sits on is a
corner of the path or only of its flattening.
