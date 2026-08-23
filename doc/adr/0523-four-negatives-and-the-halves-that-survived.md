# ADR 0523 — Four negatives re-derived, the halves that survived, and an erratum that strikes nothing out

Status: accepted, 2026-08-23. Session the six-hundred-and-eighty-sixth, a clause round under
`doc/todo/01`'s sixteenth sweep. Gives `long_mitre_census`, `hollow_glyph_census` and
`border_precedence_census` the three-scope selector `witness_census` and `absence_audit` already
have, records in `witness_census`'s module comment what its third column is for, amends the ledger
rows of §8.4.3.5, §9.7.4.2, §12.5.4 and §14.8.2.5.3, adds one row to `doc/errata-read.md`, and
corrects three of `doc/todo/01`'s four groups. Extends ADRs 0405, 0490,
0493, 0496, 0502 and 0516; changes nothing ADR 0350 or ADR 0398 decided.

## 1. What this decides

Four more of the queue's negatives, re-derived over `CC-MAIN-2021-31` with the control run stated
beside the crawl run (ADR 0490). **Three are false and one holds**, and in two of the three a
*sharper* claim survives the falsification, which is the finding worth more than the counts.

| clause | the claim | pdf.js 974 | curated 1251 | crawl 65 944 |
|---|---|---|---|---|
| §8.4.3.5 | a long mitre that is dashed, or at or under a device pixel, has no witness | 0 | 0 | **0 — it holds** |
| §8.4.3.5 | …and no crawled page has a long mitre *at all*, though 116 state a limit admitting one | 2 fixtures | 2 fixtures | **0 of 65 659** |
| §9.7.4.2 | no corpus document combines a `/CIDToGIDMap` stream with a wholly hollow program | 0 | 0 | **3** |
| §12.5.4 | no corpus document writes Table 168's `B` or `I` | 0 | 0 | **170** |
| §12.5.4 | no corpus document states a non-zero `/Border` radius the precedence discards, on a subtype whose `/BS` is a border | 0 (6 on `/Ink`) | 0 | **3, all on `/Link`** |
| §14.8.2.5.3 | no corpus document writes `/ReversedChars` | — | **1** | **3, of which one is not the tag** |

**The queue's own script moved 26 done / 20 owed → 30 / 16 over an unchanged population of 46.**

## 2. The instrument each row named already existed, and each had one population

`doc/todo/01` filed §8.4.3.5, §9.7.4.2 and §12.5.4 as rows that "owe a `--crawl` argument rather
than a re-reading", and that was right. What it cost is three examples' worth of the same edit ADR
0493 made to `witness_census` and ADR 0496 to `absence_audit`: a `Scope` of `--pdfjs`, the curated
corpora or `--crawl`, a recursive collector, and `rayon`. Two of the three took explicit paths on
the command line and keep taking them.

**Each was run against the population its sentence was measured over first, and each reproduced
that sentence's numbers exactly** — 33 781 constructed borders with one `U`, one `D` and no `B` or
`I`; 42 `/CIDToGIDMap` streams in 30 documents with 214 of 221 programs partly hollow and no
intersection. That is the control ADR 0493 asks for, in the direction nobody usually runs it: not
a planted witness, but the old number, re-derived. A census that had drifted would have printed a
different figure for the population the row names, and there would have been nothing to say about
the crawl until that was explained.

## 3. A false negative can have a *sharper* half that survives, and it is a different sentence

ADR 0516 found this at §11.6.5.2, where a claim false by 2882 documents named a refusal reached by
six. Two of this round's four are the same shape and one is sharper still.

**§9.7.4.2.** Three crawled documents combine a remapping stream with a program whose every `loca`
entry is empty — `0100390.pdf`, `1776718.pdf`, `1899548.pdf`, from an iText, a QuarkXPress and one
producer the file does not name, so the combination is not one OCR vendor's habit. But ADR 0350
did not build its fixture for the *structure*; it built it for the symptom — a text layer whose
codes all land on empty outlines, where a reader deriving a character's extent from the glyph paints
no selection. On every page of all three, `codes_reaching_a_blank_glyph` is **zero**: the hollow
program is embedded and never shown. So the structure has three witnesses and the symptom has none,
and `test-scenes::scanned_ocr_pdf` keeps its job for the reason it was given it.

**§12.5.4.** Six pdf.js annotations state a non-zero `/Border` corner radius under a `/BS` that
Table 166 makes the precedence discard, and all six are on ink annotations — a subtype this clause
says a `/BS` is *not* a border for, which is why the row could say "no corpus document witnesses
it" while the census printed 6. Three crawled annotations state one on a **`/Link`**, which is a
subtype whose `/BS` is a border, so each of the three is a rectangle this tree would have drawn
with rounded corners under the reading the four-hundred-and-fifty-eighth session removed. The
falsification and the sharpening are the same three documents.

**The rule, stated once**: when a census over a noun falsifies a claim, ask which *sub-population*
the claim's consequence is about, and count that too. The two numbers are the finding; either one
alone is a wrong answer with a right arithmetic behind it.

## 3a. And one held, which is worth as much

The last three rounds on this queue found ten of eighteen and seven of eight negatives false, which
is a rate at which a round stops expecting the other answer. §8.4.3.5's is the other answer.

