# 1373: a meet of convex sets is measured from the polygons, and a step's host costs are priced

Raster slot, batch fifty-eight. ADRs 1582 and 1583; no ledger row moved, no question.

**Counted first** (ADR 1582 section 1). The brief's premise did not hold: the page's 3 281 dot clips
are 3 280 distinct shapes (each written in page coordinates to 0.001 pt), each four cubics and no
circle, so a phase memo and a circle's closed form are exact for nothing there. The 3 515 regions
a frame are single-dot clips under full-tile shadings (no pixel cut by both); the exact meets are
3 256 others, a 33-point disc under one clip of 3 014 dots, 45 000 instructions a pixel.

**Kept.** Where the mark is one convex polygon and the clip is, in the tile, convex subpaths with
boxes apart, the helpers measure the meet from the intersection polygon (`raster::convex`,
`encode::meet::convex`); convexity under Shewchuk's bound or declined; the walk's work unchanged.
6 444 of 6 512 meets take it; callgrind 5.649 → 5.224 G. `zoom_frame`, exports, pinned, 4 × 5:
1× frame 81.3–83.3 → 79.5–80.8 ms, step 78.4–79.9 → 77.2–77.8, CPU backend 47.3–47.7 and
56.2–56.7: 1.68× and 1.37×. `frame_budget`: turn 157.86–160.31 → 154.59–155.23, `scene` −5 ms.

**Tried, not kept.** Convexity decided on the walk (7.7 ms a frame for 4.7 of edges); a convex
region's coverage as a polygon (1.20 against 1.50 µs a dot, not byte-identical by construction);
setting a pass's pipeline only when it changes (`finish` 1.43 / 1.46 ms either way, ADR 1583).

**Priced** (ADR 1583): a step's `finish` 2.9 ms, `submit` 0.8, poll 3.0, readback 1.5 over its three
renders — an instanced shading draw, and the black frame walked while the chromatic one draws.

**Docs.** `doc/performance.md` 3e's two rows re-taken; `doc/checks/turn-path.toml`'s bands by its
rule (turn 131.4–192.1, step 71.4–104.2); `doc/QUORRA_FEEDBACK.md` 69; `doc/todo/36`;
`doc/state-of-play.md`.

**Gates.** `rustfmt --check` on the nine source files: 0. Clippy `-D warnings` on `raster-gpu`, all
targets: 0. `cargo nextest run -p raster-gpu`: 0, 694 passed. `cargo test -p conformance`: 0.
Behind the lock, exports of HEAD and of the change in their own target directories (md5-distinct):
`render-raster --test corpus`, six arms each, all exit 0, digests by name 0 moved of 968 / 964 /
968 / 963 / 968 / 964, verdicts equal, 1-vs-N 0, in 2822 s; `raster_golden` on the change: 0, held
974, moved 0, in 21 s. On the worktree: `tools/batch.sh raster-examples` 0, 14 passed, in 150 s;
`turn_path` three runs, each 0, 22 judged, 0 outside, in 82, 82 and 71 s.
