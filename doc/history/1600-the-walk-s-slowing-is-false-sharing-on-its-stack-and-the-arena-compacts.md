# 1600 — The walk's slowing is false sharing on its own stack, and the arena compacts

Slot 2 of batch ninety-five, 2026-10-19, a "pixels" round: ADR 2022's unexplained walk and round
1597's uncharged arena holes. ADRs 2036, 2037; no Q file; no ledger row moves (no clause is cited).

**Premise, held**: ADR 2022 §3 and the patches; the arena's released words, charged to nothing.
**Hypothesis**: "one hunk's, memory layout, not work" — layout yes, a hunk no: one binary is fast or
slow as an unread environment variable moves its stack (period 64 bytes), so a bisection would name
whichever hunk last moved the frame. The holes on the witness: none in twenty notches, as a zoom
releases nothing; turns and eviction do.

**Found** (ADR 2036): no `perf`, so two programs on `perf_event_open` (`/home/AI/r1600-walk-false-
sharing/`) sampled the main thread per instruction: slow layouts put 3 400–12 600 of the walk's
samples on one load beside a store in its frame. The three line makers read the `line_at` closure's
captured slice header and the view out of that frame on every line. **Fix, in the export only**: a
128-byte-aligned `Ahead` the makers read, and the closures taking their slices by `move` — 32 of 32
positions and 12 of 12 randomised processes at HEAD's samples, where the build alone was slow in 10.
**Clock**: with it the fit's 1.25× step 31–45 ms faster, the window's steps HEAD's, the window's first
frame 48 ms slower (264 MB of lines) — not faster in the window, so the build stays out and item 2
waits for item 3; HEAD itself strikes nothing (0 of 20, 0 of 16), so the fix returns with the build.

**Built** (ADR 2037): `SegmentArena::forget` compacts where holes would outnumber live words — live
ranges copied down on the device, entries re-pointed, the buffer at most four times its live bytes
or 1 MiB; `ArenaBytes`, `Device::segment_arena_bytes`, `zoom_frame`'s `ZOOM_FRAME_ARENA`; the test
fails under both plants (entries left at old offsets; the bound read against all words used).

**Proof that nothing moved**: the arena's compaction runs only on a release; the arms by page against
HEAD's: gpu and compute at 1× (968 pages each) and 4× (965), 0 differing, 0 one-sided;
`zoom_frame` sends HEAD's bytes in all three settings.

**Files**: `raster/crates/raster-gpu/src/{compute.rs,device.rs,device/resident.rs,lib.rs}`, `tests/
arena_compaction.rs`; `crates/render-raster/src/lib.rs`, `examples/zoom_frame.rs`; ADRs 2036, 2037;
`doc/todo/75`; `doc/performance.md` 3d.

**Gates.** `rustfmt --check` 0 (seven files); `clippy -D warnings --all-targets` 0 (`raster-gpu`,
`render-raster`); nextest 837 passed, 5 skipped; `t2-raster_golden` 2 passed, exit 0; the four device
arms exit 0; one `--clock` hold, exit 0: the turn gate 33 banded, 33 judged, 0 outside, and
`zoom_frame` three builds × three settings × two runs; `cargo test -p conformance` 442 passed, 0 failed;
`tools/state.sh --round 1600 docs` exit 0. A first hold lost its `zoom_frame` lines to a script's
argument order and was taken again whole. **Unfinished**: item 2, above. 7 450 s.
