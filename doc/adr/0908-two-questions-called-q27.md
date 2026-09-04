# 0908 — Two questions called Q27, and the number a round is allowed to read

Session 934. Status: **accepted**. The round's other record is
[ADR 0909](0909-a-memory-figure-guarded-by-a-clock.md).

## Context

`doc/questions/` has one rule and its whole index rests on it: a question is a `Q` file, the owner
answers it with an **`A` file of the same name**, and a `Q` with no matching `A` is open. Nothing
else has to be read to know what is waiting.

On 2026-09-04 two rounds took the number 27. Session 926 asked
`Q27-a-font-the-file-does-not-carry` on `main`; session 927 asked
`Q27-cost-floors-for-the-other-seven-walks` on a branch of its own, the same day. Neither round did
anything wrong — each read `ls doc/questions/`, which is what the README asked for, and each read a
directory in which 26 was the highest number. Their branches could not see each other, and the two
files met for the first time in round 929's merge. Round 929 found the collision, declined to fix
it unilaterally, and wrote it into `Q28`; round 931 merged that finding onto `main` and named the
round that merges 929 as its owner. That is this round.

**Nothing dangled**, which is why it survived three rounds of gates: every reference to either
question cites a *filename*, so every link resolved and `--bin pointers` had nothing to say. What
was broken is narrower and worse. **One of the two could never be answered.** An `A27` would name a
question and not say which, and the folder's index — a `Q` with no `A` is open — would be a
statement about a pair.

## Decision

### 1. The font question moves, and it moves to a number out of this round's own block

`Q27-a-font-the-file-does-not-carry` becomes
[`Q35-a-font-the-file-does-not-carry`](../questions/Q35-a-font-the-file-does-not-carry.md), and
`Q27` is the cost-floor question alone.

**Which one moves was decided by counting the sites that name it**, because a renumber's whole cost
is the references it falsifies. The font question is named in eight places — three of them markdown
links, in ADR 0893, `doc/history/926` and `doc/todo/03` — and the cost-floor question in seventeen,
across ADRs 0895, 0898 and 0899, `doc/history/927`, `doc/history/929` and `Q28`, not one of them a
link. Eight against seventeen, and the eight include every link either has, so moving the font
question is both the smaller edit and the one that repairs rather than merely renames.

**The new number is 35 because it is the only number this round could know was free.** Sessions 932
and 933 were running beside this one from branches this tree cannot read, and `Q30` to `Q34` are
theirs to take if they need them; 35 is in the block this round was given. Taking `Q30` — the next
free number by `ls` — would have been the identical mistake one directory later. **The gap between
29 and 35 is correct**, and the README now says so, because a round that tidies it reintroduces
exactly the reasoning that caused this.

### 2. Every site was moved with it, including two history files

ADR 0893's link and sentence, `doc/history/926`'s link and sentence, `doc/todo/03`'s link, `Q28`'s
collision paragraph and `Q29`'s aside. `doc/todo/02` §6 says a round writes into no other round's
history file, and that rule is about **bookkeeping** — a round recording its own work where the
next reader will not look for it. A link that no longer resolves is not bookkeeping; it is the
document being wrong. So the two history files were repaired in place and each says, in one clause,
that the number moved and why, and `Q35`'s first paragraph carries the argument once so that the
other seven sites do not have to.

`Q28`'s paragraph was **amended rather than deleted**. It is the record of the collision as round
929 found it, and it now says what settled it. The lesson it drew is the one this ADR keeps: *a
counter shared between rounds needs an allocator, and `ls doc/questions/` is not one when two
branches cannot see each other.*

### 3. The convention gains an allocator, and the tree gains a check

The README now states two things it did not:

- **A number comes from the round's own reserved block, never from `ls`.** That block is the one
  counter every round can see, and it is outside the tree — which is why the tree cannot enforce
  it.
- **A collision cannot reach `main` unnoticed.** `tools/conformance/tests/questions.rs` fails when
  two `Q` files share a number, when two `A` files do, when an `A` file answers a question that is
  not in the directory, when an `A` file's slug has drifted from its `Q` file's, and when a name
  here is not `<letter><number>-<slug>.md`. It runs under `cargo test -p conformance`, which is the
  last line of `doc/todo/02` §2's sequence and which **every merge round runs** — and a merge is
  precisely where this defect is made.

## Why a check rather than a stronger rule

The instruction that raised this asked for recurrence to be made *impossible rather than unlikely*,
and it is worth being exact about which half each mechanism buys.

A duplicate cannot be made impossible from inside the repository: two branches genuinely cannot see
each other's files, and any scheme derived from the directory's contents is the scheme that already
failed. What *can* be made impossible is a duplicate **surviving the merge that creates it**, and
that is what the checker does — the collision becomes a red gate on the day it is made, in the run
the merging round is already obliged to make, rather than a thing three rounds notice and one round
is eventually told to own.

The other half — making it unlikely in the first place — is the reserved block, and it is a
statement about how rounds are handed out rather than about this tree. Writing it into the README
is what a document can do: the next round reads "never from `ls`" where it used to read a rule that
told it to.

**A name-derived scheme was considered and rejected.** Numbering a question by the session that
asked it (`Q926-…`) would be collision-free by construction, because session numbers are allocated
outside the tree already. It was not taken: it renumbers all thirty existing files, breaks every
reference in `doc/`, and it buys nothing the checker does not, since the failure mode it prevents is
now a red gate either way. A convention with thirty files in it is not changed to avoid a defect
that has occurred once and is now caught.

## What it was run against

Trap 13: the checker was run against the defect before it was believed. With the duplicate restored
it fails naming both files and the number; with an `A29` whose slug had drifted, an `A31` answering
nothing, and a `Q-no-number.md` beside them it reports all three, one line each. The clean tree
passes and prints the thirty questions with a number apiece and which of them are open, which is
what a passing run of a document checker is for.
