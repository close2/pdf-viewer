# 1572 — A window is a selection of its page's commands, not a copy of them

Slot 2 of batch ninety-one, 2026-10-19, a pixels round under the owner's switch to code improvement.
ADR 1980; ADR 1981 not used; no question; no ledger row moves (the change cites no clause).

**Premise, held in its mechanism and not its address**: the 31 `&DisplayList` readers the grep
counts are all `render-cpu`'s; raster's crates read a `raster_scene`, and the device's encoder that
reads a display list is `render-raster`'s (`scene::Encoder::commands`, from `present.rs` and
`lib.rs`). Six readers take the page's top level; the rest read the tables or a nested list. The copy
also cloned the clip table and its index per window, which ADR 1941 had not priced. **Hypothesis,
held and exceeded**: no digest moves; the 83 % window lands 118–270 ms sooner on four threads and
100–130 on twenty-four, against the 60–70 the copy and the drop were.

**Built.** `DisplayList::reaching` answers a `Window` of 32-bit indices; readers take a `Selection`
(`Selection::drawn`, `Window::copied` for instruments); the page's `MarkBounds` are asked by index,
so `MarkBounds::kept` is gone; a window's planner runs state their extents; the encoder takes an
iterator. **The crossing moved** to 70–83 % of the page on both pools (from 49–63 %); the half is
left, because a bound in commands kept is `viewer_host::oversize`'s (ADR 1980 §4).

**Proof that nothing moved.** The six arms of the measured export against HEAD's arms:
968, 968, 968, 965, 965, 965 pages, 0 digests and 0 mean errors differ. Every whole and window drawing
of `window_against_whole` has HEAD's digest at eight magnifications on 4 and 24 threads in two holds
(32 of 32). `raster_golden`: held 974, moved 0; the division test 176 pages, ≤ 1 308 px, ≤ 3 levels,
ADR 1758's figures. Plants: a window's bounds asked by position rather than index failed two
`culled_window` tests, and the planner's failed both `strips` tests. The tree differs from the export
in comments and tests only.

**Files.** `pdf-render/src/{selection,reach,group_cost,strips,lib,display_list}.rs`,
`render-cpu/src/{lib,strips,soft_mask,glimpse,reduced_images}.rs`, `render-cpu/tests/culled_window.rs`,
`render-raster/src/{lib,present,scene}.rs`, `render-raster/examples/{tile_price,offscreen_cull}.rs`,
`pdf-model/examples/strip_count_price.rs`, ADR 1980, `doc/{performance,state-of-play}.md`,
`doc/todo/30`, this record. **Unfinished**: the bound restated in commands kept (`viewer-host/src`,
Slot 6's this batch); why the whole page draws 20–150 ms sooner on four threads too.

**Gates.** `rustfmt --check` 0; `clippy -D warnings --all-targets -p pdf-render -p render-cpu -p
render-raster` 0; `nextest` 490 passed and 104 passed (3 skipped); `cargo test -p conformance` 436
passed, 1 failed (the records gate, on 1575's draft record alone); `state.sh docs` 0. Arms cpu/gpu/compute at 1×
and 4× 0 each; `raster_golden` 0. Clock: the first hold (load 6.7–12.5) ran the turn gate from a
copy, unjudged; the second (load 1.2–2.2) judged it, 33 of 33 inside for HEAD and twice for this,
every step within 7 % of HEAD's either way; the third (3.8–9.2) re-took the example. Duration 8 300 s.
