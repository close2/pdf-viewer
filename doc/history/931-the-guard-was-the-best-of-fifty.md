# 931 — Three merges, and the guard that was the best of fifty

Date: 2026-09-04.
ADRs: [0902](../adr/0902-the-guard-is-the-best-of-fifty-and-every-figure-is-a-first-pass.md),
[0903](../adr/0903-what-a-declined-figure-now-says-and-what-a-quiet-machine-still-owes.md).
Question: [`Q29`](../questions/Q29-a-clock-gate-on-a-machine-three-rounds-share.md).
Touched: `crates/viewer-ui/tests/launch_path.rs`, `doc/checks/launch-path.toml`,
`doc/todo/42`, `doc/traps/instruments-and-reports.md` (trap 34), `doc/HANDOVER.md`, two ADRs, one
question, this file. **No pixel moves**: no crate that draws changed.

A merge round with a measurement in it. Three branches came in — 925, 927 (carrying 923) and 928 —
and then the one figure session 926 could not settle.

## 1. The merges

All three merged clean; the conflicts the round was warned to expect in `doc/todo/02`,
`doc/todo/42`, `doc/checks/launch-path.toml`, `tools/state.sh` and the ledger did not arise,
because `main` had not moved under any of those hunks since each branch took its own base. Each
merge commit's body says what came in and what was read.

**The ledger was checked row by row on both merges that touch it**, which is the check a diff
hides: 875 rows on `main`, 875 on the branch, 875 merged, no duplicate clause, none added, none
dropped, and every status compared against *both* parents. 927 moves one note (§14.3.2) and no
status. 928 moves three statuses — §12.11.5 to `out-of-scope`, §14.9.2 and §14.9.2.2 to
`implemented` — and `main` had moved no status at all since the merge base, so nothing was
resolved by preferring a side. No row is `unreviewed`.

**Worktrees closed: r922, r923 and r925**, each verified an ancestor of `main` with
`git merge-base --is-ancestor` and each with no process of its own. **r927 and r928 stay open**:
rounds 929 and 930 branched from their tips.

**One thing the merges surfaced and this round did not fix**: `doc/questions/` now holds two `Q27`
files. Session 926 asked `Q27-a-font-the-file-does-not-carry` on `main` and session 927 asked
`Q27-cost-floors-for-the-other-seven-walks` on its own branch, on the same day, neither able to see
the other. Round 929 has already found the collision and stepped around it by taking `Q28`, in a
file that is not merged yet and that points at both names; renumbering either now would break it.
The round that merges 929 owns this, and this round took `Q29`.

## 2. The figure that would not sit, and what settled it

`doc/PDF20_AN001-BPC.pdf`'s cold open read 0.87, 1.30, 0.72 and 0.86 ms against `0.49 .. 0.80` in
session 926's four runs. That round wrote three hypotheses into `doc/todo/42`. **All three are
refuted**, and ADR 0902 has the evidence:

- **The copies are reflinks.** `std::fs::copy` is `copy_file_range` and on btrfs that is a
  reflink; `filefrag -v` prints the same physical extents for the gate's copy and for the
  repository file, flagged `shared`. No copy this gate has ever made has had an extent of its own,
  so the build-directory sweep could not have moved one. A copy written with `--reflink=never`
  reads cold in the same time.
- **The disk is not what moves it.** In every excursion the *warm* open — which has no disk in it —
  moved by a larger proportion than the cold one, while the disk probe stayed inside its band. A
  latency probe would have separated nothing.
- **There is no regression.** `git diff db4a76f1 HEAD` over the crates on the open path is session
  925's outline change — the same walk, held in a `OnceCell` — plus `type3.rs`, a test and an
  example. And the calibration work *is* `Document::open` plus `Pages::new` plus `interpret`, and
  it reads 0.703 to 0.749 ms against the band session 922 derived for it.

**What is true is a fourth thing:** the guard is the quickest of fifty passes inside one warmed
process and every figure it guards is one first pass in a fresh one. Over twenty-six runs the guard
moved 1.3 % while the figures moved by factors of two. That is trap 34.

## 3. And the band that will not sit is not the one 926 named

Over thirty-eight runs of the gate on merged `main`, **`doc/pdf.js/test/pdfs/bug1815476.pdf`'s cold
open is the figure that fails**, seven times against `PDF20_AN001-BPC.pdf`'s once. Its band is
`0.32 .. 0.50`; its median over those runs is 0.50, and the three failures in the last batch read
0.520, 0.521 and 0.566 with their children's probes at 0.711, 0.716 and 0.773 — a machine the guard
calls fine. It was banded from fourteen runs where the other three rows had thirty.

**No band was moved, for the second round running**, and now on evidence rather than on an
inability to choose: a band is a claim about a machine, and widening one to admit a loaded machine
puts the loaded machine into the claim. What the gate needs instead is a band on the first-pass
probe, which needs about ten minutes of an idle machine — and this round had one-minute load
averages of 3.5 to 35.7 throughout, with two other rounds building and walking corpora. That is
`Q29`, with three options and a recommendation.

## 4. What the instrument does now

`calibration_pass` kept the first of its fifty readings instead of throwing it away; every child
prints `calibration_first_ms` beside `calibration_ms` and the run prints the smallest. It costs
nothing — the passes were already being run. The check file accepts a `calibration_first_ms` band
and the guard honours it, so deriving one is an edit to `doc/checks/launch-path.toml` rather than
to the harness; the file states none, so nothing is judged on it yet (`doc/todo/05`'s rule).

And **a figure that is declined now says which probe declined it and what all three read**, as does
a figure that fails. Session 926 had four runs, one figure outside its band and no way to tell a
processor refusal from a disk refusal, which is most of why `doc/todo/42` carried three hypotheses
instead of a finding.

## 5. What the gates said

`doc/todo/02` §2 ran **whole** on the merged result, which is the merge rule and not a judgement
about how small the diff looked: both lint lines under `RUSTFLAGS="-D warnings"`, both `fuzz/`
lines, every corpus walk under `tools/bounded.sh --tree 12` with the one-walk rule waited on by
`/proc/PID/exe` before each. **Every line exited 0**, the conformance gate included. §5's ten
binaries and two libraries were rebuilt in `release` and installed before any measurement, which is
the rule this round measures under.

**And the launch line declined every clock figure inside the sequence, which is a finding for
`Q29` rather than a footnote.** It ran at a one-minute load average of about 20 — two other rounds
building and walking — read its calibration at 1.577 ms against `0.62 .. 0.78`, printed
`NOT JUDGED`, printed all twenty-eight figures with the reason beside each, and exited 0. That is
ADR 0884's guard working exactly as designed. It is also the shape of the problem: on this machine,
run the way `doc/todo/02` §2 runs it, this gate's clock half will almost never judge anything —
so the four numbers principle 2 names are gated in principle and unwatched in practice. The eight
figures with no clock in them were judged and held, on every run this round took.
