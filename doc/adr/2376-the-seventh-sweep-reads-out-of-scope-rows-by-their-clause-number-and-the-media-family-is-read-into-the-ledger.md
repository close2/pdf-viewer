# 2376 — The seventh sweep reads `out-of-scope` rows by their clause number, and §13.2's family is read into the ledger

Session 1771, the clause slot of batch 117. Status: accepted; the moves are authorised by the batch's
brief under ADR 2370's rule 2, and the instrument answers trap 268. ADR 2377 is unused.

## Context

Review 1768 found twenty-one rows of §13.2 standing `out-of-scope` on clause 13's exclusion while
`pdf_model::media` cited their clauses and judged them for §13.2.2's viability (ADR 1960). Nothing
had read them: the seventh sweep, `--bin inapplicable`, took its population from `inapplicable` rows
alone, and its cousin check skipped `out-of-scope` rows too. Trap 268 states the rule — a sweep for a
decayed reason reads every status that can decay, and a clause citation under a row saying the
clause is not this program's is a finding whatever the row's status.

Widening the population alone would not have named these rows. An exclusion's notes are written to
one template: each names `CLAUDE.md` and the entry and quotes one sentence of its clause, so every
`out-of-scope` row states the same identifiers, all of them shared vocabulary of high reach.

## Decision

1. **The sweep reads `inapplicable` and `out-of-scope` rows** (`conformance::inapplicable::SWEPT`),
   and a cousin is a row holding neither status.
2. **Every swept row states its own clause number as a term** (`Kind::Citation`), named by a source
   holding `§` and the number with nothing after it that continues a clause number. The count of
   swept rows whose clause a source cites is printed first (`Report::cited`). The citation is added
   by `sweep`, not by `terms_of`, because the fourteenth sweep reads the same prose for the names a
   debt would have, and a row's clause number is no such name.
3. **The family's rows are read sentence by sentence against `media.rs`** and moved to the status the
   reading makes true: twelve to `implemented`, five leaves to `partial` on debts each note names,
   and four headings with §13.2 to `partial` as their aggregates. A `partial` here is not the parking
   ADR 2370 forbids; each names a verdict the code gives that the clause contradicts.
4. **Software with no name matches no software identifier, and §13.2.7.4.2 and §13.2.7.4.3 are
   `implemented` on that reading.** The two pieces of software this program asks about — the player
   this machine starts for a content type, and this program for Table 279's `/V` — have no software
   name, so §13.2.7.4.2's first step fails for every `/U`, and no scheme, unescaping or version
   comparison can change the verdict. **The trigger that reopens it**: a host stating its player's
   software name, which makes those steps and §13.2.7.4.4's comparison decide something.

## Consequences

On HEAD's ledger the widened sweep reads 118 rows (34 `inapplicable`, 84 `out-of-scope`) and finds 31
whose clause a source cites, ten of them among the twenty-one. Of the eleven it misses, three are
headings and five are leaves whose reading found a debt or no requirement; three — §13.2.4.1,
§13.2.6.1 and §13.2.7.4.2 — are executed under a table's number or a neighbour's citation, which the
term does not read, and a table term taken from the clause's own captions is the next widening.
Every swept row is still a finding on the template's shared words, printed last by reach, as the
sweep has always printed noise. The `partial` rows' debts are two
viability verdicts in `media.rs` (an image clip's must-honour offset, and an `/MU` of non-viable
player infos), reported to the round that owns that file rather than fixed here.
