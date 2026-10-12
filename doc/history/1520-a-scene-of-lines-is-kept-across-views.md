# 1520 — A scene of lines is kept across views, and a run of lines is one job

Slot 6 of batch eighty-two, 2026-10-12, a pixels round of the owner's performance target; the
partial-row word yields to it (brief §3). ADRs 1877, 1878; no question. No ledger row moves.

**Premise.** Held: every device step on the witness rebuilt its scene (1 625 ms, scene 713, encode
680). **Hypothesis.** Half held: what read the view was `thin_segment`'s 2 × 2 block and 28 684 round
dots, whose §10.7.4 road is each view's; then the encode, 1 334 instructions a line, a job per line.

**Built.** A one-segment stroke is a raster line at every view, width and §10.7.5 flag unresolved; a
round dot is a line where `thin_segment` admits it or it is a pixel across, and is kept beside the
scene and asked again at each view (`DotAsLine`); `FrameCost::scene_kept`. In raster a run of lines
behind a queued mark is one job, and an instance one 60-byte write. Round 1519's `segment_ends` hunk
taken into `thin_line` (the tree has it; the export used HEAD's slice pattern, which lacks it).

**Figures** (witness, one clock hold, three alternating runs, load 4.5–7.0). `zoom_frame` `Cpu`, a
1.25× step 969, 978, 996 → 385, 395, 439 ms; every step 912–1 051 → 334–467 ms; `Compute` 970–1 451 →
377–535 ms; replay 179 ms. Encode 1 334 → 734 instructions a command (callgrind). Owner's display,
`quorra`, ten notches: last notch to its frame 1.020, 0.425 → 0.191, 0.192 s, notches 8–10 each on the
device in 189–209 ms with the scene kept. No Xvfb figure: `llvmpipe` refuses the witness (ADR 1860).

**Bytes.** Kept scene = fresh build at 1.25×, 2×, 4× and back (digests). Device vs oracle at the fit,
max 15 levels, mean 0.1881 → 0.1911. Six arms vs `/home/AI/arms-1515`: 51–70 pages moved an arm,
rises ≤ +0.0082, `bug1743245.pdf` `Gpu` 1× 2.61 → 0.11 — looked at, the grid's hairlines now the set.

**Unfinished.** ADR 1877 §5's resident instances, the view a uniform (needs `line.wgsl`, slot 5's
this batch); a run of lines encoded on the fan-out's threads (ADR 1878 §3).

**Files.** `crates/render-raster/src/{stroke,present,scene}.rs`, `tests/lines_across_views.rs` (new),
`examples/zoom_frame.rs` (`ZOOM_FRAME_DIGEST`); `raster/crates/raster-gpu/src/encode.rs`,
`encode/{instance,parallel}.rs`, `encode/parallel/commit.rs`; ADRs 1877, 1878, `doc/todo/69`,
`doc/state-of-play.md`.

**Gates.** `rustfmt --check --edition 2024`, every file: exit 0. `RUSTFLAGS="-D warnings" cargo clippy -p
render-raster -p raster-gpu --all-targets`: exit 0. `cargo nextest run -p raster-gpu -p render-raster`
(queued 4 680 s, held 45 s): 822 passed, 5 skipped; the new tests fail with their defects planted.
`cargo test -p conformance`: 432 passed. Clock, queued 387 s, held 143 s, exit 0: `zoom_frame`, the
owner's display, and a `turn_path` that failed for the export's missing documents (trap 150), so again:
queued 1 200 s, held 164 s, exit 0, 33 banded, 21 judged, 0 outside. Six arms (`--tree 12`, queued
3 883 s, held 1 419 s): exit 0 each, 0 pages one-sided. Duration 11 536 s.
