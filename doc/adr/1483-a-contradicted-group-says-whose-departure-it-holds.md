# 1483 — A contradicted group says whose departure it holds, and the ranking names the next page by it

Session 1324. Status: **accepted**.
Context: ISO 32000-2 §10.3.2, §10.4.2.1, §10.7.1, §10.7.4, §7.4.7, §7.4.8, §9.5, §12.5.3, §12.5.4,
§11.5.3; ADRs 0499, 0805, 1416, 1435, 1437, 1441.
Rows: none moved. No pixel of this tree moved.

## 1. The instruction the ranking ended on could not be carried out from its output

`rank_the_contradicted_by_the_bound` ended on "the next page to take is the highest row whose note
names a departure of ours rather than a reference's", and nothing in the run said which notes those
were. Principle 5 gives a disagreement three outcomes with three consequences: we misread, they
misread, or the clause leaves it open. That is a property of the group, not of a run. So it is
stated once, in `oracle.rs`'s `WHOSE_DEPARTURE`, beside the clause the note decides it by. Each
ranked row prints it, and `name_the_next_departure_of_ours` prints the pool's split and the
highest-ranked page held as a departure of ours. `every_held_page_names_whose_departure_it_is`
(not ignored, reads no corpus) holds the table to the groups both ways: a group holding a page has
a row, a row names a group holding a page, and the group's own note cites the row's clause.

## 2. The verdicts, on the run of 2026-10-01 (1024 agree / 47 contradicted / 836 ambiguous, unchanged)

| ranked | page(s) | group | whose | deciding sentence |
|---|---|---|---|---|
| 127.75x | `xobject-image.pdf` | `ON_A_PAGE_WE_REPORT` | choice | §7.4.8: DCTDecode may take its parameters from the codestream; the file contradicts itself and no clause states a recovery |
| 37.63x | `issue21346.pdf` | `LUMINOSITY_OF_A_CIE_BASED_MASK` | references' | §11.5.3's CIE-based branch |
| 33.47x … | three `bitmap-*` | `SHARED_JBIG2_DECODER` | references' | §7.4.7: JBIG2 "defines decoder behaviour" |
| 29.19x … | four pages | `DEVICE_CMYK_CONVERSION` | choice | §10.4.2.1 ranks §10.3 first; §10.3.2's NOTE licenses the source assumption |
| 4.22x … | three links | `LINK_BORDER` | references' | §12.5.4 "shall be drawn completely inside"; Table 167's Print flag "shall be ignored" without an appearance |
| 3.10x | `issue15716.pdf` | `SUBSTITUTED_FONT` | choice | §9.5 NOTE 5 |

**`DEVICE_CMYK_CONVERSION` is right by the standard's order.** This tree is on §10.3's route, with
`CMYK_CORNERS` as the CIE-based source assumption §10.3.2's NOTE permits. So are the references,
with a SWOP press. §10.4.2.5's formula would draw `transparent.pdf`'s bottle black, and we draw
(28, 32, 40). The defect the brief named, our formula against their ICC route, is not present.

**ADR 1441 did not reach `issue15716.pdf`.** `/ZapfDingbats` is one of the standard 14, which
`pdf_font::standard` answers before any descriptor ranking runs. The page is still contradicted
at 3.10x.

Of the 47 pages: **2 are departures of ours, 12 are the references', 33 are choices.** The two of
ours are §10.7.4 row's departure (1), a partly covered pixel painted partly. One is at an image's
edge (`issue4436r.pdf`), the other at a clip's edge (`issue7891_bc1.pdf`, ranked next at 1.11x).
That row records both. Neither page asks for anything short of revisiting that departure on its
own row.

## 3. What would revisit it

A group whose note changes its reading must change its `WHOSE_DEPARTURE` row in the same edit.
The test catches a clause the note no longer cites, but it cannot catch a verdict that flipped.
