# 1575 — The raster examples are a gate of the merge

Session 1370. Status: **accepted**. Closes the gap ADR 1563 section 3 named; amends nothing.
Context: `CLAUDE.md` principle 1 (warnings are errors in CI) and principle 4; ADRs 0060 of
`raster/` (every example's `--check`), 0798 (the bound), 1392 (a gate that ran nothing), 1476 (the
two clocks), 1487 (a state section writes nothing), 1559 (a stale corpus), 1563.
Code: `tools/batch.sh` (`raster_example_names`, `raster_examples`, the `t2-raster_examples` line of
`gates()`), `tools/state.sh` (`gate_names`, `section_gates_cost`), `tools/main-checkout.py`
(`stale_corpora`, `unseeded`), `tools/conformance/tests/batch.rs`
(`the_raster_examples_gate_reads_every_example_ci_runs`), `doc/todo/02-every-round.md` section 2.

## 1. What was missing

`cargo test` builds no example, so every `assert!` under `raster/crates/raster-gpu/examples/` runs
only where something executes it, and only CI's `raster-examples` job did. ADR 1563 found two of the
fourteen stale there and nowhere else, and its section 3 says which gate would have caught them
here: none.

## 2. Decision

`tools/batch.sh raster-examples` runs CI's loop, and `gates()` runs it as `t2-raster_examples`,
behind the heavy-walk lock through `tools/bounded.sh`, beside `t2-turn_path`.

- **The list is CI's, read and not copied.** The names come out of the `for example in` loop of
  `.github/workflows/ci.yml`. `raster-gpu`'s `tests/example_checks.rs` holds that loop to
  `examples/`, and `tests/batch.rs` holds this reading to the same directory, so a loop reformatted
  past the `awk` fails a test rather than leaving a green gate that ran fewer examples.
- **The build and the runs are bounded apart.** `cargo build --release -p raster-gpu --examples`
  first, under `timeout` (2400 s), then each `xvfb-run -a cargo run --release … -- --check` under
  its own (600 s). A slow build therefore never reads as a hung example. `timeout` signals its whole
  process group, so `xvfb-run`, its server and the example stop together.
- **One line per example, and the exit status is the answer.** Each line gives the example's exit,
  its seconds and its log; the summary reads `raster examples: N passed, M failed`, and the gate
  exits non-zero when one fails. `run` keeps the output's last thirty lines as the `.fail.` file,
  and those are the per-example lines. Logs go beside the gate log, in `<log>.raster-examples/`.
- **`release`, as CI builds them, and a walk's tree ceiling of 12 GiB.** Measured: the release
  build peaked at 7.85 GiB over the process tree, inside a build's 8 GiB by too little to survive a
  heavier dependency.

**Its cost, measured** on 2026-10-06 with the release directory part-warm and a load average of
about 6: wall 288 s; the build 210 s; the fourteen examples 77 s together, `function_paint` the
dearest at 23 s, `encode_threads` 12 s and the other twelve 3–5 s. All fourteen pass. With the build
warm the gate costs about the examples' 77 s.

## 3. The two instruments beside it

- **`tools/state.sh gates-cost` names a gate `gates()` runs that the log has no line for.** The
  ranking is of lines the merge's last run wrote, so a gate added since then, this one first, was
  absent from it without a word. `gate_names` reads `gates()` with its two loops expanded, and the
  section's last line counts both lists.
- **`tools/state.sh main-checkout`'s `fuzz/corpus` line carries the owner's re-seed command** for
  every target with no seeds on the disk, plus every target the last census found stale. The census
  is `fuzz-stale` (ADR 1559), and a state section writes nothing (ADR 1487), so it cannot leave a
  result for this line to read. The stale targets therefore come from the newest record whose
  `**Stale today:**` sentence names targets, and the line names that record. A record is a census of
  its day, which is why it is named rather than trusted silently.
