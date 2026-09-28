# 1255 — A row is settled by clusters, and a stroke is its set

Batch thirty-nine. §10.7.4 stays `partial`, narrowed to one residue; §10.7 follows unchanged.

## The cost (ADR 1347)

Profiled per function: ADR 1341's walk cut every suspect row at every piece end and deposited
every piece per sub-strip. Now a row is split into clusters (pieces sharing pixel columns; the
winding left of one is the same at every height), only a cluster whose pixels can take three
windings is walked, a chain of edges is one strand cut only where it meets another, runs are
deposited once. Callgrind, five draws, against ADR 1341: `issue14415.pdf` −10.9%, `issue19802.pdf`
−21.3%; against the tree before 1341 still +31.0% and +18.8% (72% of walked clusters genuinely
leave the range), `issue20232.pdf` +5.8% against 1341. Held to the whole-row walk on 3000 random
marks (60 000 once); `raster_golden` moved one level in pixels both constructions measure exactly.
Item 3's residue is kept and priced: every mark by clusters costs page 101 +114%.

## Items 8 and 4, and 1258's ring (ADR 1348)

A solid fill its matrix carries onto a line along a page axis is §10.7.4's line (fixture: 80 px of
one row; planted away, refused). A stroke so collapsed stays owed. Item 4's image edge was paid
already (new witness 0.502 vs 0.255); a group below opacity 1.0 is a §10.7.1 choice. A curve
stroked wider than it bends is the union of convex pieces (disk 1256.03 against 1256.72); it goes
to the library's converter, because the exact walk on those pieces reached 20.5G instructions.

## Files outside the brief's list

`pdf-render/src/{collapsed,display_list,lib,paint}.rs`, `pdf-model/src/content.rs`,
`pdf-model/src/content/report.rs`, `pdf-model/tests/variable_text.rs` (a singular `Tm` onto the
x-axis now draws its line), `pdf-model/tests/raster_golden.tsv` (35 lines), `doc/state-of-play.md`,
`doc/checks/fixed-documents.toml` (0546320 and 1407697 no longer report the matrix: each mark is a
fill ADR 1348 draws as its line; inks 23.771 and 13.790 inside their bands).

## Gates

fmt, clippy `-D warnings`, tests: render-cpu, pdf-render, pdf-model 0; conformance 0. Under the
lock, isolated on HEAD plus these files: `raster_golden` 35 moved, looked at, regenerated;
`pdf-model --test corpus` 0; `render-raster --test corpus` 946/5 before and after; oracle: no new
contradiction (its 7 "no longer contradicted" are specs and a submodule the export lacks).
