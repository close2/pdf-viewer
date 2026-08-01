# ADR 0108 — A reason that expired, and the action it was blocking

Status: accepted, 2026-08-01.

## Context

ADR 0107 ended with a rule worth acting on rather than only recording:

> **A note that gives a reason gives a trigger, and nothing fires it.** "While X does not exist"
> expires the day X lands, and no gate in this project can see that day arrive.

So this session swept the ledger's 823 notes for conditional reasons — `while … does not exist`,
`until …`, `needs §…`, `the day …` — and read each against what the tree now holds. Sixty-two
fragments matched; most were history ("until the eleventh session") rather than a condition.
Three were live.

## What the sweep found

**§12.6.4.5's `GoToDp` was refused because §14.12 was `unreviewed`.** It has not been
`unreviewed` since the fifty-sixth session; §14.12 is `inapplicable`, on the sound reasoning that
document parts are a production-workflow structure and nothing in the family changes a mark. And
the action stayed refused, because the row that named the blocker was never read again after the
blocker changed.

**§14.12's `inapplicable` had decayed, and `doc/HANDOVER.md` names this exact failure:**

> **An `inapplicable` row decays exactly as a `silent` one does.** §12.7.4.2's field names were
> `inapplicable` for eighteen sessions on sound reasoning … until §12.6.4.11's hide action was
> implemented and a field name decided whether an annotation is drawn.

This is the second instance. §12.6.4.5 says a `GoToDp` action "changes the view to the Start page
of a specified DPart", which makes a `DPart` dictionary decide **which page is shown** — a
question §12.3.2's destinations answer and no marking clause does. An `inapplicable` row means
nothing is owed *by the clauses that reach it today*, and today one does.

**§12.7.8.3.1 said `/Pages` "needs §12.7.7's named pages".** Named pages landed in the
hundred-and-first session (ADR 0091) and `forms_data::read_pages` has applied `/Pages` ever
since. The note went on saying it was owed for seventeen sessions.

## Decision

**Perform `GoToDp`.** Table 206's `/Dp` is kept as an `ObjectId` — resolving it needs the page
tree, which belongs to the caller — and `document_part::first_page` answers where the part
begins.

The one reading the clause does not spell out is a `GoToDp` naming a *node*. Table 409 makes
`/Start` and `/DParts` exclusive, so a node has no `/Start` at all and a reader that looked only
there would have nowhere to go. §14.12.3 settles it:

> The order of page objects as defined by the page tree shall be in the same order in which page
> objects are referenced from leaf node DPart dictionaries in a depth-first traversal of the
> document part hierarchy.

So a node's first page is its first leaf's, depth first — a reading of the clause rather than a
convention. `/DParts` is "[a]n array of arrays" and is walked as one, which changes nothing for
the *first* leaf and would change the count of children.

`crates/pdf-model/src/document_part.rs` reads that and nothing else: the tree's `/DPM`,
`/NodeNameList` and `/RecordLevel` are still the job ticket's and still `inapplicable`. The
module's own header says so, because a module inside an `inapplicable` clause needs to say what
made it applicable.

## Consequences

`reported` falls **36 → 35** and this program performs **eleven** of §12.6's actions. Tests
873 → 876; no gate moved, and no corpus document states a `/DPartRoot`, so the three unit tests
are the whole of the evidence — trap 8 again, and the third session running.

Two things to carry:

**The sweep is repeatable and cost twenty minutes.** One regular expression over the ledger's
notes, then reading the sixty-two matches. Two of the three findings were rows that named a
*blocker* rather than a gap, which is the class no gate can watch: the row is true when written,
false when the blocker lands, and nothing re-reads it because nothing changed in *its* clause.

**Write the trigger where it can be found.** A note saying "needs §X" should be greppable from
§X's own row, and today it is not — §14.12's row said nothing about §12.6.4.5 depending on it.
The cheap discipline is the one `doc/HANDOVER.md` already states for the other direction: when a
family is implemented, `grep` the ledger for its clause number and read what comes back.
