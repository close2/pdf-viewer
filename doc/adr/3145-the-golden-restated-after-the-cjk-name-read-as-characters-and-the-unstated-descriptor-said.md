# 3145 — The raster golden's 7 moved pages, restated after a CJK name read as characters and an unstated descriptor said

Session 2111. Status: **accepted.** Context: ADR 1016 (the golden), ADR 3051 §1 (the method: build the commit before
and the commit after on this machine, never splice a line), ADR 3113 (the last restatement), ADR 3117 (a `/BaseFont`
read through its collection's encodings), ADR 3119 (a font outside the fourteen stating no descriptor is said), ADR
3133 (the walk refuses a run with a wall-clock refusal), ADRs 3146 and 3147 (this round's other changes),
`crates/pdf-model/tests/raster_golden.rs`. Clauses: ISO 32000-2 §9.5, §9.6.2 (Table 109's `FontDescriptor`), §9.8.2.

## 1. The method

The golden at `4a66dc26` is ADR 3113's, written at `9095a81e`. Round 2108 saw 7 pages moved at `d24c3aea`, and the check
walk at `4a66dc26` (this round, no wall-clock refusal) held 967 and moved the same 7. Of the commits after `af84fb4b`'s
restatement and its parent `3a742aaf`, `e2ef18ed` and `d24c3aea` change only `pdf-transform`, outside `pdf-model`'s
graph; `3a742aaf` (`pdf-font`, `pdf-model`), `c862f264` (`pdf-model`'s rich text and variable text), `25c93430`
(`pdf-render`'s shading) and `4a66dc26` (`pdf-sandbox`) are inside it.

Round 2103's scratch dump (each first page's list and reports as `{:#?}` text, its raster as bytes, digested as the
golden digests them) was built at `e2ef18ed`, `3a742aaf`, `c862f264`, `25c93430` and `4a66dc26`. At `e2ef18ed` its
digests equal the golden's on all 7; at `3a742aaf` they move to the values the check walk reports, and the three later
commits leave all 7 equal to `3a742aaf`'s.

## 2. The classes

| pages | what the dumps show |
|---|---|
| `issue3521` | with path geometry set aside the lists are equal line for line, and the reports equal: only the outlines of the 7 glyph fills change. Its 黑体 in GBK is now read as Hei, a sans style (ADR 3117), so "我们都是黑体字" is drawn by a sans face where the descriptor's Serif flag had chosen a serif one; 1 560 pixels in the line of text differ, and the two crops show Song strokes become gothic ones |
| `issue17929`, `issue18099_reduced`, `issue2391-2`, `issue6069`, `issue6106`, `issue6108` | list and raster equal; the reports gain exactly one item each, ADR 3119's "font … states no /FontDescriptor, which ISO 32000-2 Table 109 requires outside the fourteen standard fonts". The files agree: `issue6069`'s `/Arial` and `issue2391-2`'s `/Monaco` are `Type1` dictionaries with no descriptor, and `issue17929`'s font spells the key `/GontDescriptor` |

## 3. The decision

All 7 are explained by `3a742aaf` and nothing else, so the update mode rewrote the file in the worktree, with this
round's ADRs 3146 and 3147 built in, and its diff is exactly those 7 lines, each equal to the dump's digests at
`4a66dc26`; no line was written by hand. The update walk held the other 967, so this round's own change moved no page:
`issue4722`, the one tracked file naming `Century Gothic`, is a `CIDFontType2` under `Identity-H` with no program and
no `/ToUnicode`, which is refused before a family is asked (§9.7.5.2), and no tracked file names `HiraMin` in bytes a search reads uncompressed.
