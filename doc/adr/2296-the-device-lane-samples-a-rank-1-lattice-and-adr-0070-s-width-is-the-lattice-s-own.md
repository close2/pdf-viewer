# 2296 — The device lane samples a rank-1 lattice, and ADR 0070's width is the lattice's own

Status: implemented. Session 1731, 2026-10-23, the pixels slot of batch one hundred and twelve,
under the project owner's switch to code improvements and bug fixes of 2026-10-14. Clause: ISO
32000-2 §10.7.4 (the row stays `departed`, its note amended), read with §10.7.1's NOTE that the
scan-conversion algorithm is not defined by PDF. Context: raster's ADRs 0006, 0016, 0070, 0076;
ADR 2286. Code: `raster/crates/raster-gpu/src/winding/lattice.rs` (new), `winding.rs`'s
`sample_offsets`, `encode/thin.rs`'s `vanishing_width`. Amends raster's ADR 0070 (its threshold)
and ADR 0076 (its pitch).

## 1. The premise, checked

`winding::sample_offsets` laid sixteen samples on an ordered 4×4 grid, so the samples stood in four
columns of four: an axis-aligned edge crosses them four at a time and reads one of five levels.
ADR 0076 recorded the consequence as a non-conformance and priced more samples, never another
pattern. The standard does not choose one: §10.7.1's NOTE says the specifics of scan conversion
"are not defined as part of PDF", so the pattern is ours to choose, and what binds it is §10.7.4's
rule that no shape disappears.

## 2. The pattern

Sample `i` of `n` at `(i + ½)/n` across and `((g·i mod n) + ½)/n` down: a rank-1 lattice, the
only lattices with `n` distinct columns and `n` distinct rows. An axis-aligned edge now reads one
of `n + 1` levels, its error at most `1/(2n)` — a thirty-second of a pixel at sixteen, where the
ordered grid's was an eighth. The generator is derived: of the `g` coprime to `n`, the one with the
smallest vanishing width (§3), the smallest on a tie — 3 at sixteen samples (5, 11 and 13 are its
mirror images, so the brief's `5i mod 16` is the same lattice turned).

## 3. ADR 0070's argument, re-derived first

ADR 0070 keeps a mark on the processor lane when its thin axis — the narrower side of its device
box, or a stroke's own width — is below the width at which the lane could miss it. On the ordered
grid that width was one number, `1/√n`, because the grid is a product: a box that wide and tall
holds a column and a row, and the sample where they cross. **The hypothesis that the lattice keeps
that guarantee at `1/√n` does not hold.** A lattice is not a product, and two widths answer:

- **the widest empty band**, any angle, infinitely long: the spacing of the lattice lines along
  its shortest vector, `1/√m` for that vector's squared length `m` in units of `1/n` — `1/√10 =
  0.3162` at sixteen, along `(1, 3)`;
- **the widest empty axis-aligned square**: `5/16 = 0.3125` at sixteen.

The threshold is the larger, `SampleLattice::vanishing_width`, `0.3162` against the ordered grid's
`0.25`. No pattern of sixteen distinct columns and rows can bring the square back to a quarter:
squares of side `1/√n` would tile the plane one to a sample, and a lattice of squares tiles only
with a vector along a side. Both widths are re-derived by brute force in the module's tests, at
every count the device admits (4 to 64), and two defects planted in the closed forms turn them red.
What the lattice gives back is the axis-aligned band, which is most of what a page draws thin:
`n` distinct columns put a sample in every pixel along a band `1/n` wide.

## 4. The price on slanted edges, stated

The lattice answers a straight edge along its shortest vector worst: `0.187` of a pixel against the
ordered grid's `0.125` over every angle (the ordered grid's worst is its axis-aligned edge), and on
`tests/coverage_lanes.rs`'s triangle `0.154` against `0.053` on its steepest edge. That test now
derives its bound from the lattice per edge rather than stating an eighth.

## 5. Measured

Against HEAD's six arms, my export's, every page compared by its frame digest and mean error:

| arm | digests moved | mean error down | up | sum of the page means |
|---|---|---|---|---|
| `gpu` 4x | 329 of 965 | 255 | 52, the largest +0.0214 (`issue5501`) | 23.09 → 18.97 |
| `gpu` 1x | 110 of 968 | 78 | 24, the largest +0.0186 (`issue2177`) | 53.31 → 51.16 |
| `cpu`, `compute`, 1x and 4x | 0 | 0 | 0 | unchanged |

**The glyph pages at 4x fall**: of the forty pages where HEAD's `gpu` lane led its `cpu` lane by the
most, thirty-nine fall — `issue21068` 0.4903 → 0.0795, `endchar` 0.5775 → 0.2194, `issue16038`
0.4895 → 0.1455, `issue8187` 0.4140 → 0.1491 — and one rises, `issue5994` +0.0031. **The knockout
pages ADR 2286 moved fall where they rose**: `knockout_groups_test` 0.4500 → 0.2421 at 1x and
0.2003 → 0.1485 at 4x, `issue1905` 0.2700 → 0.1418, `alphatrans` 0.0776 → 0.0555 and 0.0420 →
0.0372; of their nineteen page-arms thirteen fall, two hold, and four rise — `issue20062` at
both scales (+0.0021, +0.0011), `issue17069` and `bug852992_reduced` at 1x (+0.0003, +0.0001). **The hypothesis that no page gets worse did not hold.**
`issue20062` is the reason drawn: a single green triangle whose edges run along `(1, 2)` and
`(2, 1)`, near the lattice's worst direction, and nothing axis-aligned on it at all — §4's price,
on a page that pays only that. Looked at (trap 1): `knockout_groups_test` and `issue20062` on the
device lane, right. `t2-turn_path` under one `--clock` hold: 33 figures banded, 33 judged, 0
outside, children at load 2.4 to 2.7; the turn draws on the processor and compute lanes, which
this does not reach, and the encoder's per-frame width is a search over sixteen corners.

## 6. Alternatives

- **More samples**: ADR 0076's arithmetic stands — the overshooting rung does not shrink with `p`.
- **A searched 16-rooks table**, measured and not taken. A hill-climb over permutations of sixteen
  rows, minimising the worst straight-edge error with the empty square held at `5/16`, found rows
  `[3, 11, 15, 6, 2, 9, 14, 5, 10, 0, 13, 4, 7, 12, 1, 8]`: worst slanted edge `0.123` (the
  ordered grid's `0.125`, the lattice's `0.187`). On the `gpu` 4x arm it moves the same 329
  digests, 269 down and 30 up (largest +0.0073), sum 18.67. It is better on the corpus and it is
  not a derivation: a table found by search for one count, whose band width a lattice's closed form
  cannot give and whose other counts would each need a search of their own. Not taken this round
  for that reason and its time; it is the first thing to revisit.

## Revisit when

- a round can give the searched table what the lattice has here — its widths re-derived by brute
  force in the module's tests and a pattern for every admitted count — and its arms show it ahead
  (§6's figures say it is, at 4x);
- a caller sets `coverage_samples` in earnest away from sixteen: at four the only lattice with a row
  to each sample is the diagonal, whose vanishing width is `0.75`.
