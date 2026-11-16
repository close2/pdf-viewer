# Q876 — May a round's test build leave out the examples, and may tests become one binary per crate?

Source: round 2048, your evaluation round ("Can we for instance improve on flags for the compiler
(either to reduce disk-space, or to improve speed,...)"). The review is
`doc/reviews/2048-the-build-and-the-agents.md` §3; ADR 2912 is the part built without asking.

Both parts change what a gate builds, which is why they are asked rather than done.

## 1. The examples

`cargo nextest run -p <crate>`, a round's gate for each crate it touches, builds every example of that
crate and runs none of them: nextest runs tests, and `cargo test` runs no example either. `pdf-model`
has 142 examples. Its tests built cold under the tree's profile (ADR 2912), one sample each:

| | build directory | processor time | wall | load |
|---|---|---|---|---|
| `cargo test --no-run -p pdf-model` | 17 124 MiB | 3 318 s | 152 s | 63 |
| the same with `--lib --bins --tests` | 12 408 MiB | 2 671 s | 138 s | 83 |

So −28 % of the directory and −20 % of the processor time for that crate.
What it would lose: an example's link error would surface at the merge instead of in the round. Its
compile errors would not — `clippy --all-targets`, which every round runs, type-checks examples — and
the post-merge train's `cargo nextest run --workspace` still builds them all.

## 2. One integration-test binary per crate

There are 584 integration-test files, each its own binary linking the crate's whole graph: `pdf-model`
239, `viewer-core` 70, `render-cpu` 44, `tools/conformance` 42. Linked programs are 94 % of a build
directory (ADR 2912), so merging each crate's tests into one binary is the largest saving left, but it
is not measured: it cannot be without doing it. Its costs are certain: a large mechanical change; every
`--test <name>` in `tools/state.sh`, `doc/todo/02`, CI and the briefs becomes a name filter; tests that
share a process share its globals, environment and working directory; and the 108 tests that find a
program from their own path (ADR 2910) change with it.

## Recommendation

1. **Yes**: a round's nextest of touched crates runs `--lib --bins --tests`, and `doc/todo/02`'s tier 1
   says so, with the merge's train as the place an example's link is checked.
2. **A pilot, not the whole**: one round merges `render-cpu`'s 44 tests into one binary, measures size,
   build and run time against today, and reports; the decision for `pdf-model` follows the number. Do it
   with or after the Cargo 1.100 layout repair (ADR 2910), since both move how a test finds its
   neighbours.
