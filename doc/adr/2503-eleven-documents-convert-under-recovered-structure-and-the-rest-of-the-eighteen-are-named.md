# 2503 — Eleven of the eighteen convert under `recovered-structure`, and the seven that do not are named

Status: implemented. Session 1834, the R01 slot of batch one hundred and twenty-four. The measured
outcome of ADR 2502. Context: ADR 2490's row R01 (23 documents declined as a source, the reader
draws a page of 18, 5 draw none) and its upper bound.

## What was measured

The census command of `doc/pdfa-blocking-issues.md` section 1, in an export of HEAD and in an export
of HEAD plus this round's hunks (against HEAD's six arms nothing else moves: no render, raster or
interpreter source changed), with a copy of the CJK pack named, then without one. The census lines,
pack named, `converted, refused, declined as a source`:

| census line | HEAD | with this round |
|---|---|---|
| pdf.js `as-if-printed` 2b | 804, 149, 10 | 805, 149, 9 |
| corpora `as-if-printed` 2b | 351, 116, 14 | 361, 117, 3 |
| pdf.js `as-if-printed` 4 | 807, 148, 10 | 808, 148, 9 |
| corpora `as-if-printed` 4 | 353, 121, 13 | 363, 122, 2 |
| pdf.js `as-shown` 2b | 768, 185, 10 | 769, 185, 9 |
| corpora `as-shown` 2b | 304, 164, 13 | 314, 165, 2 |
| pdf.js `as-shown` 4 | 772, 183, 10 | 773, 183, 9 |
| corpora `as-shown` 4 | 304, 170, 13 | 314, 171, 2 |

`rank.py` over the two logs: documents refused or declined at every target under `as-if-printed`
262 to 251 and at one target at least 319 to 308; under `as-shown` 351 to 340 and 397 to 386. Eleven
documents leave the population at every line, and R01's remedy row falls from 18 documents to 6
(the six of ADR 2502's "what remains"); the five of section 7.2 are unchanged.

Without the pack the same eight lines move by the same amounts (pdf.js `as-if-printed` 793 to 794
converted at 2b and 796 to 797 at part 4, 160 and 159 refused unchanged; corpora 351 to 361 at 2b and
353 to 363 at part 4; `as-shown` 768 to 769 and 304 to 314), so the pack is no part of this remedy.

## The eighteen, one by one

- **Converted at 2b and at part 4 under both shipped profiles, veraPDF `isCompliant` at all four
  runs of each** (11): the five `T02-01_*` documents, the two `T02-02_*` documents, the three
  `T04_*` documents and `issue9418`. Page 1 of each renders from the output as from the source (the
  render verb's PNG, byte for byte). Every other R01 file has at most one page object.
- **Converted structure, refused at the file header** (1): `UnknownFilter-xrefstm` states `%PDF-3.5`,
  which the `file-structure/file-header` row refuses at both parts (a different row, R06b's). It
  leaves the population "declined as a source" and enters "refused", which is why the refused figure
  of the corpora lines rises by one.
- **Refused by name, ADR 2502 decision 6** (6): `issue19484_1` and `issue19484_2` (encrypted),
  `poppler-395-0-fuzzed` (signed), `issue9105_other` (page 1 is not an indirect object),
  `poppler-742-0-fuzzed` (a page's dictionary does not parse whole) and **`027613`**. That file is
  linearized: its parameter dictionary states `/N 429` (§F.3.3, Table F.1) and its bytes hold 429
  page objects, but this reader reaches only the first-page cross-reference section and draws one
  page. A first version of the rebuild wrote that one page as a conforming PDF/A and said it had
  drawn one; the stated count is now compared (ADR 2502 decision 7) and the file is refused. That
  the reader reaches one page of a linearized file is a robustness-lane candidate and is not
  repaired here.

veraPDF was run on every output of the eleven at 2b and at part 4 under `as-if-printed` and
`as-shown`, 44 outputs, 44 compliant. Both output shapes were among them: the producer's catalog kept
and a catalog written afresh, each over pages found by their own declaration.

## What this does not claim

- The upper bound of ADR 2490's row was 18 and this removes 11 of them from the population; the 5
  that were never going to draw (section 7.2) and the 7 above are what remains of the 23.
- The rebuilt tree is flat. A document with a producer's tree that the checks decline for one node
  loses the tree's other nodes: its pages are the producer's, their grouping is not, and the
  attributes the grouping carried are copied down (ADR 2502 decision 2).
- A document whose pages the reader draws in an order its producer's tree did not state gets the
  reader's order; the standard states no order for a recovery that does not exist, and the report
  says that the pages were found by their own declaration, in ascending object number.
