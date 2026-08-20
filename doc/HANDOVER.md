# Handover

Read `/CLAUDE.md` first — the five principles, what *done* means, and the closed exclusion list.
**Principle 5 is the one that changes how you work**: the specification is the only source of
truth, and agreement with poppler, mupdf or pdf.js is evidence that we read it right, never the
definition of right.

**This file is an index and nothing else.** It says which file this round opens and which traps
this round is in a position to spring; everything it used to state in full now lives one hop away,
in the file a round with that job opens. It carries no numbers — `tools/state.sh` prints those,
and `doc/traps/instruments-and-reports.md` says how to read them (ADR 0281).

**A lesson lives here exactly once**: in a trap if it changes how you write code, in
[`doc/habits.md`](habits.md) if it changes how you work. A session's narrative belongs in its ADR
and in [`doc/history/`](history/README.md), nowhere else. This file has been halved five times; if
you find yourself retelling a session here, you are undoing that (ADRs 0232, 0281, 0428).

**And the round's own record is one *new file*, never an edit to an existing one.** Write
`doc/history/<session>-<slug>.md` — a file that did not exist before, named so that `ls` sorts it
last — and write nothing about the round anywhere else. A round does **not** append to
`doc/history.md`, whose table holds sessions 5 to 445 and is closed — **a *closing* round is the
one exception**, and what it appends is its block summary, below that table, beside the other
blocks' (`doc/history.md`'s own preamble has the rule; this file and
[`doc/history/README.md`](history/README.md) both stated the prohibition without its exception
until the six-hundred-and-fifth round went looking for it). It does not extend a table here, in
[`doc/todo/README.md`](todo/README.md) or in [`doc/todo/02-every-round.md`](todo/02-every-round.md);
and it does not touch a neighbouring session's file. One round, one added file, and
[`doc/history/README.md`](history/README.md) says what goes in it.

## Which file this round needs

Each of these is *all* of what it holds, not a précis. Open the one your round is about — that is
what these files are split by.

**Every round, whatever it is about — and this list is short on purpose:**

| | |
|---|---|
| [`doc/todo/README.md`](todo/README.md) | the index of owed work, one file per item, `ls` sorting by priority |
| [`doc/todo/02-every-round.md`](todo/02-every-round.md) | what a round does around whatever it takes: which gates its change needs, the sweeps, the binaries, the commit |
| [`tools/round.sh`](../tools/round.sh) | run it first: the next session number, the reading list for this kind of round, whether the full gate sequence is owed, and the four things a round has got wrong before |
| [`doc/environment.md`](environment.md) | the machine, the agent's account, the display, the build directory, the working agreements, and the one command a fresh clone needs |
| the trap group this round is in | below — each trap is a mistake somebody actually made |

**And then by what the round is:**

| a round that | opens |
|---|---|
| asks what the program already does | [`doc/state-of-play.md`](state-of-play.md) — the capability list, and which clause each came from |
| wants a number | `tools/state.sh` — `quick` in seconds, the whole thing in minutes; never a document |
| reads a clause, or writes a ledger row | [`doc/habits.md`](habits.md) *Reading the specification* and *The ledger*, [`doc/ledger-and-claims.md`](ledger-and-claims.md), [`doc/errata-read.md`](errata-read.md), [`doc/todo/01`](todo/01-ledger-partial-rows.md) |
| judges a page against other renderers | [`doc/habits.md`](habits.md) *Judging against other implementations*, [`doc/oracle-and-corpus.md`](oracle-and-corpus.md), [`doc/todo/00`](todo/00-ambiguous-bucket.md) |
| **measures** anything | [`doc/habits.md`](habits.md) *Measuring*, [`doc/performance.md`](performance.md), [`doc/verify.md`](verify.md) — and `tools/state.sh`, because the number has to be printed rather than quoted |
| writes a host, or adds a message | [`doc/ui-boundary.md`](ui-boundary.md), [`doc/todo/30`](todo/30-a-native-host.md)–[`33`](todo/33-annotation-editing.md) |
| adds or questions a dependency | [`doc/stack.md`](stack.md), [`doc/third-party-data.md`](third-party-data.md), [`doc/PLAN.md`](PLAN.md) §1 |
| runs the program | [`doc/running-the-viewer.md`](running-the-viewer.md), [`doc/environment.md`](environment.md) |
| runs an instrument that is not a §2 gate | [`doc/verify.md`](verify.md) — `deny`, the fuzzers, callgrind, the cross-target checks, the census examples, AT-SPI |
| looks for where something lives | [`doc/crate-map.md`](crate-map.md), [`doc/PLAN.md`](PLAN.md) |
| asks *when* something landed | [`doc/history/`](history/README.md), one file per round from 446 on, and [`doc/history.md`](history.md) for the rows before it — that is the only place session bookkeeping goes |

