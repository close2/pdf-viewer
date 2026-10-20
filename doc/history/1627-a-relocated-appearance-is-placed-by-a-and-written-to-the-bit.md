# 1627 — A relocated appearance is placed by `A`, and its matrix is written to the bit

Slot 5 of batch ninety-eight, 2026-10-20, a clause round. ADRs 2090, 2091; no question.

**Premise and hypothesis.** The premise held: `6-3-1-t01-fail-f` relocated differs from its
annotation in 47 pixels at 72 dpi (at most 17/255), 477 at 720. The hypothesis did not: the
annotation path snaps nothing — its display list and that of the same stream under `1 0 0 1 75.0341
720.288 cm … Do` carry one transform. The relocation wrote `75.03 720.29` (`preserve::number`, two
places); that operand alone rewritten gave 0 pixels at 72 dpi, 1 at 720 (control re-encoded: 0).

**Found on the way.** `fail-g`'s appearance states a translating `/Matrix`, and its line was
relocated 759 pixels away (looked at): the converter's `cm` was §12.5.5's `AA` and §8.10.1's `Do`
applied `/Matrix` again. With `cm` set to `A` by hand the page was byte-identical at 72 and 720 dpi.

**Built.** ADR 2091 in `pdf-model`: `annotation::alignment` is steps 1–2 (`A`), `placement` is
`/Matrix` then `alignment` (the annotation path's arithmetic unchanged), and `appearance::placement`
hands the converter `A`; its test red on HEAD (`q 2 0 0 2 -180 -370 cm`). ADR 2090 is slot 1's to
apply: `scratchpad/r1627/relocation-matrix-for-slot-1.patch`, five `-U0` hunks — `preserve::
matrix_operand` (the f32 widened and spelled as `pdf_syntax::write` spells a real) at its two call
sites with a unit test, and a fractional-origin `/Matrix` test in `tests/archive.rs`, red on HEAD
with `20.03 30.29`; `patch -p1 --dry-run` clean on HEAD and on the worktree.

**Measured.** The veraPDF annotation witnesses under `as-shown` (2b, 4, 4e, 4f; 17 converted
pages) against the annotation path, HEAD → both fixes: `fail-e` 160 → 0 (720 dpi 4 000 → 0),
`fail-f` 47 → 0 (477 → 1), `fail-g` 759 → 0 (66 896 → 0), 14 at 0 before and after. The pixel left
is no placement's (ADR 2090 §3): its stroke covers 0.949 of it and draws `(5, 6, 15)` under the
`/BBox` clip `Do` keeps, `(0, 0, 0)` unclipped as ADR 0165 lets the annotation draw it.

**Rows.** §12.5.5 stays `departed`; its note gains the `A`/`AA` sentence and its tests the new one.
§8.3.4 unchanged: `cm` composes as stated.
**Gates.** rustfmt per file clean; `cargo fmt --check` and `RUSTFLAGS=-D warnings clippy
--all-targets` for `pdf-model` exit 0; its nextest 2121 passed, 0 failed; `cargo test -p
conformance` 443 passed, 0 failed. Six arms of HEAD plus the three `pdf-model` files, by page digest
against `/home/AI/arms-1623/`: 0 of 968 (1x) and 0 of 965 (4x) moved, each exit 0 (calibration:
HEAD's cpu-1x and gpu-1x differ on 106); there `raster_golden` 2 passed, `fixed_documents
--ignored` 1 passed. With the patch: `pdf-transform` nextest 566 passed, 0 failed, clippy exit 0,
`archive_corpus --ignored` 3 passed. `tools/state.sh docs` exit 0.

**Unfinished.** The patch (slot 1's directory). Found, not fixed: `render-cpu` covers a stroke's
interior pixel fully without a clip and at the geometry's 0.949 under one that removes nothing. 6 700 s.
