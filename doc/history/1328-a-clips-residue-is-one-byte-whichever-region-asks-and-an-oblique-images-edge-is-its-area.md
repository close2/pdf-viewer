# 1328 — A clip's residue is one byte whichever region asks, and an oblique image's edge is its area

Raster, from the clauses alone. ADRs 1491, 1492. No question. `crates/render-cpu/` was not opened (A76).

**Why admission moved bytes (ADR 1491 section 1).** A probe compared every per-tile residue fill on
`bug1721218_reduced.pdf` (185 196 asks) with the region crop, and both with `area_in_pixel` and an
independent `f64` integral. Three pixels differed. Each lies within 4·10⁻⁵ of a level's half, and
two tiles over one pixel also disagreed with each other. Neither construction is wrong by geometry;
`f32` cannot decide those pixels. A clip's residue is now filled in `f64` (`clip_mask`, a generic
`fill_over` and `exact.rs`). Marks keep `f32` for the compute lane's sake. All asks now equal the
rounded area, and a new test holds a tile to its region's crop on 2.9 M pixels.

**The rule, priced (section 2).** Admit when `region + 16·pieces ≤ uses × (tile +
16·pieces·tile_rows/region_rows)`. The 16 is measured: 6.4 ns a byte against 103–143 ns a row
piece. The archetype rows moved: dense text 0/40 → 2/0, artwork 67/380 → 164/68. Texels are
unchanged, and `raster-pages` records the new rows.

**Fewer-side winding (section 3)**, measured alone: `area_in_pixel` 4.95 G → 3.26 G, 0 pages moved.
On the page the instruction count goes 18.47 G → 18.25 (`f64`) → 11.19 (priced) → 9.51 G (fewer
side), and the zoom step 514 → 274 ms at load 9. The CPU backend takes 84 ms.

**Oblique image (ADR 1492).** §10.7.4's image paragraph is the centre rule. The ledger row's
departure (1) and ADR 0805 read an image's edge as partly painted, so the shader now takes the
parallelogram's area in the pixel. The encoder meets it with a residue through ADR 1467's edges.
The fixture was watched failing against HEAD's shader and against the oblique meet switched off.
For the ledger round: the §10.7.4 row's `code` could name `image.wgsl` and `encode/rare.rs`.

**Corpus.** Exported trees, own target directories, per-page digests against HEAD. At 1×, four
pages move on all three lanes, every one an oblique image (counted by a probe, since removed), all
closer to the oracle or equal to it. At 4× there are eight `f64` pages, equal to four places, plus
four oblique ones. Verdicts are unchanged, and one-versus-many is 0 on all sixteen runs.

**Gates.** Exit statuses: `rustfmt --check` on my files 0. `RUSTFLAGS=-D warnings cargo clippy
--all-targets` on `raster-gpu` 0, `raster-pages` 0 and `render-raster` 0. `cargo test` on
`raster-gpu` 0, `raster-pages` 0 and `render-raster` 0. `cargo test -p conformance` 0. Tier 2,
behind the lock, on an exported HEAD+change tree: `render-raster --test corpus` CPU, GPU and
compute at 1× and 4×, each 0. `pdf-model --test raster_golden` 0, unchanged, since render-cpu is
untouched. `render-gpu --test headless_gpu` 0. `viewer-ui --test launch_path` 0.
