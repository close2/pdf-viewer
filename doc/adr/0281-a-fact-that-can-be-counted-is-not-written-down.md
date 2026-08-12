# ADR 0281 — A fact that can be counted is not written down

Date: 2026-08-12 (session 446)
Status: accepted
Supersedes nothing; extends ADR 0232, which split `doc/HANDOVER.md` into ten files by *topic*.

## Context

The project owner asked for `CLAUDE.md` to be adapted and simplified, and gave the reason as an
argument rather than a preference:

> A round that measures should not have the possib[ility] to derive from our md files. When we ask
> a subtask to read our .md files, we have far too much information there. […] Is it really
> important to know which round did what? […] why do we need to document, that seven of Annex O's
> eleven shalls are carried out instead of providing a way to count them. This is information not
> relevant to any subtask.

The evidence was checkable and checks out. `doc/HANDOVER.md` was halved to 909 lines by ADR 0232
and stood at **1845** fifty rounds later — it had doubled, and the growth was almost entirely one
shape: a gate table whose cells had become a running chronicle of every round that touched them,
and a "Where we are" that dated each capability to the session that built it. **816 of those 1845
lines** — the whole span from *The gates, today* to *The ledger* — were per-round narrative.

The cost is measurable in the same place. The four-hundred-and-forty-fifth was a closing round
whose entire job was verification, and it spent itself correcting **28 stale numbers across 11
files**; six of the preceding rounds each caught one. Every one of those was a *derived* fact
written beside the gate that derives it, and the sharpest case was arithmetic: `doc/todo/02` §2
carried "1550 + 1 = 1551" while the test count two lines above it read 1619, because a round that
copies a gate's own number into the line it is told to update leaves the sum beside it untouched.

`CLAUDE.md` carried the standing example the owner named. It said seven of Annex O's eleven
`shall`s were carried out and four reported; `doc/todo/39` and the ledger have said eight and
three since session 414. Nothing in the tree could see the disagreement, because **no gate reads
a sentence** — every gate ratchets a count.

And there is a second cost that no correction round can pay off: a subagent given a round to
*measure* something can read the answer in a document instead of running anything. A table of gate
figures is not merely stale, it is an invitation to write "unchanged".

## Decision

**A fact that can be counted is not written down. What is written down is the command that counts
it.**

The rule binds *derived* facts — counts, rates, gate results, "N of M", session numbers, dates. It
binds nothing else. The principles, the argument for a decision, a trap, a clause reading and a
measurement that justifies a constant are not derivable by any command, they are the project's
memory, and deleting one is the only unrecoverable mistake this restructuring could make. Where a
number is the *anchor* of a lesson — "this claim survived thirty-two sessions", "the references
disagree at worst-tile 26–28 on text pages" — it is part of the argument and it stays.

### 1. One command prints the state

`tools/state.sh`, with named sections and two speeds: `quick` (ledger, conformance, Annex O,
populations on disk, binaries, build directory) in seconds, and the whole thing — every gate
`doc/todo/02` §2 runs — in minutes.

**It is a shell script rather than a Rust binary**, and the reason is the same one that makes it
useful: its whole job is to run other programs and show what they said. A Rust binary would put a
compile between a question and its answer, would have to be built and installed into `target/`
before it could report anything, and its source would stop being a readable list of the commands
these documents used to state in prose.

**It performs no arithmetic.** Each section runs a command and prints the lines of that command's
own output that are its summary, chosen by a regular expression that never rewrites one. That is
the whole defence against the failure the four-hundred-and-forty-fifth found: there is no derived
number for a round to leave behind, because the script derives nothing.

Two sections are not gates and are worth naming. `annex-o` answers the owner's example by reading
`Parameter::unhonoured` — the variants reaching `return None` are carried out, the arms after it
are the refusals, and each arm carries its own reason, so the script prints them verbatim rather
than counting them and describing them. `counts` is a handful of `ls` and `find` invocations for
the populations documents used to state: the fuzz targets, the ADRs, the open todos, the corpora.
The answer is on the disk rather than in a sentence about the disk.

### 2. Session bookkeeping lives in `doc/history.md` and nowhere else

`doc/history.md` already exists, already holds one row per session, and is read by no round doing
work. An instruction file citing a session number to **date** a claim is bookkeeping and moves
there; one citing an ADR for an **argument** is a pointer and stays.

### 3. The instruction files are split by *reader*, not by topic

ADR 0232 split by topic, which was right and is why ten files exist. What it did not do is tell a
round which of them it needs, so the handover kept a précis of each — and a précis is a second
copy to keep in sync. `doc/HANDOVER.md` now opens with two tables: what *every* round needs, and
then one row per kind of round — reads a clause, judges a page, **measures anything**, writes a
host, takes a dependency, runs the program, asks *when*. That question is what decides where a
paragraph goes, and it is the one that reduces a subagent's context.

Two files are new because `CLAUDE.md` should hold principles and nothing else:
`doc/stack.md` (the stack table and why `rustybuzz` is not in it) and `doc/environment.md` (the
working agreements, the machine, the agent's account, the `Xvfb` recipe, the build directory, and
the one command a fresh clone needs).

### 4. An index is one line per item

`doc/todo/README.md`'s cells had grown into paragraphs restating each file's own header block, and
that is where the longest-lived false claim this project has recorded lived: the index said §10.5's
transfer function was ignored and `silent` for **eighty-seven rounds** after it was implemented.
The cells are one line now. Each file's header block carries its status, its witnesses, its clauses
and its code, and that is the authority: read the line to choose an item, open the file to take it.

## Consequences

**What this costs.** Somebody who wants a number now runs a command instead of reading a sentence,
and for the corpus-scale gates that is minutes rather than seconds. That is the intended trade:
the sentence was wrong 28 times in one round's audit, and the command has never been.

**What it does not cost.** Not one trap, habit, clause reading or argument was deleted. The five
shapes a refusal takes when it has outlived its reason moved from `doc/HANDOVER.md` into
`doc/habits.md`'s ledger section, where a clause round reads them, and they moved verbatim. Every
trap kept its number so that the several dozen source comments citing "trap 8 in
`doc/HANDOVER.md`" still resolve.

**The reference check is part of the work, not a courtesy**, which ADR 0232 established: it moved
sections and kept every heading an outside reference named, so no ADR had to be edited. This round
could not keep every heading, because the point was to delete some. So the live documents were
grepped for each removed heading and pointed one hop further on — nine documents and two source
comments — and every relative link in `CLAUDE.md`, `doc/`, `doc/todo/` and all 280 ADRs was
resolved against the filesystem. ADRs describing what a file *used to* hold are history and were
left alone, which is the same rule the ledger uses for a row that records a decision.

**What could go wrong, and the tell.** A round that wants a number and does not want to wait will
copy one out of `doc/history.md`, which is exactly the file that legitimately holds last round's.
The defence is that history rows are dated by session and read as such, where a gate table reads
as *now*. If that turns out to be too weak, the next step is not a table — it is `tools/state.sh`
writing its output to an untracked file with a timestamp, so that a stale answer says how stale it
is.