---

## Traps — read the group this round is in, before writing code

Each trap is a mistake somebody actually made in this tree, and they are grouped by **what a round
is doing** rather than by their numbers: a round that skips the group its work is in repeats a
mistake somebody paid for. Open one file; the numbers inside it are not consecutive and are not
meant to be.

| open this | when the round | traps |
|---|---|---|
| [`doc/traps/pixels-and-rasterisers.md`](traps/pixels-and-rasterisers.md) | can change a pixel — the interpreter's marks, either rasteriser, colour, a cross-backend scene | 1, 2, 6, 12b |
| [`doc/traps/oracle-and-references.md`](traps/oracle-and-references.md) | reads a verdict, diagnoses a page, invokes another renderer, or moves a tolerance | 3, 9, 12 |
| [`doc/traps/parsers-and-streams.md`](traps/parsers-and-streams.md) | touches `pdf-syntax`, a filter, a font program, an image codec, or decides what to do with input it cannot fully handle | 4, 5, 8 |
| [`doc/traps/the-interactive-loop.md`](traps/the-interactive-loop.md) | turns a press into a command, or converts between the page's space, the display list's and the raster's | 12a |
| [`doc/traps/instruments-and-reports.md`](traps/instruments-and-reports.md) | runs a gate, believes a number, adds a report, sweeps for a defect — **and any round that writes Rust at all**, for trap 7 | 7, 10, 10a, 10b, 11, 13 |

**Two of them are not optional for the round they are about.** If this round can change a pixel,
**trap 1** — *the metrics lie, look at the page* — is the one that has paid every session since the
tenth. If this round adds a report, **trap 11** is what stops it firing on a condition the clause
does not state.

Each group file also carries the standing facts about its own area — what used to be this file's
"Things worth knowing", now beside the traps that are about the same machinery.

**Every trap keeps its number**, because `crates/`, `tools/`, `doc/conformance/ledger.toml` and
dozens of ADRs cite them by number, and an ADR is not edited to follow a file that moved underneath
it (ADR 0232 §2). The index below resolves any such citation in one hop:

| trap | | |
|---|---|---|
| 1 | The metrics lie. Look at the page. | pixels |
| 2 | A paint is positioned in the *path's* space, not the device's | pixels |
| 3 | An oracle is only as good as how it invokes the other renderers | oracle |
| 4 | Test against real documents, not hand-written fragments | parsers |
| 5 | Unsupported input must stay loud | parsers |
| 6 | Colour: one conversion, and the specification often has no answer | pixels |
| 7 | `#[expect]`, never `#[allow]` | instruments |
| 8 | A corpus finds what documents contain, not what the specification says | parsers |
| 9 | Two references can agree because they share code — or because they share a *gap* | oracle |
| 10 | The sandbox worker is a separate binary, and Cargo will not rebuild it for you | instruments |
| 10a | A cached reference render is a fourth thing that can be stale | instruments |
| 10b | A *new module file* is a fifth thing Cargo will hand you stale | instruments |
| 11 | A report is only as good as the condition it fires on | instruments |
| 12 | A bound derived from two agreeing references is tighter than the arithmetic | oracle |
| 12a | The display list's space is not the raster's, and a doc comment said it was | interactive loop |
| 12b | A test suite made of small scenes tests small scenes | pixels |
| 13 | A sweep for a defect must be run against the defect before it is believed | instruments |

---

## Habits these sessions earned

Each was paid for once. Traps are about code; these are about how to work. Every one keeps the
anchor that makes it checkable.

**[`doc/habits.md`](habits.md)** holds all of them, in six sections. Open the one the round is
about; a habit is worth reading when you are about to do the thing it is about, which is why they
are no longer here.

| section | about |
|---|---|
| Reading the specification | what a modal verb means, what a silence is and is not, when a claim about the standard decays — and what `doc/md/` is, which is the instrument all of it is read through |
| Judging against other implementations | what an agreement is evidence of, what a reference is being asked, when a measurement is of the instrument |
| Tests, gates and reports | what discriminates, what a ratchet's direction means, what a suite of small scenes proves |
| The ledger, and claims about this tree | how a row, a comment or a todo file goes stale, and which greps find it |
| Measuring | A/B in one sitting, attribute by removing the suspect, and which number to quote for which change |
| Code, bounds and dependencies | what a cache's key claims, what a clamp decides, what a dependency is in a position to break |

**Three of them bind every round rather than a particular kind of work**, and
`doc/todo/02-every-round.md` §7 is where those live, beside the round they bind.
