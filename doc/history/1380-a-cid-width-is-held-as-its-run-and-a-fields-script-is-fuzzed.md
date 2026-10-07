# 1380 — A CID's width is held as its run, and a field's script is fuzzed

Robustness slot of batch fifty-nine. Writes ADRs 1596 and 1597 and no question. No ledger row's
status moved. §9.7.4.3's note now names `pdf_font::runs`. The round was cut at 16:52 by the OOM
kill of the agents' scope and was resumed at 22:11.

**The cut.** My diff was read for spawners and holds none. My one process in 16:42–16:52 was
`seeds.sh check xfdf`'s `cargo fuzz build`, which was blocked on the build-directory lock from
16:47; its `-runs=0` pass is one process. At 16:50 this agent saw dozens of
`scratchpad/r1377/geiger/bin/cargo-geiger metadata` processes beside a load of 119. After the
resume, every run was behind the lock with `ulimit -u 8192`, every libFuzzer run was ONE process
(no `-fork` or `-jobs`; `page`'s line states `-fork=6`), and the user's thread total read 215
before the first run and 124 after it. Nothing was written to the main checkout's corpus or
artefacts.

**`/W` (ADR 1596).** `Runs<T>`: disjoint sorted runs, first statement wins, at most two runs per
statement. No budget, because the array bounds it. `issue16553.pdf` page 1 104.4 → 62.3 M
instructions (widths 44.8 → 5.7 M). The CFF charset is inverted once: `issue215.pdf` 9.29 → 8.71
M. `post` searches 64 names before building its map: `S2.pdf` 42.15 → 40.91 M. Seeds in
`fuzz/seed_widths.py`.

**Warm open.** Ten runs at 22:17–22:19, at loads 12.1 → 3.1 (siblings resumed at 22:16). Eight
read 0.263–0.294 ms. Runs 3 and 5 read 0.580 and 0.617 at loads 6.7 and 5.7. The band stays, with
its reason in the file. Trap 110 is untested because no run was taken under a load of 2.

**Fuzz (ADR 1597).** `aform` target and `fuzz/seed_aform.py` (350 seeds). Twenty minutes each from
fresh seeds; edges: `page` 31 050 → 34 241, `forms_data` 1254 → 2589, `xfdf` 1912 → 3650,
`fetched_import` 6746 → 10 053, `aform` 1366 → 1896. No crash. Defect: a picture of one repeated
letter was quadratic, at 991 ms a call. It is now 9 ms and has a regression test. A second
`aform` run on the fix went 1899 → 2103 in 40.7 M executions. `page`'s one slow unit is a seed
(the Type 3 cycle `doc/todo/49` lists). `tools/state.sh fuzz` printed the same table before and
after.

**Gates.** rustfmt `--check` on the eight files: 0. Clippy `-D warnings` on `pdf-font` and
`pdf-model`: 0. Fuzz-crate fmt: 0. Fuzz-crate clippy: 0 after one fix. `cargo nextest run -p
pdf-font -p pdf-model`: 0, 2125 passed. `cargo test -p conformance`: 0, 390 passed. Tier 2 behind
the lock: all twelve lines 0, 499 s. The fuzz lock was aligned to the workspace's for three
packages.