Over 65 659 openable crawled first pages, **not one has a mitre join whose geometry reaches the
ratio `tiny-skia` bevels** — so neither of the two shapes ADR 0398 declines, a dashed stroke and one
at or under a device pixel, has a witness either, and the sentence stands on a population fifty-three
times the size of the one it was written for.

What the run adds is the *shape of the population around* the defect, and that is a finding the
falsifications do not have an analogue for: **116 crawled first pages state a mitre join whose limit
admits a ratio over 90.51, across 4419 strokes, and none of those strokes has the angle.** A large
`M` is ordinary; a long mitre is not. `pdf-differences`' two files remain the whole of the world this
construction draws differently on, which is exactly the risk ADR 0398 weighed and is now measured
rather than assumed. **A negative that holds is not a null result** — it converts a construction's
justification from "no file we have does this" into "no file in sixty-six thousand does", and it
prices the next round's attention somewhere else.

## 4. The instrument for a marked-content tag was the third column of a name census

`doc/todo/01` moved §14.8.2.5.3 into the group needing "a content-stream census, which nothing in
this tree has", on the correct ground that `/ReversedChars` is a marked-content tag and a structural
walk over the object graph would report a false zero. **The premise about the instrument was
wrong.** `witness_census` asks three questions of every document, and the third is a substring
search of every stream's *decoded* data — which is exactly a content-stream census for anything
whose witness is a token. It finds the tag in `issue19971.pdf` (curated) and in `2268468.pdf` and
`5958456.pdf` (crawled), each opening `BT /ReversedChars BMC` over Arabic, Persian and Hebrew.

It also prints its own discriminator. `5343111.pdf` scores the tag *as a name* rather than only in
a stream, and reading it shows why: the document states `/S /ReversedChars` as a **structure element
type** with a `/RoleMap` sending it to `/Span`. That is a name in §14.7's tree and not a tag on any
page — ADR 0403's warning, caught by the census's own column layout rather than by a spot check.

**And the population was narrower than the sentence in a second way.** The old measurement was over
*first pages*; `issue19971.pdf` writes its tag on page 6. A curated witness would have been
invisible to it however much the submodule grew, which is a decay a re-run over a larger corpus does
not fix and a re-reading of the instrument does.

What follows for the queue: **§9.7.5.4's `beginrearrangedfont` and `beginusematrix` are tokens in an
embedded CMap stream and are the next row this instrument settles.** Its control run is taken and is
in this round's record — 0 of 1251 curated documents for either operator, with `usecmap` found in one
document's decoded stream as the positive control that the search reaches a CMap operator at all —
and the crawl run is owed. The remaining four in that group are shapes rather than tokens — a path
segment with no current point, a `q`/`Q` inside a text object, a codespace range read two ways, a
tiling pattern's paint — and still need the operator census.

## 5. Issue #154, and the fourth erratum that strikes nothing out

Running `spec-errata emit` on every clause this round touched — the rule ADR 0426 states and the
fourth consecutive round it has paid for — found **Errata Collection 3's Issue #154 on §8.4.3.5,
recorded nowhere in this tree**. It is a bare Caret, `Review/Completed`, whose `/Rect` lands between
"limit" and "shall" on the line that reads *The miter limit shall impose a maximum on the ratio of
the miter length to the line width*, inserting "shall be a number greater than or equal to 1.0 and".

`spec-errata check` cannot see it and never could: it compares this tree's quotations against
*struck* text, and a pure insertion strikes nothing. `doc/errata-read.md` now records four `Caret`s
with no `StrikeOut` — #293, #34, #536 and this one — which is a shape rather than an anecdote:
**an erratum that only adds is invisible to every instrument here except `emit`.**

The erratum vindicates the code and replaces its argument. `content.rs` clips the mitre limit below
at 1, and this row justified that by inferring the floor from the clause's own ratio `1 / sin(φ/2)`,
which never goes below 1. The amended clause requires the value outright. `CLAUDE.md` principle 5's
"a clause that permits is a clause that has been read, and it is a stronger answer than one that
does not apply" has a sibling here: **a clause that states a bound is a stronger answer than one
that implies it**, and the derived version had stood since the twenty-fourth session.

## 6. What none of this changes

No code path moves. §8.4.3.5's clamp, §9.7.4.2's four routes, §12.5.4's `B`-and-`I` report and
§14.8.2.5.3's reversal are all unchanged, and every one of the four rows keeps its status. What moves
is what each rests on: a count taken over a population fifty-three times the size, a symptom
counted apart from its structure, an erratum recorded, and — in §12.5.4's case — a report that had
no witnesses in either curated population and has 170 in the world, which is the first evidence
that the refusal it prints is ever read by anybody.

One thing is left open on purpose. §14.8.2.5.3's new witness reads back in the opposite direction
from `pdftotext`'s, and every code on its page sits inside a `/Span` whose §14.9.4 `/ActualText`
names one character — which this clause's `shall` does not mention, since it is about "the sequence
of the characters as found in the show string operator". Exactly one of the two implementations is
reversing that page. Principle 5 says that is a question to take back to the specification and not
a target to move toward, and this round had no reading of §14.9.4 good enough to decide it, so the
row carries the question and the code is untouched.
