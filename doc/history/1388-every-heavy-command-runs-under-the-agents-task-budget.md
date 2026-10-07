# 1388 — Every heavy command runs under the agent's task budget, and the owner's list names the cgroup

Instruments slot of batch sixty. ADR 1612; no row, no question.
**Task bound.** `tools/bounded.sh --tasks N` sets `RLIMIT_NPROC` beside `--data` with `prlimit`.
The default is the agent's budget, 8192, and the script refuses a larger value. `--task-budget`
prints the figure, which is written only here. `batch.sh`, `state.sh`, `fuzz.sh` and
`drive-windows.sh` read it and set `ulimit -u` before anything runs. A failed run whose standard
error shows a refused fork ends on `STOPPED BY THE TASK LIMIT`. Self-test case 6 runs a fork loop
capped at 128 children under `--tasks 64`. It is refused at the first fork, because the limit counts
every task of the user. `tests/bounded.rs` holds every heavy `tools/*.sh` to the budget (4 scripts).
Through a pipe the self-test took 71 s: case 5's abandoned `sleep 60` samplers kept bash's saved
descriptors open. That case now runs in an `exec`-redirected subshell, and the self-test takes 15 s.
**Legible.** `batch.sh check` prints `tasks of AI now, and the bound <n>; ulimit -u <limit> where
called`, with `ABOVE the budget` when it is. `tests/batch.rs` plants a budget of 6000 under a soft
limit of 7000 and sees the word, and sees it absent at 8192. User AI's tasks, sampled every 5 s
over the round: 120 to 647 (439 samples).
**Cgroup.** `main-checkout.py`'s `agent's cgroup` line finds the scopes from the processes of user
AI. For each `scope:` it gives the tightest `pids.max` and `memory.max` on the path. The list's new
"bound the agent's scope" item, ranked before the re-seed, writes 16384 and 40G into each
unbounded tab, or launches with `systemd-run --user --scope -p TasksMax=16384 …`. On the day:
2 tabs, 27 and 228 tasks, the konsole scope's `pids.max` 76003, `memory.max` max. `owner_section.rs`
plants two scopes, one bounded by its slice, and a stranger's process. `doc/environment.md`: one
agreement paragraph and the entry. HANDOVER's heavy-walk row names the bound.
**Documents.** Read against ADRs 1590–1601. state-of-play and PLAN: Tier 1 runs in no window,
because no host supplies a runner; crate-map already had `pdf-script`'s row; PLAN §5a stands.
Ordinals (`spelled_ordinals.rs`): todo 33 17 → 0, 28 17 → 0, 42 15 → 0, 37 14 → 0; all of
`doc/todo/` 176 → 113. Records 1377–1382: 36–40 lines, each with `**Gates.**`.
`gates-cost` names 34 gates; `batch.sh` holds no new one. `doc/todo/65` re-read after 1387 (no
status moved): the A193 sentence now names Tier 1 (ADRs 1590, 1591), and §12.10.2's bullet ADR 1593.

**Gates.** `bash -n`, five scripts: 0. `rustfmt --check` on `batch.rs`, `bounded.rs`,
`owner_section.rs`: 0. `RUSTFLAGS=-D warnings cargo clippy -p conformance --all-targets`: 0.
`cargo test -p conformance --no-fail-fast`: 101, 390 passed and 2 siblings' failures (`install`:
`pdf-script-worker`; `state_sections`: `script_corpus.rs`); `bounded` 2, `batch` 13 less that one,
`owner_section` 4 passed. `batch.sh check`: 1, `cargo fmt` on siblings' files only. Tasks: 201.
