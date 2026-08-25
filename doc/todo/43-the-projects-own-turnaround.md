# The project's own turnaround — what is left after 608 s became 268

Status: **open, and it is the *project's* performance rather than the *program's*.** The second item —
three gates said to have doubled — was taken in the four-hundred-and-forty-seventh and is **closed**:
one of the three had moved, for a reason that was not rendering at all, and the last section of this
file is what the bisect found.
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
| §5's release binaries | *re-derived in §1* | one fat-LTO link, and it is the whole critical path |
| the eight gate *runs* | 101.6 s | 31.2 text, 29.5 quorra, 25.5 oracle, 9.1 JPEG 2000, the rest under 4 |
| the gate binaries' compilation | 43.3 s | thin LTO over 16 units; the six `pdf-model` gates are 30.1 of it, quorra 13.0 |
| the workspace's tests | 21.9 s + 5.2 compile | was 235.7 |
| `pdfref-hayro` | 11.3 s | a binary whose *output* does not depend on our code at all |

## 1. §5's fat link is the single largest item — and it is **bought**, measured at last

**Answered in the seven-hundred-and-fifty-second session, and the answer is that it stays.** ADR
0666 has the measurement. This section had asked since ADR 0222 what the fat link is *for*, and
named the candidate itself: if `lto = "thin"` costs nothing measurable, `[profile.release]` could
follow the gates. It costs a great deal.

**The A/B needed no edit at all, which is why it should have been run 300 rounds ago.**
`[profile.gates]` *is* `[profile.release]` with `lto = "thin"` and `codegen-units = 16` — it
inherits everything else — so the two arms were already sitting in `Cargo.toml`, and two further
profiles separate the settings. Callgrind rather than a clock, because this file's own denominator
warning applies and the machine had three parallel rounds on it:

| against `release` (`fat`, 1) | `callgrind_open` | `callgrind_interpret` | `callgrind_rasterise` |
|---|---|---|---|
| `gates` (`thin`, 16) | **+12.30%** | +1.65% | +4.07% |
| `thin`, 1 | +4.06% | +1.84% | +0.67% |
| `fat`, 16 | +10.72% | −0.01% | −0.00% |

Exactly reproducible — three passes of each arm agreed to the instruction, so there is no spread
to hide behind and the rule against reading a difference smaller than one does not bite.

**Both settings are load-bearing, and on different paths**, which is the finding worth more than
the verdict. Cross-crate inlining is what the interpreter and the rasteriser gain: `fat, 16` is
`release` to within 0.01% on both, so `codegen-units` is irrelevant there. The single code
generation unit is what §7.5's cross-reference parse gains: `fat, 16` is +10.7% on it, and only
the two together reach the shipped number. So there is no cheaper combination — the obvious
compromise, `thin` at `codegen-units = 1`, is still +4.06% on the launch path's largest step.
Binaries are also about a **quarter** smaller under `release` than under `gates`.

**What it costs, re-derived, because the figures this section carried had decayed.** §5's six
binaries after touching one file in `pdf-model`: **94.5 s wall under `release` against 50.4 s
under `gates`**, and `viewer-ui`'s own unit is **93.59 s** of the first — it starts at 0.8 s and
*is* the critical path, every other binary linking underneath it and finishing 45 s earlier. The
66.4 s this section recorded is now 93.59, the 79.3 s in the table above is now 94.5 for twice as
many binaries, and `Cargo.toml`'s own comment said 78 s. All three are corrected. The load was 4
to 7 during the `release` arm and 27 during the `gates` arm, which biases *against* the arm that
won on time, so the 44 s gap is a floor rather than a ceiling.

**And the prize was priced against a cadence that no longer exists.** "A round would lose another
60 s" was written when §5 ran every round; ADR 0428 made it every fifth round and before any
measurement. So the trade on offer was ~44 s of a fifth of a round against 12.3% of every
`Document::open`, and `CLAUDE.md` principle 2 makes startup a first-class requirement. Declined,
with numbers, and this question is closed rather than open.

