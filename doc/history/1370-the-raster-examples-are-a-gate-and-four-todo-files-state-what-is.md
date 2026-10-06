# 1370: the raster examples are a gate, and four todo files state what is

This round had the instruments slot of batch fifty-seven. It wrote ADRs 1575 and 1576, moved no
ledger row and wrote no question.

**`t2-raster_examples`** (ADR 1575). `tools/batch.sh raster-examples` runs CI's loop: the fourteen
names it reads out of `ci.yml`'s `--check` step, each run with `--check` under `xvfb-run`. It builds
first, under a timeout of its own, then runs each example under its own timeout, which ends the
example's whole process group. It prints one line per example and a summary, and its exit status
is the answer. `gates()` runs it behind the lock through `tools/bounded.sh --tree 12`. The ceiling
is 12 because the release build peaked at 7.85 GiB, too close to a build's 8. Measured behind the
lock, part-warm: **288 s**, of which the build was 210 s and the fourteen examples 77 s. All
fourteen pass. A run with a 3 s per-example timeout checked the failure path:
fourteen `exit=124` lines, `0 passed, 14 FAILED`, exit 1, and no Xvfb left running.
`tests/batch.rs` holds the names it reads against `examples/`. `doc/todo/02` section 2 names it
for `raster/crates/raster-gpu/src/`, as does the habit paragraph. `gates-cost` now names any gate
`gates()` runs that the log has no line for. Round 1366 runs the loop by hand; the lock orders us.

**The re-seed line.** `main-checkout`'s `fuzz/corpus` line gives the owner's command for every
unseeded or stale corpus. On the main checkout it names nine, not six: seven unseeded (`shaping`,
`jbig2`, `jpx`, `linearize`, `embed`, `vfs_write`, `find`) and two stale (`forms_data`,
`display_list`). The stale two come from record 1362's census, which the line names. A state
section writes nothing (ADR 1487), so `fuzz-stale` cannot leave a result behind for this line.

**Todo files** (ADR 1576). Lines spelling a round by ordinal: `00` 107 → 0 (1931 → 1616 lines;
step 7's run log became one block of what its runs taught), `03` 108 → 0, `02` 18 → 0, `38`
10 → 0 (six headings now name what was built). `doc/todo/` as a whole went 1109 → 472, counting
siblings' files. `spelled_ordinals.rs` prints that figure per file and does not hold it yet.
ADR 1576 section 3 says what holding it would take: about thirty files rewritten, then a falling
ratchet, then a zero ceiling, plus the lower-case digit form.

**Navigation.** The four documents were read against ADRs 1553–1563, and twelve sentences were
added or corrected. `crate-map`'s "112 entry points" was false, and the count is now a `grep`.
Records 1359–1364: 34–40 lines, each with Gates. `doc/todo/65`, re-derived after round 1369's
record, places the fourteen open rows once each and needed no change.

**Gates.** `rustfmt --check --edition 2024` on `tests/batch.rs` and `tests/spelled_ordinals.rs`:
exit 0. `bash -n` on `batch.sh` and `state.sh`: 0. `RUSTFLAGS=-D warnings cargo clippy -p conformance
--all-targets`: 0. `cargo test -p conformance`: 0, 52 binaries passed. The raster-examples gate
behind the lock: exit 0, 14 passed, 288 s.
