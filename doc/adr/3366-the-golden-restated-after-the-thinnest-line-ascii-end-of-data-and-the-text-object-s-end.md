# 3366 — The raster golden's 11 moved pages, restated after the thinnest line, ASCII end-of-data and a text object's end

Session 2166. Status: **accepted.** Context: ADR 1016 (the golden), ADR 3051 §1 (the method: build the commit before
and the commit after on this machine, never splice a line), ADR 3145 (the last restatement, round 2111, `3b218c9f`),
ADR 3133 (the walk refuses a run with a wall-clock refusal), ADRs 3281, 3333 and 3353–3355 (the changes that moved the
pages), ADR 3365 (this round's strips fix), `crates/pdf-model/tests/raster_golden.rs`. Clauses: ISO 32000-2 §7.4.2,
§7.4.3, §8.4.3.2, §8.4.3.5, §9.4.1, §10.7.5.

## 1. The method

The check walk at `bec8ae27` (bounded, `--profile gates`, no wall-clock refusal) held 963 and moved round 2163's 11.
Round 2111's scratch dump (each first page's list and reports as text, its raster as bytes, digested as the golden
digests them, and here also drawn at 2, 4, 8 and 16 strips) was built at `3b218c9f`, `7e990758~1`, `7e990758`,
`bec8ae27~1`, `bec8ae27`, `dd12e73b~1` and `dd12e73b`. At `3b218c9f` its digests equal the golden's on all 11, and at
`bec8ae27` they equal, line for line, the values the update walk wrote.

## 2. The classes

| pages | moved at | what the dumps show |
|---|---|---|
| `PDFBOX-3148-2-fuzzed` | `dd12e73b` (round 2145, ADR 3281) | reports only: ASCII data ending before its EOD marker is truncated and said; equal at `dd12e73b~1` |
| `bug1703683_page2_reduced`, `bug1721218_reduced`, `bug1743245`, `bug1755507`, `issue19360`, `zerowidthline` | `7e990758` (round 2158, ADR 3333) | raster only, lists and reports equal. Each strokes `0 w` or a `/SA` stroke §10.7.5 promotes: the thinnest line is one device pixel every way and has no mitre. 32, 3, 49 697, 2, 1 023 and 345 pixels; `zerowidthline`'s and `issue19360`'s stroked glyphs lose only mitre tips (every moved pixel of `zerowidthline` lighter; crops in `r2166/side-*.png`), `bug1743245`'s ruled sheet is its rules drawn as device hairlines |
| `bug1065245`, `issue9458` | `bec8ae27` (round 2163, ADR 3353) | reports only: one "ET outside a text object" each |
| `issue10388_reduced` | `bec8ae27` | reports only: "TJ" and "Tm outside a text object" |
| `poppler-90-0-fuzzed` | `bec8ae27` | reports only: `Td`, `Tj` and `Tm` outside a text object are added |

`7e990758~1` and `bec8ae27~1` are equal on all 11, so `604e04e4`, `66fa552f` and `220b3bd4` move none; the commits
from `3b218c9f` to `7e990758~1` move only `PDFBOX-3148-2-fuzzed`. Rounds 2146 and 2147 (ADRs 3285 and 3290) moved no
page.

## 3. The decision

All 11 are explained, each by one commit and one cause, so all 11 are restated by the walk's update mode; the diff of
`raster_golden.tsv` is exactly those 11 lines. No page is left moved. ADR 3365's strips change moves no golden digest:
the golden draws in one strip.