A cheaper half-measure that has not been measured either: §5 does not need to run when a round
changed nothing that reaches `viewer-ui`. Cargo already knows that and skips it; what it cannot
skip is the *relink* after any change to `pdf-model`, which is most rounds.

## 2. Caching *our* renders: priced and not taken

The project owner's own suggestion, and the reason it was not built is three measurements rather
than a preference:

- **Reference renders are already cached** (ADR 0020) and the oracle reports **6173 of 6189 from
  the cache, 99.7%**. That half is done.
- **The oracle's floor is one page.** Its wall clock is 24.5–25.7 s, and
  `22060_A1_01_Plans.pdf` page 1 alone was **8.6–9.8 s** of it. A cache saves nothing on the run
  that has to produce that page, which is every run that changed the code that produces it.
  **That witness is no longer the floor and the argument is unchanged**: the raster cache of the
  five-hundred-and-thirty-ninth session (ADR 0374) took its page-one interpretation from 58.7 G
  instructions to 6.5 G, and the oracle's *slowest pages* line names other documents now. Read the
  line rather than this sentence — it prints the five, run by run, and the point was never which
  page it is.
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

## A third denominator, opened by the four-hundred-and-forty-fifth and closed by the four-hundred-and-forty-seventh

The four-hundred-and-forty-fifth re-took `doc/HANDOVER.md`'s gate-timing column, which had been the
three-hundred-and-ninety-eighth's for forty-seven rounds, and reported that **three of ten timings had
roughly doubled** — the oracle, the quorra comparison and the corpus gate, which it noted were "exactly
the three that rasterise all 974 first pages". It handed over a hypothesis (§11.4.7's page group drawn
as two rasters since ADR 0262) and an instrument (bisect the window).

**The bisect was run and the answer is smaller and stranger than the handover.** ADR 0282 is the whole
argument; three things belong here because they are about *this file's* denominator.

- **Two of the three never moved.** Session 398's own commit `244b86a`, checked out beside `351bfed` in
  one sitting on one machine, gives corpus **4.6/4.6/4.7 s** against HEAD's **3.9/4.0/4.0** and quorra
  **34.6/34.5** against HEAD's **34.1/35.0**. The corpus gate is *faster* at HEAD. Neither end's
  reported figure reproduces; what reproduces is the difference between them being nothing. The page-group
  hypothesis is excluded, because the gates ADRs 0262 and 0275 touched are the two that did not move.
- **The one that moved is `92579c2`, and it is not rendering.** The four-hundred-and-seventh session added
  a second `#[ignore]`d test to `oracle.rs` — a derivation whose own doc comment says it "is not itself a
  gate" — and `doc/todo/02` §2 invokes that binary with `--ignored`, which un-ignores every test in it. The
  two then walked the corpus under `rayon` in one process. The derivation now declines unless
  `PDFVIEWER_ORACLE_SPREAD` is set: the oracle line goes **47.0/46.3/55.5 s → 24.4/25.1/30.2 s**,
  interleaved, three samples each.
- **The corpus gate was the wrong probe and this file named it.** It said the corpus gate "takes seconds,
  which makes it the right bisect probe". Cheap is not the same as sensitive; that probe shows this effect
  *backwards*, and a round following the instruction would have concluded there was nothing to find. The
  probe that worked was the expensive one, at about two minutes a step and six steps.

**What this leaves for the denominator this file owns.** The whole of `doc/todo/02` §2, end to end through
`tools/state.sh`, is now **2 m 37 s**. §1's fat link is again the single largest item in a round and is
**measured against what it buys since the seven-hundred-and-fifty-second** (ADR 0666), which is the
top of this file and is now an answer rather than a question.

**And one lesson that is this file's rather than the ADR's**: *no gate measures a gate*, which is what the
four-hundred-and-forty-fifth said, and it is still true. What the four-hundred-and-forty-seventh adds is
that a gate's own printed timings are not a substitute — the oracle's `processor time` and `slowest pages`
rows read a factor of two high for thirty-nine rounds, and one of them was quoted *in this file* as
evidence for the regression it was a symptom of. A number a gate prints about itself is only as good as
what else is in its process.

