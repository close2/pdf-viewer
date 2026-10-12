# 1521 — CI's runner has four processors and Ubuntu's `glibc`, and three tests had only met this machine

An instruments round outside the batches, asked for by the owner on 2026-10-13: "please use [a]
round to check why the github CI doesn't build." Worked in a clone of `4c0c04f9` at
`/home/AI/ci-check`; nothing in the batch worktree was touched. ADR 1879. No ledger row moves.

**Found.** Every run since `8e1a7205` (2026-10-06) is red on one job, `test`, at `cargo test
--workspace`; CI's environment (secret, submodules, toolchain, runner image) is sound. The job was
re-run in an `ubuntu:24.04` container set up as the workflow sets the runner up, under `taskset` to
four processors. Three test binaries fail there and pass here: `pdf-transform --test writer` (two
renders planned at other strip rows, a pixel moved by a rounding level, ADR 0219), `raster-gpu
--test encode_thread_sets` (expects `THREADS - 1` whatever the machine lends), and `pdf-vfs --test
confined` (the probe is started without `MALLOC_ARENA_MAX=1`, and Ubuntu's `glibc` opens
`/proc/sys/vm/overcommit_memory` inside the confinement — `SIGSYS`, seen under `strace`). `cargo
test` stops at the first failing binary, so CI only ever showed the first. The `nightly` job fails
too but is `continue-on-error`: Miri meets `crossbeam-epoch`'s retag through `reach.rs`'s
`par_iter` (ADR 0450's failure in a new place), and the fuzz build meets nightly's
`recursion_depth_exceeding_limit` lint on `pipeline/warm.rs:86` under `-D warnings`.

**Built.** `rendered` states one strip; `encode_thread_sets` expects the count the device computes;
`run_probe` starts the probe as the spawner starts a worker, and `serve.rs`'s comment no longer
says a one-thread process cannot reach the allocator's question; `reach.rs`'s seven tests decline
under Miri with `blending.rs`'s reason; `raster-gpu` states `recursion_limit = "256"`.

**Not done.** The raw logs were not read: the token fetch was refused by the agent's permission
system, so every error line is from re-running the jobs. The "Fuzz briefly" step was not run.
`function_conformance` on this machine's GL adapter (`radeonsi`) reads `mod/negative-dividend` as 2
where the reference says -2 — not CI's, CI has no GPU; a real disagreement left to a pixels round.

**Gates.** In the container on the patched tree: `cargo test --workspace` exit 0 (6 973 passed, 47
ignored; 101 before). On this machine under `taskset` to 4 and to 24 processors: `writer`,
`encode_thread_sets`, `confined` exit 0 each. `cargo +nightly-2026-10-12 miri test -p pdf-render -p
pdf-syntax --lib` exit 0 (1 before); `cargo +nightly-2026-10-12 fuzz build --dev` under
`-D warnings` exit 0 (1 before); `cargo fmt --all --check` and clippy with `-D warnings` on every
changed crate exit 0. Duration 3 265 s.

**Owner.** Nothing required. `ubuntu-latest` becomes Ubuntu 26 on 2026-10-19 (the API says so);
pinning `ubuntu-24.04` would keep this reproduction valid and is the owner's call. The Miri step
took 38 minutes on run 38185269174 against the `nightly` job's 60.
