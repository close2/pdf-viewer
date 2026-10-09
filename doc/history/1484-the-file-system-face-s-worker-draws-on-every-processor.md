# 1484 — The file-system face's worker draws on every processor

Slot 5 of batch seventy-six, 2026-10-09, a measure round. ADR 1804; ADR 1805 and Q380 not used. No
ledger row moved: the contract was a named measurement, not a clause.

**Premise.** Held. Round 1470's figures and ADR 1554's are as quoted, with one correction: the
constant is in `serve.rs`, not `worker.rs`. **Hypothesis.** Half held. The walk did not shorten.
Pinned to the eight fast CPUs, `read_corpus` took 147.3 and 163.0 s on one thread and 155.1 and
163.0 s on eight. Unpinned it took 245.6 and 180.0 s against 160.0 and 237.8 s, which is the load,
not the width. No byte moved: all eight runs print the same counts, 3 217 renders their generator's
bytes and 0 not. The walk is (test + workers) ÷ 4. Its four lanes each ask, wait, then compute the
same file themselves (267–274 s of test time, 295–319 s of worker time). The question a face's user
waits on does shorten. `vfs_cost`, pinned, minimum of 15 asks: `issue19971.pdf`'s 150 dpi render
116.0 → 78.2 ms and its text 74.1 → 35.5 ms. No question on five documents is slower by more than
3%.

**Built.** `serve::confine` builds the pool as wide as `available_parallelism` answered before the
confinement wherever `MALLOC_ARENA_MAX` is set, and one thread wherever it is not. This is
`viewer-confined`'s arrangement. The brief's "width stays one" fallback is declined on principle 2:
latency ranks above a test's throughput (ADR 1804 section 2). `STRIPS` stays one. The pool's stacks
are stated and counted in the message budget. `WorkerLimits` carries the width. A probe holds the
width inside the confinement on ADR 0865's two-image page. One false sentence in
`confined-transport/src/host.rs`, a comment, now says what is.

**Gates.** Tier 1: `rustfmt --check --edition 2024` on the four Rust files, exit 0. `cargo clippy -D
warnings --all-targets -p pdf-vfs -p confined-transport`, exit 0, calibrated by a planted cast that
failed it. `cargo nextest run -p pdf-vfs -p confined-transport`: 92 passed, 2 skipped. The new
probe was planted one thread wide and failed with exit 17. `cargo test -p conformance`, exit 0.
Tier 2: the two A/B holds (`--clock`), exit 0, waiting 1 363.5 s and 102.0 s and holding 935.8 s and
701.6 s. Inside them, `write_corpus` ran at both widths, 974 documents, exit 0, the same counts.
On the built tree, the two walks ran as small walks: `read_corpus` exit 0, 194.0 s unpinned, the
same counts; `write_corpus` exit 0, 29.5 s. They waited 935.3 s and 0.0 s. `launch_path` is not owed.

**Unfinished.** Nothing of the contract. Adding lanes to the walk is what would shorten it, and that
is the lock's ceiling, not this crate's.

Duration 5 054 s.
