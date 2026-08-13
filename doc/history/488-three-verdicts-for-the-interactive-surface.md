# 488 — Three verdicts for the interactive surface, each its own shape

**Finding.** The design round `doc/todo/05` owed, and its yield is a measurement as much as a
decision. The tolerance for a selection-geometry gate was measured the way the raster
`Tolerance` was — reference against reference before it judges us — over matched word pairs
from `pdftotext -bbox` and `mutool draw -F stext` on the corpus's first pages, and the spread
splits by axis: two extractors sharing no code land a word's *horizontal* edges within a
ten-thousandth of a point of each other at the median, while the *vertical* extent of a word
box is each library's own line-height convention (their height ratio is a quarter apart at the
median) and the vertical centre is the comparable quantity. The first, naïve run reproduced
trap 3 in the text domain three mechanisms deep before any instrument existed to be misled:
`pdftotext -bbox` answers in the MediaBox unless `-cropbox` is passed where mutool answers in
the CropBox, mupdf scales stext coordinates by Table 31's `/UserUnit` where poppler does not,
and mupdf applies `/Rotate` where `pdftotext -bbox` does not. And the references emit matched
words in the same *order* on only about two thirds of documents — each answers the order
question its own way — which settles reading order out of the geometry verdict on measurement
rather than on argument. The design that came out is three instruments with three verdict
shapes, stated per instrument rather than forced into one: **bounded** for selection geometry
(horizontal edges tight, vertical centres relative to word height, the drag composed end-to-end
in the headless harness from the reference's own box), **exact** for the save path (the prefix
property, our own readback, and the references reading the saved file — with the existing
raster oracle reused whole over a saved sample), and **a ratchet, honestly named one**, for the
accessibility tree, whose first count is the one that makes `doc/todo/31`'s 8192-element
truncation corpus-visible. ADR 0323 carries the design, the numbers, the denominators with
their refusal arithmetic, where each instrument runs, and the build order.

**Date.** 2026-08-14.
**ADR.** [0323](../adr/0323-three-verdicts-for-the-interactive-surface.md).
**Touched.** `doc/adr/0323-three-verdicts-for-the-interactive-surface.md` (new),
`doc/todo/05-an-instrument-for-the-interactive-surface.md` (design recorded settled; three
build items remain, one round each), `doc/todo/README.md` (the index line's claim), this file.
No code, no test, no ledger row: the round built nothing and measured with throwaway scripts,
uncommitted by design — the instrument rounds re-derive the bounds with committed code before
anything gates. The gate sequence §2 names for a docs-only change was run to prove the tree
stands: `fmt`, `clippy`, `nextest`, the doctests and the conformance gate, all green, nothing
moved.
