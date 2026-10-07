# 1385 — The encoder pool is measured and left, and the turn gate holds a long-lived device

Raster and perf slot of batch sixty. ADRs 1606 and 1607; no ledger row moved, no question.

**The premise, checked.** ADR 1594's "sixteen command buffers at once" is one allocation of
fourteen: `wgpu-core` gives a render pass two HAL command buffers, so `bug1721218_reduced.pdf`'s first
frame (221 passes) makes 17 allocations, 10.5 M instructions, and every launch-gate row's first frame
makes 2, 1.2 M (callgrind call counts, HEAD's `zoom_frame`). Eight frames on one device: 84
allocations there, six encoders of 14; four on `PDF20_AN001-BPC.pdf`.

**The pool, grown and measured (ADR 1606).** One binary with scratch knobs, removed since: six
encoders after the warm set, two before it, three on a thread of their own. Thirty interleaved
`first-page` children an arm, load 1.5–1.7: after the warm set the first frame's `device` read
7.10 → 7.89 ms at the minimum and 8.08 → 9.23 at the median on `bug1815476.pdf`; the other arms
inside the spread. Eight-frame sequences: no arm moved a frame. Depth enough for the heavy page cost
37 MB resident. Nothing is built under `raster/`; the arm is a patch in scratch, deleted.

**The seventh frame (ADR 1607).** `frame_cost::round` draws the turn again as the seventh frame of a
device that drew the warm-up document's pages 1–5 and 1; `turn_path` judges `seventh_ms`;
`frame_budget` prints it; section 3e has a `seventh` row per page, taken 2026-10-07 in one sitting
(three runs of five rounds, load 3.8–6.0). `tools/state.sh`'s `frame` pattern gained `seventh` (one
word in a file this round does not own).

**Warm open.** `PDFVIEWER_LAUNCH_WARM_CORES` makes `launch_path`'s open children spin 30 ms first.
`bug1815476.pdf`, ten runs an arm at load 1.54–1.68, nine pinned children each: warm cores 0.288–0.307
ms in ten of ten; cold 0.267–0.329 in five and 0.402–0.610 in five, calibration in band. Trap 110's
idle clock, so the band stays, with that reason in `doc/checks/launch-path.toml`. Threads of user
AI: 148 before the first heavy run.

**Gates.** `rustfmt --check` on my four Rust files: 0. Clippy `-D warnings`, `render-raster` and
`viewer-ui`, all targets: 0. `cargo nextest run -p render-raster -p viewer-ui`: 0, 248 passed.
`cargo test -p conformance`: 376 passed, 1 failed — `records.rs` on rounds 1383's and 1384's
records in flight, not this one. Behind the lock: `turn_path` three times — 101 (the Type 3 page
alone, all three rows at 13.0–14.5 ms, the older bands too; a re-take minutes later read 11.2–11.5),
then 0 and 0, 33 judged, 0 outside, in 108 s and 77 s; `launch_path` counted, with `pdf-sandbox
--bins` rebuilt in the lock: 0, 26 banded, 0 outside, in 205 s; with clocks and the flag: 0, 43
banded, 0 outside, in 48 s. No change under `raster/`, so the six corpus arms, `raster_golden` and
`raster-examples` are not owed.
