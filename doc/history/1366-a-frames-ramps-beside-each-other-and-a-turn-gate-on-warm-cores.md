# 1366 — A frame's ramps are sampled beside each other, and the turn gate starts on warm cores

Date: 2026-10-06. ADRs: 1567, 1568, 1577. The RASTER slot of batch fifty-seven. No ledger row
moved; no question written.

**The 1× frame (ADR 1567).** Callgrind by function on `FrameSlot::render`: 1.603 G at one thread,
1.09 G on the frame's own thread at eight; then the clock. The 1× frame is the step plus a first
sight: 132 ramps sampled (5.4–6.4 ms, a third of it `roundf`) and made into textures (1.4–1.6),
and the outlines handed over (~4.5). Two probe arms ordered by the clock: every residue region
answered as a full tile took the frame 88–92 → 59–60 ms; every ramp left unsampled 83–87. The
regions are the larger lever — not ADR 1555's 4.5 ms — but its movable part is ADR 1555's 1.6 ms
and the rest each dot's exact fill, so it is recorded, not built. Kept: a ramp's table walked by a
forward cursor and rounded without a call, a frame's new ramps sampled on its encode threads.
1× frame 85.2–88.8 → **81.5–83.9 ms**, CPU backend 47.5–48.5: 1.79× → **1.72×**; step 1.38×,
unmoved. `doc/QUORRA_FEEDBACK.md` section 68.

**The first frame's unattributed span (ADR 1568).** It is `QuorraRasterizer::rasterize`'s pass
over the read-back pixels (premultiply, the medium, demultiply): 0.56–3.44 ms per child against
0.7–3.4 ms of span outside `scene` and `device` on the five rows. A window pays none of it; not a
lever in the encode or the presenter, so not taken. **Handed to round 1367:** `launch_path.rs`
could print `FrameCost::total − scene − device − settle` as its own stage beside the readback.

**The turn gate (ADR 1577).** Six interleaved gate runs on exported HEAD: cold, the mesh turn
11.17–11.69 ms (two runs OUTSIDE, one not judged), 20/20/18 of 22 judged, the probe declining 24 of
90 children; with 30 ms of spin on the pinned cores before each round, 9.34–9.78 ms, 22/22 judged
each run, 2 of 90 declined, heavy rows within 5%. Banding the light rows by the clock's spread
would put the mesh row's top at 14.1. So `frame_cost::round` warms the cores; no band moved.

**Tables.** `doc/performance.md` §3e: all 22 rows re-taken 2026-10-06 (warm, three runs of five,
pinned, load 1.8–2.2); `bug1721218_reduced.pdf` turn 161.26 → 159.23. `turn-path.toml`: header,
two observed lines, two `why`s; no band moved.

**Gates.** rustfmt --check on my five Rust files: exit 0. `RUSTFLAGS="-D warnings" cargo clippy -p
raster-gpu -p render-raster --all-targets`: exit 0. `cargo nextest run -p raster-gpu -p
render-raster`: 789 passed. `cargo test -p conformance`: exit 0, 382 passed. Corpus gate, CPU, GPU
and compute lanes at 1× and 4×, change against exported HEAD (2843 s behind the lock): 0 pages
moved on six arms, verdicts equal, one-versus-many 0. `raster_golden` on the change: held 974,
moved 0 (19 s). `turn_path` twice in a row on the worktree behind the lock: exit 0 both, 22 of 22
judged, 0 outside (83 s, 71 s). `ci.yml`'s 14 raster examples with `--check` under Xvfb behind the
lock: 14 passed, 0 failed (74 s).
