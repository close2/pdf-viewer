# 1643 — A folding stroke's union measured from its boundary is exact, and held out for its price

Slot 5 of batch one hundred, 2026-10-20, a clause round on §10.7.4. ADR 2122; Q538.

**Premise and hypothesis.** The premise held: ADR 1348's pieces go to `tiny-skia`'s sixteenths,
50 levels off at `issue14415.pdf` 4x `(1628, 407)`, and the paperclip witness reads 255 where its
ladder is 242.2. The hypothesis did not: one non-zero accumulation of every piece's edges is the
winding number's integral, which `crate::area` already takes and which overcounts at a fold's rim.

**Built, in a patch.** `render_cpu::area::union`: the filled set's boundary as stretches of the
pieces' edges, each contour walked once from a winding a crossing or a ray fixes, predicates exact
on a `2^-21` grid with simulation of simplicity, then one ordinary accumulation; `mask_fill` routes
`Exact::Pieces` to it, and `folding_stroke_under_a_clip.rs` gains a ladder test red on HEAD (255
against 242.3). Two defects a dump probe found: a retried ray reused its marks, and a ray point on
a collinear edge read the sliver. Against the set walk, 378 marks of the three pages at 1x and 4x
within one level; the witness pixel 243.

**Priced, and held out.** `callgrind_rasterise` page 1, three draws: `inks.pdf` 281 870 234 →
694 269 784 (+146.3%), `rotated_ink.pdf` 16 958 633 → 24 674 242 (+45.5%), `issue14415.pdf`
984 881 667 → 1 150 169 962 (+16.8%). Past the fifth on two of three, so nothing is routed: the
patch is `scratchpad/r1643/edit/folding-union-boundary.patch` (1 596 lines, applies to HEAD), the
owner is asked (Q538, recommending the wait), and the §10.7.4 note states the sixteenths' bound.

**Rows.** §10.7.4 stays `departed`; its note's pieces sentence states the bound and ADR 2122's
prices. No pixel moves: `crates/render-cpu` is HEAD's plus slot 2's hunks, mine reversed as `-U0`.

**Gates.** With the patch, in the shared worktree: `rustfmt --edition 2024` per file clean, `cargo
fmt --check -p render-cpu` exit 0, `RUSTFLAGS=-D warnings cargo clippy -p render-cpu
--all-targets` exit 0, `cargo nextest run -p render-cpu` 224 passed of 224. In the tree as left:
`cargo test -p conformance` 450 passed, 0 failed (an earlier run's one failure, `round_numbers`
on slot 2's scratch `pdf-model/examples/r1640_isolate.rs`, gone with that file). On the export (HEAD + the patch): `raster_golden` held 949,
moved 25 of 974, raster only (exit 101, the TSV not regenerated since nothing is routed), division
census 183 pages moving, at most 943 pixels and 3 levels, 11 past the bound, as on HEAD;
`fixed_documents` 5 passed and `hostile_geometry` 4, `--include-ignored`. The six arms are not owed.

**Unfinished.** A measure under a fifth: at least 43% of `inks.pdf`'s crossings bound nothing.
About 6 200 s.
