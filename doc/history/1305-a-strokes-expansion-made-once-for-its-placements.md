# 1305 — A stroke's expansion is made once for its placements

Date: 2026-10-01. Branch `batch-1303-1308`, shared worktree. ADR:
[1445](../adr/1445-a-strokes-expansion-is-made-once-for-its-placements-and-tiling-across-subpaths-is-not-traded-for-the-fills-question.md).

## What was asked, and what the measurement said

The performance slot: the Type 3 page (stroked glyphs in a tiling cell) re-expanded and re-tiled
each glyph stroke at every placement, and tiled across subpaths where the fill's question could
have answered. A probe export found 144 expansions of 24 shapes on the turn, all through
`encode/parallel.rs`; `stroke_pieces` was 66% of the process's instructions.

## What was built

- `encode/expansion.rs`: every stroke is flattened and stroked under its linear part and then
  translated, on both expansion paths; a shape the scene places twice or more shares one
  expansion per frame through a slot, bounded by a sixteenth of the frame budget.
- `tests/encode_threads.rs`: a stroke placed thirty times draws what it draws placed alone, at
  two budgets and five thread counts; a planted per-placement cache fails it.

## What was measured and dropped

Tiling across subpaths only past the fill's bound: +38% instructions on the Type 3 page, +13% on
`issue14415.pdf` — each glyph must be tiled alone before the fill can be asked.

## What it cost, and what it did not keep

Type 3 turn 11.48 → 5.98 ms, step 11.65 → 6.01 (pinned); other pages within noise. Not
byte-identical, and no translation cache can be: 52 pages move at 1×, 80 at 4×, gate counts and
the 1-vs-N check unchanged. Where pieces tile, every moved pixel checked moved toward its exact
area; where the fill's question is past its bound, both arms are wrong and move both ways.

I did not open `crates/render-cpu/`.
