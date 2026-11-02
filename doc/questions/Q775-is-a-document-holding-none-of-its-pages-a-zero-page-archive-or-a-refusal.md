# Q775 — Is a document holding none of the pages it states archived with no page, or refused by name?

Source: round 1899 (slot 9 of batch 130), reading `T02-03_006` against A727 (ADR 2596).

## The question

`T02-03_006`'s root page tree node states `/Count 1` and `/Kids [2 0 R]`, and the cross-reference
section places object 2 at an offset where object 3 sits, so the one kid names no dictionary and the
file holds no page. Every profile refuses it today (exit 4), the converter's sentence naming Table 30's
`/Kids` entry that names no dictionary and saying this reader draws no page of it (ADR 2502).

ISO 32000-2 §7.7.3.2's Table 30 makes `/Count` the number of leaf nodes descending from the node, and
neither it nor the Arlington model's `PageTreeNodeRoot` (`Count >= 0`) forbids zero; no sentence of
ISO 32000-2, ISO 19005-2 or ISO 19005-4 this round found requires a document to hold a page. So a
document with no page is not itself non-conforming: what the file breaks is the count, one page stated
and none held, which is exactly A727's `page-not-held`.

Under a profile taking `page-not-held` (and `recovered-structure`, which the rebuild needs), the pages
carried would be zero. Two readings:

1. **A zero-page archive is written**: a catalog, a root node with `/Kids []` and `/Count 0`, and —
   under `append-statement` — the statement page as the one page, naming the one page not held with
   the two counts. Under `discard` the output has no page at all.
2. **It is refused by name**: a conversion carrying none of the pages it was asked to archive has
   archived nothing a reader saw, and the refusal names `page-not-held` with the counts, one sentence
   for the checker and the converter.

## What is built, and what is not

Nothing for this question. The converter's refusal names the page not held today; the checker's row
`file-structure/page-tree-kids-name-dictionaries` fails the same kid.

## Recommendation

Reading 1 under `append-statement` only (the statement page is then the archive, and it says the
document held nothing), reading 2 under `discard`, where the output would be a file showing nothing
and saying nothing. The owner's 2026-10-27 stance — archive every PDF a company can see, a stated loss
rather than a refusal — points to reading 1 wherever a statement is written.
