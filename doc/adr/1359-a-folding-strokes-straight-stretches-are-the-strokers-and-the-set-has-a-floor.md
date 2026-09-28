# 1359 — A folding stroke's other stretches are the stroker's, one chain is one strand, and the exact set has a measured floor

Status: accepted. Session 1261. Amends ADR 1348 section 1 (what is pieces) and ADR 1347 sections
2 and 4 (which clusters are walked; what it costs).
Context: ISO 32000-2 §8.4.3.2 to §8.4.3.5, §10.7.4, §10.7.1; CLAUDE.md principle 2 and "on the
tension between 2 and 4"; ADRs 1082, 1341, 1347, 1348; traps 50, 54, 56; habits 45, 47.
Code: `crates/render-cpu/src/lib.rs` (`folding_stroke_outline`, `non_folding_runs`,
`folding_segments`, `push_wound_positively`, `Pieces::bridged`), `crates/render-cpu/src/area.rs`
(`RowScratch::needs_walk`, `Edge::by_row`).
Tests: `crates/render-cpu/tests/stroke_wider_than_its_curve.rs` (two new closed forms),
`lib.rs`'s `a_stretch_is_wound_the_way_the_pieces_are`, `area.rs`'s whole-row reference unchanged.

## 1. What changed, each measured alone

`callgrind_rasterise`, `RAYON_NUM_THREADS=1`, five draws, per function, each arm its own export and
target directory, binaries `md5sum`-distinct (trap 50):

| change | `issue14415` | `issue19802` | `issue20232` | p. 101 |
|---|---|---|---|---|
| one chain is one strand (`needs_walk`) | −0.09% | −1.86% | −0.19% | 0.00% |
| each edge's row found once (`by_row`) | −0.46% | −1.34% | −0.70% | 0.00% |
| the non-folding stretches to the stroker | −8.74% | 0.00% | +0.01% | 0.00% |
| all three | −9.16% | −3.12% | −0.86% | +0.00% |

**One chain is one strand**: pieces of one chain meet a row in one stretch, which takes two adjacent
windings; the walk found that only after sorting and cutting (6149 of 11 145 walked clusters per draw
on `issue19802`). **The stretches**: `issue14415.pdf`'s strokes fold at 348 of their 1827 curves; the
rest are now the stroker's outline, butt-ended, flattened to the chords' tolerance, wound as the
pieces are, with every cap and every join where a stretch meets a folding curve a piece turned from
the stretch's exact tangent and reaching `overlap` into it. Built first with chord-turned joins, it
left a seam of 45 levels inside `inks.pdf`'s strokes (trap 56's shape: a wedge between the stretch's
butt end and a join square to its last chord); the exact tangent closed it. At the stroker's default
resolution a ring of radius 23 read four units heavy, so the stretches are stroked at sixteen times.
Fixtures against `numpy`'s distance set: a folding arch between two straight stretches 2025.99
against 2024.62 (the all-pieces construction reads 2025.86 there); a stroker's ring around a folding
disk, either winding, 1661.90 within two units. Tried and rejected: tracing a mark once for both
constructions — map_point went out of line on every mark, page 101 +2.1% (habit 47).

## 2. The target, and the floor

| page | before ADR 1341 | ADR 1347 | now | against before |
|---|---|---|---|---|
| `issue14415.pdf` | 1 472 472 463 | 1 926 617 160 | 1 750 057 872 | +18.85% |
| `issue19802.pdf` | 1 106 004 036 | 1 307 007 247 | 1 266 261 918 | +14.49% |
| `issue20232.pdf` | 366 226 972 | 423 137 176 | 419 508 802 | +14.55% |
| ISO 32000-2 p. 101 | 1 420 241 605 | 1 384 323 725 | 1 384 392 452 | −2.52% |

Control `issue12295.pdf` −0.17%. **The target — within a few per cent — is not met, and the floor is
measured.** With the walk switched off the same tree draws `issue19802` 19.3% and `issue20232`
23.8% *cheaper* than before ADR 1341 (the exact integral is cheaper than the supersampler); the walk
is 373 M and 140 M. On `issue20232` 99.4% of the clusters walked part from the set, so the walk is
all necessary work and +14.5% is its floor under this construction. On `issue19802` 56% do; even an
oracle skipping the rest in advance saves at most their share of `walk` (≤ 45 M), leaving ≥ +10.4%
— the edge ordering and row clustering the walk needs to find them are the remainder. Going lower
needs a construction that knows where a mark overlaps without clustering its rows, which trap 54
says no row-level sign provides.

## 3. The sixteenth stays

Folds measured exactly by the area walk (a prototype of the stretches on the stroker, the folds'
pieces through `crate::area`) cost `issue14415.pdf` 735.8 M per draw against 333.7 M with the
supersampler — +120%, the pieces around a cusp overlapping in every row; every piece walked exactly,
as ADR 1348 found, is 2168 M against 375.5 M. ADR 1348's decision stands, now for the folding curves only.

## 4. Pixels

`raster_golden`: 24 first pages moved, every one `raster only`, all by the stretches (the two walk
changes move none, checked page by page); boundary pixels by one or two sixteenths, largest
`issue20513.pdf` (362 pixels, −2.23 ink, and a seam of ADR 1348's own inside it closed).
`render-raster --test corpus` 946 agree / 5 differ before and after; the oracle's verdicts unchanged.