## The compiler cache, opened by the five-hundred-and-ninth

`sccache` has been the `rustc-wrapper` since the four-hundred-and-eighty-fourth session and had never
been read as a *turnaround* question, only as a curiosity with a bad hit rate. It belongs in this file
because a worktree round builds into a fresh directory, which is precisely the case a compilation cache
exists for — and this project had switched it off by accident. ADR 0344 is the measurement; three things
belong here because they are about a round's wall clock rather than about the cache.

- **An exported `CARGO_TARGET_DIR` is the switch, and `--target-dir` is the same thing without the
  cost.** `sccache` folds every `CARGO_*` environment variable into its Rust cache key, so a per-worktree
  export gives each round a cache nothing else will ever read. Same source, same warm cache, fresh target
  directory: **79.01 % of Rust compilations hit** when the directory is named on the command line and
  **0.00 %** when it is exported, three samples of each, exactly reproducible. This costs nothing to fix
  and needs no agreement from a parallel round.
- **The wall clock it buys is not measurable here, and that is a finding rather than a gap.** Three
  samples of each condition: 117.3 / 227.7 / 117.7 s cached against 167.3 / 156.5 / 218.6 s uncached. The
  spread inside a condition exceeds the difference between them, because several rounds share
  twenty-four cores. What is countable is 335 of 780 compiler invocations skipped. **This file's whole
  denominator has that problem** — every number above was taken on a machine that may or may not have had
  a neighbour on it, and none of them says which. A round re-taking any of them should say what else was
  running, and prefer a counter to a clock where one exists.
- **Three-eighths of a round's compilation is outside any compiler cache.** Binaries and test harnesses
  are refused by `sccache` (`crate-type`), and so is every workspace-member `clippy` check
  (`multiple input files`, because cargo composes `sccache clippy-driver rustc …` and the parser sees two
  inputs). That is the same population §1 and the gate-binary row are about: **the remaining prize is
  fewer and cheaper links, not a better cache**, and this is now measured rather than assumed.

## The denominator nobody had measured: what a round *reads* and which gates it *chooses*

Opened and half-closed in the five-hundred-and-ninety-third, on a measurement the project owner
took (ADR 0428). Every number above is about a *command*; this section is about the two costs
around them, and the owner's measurement is what put them here:

| step | cost |
|---|---|
| `fmt` + `clippy --workspace --all-targets` + `nextest --workspace` + doctests | 37 s |
| the eight corpus-scale gates, `pdfref-hayro` and the conformance gate | 120 s |
| **all of `doc/todo/02` §2** | ~2.6 min warm; ~4 min after touching `pdf-model` |
| §5's release binaries, after touching `pdf-model` | 95 s |
| a whole round, seventeen rounds measured | 24–82 min, mean ~50 |

**So §2 is about eight per cent of a round**, and this file's first four sections have been
optimising eight per cent. The other ninety-two is reading and choosing, and neither had ever been
measured or even named as a cost.

- **Reading.** `doc/HANDOVER.md` was ~850 lines and `doc/todo/02` ~420, and every round read both
  whole. Both are indexes now and the detail is reachable by what the round is *about* —
  `doc/state-of-play.md`, `doc/traps/`'s five groups, and `doc/todo/01`'s sweep catalogue. The
  every-round pair is 529 lines where it was 1336.
- **Choosing.** §2 now carries a change→gate map derived from the crate graph, with the full
  sequence kept for every fifth round, for any round that can change a pixel, and for every merge.
  §5's binaries follow the same cadence with one addition that is not a relaxation: **before any
  measurement, always.**
- **`tools/round.sh`** is the third piece and the one that makes the first two operative: it says
  which session is next, what this kind of round reads, which gates it needs, whether the full
  sequence is owed, and whether the four things a round has got wrong here are wrong now.

**What is left in this section** is a measurement rather than a change, and it is the same shape as
§1's: nobody has measured what the map *saves* across a run of rounds, or what it costs when it is
wrong. The instrument is the one this file already uses — a round's own wall clock, said with what
else was on the machine — and the tell that the map is too loose is a merge round finding something
the branch rounds did not.
