# 1448 — The five held instructions build the worker inside the hold

Slot 5 of batch seventy, 2026-10-08, an instruments round. ADR 1732; ADR 1733 and Q344 not used.
No ledger row moved. **Premises.** `bounded.rs`'s five held files held, eight lines between them as
ADR 1718 counted. `doc/history/1443` has no "Unfinished" heading; its held list is in "Instructions".
`main-checkout.py:309` needed no hunk: once slot 4 pinned the fork, `patches` counted a pinned `Fork:`
as applied, and slot 4 had already rewritten the docstring.

**Re-spelled (ADR 1732).** Each instruction is now its `tools/state.sh` section spelled out:
`tools/bounded.sh --lock` in the section's lane, the worker as `--build '<profile> -p pdf-sandbox
--bins'`, any second program as a second `--build`, then the gate. `raster_golden.rs` covers the run
and the regeneration, both `--tree 12`. `fixed-documents.toml` and `doc/todo/03` §20 are `--tree 6`.
`launch-path.toml` is `--clock` with the script worker's `--build`, and `doc/todo/02` §5's
measurement is `--clock`. `doc/todo/02` §2's tier blocks open on the wrapper's shape, and the bare
`--bins` lines are gone. Tier 3's `pdf-vfs` and `viewer-confined` builds go too: ADR 1718 §2 had
already taken them out of the merge. `HELD_WORKER_BUILD_INSTRUCTIONS` is `[&str; 0]`. The sweep
reads 2 011 files: 0 held, 0 owed.
Planted back by reversing this round's `launch-path.toml` patch, it names `launch-path.toml:[8]`.
`sandbox_gates` named `-p pdf-script-worker`, which this round had first put in a trailing comment.

**Sweeps read.** `round_numbers.rs`: `HELD` is empty, 1 473 sources read, 0 lines. Bare `flock`:
1 held, `doc/rfc/0008`:247 (no slot owns it), 0 owed. Undeclared `--lock`: 0 held, 0 owed.
Records 1438–1443 are 34–40 lines, each with `**Gates.**`, no edit.

**Taken from slot 4** (round 1447, ADR 1730): `doc/environment.md`'s `doc/patches` paragraph,
`doc/HANDOVER.md`'s patches row and `doc/todo/65`'s A227 sentence now say the manifest pins the fork.
The patches line on this tree reads "2 whose base it no longer pins, 0 for a fork the owner is to
create". `doc/todo/65`, re-derived after slot 3's record: 19 `partial` and 4 `reported` rows, as at
HEAD. No status moved, and §12.8.3.4.4 stays in bucket 2 with slot 3's sentence.

**Unfinished.** Slot 4 named two stale texts this slot does not own: §7.4.7's ledger note, which
still says the patches wait on the fork pin, and `image/cut.rs:344–348`.

**Gates.** `rustfmt --check --edition 2024` on `raster_golden.rs` and `bounded.rs`: exit 0.
`RUSTFLAGS="-D warnings" cargo clippy --all-targets`: `-p conformance` exit 0, `-p pdf-model` exit 0.
`cargo nextest run`: `-p conformance` 417 passed, `-p pdf-model --test raster_golden` 2 passed and
1 skipped. `cargo test -p conformance`: exit 0 after slots 3 and 4, then 101 only on slot 1's 42-line
record 1444. `bash -n` on the three scripts: exit 0. `tools/bounded.sh --self-test`: exit 0, 34 s.
`tools/batch.sh check`: exit 1, 915 s, only on `cargo fmt --all --check` in slot 2's
`viewer-ffi/tests/unsafe_position.rs`; the fuzz check is clean.
