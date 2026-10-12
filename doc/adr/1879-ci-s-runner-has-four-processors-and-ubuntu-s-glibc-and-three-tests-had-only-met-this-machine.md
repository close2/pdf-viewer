# 1879 — CI's runner has four processors and Ubuntu's `glibc`, and three tests had only met this machine

Session 1521, an instruments round asked for by the owner on 2026-10-13 ("please use [a] round to
check why the github CI doesn't build"). Status: **accepted** and **built**. Context: ADRs 0218, 0219, 0450, 0865,
1554, 1758, 1804, 1841; trap 150's shape (an export has no corpora) turned the other way, a runner
that has fewer processors. Code: `rendered` in `crates/pdf-transform/tests/writer.rs`; the last
assertion of `raster/crates/raster-gpu/tests/encode_thread_sets.rs`; `run_probe` in
`crates/pdf-vfs/tests/confined.rs` and `ARENA_LIMIT_VARIABLE`'s comment in
`crates/pdf-vfs/src/serve.rs`; the tests of `crates/pdf-render/src/reach.rs`; the crate attributes of
`raster/crates/raster-gpu/src/lib.rs`. Cites no clause.

## 1. What was failing, found by running the job rather than reading about it

The last green `test` job is run 36533870995 (`4adfa689`, 2026-09-29); every push from `8e1a7205`
(2026-10-06) on failed it at `cargo test --workspace` and so never published a snapshot. The `nightly`
job has failed for longer, but it is `continue-on-error`, so it colours a run's job list and not its
conclusion. The raw logs need a token, so the `test` job was run again in an `ubuntu:24.04` container
with the job's own apt steps, the pinned 1.97.1, `RUSTFLAGS=-D warnings`, `WGPU_BACKEND=vulkan`, a
depth-1 checkout with its seven submodules, exactly the thirty files `doc/specifications.zip` holds,
no device but lavapipe, and `taskset` to four processors, as the runner has. Three targets failed
there, and none of them fails on this 24-processor machine:

| target | first error | cause |
|---|---|---|
| `pdf-transform --test writer` | `nothing outside /Rect changed`, left 1, right 0 | `rendered` asks `RenderPlan { strips: None }`, so `render_cpu::plan_strips` takes the count from `available_parallelism`. The two renders compared are two lists, the annotation's and the bare page's, and they plan cuts at other rows; a cut moves a pixel by a rounding level (ADR 0219, ADR 1758 measures it). It fails at 2 to 12 processors and passes at 1, 16 and 24. |
| `pdf-vfs --test confined` | `a_confined_generator_can_still_derive_every_file`: `ExitStatus(unix_wait_status(159))`, i.e. `SIGSYS` | Under `strace` the killed call is `openat("/proc/sys/vm/overcommit_memory")`: `glibc` shrinking a non-main arena's heap asks it once. `run_probe` starts the probe without `MALLOC_ARENA_MAX`, which `confined_transport::Host::start` sets for every real worker, so the probe's test thread and its pool thread each have an arena. On this machine's Arch `glibc` the probe passes, and why it does was not traced; in the container it is killed every time. |
| `raster-gpu --test encode_thread_sets` | left 3, right 7 | The device holds `encode_threads` to `available_parallelism` (`device/construct.rs`), and the test expects `THREADS - 1` = 7 whatever the machine has. It fails at 2, 4 and 6 processors and passes at 8. |

`cargo test` stops at the first failing binary, so the runner reported the first of these and the
other two were waiting behind it.

The `nightly` job, run with the nightly each run would have installed:

- **Miri** (`4c0c04f9`, nightly-2026-10-12): `Undefined Behavior: trying to retag from <…> for
  SharedReadWrite permission … crossbeam-epoch-0.9.20/src/internal.rs:562:9`, reached from
  `crates/pdf-render/src/reach.rs:73`, the `par_iter` of `MarkBounds::measure`. That is ADR 0450's
  crossbeam failure in a new place: every test of `reach.rs` measures a list on rayon's pool.
- **The fuzz build** (`849a5cfc` and `f9d609c2`): `error: overflow evaluating the requirement
  {closure@raster/crates/raster-gpu/src/pipeline/warm.rs:86:20}: Send`. It is the future-incompat lint
  `recursion_depth_exceeding_limit` (rust-lang/rust#159228), which nightly has and 1.97.1 does not,
  made an error by the workflow's `-D warnings`.

## 2. Decision

1. **`rendered` states one strip.** It is the test's instrument and not a claim about the strip
   split, and the comparison it serves is between two lists, which `RenderPlan::strips`' own comment
   ("`None` is left to a caller that compares two renders of its own, each side asking the same
   machine") did not foresee: the same machine is not the same plan for different lists.
2. **The thread count is expected the way the device computes it**, `THREADS` held to
   `available_parallelism`, less the calling thread.
3. **`run_probe` starts every probe with `MALLOC_ARENA_MAX=1`**, as the spawner starts the worker the
   probe stands for, and `serve.rs` no longer says a one-thread process "cannot reach the question at
   all": its arenas other than the main one can.
4. **Each test of `reach.rs` declines under Miri**, with the reason `blending.rs` already gives and a
   module comment saying why all seven do, as ADR 0450 decided the declination lives beside the test.
5. **`raster-gpu` raises its `recursion_limit` to 256**, the remedy the lint names. On 1.97.1 the
   attribute changes nothing.

## 3. Measured

- In the container, on the patched tree: `cargo test --workspace`, exit 0 (6973 passed, 0 failed, 47
  ignored); exit 101 on the unpatched tree, with the three failures above.
- On this machine, under `taskset` to 4 and to 24 processors: `writer` 10 passed, `encode_thread_sets`
  1 passed, `confined` 17 passed, at both.
- `cargo +nightly-2026-10-12 miri test -p pdf-render -p pdf-syntax --lib`: exit 0, 267 passed and 11
  ignored, 121 passed and 17 ignored (2157 s and 659 s under a loaded machine).
- `cargo +nightly-2026-10-12 fuzz build --dev` under `-D warnings`: exit 0.
- `cargo fmt --all --check` and `cargo clippy` with `-D warnings` on every changed crate: exit 0.

## 4. Left to its owner

- **The finding is the same as ADR 0450's and in a new form**: every gate here runs on 24 processors
  and Arch's `glibc`, and two tests encoded the first without saying so. A tier-1 line that runs
  `cargo nextest` once under `taskset -c 0-3` would have caught both; whether that is worth its cost is the merge's
  decision, not this round's.
- **The `nightly` job is close to its 60-minute ceiling**: the Miri step took 38 minutes on run
  38185269174, before the fuzz build and the fuzz runs.
- **`function_conformance` on a GL adapter**: with no Vulkan device visible and the real GPU reached
  through GL (`radeonsi`), `mod/negative-dividend` reads 2 where the reference says -2. That is not
  CI's (CI has no GPU, and lavapipe and llvmpipe both pass), but it is a real disagreement on this
  machine's GL path.

## 5. The first run after this ADR, and the one failure it had hidden

Run 38241435033 (`c6fb18fb`) passed the three targets above and failed one more, which `cargo test`
had never reached behind them: `tools/conformance/tests/batch.rs`'s
`check_compiles_the_fuzz_workspace_and_names_a_target_that_does_not`. It plants a type error in a
sandbox's fuzz target and asserts that `tools/batch.sh check` names it — `error[E0308]` under the
line `cargo check of the fuzz workspace fails`. The workflow sets `CARGO_TERM_COLOR: always`, so on
the runner cargo's line begins with a colour escape and the script's `grep -E '^(error|warning)'`
matches nothing; here, where nothing sets the variable and the pipe is not a terminal, cargo prints
plain lines and the test passes. Reproduced on this machine with the variable set, before and after
the fix. The `cargo check` the script runs now says `--color never`: the lines are read by a grep,
not by a person, and a reader's colour setting is not the script's to inherit.

The second instance was behind the first: run 38248537857 (`e2383644`) failed
`tools/conformance/tests/worker_features.rs`, whose `cargo tree -f '{p}|{f}'` lines arrived with the
`(*)` of a repeated node wrapped in escapes, so `thiserror`'s features read `std (*)` and the check
named a feature nobody turned on. That `cargo tree` says `--color never` too. The rule both share:
a cargo line a program parses is asked for uncoloured, whatever the environment says — and the
whole workspace's tests were run here under `CARGO_TERM_COLOR=always` before this was committed, so
that the third instance, if there is one, is found on this machine rather than on the runner.
