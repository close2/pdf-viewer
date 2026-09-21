# Handover

Read `/CLAUDE.md` first — the five principles, what *done* means, and the closed exclusion list.
**Principle 5 is the one that changes how you work**: the specification is the only source of
truth, and agreement with poppler, mupdf or pdf.js is evidence that we read it right, never the
definition of right.

**This file is an index and nothing else.** It says which file this round opens and which traps
this round is in a position to spring; everything it points at is stated in full one hop away, in
the file a round with that job opens. It carries no numbers — `tools/state.sh` prints those, and
`doc/traps/instruments-and-reports.md` says how to read them (ADR 0281).

**A lesson lives here exactly once**: in a trap if it changes how you write code, in
[`doc/habits.md`](habits.md) if it changes how you work. A session's narrative belongs in its ADR
and in [`doc/history/`](history/README.md), nowhere else. If you find yourself retelling a session
here, you are undoing what this file is — and `CLAUDE.md`'s comment rule says the same of every
sentence in it: the current reason, the ADR by number, and a retired sentence deleted rather than
annotated (ADRs 0232, 0281, 0428, 0974, 0983, 1023).

**And the round's own record is one *new file*, never an edit to an existing one.** Write
`doc/history/<session>-<slug>.md`, named so that `ls` sorts it last, and write nothing about the
round anywhere else: not into `doc/history.md`, whose table is closed and whose one exception —
a block summary — belongs to a closing round alone; not into a table here, in
[`doc/todo/README.md`](todo/README.md) or in [`doc/todo/02-every-round.md`](todo/02-every-round.md);
and not into a neighbouring session's file. [`doc/history/README.md`](history/README.md) is the
argument and says what goes in it.

## Which file this round needs

Each of these is *all* of what it holds, not a précis. Open the one your round is about — that is
what these files are split by.

**Every round, whatever it is about — and this list is short on purpose:**

| | |
|---|---|
| [`doc/todo/README.md`](todo/README.md) | the index of owed work, one file per item, `ls` sorting by priority |
| [`doc/todo/02-every-round.md`](todo/02-every-round.md) | what a round does around whatever it takes: which gates its change needs, the sweeps, the binaries, the commit |
| [`tools/round.sh`](../tools/round.sh) | run it first: the next session number, the reading list for this kind of round, whether the full gate sequence is owed, and each thing a round has got wrong here before |
| [`doc/todo/02-every-round.md`](todo/02-every-round.md) §8 | **the round's own contract**, which is what a brief is written from: the ledger rows named by number, the record budget a record is counted against, scratch under `scratchpad/r<round>/` because a path a sibling also writes is a log one of you loses, and waiting on a pid you hold rather than on a `pgrep -f` that matches its own command line |
| **one heavy walk on the machine at a time** | six rounds share this machine, and six concurrent corpus walks is what the kernel's out-of-memory killer takes a round for. Anything that walks a corpus — a `tools/state.sh` section that runs for more than a few seconds, any `--profile gates --test … --ignored` line — goes behind `RAYON_NUM_THREADS=4 flock /home/AI/heavy-walk.lock tools/bounded.sh -- <command>`: the lock queues you, and you wait *in* it rather than polling around it. It costs wall-clock and it costs no correctness; a killed process costs the batch. `cargo test -p conformance` is not a walk and needs neither |
| [`tools/batch.sh`](../tools/batch.sh) | the orchestrating loop's command — `open`, `gates`, `check`, `close` — with §8 as its reason. `check` prints the six things a merge would otherwise have to remember and exits non-zero if any bites; run it before reporting. A sibling's in-flight file shows up in it as a finding, which is the instrument working |
| [`tools/worktree.sh`](../tools/worktree.sh) | the *per-round* worktree beside it — a branch with a build directory of its own, the gitignored data linked in, every gitlink pinned — and `close` takes the checkout and the build directory away together. [`doc/environment.md`](environment.md) is the prose |
| [`doc/environment.md`](environment.md) | the machine, the agent's account, the display, the build directory, the working agreements, and the one command a fresh clone needs |
| [`doc/traps/README.md`](traps/README.md) | the trap index: one line per trap, the position that springs it and the rule; open a group file where a line bites |

**And then by what the round is:**

