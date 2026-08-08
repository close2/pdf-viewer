# ADR 0232 — Where knowledge lives, when a handover is read by one round

Status: accepted, three-hundred-and-ninety-fifth session.

## Context

`doc/HANDOVER.md` was **3452 lines**. It states its own rule in its first twenty:

> **This file is the state of play, the traps and the habits** — where something is written
> elsewhere, this is a pointer, and the pointer is the whole entry.

and

> **A lesson lives here exactly once**: in a trap if it changes how you write code, in Habits if it
> changes how you work, in the numbers if it is a fact about today. A session's narrative belongs in
> its ADR and nowhere else.

It was halved in the fifty-ninth session and again in the hundred-and-thirtieth, to about 1500
lines. It had grown back to 3452, and the growth was not random. Sections, in size order:

| lines | section |
|---|---|
| 1072 | What to do next |
| 635 | Habits these sessions earned |
| 472 | Where we are |
| 397 | How the project got here |
| 301 | Traps — read these before writing code |
| 223 | Run it |
| 189 | Verify it |
| 60 | Things worth knowing |
| 54 | Environment |
| 24 | Crate map |

The project owner raised it directly: remove what is obsolete, **and extract what is still true but
is only needed by whoever is doing that particular thing**, so that a sub-agent can read it on
demand.

That is a stronger instruction than "shorten", and it names the real cost. A 3452-line file is not
expensive because it is long; it is expensive because **a round reads the whole of it to find the
tenth of it that its work is about**, and because every paragraph in it is a claim somebody has to
keep true. The three-hundred-and-ninetieth and three-hundred-and-ninety-fourth sessions each found a
gate row in this file wrong; the three-hundred-and-eighty-seventh found its longest-lived stale
claim at 364 sessions. **A claim nobody reads is a claim nobody checks.**

## Decision

**Two operations, kept distinct, because they answer to different rules.**

### 1. Extraction is a *move*, and the pointer is the whole entry

Ten files under `doc/`, each holding **all** of what it took — not a summary. A round that needs the
detail opens exactly one file and finds everything the handover used to say about that subject.

| file | took | why it is not read every round |
|---|---|---|
| `doc/history.md` | "How the project got here", 397 lines, plus the twenty-round block summary | narrative by the file's own definition; read when asking *when* |
| `doc/habits.md` | the six method sections, 635 lines | method for a *kind* of work: a clause read, a comparison, a gate, a measurement |
| `doc/ui-boundary.md` | §0, 278 lines | the specification of an interface, read when the interface is used |
| `doc/performance.md` | §3b and §4, plus the launch timeline and the `window` row, 385 lines | read before optimising or before quoting a number |
| `doc/oracle-and-corpus.md` | §3 and §3a, 180 lines | read when taking a page off a ranking |
| `doc/ledger-and-claims.md` | "The ledger"'s failure modes and §2 | read before writing or believing a row |
| `doc/third-party-data.md` | §1 | read before taking a dependency |
| `doc/verify.md` | "Verify it" minus the gate sequence | read when a change needs one instrument |
| `doc/running-the-viewer.md` | "Run it" | read when running the program |
| `doc/crate-map.md` | the crate table | read when looking for where something lives |

**Moved prose keeps its words.** A reader of the diff can tell a move from an edit, which is what
makes it possible to check that nothing was quietly rewritten while it was being carried.

**What stayed, and the test for it**: the state of play, the verified numbers, and the **traps** —
which are 301 lines and earn every one, because each is a mistake somebody actually made in this
tree and because they are cited by number from `crates/`, `ledger.toml` and a dozen ADRs. The rule
applied was: *does a round that does not yet know what it is doing need this?* The gate table, the
ledger counts, the traps and the shape of choosing pass it. A per-session table does not.

### 2. Every section number an outside reference names stays a heading

`CLAUDE.md`, `doc/PLAN.md`, `doc/RENDER_LIBRARY.md`, `doc/todo/*` and **dozens of ADRs** cite this
file by section — "`HANDOVER.md` §0", "§1", "§3a", "§4", "its 'Verify it'", "trap 12b". An ADR is a
record of what was decided at the time and **is not edited to follow a file that moved underneath
it**.

So §0, §1, §2, §3, §3a, §3b and §4 are still headings in `doc/HANDOVER.md`, and each is now a
pointer of three or four lines. Every existing reference resolves, unchanged, in one hop. The live
documents — `PLAN.md`, `RENDER_LIBRARY.md`, two todo files — were pointed one hop further on so that
a person following them lands on the detail rather than on the pointer.

**A reference that now points at nothing would be a worse outcome than a long file**, and keeping
the numbering is what costs nothing to guarantee it.

### 3. Deletion is for what is false, superseded or narrative — and it is a finding

Four claims came out because they were **false**, and each is recorded in the handover at the point
where it was wrong rather than removed silently:

- **`1369 tests + 1 doctest = 1370`.** The gate printed `1371 tests run: 1371 passed, 9 skipped`.
  Session 394 added exactly two `#[test]`s (`git show 2ea3aff | grep -c '^+.*#\[test\]'`) and moved
  none of the three numbers.
- **`5139 citations, 516 quotations`** in the gate table, **while a paragraph three lines above said
  5195** — the file disagreed with itself, and the gate prints **5215** and **520**. Two numbers
  written from two different rounds, neither re-read.
- **"Sandboxing the interpreter and rasteriser"** listed under *what is not implemented*, fourteen
  rounds after ADR 0218 built it and while the same file's "Where we are" described it in twenty
  lines. `doc/todo/34`'s own header says *built*.
- **"`doc/md/` … committed"**. `.gitignore` carries `/doc/md/` and `/doc/*.pdf`; what is tracked is
  the encrypted `doc/specifications.zip`. False since the three-hundred-and-eleventh session
  (ADR 0187), 84 rounds.

And one that was **stale rather than false**: "JPEG 2000 at a reduced resolution level, which now
waits on the decoder's API" — `67c996e`, the commit before this round, established that
`tmp/hayro` makes that a branch rather than a blocker, and `doc/todo/24` already said so.

**The narrative deleted** was the gate table's `tests` and `conformance` rows, which had become a
retelling of every round's counts back to the two-hundred-and-eighty-fifth — thousands of words of
session history inside a table whose job is to say what is true today. The `window` row was **moved
rather than deleted**, to `doc/performance.md`: it is not a gate and never was, but every figure in
it was taken by running the program through the loop no gate touches. The deleted rows' lessons are
all stated elsewhere: that a count is read off the gate and not derived is in the table's own preamble
and in `doc/habits.md`; that a ratchet's direction must be read before believing `FAILED` is in
`doc/habits.md`; the per-round movements are `doc/history.md`'s and each round's ADR's.

### 4. Two documents stating one command is how they drift, so one owns it

`doc/HANDOVER.md`'s "Verify it" and `doc/todo/02-every-round.md` §2 both listed the round's gate
sequence, and they had drifted exactly as predicted: "Verify it" said 1369 tests where the gate
printed 1371, and **never listed `render-quorra`'s corpus gate at all**.

**`doc/todo/02` §2 owns the gate sequence.** It is the round's own instructions, it is where the
`--profile gates` argument and trap 10's two build lines already live, and it is what a round reads
first. `doc/verify.md` owns everything else — `cargo deny`, the twelve fuzz targets and which need a
seeded corpus, the two cross-target checks, the callgrind counters, the census and ladder examples,
the AT-SPI recipe — and says so in its header. The same edit retired `doc/todo/02`'s own "the five
fuzzers", written one paragraph below a sentence naming twelve.

## Consequences

**3452 → 909 lines**, and the ten new files hold 2762 between them. The arithmetic is not a
reduction in words and is not meant to be: what changed is **how many of them a round has to read to
start**.

**Each new file carries a header block** — a status line, a "read by" line, and the pointer back —
in the style `doc/todo/*.md` uses, and every one is listed in a table in the handover's first twenty
lines, which is the first thing a reader meets.

**The risk this takes on is the one `doc/todo/README.md` already carries**: a third population of
prose that no sweep watches. `doc/todo/01`'s eighth sweep — every path a note cites, globbed — now
has ten more files to walk, and that is the mitigation rather than a hope, because it is a sweep
that already runs.

**What this does not change**: the handover is still the file `CLAUDE.md` sends a new reader to, it
still holds the numbers and the traps, and a round that reads only it can still run — the gates are
`doc/todo/02` §2's and always were.

**The measurement that would say this was wrong**: a round that needed something and could not find
it. The next few rounds are the test, and the cheap correction is to move a paragraph back.

## What was verified rather than assumed

Every number in the handover's gate table was read off a run of `doc/todo/02` §2 in this session:
corpus `974 documents in 6.7s: 0 unopenable, 8 locked, 2 encrypted beyond us, 5 pageless, 70
incomplete, 0 slow`; oracle 1794 pages, 1688 complete, 899/859 agree, 74/67 contradicted, 786/751
ambiguous, with the undiagnosed list empty; text `99.2% (24043/24243 words), 25 below 90%`; quorra
`957 pages compared: 920 agree, 36 differ, 1 refused, 17 not comparable`; dates 1514 of 1545; XMP 318
read and 1 refused; JPEG 2000 14 byte-identical; conformance 5215 citations, 520 quotations, 206
tables cited by the tree and 247 in the ledger's notes; the ledger 875 rows at 400/252/19/83/8/113.
**214 s** for §2 whole.

The fuzzers, `cargo deny`, the cross-target checks and the window were **not** run and are not
claimed. No crate under `crates/` was touched this round, so there was nothing for them to catch —
which is a reason not to run them, not evidence that they pass.
