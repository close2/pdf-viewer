# 1416 — A done todo file that something cites is kept whole, and listed apart from the owed work

Session 1289. Status: **accepted**. Amends `doc/todo/README.md`'s rule that a done item's file is
deleted, for the case the rule did not foresee and eight files already met.

Context: `doc/todo/README.md` (*What a file holds*, the index), the eight files whose `Status:` says
their item is done — `12`, `13`, `14`, `25`, `26`, `37`, `39`, `53` — and `CLAUDE.md` *Where
knowledge lives* ("deleting one is the only unrecoverable mistake a round can make").

## The question

`doc/todo/README.md` said that when an item is done its file is deleted and its argument lives on
in an ADR. Eight done files were not deleted, each because a comment under `crates/` or `tools/`, a
ledger note or `CLAUDE.md` points a reader at it. They sat in the index of owed work, beside the
sentence "What is not implemented has a file, and that is what this index is", which they made
false. The question put to this round: is a file that says "done, see ADR N" a pointer rather than
a todo, and should it be reduced to its header block pointing at the ADR?

## What the eight are

None of them says only "done, see ADR N". Each is between thirty-five and four hundred and forty
lines of argument: a clause reading (`13` is the reading `CLAUDE.md`'s clause 10 entry quotes as
where "the standard's own words under each clause number" are), a measurement and its method (`12`,
`14`), the argument a `departed` row cites (`26`), a list of residues and what changed each answer
(`53`). `12`'s header claims that every *decision* in it is in the ADRs it names; no file claims
that every *paragraph* is, and none has been checked paragraph by paragraph against its ADRs.

## Decision

1. **A done todo file that something outside `doc/adr/` cites is kept whole.** Its header's
   `Status:` says it is done, and a `Cited by:` line names what cites it, which is the reason it is
   kept — so a round deciding whether it may go reads the reason in the file rather than grepping
   for it.
2. **It leaves the index of owed work.** `doc/todo/README.md` lists it in a second table, *Done, and
   kept because something cites it*, so the first table is again exactly what is owed and a round
   choosing work does not choose from the second.
3. **Reduction to the header block is allowed only where every paragraph of the body is shown to be
   held by an ADR the header names.** A header pointing at an ADR is the right shape for a file that
   is only a pointer; for a file holding a reading nobody else holds, it is a deletion of that
   reading behind a working link. No reduction was made this round, because none of the eight has
   had that check.
4. **A done file nothing cites is still deleted**, as the README has always said.

## What this costs

The todo directory keeps files that are not todos, and the index has two tables. The alternative —
moving each reading into a new ADR and repointing every citation — rewrites comments across crates
to follow a document that moved, and an ADR written to hold another file's prose is a record of
nothing that was decided.

## What would change the answer

A round that verifies one of the eight against its ADRs and finds every paragraph held may reduce
it to its header block under point 3, and says so in its record.
