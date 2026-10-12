# 1851 — The merge runs the launch gate whole, and its line names each figure outside its band

Session 1507, an instruments round of batch eighty. Status: **accepted** and **built**. Context:
ADRs 0884, 0916, 0917, 1392, 1476, 1513, 1620, 1621, 1684, 1718, 1839 (section 4);
`doc/questions/A29`; traps 10, 13, 97, 109, 130. Code: `gates`, `gate_summary` and `clock_gates`
in `tools/batch.sh`; the per-figure loop, `outside_name` and the summary line of
`crates/viewer-ui/tests/launch_path.rs`; `SPAWNED_PROGRAMS` in `tools/conformance/tests/bounded.rs`;
the test `the_merge_runs_the_launch_gate_with_its_clocks_and_its_line_names_what_moved` in
`tools/conformance/tests/batch.rs`. Cites no clause.

## 1. What was missing, checked before anything was built

- **The merge did not run `launch_path`.** `gates()` held 33 lines, two builds and 31 gates, and
  `launch_path` was not one; ADR 1839 section 4 left that to this file's owner. Principle 2 says a
  launch regression fails the build, and once a batch the merge is the build.
- **"2 outside" named neither, and the reason is a filter.** The gate pushes a judged figure that is
  outside its band into the panic's message only. `tools/state.sh launch` keeps the lines that open
  `launch-path:`, so it printed the count and dropped the names with the panic; and `gates()`'s
  summary column takes the last `test result` line, which names nothing either. The run that read
  "2 outside" is batch seventy-nine's merge at 11:25:40 (exit 101); its output was not kept, and the
  orchestrator's copy `/home/AI/launch-1496.log` is the re-run after two bands moved, "0 outside".
- **The clock half had not been run since its bands were taken.** Every `launch_path` line in the
  records since round 1392 (2026-10-07) is the counted half, "31 figures banded": no
  `PDFVIEWER_LAUNCH_CLOCKS`, so the warm open, the cold open and the time to first page were printed
  and not judged. A `--clock` hold for the counted half buys nothing, since none of its figures can
  be moved by a neighbour.

## 2. Decision

1. **`gates()` runs `t2-launch_path` whole**: `env PDFVIEWER_LAUNCH_CLOCKS=1 cargo test --release -p
   viewer-ui --test launch_path`, named in `clock_gates`, after `turn_path`, with its two workers
   built ahead of it in its own profile: the sandbox's, which `require_the_sandbox` asks for, and the
   script stage's (ADR 1620). The merge's hold is the stated quiet moment `A29`'s option 2 asks for,
   so the clocks are judged there once a batch, and a round keeps the counted half, `tools/state.sh
   launch`. The gate's own guards still decline a clock figure the machine was unfit to judge.
2. **Every figure outside its band is a line of the report's prefix**, `OUTSIDE` when judged and
   `(not judged) outside:` when not, with its description, key, figure and band; and **the summary
   names them**, document file name, key, figure and band, `; `-separated.
3. **The merge's line for that gate is that summary**: `gate_summary` takes the gate's own summary
   where it printed one and the ordinary `test result` line otherwise, and `tools/batch.sh summary
   <gate>` runs it on standard input so a test can plant an output.
4. **`bounded.rs` knows the script worker as a spawned program** (`pdf_script_worker::WORKER_PROGRAM`
   in the test's source), so the new build is held to a gate that needs it and the gate to the build.

## 3. Measured, in one clock hold (queued 1 275 s behind the arms export, held 311.5 s)

- **Calibrated by planting** (trap 13): two `open_kinstructions` bands narrowed to one thousand
  instructions and restored (`cmp` after): the counted run said `2 outside: PDF20_AN001-BPC.pdf
  open_kinstructions 4031.114 not in 3989.000 .. 3990.000; opt_demo.pdf open_kinstructions 1369.326
  not in 1340.000 .. 1341.000`, and `tools/batch.sh summary t2-launch_path` returned that line.
- **The gate, once, at a load average of 2.7**: exit 101, 20.8 s of test, 53 figures banded, 2 not
  judged, **5 outside — every row's `warm_open_ms`**: 0.465 ms against 0.260–0.400, 1.259 against
  0.790–1.180, 5.820 against 3.700–5.600, 0.382 against 0.230–0.350, 0.143 against 0.080–0.140.
  HEAD's own export, in a second hold at a load of 11.7–17.3 that siblings' builds made, read 0.52,
  1.27, 6.01, 0.40 and 0.15 ms — the same side of every ceiling, printed and not judged, which a load
  inflates and so cannot clear HEAD, but nothing in it puts the figure on this batch. **So the
  merge's first `t2-launch_path` is expected red on the warm open.** The rest of that hold ran at a
  load of 24 to 52 and was stopped; it judged nothing.
- **Cost**: the clock run is about 21 s once built; what the merge pays beyond it is the release
  builds, each paid where a sibling has changed what it links: the sandbox worker 1–6 s, the script
  worker 93 and 201 s, the test 187 s — and 0.16 s when nothing had changed. The brief's "about
  450 s held" was holds that included builds and a queue.

## 4. Left to its owner

- **The warm open's five bands.** They are round 1392's of 2026-10-07 and nothing judged them since.
  Whether the figure moved with the tree or with the machine is an A/B of HEAD against `b87299eb` in
  one quiet hold; until then trap 97 is the rule — a slowdown nobody chose is a failing gate, and a
  band moved on purpose carries its reason beside it.
- A later build of the shared tree read `peak_anon_mib` under its floor on two rows (18.949 MiB
  against 19.5, 22.477 against 23.0, at 02:45): a sibling's change to what page one allocates, and
  the merge's to read.
