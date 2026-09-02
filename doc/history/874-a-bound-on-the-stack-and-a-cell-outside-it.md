# 874 — A bound on the stack, and a cell outside it: `MAX_FORM_DEPTH` is 64 and counts every nested content stream, twenty-five of its twenty-seven "cycles" were the instrument's, and a tiling cell recursed until the stack aborted the process

Date: 2026-09-02.
ADR: [0793](../adr/0793-a-bound-on-the-stack-not-a-guard-against-a-population.md).
Touched: `crates/pdf-model/src/content.rs`, `crates/pdf-model/src/content/run.rs`,
`crates/pdf-model/src/content/xobject.rs`, `crates/pdf-model/src/content/text.rs`,
`crates/pdf-model/src/content/pattern.rs`, `crates/pdf-model/src/content/transparency.rs`,
`crates/pdf-model/src/content/annotations.rs`, `crates/pdf-model/examples/form_depth_cost.rs` (new),
`crates/pdf-model/tests/hostile_budgets.rs`, `crates/viewer-confined/tests/support/amplification.rs`,
`crates/render-quorra/tests/corpus.rs`, `tools/bounded.sh`, `doc/environment.md`,
`doc/conformance/ledger.toml`, `doc/HANDOVER.md`,
`doc/traps/instruments-and-reports.md`, `doc/todo/03-more-corpora.md`,
`doc/todo/10-bounds-that-cap-size.md`, `doc/todo/49-restrictions-worth-re-examining.md`,
`doc/todo/README.md`, `doc/adr/0798-a-walks-cost-is-what-is-in-flight.md`; and the merge commit
before this round's own.

## The merge, and two restarts

The previous 874 agent merged `round-867`'s rounds 870 and 872 into `main` as `4f544f1f` with the
full §2 sequence, then began this file's subject and was lost: its stack probes ran at depth
20 480 with ~20 GB reserved apiece, several beside each other in the session's own cgroup, and
`systemd-oomd` took the session. This agent resumed from its uncommitted example, read the
example end to end first, and kept every probe under `tools/bounded.sh` at depths under the bound.

**And then caused the second incident itself**, which is recorded here because this round owns
it. To name the Mozilla tracker's seven witnesses it launched
`tools/bounded.sh --data 32 -- safedocs survey --dir …/batch3/MOZILLA` in the background at
09:03:14 — the whole 32 GiB walk budget for one 24-thread process, no `--tree` — beside the
desktop, the Claude process, `sccache` and two other rounds. The user slice's memory peaked at
61.09 GB of 61.9; from 09:05 every shell call of this round and round 875's stalled (this
transcript degrades to `echo hi` probes); the survey was killed at 09:07:23; and at 09:08:04 the
Claude process aborted by its own `abort()` — not `oomd`, not the kernel, whose `oom_kill` is 0 in
every cgroup. `RLIMIT_DATA` is per process and 32 GiB was a budget for one walk on a machine
running three rounds. The owner's four rules — one walk at a time across all rounds, `--data` never
above 12 GiB a round, `--tree` on every bounded run (12 for a walk, 8 for a build), under 16 GiB in
flight per round — are in `doc/environment.md` and in `tools/bounded.sh`'s header with the
timeline; the script now refuses `--data` above 12 without `--tree` and defaults `--tree` to 12.
The survey was re-run at `--shards 2` (12 threads, 16 GiB) and named the seven; its 256
`incomplete` verdicts whose only fault was a stale `pdf-sandbox-worker` measured nothing and
nothing below rests on them.

## The decision track: `doc/todo/49`'s `MAX_FORM_DEPTH`

**Measured first.** `examples/form_depth_cost` bisects the smallest thread stack a chain of one
kind of nested stream draws on, one child process per probe, and prints the difference of two
depths per level. Release, 8 against 56 deep: a form 3925 bytes, a transparency group 5290, a
Type 3 glyph description 9130, a soft-mask group 5461 over its four; dev: 3498, 4864, 4949. Every
thread `interpret` runs on is `std`'s 2 MiB default — nothing in the tree sizes one — so 64 levels
of the costliest kind are 570 KiB and the bound is **64**, argued from that and not from the
witnesses.