| a round that | opens |
|---|---|
| asks what the program already does | [`doc/state-of-play.md`](state-of-play.md) — the capability list, and which clause each came from |
| asks what is left, and why it is not done | [`doc/todo/65`](todo/65-the-remaining-frontier.md) — the `partial`/`reported` rows grouped by their blocker (host surface, dependency, architecture, feature depth, nothing owed), for steering the campaign |
| wants a number | `tools/state.sh` — `quick` in seconds, the whole thing in minutes; never a document |
| reads a clause, or writes a ledger row | [`doc/habits/reading-the-specification.md`](habits/reading-the-specification.md) and [`the-ledger-and-claims-about-this-tree.md`](habits/the-ledger-and-claims-about-this-tree.md), [`doc/ledger-and-claims.md`](ledger-and-claims.md), [`doc/errata-read.md`](errata-read.md), [`doc/todo/01`](todo/01-ledger-partial-rows.md) |
| moves a ledger row that records **one decided departure** inside an otherwise-executed clause | it takes `departed`, not `partial`: every requirement executed except the one the note's first sentence names and its ADR prices; `tools/state.sh` counts it apart from `implemented` and `partial` (the owner's word, `doc/questions/Q63`, ADR 1119) |
| judges a page against other renderers | [`doc/habits/judging-against-other-implementations.md`](habits/judging-against-other-implementations.md), [`doc/oracle-and-corpus.md`](oracle-and-corpus.md), [`doc/todo/00`](todo/00-ambiguous-bucket.md) |
| needs the owner's word on something | [`doc/questions/`](questions/) — one `Q` file per open question, answered by an `A` file of the same name whose `Status` header says what the answer left open; write yours in the same commit as the work that raised it |
| **measures** anything | [`doc/habits/measuring.md`](habits/measuring.md), [`doc/performance.md`](performance.md), [`doc/verify.md`](verify.md) — and `tools/state.sh`, because the number has to be printed rather than quoted |
| writes a host, or adds a message | [`doc/ui-boundary.md`](ui-boundary.md), [`doc/todo/30`](todo/30-a-native-host.md)–[`33`](todo/33-annotation-editing.md), and [`doc/todo/38`](todo/38-a-documents-restrictions-have-levels.md) for what the three windows' restriction menu still owes |
| validates a document against ISO 19005, or converts one | [`doc/rfc/0006`](rfc/0006-pdf-a-validation-and-conversion.md) and [`0007`](rfc/0007-a-refusal-is-a-question-somebody-can-answer-in-advance.md) (the designs), [`doc/pdf-a-mitigations.md`](pdf-a-mitigations.md) (every refusal's remedy), [`doc/todo/66`](todo/66-the-mitigation-catalogue-build-out.md) (which remedies this version carries out, and the command that prints it), [`doc/third-party-data.md`](third-party-data.md) for the texts |
| adds or questions a dependency | [`doc/stack.md`](stack.md), [`doc/third-party-data.md`](third-party-data.md), [`doc/PLAN.md`](PLAN.md) §1 |
| quotes, or wants to quote, a standard that is not ISO 32000-2 | [`doc/third-party-data.md`](third-party-data.md), which states the position per text, and ADRs 0187 and 1085 — see the rule below |
| runs the program | [`doc/running-the-viewer.md`](running-the-viewer.md), [`doc/environment.md`](environment.md) |
| runs an instrument that is not a §2 gate | [`doc/verify.md`](verify.md) — `deny`, the fuzzers, callgrind, the cross-target checks, the census examples, AT-SPI |
| looks for where something lives | [`doc/crate-map.md`](crate-map.md), [`doc/PLAN.md`](PLAN.md) |
| asks *when* something landed | [`doc/history/`](history/README.md), one file per round from 446 on, and [`doc/history.md`](history.md) for the rows before it — that is the only place session bookkeeping goes |

---

## The texts under `doc/md/`, and the one rule that is not about code

ISO 32000-2's sentences are quoted verbatim and the conformance gate checks every one of them
against `doc/md/`. **Every other specification text on this disk is held as licensed to a single
reader: cite the clause or the section and paraphrase, never quote — in a code comment as much as
in a document — and nothing of it is committed** (`/doc/*.pdf` and `/doc/md` are ignored; what is
tracked is the encrypted `doc/specifications.zip`). ADR 0187 is the position. ISO 19444-1, which is
the only text this tree has of the format §12.7.8 names and defines nowhere, is held under it, and
the three ETSI texts are stricter still — their notice permits no reproduction in any form, so
nothing from either appears between quotation marks or after a `>` (ADR 1085).
[`doc/third-party-data.md`](third-party-data.md) states it per text, with where each came from.

---

## Traps — read the index, open the group a line bites in

Each trap is a mistake somebody actually made in this tree, and the whole of it — the incident, the
evidence and the argument — is in one of five group files grouped by **what a round is doing**.

**[`doc/traps/README.md`](traps/README.md) is the index and it is what a round reads**: one line per
trap, giving the position that springs it and the rule, plus the table of which group file is for
which kind of round. A round reads the condition column against what it is about to do and opens a
group file only where a line bites — which is the whole change ADR 1036 made, because the group a
round "is in" runs to hundreds of lines (`wc -l doc/traps/*.md`) and the line it needed is six of them. The index also states why
every trap keeps its number and resolves any citation by number in one hop.

A round that skips the trap its work is in repeats a mistake somebody paid for, and the index is how
it finds out which one that is.

---

## Habits these sessions earned

**One file per kind of work**, the way `doc/traps/` is grouped, and [`doc/habits.md`](habits.md)
is their index — it states what a habit is and what each keeps, and it keeps the six section
headings so that a citation naming one still resolves (ADR 0983). Open the one the round is about;
a habit is worth reading when you are about to do the thing it is about, which is why they are not
here.

| open this | about |
|---|---|
| [`doc/habits/reading-the-specification.md`](habits/reading-the-specification.md) | what a modal verb means, what a silence is and is not, when a claim about the standard decays — and what `doc/md/` is, which is the instrument all of it is read through |
| [`doc/habits/judging-against-other-implementations.md`](habits/judging-against-other-implementations.md) | what an agreement is evidence of, what a reference is being asked, when a measurement is of the instrument |
| [`doc/habits/tests-gates-and-reports.md`](habits/tests-gates-and-reports.md) | what discriminates, what a ratchet's direction means, what a suite of small scenes proves |
| [`doc/habits/the-ledger-and-claims-about-this-tree.md`](habits/the-ledger-and-claims-about-this-tree.md) | how a row, a comment or a todo file goes stale, and which greps find it |
| [`doc/habits/measuring.md`](habits/measuring.md) | A/B in one sitting, attribute by removing the suspect, and which number to quote for which change |
| [`doc/habits/code-bounds-and-dependencies.md`](habits/code-bounds-and-dependencies.md) | what a cache's key claims, what a clamp decides, what a dependency is in a position to break |

**Three of them bind every round rather than a particular kind of work**, and
`doc/todo/02-every-round.md` §7 is where those live, beside the round they bind.
