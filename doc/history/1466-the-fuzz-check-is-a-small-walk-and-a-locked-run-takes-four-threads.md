# 1466 — The fuzz check is a small walk, a locked run takes four threads, and the history sweep reads zero

Slot 5 of batch seventy-three, 2026-10-08, an instruments round. ADRs 1766, 1767; no row, no question.

**Premise.** `grep -c 'batch=batch-1456-1461' /home/AI/heavy-walk.log` gives 73 lines, not 39 (the
merge's 33 and the export came after the brief), and `kind=long` 0: held. **ADR 1756's prediction is
not tested by batch seventy-two, which did not hold as the contract read it**: the batch ran one 23.2 s
seed census and no campaign, so built, `--long` and `--long` with the turn each replay the rounds at
5 862.4 s against 5 885.0 s observed (31 of 37 lines within 1 s), no clock turn was taken and nothing
queued behind a long hold. ADR 1706's re-ask holds: one lane less the built rule, 6 944.1 s.
**Found instead**: `tools/batch.sh check`'s `cargo check` of the fuzz workspace, a large walk by ADR
1710, queued 4 659.8 s for 6.6 s of hold, 44% of the batch's 10 544.8 s, losing the pollers' race on
the first lane. Its fifteen lines peak at 0.02 to 1.58 GiB, so it is now `--tree 6` (ADR 1766):
replayed, 1 307.8 s. Batch seventy-three's 16 lines at this END hold no `kind=long`; its first check,
large, queued 1 061.6 s behind the export, this round's, small, 325.2 s behind both lanes' holders.

**Taken from slot 1** (round 1462): the rule line pinned `RAYON_NUM_THREADS=4` where the wrapper gave
a locked run `nproc / --shards`, 24 here. The four is right — the lanes' ceilings were measured at it
— so `tools/bounded.sh` gives every `--lock` run four threads, a caller's setting kept, and the rule
line drops the prefix (ADR 1766 section 4).

**Built besides.** `tools/comment-history.py` kept and restated as the reading of `CLAUDE.md`'s grep:
its `history` class read 22, every one this program's or `setsid`'s session; the article forms move to
`unread`, a quotation is a mention, two legitimate shapes join, and it reads 0 (ADR 1767). Round
1460's `pgrep -f` wait-loop is habit 74's incident sentence: no trap carries the rule line, so the
habit that states it does. ADR 1756's `table71.py` crashed on any batch but seventy-one; it runs on
both, kept at `/home/AI/lock-replay-1466/` with `check72.py`.

**Calibrated** (trap 13): the check's `kind=small lane=2` assertion failed on the old script
(`kind=large lane=1`); the threads test reads 24 from HEAD's wrapper; fourteen of fifteen planted
comment lines classify as planted, the fifteenth by an older `LEGITIMATE` phrase. Records 1456–1461:
39 to 40 lines, each with `**Gates.**`. `doc/todo/65` re-derived: 19 `partial`, 4 `reported`, each
named, no status changed, no edit.

**Gates.** `bash -n` on `tools/bounded.sh`, `batch.sh`, `state.sh`: exit 0. `tools/bounded.sh
--self-test`: exit 0, every case. `cargo test -p conformance --no-fail-fast`: exit 0, 426 passed.
`RUSTFLAGS="-D warnings" cargo clippy -p conformance --all-targets`: exit 0; rustfmt `--check` on
`bounded.rs`, `batch.rs`: exit 0. `tools/batch.sh check`: exit 1, its two lines siblings' mid-edit —
the fuzz `script` target's `view::Layer` missing `intent`, and `pdf-model/src/lib.rs`'s format.
Duration 4 650 s.
