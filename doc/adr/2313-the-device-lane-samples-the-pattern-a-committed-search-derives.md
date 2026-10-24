# 2313 — The device lane samples the pattern a committed search derives

Status: implemented. Session 1739, 2026-10-24, the pixels slot of batch one hundred and thirteen,
under the project owner's switch to code improvements and bug fixes of 2026-10-14. Clause: ISO
32000-2 §10.7.4 (the row stays `departed`, its note amended), read with §10.7.1's NOTE that the
specifics of scan conversion "are not defined as part of PDF". Context: ADRs 2296 (whose
alternative this takes) and 2312 (the widths it is held to); raster's ADRs 0006, 0070, 0076.
Code: `raster/crates/raster-gpu/src/winding/pattern.rs`'s `SEARCHED` and `SamplePattern::of`,
`examples/sample_pattern.rs` (new), `winding.rs`'s `sample_offsets`.

## 1. The premise, checked

ADR 2296 section 6 measured a searched sixteen-row table and withheld it on two grounds: a table
found by search is not a derivation, and its band width had no closed form. ADR 2312 removes the
second. The first is answered by making the search the derivation: a program, a seed and a stated
objective, whose output a test and a CI step can re-derive.

## 2. The derivation

For each count the device admits (4, 9, 16, 25, 36, 49, 64), `examples/sample_pattern.rs` searches
the permutations of `n` rows for the pattern whose worst straight edge is nearest the area behind
it — `SamplePattern::straight_edge_error`, the largest gap over every direction and placement
between a pixel's area behind one straight edge and its samples' share, exact per direction and
compared at 512 directions — **under the constraint that neither of ADR 2312's widths is wider
than the count's rank-1 lattice's**, so the threshold of ADR 0070 cannot rise. The search is
simulated annealing over transpositions: eight chains from the lattice, each drawing from
SplitMix64 seeded by 1739, the count and the chain; 40 000 proposals a chain; a move that breaks
the constraint refused; a move no worse — by the worst, then by the mean over directions — taken,
a worse one at the Metropolis probability under a temperature falling linearly from three
hundredths of the lattice's worst to zero; the best pattern any chain met kept, the earlier chain
on a tie. `--check` re-runs it and fails on any count whose rows differ from the table (28 s on
this machine at load 6.6); CI runs it with the other examples.

It is the best this search finds, not a proven minimum: `16!` permutations are not enumerated.
The table is ours to choose under §10.7.1's NOTE, and the objective is the one ADR 2296 measured
the lattice's cost in.

## 3. What it derives

At 16 384 directions, against the lattice of the same count:

| samples | worst straight edge | widest empty band | widest empty square |
|---|---|---|---|
| 4 | 0.3086 (lattice 0.5000) | 0.3536 (0.7071) | 0.75 (0.75) |
| 9 | 0.1667 (0.2778) | 0.2357 (0.4472) | 4/9 (4/9) |
| **16** | **0.1146 (0.1875)** | 0.1118 (0.3162) | 5/16 (5/16) |
| 25 | 0.1160 (0.1400) | 0.1843 (0.2425) | 6/25 (6/25) |
| 36 | 0.1055 (0.1111) | 0.1852 (0.1961) | 7/36 (7/36) |
| 49 | 0.0884 (0.0918) | 0.1577 (0.1644) | 8/49 (8/49) |
| 64 | 0.0759 (0.0781) | 0.1370 (0.1414) | 9/64 (9/64) |

At sixteen the rows are `[5, 15, 10, 2, 7, 12, 1, 4, 11, 8, 14, 3, 6, 13, 9, 0]`. ADR 2296's table
measures 0.1264 by the same instrument (it quoted 0.123 from a coarser one), so this one is
nearer on the objective. Past sixteen the square binds and the search moves little; the mean over
directions there is slightly worse than the lattice's, which the objective does not weigh. The
vanishing width at sixteen falls from `1/√10` to `5/16`, because the band is no longer the wider
width: no mark the lattice drew on the device moves to the processor, and the marks between the
two widths move to the device.

## 4. Measured

Against HEAD's six arms, my export's, every page compared by its frame digest and mean error:

| arm | digests moved | mean error down | up | sum of the page means |
|---|---|---|---|---|
| `gpu` 4x | 315 of 965 | 166 | 23, the largest +0.0022 (`endchar`) | 18.97 → 18.72 |
| `gpu` 1x | 96 of 968 | 57 | 4, the largest +0.0012 (`issue16742`) | 51.16 → 51.12 |
| `cpu`, `compute`, 1x and 4x | 0 | 0 | 0 | unchanged |

**No page is worse by a level**: the largest rise is a five-hundredth of one. **The glyph pages
fall again**, not each of them: of the forty pages where HEAD's `gpu` lane at 4x leads its `cpu`
lane by the most, 29 fall, 8 hold and 3 rise — `endchar` +0.0022, `issue16038` +0.0003,
`issue16473` +0.0001 — and their sum falls 2.7786 → 2.6334. ADR 2296's worst case, `issue20062`'s
slanted triangle, falls at both scales (0.0018 → 0.0010, 0.0047 → 0.0028), and its largest rise,
`issue5501`, falls 0.0660 → 0.0535. Looked at (trap 1): `endchar` and `issue20062` at 4x on the
device lane beside the oracle, right. ADR 2296's table did better on this arm's sum (18.67); the
objective is the edge error, and this table is the one the program derives.

`t2-turn_path` under one `--clock` hold: 33 figures banded, 3 judged, 0 outside; the other thirty
not judged at load 11 to 35 from the machine's other rounds. The turn draws on the processor and
compute lanes, which this does not reach.

## 5. Alternatives

- **ADR 2296's table**: better on the arm's sum, worse on the objective, and found by a search
  nobody can re-run; taking it would ship the corpus's preference as a derivation.
- **The mean over directions as the objective**: it would trade the worst edge for the typical
  one; ADR 2296 priced the lattice by the worst, so the derivation keeps that measure.
- **A longer search**: the chains at sixteen landed between 0.1146 and 0.1218 in trials, so a
  longer one may find more; the table is re-derived by its program, and a better program is a
  change to the table with its arms.

## Revisit when

- a caller sets `coverage_samples` away from sixteen in earnest: those counts' patterns are derived
  and their widths held, but no arm has drawn with them;
- ADR 0070's condition gains a third width (ADR 2312 section 4), which would change the constraint.
