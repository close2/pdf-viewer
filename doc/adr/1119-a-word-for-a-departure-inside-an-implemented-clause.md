# 1119 — A word for a departure inside an otherwise-implemented clause

Session 1105. Status: **accepted**.

ADR 1035 section 3 read every `partial` ledger row against the standard and found twenty that are
`partial` *correctly* under the ledger's own vocabulary and *wrongly* under what a reader takes
`partial` to mean: each records one `shall` addressed to this program that the project decided
against, with its cost measured and written down, while every other requirement of the clause is
executed. The ledger could say *decided* for a whole clause — `out-of-scope`, `inapplicable` — and
had no way to say it about one sentence inside a clause otherwise implemented. So each of the twenty
wore `partial`, the same word as a row nobody has finished, and every "how much is left" figure this
project prints counted it. ADR 1035 section 3 forbade re-statusing a departure to `implemented`
meanwhile, because that would hide the sentence, and put the question to the owner as
`doc/questions/Q63`.

## The decision

The owner answered on 2026-09-14: **"Add `departed`."** (`doc/questions/A63`.) The word means *every
requirement of the clause is executed except the one the note names, which was decided against with
its cost recorded*. Two riders come with it, and both are what keep it from being the hiding ADR 1035
refused:

- **`tools/state.sh` counts `departed` as its own figure**, beside `implemented` and `partial`,
  never folded into either. Folding it into `implemented` would hide the sentence; folding it into
  `partial` would go on counting a decision as debt. A decision stays visible as a decision.
- **The note's first sentence says what was departed from, and names the ADR that decided it.**
  "Decided against with its cost recorded" is a claim about a document somebody can open, so the row
  has to name one — `check` refuses a `departed` row whose note names no ADR, calibrated by a plant
  (trap 13).

## The aggregate rule is ADR 1035 section 5's

A `departed` row **owes nothing**: `Status::owes` is false for it, so it joins the five settled
statuses. It follows that a heading above one owes nothing on its account — `Ledger::is_aggregate`
reads `Status::owes`, so a parent flips to settled when its last unsettled child does, a `departed`
child counting as settled. `Problem::AggregateWithoutDebt` is the counterpart gate unchanged: a
heading owing what no descendant owes is still a finding. The rule reads no prose and does not claim
to (ADR 1035 section 4), which is also why the ADR-naming check is the shallowest test that can be
wrong in one direction only: it refuses a row naming no argument, and cannot judge whether the
argument fits.

## What this round moved

Fourteen rows whose entire remaining residue is a decided departure with its cost recorded, each
moved with its note's first sentence naming the departure and the deciding ADR: §7.4.2 (ADR 0036),
§7.4.8 (ADR 0036), §7.5.5 (ADR 1035 section 3), §7.10.2 (ADR 0098), §8.5.3.3.1 (ADR 1060), §8.6.5.7
(ADR 0272), §10.4.2.5 (ADR 0263), §10.7.5 (ADR 0028), §12.4.4 (ADR 0230), §12.4.4.1 (ADR 0230),
§12.5.2 (ADR 0304), §12.5.5 (ADR 0030), §12.5.6.19 (ADR 0239), §12.11.6 (ADR 0460). `partial` fell
from 123 to 109; `departed` is 14; `implemented` is unchanged at 558.

Six ADR-1035-era candidates were read and **left `partial`**, because a row with any undecided debt
is not a pure departure (§10.7.4, §7.4.6, §8.7.4.5.8, §12.6.4.15, §12.7.8.3.4, §11.3.7.3). §12.5.2's
own note carried a retired session-1040 sentence claiming its `/AF` and `/Lang` had no reader; the
reader — `structure::annotation_languages`, in the row's own `code` and tested — has existed since
session 1051, so the sentence was deleted rather than left to make the row look ownerless.

ADR 1035 is the rule this completes; `doc/questions/Q63` and `A63` are the question and the owner's
four words.
