# 1911 — the machine-local places come from one file

Session 1911, a single round (instruments), the first on the server VM. ADRs 2656 and 2657; Q776
unused.

**What.** `tools/places.sh` defines every machine-local place the tools use: two roots,
`PDF_VIEWER_SCRATCH` and `PDF_VIEWER_AGENT`, a builds root under the first, and the lock, logs,
worktree, build directories, arms, briefs, saved .NET SDK and T.88 data as names under them, each
the environment's where set and otherwise derived from `$HOME`. `bounded.sh`, `batch.sh`,
`state.sh`, `round.sh`, `worktree.sh` and `dotnet-check.sh` source it; `t88_conformance.rs` reads
its variable through it; `bounded.sh --lock` makes the lock's directory on first use.
`main-checkout.py` defaults the agent account to the one running it and prints the main checkout's
own path; the `function_paint` spike reads this tree's corpus through its manifest directory.
Six navigational documents name the variables or "the main checkout" where they named a directory.

**Premise.** The census held: 56 lines in 10 files under `tools`, `crates` and `fuzz`, 3 in
`main-checkout.py` and the spike, and the six documents; widened to the whole tree but the records
(trap 25) it also finds `doc/orchestrator-handover.md`, `doc/third-party-data.md`, eleven question
files and raster documents, which the contract did not give this round.

**Hypothesis.** One flat marker list cannot recognise both the records' old pointers and what new
records will name: a wider marker would refuse ADR 2462's tilde path, written under no rule. Each
marker now carries the first session and ADR it holds (ADR 2657). The bare-`flock` reader also
refuses a lock spelled `"$HEAVY_WALK_LOCK"`, which it passed before; the planted lines show both.

**Gates.** `cargo fmt --check -p conformance -p pdf-sandbox -p raster-gpu` exit 0; `RUSTFLAGS="-D
warnings" cargo clippy -p conformance -p pdf-sandbox -p raster-gpu --all-targets` exit 0 (its first
run refused the calibration test at 123 lines; split in two). With no override exported:
`tools/bounded.sh --lock --round 1911 --tree 2 -- true` exit 0, its line in the new log;
`tools/round.sh instruments` exit 1 on the two failures it printed at open (the main checkout's
`target/`, CI on main); `tools/worktree.sh list` exit 0; `tools/dotnet-check.sh` exit 69, no SDK;
`tools/batch.sh check` exit 0; `tools/state.sh --round 1911 quick` exit 101, its one red the
conformance section's `fuzz_wrapper` (below). `cargo test -p conformance --no-fail-fast` exit 0, 69
binaries, 471 passed, run from the worktree's physical path; `cargo test -p pdf-sandbox --test
t88_conformance` exit 0, the data absent and said. Planted, a sentence naming an arms file under
the agent root by its variable: `--test evidence` exit 101 naming it; removed, exit 0.

**Found, not fixed.** `fuzz_wrapper`'s two tests fail when cargo runs from the worktree reached
through the home directory's `scratch` symlink: `tools/fuzz.sh` prints the logical path and the test
compares cargo's physical one. Run from the physical path, 3 passed. The documents above remain.
