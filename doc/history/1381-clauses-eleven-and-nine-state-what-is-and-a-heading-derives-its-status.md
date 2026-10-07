# 1381 — Clauses 11 and 9 state what is, and a heading derives its status

Ledger slot of batch fifty-nine. ADRs 1598 and 1599; one row moved; no question written.

**The notes.** ADR 1547's rule, applied to clauses 11 and 9. By `tests/ledger_notes.rs`'s rule,
clause 11 had 35 rows carrying 217 session ordinals, and clause 9 had 43 rows carrying 179. Eight
forked readers rewrote them, working from JSON copies. A checker read each output: every kept
double-quoted quotation, backticked name, ADR and `§` number had to be in the old note, no ordinal
could remain, quotes had to balance and no `\u` could appear. Chunks went in one at a time under
`flock /home/AI/ledger.lock`, each with `count == 1` asserted on a fresh read and `cargo test -p
conformance` after it. Clause 11's notes went from 266 536 to 198 626 escaped characters and clause
9's from 217 087 to 180 762. The ledger's count went from 693 to 297, and the constant holds 297.
Spot-read against their old notes: §11.4.1, §11.6.3, §11.7.2, §11.7.4.4, §11.5.3, and §9.2.4,
§9.4.4, §9.6.4, §9.8.3.3, §9.10.2. §11.7.4.4's claim that ADR 1205 states a non-isolated part's
shape was checked against `transparency.rs`. No clause-9 or clause-11 status moved.

**The aggregates (ADR 1599).** A heading's status is derived now; nobody writes it by hand.
`Ledger::derived_status` computes it from the leaves: the shared status where every leaf owes
alike, `partial` where they differ. `Problem::AggregateStatus` fails on a mismatch and names the
row, and `--bin ledger -- --write` sets it. Such a row's note opens "Aggregate of the rows below"
(`Problem::AggregateNote`). The frontier gate now requires `doc/todo/65`'s aggregate list to be
exactly the rows `is_aggregate` names. A heading whose every leaf has settled keeps a hand-written
settled word, because which one is a reading.

**What the rule found.** Seven headings sit over a row that owes, not four: §7.4, §7.6, §7.6.5,
§12.8, §12.8.3, §12.8.3.4 and §12.10. None prints text of its own in `doc/md/`. **§7.4 was
`implemented` over the `partial` §7.4.7 and §7.4.9, and is now `partial`** (ADR 1598; the heading
states nothing, and ADR 1535's rule and ADR 1035's then decide it). A forked reader cut the seven
notes to the opening phrase. Every sentence no child carried was moved into the child it is about:
§7.4.1, §7.6.2, §7.6.4.2, §7.6.5.1, §12.8.1, §12.8.3.1, §12.8.3.4.1, §12.10.3 and §12.10.4. Twelve
pointers to a heading's note were re-aimed. In `doc/todo/65`, §7.6.5 and §12.10 left their bullet
heads for the aggregate list. `doc/todo/01` and `doc/PLAN.md` §5a say the same.

**Gates.** `rustfmt --check --edition 2024` on `ledger.rs`, `frontier.rs`, `bin/ledger.rs` and
`tests/ledger_notes.rs`: exit 0. `RUSTFLAGS="-D warnings" cargo clippy -p conformance --all-targets
--keep-going`: exit 0. `cargo test -p conformance --no-fail-fast`: exit 101, every binary passing
(lib 326 tests, four of them new) but `fuzz_workspace` (`fuzz/Cargo.lock`, a sibling's). `ledger_notes` passes at 297,
and `the_frontier_map` and `the_ledger_agrees` pass. `--bin quotations`: exit 0, with 3 ledger
divergences, the same three as before.
