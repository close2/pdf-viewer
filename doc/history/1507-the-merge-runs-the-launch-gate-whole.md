# 1507 — The merge runs the launch gate whole, and its line names each figure outside its band

Slot 6 of batch eighty, 2026-10-12, instruments. ADR 1851; ADR 1852 and Q403 not used. No ledger
row moved: the partial-row word yields to the owner's performance target; no clause was touched.

**Premise.** Held: `gates()` ran 33 lines, none `launch_path`; the "2 outside" run was the 11:25:40
merge run (exit 101, output not kept; `launch-1496.log` is the re-run), its names in the panic's
message, which `state.sh launch`'s `^launch-path:` filter drops. "About 450 s held" did not: that
was builds and queue; the clock run is 20.8 s once built.

**Hypothesis.** Held: the per-figure judgement had name, figure and band, and now prints each outside
figure under the report's prefix and names them in the summary. The merge pays its release builds
(1–6 s, 93–201 s, 187 s where a sibling changed what they link, 0.16 s where not).

**Built.** `t2-launch_path` in `gates()` and `clock_gates` with `PDFVIEWER_LAUNCH_CLOCKS=1` (a clock
hold for the counted half judges nothing a neighbour moves; the merge's is `A29`'s quiet moment),
after two release worker builds; `gate_summary`, `tools/batch.sh summary <gate>`; the script worker in
`SPAWNED_PROGRAMS`. `A393` transcribed, principle 2's paragraph amended, ADR 1831 ratified.

**Measured** (one clock hold, queued 1 275 s, held 311.5 s): two `open_kinstructions` bands planted
narrow and restored gave `2 outside: PDF20_AN001-BPC.pdf open_kinstructions 4031.114 not in 3989.000
.. 3990.000; opt_demo.pdf …` as the log's line. **The gate at load 2.7: exit 101, 53 banded, 5 outside,
every row's `warm_open_ms`** (0.465 ms against 0.260–0.400 … 0.143 against 0.080–0.140); HEAD's
export at load 11.7–17.3 read the same side, unjudged. Before: no merge line, "N outside"; after: named.

**Unfinished** (ADR 1851 §4): the warm open's bands, unjudged since 2026-10-07, on which the merge's
first `t2-launch_path` is expected red; a quiet A/B of HEAD against `b87299eb` decides tree or machine
(siblings' builds at load 24–52 prevented one). `peak_anon_mib` under two floors is a sibling's.

**Files.** `tools/batch.sh`, `tools/state.sh`, `crates/viewer-ui/tests/launch_path.rs`, `tools/conformance/
tests/{batch,bounded}.rs`, `doc/todo/02` (§2, §8), `CLAUDE.md`, `doc/PLAN.md`, `doc/state-of-play.md`,
`doc/questions/A393-…`, ADR 1831's status line, ADR 1851, this record.

**Gates.** `bash -n` on both scripts: exit 0. rustfmt on the three `.rs`: clean. `RUSTFLAGS="-D
warnings" cargo clippy -p conformance --all-targets`: exit 0; `-p viewer-ui` exit 101 on a sibling's
`render-cpu` (`draw_stroke` 103 lines), so run on HEAD's export with my test file: exit 0. `cargo
nextest run -p conformance`: 431 passed; `-p viewer-ui`: 178 passed, 2 failed in a sibling's mid-edit
`bin/quorra/stale.rs`. `cargo test -p conformance`: 429 passed, 2 failed, both on siblings' unfinished
records (1502, 1503, 1508), `batch`, `bounded`, `questions` green (the wrapper's self-test failed once
at load 50, green alone). `tools/batch.sh check`: exit 0. The new gate once (`--clock`): exit 101.
