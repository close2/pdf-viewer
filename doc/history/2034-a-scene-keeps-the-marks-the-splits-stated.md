# 2034 — A scene keeps the marks the splits stated, and another view asks their answers again

Session 2034, a ledger round in the audit worktree. Rows 8.5.3.2 and 10.7.4; `doc/todo/76` row 8; ADR 2884.

**The slice.** ADR 0705's deferral: a stroke whose §8.5.3.2 splits made a dot or a zero-length dash's cap
consumed the view, so a zoom rebuilt the scene of every page holding one. A800 lifted the trigger.

**The premise.** Unbuilt and effort only (no `A*` file cites ADR 0705). The construction the brief named —
`point_mark` mirrored in raster's line shader — did not hold: the oracle takes a dot inside a path by
`thin_dots`, which declines all of a path's dots when two meet one pixel (ADR 1875), so its answer for one mark
depends on its neighbours, and a per-placement shader would be a second decision beside the shared one.

**Built.** `pdf_render::stated_point_mark` is public: the one §10.7.4 decision both splits call. A stroke that
made marks records `StatedMarks` (transform, stroke, width, `PointMark`, and ADR 1877's line question for a
lone round dot); `Retained::draws` keeps a view-free scene where every record gives the same width bits and the
same `PointMark` at the new view, which makes the kept scene that view's build. A mark restated as its pixel still
consumes the view. Zoom keeps marks a pixel across; a scroll keeps restated marks; zero width rebuilds.

**Measured.** `examples/dotted_zoom` (27 750 round marks, release, minimum of five): a step kept at 10.7–19.8 ms,
scene 0.01 ms; the same steps rebuilt at 31.7–42.1 ms, scene 7.5–11.9 ms — on lavapipe, because this VM had no
render node for the RX 580 (`/dev/dri` held `card0` alone; a run pinned to `radeon_icd.json` found no adapter).

**Tests.** `marks_across_views.rs` (three, byte for byte against a fresh build). **Plants**: a record that
always holds fails the second; a pixel-restated mark kept fails the third; marks that always consume fail the
first two.

**Gates.** Over `pdf-render` and `render-raster`: `cargo fmt --check` exit 0; clippy `-D warnings
--all-targets` exit 0; nextest 438 of 438 passed (6 skipped), on lavapipe, 227 bring-ups `type=Cpu` (no RX 580
render node); rustdoc `-D warnings --document-private-items` exit 0. `cd fuzz && cargo check --bins` exit 0.
Conformance after `git add -N`: exit 101 on this record's gates paragraph only, then `--test records` exit 0.
Tier 2: `corpus` 101 as HEAD (joined issue15053, mixedfonts; 61 incomplete); `raster_golden` 101 as HEAD (the 23
first pages, each "an interpreter change"); `render-raster --test corpus` 101 on lavapipe, `issue6931_reduced.pdf`
outside the held list (mean 0.052) — it states no `d` and no `J`, so no mark reaches the new code, and nothing
this round changes draws a first frame differently; `turn_path` stopped by me after 27 children OUTSIDE bands
taken on the RX 580, run on lavapipe at load 18–163. `state.sh --round 2034 docs` exit 0.

**Left of the rows.** 8.5.3.2 stays `implemented`. 10.7.4 stays `departed` on its three sentences and one
shape; this round changes none of them. A step across §10.7.4's boundary, a zero width and an adjusted width
under half a pixel still rebuild a scene, because the shared answer differs there.