**Then three findings.** The example's pattern kind overflowed 256 MiB at sixteen deep, and seven
hand-built cycle fixtures under `bounded.sh --data 2` found why: `run_cell` ran a tiling cell at
`MAX_FORM_DEPTH - 1`, so a pattern whose cell fills with itself, a form and a cell reaching each
other, and a `d0` glyph doing so through a pattern each ended in `fatal runtime error: stack
overflow` — a seven-object file apiece, open since patterns were first drawn (`9efe4406`). The
same constant explained ADR 0271's population: lifting the bound lifted the cell's start with it,
so a cell holding two levels of forms reached 256 as surely as 16. With one counter in
`Interpreter::run` and the `form_depth` parameter gone from eleven signatures, all twenty-seven
witnesses were re-run at 64 under `bounded.sh --data 2 --tree 6`, one at a time: the crawl's four,
`MOZILLA`'s seven (`1351498-0`, `1351498-3`, `1383267-0`, `1383267-5`, `750664-0`, `843203-0`,
`973589-1`) and fourteen of GHOSTSCRIPT's sixteen draw whole reporting nothing; the two real
nestings (`697655-0`, `695948-0.zip-0`) draw whole and were looked at — the *STATUTS* page and the
boxed regulation paragraph, where round 871's renders were blank; and `698226-0` and `700301-0`
report the bound at 64 and, in a scratch build, at 256 with the same command counts. Two cycles
of twenty-seven. A cycle guard by identity was considered and declined in the ADR: a stream
inside itself is not always infinite, because what it invokes depends on the state it inherits.

**And one thing recorded rather than fixed.** A chain of tiling patterns each filling with the
next is 9ⁿ commands even for a fill inside one cell — the span takes a neighbour each side — and
eight deep is 8 503 056 commands and 2 GiB before `MAX_OPERATIONS` stops it. `doc/todo/49`'s
count-not-cost item has the witness; the example's `--write` hands the chain to `open_one`.

Trap 29 is the general lesson: a bound lifted in a scratch build is lifted only where the code
reads the constant, and a lifting experiment wants a control the way a sweep does.

## `doc/todo/17`'s pointers

`--bin pointers` does not flag a bare `doc/todo/NN` — it counts them as *a form* — so ADRs 0791
and 0798 were read by hand. ADR 0791 states in its own header that it deleted the file and carries
its argument, and is left alone; ADR 0798 pointed forward at the file as though it existed, and
gained a parenthetical correction naming ADR 0791 and the README's reservation rule, in the shape
the sweep marks as a correction rather than an edit.

## Gates and binaries

The full `doc/todo/02` §2 sequence ran on `main` after the last edit, each line under
`tools/bounded.sh` at `--data 8` and `--tree 8` for a build or `12` for a walk, alone on this side
of the machine, with the figures in the run's logs: formatting and `clippy` under `-D warnings`
silent for the workspace and for `fuzz/`; **2969 tests passed, 19 skipped**; doctests green;
the corpus gate at **974 documents, 64 incomplete**; the oracle at **1945 pages, 1841 complete,
104 incomplete**, green; the three text gates green (99.67% of matched words in bounds, 493 of
503 documents fully in); the two censuses green; dates 1514 of 1545; XMP green; JPEG 2000 green;
quorra at **958 pages, 932 agree, 22 differ, 4 refused** and the 4× lane at 953, 938, 11, 4;
fixed documents 56 of 56; the transform gate at 189.8 pages/s over a floor of 40; conformance,
`--bin quotations` and `--bin pointers` green.

Three lines failed first and each was this round's. The first corpus run died of a 1.9 GB
allocation under its 8 GiB `RLIMIT_DATA`, which was `ContentStreamCycleType3insideType3.pdf` at
25 GB — found by walking the corpus one document at a time and, to this round's discredit,
without a bound the first time; the copy-before-charge fix above is what closed it. The
workspace `nextest` was killed by this round's own `--tree 8`, a test run of 2969 processes
being a walk (it passed at 12). And the quorra gate's ratchet caught the same document arriving
in the device's refusals at 1× and, on the lane a round owes when it moves a refusal, at 4× —
*383577888 scene-derived bytes, over the stated budget of 268435456* — which is pinned in both
lists with its reason: four million commands of a cycle are over the budget at any scale, and
the CPU backend draws it and says so. §5's binaries, `libviewer_ffi.so` among them, were rebuilt
and installed after the sequence.
