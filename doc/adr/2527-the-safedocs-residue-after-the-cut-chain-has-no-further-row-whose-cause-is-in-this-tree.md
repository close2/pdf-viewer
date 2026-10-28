# 2527 — The SafeDocs residue, read after ADR 2526: no further row has its cause in this tree

Status: implemented. Session 1846, batch one hundred and twenty-five, the parsers slot, second half. The
decision is a reading and builds nothing. Context: ADR 2508 section 1 (the residue ranked by documents),
ADR 2526 (the cut chain), ADR 0836 (a Flate stream whose check value disagrees), ADR 0810 and ADR 0828
(`MAX_TILE_COPIES`), ISO 32000-2 §9.7.5.2, §8.3.4, §7.5.5.

## 1. The survey

`safedocs survey` over the whole cache, 65 944 documents, with this round's change built in an export (the
confined sandbox worker built beside it), under the heavy-walk lock as a long run: **173 unopenable, 47 locked,
4 encrypted beyond us, 41 pageless, 409 incomplete, 8 slow**, exit 0. The batch's previous survey (record 1837,
before its own fix) had 431 incomplete; the 22 it took are the difference, and this change moves none of the
remaining rows because a document whose chain is cut reported nothing before it either: it drew fewer pages
and said so nowhere (ADR 2526 section 5 counts them with the page counts).

## 2. The rows, each read to what the file or the standard says

`python3 -I -B tools/survey-residue.py` on the log, the head by documents, with what was opened for each:

| documents | row | whose, and what was read |
|---|---|---|
| 245, 53 | a font program with no outline for any code shown, and the text operations skipped with it | the file's and the standard's choice (ADR 2508 section 1 names ADRs 0269, 0270, 1411) |
| 24 | a content stream undecodable under `FlateDecode` | the file's: the filter fails on the bytes (ADR 2508 section 1) |
| 18, 14 | a Flate stream, content or font program, whose RFC 1950 check value disagrees | the file's: RFC 1950 makes the Adler-32 the test of the bytes, §7.4.4.1 makes the RFC normative, and `whole_program`'s documentation (ADR 0836) records what admitting such a font drew |
| 13 | a Type 0 font over a descendant with no embedded program and `/Identity-H` | §9.7.5.2: "The Identity-H and Identity-V CMaps shall not be used with a non-embedded font." |
| 11 and the `Operator`, `OperandShortfall` rows | `BT` without `ET`, operators glued to numbers, `TJ` inside an array | damage in the content stream: §9.4.1 and §7.8.2 state the grammar the bytes break |
| 9 | a command under a noninvertible matrix | the file's: §8.3.4's sentence is quoted in `Unsupported::NoninvertibleMatrix`, "[u]se of a noninvertible matrix when painting graphics objects can result in unpredictable behaviour", and the report counts only what is still refused |
| 7 | a content stream unreachable | the file's: the seven documents have no `startxref` and no `N G obj` header for the object (truncated downloads) |
| 7 | `MAX_TILE_COPIES` | a bound chosen and measured (ADRs 0810, 0828): its two witnesses' tilings are worth 1.087 of 255 |
| 7, 6 | no character code maps to a glyph; no font resource of the name the page uses | the first is the font's; of the second's four documents three (`1161462`, `3006847`, `6204793`) never write the name in any dictionary |

The unopenable 173 are 168 files that are no PDF (140 begin with an HTML document, 28 are error text or
XML, none holds `%PDF-` or an object header) and 5 with no object header; the 45 documents with no catalog
(41 pageless) hold no `/Type /Catalog` anywhere, 24 of them no `startxref`, and the three with an object
stream hold one the file's own bytes do not decode.

## 3. What was not decided

One document, `2145704`, reports `no /Font resource named /` where the name printed is empty; its content
streams and resources write only ordinary names (`/F0` to `/F4`) in a qpdf-normalised copy, so the cause is
not read here, and the survey line does not say which page. It is one document and is a candidate for the
next parsers slot.

## 4. The finish line

The residue's row count is not lowered by this half: every row with more than three documents has its cause
in the file, in the standard, or in a bound that was measured. The rows' counts are the log's; the log is a
session artefact and `python3 -I -B tools/survey-residue.py <survey log>` reproduces the table from any rerun.
