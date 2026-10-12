# 1516 — The second lane grants small walks while a clock run waits for the first

Slot 2 of batch eighty-two, 2026-10-12, an instruments round under the owner's performance target;
the partial-row word yields to it. ADRs 1869 and 1870; no question. No ledger row moves.

**Premise, held.** Review 1513's instrument findings stand on this log. `lanes.py` gives lane 2
idle behind the gate as 5 365, 3 545 and 9 179 s for batches 79–81. `tools/state.sh batches`
printed "no gate wall-time line" for all eight batches since 74, because the bodies write "gates
green". **Hypothesis, held**: the clock run claims lane 1 while it waits, takes lane 2 only once it
has lane 1, and small walks get lane 2 meanwhile; the direct cost is 84 s over three batches. It
missed that an earlier long ticket also holds lane 2 idle, so one is ignored while the turn is held.
The ceiling is stated (600 s), not enforced; at 300 s batch 80's clock runs would pay 12 627 s.

**Built.** `tools/bounded.sh`: `<lock>.first`, the clock run's look for the first lane under the
gate, walks looking under the gate held shared, long tickets ignored while the turn is held,
`small_walk_overran`, self-test case 9 rebuilt (case 14 widened). `tools/state.sh`: `batches`
reads "N of M gates green"; `gates-cost` prints the small walks' queue while the second lane stood
free (9 035.6, 9 406.8, 19 705.7 s on batches 79–81). Both replaced by rename (trap 130).

**Figures.** `sidelane.py` (`/home/AI/lock-replay-1516/`) replays batches 79–81. It is calibrated
to 105/112 and 90/103 lines within 1 s, and to lanes.py's 9 179 s exactly. The rounds' walk queue
goes 33 526 → 25 784, 50 707 → 36 725 and 54 413 → 26 245 s. Live, this round's own clock run
waited 125 s for lane 1 while three small walks took lane 2, then 13 s for lane 2. Third item, one
`--clock` hold of 384 s, HEAD against HEAD without ADR 1861: the three opens' `open_kinstructions`
are all ADR 1861's (4 031.383 → 3 972.634, 13 041.702 → 12 752.380, 70 297.058 → 68 847.136
thousand). No turn figure is the lexer's; the one exception is `bug1721218`'s interpretation, at
1.85 ms of the 9.19 it moved. The 22 turn floors moved in the encode, the transfer or a decode,
each band's comment says which, and none is divided between the device-lane ADRs and the day.
**Premise miss**: the brief counted thirteen floors moved, and there are 27 (22 + 5).

**Files.** `tools/{bounded,state}.sh`, `tools/conformance/tests/{bounded,state_sections}.rs`, `doc/environment.md`,
`doc/todo/02-every-round.md`, `doc/checks/{turn-path,launch-path}.toml`, ADRs 1869, 1870, this record.

**Gates.** `bash -n` on both scripts: exit 0. `tools/bounded.sh --self-test`: every case holds, three
runs side by side, and five planted defects each fail case 9. `rustfmt --check --edition 2024` on the two test files: 0.
`RUSTFLAGS="-D warnings" cargo clippy -p conformance --all-targets`: 0. `cargo test -p conformance`: exit 0, 432 passed.
`tools/batch.sh check`: exit 1, on records 1516 and 1518 numbered behind CI's committed 1521 and on slot 6's `lines_across_views.rs` formatting; nothing of this round's. Build (`--tree 12`): queued 254 s, held 948 s, exit 0.
Clock hold: queued 139 s, held 384 s; `turn_path` 33 of 33 inside in four runs; `launch_path` exit
101 in both arms, on `bring_up_ms` (slot 3's) and one `first_page_ms` each. Duration 3 800 s.
