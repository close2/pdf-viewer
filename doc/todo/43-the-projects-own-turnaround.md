# The project's own turnaround — what is left after 608 s became 268

Status: **open, and it is the *project's* performance rather than the *program's*.**
Priority: 43

Everything else in the `40`–`49` band is about how fast `pdf-viewer` runs. This one is about how
fast a *round* runs, which is a different denominator with the same rule over it: principle 2's
"genuinely is decided by measurement, never by assumption" binds here exactly as it does to a
rasteriser, and every number below was printed rather than estimated.

ADR 0222 is the measurement and the four changes it justified. One round's `doc/todo/02` §2 + §5
sequence, after touching one file in `pdf-model`, went **607.9 s → 268.0 s** (two samples of the
result, 268.0 and 266.6). What is left, in the order the measurement ranks it:

| what | now | what it is |
|---|---:|---|
| §5's three release binaries | **79.3 s** | one fat-LTO link, 66.4 s of it in one codegen unit |
| the eight gate *runs* | 101.6 s | 31.2 text, 29.5 quorra, 25.5 oracle, 9.1 JPEG 2000, the rest under 4 |
| the gate binaries' compilation | 43.3 s | thin LTO over 16 units; the six `pdf-model` gates are 30.1 of it, quorra 13.0 |
| the workspace's tests | 21.9 s + 5.2 compile | was 235.7 |
| `pdfref-hayro` | 11.3 s | a binary whose *output* does not depend on our code at all |

## 1. §5's fat link is now the single largest item, and it is deliberate

66.4 s of a 76.9 s `cargo build --release --bin pdf-viewer` is one unit — `viewer-ui`'s binary,
fat link-time optimisation over `codegen-units = 1`, with `user` time equal to `real` time on a
24-core machine. `cargo build --timings` says so.

It stays until somebody measures what it is *for*. The question nobody has asked is what
`lto = "thin"` costs the launch path and the page-turn latency; if the answer is "nothing
measurable", `[profile.release]` could follow the gates and a round would lose another 60 s. That
is a **measurement task, not a change**: the numbers to take are `doc/todo/42`'s launch timeline
under `Xvfb` and the callgrind counters, both ways, and the rule against curve-fitting applies —
a difference smaller than the spread is not a difference. Until then the shipped binaries keep
the optimisation they have always had.

A cheaper half-measure that has not been measured either: §5 does not need to run when a round
changed nothing that reaches `viewer-ui`. Cargo already knows that and skips it; what it cannot
skip is the *relink* after any change to `pdf-model`, which is most rounds.

## 2. Caching *our* renders: priced and not taken

The project owner's own suggestion, and the reason it was not built is three measurements rather
than a preference:

- **Reference renders are already cached** (ADR 0020) and the oracle reports **6173 of 6189 from
  the cache, 99.7%**. That half is done.
- **The oracle's floor is one page.** Its wall clock is 24.5–25.7 s, and
  `22060_A1_01_Plans.pdf` page 1 alone is **8.6–9.8 s** of it. A cache saves nothing on the run
  that has to produce that page, which is every run that changed the code that produces it.
- **The key would have to cover the code, and the code is why the gate is running.** Trap 10a is
  this project's record of a stale cached render; the rule it earned is that a key which cannot
  lie names every input, and for our own output that includes the binary. Under `--profile gates`
  that binary is relinked on every round that touches `pdf-model`. The cache would miss exactly
  when it is wanted.

**What would change this**: a key that covers the code *at a finer grain than the binary* — say,
a hash of the crates a gate's answer can depend on, so that a round touching only `viewer-ui`
keeps the oracle's cache. That is a real design and not an obviously good one; it re-introduces
the thing trap 10a caught, in a form where "which crates can reach this pixel" is a judgement
rather than a fact. Do not build it without an argument for why that judgement cannot be wrong.

## 3. Three things measured and rejected, so they are not measured again

- **A faster linker.** `lld` is on this machine. The dev link is 5.2 s for about 120 binaries
  after `debug = "line-tables-only"` and `opt-level = 1`, and the release cost is LLVM's LTO
  rather than symbol resolution — 66.4 s of 76.9 inside one codegen unit, where a linker swap
  cannot reach. Worth measuring again only if the dev compile grows.
- **Sharing one build between `clippy` and the tests.** Cannot be done: `clippy-driver` is a
  different compiler and Cargo fingerprints it separately, by design. It also does not matter —
  incremental `cargo clippy --workspace --all-targets` is **3.3 s**, because it only checks.
- **`cargo clean` for speed.** The target directory was 311 GB and the warm no-op build was
  0.42 s. Sweeping it is disk hygiene, now `doc/todo/02` §5a, and buys no time.

## 4. What is not measured at all

- **CI's wall clock.** This round measured the *local* round only. CI runs `cargo test
  --workspace` on two cores, where nextest's whole advantage is small, and it never runs the
  `--ignored` corpus gates — so none of ADR 0222's four changes reaches it except the dev
  profile, which makes CI's compile slower and its test run faster by an unmeasured amount.
  Somebody should read one CI run's step timings before changing anything there.
- **The fuzzers and the nightly job**, which are outside §2 and were never timed.
