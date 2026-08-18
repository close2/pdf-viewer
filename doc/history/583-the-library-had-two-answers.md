# 583 — The library had two answers, and the mask was blamed for five hundred sessions

`render-cpu` asks `tiny-skia` for its high-precision raster pipeline, which makes §11.3.6's weighted
average exact at every one of the 256 values a mask can take, empties the contradicted group whose
*name* had blamed the eight-bit mask for a level it does not produce, and costs nothing measurable.

Date: 2026-08-18.
Argued by: [ADR 0418](../adr/0418-the-library-had-two-answers-and-the-mask-was-blamed.md).

Touched: `crates/render-cpu/src/{lib,shading}.rs`, `crates/render-cpu/tests/soft_mask.rs`,
`crates/pdf-model/tests/{oracle,transparency_groups}.rs`,
`crates/render-quorra/tests/sub_pixel_coverage.rs`, `doc/conformance/ledger.toml`,
`doc/HANDOVER.md`, `doc/oracle-and-corpus.md`, `doc/todo/{00,11}.md`.

## The demand half

The group was chosen by trap 1's own tell rather than by a ranking. `CONTRADICTED_MASK_QUANTISATION`
held one page and two claims: that the verdict comes from "`mupdf` and `ghostscript` … within one
level of *each other*", which is `CONTRADICTED_TIGHT_CONSENSUS`'s mechanism in that group's own
words, and that the level itself "comes from the mask being quantised", which no line under it
measured and which the group's whole name rests on.

The second is refuted by its own arithmetic. The mask is one byte, the byte is
`round(0.75 × 255) = 191`, and 191 through §11.3.6's weighted average against the destination the
grey fill actually wrote is `(223, 99, 80)` — the closed form the note already quotes, and `hayro`'s
pixel byte for byte. An eight-bit mask predicts the answer we were *not* producing. Ours was
`tiny-skia`'s **low-precision** pipeline, whose `div255(v) = (v + 255) >> 8` is an upper bound on a
division by 255 rather than its rounding, spent twice per pixel and biased the same way both times;
computed by hand out of the library's source it gives `(223, 100, 81)`, which is this page.

Swept over all 256 mask values the high-precision pipeline is exact at every one and the
low-precision one departs by up to two levels. `HIGH_PRECISION_PIPELINE` asks for the first, a new
test sweeps all 256 with no slack, and flipping the constant fails it.

Priced, because a correctness fix still is: ISO 32000-2 page 101 **−2.1%** of rasterisation
instructions, `alphatrans.pdf` **−1.4%**, `firefox_logo.pdf` **+0.6%**.

Three other instruments moved, and each says more than the page. A knockout test had been asserting
that two channels with *identical* arithmetic land a level apart. `render-quorra`'s turned ladder
had been reading −0.2% at its thinnest 45° rung for a construction that is 16.8% short, because ADR
0268's substitute carries a rule's given-up width in the paint's alpha and the bias was therefore
largest exactly where the mark is thinnest. And `doc/todo/00`'s step 7, run before and after over
all 786 ambiguous pages, moves one page by more than 0.36 — `issue12295.pdf`, which states 65 859
strokes 0.1366 of a device pixel wide, so it is the ladder's finding at page scale rather than a new
one.

## The spec half

§11.3.6, whose row said "the formula is tiny-skia's and Vello's rather than this tree's, and what is
implemented here is the choice of what they composite onto". That sentence named one library and two
answers. The row keeps its `partial` — choosing between a library's two evaluators is not
implementing the formula — and now records which of them is chosen, why the other is not the clause,
and what it cost. `spec-errata emit` over clause 11 before writing: nothing touches §11.3.6.

## What is still owed

`issue12295.pdf` draws its ECG traces as a ghost where all four references draw them dark, and that
gap is `doc/todo/11`'s standing item about a mark whose ink is under one of an eight-bit raster's
levels — not this round's, and larger than it. `TURNED_TOLERANCE`'s margin over the defect it exists
for is 1.7× rather than 2×.
