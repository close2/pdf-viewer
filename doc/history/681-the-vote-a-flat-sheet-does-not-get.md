# 681 — The vote a flat sheet does not get

675 found that a voting reference whose raster is constant contributes nothing to a verdict, and
deliberately did not act on it because acting would move pages between four lists at once
(ADR 0499). This round acted. Parallel round, worktree `r681`, branch `round-681`. **No pixel
moves**; what moves is 29 verdicts of 1794. ADR 0513 has the argument and the tables.

## 675's claim, reproduced first

Exactly as stated. `bitmap-symbol-texthuffrefinecustom.pdf` page 1 prints *mean 13.12, worst tile
144.56, differing 5.15%, ssim 0.8990*; `magick identify -format '%k'` says `mupdf.png` and
`ghostscript.png` carry **one colour** apiece over 399 × 400 and `255 × (1 − 0.948543)` is 13.12.
Same on `issue11549_reduced.pdf` and `issue11740_reduced.pdf`.

Two things the reproduction added that were not in 675's account:

- **The logs are not the instrument.** On three of those four pages all three reference renderers
  are **silent** while two return blank paper and exit 0. A rule firing on a renderer that reported
  an error would have reached one page of four.
- **`%[fx:minima] %[fx:maxima]` is the wrong tell** and the trap said to use it. A solid *blue*
  sheet reads `min=0 max=1` exactly as a page with ink does, and one corpus reference panel is a
  solid blue sheet. `%k` — the count of distinct colours — is the tell, and the trap says so now.

## The discriminator, which took two attempts and the first one was measured

"Constant" is **not** the predicate, and the round found that out by building it and running the
corpus. A raster of one colour is a *failure* on a page with marks and a *reading* on a page that
is a flat sheet, and our own render cannot separate those without circularity in both directions.

**Attempt one** — *a flat raster abstains where any reference drew* — moved 32 pages and cost
**nine** agreements. Three of the nine were pages where a reference's marks were tiny enough to sit
inside the bound: a flat sheet is that raster as far as this instrument can measure, and refusing
it a vote bought nothing.

**Attempt two, which shipped** — *a flat raster abstains where a reference that drew marks fails to
agree with it*, by the same `Tolerance::accepts` that decides every other agreement here. It moved
29 pages and cost six agreements. The three pages the refinement saved are the measurement that
justifies it.

## What moved

| verdict | before | after |
|---|---|---|
| agrees | 908 | **902** |
| contradicted | 65 | **60** |
| ambiguous | 786 | **768** |
| not comparable | 13 | **42** |

`our geometry`, `reference geometry` and `no render` did not move. By direction: 19 pages
`ambiguous → not comparable`, 4 `contradicted → not comparable`, 6 `agrees → not comparable`, 1
`contradicted → ambiguous`. **Nothing moved toward a verdict that flatters us**, and on the further
21 pages where a reference abstained while two readings survived, not one verdict changed.

**The six lost agreements are the finding, not the price.** On each, two flat sheets outvoted a
renderer that drew *and our own raster was one of the flat ones* — `issue17333.pdf` page 1 where
`mupdf` and `hayro` place a mark three renderers including us do not; `issue18042.pdf` pages 1–4
where `mupdf` alone draws at 15.9 of 255 on a page this tree reports; and
`text_field_own_canvas_calc.pdf` page 3 where `ghostscript` and `hayro` place a light grey mark and
we do not. The gate said "PASS — agrees" on all six. None is diagnosed; `doc/todo/00`'s method is
owed on the three that are not `issue18042.pdf`.

## What the rule cannot reach, named rather than reached for

- `bitmap-symbol-context-reuse.pdf` — all three references flat (`mupdf` black, the other two
  white), so none of them drew marks, nothing abstains, and the page stays contradicted on two
  failures agreeing. The only evidence that it has marks is our own render.
- `recursiveCompositGlyf.pdf` — a flat sheet **is** the page (§9.3.6's "if the only glyphs shown
  have no outlines … no clipping shall occur"), and the only renderer with marks is `ghostscript`,
  which recovered a malformed composite glyph. The rule abstains the wrong two. Every refinement
  that would rescue it reads our own render, which is what would hide the six above.

`NOT_COMPARABLE_A_FLAT_SHEET_IS_THE_PAGE` holds the second by name.

## Where it landed

`pdfref::is_uniform` and `pdfref::consensus_abstentions`, `Triangulation::abstained`, seven new
`pdfref` tests that pin the rule *and its three refusals*, a census line on every oracle run, and
group surgery in `oracle.rs`: `CONTRADICTED_SHARED_JBIG2_DECODER` 7 → 4,
`CONTRADICTED_REFERENCES_DREW_NOTHING` 2 → 0 (kept for its argument),
`AMBIGUOUS_SHARED_JBIG2_DECODER` 19 → 1, `AMBIGUOUS_REFERENCE_DREW_NOTHING` 6 → 7, four new
`NOT_COMPARABLE_*` groups holding 29 pages. `doc/oracle-and-corpus.md` gains §3f and trap 9 gains
two paragraphs.

**No ledger row moves**: this changes an instrument, not an implementation. §9.3.6, the one clause
a new note cites, is `implemented` and untouched.

## Gates

The whole of `doc/todo/02` §2, green, on a quiet machine; §5's binaries rebuilt and installed,
which `tools/round.sh` had flagged as missing. Sweeps: `quoted` found one hit in a note this round
wrote — a superseded gate figure quoted in the past tense — and it was replaced with the figure the
gate prints today; `overtaken`, `pointers` and `quotations` show no hit this round created.
