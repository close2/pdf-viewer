# 1460 — A long run holds the second lane only, and a clock run behind it stops no walk

Slot 5 of batch seventy-two, 2026-10-08, an instruments round. ADR 1756; no row moved, no question.

**Premise.** The log held it: round 1452's corpus gate `wait=4289.1s` behind round 1455's census and
`large-hold.sh`, which held both lanes from 14:27 to 15:23 for all but 173 s. **That pinning is the
cure did not hold.** ADR 1706's replay, taught the kind, gives batch seventy-one's nineteen other
walks 21 217.8 s with every campaign long against 23 846.5 s built, worst 5 622.2 s: long runs fill
the second lane end to end, a clock run waits at the gate for each, and the gate stops the first lane
meanwhile. So a clock run now takes a clock turn and waits for a long run without the gate:
7 646.3 s, none over 1 800 s; the costs are a fuzz round's one lane and a clock run's wait for the
long run it found (ADR 1756 sections 3 and 4).

**Built.** `tools/bounded.sh --long`: lane 2 only, `--tree` 6 by default and refused above, refused
without `--lock`, beside `--clock`, inside a hold of lane 1 and under its bare `flock`; `kind=long`;
the turn `<lock>.clock` and the mark `<lock>.lane2.long`; self-test cases 11 and 12. `tools/state.sh`:
`walk long`; `fuzz-stale`, which ran its census unlocked, walks `long` and `large` (`jbig2 jpx`);
`gates-cost` lists each long hold and what queued behind it. To `--long`: `fuzz/seeds.sh`'s header,
`doc/verify.md`'s census and campaign lines, `tools/main-checkout.py`'s re-seed line, the rule line.
**Taken from slot 2**: the rule line's `--build` spelled as cargo takes it, held by a sweep. **From
slot 1**: `tools/state.sh batches` read a grouped figure's last group (2 135 s for 47 135 s) and no
gate line; it joins groups first, behind `--batch-figures`, held to four bodies summed by hand.

**Calibrated** (trap 13). Planted in copies of the script: a long run allowed lane 1 fails case 11; a
long run allowed inside lane 1 fails its refusal; a clock run holding the gate while it waits fails
case 12 on the small walk (63.4 s). In the tree: `verify.md`'s campaign line without `--long`, and the
rule line's old `<profile>`, are each named by their sweep. Records 1450–1455: 33 to 40 lines, each
with `**Gates.**`. `doc/todo/65` re-derived after slot 4: 19 `partial`, 4 `reported`, no status line
changed, so no edit.

**Unfinished.** `doc/todo/02` section 0 and trap 10's row still spell `<profile>` and name no
`--long`: slot 1's prose and the traps, held by the sweep's ratchet.

**Gates.** `bash -n` on `tools/bounded.sh`, `state.sh`, `batch.sh`, `fuzz/seeds.sh`: exit 0.
`tools/bounded.sh --self-test`: exit 0, twelve cases. `cargo test -p conformance --test bounded`: 18
passed; `--test state_sections`: 5 passed; `--test round_numbers`: 2 passed; `cargo test -p
conformance --no-fail-fast`: 423 passed, 1 failed, `records.rs` on 1459's and 1461's records
mid-write. `RUSTFLAGS="-D warnings" cargo clippy -p conformance --all-targets`: exit 0; rustfmt:
exit 0. `tools/batch.sh check`: exit 1, its one failing line the same records (this one's then
unwritten); the fuzz workspace's check clean, `cargo fmt --all --check` clean. Duration 5 900 s.
