# 1422 — A frame starts its threads at each drain, and the offscreen host pass is the medium

Pixels slot of batch sixty-six. ADRs 1686 and 1687; Q321 not used. No ledger row moved.

**Counted** (ADR 1686 section 1). `raster_gpu::threads::started` counts the threads raster starts at
its four spawn sites; `frame_budget` and `first_frame` print a `threads` column and `turn_path`'s child
line three fields. Per frame at 24 threads (eight pinned): the text page's turn and seventh 115 (35),
five drains past the floor; the vector page 0; the image page 23 (7), not the encode's; every step 0.
Page 7's first frame starts 46 (14), as ADR 1668 said, and frames 2 to 10 none.
**Built, measured, reverted** (sections 2 to 4). As an upper bound, a process-wide `rayon` pool took the
text turn's encode from 4.53–4.83 to 4.05–4.26 ms unpinned and nothing pinned. A pool of the frame's
own was built in three versions, started once at its first drain past the floor (23 threads, not 115).
The first lost on pages that drain once: `issue14415.pdf`'s step 5.38–6.42 against 4.60–5.51. The last
moved the text turn 0.1 to 0.4 ms at the median, outside HEAD's spread in two of six row-runs and not
at all pinned. Not built.
**Named** (ADR 1687). The 4× host pass is `QuorraRasterizer::rasterize` premultiplying, compositing
§11.4.7's medium (`Medium::PAGE_ONLY` marks) and demultiplying. `first_frame`'s new `host` column reads
17.0–17.8 ms on *every* 4× frame and 1.2 at 1×. A window owes none of it: its scene draws the medium
on the device, and only `render-raster`'s tests, examples and gates call `rasterize`.

**Premise.** Held: 46 spawns on page 7's first frame, and the window not paying the host pass. Not
held: that the host pass is paid on frames 1 and 2. It is 17 ms of every 4× frame; the further 7 ms on
frames 1 and 2 came in one run of five, and the 4× first frame's excess is mostly `device`.

**Gates.** `rustfmt --check` on the ten files: 0. `RUSTFLAGS="-D warnings" cargo clippy -p raster-gpu
-p render-raster --all-targets`: 0. `cargo nextest run -p raster-gpu -p render-raster`: 0, 805
passed, 5 skipped. `cargo test -p conformance`: 0, 405 passed. Tier 2 behind the lock, from an export
of HEAD plus this diff alone: the six corpus arms all exit 0. Against `/home/AI/arms-1420/` 0 pages
differ by digest on each of the six (checked by planting one), and 0 differ between one encode thread
and many. `raster_golden`: 0, held 974, moved 0. `turn_path` twice: 0 and 0, 33 of 33 judged, 0
outside. `RUSTDOCFLAGS="-D warnings" cargo doc -p raster-gpu --no-deps`: 101, on HEAD's
`startup.rs` (two links to the private `wake_render_nodes`), which this round did not touch.

**Unfinished.** The Type 3 page starts 46 threads where one set would do: its exact-meet helpers and
its fan-out are two sets (ADR 1686 section 5). The fused medium pass that ADR 1687 section 4 names is
owed only when a gate's wall clock is the question. `launch-path.toml` and `turn-path.toml` were not
re-taken, and no banded figure moved.
