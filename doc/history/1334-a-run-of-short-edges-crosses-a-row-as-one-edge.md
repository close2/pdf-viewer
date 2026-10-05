# 1334 — A run of short edges crosses a row as one edge

Raster, from the clauses alone. ADR 1503. No question. `crates/render-cpu/` was not opened (A76).

**Measured first (ADR 1503 section 1).** A probe tree counted the meet's steps per pixel on one
`zoom_frame` pair of `bug1721218_reduced.pdf`. There are 45 734 asked pixels, each with 24 bands,
20 edges through it and 226 partial edges beside it. Of `area_in_pixel`'s 3.29 G, partial
windings took 1.41 G and the row sort that pushes them 0.70 G. Band areas took 0.50 G, crossings
0.28 G and the cut sort 0.26 G. Allocation is 26 M.

**Kept: runs (section 2).** A row lists runs of consecutive same-direction edges whose parts abut.
Abutting half-open spans partition their union, so a ray meets one edge of the run, and a run
across the whole row joins the prefix sum. A pixel reads the runs that reach it edge by edge.
Runs are capped at one pixel wide; uncapped, `area_in_pixel` grew to 3.67 G. The bucket bound
still counts edge-row pairs. Result: `area_in_pixel` 3.25 G → 1.79 G, the pair 9.48 G → 7.89 G,
and the 1.25× step 250 → 200 ms in interleaved minima at load 3.5. The CPU backend takes 84 ms.

**Not kept, each measured.** (a) A meet-row holds 1.94 asked pixels, too few to share bands. (b)
Already applied by `both_cut`: 0 of 45 734 asked pixels have a set with no edge through them.
(d) Allocation is 1.3% of the meet. Bucketing through edges by band cost more than it saved
(1.91 G). Hoisting each edge's slope saved 1%.

**Repeats.** One 1× walk asks 30 624 pixels in 7 613 meets on 19 pages; the top ten are named in
ADR 1503. 68 asks repeat a pixel under the same mark and chain. 14 719 repeat it under another
mark (59% of `bug1721218`'s), and that is the next lever: keep the clip's side of a pixel for the
frame. After it come 14 060 small region fills at 1.13 G.

**Fixture.** `a_finely_flattened_curve_meets_as_its_pieces_do` meets a 3000-gon with a half-plane
over 144 pixels, both windings and both rules, against the convex closed form to 10⁻⁹. It was
watched failing at 1.0 with a run's span left short.

**Gates.** Exit statuses: `rustfmt --check` on `meet.rs` 0. On the exported HEAD-plus-change tree,
clippy `-D warnings --all-targets` on `raster-gpu` and `render-raster` 0, and `cargo test` on
`raster-gpu` 0. `cargo test -p conformance` 0. In the worktree, `cargo test -p render-raster` 0.
Behind the lock, `render-raster --test corpus` on three lanes at 1× and 4× was 0 on each run,
with 0 of 966 / 962 per-page digests moved against HEAD and one-versus-many 0.
`raster_golden` 0, unchanged. `headless_gpu` 0, 39 passed. `launch_path` 101: one figure moved,
`xfa_filled_imm1344e` open at 1821.6 k against a ceiling of 1820. HEAD reads 1819.4 there. The
2.2 k difference is all in unnamed libc code reading `/proc/self/maps`; no named function differs.
