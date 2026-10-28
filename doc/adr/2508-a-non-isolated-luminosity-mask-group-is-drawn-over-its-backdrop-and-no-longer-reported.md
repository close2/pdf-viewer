# 2508 — A non-isolated luminosity mask group is drawn over its /BC backdrop and is no longer reported as a blend with a backdrop it excludes

Status: implemented. Session 1837, batch one hundred and twenty-four. Context: ISO 32000-2 §11.6.5.1,
§11.4.3, §11.4.4, Table 145 (`/I`), §11.3.5; ADR 0237 (a group whose elements blend states its backdrop in the
list), ADR 0307 (the report's two conditions), `doc/history/1830` (the proposal to rank the SafeDocs survey's
residue by documents). Code: `crates/pdf-model/src/content/transparency/soft_mask.rs`
(`state_mask_backdrop`, `Interpreter::note_mask_group_structure`). Tests:
`crates/pdf-model/tests/soft_masks.rs`. Ledger: §11.6.5.1's row. ADR 2509 and Q703 are unused.

## 1. The ranking, and why this row

The residue is what `tools/safedocs survey --dir <population>` prints as `incomplete:` lines, and the
command that counts it by documents is `python3 -I -B tools/survey-residue.py <survey log>` (the round
worked in `scratchpad/r1837/`; the survey was of the 65 944 cached documents at HEAD, 431 incomplete). Its
head, by documents:

| documents | report | whose |
|---|---|---|
| 245 | a font program with no outline for any code the page shows | the files' and the standard's choice: ADR 0269 ranked this population, ADR 0270 took the six that a reader defect explained and moved it from 261 to 255, ADR 1411 read seven to their clauses and found each the file's or *neither one*, none this reader's (it counts the corpus's incomplete as "59: 55 the file's, 4 neither one, none this reader's"), ADR 0808 repaired the one reader defect it found. This round did not read the 245 again; 183 of them state one or two codes. |
| 53 | text operations skipped | the same fonts |
| 24 | a content stream undecodable under `FlateDecode` | the file's: the filter itself fails on the bytes |
| **22** | **`TransparencyGroup`: non-isolated, and an element blends with the backdrop it excludes** | **this reader's, below** |

The first three are not this tree's. The fourth is, and it is one construction in twenty-two documents.

## 2. What the report meant

`Interpreter::note_group_structure` reports a group that is not isolated (Table 145: `/I` "if false, they
shall be composited against the group's backdrop", default false) when a command in it blends and the group
was drawn on transparency. ADR 0237's construction states a *painted* group's backdrop in the display list, so
the condition is true of it only for the groups the construction refuses. A soft mask's group is evaluated into
a raster built on transparency by `pdf_render::SoftMask`, and `build_soft_mask` passed `isolated: true` for every
mask, so every mask group that was non-isolated and held a blend mode was reported: twenty-two documents, all of
them luminosity masks holding a `Multiply` (a `/BM Multiply` group or fill), over a white, black or grey `/BC`.

## 3. What the clauses say

§11.6.5.1, of a luminosity mask's group G: it "shall be composited with a fully opaque backdrop whose colour
is everywhere defined by the softmask dictionary's BC entry." Table 145's `/I`: "if false , they shall be
composited against the group's backdrop." So a non-isolated luminosity group's elements are composited onto
the `/BC` colour, and §11.3.5.2's `Multiply`, `B(cb, cs) = cb × cs`, reads that colour: over a black `/BC` it is
black and the mask is 0; drawn on transparency it is the source. §11.4.4 composites the group's result with its
backdrop afterwards, so for a non-knockout group painted Normal at full opacity the group is its elements drawn
onto the backdrop. Outside the `/BBox` §11.6.5.1 gives the mask the backdrop's luminosity, which the raster's
transparent pixels already say (`SoftMask::value` composites onto `/BC` after the fact).

The alpha subtype is different: §11.6.5.1 has the group "evaluated to compute a group alpha only", and a blend
function moves a colour and never an alpha, so there is no pixel for a report about a blend to be about.

## 4. The change

`state_mask_backdrop` puts a Normal fill of the `/BC` colour, one unit larger than the `/BBox` and under the
box's own clip, first in the mask's list (and in the black half of a four-component mask's pair), when the group
is luminosity, non-isolated, non-knockout, states a `/BBox` and a command in it blends. The report's `isolated`
argument is then `!(backdrop_stated || subtype is Alpha)`. A group with no `/BBox` has no extent to fill and
keeps the report; a knockout group keeps it (§11.4.6's NOTE 6, as before). The call moved into
`note_mask_group_structure` because `build_soft_mask` passed clippy's hundred lines.

## 5. Evidence

- The twenty-two documents (`tools/safedocs survey` of that population, before and after, `RAYON_NUM_THREADS=4`
  bare, 22 documents in about four seconds): **22 incomplete before, 0 after**, all 22 now `complete`.
- Page one of each at scale 1, HEAD's export against the fixed one: ten are identical and twelve differ, none
  by more than three levels of 255. The counts, in the order of the survey's listing: 9, 24, 1, 31, 9, 4, 2, 7, 2
  (`3867162.pdf`), 5, 60 and 887 pixels (`7065071.pdf`, of which 111 differ by two or more levels and the
  rest by one). The signs are mixed and the pixels scattered over the multiplied
  shapes, which is what two routes to one number look like: HEAD composites an 8-bit raster onto `/BC` after
  the fact, the fixed list draws onto `/BC` and takes the luminosity of that. They are this change's pixels and
  the figures are theirs; no pixel moves by more than three.
- None of the twenty-two is among the tracked corpus's pages: against HEAD's six arms, by page, **0 of 968 pages
  (1x) and 0 of 965 (4x) moved** on any of the three lanes, and `raster_golden` reports `held 974, moved 0`, so
  no line of `raster_golden.tsv` changes. The other tracked-corpus gates (`corpus`, `text_extraction`, `oracle`)
  pass on the fixed export.
- Tests, each planted and reverted: a black `/BC` with a `Multiply` element gives mask 0 and no report, a
  white one gives 0.5, outside the box is 0; an alpha mask with a `Multiply` is not reported and keeps its alpha;
  a group without `/BBox` keeps its report. With the fix reverted, the first two fail on the report assertion.

## 6. Cost

A `Multiply` mask group draws one more fill (its list grows by one command per half). The fill is the box, and
the mask raster was already the box. Masks with Normal elements only are the list they were.
