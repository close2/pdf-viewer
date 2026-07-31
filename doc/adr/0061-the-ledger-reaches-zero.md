# ADR 0061 — The ledger reaches zero

Status: accepted, 2026-07-31.

## What happened

**Every one of ISO 32000-2's 823 technical subclauses has now been read against this code.**
`UNREVIEWED_CEILING` is 0, and the assertion that guarded it is now an equality: a row that
arrives `unreviewed` — because a future edition of the standard gains a subclause, or because
`bin/ledger` finds one this file lacks — fails the build until somebody reads it.

The count started at 314 in the ninth session and came down in twenty-eight family reviews. The
shape of the answer:

| status | rows | |
|---|---|---|
| `implemented` | 256 | every normative requirement in the clause is executed |
| `partial` | 159 | some are, and the note says which are not |
| `silent` | 195 | not implemented, and nothing says so |
| `inapplicable` | 89 | describes a marking device, a layout engine or a production workflow |
| `out-of-scope` | 87 | on `CLAUDE.md` principle 5's closed exclusion list, which the row names |
| `reported` | 30 | not implemented, detected and named at runtime |
| `writer-side` | 7 | addresses a PDF writer; we do not create files |

**195 `silent` rows is the finding.** The number was 2 as recently as the forty-second session,
and every one of the 193 that arrived since came from *reading* rather than from any change to
the code. `unreviewed` and `silent` are different admissions — one is *we have not asked*, the
other is *we asked, and we owe it without saying so* — and the whole exercise has been converting
the first into the second, third or fourth.

Where the silence is:

- **Clause 12's interactive half.** Nothing follows a link, performs an action, edits a field or
  shows a panel. Almost every row of it is a *viewer* rather than a clause, which is a true
  summary of what this program is: it renders pages correctly and does nothing when a person
  clicks on one.
- **Clause 14's structure.** §14.7's tree, §14.8's types and attributes. None of it changes a
  mark, and §14.9 is what says how much of it accessibility needs.

## What the instrument turned out to be for

The ledger was built to answer "which of the standard's requirements are implemented", and it
answered that. What was not anticipated is that it would produce **work items no other instrument
in this project could**:

- **A missing component.** Four rows in two clauses named one absent data structure — a name or
  number tree — which no single clause review would have shown and no corpus document would ever
  have asked for. ADR 0053 built it; all four rows have now closed on it, the last in this
  session.
- **Three false claims.** §8.7.3.1's "`/BBox` clips the cell", §8.7.2's pattern space inside a
  form, and §14.6.2's "takes both forms" were all written from the clause during a review and
  were true of no code. Each cost a visible defect on a real page and each was found by the
  oracle rather than by the ledger — so the ledger's notes are a hypothesis the gates test, and
  not the other way round.
- **A ranking the corpus cannot give.** §6.3.2.2 puts optional content and annotation appearances
  above almost everything a demand curve would rank first, because a demand curve cannot rank a
  requirement no file exercises.

The `FILE_ONLY_EVIDENCE_CEILING` added in ADR 0057 is the direct answer to the second of those:
58 `implemented` rows still name a whole test *file* as their evidence, which passes whatever it
contains. That number may only fall, and it is the ledger's own next piece of work.

## What is not finished

Reading every clause is not implementing every clause, and the ledger says so in its own
vocabulary — 195 silences and 30 reports are the distance left. **What has changed is that the
distance is now measurable and itemised**, and that no part of the standard is unexamined: there
is no longer a place where "we have not looked" can hide a requirement nobody has thought about.
