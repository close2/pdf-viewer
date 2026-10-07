# 1387 — The ledger names no session

Ledger slot of batch sixty. ADR 1610; no status moved, so no ADR 1611; no question written.

**The rule, extended.** `tests/ledger_notes.rs` had counted two shapes of a session ordinal. It now
counts four, adding a spelled ordinal below a hundred followed by `session`, and a number followed by
`sessions`. A unit test pins both sides of the line: a fraction, a page position, a sweep's name and
"a second round" are not counted. By the old rule the ledger carried 297: clause 14 had 159, 10
had 100, O 23, D 5, 6 and I 4 each, and E 2. The extended rule found 63 more in clauses 7, 8, 9,
11, 12, 10 and 14, for 360 in all. Now there are 0. `ORDINALS_IN_THE_LEDGER` is 0 and held with
`==`, and the test is `no_ledger_note_names_a_session`. `doc/PLAN.md`'s sentence about it says the
same.

**The rewrite.** Eight forked readers worked from JSON copies of the rows, and a ninth re-read the
33 `inapplicable` and `writer-side` rows the eight did not have. Changed: 119 rows, 425 217 → 350 077
characters. The families before → after, in characters:
clause 10 161 936 → 125 519 (§10.7.4 alone 92 286 → 67 878), clause 14 209 003 → 177 147, Annex O
18 129 → 13 900.

A checker was the gate on each output: every kept quotation, backticked name, ADR and `§` number
had to be in the old note, no shape could remain, quotes had to balance, and no `\u` could appear.
The new quotations in §7.6.7, §14.10.6 and E.2 were checked against `doc/md/`. Each chunk went in
under `flock /home/AI/ledger.lock` with `count == 1` asserted on a fresh read, and `cargo test -p
conformance` ran after each one. Spot-read against their old notes: §10.4.2.4, §10.7.3, §10.7.4,
§14.3, §14.8.5.4.3, §14.9.4, §14.11, O, D, I.2, 6.1, §7.4.4 and §9.3.8. In §14.8.5.4.3, a
reader had called `/Width` and `/Height` results. Table 379 makes them a constraint, and the note
is corrected.

**Condition rows.** Every `inapplicable` and `writer-side` note states one condition in the
standard's words. The tree was checked against each one: `/SeparationInfo`, `/OPI`, `/SpiderInfo`,
`/DPM`, `/BoxColorInfo`, `/ProcSet`, the layout attributes, and the writers' `/EncryptedPayload`. Each
condition still holds, so no status moved, and `doc/todo/01` and `doc/todo/65` are unchanged.

**Gates.** `rustfmt --check --edition 2024 tools/conformance/tests/ledger_notes.rs`: exit 0.
`RUSTFLAGS="-D warnings" cargo clippy -p conformance --all-targets --keep-going`: exit 0. `cargo test
-p conformance --no-fail-fast`: exit 101, 390 passed and 3 failed. The three failures are
siblings': `install_names_every_program…` (`pdf-script-worker`), `every_ignored_test_file…`
(`script_corpus.rs`), and `every_record_since_the_rule_states_its_gates` (record 1388).
`ledger_notes`, `the_frontier_map` and `the_ledger_agrees…` pass. `--bin quotations`: exit 0,
with 3 ledger divergences, the same three. `ps -u AI -o nlwp=` summed: 197 threads.
