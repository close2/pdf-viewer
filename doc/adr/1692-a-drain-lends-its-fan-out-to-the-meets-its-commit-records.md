# 1692 — A drain lends its fan-out to the meets its commit records, and the photograph's threads are its reduction's

Session 1428. Status: **accepted**, built. Answers ADR 1686 section 5's third reopening ("a page whose
exact-meet helpers and fan-out could share one set of threads"). Amends ADR 1541 in one respect: the
frame's own helpers are started only where a meet is given and no thread is claiming. Keeps ADR
0023's shape (every thread entered inside `Device::render` and left before it returns), ADR 1505's
claiming and ADR 1686's decision (no pool of the frame's own).
Context: ISO 32000-2 §10.7.4; `CLAUDE.md` principle 2 and the rule on optimisations; ADRs 0023,
1467, 1505, 1541, 1668, 1686; traps 66, 101, 105.
Code: `raster/crates/raster-gpu/src/encode/parallel.rs` (`rasterise_then`, `Handed`, `HandBack`,
`Release`; `rasterise_all` is now the tests' wrapper), `encode/parallel/commit.rs` (`drain`,
`drain_last`), `encode/meet/helpers.rs` (`Helpers::idle`, `start`, `lend`, `waiting`; `Lent`),
`encode/meet/deferred.rs` (`place_exact`, `meet_helpers`, `hand_on_waiting_meets`), `encode.rs` (the
two frame-final drains). Test: `raster-gpu/tests/encode_thread_sets.rs`.

Every figure is the 890M through RADV, `examples/frame_budget` built `--release` from two exports
of HEAD (`md5sum`-distinct), one with this change, each in its own target directory, five rounds a
run and the minimum of each, the arms interleaved run by run behind the lock (`--clock`).
"Pinned" is `turn_path`'s eight fast CPUs and eight threads; "unpinned" the 24 a viewer gets here.
Milliseconds; ranges are over runs.

## 1. Where the Type 3 page's two sets were

A timeline probe of one frame (24 threads): the walk queues the page's 152 stroked glyphs and drains
once, at the frame's end (weight 85 689); the fan-out's 23 threads make the masks and are joined;
then the commit meets each tile with the tiling's residue (§10.7.4, ADR 1467), and at its sixth meet
the recorded meets pass ADR 1541's 64-pixel floor and 23 helpers are started for 60 meets of 545
pixels. Every meet of the frame is recorded in that one commit, and the fan-out's threads had left
before it began. The zoom step is the same drain on the replay path.

## 2. The construction

The commit runs inside the fan-out's scope. A drain with any job under a residue clip lends its
threads to the frame's meets: each, once no job is left to claim, claims meets the commit gives
(`Lent::help`) until the drain lets it go. The masks reach the commit only when every thread has
handed its own back, so the commit reads what it read after the join. A guard lets the lent threads
go on every path out — an error, and a panic on the drain's thread — before the scope joins them.
The frame's own helpers stay for a meet given while no thread is claiming: a walk below the floor,
or the marks after a drain.

**How a lent thread leaves is the part that measured.** The first version kept every drain's lent
threads until nothing was given and made the rest on the drain's thread beside them. On
`bug1721218_reduced.pdf`, whose one drain of 3 024 jobs is followed by 11 ms of walk and 232 meets
more, that held the walk at the scope's join for a backlog ADR 1541's helpers had made beside it:
pinned turn 145.79–148.35 against 139.69–141.92, seventh 144.59–146.44 against 139.84–140.83. So
only the frame's last drain finishes its backlog that way (`drain_last`, where the settle is all
that follows); after any other the lent threads leave with the meet each has claimed and what is
left goes to the frame's own helpers, started for it (`hand_on_waiting_meets`). A count of claiming
threads outside the work's lock serves the question asked at every meet.

## 3. Figures

| page, row | threads before → after, pinned (unpinned) | budget before | budget after |
|---|---|---|---|
| Type 3, turn | 14 → 7 (46 → 23) | 10.23–10.81 | 10.46–10.85 |
| Type 3, seventh | 14 → 7 (46 → 23) | 10.04–10.31 | 9.98–10.33 |
| Type 3, step | 14 → 7 (46 → 23) | 11.20–11.95 | 11.01–11.45 |
| `bug1721218_reduced.pdf`, turn | 37 → 37 (114 → 114) | 139.44–142.10 | 139.73–141.53 |
| `bug1721218_reduced.pdf`, step | 21 → 21 (69 → 69) | 82.57–83.60 | 81.91–83.08 |

Pinned, five runs, load 0.99–2.02. The Type 3 page's encode moves inside its spread on every row
(step 10.63–11.34 → 10.39–10.88); unpinned, three runs of the first version, its encode read
9.22–9.70 against 9.89–11.22 on the turn and 10.54–11.05 against 11.39–12.27 on the step.
`bug1721218_reduced.pdf` keeps both sets, its meets after the drain being the frame's own helpers'
work. `issue14415.pdf` and `bug1743245.pdf`, whose drains carry no residue, start 7 (23) as before,
inside their spreads. Judged by the device stages (ADR 1668): `transfer`, `elsewhere` and `execute`
moved on no page.

## 4. The photograph's 23 threads

They are `raster/reduce.rs`'s: `area_averaged_cells` divides a reduced window's rows across the
host's threads, called from `device/resident.rs` when the device realises the image, after the
encode has returned. On `issue12841_reduced.pdf` the turn and the seventh frame each start one such
scope, 48 bands of a window of 2 359 296 source samples, 23 threads (7 pinned); the step starts none,
its reduction being kept for the device's life. The frame draws three commands and no drain reaches
the fan-out's floor, so the reduction's is the frame's only set and there is no second set to fold
it into. Sharing it with an encode's fan-out on a page that has both would need the fan-out's threads
to outlive the encode into the device's realisation, which is ADR 1686's frame pool, declined there.

## 5. Exactness

Which thread made a mask or a meet is no part of its bytes, and the settle order is unchanged.
`raster-gpu`'s 706 tests pass, `tests/encode_threads.rs`'s byte equality across thread counts
among them. `encode_thread_sets.rs` draws 120 strokes under a curved clip at eight threads, holds
the frame to the one-thread frame's bytes and to 7 threads started; planted back (no drain lends),
it reads 14. The six corpus arms of the export with this change against HEAD's
(`/home/AI/arms-1426/`): 0 pages differ by digest on each of `cpu`, `gpu` and `compute` at 1× and 4×
(968, 968, 968, 964, 963, 964 pages; a planted digest is named), and the `cpu` 1× arm at one encode
thread against it at 24 differs on 0 of 968. `raster_golden` held 974, moved 0.
