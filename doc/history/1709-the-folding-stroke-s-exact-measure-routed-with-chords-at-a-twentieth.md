# 1709 — The folding stroke's exact measure routed, with chords at a twentieth

Session 1709, slot 6 of batch 109, a pixels round under `doc/questions/A559`; 2026-10-23. ADRs
2252, 2253; no `Q`. Round 1664's walk rebased onto `ee365e4f` (offsets only).

**Premise.** Held: at a twentieth, one thread +14.2, +10.1, +8.8 % on `inks`, `rotated_ink`,
`issue14415`; 24 threads, four runs, means +13.7, +2.3, +5.9 % (ADR 2252 §3). **Not held, thrice.**
(1) No strip finds a boundary twice: `Pairs::compare` is the same at 1 and 24 threads, since the
planner keeps a folding stroke's rows whole; the second lever's ceiling is zero, not built, and a
24-thread total carries rayon's idle loop (ADR 2253). (2) The §10.7.4 row stays `departed` for its
other departures; its note now states the flattened set's measure and drops the wait on Q559. (3)
Coarser chords move more than rims: drawing the moved pages (trap 1) found a mitre decided by a
chord (`annotation-fileattachment`, 160 levels at 4x), a cap turned by one, and the walk dropping a
stretch (255-level streaks on `pr12564`, ink 12 levels short on `issue12823`). Fixed: §8.4.3.5's
mitre and §8.4.3.3's cap from the path's tangents, taken as vectors; a boundary whose accumulated
rows do not sum to zero is declined to the library (13 marks at 1x, none at 4x). Each fix has a test
red on its fault (trap 13); the flattened-set ladder is red on the library, (238, 231) 208 against
202.5.

**Pixels.** `raster_golden` 26 of 974 moved, raster only, re-taken; every page is named with its
pixels and worst level in ADR 2252 §4 — rims, mitres and caps of folding strokes.

**Found, not fixed** (ADR 2252 §5): why the walk drops a stretch (dump: `scratchpad/r1709/probe/`);
a join the pieces lose at some chords and scales (`issue14438` (1594, 1093), `inks`), on HEAD too.
Proposed trap (pixels): *a resolution constant changes only after its pages are drawn at two scales.*

**Files.** New: `render-cpu/src/area/union.rs`, ADRs 2252–2253, this record. Edited: `render-cpu/src/
{area,scan,strips,stroke}.rs` (the last two comments), `src/stroke/folding.rs`, `tests/{folding_stroke_
under_a_clip,stroke_cuts,stroke_wider_than_its_curve}.rs`, `pdf-model/tests/raster_golden.tsv`, the
§10.7.4 row, ADRs 2122 and 2164 (status), `doc/state-of-play.md` (a clause).

**Gates.** Worktree: `rustfmt --check` per file 0; `cargo fmt --check -p render-cpu` 0; `-D warnings
clippy -p render-cpu --all-targets` 0; nextest 240 of 240; `tools/state.sh --round 1709 docs` 0;
`cargo test -p conformance` 0. Exports: `raster_golden` re-take 0, check held 974 moved 0, division
census 29 pages, 383 px and 3 levels at most, 1 named (passes); `fixed_documents` 5,
`hostile_geometry` 4, `--include-ignored`, 0; arms against `/home/AI/arms-1704/`: 0 of 5 799 digests
moved, mean error down on 26 of 26 at 1x in each lane and 24 of 24 at 4x but 20 on the device lane;
`t2-turn_path`, one `--clock` hold H U H U before the tangents-as-vectors edit (same pixels), 4 runs
exit 0, 33 of 33 inside. Duration: 10 517 s, about 180 tool uses.
