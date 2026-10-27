# 1792 — A zoom step is coverage at the new scale, and a column's rebuild is a millisecond

Session 1792, slot 5 of batch 119, a "pixels" round on the owner's question of 2026-10-26; ADR 2418; ADR 2419 unused;
Q658 unused. No ledger row moved; no pixel moved; the one code change is an instrument.

**Question.** Why a page cannot sit in GPU memory with only the view-point changed, and what blocks it. Answer
(ADR 2418): for one page it does; what a step still computes is coverage at the new scale (count, emit, deposit,
content), which only the blurry stand-in's texture escapes. Removable: the second flatten inside the kernels, the
heavy page's walk and processor-rasterised sheet, and a column's rebuild.

**Measured.** `zoom_frame` on the Radeon 890M, `Coverage::Compute`, 800 × 1600, twelve steps, minima of three rounds,
two passes in one `--clock --nice 0` hold, every arm on `pp_dpm_sclk`'s middle line (642–1112 MHz), no other holder
of the render node; emit and deposit timed apart by a measurement patch the tree does not carry. Then quorra (HEAD,
release) in a nested headless KWin of this round's own at 2880 × 1800, scale 1.6, notches through EIS, in a second
hold: notch to sharp `issue19802` 2–22 ms (20 of 24 inside 8.33), the ISO column 6–29 ms (7 of 24), the plan
14–49 ms (0 of 24).

**Hypothesis.** "On one page the device kernels are most of a step and are the inherent term" held. "On a column the
rebuild is the removable one and lever 1 reaches 12 to 14 of 24" did not hold as stated: raster's own rebuild costs
a column at most 1.2 ms a step and every headless column step is inside 8.33 ms; the window's extra comes with 33–82
resource uploads a step against 2 headless, which points at the settled 2× pass alternating with the step's baked
scene. Lever 1 was priced and not built (ADR 2418 §4); the arms are unmoved because nothing that draws changed.

**Code.** `crates/render-raster/examples/zoom_frame.rs`: `ZOOM_FRAME_COLUMN`, `ZOOM_FRAME_COLUMN_ONLY`,
`ZOOM_FRAME_GAP` — a column drawn as `viewer_core::layout` lays one, and one page of it alone at its place.
`doc/todo/75`: two pointers to ADR 2418.

**Instruments.** The session worked in `scratchpad/r1792/`, with two exports of its own and a nested KWin of its own,
stopped with TERM after each arm; no Xvfb was started. The first window arm of `issue19802` ran beside another
process of this account on the render node and at a load of 30, so it is read as a range only.

**Gates.** `rustfmt --check` on the example: exit 0. `cargo fmt --check -p render-raster`: exit 0. `RUSTFLAGS="-D
warnings" cargo clippy -p render-raster --all-targets`: exit 0. `cargo nextest run -p render-raster`: exit 0, 112
passed, 3 skipped. The `zoom_frame` hold: exit 0 (225 s, ten arms each exit 0); the window hold: exit 0 (489 s,
six arms each exit 0). `tools/state.sh --round 1792 docs`: exit 101, its reds a private link in `conformance` and the
three viewer crates, none in `render-raster` (siblings' open edits). `raster_golden`, the six arms and `turn_path` are owed only by a build of lever 1, which was
not made. `cargo test -p conformance --no-fail-fast`: exit 0, 69 targets, 469 passed (its first run was red on `records` for this
record's own unfinished gates sentence).
