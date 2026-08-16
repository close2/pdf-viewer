# ADR 0388 — The seventh sweep becomes a program, and the two ADRs a correction stopped short of

Status: accepted, 2026-08-16.

## Context

`doc/todo/01`'s seventh sweep is the only one that reads `inapplicable` rows. Every other sweep
walks the statuses that *owe* something, which is exactly the property that let five wrong reasons
sit in this status undisturbed until the three-hundred-and-fifty-ninth session read them (ADR
0205): §14.11.3's printer's marks and §14.11.6.2's trap networks were "a screen is not a printer"
while `PrinterMark` and `TrapNet` had been in `annotation.rs`'s `STANDARD_SUBTYPES` from the start,
and §12.5.6.20's and §12.5.6.21's own rows said so.

It has been a description for nine runs, and the descriptions' numbers are the argument for
building it: 25 of 83, 64 of 83, 72 of 83, 71 of 83, 27 of 80, 66 of 80, 43 of 80, 61 of 80, 57 of
80, 47 of 80 — over rounds in which the population itself barely moved. Every one of those levels
was set by a **stop-list a session wrote from memory**, so no run could be compared with the run
before it. That is ADR 0360's argument for the caller sweep, arriving at the sweep whose numbers
swung hardest.

## Decision 1 — `cargo run --release -p conformance --bin inapplicable`

**Taken.** `conformance::inapplicable`, the eleventh of the fifteen sweeps to be a committed
program, beside `entries`, `quotations`, `unread`, `blockers`, `capabilities`, `retired`,
`callers`, `pointers` and `tables`.

The population is every `inapplicable` row. The vocabulary is the row's **own** title and note —
the point of this sweep is that the row supplies both the claim and the words to test it with —
and the question is whether any Rust source under `SOURCE_ROOTS` names them.

### The two judgements the program takes over from the person

**Rarity replaces the stop-list.** `doc/todo/01` states the discriminator in one sentence — "most
are noise (`DeviceCMYK`, `XObject`, and the sweep's own English — `Nothing`, `Whether`), and the
signal is a *rare* word" — and each hand-run implemented it by guessing which words are common.
`Term::reach` is the count of naming files, taken from the tree the sweep is asking; terms print
rarest first, rows print by their rarest term, and **nothing is dropped**, so a word a person would
have excluded is demoted rather than hidden. The level is now a property of the ledger and the
tree rather than of the session.

**A cousin row, which is where this sweep's findings have actually been.** All five defects of its
first run were the seventh failure shape — two rows about one mechanism, disagreeing, one naming a
*capability* and the other naming *code* — and those rows are cousins rather than parent and child,
which is why the arithmetic sweep cannot see them. Each named term now carries the rows that are
**not** `inapplicable` or `out-of-scope` and say the same word: the ledger holding both answers at
once, printed as the pair it is.

### And the extraction takes two derived rules where a list would do

The seventh sweep's own noise has always been English capitalised at the start of a sentence. Two
rules remove it without knowing a word of English:

- a capital that is **not the first letter** makes a word an identifier wherever it stands —
  `DPart`, `TrapNet`, `GoToDp`, `RGB`, `STANDARD_SUBTYPES`, `Command::Zoom` — which no English word
  has;
- a plain `Capitalised` word is an identifier only where it does **not** open a sentence, so
  "a `Rendition` annotation" is one and "Nothing is read here" is not.

Between them they took the first run from 440 stated terms to 305 with no list of words anywhere in
the program. What survives is judged on reach, not on somebody's memory.

**Not a gate**, for ADR 0249's ratio reason unchanged: most of what is left is the standard's shared
vocabulary, and no tighter predicate removes it without removing the rare words too.

## Decision 2 — §14.5's `inapplicable` reason is the retired exclusion, and the conclusion is kept

**The first run's one defect.** §14.5's page-piece dictionaries were `inapplicable` because
"[n]othing here writes a PDF, so there is no private data to recognise" — the wording `CLAUDE.md`
amended in the hundred-and-thirty-seventh session, in a tree that has written §7.5.6's incremental
update since the hundred-and-thirty-fifth, and forty-three rounds after ADR 0345 corrected the same
sentence in the ledger's own *generated* header. The sweep found it because `tests/saving.rs` names
`/PieceInfo` — under a row saying nothing here writes.

The status holds and the argument is replaced, which is `doc/todo/01`'s fifth shape. What the clause
actually says is narrower and does not depend on whether this program writes: a data dictionary
holds "private data needed by the PDF processor" *that produced it*, keyed by that processor's own
name, and this one has produced none. Nothing is owed by the writing this program does do — Table
351's required `/LastModified` binds a data dictionary a processor writes, and the clause's one
`shall` addressed to a reader ("modification dates shall be compared only for equality and not for
sequential ordering") binds a processor comparing them against private data of its own.

## Decision 3 — two ADRs carrying a claim a later round disproved, amended in place

This round was pointed at a hazard rather than at a population: **eight sessions since the last
sweep corrected earlier ADRs in place**, and the sweep before this one found that six of nine
document defects were a round's own correction that stopped at the code. So the fourth sweep was run
over those rounds' own nouns, and it paid twice — both times on the shape ADR 0265's rule exists
for, a claim a later round disproved and left standing in the document that made it.

| document | said | the round that disproved it | where the correction lived |
|---|---|---|---|
| ADR 0382 §6 | "**The escape hatch is complete and needs nothing from upstream**" — a presenter rendering into a `Target::Texture` it owns | ADR 0383, one round later: presenting that texture needs the surface, `Device` keeps it private, and configuring one of a host's own needs a format only a `&wgpu::Adapter` gives | only in ADR 0383, for five rounds |
| ADR 0384 §6 | "**It cannot be worked around from here**", of the reprojection an atlas repack costs | ADR 0385: every clause of it is true about *capturing* and a non-sequitur about *drawing* — the base was thrown away by `Stale::settled`, not by the repack | only in ADR 0385, for three rounds |

Both are struck and re-argued in their own documents, with the correcting ADR named. The first is
the sharper of the two because the sentence is a *decision's* conclusion: a reader of 0382 who never
opens 0383 comes away believing this tree has a working escape hatch it does not have.

## Decision 4 — a count of the transition styles, wrong in two rows because the ADR they came from says both

**From the blame band, and it is the seventh failure shape inside one document.** ADR 0230 says
"[t]he other five are named and **reported by name**", and twelve lines later, under its own table,
"`R` is therefore not reported and the other four are." §12.4's row and §12.6.4.15's row were both
written from the first half and both said five. The code has always agreed with the second half:
`transition::note`'s doc comment says outright that "`R` is the one style with nothing to report,
because Table 164 defines it as the cut", and the `match` reports `Blinds`, `Glitter`, `Dissolve`
and `Fly`.

Seven drawn, four reported, and `R`, which is the cut a file asked for. All three places say that
now, the ADR included.

## What this does not change

No raster, no verdict and no clause. The sweep is a reading list; §14.5 keeps its status; the two
ADR amendments and the two row corrections are prose about what is already true of the code.

## Clauses

**None newly implemented.** §14.5 stays `inapplicable` with its reason corrected, §12.4 and
§12.6.4.15 stay `partial` with their counts corrected, and nine rows of the commit-534-to-536 blame
band are read and kept with the evidence that kept them recorded in their notes — which is what
moves the blame pointer without a stamp.
