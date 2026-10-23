# 1704 — Batches 104–108 reviewed against the principles

Session 1704, slot 1 of batch one hundred and nine, a docs round on the owner's request of 2026-10-22;
2026-10-23. ADR 2242 (2243 unused); no `Q`. Review: `doc/reviews/1704-batches-104-to-108-against-the-
principles.md`, scripts in `doc/reviews/1704/` (`slots.py` copied from 1456's with the change stated;
`added.py` and `arms.py` new). Report, no repair.

**Premise, held.** The span is `5ded4916..ee365e4f`: 48 commits, 32 records, ADRs 2178–2241. The
three receipts I checked hold: 38 of 39 gates at 2 920 s, the final arms against `9cd2c665`, and the
1.036–1.383 MiB OCCA drift. The drift is in `open_peak_mib`, not `peak_anon_mib` as briefed.

**Hypothesis, in part.** The code is sound with one exception: an owned `Conversion` is built on every
fill, stroke and glyph to report a black-generation bypass (`content/colour.rs:188–213`). It is
unpriced and was never measured on text. The paperwork departs further than briefed: 8 of 32
durations, 0 tool-use counts, 0 premise paragraphs, 0 trap or habit citations, 63 receipts outside
the tree. The red launch gate is not the only departure. Principle 2's comment rule fails for four
accepted costs, and principle 4's legibility fails in the ADRs (14 of 60 without a Status line) and
the records. Five pages moved, all in batch 104, each claimed by record 1672 or 1673 and named by no
ADR. None is unclaimed.

**Judged.** An orchestrator may not accept a red gate by ADR. The timing half met habit 101 and owed
ADR 1969's re-take from the second merge. The memory half failed HEAD's comparison, and its OCCA drift
falsifies `doc/checks/launch-path.toml`'s own premise for the figure. ADR 2242 records the four answers
a red gate takes. Nothing is reverted.

**Instruments calibrated (trap 13).** `slots.py` reads all 16 rounds of `66c6ef23..5ded4916`.
`premises.py` finds 4 paragraphs in 1663–1670. `cites.sh` finds 14 trap citations in 1639–1670.
`added.py` names the expects in tests and the one history comment. Three subagents read the ADRs,
the instruments and the paint path; each claim used here was checked at its file and line.

**Found, for the orchestrator now.** `cargo test -p conformance` in the shared worktree fails `flags`
on `crates/viewer-confined/src/protocol/tests/commands.rs:158` (`--trust-anchors`). That file is
untracked, a sibling's mid-edit, so it is not HEAD's (habit 138: my gates ran in my export).

**Gates.** `cargo test -p conformance --no-fail-fast`: in the worktree 65 binaries ok, and `flags` fails
on the sibling's untracked file only. `documents`, `cited`, `spelled_ordinals`, `round_numbers`,
`ledger_notes` and `records` are ok. My export runs the same; there `bounded`, `state_sections`,
`submodules` and `workspaces` fail for want of a git index. `--bin quotations` exit 0, `--bin
pointers` exit 0. No crate was touched, so tier 1 owes nothing else.
