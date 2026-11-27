# Round 2268 — a neutral colour is not divided by zero, and the meet's savings are measured

A pixels round in `pdf-viewer-audit`, rebased onto main `a59e8c437`.

## issue18032's dark pixels

Five pixels inside white glyphs, drawn black by raster. Reduced: a white "d" partly covering a
pixel under an isolated group of blend mode `Color` painting white. §11.3.5.3's `ClipColor`
divides by `x − l`; the backdrop's straight colour, divided on the device, stood a hair past 1.0,
white is neutral, `x == l`, and zero over zero stored as black. Both divisions are now taken only
where the denominator is positive, in `composite.wgsl` and in the CPU oracle's `blend.rs`, where
the division was latent (ADR 3773). vello drew the reduced scene white; its `clip_color` is the
same unguarded formula, upstream.

## The meet's exact savings

Callgrind on the Type 3-in-Type 3 page, one rayon thread, `gates` profile, every digest the
baseline's. Raster 63.38 G → 61.94 G (−2.3 %): each edge's rows found once in `RowEdges::build`.
Unstable meet sorts measured worse (66.86 G), the boundary order carried between bands even. The
oracle passes 42.5 M box-meeting edge pairs to the exact test; a prefilter saved nothing (31.03 G
against 31.01 G), an `i64` determinant cost more (31.84 G). ADR 3774 has the table.

## Pixels and turns

First pages of 957 corpus documents, raster and oracle digests before and after: raster moved
`issue18032` alone, its five pixels; the oracle moved none. Turn walk: the Type 3 page 2548 ms,
inside the band A1073 kept (main's walk 2307); EC3 p101 outside (35.3 ms against 34.0, children
35.3, 1982 and 34.2 at loads 33–118) — it makes no exact meet and no non-separable blend.

**Ledger.** 11.3.5.3 `implemented` → `implemented`: `composite.wgsl` and the two tests on the row.

**Gates.** On `a59e8c437`: fmt 0; clippy `-D warnings --all-targets` of `-p render-cpu -p
render-raster -p raster-gpu -p render-gpu -p pdf-model -p viewer-core -p test-scenes` 0;
`cargo test -p conformance` 0 but this record's own gates paragraph, now written. On `41c9f29d1`:
`pdf-sandbox` 0; nextest of the seven 5034 passed, 2 failed, the manifest's two (no difference);
`raster-gpu` and `render-raster` again after the last edit, 953 passed; rustdoc of `render-cpu`,
`raster-gpu`, `render-raster` 0. Turn walk 101 (above).
