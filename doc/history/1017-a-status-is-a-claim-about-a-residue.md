# 1017 — A status is a claim about a residue

Brief: make `partial` mean something again. `doc/reviews/1012-where-the-effort-goes.md` measured
that one of 875 rows changed status in 58 sessions, that 58 of 206 `partial` rows are headings
that cannot be worked on, and that 60 of the 148 leaves "are not gaps".

## What was done

Read all 206 `partial` notes (969 KB) against `doc/md/ISO_32000-2_sponsored_EC3.md`, then:

- **ADR 1035** states the re-statusing rule: a status is a claim about a clause's *residue*, the
  four settled words are the four ways a residue can be empty, principle 5's exclusions remove
  requirements from the population rather than from the clause, and a row is not moved on a guess.
- **Five rows moved**, each on evidence its own note already held: §8.6.4.4, §12.3.4, §12.5.6.6,
  §12.7.3 → `implemented`; §12.5.6.17 → `out-of-scope` (`clause-13-multimedia`).
- **`Ledger::is_aggregate`, `Ledger::owing`, `Status::owes`** in `tools/conformance/src/ledger.rs`:
  a heading is a row that still owes something and has a descendant row that still owes something.
  Mechanical, reads no prose. The gate prints the debt beside the status counts.
- **`Problem::AggregateWithoutDebt`** gates the counterpart: a row owing what no row below it owes.
  Fires on nothing today; five unit tests pin both directions.

## Numbers (from a command over the tree; sibling rounds move them under it)

- **159 of the 218 unsettled rows owe a debt of their own; 59 are headings.** The gate prints both.
- Of the 143 `partial` leaves: **106 are real viewer debt** (89 workable here, 11 needing a host
  surface, 6 blocked on a document or an upstream), **20 are deliberate departures with the cost
  measured**, **7 are sibling aggregates**, **10 do not name their own residue**.

## The finding the brief did not predict

The review's class E is mostly **not** mis-statused. Twenty rows record a `shall` this tree decided
against with its cost written down, and under the ledger's own definition those are correctly
`partial`. What the ledger lacks is a **word** for "decided against, one sentence inside a clause
that is otherwise implemented" — `out-of-scope` and `inapplicable` say it for a whole clause and
nothing says it for a residue. ADR 1035 §3 leaves that to the owner and forbids re-statusing a
departure into a settled word meanwhile.

Two of the review's named examples do not survive reading: §12.5.6.3's "nothing is owed" is
"nothing is owed **for rendering**", and setting an annotation's `/State` is `CLAUDE.md`'s scope;
§14.11.6.2's three writer-side entries sit beside a reader's `shall` its own note names.

## Gate

`cargo test -p conformance` passed with the new rule and the new print;
`every_quotation_is_the_standards_own_words` fails on `crates/pdf-model/src/structure.rs` and
`measurement.rs`, sibling rounds' live edits. Clippy and fmt clean. Re-run later the ledger would
not parse at all — round 1015 left an unescaped `"` in §12.5.6.7's and §12.5.6.9's notes while
writing them. Reported rather than repaired; with those two lines neutralised in a copy the file
parses, this round's five rows are as written, and the aggregate gate still fires nothing.
