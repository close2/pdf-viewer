# 1635 — A clip that contains a mark does not band its draw

Slot 5 of batch ninety-nine, 2026-10-20, a clause round on §10.7.4. ADR 2106; no question.

**Premise and hypothesis.** The premise held: the paperclip of veraPDF's `6-3-1-t01-fail-f`, as a
stroke alone with and without `0 0 7 17 re W n`, is 255 against 240 at `(798, 567)` at 10x and
nothing else at 1x, 4x or 10x. Its area, ladders at 16x and 64x, is 242.2 — so neither drawing is
the integral. The hypothesis did not hold: both take one converter, ADR 1348's pieces through
`tiny-skia`'s sixteenths. The clip, removing nothing, was left off (ADR 1095) but still banded the
draw: a 173-row surface 546 rows down, ADR 0219's residual, one sample. ADR 0165 is not involved.

**Declined, measured.** Pieces by `crate::area`: 243 under both, but `inks.pdf` 0.28 → 31.0 G
instructions. Library coordinates on an `f64` device grid: the pieces' overlap rounds to an
abutment, the radius-20 disk 1255.23 / 1255.60 against 1256.72 ± 1. Both in ADR 2106 section 3.

**Built.** `MaskCache::effective` answers a mark its clip contains as an unclipped one — no mask
built, the surface's rows. `render-cpu/tests/folding_stroke_under_a_clip.rs`, the witness's
geometry and matrix, asserts the two drawings byte-equal; red on HEAD (255/240), green after. The
converted witness against its annotation: HEAD 0 px at 72 dpi and 1 at 720, after 0 and 0.

**Found.** At `issue14415.pdf` 4x pixel `(1628, 407)` the lattice covers 0.88 where the ladder
says 0.683 and the area 0.682: folding strokes are up to 50 levels off the integral, priced and
left (ADR 2106 section 4).

**Rows.** §10.7.4 stays `departed`; its note's "exact for every shape" names the pieces' exception
and gains the band sentence; the new test is on the row. Shared files: `mask_cache.rs` and
`raster_golden.tsv` (42 lines), each as `-U0` hunks in `scratchpad/r1635/edit/`.

**Gates.** rustfmt per file clean; `cargo fmt --check -p render-cpu` and `RUSTFLAGS=-D warnings
cargo clippy -p render-cpu --all-targets` exit 0; `render-cpu` nextest 221 passed, 0 failed;
`cargo test -p conformance` ledger 326 passed, `records` 1 failed on sibling record 1634's Gates
paragraph; `tools/state.sh docs` exit 0. On the export (HEAD + the hunks): six arms 0 of
968/968/968/965/965/965 frame digests moved, each exit 0, oracle error moved on 3 pages;
`raster_golden` 42 of 974 moved, raster only, each at most 1 level, looked at, TSV regenerated (exit
0 after); division census HEAD 173 → 183 pages moving, at most 943 px and 3 levels, 11 past the
bound both; `fixed_documents --ignored` 5 passed, `hostile_geometry` 4, `strip_parallelism` 2.
Callgrind: page 101 −1.1%, `issue840.pdf` −9.2%, none dearer than +0.1%.

**Unfinished.** A cheaper exact measure of a union of convex pieces. About 6 000 s.
