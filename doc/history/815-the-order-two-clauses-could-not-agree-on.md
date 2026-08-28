# 815 — The order two clauses could not agree on, and two tables nothing watches

Finding: **an erratum removed a contradiction between Table 31 and §12.4.3 that a documented
choice in `article.rs` was resting on** — a page's `/B` array is in reading order according to
both amended sentences, so the comparison against the threads went from the set to the sequence —
and, one rank up, **an erratum renumbers Annex O's two tables where no instrument in this tree can
see it**, with the retired numbers standing on 75 lines across 27 files.

Date: 2026-08-28. Argued in ADR 0746. (0745 is a sibling round's; this number was taken two above
the tip on that reservation.)

Touched: `crates/pdf-model/src/article.rs`, `crates/pdf-model/src/variable_text.rs`,
`doc/conformance/ledger.toml` (§7.7.3.3, §9.3.4, §9.4.4, §12.4.3, §O, §O.2.1, §O.2.2),
`doc/errata-read.md`, `doc/todo/01-ledger-partial-rows.md`, `doc/adr/0746-…`.

## What the rule was asked and what it answered

The errata selection rule's sixteenth use, and the first to run step 5 twice. `spec-errata emit`
over `doc/ISO_32000-2_sponsored_EC3.pdf`, step 2's two greps, step 3's attribution with the family
guard, step 4's two rankings, step 5's ranking by issue where they went flat.

The base population reproduces the fifteenth use's closing arithmetic exactly — **302 issue
numbers carry a strike or a caret under the recipe's own single-issue line parse and 60 were named
nowhere**, which is 811's 61 less its one verdict; the multi-issue parse's 310 and 62 reproduce the
same way. Seventh consecutive use.

**The parse those two figures come from was pinned rather than assumed.** Six uses had reproduced
302 and 310 without anything saying what distinguishes them: the single-issue line parse keeps the
lines whose subject carries exactly one issue number, the multi-issue parse takes every number on
every line, and eight numbers appear only as the second number of a two-issue line — exactly the
gap. That definition is in the recipe now.

**The row ranking is flat and flatter**: both rankings top out at two annotations, seven live rows
tied there and 36 over every row, where 811 found eight and 39. The plateau shrinks as verdicts
land rather than resolving, which is what step 5 was added for. By issue: two issues at four
annotations, two at three, 28 at two, 21 at one.

## Step 5's second outing, and the part a single run could not have shown

It discriminated — a head of four against a floor of one — and it chose the reading, which is what
a selection rule owes. Two things it did **not** do are the round's methodological answer:

- **It did not produce a unique head.** Two issues tie at four annotations, and the tie went to the
  third use's tie-break, which is the row ranking's own instrument. So step 5 does not replace the
  tie-break; it hands it a tie of two instead of a tie of 39, which is the size the tie-break was
  written for.
- **The head confirmed and paid nothing**, because the tie-break prefers a requirement level and
  the requirement level was inside `CLAUDE.md`'s clause-13 exclusion. The three issues below it
  paid. Step 4's practice — head to a verdict, then downward until a row pays — is what the issue
  unit needs as well, and that is now recorded rather than assumed.

The settled/live split has not shifted: 42 of the 60 unread issues touch only a settled row and 11
touch a live one, against 43 and 12 at 811's base, with seven on no row at all. One left each
column and both were this round's.

## What the four errata were worth

`doc/errata-read.md` has all eight annotations with their rectangles against `pdftotext -bbox`.

The one that changed code turns two contradicting sentences into one: Table 31 loses *natural* and
§12.4.3's *drawing* becomes *reading*, so a page's `/B` array is in reading order in both, and
`Articles::page_array_agrees` compares the sequence where one thread supplied the beads and keeps
the set where two did. The fallback is a documented choice with its cost stated — on such a page a
`/B` out of order goes unreported — because neither sentence orders two articles sharing a page.

The one that pays nothing yet and will: Annex O's two tables are renumbered by a bare
strike-and-caret pair over each caption. `moved` cannot print it — its predicate wants a verb in
the annotation's contents and a *clause* number there — and `check` cannot either. The three Annex
O rows record the amendment; the sixth blindness and the predicate a sweep would ask are written
down.

The one that vindicated: Th is the normalized value of Tz's operand and §9.4.4's NOTE 2 becomes
normative text, both of which this tree already did. Reading it found `variable_text.rs` citing
§9.3.5 for `Tz`, which is leading — a clause number that exists and is wrong, which the citation
gate cannot see.

The head, which confirmed: a RichMediaSettings dictionary's uniqueness becomes a conditional
`shall`, three times over, inside the clause-13 exclusion.

## Two things about the instruments

**One annotation pair in the whole collection names two different issues.** Of 957 `StrikeOut` and
`Caret` annotations, every one carries both a `/Subj` and a `/T`, and exactly two disagree — both
on page 715, both this round's head. `emit` reads `/Subj` and `/Subj` is right; keyed on `/T` the
head would have held two annotations and would not have been a head.

**The owner's secondary reference is what settled that, and it is not adopted for anything else.**
`https://github.com/pdf-association/pdf-issues` is where these errata were argued; its own README
says resolutions go to ISO for ratification afterwards, so it sits one step further from the text
than the annotated collection does. It answers *which erratum is this annotation*, a question about
the file; it decides no reading, changes no population, and is fetched and gated on by nothing. The
identification was calibrated first against an issue this tree already has a verdict for.

## Gates and sweeps

The full §2 sequence, as this file's own section defines it — a fifth round owes it whatever it
touched. §4's sweeps before and after, against a pristine checkout at the base commit with its own
build directory.
