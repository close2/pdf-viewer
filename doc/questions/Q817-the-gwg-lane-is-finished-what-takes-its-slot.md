# Q817 — The GWG lane is finished. What takes its slot?

Source: round 1977 (A810, ADR 2765), the orchestrator's review of `doc/gwg-output-suite.md`.

## What stands

- **Every deficit the suite found is closed.** `doc/gwg-output-suite.md` lists ten: eight are fixed, and two (3 and
  10) are argued not to be defects from the clause. The verdicts are screen 35 pass and 16 n/a, press 51 of 51. An
  n/a is a patch whose `X` only exists to reveal ignored overprint, which a screen view does not simulate.
- **The suite is GWG 2015 output patches only.** Nothing else of GWG's is in the tree (its licence allows use only for
  testing a workflow, and its bytes are never committed).

## The choice

1. **Give the slot to a second ledger lane (recommended).** Ten rows are `partial`: 7.4, 7.4.9, 7.6, 7.6.6, 12.8,
   12.8.3, 12.8.3.4, 12.8.3.4.4, K, K.2. K.2 already has its own lane; this one would take the others, each round
   checking first that what is left is effort only. This stays inside "only ledger rows and PDF/A".
2. **Give it to a second PDF/A lane.** Rank rows still open: R09 (a), R16, R03a's CJK remainder, R07's ICC alternate,
   R12's sampled transform. Two PDF/A rounds would share `crates/pdf-transform/src/archive/` and conflict often.
3. **Another GWG suite** (for example the GWG 2022 or the Altona test suite), if you have one to place under
   `~/scratch`. It would be a new GWG lane, run the same way.
4. **Leave the slot empty**, and run two lanes.

## What happens until you answer

From round 1982 the slot runs option 1 in the old GWG worktree. Its rounds are numbered as ledger rounds and touch
no row K.2's lane owns.
