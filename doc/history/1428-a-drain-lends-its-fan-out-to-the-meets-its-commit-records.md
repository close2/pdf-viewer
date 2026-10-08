# 1428 — A drain lends its fan-out to the meets its commit records

Pixels slot of batch sixty-seven. ADR 1692; ADR 1693 and Q324 not used. No ledger row moved
(§10.7.4's row is `departed` and unchanged: no byte moved).

**Built** (ADR 1692 sections 1 to 3). A probe showed the Type 3 page drains once, at the frame's end,
and records all its 60 exact meets in that drain's commit, after the fan-out's threads were joined;
ADR 1541's helpers then started a second set. The commit now runs inside the fan-out's scope, and a
drain with a job under a residue clip lends its threads to the frame's meets until it lets them go.
The frame's own helpers start only for a meet given with no thread claiming. Per frame on the Type 3
page: 14 → 7 threads pinned and 46 → 23 at 24 threads, on turn, seventh and step. Its step reads
11.01–11.45 ms pinned against 11.20–11.95, every row inside HEAD's spread. The first version kept
every drain's lent threads until nothing was given. That held `bug1721218_reduced.pdf`'s walk at the
scope's join: turn 145.79–148.35 against 139.69–141.92. So only the frame's last drain finishes its
backlog, and after any other the backlog goes to the frame's own helpers. That page keeps its 37 (114)
threads, its turn 139.73–141.53 against 139.44–142.10.
**Named** (section 4). The photograph's 23 (7 pinned) are `raster/reduce.rs`'s row split, called
from `device/resident.rs` at realisation. It is one scope of 48 bands a turn and a seventh frame,
none a step, and the frame's only set. Folding it into the encode's would be ADR 1686's frame pool.
**Taken from slot 6**: `crates/render-raster/examples/image_phase.rs`'s comment citing round 1097 now
states its current reason. That round wrote no ADR, and the comment already cites ADR 1107 section 3.
The file is off `tools/conformance/tests/round_numbers.rs`'s held list, which is now empty.

**Premise.** Held: 46 threads (14 pinned) on every Type 3 row, in two sets, and 23 (7) on the
photograph that stay without the encode's fan-out. Not held: that one set is the whole story.
`bug1721218_reduced.pdf` records meets after its drain, and those need threads of their own.

**Gates.** `rustfmt --check --edition 2024` on the ten `.rs` files: 0. `cargo clippy -p raster-gpu
--all-targets -- -D warnings`: 0. `RUSTFLAGS="-D warnings" cargo clippy -p render-raster
--all-targets`: 0. `cargo nextest run -p raster-gpu`: 0, 706 passed, 2 skipped. On
`encode_thread_sets`, the old behaviour planted back reads 14 against 7. `-p render-raster`: 0, 100
passed, 3 skipped. `cargo test -p conformance`: 0, 408 passed. `--test round_numbers`: 0. Tier 2, from
an export of HEAD plus this diff in its own target directory: six corpus arms all exit 0. Against
`/home/AI/arms-1426/`, 0 pages differ by digest on each arm (968, 968, 968, 964, 963 and 964 pages),
and a planted digest is named. One encode thread against 24 on `cpu` 1×: 0 of 968 differ.
`raster_golden`: 0, held 974, moved 0. `turn_path` twice on the final build (`--clock`): 0 and 0, 33
of 33 judged, 0 outside. `tools/batch.sh raster-examples`: 0, 14 passed, 0 failed.

**Unfinished.** `bug1721218_reduced.pdf` still starts two sets: the meets after its drain have no
drain to lend threads. No band moved, and `doc/checks/turn-path.toml` is unchanged.
