# 1816 — The docs walk has a bound of its own and an export stamps its sources after its seed

Instruments round of batch one hundred and twenty-two, 2026-10-27, Sonnet at high effort. ADRs
2466 and 2467 both used; Q682 unused. I worked in `scratchpad/r1816/`. No ledger row moves.
**The request.** The owner's word that traps 280 and 281 are fixed in this batch, and ADR 2462
rule 7 (`tools/round.sh` stops listing `CLAUDE.md`).
**Trap 280.** The first docs walk on the shared tree reproduced the red (45 `could not document`
lines, 479 s). On an export of HEAD whose target held a copy of the batch's `doc/`, `--data 12`
failed with the 1 879 048 192-byte allocation of batch 120; the same input under `--data 24`
exited 0 after 951 s with the largest `VmData` 16.2 GiB and 7.80 GiB resident. An empty `doc/` passes
under 12. `tools/state.sh` gives the step `DOCS_DATA_GIB` and keeps the wrapper's own sentence in
its filter; ADR 2466.
**Trap 281.** `tools/batch.sh export --seed <dir>` copies the seed itself and stamps every
extracted regular file after it (the newest of now and the seed's newest file, plus a second); a
re-export does not seed twice; the record carries a `seed:` line. A test in
`tools/conformance/tests/batch.rs` seeds from a sibling's edit dated two hours ahead and the export's
first build must compile; calibrated by planting, where the build printed `Finished` and the test
failed; ADR 2467. `tools/round.sh` no longer lists `CLAUDE.md`.
**Premise that did not hold.** The brief said `export` seeds by reflink; it did not, the round
did by hand, so the option is new. A first `VmData` sampler counted every rustdoc on the machine,
a sibling's doctests among them, and was discarded for one that reads the working directory.
**Closed.** Traps 280 and 281 end with a sentence naming ADRs 2466 and 2467 respectively; habit 185 names `--seed`.
**Found, not fixed.** None in my files. `open_returns_while_the_warm_build_and_the_arms_export_run_detached`
failed once under a load average of 25 on a 20 s wait and passed in the whole run.
**Gates.** `bash -n` on `batch.sh`, `round.sh`, `state.sh` exit 0 (shellcheck is not installed).
`cargo test -p conformance --test batch a_seeded` exit 0, and exit 101 with the stamp planted out.
`cargo test -p conformance --no-fail-fast` exit 0 (69 test binaries, none red).
`tools/state.sh --round 1816 docs` exit 0 in the export, and exit 101 with a planted unresolved
link; and exit 0 on the worktree itself after 368 s (7.96 GiB resident under the 24 GiB bound). **Validation** (the orchestrator): the Opus validator's verdict was FAIL on citations only — trap 281's closing, habit 185's sentence, the stamp's comment and its test cited ADR 2466 where ADR 2467 is the export's — and PASS on the scripts, the test, the plant and the docs walk (exit 0, 7.87 GiB resident under 24 GiB); the orchestrator made the five one-token edits at the merge.
