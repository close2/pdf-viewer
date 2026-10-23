# 1717 — The union's dropped stretch, and the joins the pieces lost

Session 1717, slot 6 of batch 110, a pixels round; 2026-10-23. ADRs 2268, 2269; no `Q`.

**Premise.** Held: the thirteen declines of ADR 2252 §1 are one defect and go to zero (pr12564 9,
issue14415 3, issue12823 1 at 1x, none at 4x). **Not held, thrice.** (1) The cause is not the
walk's bookkeeping but its input: `union::approximate` split a determinant at bit 64, so a small
negative one rounded by up to `2^10`, two crossings of one point read `2.3e-9` apart past
`SEPARATION`, and a contour was walked one winding off; split at bit 32 it is the nearest `f64`
(ADR 2268). The smallest mark, three triangles and nine vertices of pr12564, is a test. (2) No piece
was culled by a chord's bound: the lost joins were a curve of no length whose chords were roundings
(issue14438), a round join's sweep taken from the sign of two nearly opposite normals (inks' notch
at a sixty-fourth), and `push_convex` winding small pieces from an `f32` area about the origin
(hairline holes at 64x) (ADR 2269). (3) The paperclip bound does not tighten under 8 levels: the
union's worst pixel is 10.4 from the curve, which is the flattening; a new unit test holds the
paperclip's pieces to their own flattened set within a level, red on the library at (796, 563).

**Guard.** A row that does not close and a walk that does not return are `debug_assert!`s (planted,
seen to fire); release still declines to the library, silently — a loud refusal needs a typed
error through `scan::mask_fill`, outside this slot. Three marks stay declined, each a contour with
no point to start from: a path that retraces itself (fileattachment 1x, 4x) and one inks piece.

**Pixels.** `raster_golden` 9 of 974 moved, raster only, re-taken; figures and reading in ADR 2269
§4. Every page's worst pixel drawn beside HEAD's; none reads worse.

**Files.** New: ADRs 2268, 2269, this record. Edited: `render-cpu/src/{area.rs,area/union.rs,
stroke/folding.rs}`, `render-cpu/tests/folding_stroke_under_a_clip.rs` (a doc sentence),
`pdf-model/tests/raster_golden.tsv` (9 lines), the §10.7.4 row's note, `doc/state-of-play.md`.

**Gates.** Worktree: `rustfmt --check` per file 0; `cargo fmt --check -p render-cpu` 0; `-D warnings
clippy -p render-cpu --all-targets` 0; nextest 246 of 246; `tools/state.sh --round 1717 docs` 0;
`cargo test -p conformance` one failure, a sibling's `viewer-ffi` names, and 0 in the export but
for five `bounded.rs` sweeps that read git's population; `--test evidence --test adr_status` 0.
Exports: `raster_golden` 965 held, 9 moved, re-taken 0; `fixed_documents` 5 and `hostile_geometry`
4, `--include-ignored`, 0 (and `hostile_geometry` with debug assertions 0); the 26 folding pages at
1x and 4x with debug assertions, 52 of 52 exit 0; the six arms against HEAD's (`tools/batch.sh
arms`): 0 of 5 799 digests moved, mean error down on all 24 page-lanes it moved in; `t2-turn_path`
under `--clock`, first at load 9 to 11 with 2 of 27 outside on pages `render-raster`'s graph reaches
without `render-cpu`, then at load 1.1 33 of 33 inside. Each fix red on its fault planted back.
Duration 10 300 s, about 155 tool uses.
