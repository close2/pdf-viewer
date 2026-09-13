# 1012 — Where the effort goes

Date: 2026-09-13. A measurement round, read-only over `crates/` and `tools/` at commit `6c6a1d3d`,
with four sibling rounds live in the same tree. Commissioned by the project owner on the observation
that *"we are doing less and less real work … more than 50% is spent on gates, merges, checks."*
Output: `doc/reviews/1012-where-the-effort-goes.md`. No ADR — ADR 1005's precedent is that a review
proposes and the owner decides. Nothing under `crates/` or `tools/` was touched.

## What was done

Five questions, four of them delegated to read-only agents and the first done here.

1. **All 206 `partial` ledger rows read and classified** — 969,033 characters of `note`. Extracted the
   rows from `git show 6c6a1d3d:doc/conformance/ledger.toml` into a working file, condensed each note
   to the sentences that say what keeps it `partial` (161 of 206 state it explicitly), read every
   family, then classified the 148 leaves into closable / blocked-on-a-text / blocked-on-a-capability /
   blocked-on-a-witness / not-a-gap.
2. **Gate archaeology** over `doc/history/912`–`1011` (98 files) and `doc/adr/0900`–`1030`, per gate
   line, distinguishing a self-caught defect from a pass and from a sibling's failure.
3. **Instrument inventory** — the 20 `tools/conformance` binaries, `tools/state.sh`'s 31 sections, the
   16 fuzz targets, 154 examples, and the trajectory of `--bin pointers`' and `--bin quotations`'
   standing figures.
4. **The preamble corpus** measured at `6c6a1d3d` and at ten-session intervals back to 880, with the
   two named compactions located by commit, plus per-trap and per-habit citation counts.
5. **The ADR and history convention** — the 52 ADRs of the window classified and their citations
   counted in `crates/`, `tools/`, other ADRs, and the rule files.

## What was found

**The stall is sharper than the commissioning check said.** `465 → 470 implemented` and
`204 → 206 partial` conceals that the ledger gained **8 rows** in the window — the whole of clause 6,
which ADR 0984 found had been outside every instrument. Diffing the 875 rows common to both commits:
**exactly one changed status, and it went backwards** (§8.6.5.5, `implemented → partial`, session 987
finding that `/Range` had been claimed as read for nine hundred sessions). Net advancement of the
pre-existing ledger over 58 sessions is **−1**, while the ledger file grew 77 KB.

**The 206 is not 206.** 58 rows are aggregate parents with an unsettled descendant and no debt of
their own — mechanically checked, and true of all 58. Of the 148 leaves, **60 are not gaps**
(deliberate departures with the cost measured, rows whose own note says nothing is owed, entries with
no in-scope consumer, debts that are a writer's or a validator's). **31 are closable now**, 4 are
blocked on a text, 52 on a capability, 1 on a witness alone. And the rows collapse into far fewer
debts than rows: §12.8's 22 rows are about five debts, 12 of them one trust store.

**The gates' yield is concentrated in their cheapest tenth.** 17 of ~18 self-caught defects over 98
sessions came from `fmt`, the two `clippy` lines, `nextest` and `cargo test -p conformance` — the last
alone accounting for 7, at a cost of seconds. The ~25 minutes of corpus-scale walking produced **one**
self-catch (`launch_path`, session 991) against **fourteen** failures from sibling edits, contention or
population drift, five of them `foreign_corpus` alone.

**Seventeen of the twenty conformance binaries have not run in sixty sessions**, `--bin callers` not
since session 525. Two have never fired as programs. `--bin pointers`' absent count has risen
monotonically 104 → 226 over 470 sessions and ADR 0372 recorded on its first run that most of it is
unrepairable by construction; `--bin quotations`' 49 stood unread for 40 sessions and 3 of the first 5
read in session 1010 were noise, one of them an unfixed matcher defect.

**The instruction corpus grew 15.3% in 131 sessions and neither named compaction removed a line** —
ADR 0974 is −11 lines, ADR 1023 is −1 line and +394 words. ADR 0983's habits split grew the habits half
by 338 lines while presenting itself as a compaction. `doc/traps/the-interactive-loop.md` was last
cited at session 798, and two of the six habit files have never been cited by any round — one of them
on `round.sh`'s own oracle reading list.

**The ADR convention is sound** — one pure-narration ADR in 52, 30 of 52 cited from compiled code.
The history convention is the questionable half: nothing reads it, three tools name it to skip it, and
20 of 51 history files share their slug with the round's own ADR. It also carries an unpriced cost:
`doc/history/` is missing from `prose::NOT_READ`, so `--bin quotations` checks 131 blockquotes and 338
quoted spans in 558 files that no round may edit.

## What was proposed

Eight things to stop, ranked, each with its measurement: tier the gate sequence (three tiers, tier 3 at
the merge); re-status the 60 non-gap `partial` rows before the four-in-six allocation runs; partition
`--bin pointers` by repairable root and fix `--bin quotations`' matcher; drop the duplicate history
file; give the fourteen dormant sweeps a cadence and delete the two that never fired; retire the 467
lines of uncited preamble; fetch the free RFCs that stand as a blocker on §12.8.3; and audit the
remaining instrument denominators after session 1004 found three wrong in one sitting.

Also listed: the 31 closable rows in cost order, with the first ten priced — the first four are hours
rather than rounds, and the first ten together are about three rounds and close fifteen ledger rows.

## What is fine

Recorded in the review's §8, because the owner asked for an honest read and not a case for the
prosecution: the ADR convention, the five core gate lines, the sweeps *as sweeps*, the deliberate
departures (well argued; it is the status they wear that is wrong), `launch_path`, and the traps that
are used — trap 13 appears in 120 history files.

## Notes for whoever reads this next

- The classification of the 148 leaves is a judgement and the review says so. The mechanical parts —
  the 58 parents, the row diff, the cluster counts — are reproducible from the commands in the review.
- One documentation defect found in passing and not fixed, since this round writes no code:
  `doc/verify.md:160` names `-p viewer-gtk --example outline_census`; the example is in
  `crates/viewer-host/examples/`.
- No corpus walk was run. The three numbers that would need one are named in the review's §9.
