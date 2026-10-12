# 1502 — A zoomed frame is drawn from the marks that reach it, and its strips are its tiles

Slot 1 of batch eighty, 2026-10-12, a pixels round of the owner's performance target (the
partial-row word yields to it). ADR 1841; no ADR 1842, no question. No ledger row moved: §10.7.4
is why a mark outside a target may be left out, and no pixel of any target changes.

**Premise.** Held: the 4× window drew all 4 408 405 commands, each strip measured every bound
again, and the planner counted marks beside the window: 5 strips, not 8.
**Hypothesis.** Held above the fit, where nothing is culled. "A tile is a strip of a sub-rectangle"
held as "a strip of the window": a column cut converts an edge mark of partial coverage a level
apart, culled or not (`culled_window.rs`), so a window is cut into rows only.

**Built.** `pdf_render::MarkBounds`: one `device_bounds` per top-level command, measured on every
core per magnification, kept in the `DisplayList` (invisible to `==` and `Debug`), moved by a
scroll with a stated slack. `DisplayList::reaching` keeps the commands whose bound meets the target;
`CpuRasterizer::rasterize` draws from it and a strip reads the kept bounds. `render-raster` culls a
baked window scene, and a page-space one only where the replaced scene read the view; offscreen it
does not (one gpu arm page moved). `tile_price` gained section 5 and a device zoom step.

**Figures** (one clock hold, eight fast cores, four threads, load 4.2–4.9; HEAD → this, two
rounds): the window at 2× 756.3, 778.1 → **512.3, 432.6 ms**; 4× 727.3, 759.6 → **255.7, 204.1 ms**;
8× 992.5, 928.6 → **338.2, 250.5 ms**; fit 961.9, 1 026.4 → 843.2, 771.1 ms, the same digest. Device,
the 4× zoom step: 935.0, 895.5 → **338.5, 307.1 ms**, 0 pixels differ. **Gesture**, last of ten
notches to the real frame, `quorra`: owner's display 0.983, 0.824 → **0.207, 0.169 s**; Xvfb 1.420,
1.673 → **0.173, 0.240 s**; real frames during the gesture 0 → 2–5; first frame unmoved.
**Unfinished.** The cull is linear, 13–113 ms a frame (a bin index, a list naming commands by
index); `command_extents` (slot 4's) does not read the kept bounds; the toolkit windows ask for whole
pages, so the cull misses them; `doc/todo/69` §3 is slot 3's to mark built.
**Files.** `crates/pdf-render/src/{reach,display_list,lib}.rs`, `crates/render-cpu/src/{lib,glimpse}.rs`,
`crates/render-cpu/tests/culled_window.rs`, `crates/render-raster/src/present.rs`, its
`examples/tile_price.rs`, ADR 1841, this record.
**Gates.** Built in a private export of HEAD plus these files (siblings' `render-cpu` hunks were
mid-edit). `rustfmt --check --edition 2024`: exit 0. `RUSTFLAGS="-D warnings" cargo clippy -p
pdf-render -p render-cpu -p render-raster --all-targets`: exit 0. `cargo nextest run` of the three:
570 passed, 3 skipped; `culled_window` fails with the margin planted at −½ (trap 13). `cargo test -p
conformance`: 431 passed. `raster_golden` (`--tree 6`, queued 2 019 s): held 974, moved 0; division
test 175 move, at most 940 pixels, 3 levels, ADR 1837's. `turn` (`--clock`): 33 banded, 0 outside.
Six arms (`--tree 12`, queued 3 933 s, held 1 463 s): exit 0 each; every digest held but gpu 1× and
4× `bug946506.pdf` (offscreen cull then removed). Xvfb photograph at six notches: 0 pixels differ.
Duration 12 100 s.
