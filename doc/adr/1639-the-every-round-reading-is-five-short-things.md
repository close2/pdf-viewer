# 1639 — The every-round reading is five short things, and the rest is opened by kind

Session 1401. Status: accepted. Code: `tools/round.sh` (`every_round_reading`, `kind_reading`,
`--lines`). Prose: `doc/HANDOVER.md`, `doc/todo/02` section 0, `doc/environment.md`'s opening
rule block, `doc/traps/every-round.md`, `doc/habits/every-round.md`, `doc/habits.md`,
`doc/traps/README.md`'s status line. Companion: ADR 1638.

## 1. The question

What must a round read before it writes anything, and what may it open only when its work reaches
it? Before this ADR the every-round list was six files and the brief — 2 748 lines — and a kind's
list added 577 to 4 272 more; `doc/reviews/1401` measured that the trap index's 126 rows were
cited five times in 49 records while its ten most-cited rows carry 78% of every citation ever
made, and that the brief's 155 lines of lessons were read by six rounds a batch and changed
nothing a round did. ADR 1036 split the traps into an index and group files for the same reason;
this takes the next step the count supports.

## 2. The shape

**Every round reads five short things, and `tools/round.sh` prints them**:

1. `CLAUDE.md` — the principles; unchanged, the owner's.
2. `doc/todo/02` **section 0** — the round on one page: the reading order, the contract, tier 1
   scoped to what was touched, the record, the ledger-edit rules, the report. The rest of that
   file is the argument and the lookup, opened where section 0 sends a round.
3. The rule block that opens `doc/environment.md` — one line per shared-machine rule, each naming
   the section or trap that argues it. The incidents stay below it, unrewritten.
4. `doc/traps/every-round.md` — the ten rows that carry four-fifths of the citations, verbatim
   from the full index, chosen by `tools/state.sh traps` and re-chosen when that count moves.
5. `doc/habits/every-round.md` — ten pointers to the habits the records show rounds paying for,
   each naming its file and heading.

Then `tools/round.sh <kind>` names the two or three files that kind cannot work without — its
trap group, its habit file, the one document that states what it changes — and the contract names
the rest. `doc/HANDOVER.md` is the index for a file neither gave, rewritten to be opened, not read.
`tools/round.sh --lines [kind]` prints the list's length, which is the figure the next review
reads.

## 3. What is kept whole

- `doc/traps/README.md` keeps every row and its number; `tests/traps.rs` reads it unchanged, both
  ways. The every-round file holds copies of ten of its rows, and a change to a row is made in the
  full index — the short file says so.
- The six habit files keep every habit; the every-round file is pointers.
- `doc/environment.md`'s incidents and `doc/todo/02`'s sections 1–8 are not shortened by
  deletion: a rule block and a section 0 are put in front of them. A trap or a habit is an
  incident with a rule attached and the history is the lesson (`CLAUDE.md`, "Where knowledge
  lives"); what this ADR cuts is how much of it a round reads up front, not what the tree keeps.
- `doc/todo/02` section 2's gate lines are untouched, so `state_sections` reads the same sequence.

## 4. The cost

A round that does not open the group file for a trap outside the ten springs it unread. It did
before — the index was read and cited five times in 49 records — and a contract that touches a
trap's position names it in the slot part (ADR 1638). The ten are re-chosen from the count, not
from memory: when `tools/state.sh traps` shows a row outside the ten climbing, the file changes.

Four kinds the old script did not know — `script`, `writer`, `archive`, `fuzz` — have lists now;
their third file is long (an RFC, `doc/verify.md`, `doc/pdf-a-mitigations.md`) and is opened at
the section the line names.
