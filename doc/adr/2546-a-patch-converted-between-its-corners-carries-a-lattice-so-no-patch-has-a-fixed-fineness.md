# 2546 — A patch converted between its corners carries a lattice, so no patch has a fixed fineness

Status: implemented. Session 1856. Amends ADR 1217 decision 3 (a mesh is no longer kept in `pdf-colour` "all or
none") and ADR 1058 decision 4's second half (the 121-vertex grid), and carries out the lifting ADR 2331 named for
§8.7.4.5.7 and §8.7.4.5.8, which move from `departed` to `implemented`.
Context: ISO 32000-2 §8.7.4.4, §8.7.4.5.7, §8.7.4.5.8, §10.7.2, §10.7.3, Table 52; ADRs 0028, 0778, 1058, 1217, 1622,
2170, 2331, 2532.
Code: `crates/pdf-render/src/shading.rs` (`PatchCorners::Grid`, `PatchGrid`, `MAX_PATCH_GRID_CELLS`,
`SurfacePatch::steps`, `SurfacePatch::tessellate`), `crates/pdf-colour/src/mesh.rs` (`MeshReader::patch_colours`,
`bends`, `LATTICE_CONVERSIONS`), `crates/pdf-colour/src/shading.rs` (`Cache::mesh_bytes_held`),
`crates/pdf-model/src/content/list_budget.rs` (`list_past`), `crates/viewer-confined/src/protocol/display_list.rs`
(corner tag 3), `crates/test-scenes/src/lib.rs` (`patch_mesh`'s third patch),
`crates/pdf-model/examples/mesh_triangle_census.rs`.

## Context

ADR 1217 sent every patch to the backend, which derives its fineness against §10.7.2's device pixels — except a patch
whose colours §8.7.4.4 requires be interpolated in a space no backend holds and whose conversion is not linear across
it to §10.7.3's tolerance. That one's interior colours have to be converted between its corners, which only
`pdf-colour` can do (ADR 0028), so its whole mesh stayed in `pdf-colour` and was evaluated at the fixed
`mesh::PATCH_STEPS` of ten: a silhouette of ten chords whatever the magnification, the departure from Table 52's
"[t]he precision with which curves shall be rendered on the output device" that ADRs 1622, 2170 and 2331 recorded.
ADR 2331 named the lifting: a colour grid carried beside the control net through `pdf_render::SurfacePatch`, the
silhouette derived in device pixels while the colours stay converted where they are.

## Decision

1. **A third thing a patch's corners carry**: `PatchCorners::Grid(PatchGrid)`, colours at a uniform lattice of the
   unit square, `(cells.0 + 1)·(cells.1 + 1)` of them, `u`-major, each count from 1 to `MAX_PATCH_GRID_CELLS` (32,
   half of `MAX_PATCH_STEPS`). `PatchGrid::new` refuses any other shape, and the decoder on the far side of the
   sandbox builds one only through it.
2. **The lattice is found in `pdf-colour`, in parameter space**, by `MeshReader::patch_colours`: the whole patch is
   one cell; each cell is asked ADR 1058's question — at its four edge midpoints and its centre, is the conversion of
   the bilinearly interpolated components within §10.7.3's tolerance of the bilinear mix of the cell's four converted
   corners? A failing midpoint of an edge along `u` doubles the cells along `u`, one along `v` doubles `v`, the
   centre alone doubles both, and the lattice is converted again. A patch the first level clears travels as four
   colours, exactly as before; its question costs the nine conversions it cost.
3. **Bounded before the work.** Every later level's conversions — the lattice built, then its cells asked — are
   charged to `LATTICE_CONVERSIONS` (`MAX_TRIANGLES`, 262 144 a mesh) before they are made, and a count past the cell
   bound is not attempted. Either stop leaves the lattice it has, the patch is drawn, and the mesh is reported as
   `max_mesh_refinement`, the name ADR 1058 gave the same fact for a triangle. What a mesh's lattices hold is at most
   that many colours, four mebibytes. §10.7.3 licenses both bounds: "[e]ach output device may have internal limits".
4. **Every patch travels.** With a lattice there is no patch a device cannot carry, so `mesh::tessellate`,
   `Corner::emit_patch`, `cell_error` and `PATCH_STEPS` are deleted, and with them the all-or-none rule: one
   sequence of patches decides §8.7.4.5.7's paint order for every mesh.
5. **The device draws a lattice patch a whole number of steps per lattice cell.** `SurfacePatch::steps` takes the
   silhouette as before, the colour from the largest cross term over the lattice's cells, counted per cell, and
   rounds each axis up to a multiple of its cell count, so every cell the device draws lies inside one lattice cell
   and is mixed from that cell's four colours. A cell count of one is the identity, so no other patch's fineness
   moves. Within the patch the device's `v`-outer order keeps ADR 0778's precedence.
6. **The meshes a page builds are charged to its memory bound.** `shading::Cache::mesh_bytes_held` counts the
   triangles, patches and lattices of every mesh the cache builds, and `list_past` adds it to the presses beside the
   list (ADR 2532's construction), so a page of many distinct meshes is refused by `Unsupported::ListBytes`.

## Consequences

- **Both rows are `implemented`.** No patch is drawn at a fixed fineness: `mesh::PATCH_STEPS` is gone, and every
  patch's silhouette is derived against half a device pixel under the device transform, which is Table 52's
  "[t]he precision with which curves shall be rendered on the output device" met for the one branch ADR 1217 left
  departed. A patch whose conversion is linear across it travels as four colours exactly as before, so a mesh that
  never took the branch draws the same pixels: no page moves against HEAD's six arms, no line of
  `raster_golden.tsv` moves, and `examples/mesh_triangle_census` finds no lattice page among the 1 477 files of
  `doc/pdf.js/test/pdfs` and `doc/corpora/`.
- **The branch is not empty in the world.** 30 of the 405 `corpus-cache` files whose uncompressed bytes state a type 6
  or 7 shading take it, 19 of them stopped at a bound and reported. On their page ones against HEAD, 15 move by more
  than two levels: the silhouette's chords become curves (Table 52), the fixed grid's facets go (§10.7.3), and a mesh
  the fixed grid cut at `max_mesh_triangles` is drawn whole. No high-water rises by a mebibyte, and one falls from
  1 204 348 KiB to 59 024 KiB. Five page ones now report `max_mesh_refinement` where HEAD reported nothing: a real
  conversion reaches the 32-cell bound, which is the limit a later round may widen.
- **The plant.** Forcing a lattice patch's steps back to ten by ten fails
  `a_lattice_patchs_silhouette_follows_the_device`: the pixel the derived silhouette covers at the device scale is
  left bare by ten chords. The test therefore tells the derived fineness from the fixed one.
- **The bounds, probed.** A hostile file — one patch, or 16 384, of a `Separation` tint whose tint transform is a
  step at one half and whose corners put the step across both middle lines, so no cell ever converges — is drawn
  under both bounds and reported as `max_mesh_refinement`. One patch refines to the 32 by 32 cell bound; down the
  16 384, `LATTICE_CONVERSIONS` runs out and the rest keep their four corners. High-water beside HEAD's, at a load
  average near 19: one patch 12 544 KiB against 12 192 KiB; 16 384 patches 31 772 KiB against 47 828 KiB at scale 1
  and 44 816 KiB against 54 480 KiB at scale 8. The wall time at scale 8 grows from 0.21 s to 1.15 s because HEAD's
  fixed ten-by-ten grid ran into `max_mesh_triangles` and dropped the mesh's tail, which every patch now reaches the
  device and is drawn under `MAX_PATCH_TRIANGLES`.
- **What a page's meshes hold is charged to its memory bound** (decision 6), so the lattice's colours cannot be
  multiplied past the list's bound by a page of many distinct meshes.
- **The limit that remains is §10.7.3's own "internal limits on the maximum and minimum tolerances attainable"**:
  `MAX_PATCH_TRIANGLES` is shared equally between a mesh's patches, so on a mesh of many patches a lattice patch's
  share can give fewer steps than it has lattice cells; the lattice is then sampled at the device's vertices rather
  than drawn a whole number of steps per cell, and the colour between them is the device's bilinear mix. The
  lattice's two bounds, `MAX_PATCH_GRID_CELLS` and `LATTICE_CONVERSIONS`, are the others, and each stop is reported.
- The wire carries one more corner shape (tag 3), decoded only through `PatchGrid::new`, so a confined worker's list
  that states a lattice its counts and colours disagree on is refused as `Unbuildable`.

## Alternatives rejected

- **Split a bending patch into sub-patches by de Casteljau subdivision, each with four converted corners.** No new
  wire shape, but §8.7.4.5.7's fold precedence is an emission order: two sub-patches of one row are emitted whole one
  after the other, so a fold across them would answer with `u`'s order over `v`'s — ADR 0778's tie-breaker promoted
  over its rule — at the granularity of a row of pieces; and the device budget is shared per patch, so every split
  would thin every other patch's share.
- **A lattice refined cell by cell (a quadtree).** A device step would then straddle cells of different sizes, and
  the whole-number-of-steps condition that keeps each drawn cell inside one lattice cell could not hold.
- **Raise `PATCH_STEPS`, or derive it from the transform `pdf-colour` holds**: ADR 1217's rejections stand.
- **Carry components and let the backend convert**: ADR 0028.
