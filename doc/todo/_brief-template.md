# The brief template — the fixed shape a batch's briefs are written from

Not a todo. Shared background for `doc/todo/02` section 8 step 2: the orchestrator writes every
batch's common brief and each slot's contract from this file, and from nothing else (ADR 1638).
Everything a round must know *every* batch lives in the tree — `doc/todo/02` section 0, the rule
block that opens `doc/environment.md`, `doc/traps/every-round.md`, `doc/habits/every-round.md` —
so the brief **points** at those and repeats none of them. A lesson the last batch taught is not
a paragraph here: it is made once as a line in one of those four places, or a trap or a habit
proposed in a report, and the brief names where it landed. The common part is at most sixty
lines; a slot's part at most twenty-five. `wc -l` both.

---

## The common part

```markdown
# Batch <n> — sessions <first>–<last> (read this, then your own slot below)

## 1. Where
One shared worktree `<path>`, branch `<branch>`. Never edit `<main checkout>` (read-only for you;
the owner's uncommitted `A` files and `doc/adr_revisit/` notes are read there). Do not commit or
push; the orchestrator merges. Scratch under `scratchpad/r<session>/` only.

## 2. Read first
`tools/round.sh <your kind>` prints the five every-round things and your kind's two or three
files; `doc/todo/02` section 0 is the contract and the rule block opening `doc/environment.md` is
the shared-machine law. Read nothing else up front; `doc/HANDOVER.md` is the index for a file
your contract needs and the list did not give you.

## 3. This batch
- Model: <name>. Numbers: ADRs <range>, Q<range>, one pair per slot, assigned below.
- The owner's answers since the last batch: <A files by name, where to read them, read-only>.
- Open questions nobody builds on: <Q numbers>.
- What changed in the tree since the last brief that every slot must know, as pointers:
  <"section 0 item 5 now says …", "environment rule block, line on …", "trap <n>"> — at most five.

## 4. The report
Rows moved (from → to) with the clause sentence; the premise that did not hold, if any; files
touched (full list); gates with exit statuses; what is unfinished and why; a proposed trap or
habit, if any; the record's `wc -l`; the duration as `<n> s`.
```

## A slot's part

```markdown
## Slot <k> — a "<kind>" round — session <n>

- **Contract**: <ledger rows by number, or the named build>. The clause is <§…> in `doc/md/`.
- **Premise and its evidence**: <each claim the contract rests on that the orchestrator checked,
  as the command run and what it printed, with its unit — "`grep -rn X crates` gives two lines,
  both tests" is the shape>. If the tree disagrees, do the real residue and say so.
- **Hypothesis**: <what the orchestrator believes and did not or cannot check by a command —
  where a cost lies, what a figure follows, how far a feature reaches — and what the round builds
  if it is false>. Tested first, reported as a premise is. Omitted when there is none (ADR 1748).
- **Owns**: <files and directories>. Siblings own: <slot → paths>; touch none of those. A build
  error in a crate you were not given is a neighbour mid-edit — wait and retry.
- **Reading beyond `tools/round.sh <kind>`**: <at most three files, each with the section>.
- **Gates owed**: tier 1 scoped to your crates; tier 2: <the lines section 2's map gives>, behind
  the lock. A figure a gate already holds is read from it, not re-taken.
- **Wall budget**: <n> h. At the budget, leave every file compiling and report what is done.
- **Numbers**: ADRs <a>, <b>; Q<n> only if the owner's word is needed.
```

## What the orchestrator does not write

- The last batch's lessons as prose. Each becomes one line in section 0, the environment rule
  block, or a proposed trap or habit — once — and the brief points at it.
- Any figure a command prints: ledger counts, gate counts, the number of passes, the record
  budget. `tools/state.sh` prints them; a round that needs one runs the command.
- Shared-machine rules in full. The rule block is the one copy; the brief names it.
- A contract's premise without its command. A premise checked by the orchestrator in the text,
  the code and the data before the brief is written costs minutes; one checked by six rounds
  afterwards costs each of them the same minutes. Most premises the records say did not hold were
  a mechanism or a feature's reach, which is a hypothesis and is briefed as one;
  `doc/reviews/1456/premises.py` counts them (ADR 1748).
