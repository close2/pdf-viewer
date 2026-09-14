# 1050 — A clip encloses the line its fill paints

Date: 2026-09-14. Ledger slot of `batch-1050-1055`, §10.7.4, ADR 1064. Adds `pdf_render::clip_region`, used
by all three backends' clip construction, plus `render-cpu/tests/zero_area_clip.rs`, a cross-backend scene
and three unit tests.

## The defect, which ADR 1060 named and left

§10.7.4: "For clipping, the clipping region consists of the set of pixels that would be included by a fill
operation", and its EXAMPLE says what that fill includes — "A zero-width or zero-height rectangle paints a
line 1 pixel wide". The fill has built that line since session 186; no backend asked it for a clip. At one,
two and three-and-a-half pixels per unit `5 20.5 30 0 re f` paints 30, 60 and 106 device pixels; `5 20.5 30
0 re W n` over a full-page fill admitted **0, 0 and 0**, and admits 30, 60 and 106 now. **Calibrated (trap
13)**: with `clip_region` planted back to `None` three of `zero_area_clip.rs`'s four tests fail at every
scale and the fourth, `a_clip_that_is_a_single_point_still_admits_nothing` — ADR 1060's control — passes.

## What was decided, and the one surprise

ADR 1064 argues it. A wholly collapsed path becomes its marks under the **non-zero** rule whatever the
operator asked; a **mixed** path is where it stops, because that region is a union of two fills under two
rules and no clip vocabulary states a union (`vello`, raster and `tiny_skia::Mask` each take one path and
one rule), so a mark joins it only outside the hull of the area-enclosing subpaths, where appending it *is*
the union — departure (4). The surprise: **a substituted region is scan-converted, not measured in closed
form**, because ADR 0476's exact area sees the ten-thousandth of a pixel the inverse of the placement leaves
behind and `tiny-skia`'s four sample rows do not — at scale 3.5 the closed form admitted **212** pixels
where the fill paints 106.

## Gates, and a corpus with nothing in it

`raster_golden` held 974, **moved 0**, unheld 0, left 0 (exit 0) — a control rather than a witness, and the
census saying so is calibrated (trap 13): at the top of `clip_region` it counts **38 267** calls over the
965 pages drawn and at the substitution **0**, so no corpus first page states a clipping path collapsed
along one axis — and a corpus holding none of a construction says nothing about the clause requiring it
(trap 8). `render-raster --test corpus` 957 compared: 930 agree, 21 differ, 6 refused, 17 not comparable
(exit 0). `oracle` 991 agrees, 62 contradicted, 835 ambiguous, 3 passed (exit 0). `pdf-model --test corpus`
974 documents, 61 incomplete, 0 slow (exit 0). `cargo nextest run --workspace` 4723 passed, 0 failed (exit
0); `cargo test -p conformance` 7 passed and 251 in its ledger suite (exit 0); `cargo fmt` and
`RUSTFLAGS="-D warnings" cargo clippy` clean on every crate this round touched. Still red on **siblings'
files only**: clippy on `pdf-signature/src/revocation.rs` (ten lints) and the `fuzz/` clippy line on
`trust::validate`'s arity in `fuzz_targets/x509.rs`.
