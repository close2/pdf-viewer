# 1268 — The walk's floor taken apart, and the floor owed per shape

Batch forty-one, render-cpu slot. No ledger row moved: §10.7.4 stays `departed` (ADR 1360's point),
and nothing here was a requirement not yet executed. ADRs 1373, 1374.

## The walk's cost (ADR 1373)

Off-arm ladder, `callgrind_rasterise`, five draws, one export per arm (trap 50): on
`issue19802.pdf` the scaffolding before the walk is 244 M (trace 37, `sort_into` 38, `by_row` 50,
`gather` 61, `cluster` 58) and the walk 128 M. An exact detector, a chain number per cell in the
accumulation, costs page 101 +7.2% unused; row sums cannot be exact (trap 54); runs in the cluster
sort cost `issue20232.pdf` +1.49%. None kept. ADR 1359's floor stands, now with its parts.

## The corner pixel (ADR 1374)

The brief's premise was half right. The pixel is not abutting pieces losing a sample: it is the
pixel holding the square's corner, 0.885 of a level of area, which the eight pieces (closed form,
ADR 0590) round to one and the whole square (`tiny-skia`'s 8.8 rectangle converter, ADR 0476)
truncates to none. The pieces are right. A sweep of 600 redactions found the real defect beside it:
a column of one level outside an edge that `f32` puts a ten-thousandth past a pixel boundary,
lifted by ADR 0419's floor. The floor is now asked per shape (`scan::owed_a_floor`). Exact
single rectangles were built and priced: `colors.pdf` +183% through the buffer, page 6 +48% as
bands; ADR 0476 stands.

## Arcs

A radius-4 and a radius-1 circle at `8 w`: the pieces' union, measured exactly, is within
`FOLD_FLATNESS`'s bound at scales 1 and 2 (0.150 against 0.785 at worst). Now a unit test.

## Gates

fmt, clippy `-D warnings` render-cpu, nextest render-cpu 174/174, `cargo test -p conformance` 0.
Under the lock: `raster_golden` 11 moved, all mine (each diffed against HEAD's own export, every
pixel one level lighter at an edge), rows regenerated, then 0; `render-raster --test corpus` 958
agree / 2 differ (`issue19083`, `issue2177`), exit 0; `pdf-model --test corpus` 0;
`fixed_documents` 0; oracle 0. Cost against HEAD: every page within ±0.03%.

Files: `crates/render-cpu/src/{scan,area,lib}.rs`, `crates/render-cpu/tests/abutting_rectangles.rs`,
`crates/pdf-model/tests/raster_golden.tsv` (11 rows), `doc/todo/11`, ADRs 1373 and 1374.
