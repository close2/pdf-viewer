# 1856 — A patch converted between its corners carries a lattice, so no patch has a fixed fineness

Slot 6 of batch 126, 2026-10-28, a "clause" round (Opus 5.5) in two sessions; ADR 2546 used, ADR 2547 and Q722 unused.
I worked in `scratchpad/r1856/` and in two exports built there, their targets deleted. Ledger: §8.7.4.5.7 and
§8.7.4.5.8 `departed` to `implemented`; §8.7.4.4's note names the lattice; `doc/todo/65`'s paragraph says so.
**Files.** `crates/pdf-colour/src/{mesh.rs, shading.rs}`; `crates/pdf-render/src/{lib.rs, shading.rs}`;
`crates/pdf-model/src/content/list_budget.rs`, `crates/pdf-model/tests/shadings.rs`,
`crates/pdf-model/examples/mesh_triangle_census.rs`; `crates/viewer-confined/src/protocol/display_list.rs`;
`crates/test-scenes/src/lib.rs`; `doc/conformance/ledger.toml`, `doc/todo/65`, ADR 2546.

**Premise.** "One constant at one branch; ADR 1217's device-pixel fineness with a colour test added": half held. The
fineness was one constant, but the branch kept a whole mesh in `pdf-colour` (ADR 1217's all-or-none rule), where no
device transform exists, so a colour test there could not have derived Table 52's silhouette. ADR 2331's named lifting
is the construction instead: a lattice travels beside the control net. **Built.** `MeshReader::patch_colours` asks ADR
1058's nine-point question of the whole patch, then of each cell, doubling along each axis that still bends, under
`MAX_PATCH_GRID_CELLS` (32 per axis) and `LATTICE_CONVERSIONS` (262 144 a mesh), both charged before the conversions
are made, either stop reported as `max_mesh_refinement`. `mesh::PATCH_STEPS`, `tessellate`, `emit_patch` and
`cell_error` are deleted. `SurfacePatch::steps` rounds each axis to a multiple of its cell count. The wire's corner
tag 3 decodes only through `PatchGrid::new`; a page's meshes count to its list bound.

**Found.** The premise that no corpus document takes the branch held for `doc/pdf.js/test/pdfs` and `doc/corpora/` (1
477 files, none) and not for the world: 30 of the 405 `corpus-cache` files whose raw bytes state a type 6 or 7 shading
take it, 19 stopped at a bound. Their page ones against HEAD: 15 move past two levels (chords become curves, facets
go, a mesh HEAD cut at `max_mesh_triangles` drawn whole; three looked at), five newly report `max_mesh_refinement`
(the 32-cell bound reached), no high-water up by a mebibyte. ADR 2546 has the figures. **Plant.** Forcing a lattice
patch's steps to ten by ten fails `a_lattice_patchs_silhouette_follows_the_device` (a pixel the derived silhouette
covers is left bare); reverted. **Probe.** One never-converging patch (a step tint transform across both middle lines)
and 16 384 of them, HEAD's high-water beside ours at a load average near 19: 12 192 against 12 544 KiB; 47 828 against
31 772 KiB at scale 1 and 54 480 against 44 816 KiB at scale 8, wall 0.21 s against 1.15 s there (every patch now
drawn).

**Gates.** In the export: `cargo nextest run` on pdf-colour, pdf-render, pdf-model, viewer-confined and test-scenes
exit 0 (2 920 passed); clippy `-D warnings --all-targets` on the five exit 0 after two fixes of my own tests; `cargo
fmt --check` on the five exit 0; `raster_golden` exit 0, no line moved; the pdf-model corpus gate exit 0; the six
render-raster arms exit 0 each, 0 of 968 or 965 pages moved against HEAD's six arms;
`cpu_and_quorra_agree_on_a_patch_mesh` and `cpu_and_gpu_agree_on_a_patch_mesh` exit 0; the locked census exit 137 (its
6 GiB tree ceiling; one safedocs file holds 2 GiB at HEAD alike), then per file: 1 882 runs exit 0. In the worktree:
`cargo check --tests -p pdf-render -p pdf-model` exit 0; `--bin quotations` exit 0, `--bin cited` exit 0; `cargo test
-p conformance --no-fail-fast` exit 101, two reds, both siblings' (records 1854 and 1858).
