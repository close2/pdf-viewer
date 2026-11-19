# 3149 — A walk whose verdict an image decides refuses a run the wall clock refused a decode in

Session 2112. Status: **accepted.** Context: ADR 3133 (a decode is budgeted in processor time, and
`pdf_sandbox::wall_clock_refusals()` counts the machine's refusals), ADR 1016 (the golden),
`doc/todo/02-every-round.md` §2's tier 2 and tier 3 blocks.

## 1. The question

ADR 3133 made `raster_golden` refuse to compare or write a run that counted a wall-clock refusal,
and said an oracle comparison should. Which other walks draw a verdict an undrawn image can move?

## 2. The census

Every `--ignored` walk the two blocks name was read for where it decodes and what it judges. A walk
judges the machine where an image decoded in the test's own process feeds a verdict:

| walk | what an undrawn image moves |
|---|---|
| `pdf-model` `corpus` | `is_complete()`'s incomplete count and its ratchets |
| `pdf-model` `oracle` | our page against the references' consensus |
| `pdf-model` `fixed_documents` | each fixed document's ink band and reports |
| `pdf-model` `text_extraction` | the incomplete count beside the score |
| `pdf-model` `drawn_while_read` (read in pieces) | two interpretations compared; a stall in one parts them |
| `render-raster` `corpus` | which pages differ, are refused or are not comparable |
| `viewer-core` `selection_census`, `accessibility_census` | word counts; elements placed by an image |
| `pdf-transform` `split`, `merge`, `pages`, `optimize` corpus | source and output drawn separately; a stall in one is `differ` or `undrawn` |
| `pdf-transform` `gate` | page one against the oracle drawn in the test |

Not affected: walks that compare one list drawn two ways (`drawn_while_read`'s drawn-while-read
half, `viewer-host`'s `drawn_ahead_in_strips`), walks that decode no image (`actions`,
`save_round_trip`, `writer_corpus`, `archive_corpus`, `pdf-archive`'s two), and walks whose decodes
are in another process (`foreign_corpus`'s poppler and mupdf, `awkward_classes`'s confined viewer,
whose only verdict is a death).

## 3. The decision

Each affected walk reads `pdf_model::image::wall_clock_refusals()` — `pdf-sandbox`'s count,
offered beside `sandboxed_decoder()` so that no walk takes a new dependency — **after its figures
are printed and before anything is asserted of them**, and refuses the run with one sentence. Refusing
rather than separating documents: the walks run documents in parallel in one process, so a count is
not attributable to a document, and a run that measured the load is run again rather than read.
The `oracle`'s guard stands before its `PDFVIEWER_ORACLE_ONLY` branch, because a filtered page is
read by a person as a verdict too.

## 4. The proof

With one refusal planted in the count and each walk cut to its first three documents (the oracle
and `render-raster`'s by their `ONLY` variables), every affected walk ended at its guard with "image
decodes in this run were refused by the wall clock", and with the plant reversed none did
(`doc/history/2112-*.md`).

## 5. The cost, and what is left

A loaded run that loses one decode to the ceiling is lost whole, as the golden's is. `gate`'s
program draws its two hundred pages in a child, whose count this process cannot read; only the
oracle page drawn in the test is guarded, and the child's half needs the program to say its own count.
