# 1429 — Twelve corpora seeded and campaigned, and a target that had stopped compiling

Slot 4 of batch sixty-seven, 2026-10-08, a fuzz round. ADR 1694; no row moved, no question.

**Premise.** Held: `tools/main-checkout.py` named the ten unseeded targets and the two stale ones
the brief lists. One premise under it did not: the `script` target did not compile at HEAD, since
ADR 1664 moved a field's widget members into `widgets` and `fuzz/fuzz_targets/script.rs` was left
building the old shape (eight E0560). It now builds one `WidgetState`, with slot 1's `on_state`.

**Seeding.** Each target by its `fuzz/seeds.sh` arm, one `--tree 6` hold each; `jbig2`'s arm writes
`jpx` too and peaked at 5.34 GiB. Then `fuzz/seeds.sh check` per target: all twelve current, and
the two re-seeded ones lead their fresh seeds (`forms_data` 1 387 to 1 285, `display_list` 1 008
to 956). `script_wire` went STALE between seeding and check (44 against 594 edges) because slot 1
raised `wire::VERSION` to 7; re-seeded, it reads 711 against 711, and `doc/verify.md` now says a
version bump does this.

**Stale today:** none of the twelve this round checked.

**Campaigns.** 600 s each, built `-s none`, run by path with a scratch corpus first, under each
line's limits; ADR 1694 section 4 has every `INITED → DONE`. No crash and no memory refusal
anywhere. `jbig2` (fork mode) left eight timeouts, all still decoding past 40 s, and `gdb` puts
each in `hayro-jbig2`'s symbol dictionary decode — ADR 1424 section 3's class, whose test is the
confined worker's deadline — so no new test; the eight are in `fuzz/artifacts/jbig2/`, named by
ADR 1694. `script` gained 54 % of its edges in ten minutes and was still finding.

**Unfinished.** Nothing of the contract. Longer campaigns on `script` and `vfs_write` are the next
spend; the campaigns' finds stayed in scratch (ADR 1423).

**Gates.** `rustfmt --check --edition 2024` on `fuzz/fuzz_targets/script.rs`: exit 0. `cargo fmt
--manifest-path fuzz/Cargo.toml --check`: exit 0. `RUSTFLAGS="-D warnings" cargo clippy
--manifest-path fuzz/Cargo.toml --all-targets`: exit 0. `cargo +nightly fuzz build -O -s none`
(all 32): exit 1 at HEAD's `script.rs`, exit 0 after. `cargo test -p conformance --test
fuzz_workspace`: 4 passed. `cargo test -p conformance`: 408 passed, 0 failed. `fuzz/seeds.sh
check`: 12 run, exit 0 each, 12 current after the `script_wire` re-seed. Twelve campaigns,
`tools/bounded.sh` exit 0 each; libFuzzer exit 0 on eleven, 70 on `jbig2`'s fork parent (its eight
timeouts). The round held the lock 40 times; its duration 12 229 s.
