# ADR 0428 — A round reads what it is about, and runs the gates its change can reach

Date: 2026-08-18 (session 593)
Status: accepted
Extends ADR 0232, which split `doc/HANDOVER.md` into ten files by *topic*, and ADR 0281, which
took the counts out of the instruction files and split them by *reader*. This one finishes the
second: it splits by **what the round is doing**, and it applies the same question to the round's
*gates* rather than only to its reading.

## Context

The project owner measured a round on this machine, on a warm tree, and the numbers are the whole
argument for what follows:

| step | cost |
|---|---|
| `fmt` + `clippy --workspace --all-targets` + `nextest --workspace` + doctests | 37 s |
| the eight corpus-scale gates, `pdfref-hayro`, and the conformance gate | 120 s |
| **all of `doc/todo/02` §2** | ~2.6 min warm; ~4 min after touching `pdf-model` |
| §5's release binaries (fat LTO, after touching `pdf-model`) | 95 s |
| a whole round, seventeen rounds measured | 24–82 min, mean ~50 |

**So §2 is about eight per cent of a round, and the time is somewhere else.** It is in what a round
*reads* before it can start and in which instruments it *chooses*. `doc/HANDOVER.md` was 882 lines
and `doc/todo/02-every-round.md` 454, and every round has been reading both in full — a round about
a ledger row reading fifteen traps about rasterisers and parsers, a round about a rasteriser reading
the sweep catalogue for a job it is not doing.

The owner's instruction was two sentences: *"Split HANDOVER and other files so that subagents really
only get to read what they need"*, and *"maybe even create a script, which prepares the worktree"*.
Beside them, two decisions: §2 relaxes into a change→gate map with the full sequence every fifth
round, and §5's binaries are rebuilt every fifth round and before any measurement.

**`doc/HANDOVER.md` already had the right idea and was undone by its own body.** Its opening table
says which file each kind of round opens; then the file itself carries the state of play, fifteen
traps, the run and verify recipes, the crate-map pointer, "things worth knowing" and a history
pointer — so the table names a hop the reader has already paid for.

## Decision

### 1. The handover is an index and nothing else

125 lines, down from 882. What it holds is the rules that bind every round whatever it is about —
where a lesson lives, and that the round's record is one *new* file — and two tables: what every
round reads, and what a round of a given kind reads beyond that.

Everything else moved, and **moved is the word**: `doc/state-of-play.md` took "Where we are"
verbatim, `doc/traps/` took the traps, and four paragraphs that were second copies of something
already stated elsewhere were merged into the file that owns the subject.

### 2. The traps are grouped by what a round is doing, not by their numbers

This is the part that matters, because **the traps are the most valuable thing in the file** and a
round that skips the relevant one repeats a mistake somebody paid for. Grouping them by number is
grouping them by the order in which they were discovered, which is a fact about this project's past
rather than about the round in front of you.

| file | opened by a round that | traps |
|---|---|---|
| `doc/traps/pixels-and-rasterisers.md` | can change a pixel | 1, 2, 6, 12b |
| `doc/traps/oracle-and-references.md` | reads a verdict or invokes another renderer | 3, 9, 12 |
| `doc/traps/parsers-and-streams.md` | touches a parser, a filter, a font program or a codec | 4, 5, 8 |
| `doc/traps/the-interactive-loop.md` | turns a press into a command, or converts between spaces | 12a |
| `doc/traps/instruments-and-reports.md` | runs a gate, believes a number, adds a report — and any round writing Rust, for trap 7 | 7, 10, 10a, 11 |

Each group file also took the standing facts about its own machinery — what was "Things worth
knowing", now beside the traps about the same code. The three that are about *reading the
specification* went to `doc/habits.md`'s section of that name, which is where a clause round already
goes.

**Every trap keeps its number, and `doc/HANDOVER.md` keeps a fifteen-row index of them.** That is
ADR 0232 §2's rule and it is not a courtesy: `crates/`, `tools/`, `doc/conformance/ledger.toml` and
dozens of ADRs cite these by number — forty-two references name the handover, several by trap — and
an ADR is not edited to follow a file that moved underneath it. The index resolves every one of them
in one hop, which is exactly what keeping §0–§4 as headings did in ADR 0232.

### 3. `doc/todo/02` §2 becomes a change → gate map

§2's own first sentence has always been *"Run the gates that can see what you touched"*, and then it
listed fifteen commands every round ran wholesale. The map makes the sentence operative, and it is
**derived from the crate graph and from what each gate reads**, not guessed:

- **The first four lines are the core and every round runs them.** They are the only thing that sees
  a lint, a broken doctest, or a test somewhere else in the workspace, and they are about a tenth of
  the sequence.
- **`pdf-render`, `pdf-syntax`, `pdf-font`, `pdf-model`, `pdf-spec`, `pdf-sandbox` and `render-cpu`
  are under everything**, because every corpus-scale gate rasterises with `render-cpu` through
  `pdf-model`. A change there runs the whole sequence.
- **`render-quorra` is under its own gate**; `render-gpu` is under **no** gate at all and a round
  touching it should say so rather than assume the corpus covered it; `viewer-core` and
  `viewer-accessibility` are under the two censuses; the host crates are under no corpus gate at
  all; `tools/conformance` and `ledger.toml` are under the conformance gate.
