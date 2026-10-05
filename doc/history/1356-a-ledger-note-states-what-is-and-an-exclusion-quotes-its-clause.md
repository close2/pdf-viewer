# 1356 — A ledger note states what is, and an exclusion quotes its clause

Ledger slot of batch fifty-five. ADRs 1547 and 1548; one row moved; no question written.

**The rule.** ADR 1547 decides that `CLAUDE.md`'s comment rule binds the ledger's notes: the
ledger is the current reading of each clause, not a record, and ADR 0281 already names session
numbers and dates as counted facts. Clause 12 carried the most session ordinals (635 of 1914 by
the new test's count); its 130 rows that carried any were rewritten as what is — 124 by eight
forked readers working from a JSON copy, each output checked by a script that every kept
quotation, backticked name, ADR and `§` number is in the old note, then applied row by row under a
lock with `assert s.count(old) == 1` and `cargo test -p conformance` after each chunk; six by this
round with the out-of-scope rows. Clause 12's notes went from 771 945 to 579 081 characters and
carry no ordinal. `tests/ledger_notes.rs` holds the count by equality at 1278 and prints the
families, clause 7 first. Two `Cited by` lines (`doc/todo/25`, `doc/todo/37`) named a ledger
note whose only mention was a retired sentence; corrected.

**The exclusions.** The thirteen `out-of-scope` rows outside clause 13 all rest on one of the four
by the clause's own sentence, now quoted (ADR 1548's table); §12.5.6.17 stays, because every
`shall` of its own is about playing and its drawn `/AP` is §12.5.5's. §14.9.2.4's note named two of
the three entries that hold the array — Table 295's `/TT` is the third, also clause 13. The 81
clause-13 rows had no note; each now names the exclusion and quotes its clause, or is one of 17
headings with no text, which the gate checks against `doc/md/`.

**One row moved: §13.4 `out-of-scope` → `reported`.** The owner approved Q33's recommendation
(A33): the stream form of `/Poster` comes off the exclusion. Nothing was built, and
`appearance::construct`'s `Movie` arm still refuses giving the exclusion as its reason, with a
comment saying Q33 asks the owner. Placed in `doc/todo/65`'s bucket 6.

**Gates.** `rustfmt --check --edition 2024` on `ledger_notes.rs`: exit 0. `RUSTFLAGS="-D warnings"
cargo clippy -p conformance --all-targets --keep-going`: exit 0. `cargo test -p conformance
--no-fail-fast`: every binary passes but `state_sections`, which fails on a sibling's
`crates/pdf-model/tests/restrictions.rs`; `the_frontier_map` and `ledger_notes` pass (2 of 2,
planted heading claim on §13.2.5 fails and restores). `--bin quotations`: 4 ledger divergences, all
on rows outside this round's edits or carried verbatim from before.
