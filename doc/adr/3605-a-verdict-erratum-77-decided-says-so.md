# 3605 — A verdict erratum #77's direction decided says so, accepted or rejected

Session 2226. Status: **accepted**. Context: `doc/questions/A1043`, ADR 3581, ADR 3582,
pdf-association/pdf-issues #77; ISO 19005-2 6.2.11.3.1 and ISO 19005-4 6.2.10.3.1 (paraphrased,
`doc/third-party-data.md`).

## Context

ADR 3582 applies #77 as the ISO 19005-4 errata page publishes it: a `CIDFont`'s supplement at
most its `CMap`'s. The PDF/A TWG decided on 2023-12-11 not to carry the change into the dated
revision and parked it on 2024-02-16 pending a white paper, while the page still publishes it.
The owner kept the erratum applied and asked that the validator say this whenever it accepts or
rejects a document on it (A1043).

## Decision

1. **A note is owed by a document, not by a row.** `pdf_archive::errata::Note` is an issue
   number and one sentence; a predicate states it through `Findings::note`, and the judgement
   carries it as `Judgement::noted` whether the row was met or failed. The erratum table
   (`errata::ERRATA`) prints under every failure of a row and could not say "only where this
   file reached the question".
2. **Where #77 decides.** `cid_system_info_agrees_with_the_cmap` states
   `SUPPLEMENT_DIRECTION` for a non-Identity `Type0` font whose `CIDFont` and `CMap` (embedded,
   or a predefined program's own) state different supplements. Equal supplements meet the row
   under either direction, so no note is printed.
3. **Where it is said.** `Report::render` prints it under the failed row and, for a met row, in
   a section of its own (`met, under a reading that is noted:`), since a met row is otherwise
   not listed. `quorra-retrieve archive`'s JSON puts `notes` on the row and lists met rows with
   notes under `met_with_notes`. The converter copies the input row's notes onto its `Decided`
   (`noted`), so a `collection-restated` answer prints and serialises the same sentence.

## Consequences

The note names the applied direction, the errata page, #77, ADR 3582 and the TWG's two dates,
and says a validator reading the printed clause may judge the font the other way. When the TWG
settles #77, the sentence and this ADR change together.
