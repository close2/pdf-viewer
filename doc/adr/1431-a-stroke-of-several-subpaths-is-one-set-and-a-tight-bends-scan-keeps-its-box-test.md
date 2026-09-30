# 1431 — A stroke of several subpaths is one set, and a tight bend's scan keeps its box test

Status: accepted. Session 1298. Amends ADR 1421 section 6 (both of its "left" items are answered
here: one built, one measured and dropped) and ADR 1397's `Stroked::tiles` (a stroke of several
subpaths can be vouched for); keeps ADR 1375's construction and ADR 1407 section 2. Supersedes
nothing.
Context: ISO 32000-2 §8.4.3.2, §10.7.4; `CLAUDE.md` principle 2 and the rule on optimisations;
`doc/habits/measuring.md` 52–56; traps 54, 56, 58, 62, 67, 73; `doc/questions/A76`.
Code: `raster/crates/raster-gpu/src/raster/stroke/disjoint.rs` (`Subpath`, `tile_stroke`,
`each_alone`, `alone`, `tile`, `takes_part`, `the_rest_stand_apart`), `stroke/sweep.rs` (`Bounds`,
`meeting`), `stroke/convex.rs` (the convex arithmetic, moved out of `disjoint.rs` unchanged),
`raster/stroke.rs` (`stroke_pieces` builds a `Subpath` per subpath and tiles the stroke whole).
Tests: `raster/tests/stroke_set.rs` (`two_subpaths_that_cross_are_one_set`,
`two_concentric_rings_whose_strokes_overlap_are_one_ring`), `raster/tests/curve_join.rs`
(`two_subpaths_that_tile`'s four strokes join the vouched-for list; the refusal list's "two
subpaths" is now two segments at a corner beside a second subpath). Planted: with a stroke of
several subpaths vouched for without the tiling across them, the cross sums to 720 against 684
and the rings to 1 534.33 against 1 342.54.

**I did not open `crates/render-cpu/`.** Every expected value is the clause's closed form or a
measurement of raster against itself.

## 1. The sweep for the tiling's scan, measured and dropped

The brief asked for the separation questions to be asked only of pairs whose boxes meet, by a sweep
or a grid. A probe export counted one `frame_budget` round on `bug1743245.pdf`: the scan over
earlier pieces looks at **31.2 M** pairs, 1.9 M of whose boxes meet; 1.74 M of those still meet the
box round what is left of the piece (ADR 1421's lever (a)) and are asked; 7.7 M fragments are
tested, 6.5 M found apart, 1.2 M cut in 5.7 M splits. So the 31 M are box tests, a handful of
instructions each, and **the questions and the cutting are already asked only of pairs that meet
— 91% of the box-meeting pairs**, because in a bend with `hw > R` every body reaches past the
centre of curvature and genuinely overlaps nearly every other. The quadratic left is the geometry's.

Callgrind, one round, each arm its own export and target directory (thin LTO, `md5sum`-distinct),
M instructions, whole process (Type 3 / `bug1743245` / `issue14415`): base 1 865.3 / 7 420.1 /
1 355.9. A sweep by left edge building each piece's earlier-meeting list, and `takes_part` by the
same sweep: 2 018.5 / 7 645.7 / 1 405.3 (the lists 104 / 466 / 25, `takes_part` 68 / 476 / 41,
against a scan and a seed test that together cost less). Closing boxes by a second order instead
of filtering the open list: 1 989.1 / 7 394.9 / 1 412.9 — **+124 / −25 / +57**. Sorting twice per
bend and building the pair lists costs what the box tests cost. Dropped; `tile` keeps its scan.

## 2. A stroke of several subpaths is tiled as one (§8.4.3.2)

> stroking a path shall entail painting all points whose perpendicular distance from the path in
> user space is less than or equal to half the line width

The path is every subpath of it, so where two subpaths' strokes overlap the overlap is painted
once. `tile_stroke` builds each subpath's pieces, finds the pairs of pieces of different subpaths
whose boxes meet (`sweep::meeting`, bounded at `MAX_FRAGMENTS` comparisons) and asks each whether
they stand apart. None overlap: each subpath is tiled alone, exactly as before, and the stroke tiles
where each subpath does. Some do: those pieces seed one tiling of the whole stroke with the tight
bends' seeds, and the stroke tiles where that tiling vouches for every piece — two pieces of a
subpath that vouches for its own are not asked again. Past a bound, each subpath alone, not
vouched for.

Counted over the corpus's first pages (an export printing per page): **830 strokes of more than one
subpath on 43 pages reach the fill's set question today, 357 of them past its bound**, where the
integral counts every overlap; after, 211 are asked and 141 are past it (1×). At 4× on the page-turn
lane, 759 on 41 pages, 150 past the bound; after, 172 and 86.

## 3. Exactness

Hashed against the tree before, one encode thread: 1× 8 pages differ (`annotation-fileattachment`,
`issue12823`, `issue14415`, `issue19182`, `issue19360`, `issue9418`, `pr12564`, `zerowidthline`),
the compute lane at 1× the same 8, 4× 5 (`issue11473`, `issue12823`, `issue14415`, `issue19182`,
`issue19360`); a stroke of one subpath draws to the byte as before on every page. **Every stroke
mask that moved was looked at** (habit 53): where the new pieces are disjoint (a 64 × 64 sample per
pixel finds no point in two) each pixel is within **0.500** of 255 × the fragments' exact clipped
area, against up to 119 levels before; where pieces still overlap (pieces the tiling did not reach
or a bound, and the fill's question past its own) a 256 × 256 sample of the set has the new mask
nearer in every stroke — the worst after is 75.6 levels from the set, the worst before 118.3.
`issue19360.pdf`'s one 4× pixel: set 111.505, before 111, after 112.

## 4. What it costs

Final tree, same instrument: **1 865.3 → 2 169.6 / 7 420.1 → 7 406.7 / 1 355.9 → 1 479.0**. On the
Type 3 page the tiling across subpaths adds 347 M to the cutting and the fill's set question gives
back 213 M (`fill_mask_settled` 365.3 → 151.9); the overlap question costs 18 M once a pair whose
pieces are both marked is passed over (90 M before). The hook and the tight L, `stroke_pieces`
inclusive: 6 190 880 → 6 199 883 and 247 466 → 251 237. Pinned `frame_budget`, minimum of three runs
of five rounds, load 1.1–3.4, before → after, ms: Type 3 turn 9.46 → **11.28**, step 11.35 → 11.57;
`bug1743245` turn 46.09 → 44.34, step 43.83 → 41.22 (its strokes are one subpath each; the path is
the same); `issue14415` turn 11.62 → 12.29, step 5.71 → 5.95.

**Kept at that cost, as a fidelity decision.** The Type 3 page moves no pixel for its 1.8 ms: its 72
crossing strokes draw the same bytes either way. The pages above had rims up to 119 levels too dark
(the least, `issue19182.pdf`'s, 11; `issue11473.pdf`'s two 4× pixels a rounding's half level).
Dropped besides: vouching only for subpaths that stand apart (+169 / +3 / +45 M, moves nothing and
corrects nothing that crosses).

## 5. Left

The lever that would give the Type 3 page its 1.8 ms back without losing a correction: tile across
subpaths only where the fill's set question is past its bound. The fill knows that and the stroker
does not; it needs the job's region, so the choice belongs where `encode/parallel.rs` calls the two
(and `encode/stroke.rs`'s path, whose pieces also reach the GPU lane's triangles, which never ask).
A fragment cut between two crossing points that round to nearly one `f32` point can hold an area of
rounding's order and either sign (1e-8 against a piece's tens, seen on the rings); the fixtures
read signs above 1e-6. The Type 3 cell's per-placement tiling is still the encoder's (ADR 1375
section 2).
