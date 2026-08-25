# ADR 0621 — An erratum read one page at a time

Status: accepted, 2026-08-25. Session the seven-hundred-and-thirtieth, beside ADR 0620 and found by
the same reading — `doc/errata-read.md`'s standing rule is that a round touching a clause runs
`spec-errata emit` on it first, and this is what the run turned up. Corrects one paragraph and adds
two rows to `doc/errata-read.md`. **No status moves, no pixel moves, no report.** Extends ADRs 0426,
0492, 0496 and 0601.

## 1. What was recorded, and what the issue actually does

The six-hundred-and-sixty-sixth session read Issue **#619** (`Review/Accepted`) while reading
§11.6.6's family, corrected `emit`'s filing of it — the two carets it saw sit at the bottom of page
436, which the outline puts in §11.6.6 while the text is §11.6.5.2's Table 143 — and wrote:

> **Issue #619, `Review/Accepted`, adds a deprecation notice to Table 143's `/ID` and `/OPI` rows.**

and closed with:

> Table 143 states it for both entries now and Table 87 still states it for one.

Every fact in that paragraph is right and the conclusion is wrong. **Issue #619 carries four
carets, not two.** One sits on page **275**, in Table 87's own `/ID` row; one on page **121**, in
Table 31's. So the issue deprecates `/ID` in every table that states it, and the unevenness the
paragraph diagnosed — Table 87 carrying the notice for `/OPI` and not for `/ID`, while §14.10.1
deprecates the whole of Web capture — is exactly what the issue repairs.

## 2. The arithmetic, which is `doc/errata-read.md`'s own

A `/Rect` is in PDF coordinates from the bottom and the page is 841.92 tall; `mutool draw -F stext`
and `pdftotext -bbox` report from the top.

| page | rect | from the top | the line there | where the centre lands |
|---|---|---|---|---|
| 275 | `[390.62046 176.32236 399.6965 183.71765]` | 658.2–665.6 | Table 87's `/ID` value cell at 654.9: `(Optional; PDF 1.3; indirect reference preferred) The digital identifier of the` | x 395.2, one glyph inside `preferred)`, which ends at 399.2 — the insertion point before the closing parenthesis |
| 121 | `[284.3599 256.12236 293.43595 263.51765]` | 578.4–585.8 | Table 31's `/ID` value cell at 575.1, the same sentence | x 288.9, one glyph inside `1.3;`, which ends at 291.5 — before that row's semicolon |

Both give the same reading of the same four words, `; Deprecated in PDF 2.0`, and both land in the
parenthetical rather than in the prose after it.

Page 121's is worth one more line, because a second erratum stands on the same span: Issue **#106**
strikes "; indirect reference preferred" from that row. The two compose — "(Optional; PDF 1.3;
Deprecated in PDF 2.0)" — rather than colliding, which is the opposite of ADR 0601's pair, and is
worth recording as the ordinary case beside the exceptional one.

## 3. The lesson is about the instrument

`emit` files an annotation by the page it is on, which is the right thing for a tool whose whole
population is a PDF's annotations. The consequence is that an **issue** whose carets are scattered
across three tables prints as three entries hundreds of lines apart under three different clause
headings, and a round reading one clause reads a third of the issue.

ADR 0492's round did not read a stale sentence: it wrote a wrong one, from facts that were right,
because the reasoning stopped at the page in front of it. That is 0610's shape — a conclusion no
sweep can print, because what is wrong is neither a number nor a pointer — and it has now appeared
twice in six rounds.

**The rule this adds to `doc/errata-read.md`:** a round recording what an erratum *leaves alone*
asks `emit` for that issue number across the whole document before it concludes anything, because
`emit`'s own filing hides the rest of the issue behind other clauses.

## 4. What it costs this tree

Nothing that draws. `/ID` is Web capture's, unread here under either printing, and both §7.7.3.3's
and §8.9.5.1's rows already dispose of it. What changes is that `doc/errata-read.md` no longer
asserts an inconsistency the standard's own errata have closed, and that two of its rows now name
the issue on the clause it touches — which is what the file is for.
