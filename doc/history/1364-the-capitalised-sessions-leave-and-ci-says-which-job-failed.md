# 1364 — The capitalised sessions leave the code, and CI says which job failed and why

Date: 2026-10-06. ADR: 1563. The INSTRUMENTS slot of batch fifty-six.

**`Session <number>`.** The 29 lines in 22 files now state the current reason with the ADR by
number (ADR 1023); the ADR was found from the round's record or `git log -S` where the comment
named none. A printed sentence went with them: `save_round_trip.rs`'s ratchet now holds its floors
"as ADR 0334 set them". `tests/spelled_ordinals.rs` holds the second count at 0 beside the first,
by equality. `tools/round.sh`'s one spelled ordinal, which the `.rs` sweep cannot see, went too.

**CI.** `round.sh` asks `gh` with the owner's token beside the main checkout; a round's shell has
no login of its own. The repository is public, so `tools/state.sh main-checkout` now reads the
last run, its failed jobs, steps and annotations with no token (ADR 1563). The run: `raster-examples`
failed on two examples whose premises had gone stale (`outline_upload`'s witness was no longer
declined by the triangle test; `retained` compared a warm frame with a cold frame's row since
ADR 1517); `nightly`'s Miri (advisory) met crossbeam-epoch through a mesh raster that asked
rayon's pool on its serial path, and a dash test sitting on the last bit of `hypot`; and on the
newest run `check`, `test` and macOS were never acquired by a hosted runner — the owner's. All four
of the tree's are fixed; no gate here would have caught any of them, which is the proposal below.

**Documents.** `doc/PLAN.md` §5a names `tests/ledger_notes.rs` and `tests/spelled_ordinals.rs`
and states ADR 1548's quoting rule at `out-of-scope`; `HANDOVER.md` indexes RFC 0009 (PLAN lists no
RFCs); `crate-map.md`'s `viewer-core` row and `state-of-play.md`'s drive passage carry ADRs 1543
and 1540. Records 1352–1358 are within budget with their gates stated. `doc/todo/65` re-derived
at the end: its membership is the ledger's 15 open rows; nothing of mine changed in it.

**Fuzz.** `tools/state.sh fuzz` prints each corpus's size and newest seed date from the pass that
counts it (`jpeg_bands` 192 419 seeds, 1.8 GiB of seed bytes); the coverage question is the new
`fuzz-stale` section, in `all` only, which runs round 1362's `fuzz/seeds.sh check`.

**Proposed for `doc/todo/02` §2:** a change under `raster/crates/raster-gpu/src/` runs every example
`ci.yml` names with `--check`, under Xvfb, behind the lock.

**Gates.** rustfmt --check on the 25 Rust files: exit 0. clippy -D warnings over the 14 touched
crates: exit 0. nextest of six crates' libs: 947 passed; `pdf-render`: 257 passed. `cargo test -p
conformance`: every binary passes but `records`, which fails on two siblings' unfinished records
(1359, 1363). `cargo +nightly miri test -p pdf-render -p pdf-syntax --lib`: 253 and 121 passed,
exit 0. `outline_upload --check` and `retained --check`: exit 0 on the device and on llvmpipe.
`bash -n` on `state.sh` and `round.sh`: exit 0.
