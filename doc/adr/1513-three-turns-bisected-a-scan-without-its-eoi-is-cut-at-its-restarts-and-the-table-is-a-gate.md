# 1513 — Three turns bisected: a scan without its `EOI` is cut at its restarts, a mark's edges are built by its job, and the table is a gate

Status: accepted. Session 1339. Amends ADR 1495 section 2 in one respect: the restart plan no longer
declines a scan the data ends without `EOI`; the entropy pass's cut still does. Amends ADR 1467 in
one respect: a fan-out job builds its mark's row-bucketed edges, where the commit built them.
Keeps ADR 1443's two chords and ADR 1467's exact meet as costs paid for §10.7.4. Supersedes nothing.
Context: `CLAUDE.md` principle 2 (perf gates in CI, a regression fails the build) and the
rule on optimisations; ISO 32000-2 §7.4.8, §10.7.4; ISO/IEC 10918-1 (ITU-T T.81, cited by section,
paraphrased) sections B.2.1, E.2.3, E.2.4; ADRs 1433, 1443, 1445, 1455, 1467, 1491, 1495, 1503,
1505, 1507; traps 13, 50, 94; `doc/habits/measuring.md` 52, 56, 59.
Code: `crates/pdf-model/src/image/restart.rs` (`decode_at`, the module comment),
`image/cut.rs` (the decline's reason); `raster/crates/raster-gpu/src/raster/meet.rs`
(`RowEdges::of_charged`, `RowEdges::charge`), `encode/meet.rs` (`Mark::edges`, `JOB_EDGE_WORK`,
`JOB_EDGE_REACH`), `encode/parallel.rs` (`Made::edges`, `rasterise`), `encode/parallel/commit.rs`,
`encode/coverage.rs`; `crates/render-raster/tests/turn_path.rs`, `tests/support/frame_cost.rs`
(the method, which `examples/frame_budget.rs` now includes), `doc/checks/turn-path.toml`,
`tools/batch.sh` (`t2-turn_path`).
Tests: `banded_decodes.rs`'s `a_scan_of_restart_intervals_without_its_eoi_is_cut_and_is_the_whole_frame`
and `a_scan_the_data_ends_inside_its_last_interval_is_never_cut_differently`; `meet.rs`'s
`a_wider_build_meets_and_charges_a_band_as_the_band_alone` (watched failing on a charge one too
high, trap 13).

**`crates/render-cpu/` was not opened** (`doc/questions/A76`).

## 1. The bisection, and the brief's premise

The brief put all three doublings in batch fifty-one's squashed commit. They are not there. Pinned,
min of five rounds, three runs interleaved commit by commit (each export its own target directory,
binaries `md5sum`-distinct, trap 50): the Type 3 page reads 11.5 ms at `98531c20`, 9.1 at `29df1ce9`
and `c9c422cf`, 12.8 at `8566f66e`, 12.9 at `18b884f8` and 12.4 at `9f544eb4`; `issue14415.pdf` 11.6
at `98531c20` and 15.5–16.3 from `29df1ce9` on. Batch fifty-one moved neither. By hunk, on exports:

| page | owner | measured |
|---|---|---|
| photograph | ADR 1495's `EOI` decline in `restart::decode_at` | turn 69.4 → 32.5 with it taken out |
| `issue14415.pdf` | ADR 1443's two chords a flat piece (`flatten_cubic`'s `out.push(q1)`) | 16.1 → 11.6 with that line out |
| Type 3 | the same line, and ADR 1467's mark edges built on the walk's thread | 12.7 → 8.1 with the line out; callgrind puts the commit's `meet_residue` at 17 M → 109 M instructions between `c9c422cf` and `8566f66e`, 87.6 M of it the mark's `RowEdges::of` (198 calls) |

ADR 1445's 5.98 was taken on its round's tree, before ADR 1443's chords joined it in the merge.
The `Real` generic fill of ADR 1491 cost the `f32` path nothing: by function name the fill's own
work is 225 M instructions at `18b884f8` and 181 M at `9f544eb4` (the closure now inlines), and the
turns did not move.

## 2. A scan whose data holds every MCU is complete, `EOI` or not

Section B.2.1 ends compressed image data with `EOI`; section E.2.3 ends the decoding of a scan when
its expected restart intervals are decoded; section E.2.4 decodes an interval MCU by MCU and then
looks for the next marker, leaving what to do on failing to find one to the decoder, and its note
says the last interval holds only the MCUs that remain. So a missing `EOI` is a missing marker after
a complete scan. The restart plan's last band is handed its intervals as the data carries them,
decoded strict, from a reader reset at a restart marker on the bytes the whole decoder reads from
that marker: the two read the tail alike, and where the data runs out before a row the whole decoder
fills the rest and the strict band is refused. The decline was argued from the cut plan's
counterexample, where the last band is re-coded and ended by `EOI`, and stays there. Byte identity:
the photograph's bands equal its whole decode; `hv_truncated.jpg` cut at 8, 16 and 64 lines equals
its whole frame, and truncated at every byte of its last interval is refused or identical; ADR 1495's
fixture and seeds pass; `jpeg_bands` ran 561 135 executions in 600 s from 216 pdf.js seeds, the band
fixtures and 24 no-`EOI` variants (2 461 edges), clean.

## 3. The mark's edges are built where the mark is

The meet reads a mark's edges in the rows where both sets cut a pixel, which only the commit knows,
so the job builds them over its whole tile (`RowEdges::of_charged`): a row's bucket holds every edge
part reaching it, gathered into runs of consecutive edges, and an edge between two consecutive edges
of a narrower build reaches the row too, so the buckets of the asked rows are the same; the bound a
narrower build would ask is kept row by row and asked at the meet (`charge`), past which the meet
keeps `min` exactly as before. A job builds them only above `JOB_EDGE_WORK` point-rows: all 19 536
residue jobs of `bug1721218_reduced.pdf` fell below a thousand and building them cost its turn 4 ms,
while all 189 of the Type 3 page were above. Interleaved: Type 3 turn 12.03 → 10.14, step unchanged
(the moved-view lane meets on the walk); `bug1721218_reduced.pdf` and `bug1743245.pdf` inside spread.

## 4. The gate

`turn_path` holds the table's twenty `turn` and `step` figures to bands, each the minimum of three
pinned children of five rounds with rayon given the pinned cores, judged under `release` where every
child's launch-gate calibration probe is in band and the load is below one per physical core; a
command count per row is a witness no clock moves. Bands by the launch gate's rule (`low` = min ×
0.85, `high` = max × 1.20) over eleven runs; first run 20 of 20 inside, exit 0, 20 s; with the
decline put back it failed on the photograph at 68.83 against 25.40 .. 39.80 (watched).

## 5. The launch band

`xfa_filled_imm1344e.pdf`'s `open_kinstructions` overrun was settled by round 1336 (ADR 1507
section 2): `pthread_getattr_np`, std's main-thread stack guard, parses `/proc/self/maps` before
`main`, one `sscanf` a mapping, and the test binary had one more mapping — the ceiling went
1820 → 1825 k with that reason beside it. This round did not move it again.