- **A documents-only change runs the core and the conformance gate**, which reads citations and
  quotations out of the tree, plus `--bin quotations` and `--bin pointers` when a document or a
  pointer moved.

**Two rules are the whole of the compromise's safety, and they are not negotiable.** The full
sequence runs **every fifth round** and on **any round that can change a pixel** — because a map is
a claim about the crate graph, and a claim decays exactly the way a ledger row's does. And §2's
merge paragraph is untouched: a merge is a round of its own and runs everything on `main`, which was
paid for by eleven `clippy::pedantic` warnings living there for five rounds while four rounds
truthfully recorded a silent lint run in their own worktrees.

§2's five notes are arguments rather than counts and stay verbatim: the `--profile gates` argument,
`nextest`, the C compiler and the C++ warnings a cold build prints, `-- --ignored` un-ignoring a
binary's every test, and `pdfref-hayro` being a program nothing built.

### 4. §5's cadence is stated as a rule about staleness

The release binaries are rebuilt **before any measurement, always**, and **every fifth round**
otherwise. The argument is §5's own and is from session 142 — a round reported the viewer as "still
lags" against a binary three hours and six commits old, one of which was the 40× page-turn fix. A
stale binary is a measurement of the past; what does *not* follow from that is paying 95 s of
whole-graph fat link at the end of a round that moved a document.

### 5. `tools/round.sh`

`tools/state.sh` answers *what are the numbers*. This answers the questions a round asks before it
has done anything: which session is next (from `ls doc/history/`, which is a fact on the disk), what
to read for the kind of work it is about, which of §2 that kind needs, and whether this round owes
the full sequence and §5.

Then it checks the four things a round has actually got wrong here, and each is a real incident in
`doc/environment.md` rather than a guess: an uninitialised `doc/arlington-pdf-model`, a compiled
build script baked against a checkout that no longer exists, `target/`'s binaries older than `HEAD`,
and an **exported** `CARGO_TARGET_DIR`, which folds into `sccache`'s key and gives the round a cache
nothing will ever read. It also notes a non-empty stash, because `refs/stash` is shared between
worktrees.

**It changes nothing.** Every command in it reads. A script that silently repaired the tree would be
the instrument altering what it measures, and the round would not know which of the four it had.

### 6. The compaction, under ADR 0281's rule

A fact a command can print is not written down. What came out:

- **`doc/todo/02` §4's sweep catalogue** — 144 lines describing seventeen sweeps, in the file every
  round reads, for a job most rounds do not do. It moved **verbatim** into
  `doc/todo/01-ledger-partial-rows.md`, which §4's own first sentence already calls "the reading".
  §4 keeps the rule and the shape the sweeps share.
- **The four second copies**: the todo bands (already `doc/todo/README.md`'s table), the two tracks
  (already `doc/todo/02` §1 word for word), the closed-by-decision list (already
  `doc/todo/README.md`'s, minus two items, which were added), and the block-summary rule (already
  `doc/history.md`'s own preamble). Each is a merge into the file that owns the subject.
- **`tools/state.sh`'s three invocations and `doc/running-the-viewer.md`'s flag list**, both of which
  were précis of a file one hop away — and a précis is a second copy to keep in sync.
- **"the corpus gate is 2 s in release"**, a gate figure that had drifted, now "seconds in release".

## Consequences

**What a round reads to start**: `doc/HANDOVER.md` 882 → 125 lines, `doc/todo/02` 454 → 388. The
every-round pair goes from 1336 lines to 513, and the rest is reachable by what the round is about.

**Nothing was lost, and it was checked rather than asserted.** Every non-blank line of the old
file's three large sections — the 405 lines of traps, the 82 of "Things worth knowing", the 195 of
"Where we are" — was searched for verbatim across every Markdown this project maintains. Two lines
were not found, and both are the section headings, deliberately re-titled. Fifteen traps before,
fifteen after, with the same fifteen numbers. At paragraph scale, 84 of the old file's 114 blocks
are verbatim somewhere in the new set and the other 30 are accounted for one at a time: two
re-titled headings, two extended tables, four merges, and the rest pointers to a file that states
the same thing.

**The risk this takes on** is the one ADR 0232 named and it is sharper here: a round that needed a
trap and did not open the group it was in. The mitigation is that the entry point names the groups by
*activity* and says which two are not optional — trap 1 for anything that can change a pixel, trap 11
for anything that adds a report — and the cheap correction is to move a trap between groups.

**The second risk is the map.** A gate map is a claim about the crate graph, and the graph moves.
That is why the full sequence still runs every fifth round: the map is allowed to be wrong for at
most four rounds, and a merge never trusts it at all.

**What was not done.** The forty-two source comments citing `doc/HANDOVER.md` by trap number were
**not** rewritten. Rewriting them would touch `pdf-model` and `pdf-font` doc comments for no gain,
and the handover's trap index resolves them in one hop — which is the same defence ADR 0232 chose
when it kept §0–§4 as headings rather than editing the ADRs that cite them.
