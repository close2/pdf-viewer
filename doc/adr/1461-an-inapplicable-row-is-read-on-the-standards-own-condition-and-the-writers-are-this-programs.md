# 1461 — An `inapplicable` row is read on the standard's own condition, and the writers are this program's

Session 1313. Status: **accepted**.
Context: `CLAUDE.md` principle 5 ("'The specification defines nothing here' is itself a claim about
the specification, and it decays"); `doc/PLAN.md` §5a's `inapplicable` and `writer-side`; ADRs 0204,
0205, 1172, 1437; ISO 32000-2 §3.15, §8.6.6.6, §8.10.4.2, §9.7.2, §9.7.3, §11.3.2, §11.4.2,
§11.7.5.1, §14.3.1, §14.5, §14.10, §14.12, §14.13.5, §14.13.10, Annexes E, L and Q.

## 1. The reading

Each of the 65 `inapplicable` rows was read against its clause in `doc/md/` and the titles around
it, and placed in one of four classes:

| class | meaning | rows |
|---|---|---|
| (a) | inapplicable on the standard's own condition, or the clause states no requirement of its own | 48 |
| (b) | inapplicable by one of `CLAUDE.md`'s closed exclusions | 0 |
| (c) | the condition was this tree's claim, and the clause decides a pixel, a reader's act or a written file | 8 |
| (d) | the condition is stated, and false of this program today | 9 |

(a) splits into conditions the clause states — a marking device (§10.6, §10.8), a PostScript printer
(§14.2), a press or a production workflow (§14.11, §14.12.1, §14.12.4.2), a capturing application
and deprecation (§14.10), a processor that lays content out (§14.8.3, §14.8.5.4), a permission
declined (§14.11.2.2, and §14.10.6's `/PZ`, ADR 1172) — and clauses whose text states no requirement
(§8.4.3.1, §8.6.6.1, §11.6.1, §11.7.1, §12.7.1, §14.11.1). §3.15 is the definition the deprecated
families rest on: "a part of ISO 32000 that should not be written into a PDF 2.0 document, and
should be ignored by a PDF processor".

## 2. The rows moved

| row | to | the sentence that moved it |
|---|---|---|
| Q, Q.1–Q.5 | `implemented` | Q.1: "This annex describes the method that PDF processor shall use to determine if a given page contains any graphical elements whose associated graphic state contains transparency…" — and `pdf_archive::survey` asks the question for ISO 19005-2 and -4 |
| §8.10.4.2 | `implemented` | "When printing a page containing reference XObjects, a PDF processor may emit any of the following items" — this program prints (ADRs 1179, 1180), through the same interpreter and `reference::Supply` |
| §14.5 | `implemented` | data dictionaries' "modification dates can be compared with … the ModDate entry of the document information dictionary" — the archive keeps `/ModDate` beside a `/PieceInfo` for that reason |
| §14.12 | `implemented` | aggregate: its writer rows are now this program's (section 3) |
| §9.7.2 | `implemented` | "The order of the glyphs in the character collection shall determine the CID number for each glyph" |
| §9.7.3 | `implemented` | "This value shall not be used in determining compatibility between character collections" — `composite::collection_names` |
| §11.3.2 | `implemented` | "the convention that 0 ÷ 0 = 0 shall also be adopted" |
| §11.4.2 | `implemented` | "The revised formulas for a simple n -element stack (not including any groups) shall be" |
| §11.7.5.1 | `implemented` | the two categories, "applied only when the final colour at a given point on the page is known" and "whenever colours are converted" |
| §14.3.1 | `partial` | "Except for the CreationDate and ModDate entries, the use of the document information dictionary for document metadata is deprecated in PDF 2.0" — `update` and `merge` still write the text entries there |
| §8.6.6.6, §14.13.10 | `implemented` | examples, run as fixtures (habit 39) |

## 3. The writer-side and out-of-scope pass

`writer-side` is not permanent (`doc/PLAN.md` §5a), and four of its six rows are this program's:

- **§14.12.2, §14.12.3 → `implemented`.** `split` and `merge` (and so `pages`) carried each page's
  `/DPart` while leaving `/DPartRoot` behind, which Table 31 forbids — "( Required, if this page is
  within the range of a DPart, not permitted otherwise; PDF 2.0 )". Both now drop the back-pointer
  with the tree. `update` keeps the document's hierarchy and so refuses by name an insertion and a
  deletion of a range's `/Start` or `/End`; a carried page leaves its `/DPart` behind.
- **Annex L → `partial`.** The archive's appended `Part` states no `/NS` and is outside the annex's
  own namespace condition; `merge` writes each source's top-level elements under one root, and two
  PDF 2.0 `Document`s there break Table L.2's first row. Priced in `doc/todo/65`.
- **§6.2 → `partial`**, as the aggregate of the writer obligations, while Annex L and §14.3.1 are.
- §7.6.7 and E.2 stay `writer-side`: no writer emits a wrapper or a developer extension.

The 94 `out-of-scope` rows each carry an `exclusion` the checker holds to the closed list; every
note read names its exclusion or is a clause 13 row. §12.6.4.17 now names RFC 0008 and the open
Q193, under which the row stays as it is.

## 4. Findings the reading produced, built

- **Web capture crossed a merge half-built.** `merge` carried §14.10.3.1's `/IDS` and `/URLS`,
  renaming a colliding digital identifier, while leaving `/SpiderInfo` behind — §14.10.1 records the
  database in both, and §3.15 says a deprecated feature should not be written into a PDF 2.0 file.
  Neither tree crosses `split` or `merge` now, and each is named.
- **§14.13.10's EXAMPLE 2 was not read.** Its named resource *is* the array of file specifications;
  `attachment::associated_in_array` reads that form beside the dictionary forms.
- **§14.10.6's note named `/PtData` and `/SI` as web capture's page entries**: `/PtData` is §12.10's
  geospatial point data and `/SI` a content set's.

## 5. Consequences

The tests named in each row are the evidence: five Annex Q cases in `survey.rs`, the DPart and web
capture fixtures in `pdf-transform`'s `split`, `merge` and `update` suites, and three fixture files
in `pdf-model` (`compositing_notation`, `multitone_examples`, `associated_file_examples`). A later
round re-reading an `inapplicable` row asks first whether a writer, the print path or the archive
path reaches it — the three places this reading found the condition false.
