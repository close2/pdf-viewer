# 1664 — The folding stroke's exact measure priced at HEAD, and its chords asked

Session 1664, slot 2 of batch one hundred and three, a pixels round under `doc/questions/A538`;
2026-10-21. ADR 2164 (2165 unused), Q559. Nothing of this round is in the tree's code: no pixel moves.

**Prices**, `callgrind_rasterise` page 1, three draws, scale 1, `RAYON_NUM_THREADS=1` (the setting
that reproduces ADR 2122 §5 to 0.1%). HEAD 66c6ef23: `inks.pdf` 283 228 522, `rotated_ink.pdf`
16 958 381, `issue14415.pdf` 985 610 733. ADR 2122's walk rebased: 694 658 593 (+145.3%), 24 692 616
(+45.6%), 1 151 226 506 (+16.8%). The clipped pieces (ADR 2164 §3): 933 072 625 (+229.4%),
29 958 783 (+76.7%), 1 275 196 858 (+29.4%). The walk with chords at 1/20 px: 324 381 784 (+14.5%),
18 255 722 (+7.7%), 1 062 902 901 (+7.8%). At 24 threads HEAD 302 015 255 / 28 869 491 /
1 383 250 540; walk +144.9 / +95.5 / +12.9%; walk at 1/20 +21.8 / +63.6 / +6.9%.

**The hypothesis did not hold.** The bucket the brief named was already ADR 2122 §3's grid, and the
discard acts after the pair is compared: `Pairs::compare` alone is 80.3 M instructions an `inks` draw
against the 67.9 M the whole measure may cost under the fifth. Built the lever's end — convex pieces
clipped by half-planes, no crossing between two pieces recorded — and it costs more: the price is the
~300 000 overlapping edge–piece pairs a draw, not the records. What reaches the fifth is coarser chords
(`FOLD_FLATNESS`, sixty-fourth → twentieth): the ladder pixel then reads 245 against the curve's 242.4
(today 255), which changes what the gate measures — Q559, recommending 1/20 with a boundary shared
between strips. Halving curves so the stroker takes the non-folding halves: 87.8 G instructions.

**Probes (habit 148)**, union against the set walk on every mark of the three pages at 1x and 4x: the
clipped pieces had two defects, both found and fixed — a stroker lobe wound against the pieces cancels
them (255 levels, `issue14415`), and a piece edge along an outline edge read by two tie rules (19
levels, reduced to 2 contours); after, within 1 level, 44 of 175 `issue14415` marks declined. The walk
at 1/20: within 1 level everywhere, one `inks` mark declined.

**Files.** New: `doc/adr/2164-…`, `doc/questions/Q559-…`, this record. Edited: ADR 2122's status line,
the §10.7.4 row's note (one sentence). Outside the tree: `/home/AI/r1664-folding-union/` (README, the
rebased walk patch and the clipped-pieces patch, both `git apply --check` clean on 66c6ef23). Shared:
`scan.rs` with slot 3 — my three hunks applied and reversed by asserted replacement, theirs untouched.

**Gates.** Both patched trees: `rustfmt --check` 0, `RUSTFLAGS="-D warnings" cargo clippy -p
render-cpu --all-targets` 0, `cargo nextest run -p render-cpu` 236 passed, the ladder test green
(243 against 242.3). Final tree: `crates/render-cpu` is HEAD's for my files, so the six arms,
`t2-raster_golden`, `hostile_geometry` and the fixed-documents test are not owed (no pixel moves);
`cargo test -p conformance`: every binary ok but `records` (1 failed, sibling 1667's record).
Duration: about 5 000 s.
