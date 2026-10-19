# 1573 — "Granted at once" is the lock's own account, and a brief names its sessions

Slot 3 of batch ninety-one, 2026-10-19, a docs round on the project's tools. ADRs 1982 and 1983; no
question; no ledger row moves.

**Premise.** Held: the self-test fails under load and passes alone. **Hypothesis.** Did not hold —
nothing is reordered; run 2 starts after the holder is waited for. The test judged "waited for
nothing" by `wait=` under one 0.5 s poll, and `wait=` from ask to grant is the wrapper's own twenty
processes and a `python3`, which the processor gives: a free lane read 0.0 s fifteen times at a load
of 26 on 24 processors, 0.5 s with six busy loops on two pinned processors, 5.3–13.8 s with the
loops at nice 0, every line `behind=-`. The old script passed 2 of 2 beside a six-job build alone,
failed 2 of 2 beside the loops and 5 of 5 beside both: lanes case 92, order case 137, and a third
the brief had not named, the stop case's 142 logged `hold=0.1s` for a run that held nothing.

**Built.** `granted_at_once` reads `behind=-` (a look that finds its lane held names the holder
before it polls) at the five sites that compared `wait=` with a poll; calibrated by a copy whose first
look misreads a free lane, which fails at run 2 and, planted at 92 alone, at the lanes case.
`lock_record` ends an unheld run at its grant reading, so `lane=-` lines say `hold=0.0s` as the
header states — the one figure moved, only on those lines. Three hang guards 20 → 60 s, the minute
the self-test states. `tools/round.sh` accepts a session the brief's slot heading names past the
branch's range and prints the slot and brief for any; refusals name the brief read. `tools/state.sh
remedies` runs `examples/archive_unbuilt` over `doc/pdf.js/test/pdfs doc/corpora` as its third
heading (a small walk, 201 s, 0.87 GiB) and leaves `quick`. The todo sweep: README lines 10, 30, 38,
41, 56, 59, 61, 69 brought to their files; 32 and 62 moved to the done table with `Cited by:` lines;
Status lines of 32, 34 (a count dropped), 56 (A286), 61 (A26), 62; 30's Q430 bullet, closed by ADR
1945, deleted. Found and left: A24's wording is not yet in `CLAUDE.md`, and A26's rule lives only in
`doc/questions/` and now `doc/todo/61`.

**Proof that nothing moved.** No crate is touched. The self-test's changes are its own; the lock's
only change is `hold=` on `lane=-` lines.

**Files.** `tools/bounded.sh`, `tools/round.sh`, `tools/state.sh`, `doc/todo/README.md`,
`doc/todo/{30,32,34,56,61,62}-*.md`, `doc/state-of-play.md`, ADRs 1982 and 1983, this record.

**Gates.** The self-test 20 times under the loops and the build on two pinned processors, nice 19,
in four small-walk holds: 20 passed, 85–97 s each. `cargo test -p conformance --no-fail-fast`: 55 of 56 targets
pass, `bounded` and `todo_citers` among them; `names` fails on sibling 1571's
`archive/overprint.rs:29` mid-edit. `--bin pointers` exit 0, `--bin quotations` exit 0.
`tools/state.sh --round 1573 docs` exit 0; `tools/state.sh --round 1573 remedies` exit 0, 146 lines.
`bash -n` on the three scripts clean. `round.sh` driven at 1573, 1577, 1570 and two briefs.
