# 1386 — A document's scripts run in a third confined process, started at the first trigger

Robustness slot of batch sixty. ADRs 1608, 1609; no row moved here (§12.6.4.17's note sentence is
handed to round 1387), no question.
**The worker.** `crates/pdf-script-worker`: `pdf-script-worker` (built only with `engine`) on
`confined-transport`'s wire, magic `PDFSCW01`, holding one `pdf_script::Realm` for its life;
`ScriptWorker`, a `ScriptRunner`, starts it at the first trigger, never at open. A run carries
its script once per worker, then by index; a new worker is told every field and replays the
document's library. Deadline 250 ms per trigger (budget-stopped runs answered in 103 ms debug,
76 ms release); a loss is named by its trigger, the next trigger respawns, four losses stop
scripts for the document. `script_open spawn_ms=1.8–2.1 first_run_ms=1.1–1.7`.
**The profile.** `Profile::Script`, from `strace -ff` debug and release: `read`, `write`, `brk`,
`munmap`, `mremap`, `madvise`, `getrandom`, `clock_gettime`, `exit_group`, the abort path, and
`mmap` only without `PROT_EXEC`; no thread, no `close`/`recvmsg`/`fcntl`/`mprotect`,
`RLIMIT_NOFILE` 0, 96 MiB `RLIMIT_AS`. Findings: Landlock needs a descriptor (the NOFILE limit now
follows it); Boa's default module loader `realpath`s `.` (a realm now uses `IdleModuleLoader`);
`'x'.repeat(16777216)` reserved 384 MiB before Boa's loop limit fired (guard now stops it,
+0.4 MiB); one call at ADR 1590's budgets peaks up to +150 MiB; Boa's parser overflows the stack
at 500 nested parentheses (contained: one named loss).
**Fuzz.** `script` (1280 seeds, `seed_script.py`) INITED 12187 → DONE 20438 edges, 778 378 runs
in 1202 s, no artefact; `script_wire` (29 seeds, `examples/wire_seeds`) INITED 548 → DONE 1228,
1 271 175 166 runs in 1201 s, no artefact. Both `-s none`, one process, behind the lock, from fresh seeds in `scratchpad/r1386/`.
User AI's tasks at the first heavy run: 164.
**Install.** Not installed until a host supplies a level; `tests/batch.rs` skips a `[[bin]]`
with `required-features` (coordinator's question, ADR 1608 section 1).

**Gates.** `rustfmt --check` on my 14 files: 0. `RUSTFLAGS=-D warnings cargo clippy --all-targets`
for `pdf-script-worker` (with and without `engine`), `pdf-sandbox`, `pdf-script` (with and
without), `conformance`, and `--manifest-path fuzz/Cargo.toml`: 0. `cargo nextest run`:
`pdf-sandbox` 48/48, `pdf-script-worker --features engine` 19/19, without 4/4, `pdf-script
--features engine` 50/50, without 7/7. `cargo test -p conformance --no-fail-fast`: 101, 392 passed, 1 failed — round
1383's record, not yet carrying its `**Gates.**`; `--test batch` 13/13, `--test flags` 2/2.
