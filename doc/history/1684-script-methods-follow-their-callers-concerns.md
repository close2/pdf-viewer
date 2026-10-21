# 1684 — Script methods follow their callers' concerns

Session 1684, slot 5 of batch 105, a code-quality round; ADR 2202 and ADR 2203.
Started 2026-10-21T21:09:50Z; finished 2026-10-21T23:37:26Z; 8857 seconds.
No owner decision needed; no model change. Ledger statuses remain unchanged.

**Choice.** The largest fresh ViewState block held 58 methods in 1827 lines.
Move its commit, event, edit, calculation, query, keystroke and request/state methods
into seven scripts/ children. scripts.rs keeps declarations, free helpers and Typed.
No behavior fix is mixed into the split. Child imports name their actual dependencies.

**Proof.** The original snapshot equals fixed baseline 020a86f3 byte for byte.
All 58 method/signature token streams and root declarations match, including literals,
after only parent-path rebasing, visibility normalization and trailing argument commas.
A changed source literal fails the comparison; the integration agent independently
checks every literal. Seventeen pub(super) methods retain crate::view visibility;
nine cross-child private helpers retain their original scripts-module boundary.

**Gates.** Token proof exit 0; scoped conformance, fmt, warnings-denied Clippy and
rustdoc exit 0. Model nextest: 2134 passed, 21 skipped, 97.321 seconds of tests.
The combined hold exits 0 after 421 seconds, peak 3.91 GiB, two build jobs.
First Clippy found two unused moved imports; removed. First conformance stopped on
another round's todo chronology; its owner corrected it before this successful retry.
The shared model suite includes the separate solid-colour fix; lexical identity
isolates this split. Final batch corpus and integration checks remain root-owned.

**Paths.** scripts.rs and seven children; todo 74 section 2, crate-map, form-field
teaching and ledger code pointers for 12.6.3, 12.6.4.17 and 12.7.3 follow the move.
No pixel change is intended. No corpus fixture or golden changed in this scope.

**Proposed habit.** A mechanical split's proof must preserve literal tokens and root
state, and its visibility mapping must be stated; runtime gates remain necessary.
