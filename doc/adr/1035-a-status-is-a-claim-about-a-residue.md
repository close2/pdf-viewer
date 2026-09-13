# 1035 — A status is a claim about a residue, and a heading has none of its own

Session 1017. Status: **accepted**.

`doc/reviews/1012-where-the-effort-goes.md` measured that over 58 sessions **exactly one of the
875 pre-existing ledger rows changed status, and it moved backwards**, while the file grew 77 KB
of prose. Its §2.1 and §2.2 found why rounds allocated to "`partial` rows" produce corrections to
notes rather than code: of 206 `partial` rows, **58 could not be worked on at all** — they are
headings whose debt is their subclauses' — and of the 148 that were left, 60 were classified as
"not a gap". `partial` had come to mean five different things at once, which is the exact
vocabulary collapse the ledger's own header says the statuses exist to prevent.

This ADR states the rule for deciding what a row's status is, so that later rounds do not
re-argue it row by row.

## 1. A status describes the clause's **residue** — what is not executed

A row's status is not an average of the clause and it is not a summary of the work done under it.
It is a claim about **the requirements this project answers that are not executed**, and the four
settled statuses are the four ways that set can be empty:

| the residue is | status |
|---|---|
| nothing — every requirement this project answers is executed | `implemented` |
| requirements with no meaning for this device, on the standard's own condition | `inapplicable` |
| requirements addressed to whoever writes a file | `writer-side` |
| requirements on `CLAUDE.md` principle 5's closed exclusion list | `out-of-scope` |

**Principle 5's exclusions remove requirements from the population rather than from the clause.**
A row whose entire residue is excluded therefore has an empty residue and takes `implemented`,
with its note naming which entries the exclusion covers and which of the four it is — unless the
*whole clause* is what the exclusion covers, in which case `out-of-scope` and its `exclusion` key
say so directly. §12.7.3 is the first kind (`/CO` is script behaviour, `/XFA` is XFA, everything
else in Table 224 is read); §12.5.6.17 is the second (the clause's one requirement of a processor
is "When the annotation is activated, the movie shall be played").

## 2. What `partial` is left meaning, and the one thing it may not be used for

`partial` is a requirement **owed**: stated by the standard, addressed to this program, not
executed, and nobody has decided against it. It may not be used to record a decision. A row whose
note ends "nothing is owed" is not `partial`; §8.6.4.4 stood that way for nine hundred sessions.

**A row is not moved on a guess.** Where the note does not name what its residue is, the row stays
`partial` and the reading is owed; inventing a justification to clear a row is worse than leaving
one wrong, because a settled status is the one nobody comes back to (that is `inapplicable.rs`'s
whole premise, and §10.5, §14.11.3, §14.11.6.2, §14.8.5.4.3, §14.8.5.4.5 and §14.8.5.8 each spent
a hundred sessions or more settled on a reason that had decayed).

## 3. A deliberate departure is `partial`, and the vocabulary has no better word

Twenty of the 143 `partial` leaves record a requirement this tree **decided not to execute**, each
with its cost measured — §7.5.5's `/Size` at 66 documents, §7.4.8's `/ColorTransform` at the one
file that exercises it, §8.6.5.7's declined `should` at under 0.88 of 255, §10.7.4's five
scan-conversion departures, §12.11.6's `shall not continue` refused on principle 3's own grounds.

Those rows are **correctly `partial`** under §2: the requirement is stated, addressed to this
program, and not executed. What the review read as sixty rows wearing the wrong status is mostly
this: not a mis-statusing but a **missing word**. The ledger distinguishes the project *choosing*
from the project *owing* for a whole clause (`out-of-scope`, `inapplicable`) and has no way to say
it about one sentence inside a clause that is otherwise implemented.

Adding a sixth status is not this round's to take — `doc/reviews/1012` §2.6 puts it to the project
owner, and a status is the ledger's vocabulary rather than an instrument's. What is decided here
is that **until the owner rules, a departure stays `partial` and is not re-statused into a settled
word**, and that the departure count is reported beside the debt count rather than folded into it.

## 4. A heading is not a piece of work — and the rule is mechanical

The standard's numbering *is* the containment, so no table is needed to tell a heading from a
leaf. [`Ledger::is_aggregate`](../../tools/conformance/src/ledger.rs) is the rule:

> a row is an **aggregate** when it still [`Status::owes`] something and at least one row whose
> clause number it is a strict ancestor of still owes something too.

An aggregate cannot be assigned to a round: it flips when its last unsettled child flips. A round
that "works on §12.8" is working on §12.8.3.3.1. `Ledger::owing` is the debt — every unsettled row
that is not an aggregate — and the gate prints it beside the status counts, so the two numbers
cannot be confused again. Today that is **159 of 218 unsettled rows**; the other 59 are headings.

Three things the rule deliberately does **not** do:

- **It does not read notes.** A rule that asked whether a note names a debt would be a judgement
  wearing a program, and `--bin quotations`' matcher is what that costs.
- **It does not settle a heading.** An aggregate keeps its status; the rule only stops it being
  *counted* as work. A heading whose note names a debt of its own is still `partial` and still
  reported as one, because the mechanical test cannot see the difference and does not claim to.
- **It does not reach a sibling aggregate.** Seven leaves — §8.7.4.1, §8.11.1, §8.11.4.1, §11.4.3,
  §11.4.8, §12.1, §12.5.1 — carry a neighbour's debt rather than a descendant's, and the numbering
  cannot see that. §11.4.8 is the clean case: "This subclause is a restatement of the group
  compositing formulas", so it states no requirement of its own and its status is §11.4.4's and
  §11.4.6's. They stay `partial` and are named here rather than given a marker the ledger's
  regenerator would drop.

## 5. The counterpart gate

The rule can be wrong in exactly one direction that matters, and
[`Problem::AggregateWithoutDebt`](../../tools/conformance/src/ledger.rs) is the gate for it: a row
that still owes something when **every** row below it has settled is carrying a debt none of its
subclauses carries. That is either a debt of its own the note has never named, or a status nobody
moved when the last child moved. It fires on nothing today, which is the right state for a gate
whose subject is a moment that has not happened yet, and it costs one pass over 883 rows.

## 6. What this round moved, and what it did not

Five rows, each on the evidence its own note already held and on the clause read again:

| clause | to | why |
|---|---|---|
| §8.6.4.4 | `implemented` | its own note: "there is no sentence in this subclause left to implement". The documented choice it records is stated in §10.4.2.5 and carried by §10.4.2.5's row |
| §12.3.4 | `implemented` | it was `partial` for generating a missing thumbnail, and the clause states no such permission — its two `may`s are that a document may contain thumbnails and that a processor may display them |
| §12.5.6.6 | `implemented` | the one entry left owed is `/DS`, which Table 177 defines by reference to the XFA specification |
| §12.7.3 | `implemented` | the two entries left unread are `/CO` (script behaviour) and `/XFA` |
| §12.5.6.17 | `out-of-scope` | the clause's one requirement of a processor is that the movie be played |

**138 rows were read and left `partial`**, and the report names which. Ten of them are left because
the note does not say what the row is `partial` *for*, or because two rows about one mechanism
disagree — §12.5.6.21 calls Table 403's `/LastModified` writer-side while §14.11.6.2 reads a
reader's `shall` out of the same entry, and §12.5.6.20 says no source names `/MN` while §14.11.3
says §12.5.6.20 reports it. Those are readings a later round owes, not statuses this one may move.
