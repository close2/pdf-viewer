# 497 — The seven seconds between two lines

**Finding.** The owner's `tmp/Entwurf.pdf` (49.7 MB, one page, 58 009 display commands) took ten
seconds to first present and the launch table could not say where: its last two lines were
`document joined 505.704 ms` and `first present 10220.077 ms (+9714.373)`. The table now carries
`interpreted, N cmd` (the first `Event::NeedsRender`, command count in the step name) and
`first scene built` (relayed from quorra's own `FrameCost::scene`, because the boundary is inside
one `present` call) — and with them the ten seconds read ~7.0 s interpretation, ~1.0 s scene
translation, ~1.7 s device. Callgrind attributed the seven seconds: **lexing is 63.6% of the
whole interpretation** — one content stream inflating to 141.12 MiB, 20 834 587 tokens for
3 185 295 operators, with a heap-allocated `Vec<u8>` per token (`read_regular_run`'s `.to_vec()`)
putting the allocator at a fifth of the total and `str::parse::<f64>` at 15.1% — while resource
lookups are under 1% and the flate of the 141 MiB is 12.7%, paid once. The encode cache
(`doc/todo/45`'s row) was priced and not built: full reuse would take the trace's median 393 ms
frame to ~56–60 ms, the phase that pays is quorra's to reuse, and the two obstacles are upstream
API questions (scene-fragment composition; the transform baked per command).

**Date.** 2026-08-14.
**ADR.** [0332](../adr/0332-the-seven-seconds-between-two-lines-of-the-launch-table.md).
**Touched.** `crates/viewer-ui/src/bin/pdf-viewer/timing.rs` (`Launch::interpreted`,
`Launch::scene_built`, marks as owned strings), `crates/viewer-ui/src/bin/pdf-viewer/dispatch.rs`
(the mark at `Event::NeedsRender`), `crates/viewer-ui/src/bin/pdf-viewer/surface.rs` (the mark
beside `presenter.present`), `doc/todo/44-a-draft-that-takes-ten-seconds.md` (rewritten from
*evaluation owed* to *measured*), `doc/adr/0332-*` (new), this file.

## How the numbers were taken

- The verification run is structural only: the machine carried nine parallel rounds, so no wall
  clock from it is quoted. What it establishes is that the new lines print under `Xvfb` on the
  owner's document, partition the former gap completely, and `first scene built` −
  `interpreted` equals the first frame line's own `scene` figure to half a millisecond — the
  relayed boundary and the frame's accounting are one measurement.
- The attribution is instructions, not time: `valgrind --tool=callgrind` over
  `examples/callgrind_interpret /home/cl/projects/pdf-viewer/tmp/Entwurf.pdf 1` (22 411 M total,
  the open ~26 M), `callgrind_annotate --inclusive=yes`, with
  `examples/content_budget_census` supplying the token, operator and decoded-byte counts. Both
  instruments already existed; nothing throwaway was built.
- The pricing in `doc/todo/44` §3 is arithmetic on the owner's own trace (28 frames: `encode`
  sum 9 869 ms of frame sum 17 064; medians 233.8 of 393.1) plus `doc/QUORRA_FEEDBACK.md` §13's
  per-command fit, which this document confirms on a second adapter (58 009 × 3.86 µs ≈ 224 ms
  against the trace's 233.8 median).

## What the next round should know

- `render-quorra` was not touched; the encode-cache item is an upstream ask first, and
  quorra's `Options::instrument_encode` (its ADR 0023) is available and unused if the ask wants
  `encode` subdivided before it is made.
- The lexer candidate (borrow token bytes instead of `.to_vec()` per token) is priced at up to a
  fifth of interpretation and is a `pdf-syntax` API change; number parsing is second at up to
  15%. Neither was started.
- `doc/todo/02` §5 was not run: this is a worktree round and the release binaries a person runs
  are `main`'s; whoever merges owns that section, as §2 says of the gates.
