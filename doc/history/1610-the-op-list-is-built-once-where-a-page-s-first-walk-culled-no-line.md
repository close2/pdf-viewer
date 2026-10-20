# 1610 — The op list is built once where a page's first walk culled no line

Slot 4 of batch ninety-six, 2026-10-20, a "pixels" round: `doc/todo/75` item 3, with item 2 (round
1593's page-space line lane and round 1600's `Ahead`) brought back with it. ADR 2056; no Q file; no
ledger row moves (no clause is newly cited; §8.4.3.2 and §10.7.5 are read in the vertex stage).

**Premise, held**: ADRs 2008, 2022, 2036 — the step's list was the view's through the host's cull.
**Hypothesis, held in its mechanism, not its population**: the view-free part is the walk's lines,
4 398 932 in 1 055 runs, none of whose boxes is asked once the page's medium covers the target; but
"resident from the fit" needs a fit — item 2 alone made a window opened zoomed send the page's 264 MB
of lines (50–80 ms of device time, explored), so a walk with no encode before it still culls.

**Built** (ADR 2056): a walk that culled no line keeps its lines as runs (`encode/kept_lines.rs`),
counted by the line makers a block at a time; the next view walks every other command and takes each
run in one step, the stream shared and adopted on the device (`LineRuns` went). A run's box is asked
only until the root's region is the target. `Counters::lines_kept`; `zoom_frame`'s `lines kept`.
Planted back (boxes not asked, saturation inverted, an instance off by one), tests fail. A per-line
note on the walk's thread had cost the fit 15–20 ms (callgrind: 154–229 M of 3 499–3 574 M, one thread).

**Measured** (one `--clock` hold, load 2.1–2.4, ms total): the window past the fit 122.6–131.9 → 91.2–
93.4 at its first frame, 117.6–125.2 → 86.5–88.4 a step, no line byte; the page-sized 1.25× step
287.9–295.2 → 212.2–215.6; a window opened at 2.4 sends HEAD's bytes, within the machine's spread;
the fit within 3–11 ms. **Proof that nothing else moved**: the four device arms by page against
`/home/AI/arms-1607/` (968, 965 pages), 0 differing, 0 one-sided; kept steps' digests are the fresh
frames' at three views on the Radeon, and a fresh device's pixels on llvmpipe (tests); fit and step
looked at.

**Files**: `raster-gpu`'s `src/{encode,frame,retained}.rs`, `encode/{kept_lines (new),line,lines_ahead,
instance,encoded,clips,device_space}.rs`, `device/{render,staging,binds}.rs`, `compose/draw.rs`,
`pipeline/layouts.rs`, `shaders/line.wgsl`, `tests/{retained_replay,line_lane}.rs`; `render-raster`'s
`examples/zoom_frame.rs`; ADR 2056; `doc/todo/75`; `doc/performance.md` 3d.

**Gates.** `cargo fmt --check` 0 and `clippy -D warnings --all-targets` 0 (`raster-gpu`,
`render-raster`); nextest 739 passed, 2 skipped and 104 passed, 3 skipped; the four device arms exit
0; `t2-raster_golden` 2 passed, exit 0; one `--clock` hold exit 0 — the turn gate HEAD 33 of 33 inside,
the change 32 and `issue12841_reduced.pdf`'s step at its floor (2.50 ms against 2.50–4.00; two
commands, 0.01 ms of encode in both trees, its other children 3.28–3.36, the hold before 2.96), exit
101, and `zoom_frame` two builds × three sessions × two passes; `cargo test -p conformance` 426 passed, 1 failed (slot 2's `quorra/renderer.rs`, moved mid-batch);
`tools/state.sh --round 1610 docs` exit 0. **Unfinished**: a window opened zoomed keeps nothing; the
vertex stage draws the page's lines (0–4 ms); the record road keeps no runs (ADR 2056 §5). 11 800 s.
