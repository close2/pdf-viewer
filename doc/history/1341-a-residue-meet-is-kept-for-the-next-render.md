# 1341 — A residue meet is kept for the next render

Raster, from the clauses alone. ADR 1517. No question. `crates/render-cpu/` was not opened (A76).

**Measured first.** Callgrind, by function name, on one `zoom_frame` pair of
`bug1721218_reduced.pdf` at one thread, exported HEAD. ADR 1503's division holds after ADR 1513:
`meet_residue` 3.53 G of `encode_scene`'s 4.17 G, of which the exact meet 2.05 G, residue tiles
1.10 G and the chains' row index 0.37 G. The 59% of repeated asks is not inside a frame. Each
zoom frame draws the four-component group as two renders, chromatic and black (ADR 1471), and the
black render asks all 6 769 of the chromatic render's meets again, under new outline ids. Inside
one render 14% of asks repeat. The 14 060 fills are 14 052 small first uses at 22 k instructions
each (0.31 G) and eight large fills (0.48 G). In those, 0.29 G was a topology sort that then ran
past its budget.

**Kept: a meet for the next render (ADR 1517 section 2).** The device keeps met tiles for one
more render. They are keyed word for word by the mark's tile, rule and polylines and a number for
the chain's content. The chain is compared by segments, not ids. A meet the edge budget could have
decided is not kept. Keys over 1 024 words are not built, which keeps the Type 3 step inside its
band. Every black-render meet hits: `encode_scene` 4.17 → 2.80 G.

**Kept: a sweep counted before its sort (section 3).** Exact: pairs less an upper bound on the
pairs apart, bucketed. `Topology::of` 0.29 → 0.09 G; `encode_scene` 2.80 → 2.61 G.

**Not built, each measured.** A clip's side of a pixel kept for one render reaches 14% of asks.
Small fills by the meet's band sum are not byte-identical by construction (habit 62).

**Fixtures.** `tests/residue_meet_kept_for_the_next_render.rs` (three arms; watched failing with
chains named by id) and `topology/count.rs` (watched failing with a loose bound).

**Figure.** The 1.25× step on the GPU lane goes from 193.1 to 147.6 ms (pinned minima of 3 × 5,
interleaved exports, load 2.3–2.5). The CPU backend's figure is 84 ms, so the step is within 2×.
For 1342's `doc/performance.md`: add a `bug1721218_reduced.pdf` step row, 147.6 ms on the GPU lane.

**Gates.** Exit statuses: `rustfmt --check` on my files 0. Clippy `-D warnings --all-targets` on
`raster-gpu` 0. On `render-raster` it was 101, from a sibling's `pdf-model/src/content/text.rs`
doc lint (it was 0 before that edit landed). `cargo test -p raster-gpu` 0 (679 passed),
`-p render-raster` 0, `-p conformance` 0. Behind the lock: the corpus at 1× and 4× on three lanes,
0 of 967 / 962 digests moved, one-versus-many 0. `turn_path` 0 (20 of 20 inside; 101 before the
key cap). `headless_gpu` 0 (39 passed). `raster_golden` 0, unchanged. `launch_path` 0 (26
banded, 0 outside).
