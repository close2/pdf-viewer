# 934 — Two questions called Q27, and a memory figure guarded by a clock

Date: 2026-09-04.
ADRs: [0908](../adr/0908-two-questions-called-q27.md),
[0909](../adr/0909-a-memory-figure-guarded-by-a-clock.md).
Touched: `doc/questions/` (a rename, the README, `Q28`, `Q29`),
`tools/conformance/tests/questions.rs` (new), `doc/adr/0893`, `doc/todo/03`, `doc/todo/42`,
`doc/traps/instruments-and-reports.md`, `doc/history/926`, `doc/history/931`, two ADRs, this file.
**No pixel moves**: no crate that draws was touched, and the only Rust this round wrote is a test
in `tools/conformance` that reads file names out of `doc/questions/`.

Merged: `round-929` — a cost floor on the reference programs the gates spawn, ADRs 0898 and 0899.

## 1. The merge

`round-929` (`2535f509`) merged `--no-ff` with **no conflict**. Its diff is three test binaries,
`tools/pdfref`, two ADRs, a history file, `Q28` and eight lines of `doc/todo/02` §2 — nothing that
draws.

**The ledger was checked rather than assumed**, which is what the recent merges have taught: round
929 touched no row at all, so the merged `doc/conformance/ledger.toml` is byte-identical to the
`main` it merged into (`git diff 2cd86362 HEAD -- doc/conformance/ledger.toml` is empty), and no row
is `unreviewed`. **And the thing session 931 found by looking for it deliberately was checked the
same way**: every trap number in `doc/HANDOVER.md`'s index appears in its group's row of the table
above it and vice versa, derived rather than read — 929 added no trap, and none had gone missing.

## 2. The number 27, taken twice

`doc/questions/` held two `Q27` files, asked the same day by rounds 926 and 927 from branches that
could not see each other, met for the first time in 929's merge. ADR 0908 is the argument; in short:

- **`Q27-a-font-the-file-does-not-carry` becomes `Q35-a-font-the-file-does-not-carry`**, because it
  is named in eight places against the cost-floor question's seventeen, and its eight include every
  markdown link either of them has. `Q27` is the cost-floor question alone.
- **35 rather than 30**, because 30 to 34 belong to the blocks of rounds this tree cannot read. The
  next free number by `ls` is the mistake that caused this.
- Every site moved with it: ADR 0893, `doc/history/926`, `doc/todo/03`, and the two files that name
  the collision — `Q28`, whose paragraph is amended to say what settled it rather than deleted, and
  `Q29`'s aside. The two history files were repaired in place; ADR 0908 §2 says why that is not the
  bookkeeping `doc/todo/02` §6 forbids.
- **`doc/questions/README.md` gains the allocator rule** — a number comes from the round's own
  reserved block, never from `ls` — and says the gap between 29 and 35 is correct.
- **`tools/conformance/tests/questions.rs` makes a duplicate loud**: two `Q` files sharing a number,
  two `A` files sharing one, an `A` answering nothing, an `A` whose slug drifted, or a name that is
  not `<letter><number>-<slug>.md`. Run against each of those before it was believed (trap 13); on
  the clean tree it prints the thirty questions and which are open.

It makes the collision impossible *to merge quietly* rather than impossible to make, and ADR 0908
says which half each mechanism buys rather than claiming both.

## 3. The machine was not quiet, so no band was derived

`Q29`'s option 1 needs an idle machine. A sampler took the one-minute load average every thirty
seconds for the seventy-five minutes from 17:58 to 19:13: **151 samples, minimum 3.30, median 12.86,
mean 16.69, maximum 61.55, 62 % of them above 10.** Session 931 declined at 3.5 to 35.7. Three
neighbours ran rather than two — 932, 933, and a 935 that appeared mid-round and ran `launch_path`
probes of its own; at 18:24 two of them were walking the corpus side by side at 642 % and 629 % of a
core. **No band was derived**, for the third round running.

What the round found instead is ADR 0909, and it is the better half of the errand. The §2 launch
line failed on **all four `peak_mib` figures at once**, a quarter below their minima, with the
calibration probe at **0.706 ms inside `0.62 .. 0.78`** — which is what let the run judge them. Nine
re-runs alone on a binary the merge had not touched a line of read **161 to 182 MiB every time**,
inside every band, two of them judging with nothing outside. `open_peak_mib`, the memory figure with
no graphics device in it, did not move by a megabyte in either state.

So `peak_mib` is a **memory** figure whose only guard is a **clock**, and a clock reads in band on a
machine with a gigabyte free and on one with sixty. That is trap 34's third dimension — the same
work, in the same *state*, and in the same *units* — and it is the one a `steady: false`
classification looks like it has already handled. `doc/todo/42` carries what is owed and `Q29` the
evidence: a busy-clock probe, which is what `Q29` recommends, would have declined none of this.

**No band was moved and no figure reclassified**, for the same reason the two rounds before this one
gave.

## 4. What the gates said

`doc/todo/02` §2 ran **whole** on the merged result, which is the merge rule: both lint lines under
`RUSTFLAGS="-D warnings"`, both `fuzz/` lines, every corpus walk under
`tools/bounded.sh --data 12 --tree 12` with the one-walk rule waited on before each by
`/proc/PID/exe` — never by a command line, which is a predicate that matches itself. §5's ten
binaries and two libraries were rebuilt in `release` and installed before any measurement.

**Twenty-nine of thirty-one lines exited 0 on the first pass.** Two did not:

- `cargo fmt --all --check` — a `match` arm in this round's own new test. Formatted; the line and
  `clippy --workspace --all-targets`, `cargo test -p conformance` were re-run after the fix and all
  three exited 0.
- `launch_path` — §3 above. Re-run alone, nine times: the four memory figures inside every band on
  every run, two runs judging with nothing outside.

`--bin pointers` and `--bin quotations` were run because this round moved a document and three
pointers; neither reports anything about `doc/questions/`.

## 5. Worktrees

**`r929` closed** — `round-929` verified an ancestor of `main` with `git merge-base --is-ancestor`
after the merge, no branch of a running round descends from it, and no process of its own was alive.
`r927` and `r928` were checked at the same time and **stay open**, because 929 and 930 branched from
their tips and 930 is not merged. `r930`, `r932` and `r933` are running rounds' and were not touched.

## What is left

- **The first-pass band, and now a memory probe beside it.** Both need the same idle ten minutes,
  and this is the third round to record that it did not have them. `Q29` is open, with this round's
  evidence in it.
- **`Q35` and `Q27` and `Q28` are all open questions for the owner**, unchanged in substance by this
  round — only their numbering and, for `Q28`, the paragraph recording how the numbering was settled.
- **Whether `peak_mib`'s band should be a ceiling** is a real question and belongs to whoever takes
  `doc/todo/42` next; it is not a thing to decide while turning a red gate green.
