# 1661 — Time to first page with a cold page cache

Session 1661, slot 7 of batch 102, a "pixels" round; 2026-10-21. ADR 2158 (2159 unused, no Q).
Principle 2's "measured with a cold page cache", `doc/todo/42` §6. No ledger row: no clause cited.

**The premise that did not hold.** The gate already measured a cold-cache open: `cold_open_ms` drops
the copy's pages (`dd oflag=nocache`, which is `posix_fadvise(DONTNEED)`) before every sample. So a
`cold_cache_open_ms` would have been that band twice over. What was warm was time to first page, and
neither arm asked whether a drop had worked. **The hypothesis** (cold open two to five times warm)
held only on the signed row, at 5.3; the other five rows were 1.2 to 1.6.

**Built.** `crates/viewer-ui/tests/launch_path.rs`: `cold_first_page_ms`, which is the first-page
child with each sample prepared as a cold open's is. A residency read-back (`fincore`, as
`cache_left_kib`) runs after every drop, and anything left declines a cold figure as
`TheCacheStayed`. A cold frame must hash as the warm one does. There is a new unit test on `tmpfs`
residency. `doc/checks/launch-path.toml` has the paragraph on what the figure measures and what it
does not, plus six bands. `doc/todo/42` §6 is rewritten as what is.

**Measured** (one `--clock` hold, HEAD's export and this one interleaved, three runs each, ADR 2158
§3). On every row the cold first page stays inside the warm one's spread, because the document
thread finishes 7 to 19 ms before the device (level with it on the font row). Each band is the
row's warm band, not the rule's over these runs. All six runs had the carve-out full (464 MiB held
by others), and that state is declined (ADR 2158 §4).

**Calibration.** A 40 ms plant on the document thread put all six cold figures outside and named
them. The carve-out declined them, as it declined the warm ones, so a judged failure needs a run
with room. The gate run with its copies on `tmpfs` declined every cold figure not already declined
for load, saying the document was "still in the page cache".

**Not built.** No cold stage in `toolkit_launch/mod.rs`. It needs a copy, a third launch set and
bands in a file this slot does not own, and it would buy a term this gate already measures (ADR 2158
§6).

**Gates.** `rustfmt --check` (test file) and `cargo fmt --check -p viewer-ui`: 0. `clippy -p
viewer-ui --all-targets -D warnings`: 0, retried after a neighbour's `pdf-transform` errors. `cargo
nextest run -p viewer-ui`: 136 passed, 1 skipped. `cargo test -p conformance`: 422 passed, 1 failed
(`no_comment_names_a_round_by_number`, on siblings' `r1655_no_descriptor.rs` and `medium_price.rs`).
`tools/state.sh docs`: 0. `t2-launch_path`: HEAD exited 101, 101, 101 (2, 2, 1 outside, ADR 1969's
sub-ms second mode); this tree 0, 101, 101 (0, 1, 7 outside); plant 101; `tmpfs` 0. Arms not owed:
`cargo tree -i viewer-ui` is empty from `render-raster` and `pdf-sandbox`. Duration: 5 400 s.
