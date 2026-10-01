# 1320 — A document's command names what the tree has, and a batch keeps its clock

Instruments slot of batch forty-nine. ADRs 1475, 1476. No ledger row moved; no question written.

**Commands (ADR 1475).** `tools/conformance/tests/commands.rs`: every `cargo run|test|nextest run|
bench|build|check|clippy` in `variables.rs`'s live population, read to the end of its code span
across a wrapped line or a `\`, names a member, an example, a test and a bin cargo resolves; a
`--release` on a run/test/bench line beside a package `doc/verify.md` runs only under `dev` is a
finding; a `tools/*.sh|.py` must exist. First read 607 commands in 323 documents, 42 findings: 40
`--release -p conformance` (`doc/todo/01` 35, `todo/48`, `ledger-and-claims.md`, two trap files),
`verify.md`'s `outline_census` under `viewer-gtk` (it is `viewer-host`'s), PLAN's deleted
`spike-window` example and the false sentence around it. All fixed; 0 now, a gate, planted twin.
`state.sh prose` gains its line. Two false rules on the way: `check`/`build` lines made
`pdf-sandbox`, `viewer-qt`, `viewer-ffi` look dev-only; a shell `\n` read as part of a bin name.

**Clock (ADR 1476).** `batch.sh gates` now logs each gate's `wall` (after the lock) and `wait` (on
it) and sums `wall` on its last line; `state.sh gates-cost` prints the orchestrator's log read-only,
dearest first, or the old "finished in" with a warning. Durations stay out of documents and
records; the commit body carries the log's last line and each round's notified time.

**Documents.** crate-map: raster-gpu (1455, 1456, 1457), pdf-sandbox (1459), pdf-transform (1461),
conformance (`commands.rs`). state-of-play: 1455, 1461 twice, 1453's check box. PLAN: §5a
`inapplicable` (1461), the spikes paragraph, `commands.rs`. HANDOVER: UI row runs
`drive-windows.sh` last; ledger row names 1461's rule; the fork row's item number. Navigation
before/after: 14/14 absent, 26/26 undefined, history 0; overtaken 6 → 0 (six notes read against
1456, 1457, 1465 — latency, no code, find; `READ` moved).

**Owner's list.** *After a merge* is now one entry per line `main-checkout` prints, in its order:
what it means, the command that clears it; `main-checkout.py`'s docstring says a new line owes one.

**todo/65.** 14 partial, 4 reported, frontier map passes. `cargo search`: unchanged from today's
note. README and `todo_citers` agree.

**Records 1309–1314**: 32–39 lines; figures are each round's own. 1311 and 1313 state no gates.

**Gates.** conformance all but `the_ledger_agrees…` (1319's §14.3.4 test, in flight); clippy
conformance 0; rustfmt 0; `bash -n` 0; batch, state_sections, documents, traps 0.
