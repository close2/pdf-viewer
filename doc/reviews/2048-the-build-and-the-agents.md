# Review 2048 — the build and the agents

Session 2048, 2026-11-16. Asked by the owner: "please use one round to evaluate our setup. Can we for
instance improve on flags for the compiler (either to reduce disk-space, or to improve speed,...) Can we
do something to reduce token load. (compact required md files,...) Do not limit the round to the
mentioned subjects but take a step back and try to find things we can improve for the build or for the
agents."

Every number below was measured in this round unless it names its source. A claim with no number is
marked **hypothesis**. The load is printed beside each timing: the post-merge train's first run and the
round's own warm-up build shared the machine with every measurement, so a wall clock here is one sample
under load. Sizes and processor seconds are steadier, and the ranking rests on those.

## 1. How it was measured

- **Build**: cold `cargo test --workspace --no-run` (what `cargo nextest run --workspace` builds) into a
  private target directory, `CARGO_INCREMENTAL=0`, the pinned 1.97.1, inside a `systemd-run --user
  --scope` whose `memory.peak` and `cpu.stat` give the peak and the processor time; then `du` by kind and
  `readelf -S` summed over every executable. Settings were tried first on `-p pdf-model` (239
  integration-test binaries, 142 examples), one sample each, and the chosen one on the whole workspace.
  The scripts and their results stayed in the round's scratch directory; the census is kept as a tool.
- **Tokens**: the harness's transcripts of the 21 round agents of rounds 2029–2047 (two of them
  handover continuations), read by `tools/round-tokens.py` (ADR 2913), which this round adds so the
  census is a command and not a paragraph.
- **Gates**: the heavy-walk lock's log (`tools/places.sh`'s `HEAVY_WALK_LOG`), every hold of the heavy-walk lock since 2026-11-05.

## 2. Findings, ranked by expected saving

| # | finding | measured | change | where |
|---|---|---|---|---|
| 1 | debug sections are ~78 % of every linked program, and linked programs are 94 % of a build directory | §3.1 | `split-debuginfo = "unpacked"`: −40 % disk, −19 % processor time on the workspace | **built**, ADR 2912 |
| 2 | the same, compressed at link time | §3.1 | `--compress-debug-sections=zlib` beside `mold`: a further −58 % on `pdf-model` | orchestrator, §6 |
| 3 | the post-merge train's worktree has no submodules, so its first run failed every gate in ~30 s | §4.1 | the submodules linked from the main checkout | **fixed by the orchestrator** for the 20:30 run |
| 4 | rustdoc held the heavy-walk lock 64.7 h in 11 days | §4.2 | warm its dependencies outside `-j 2` | orchestrator, §6 |
| 5 | a round's test build links examples it never runs | §3.2 | `--lib --bins --tests` in a round's nextest | Q876 |
| 6 | 584 integration-test binaries, 239 in `pdf-model` | §3.3 | one binary per crate, piloted on one crate | Q876 |
| 7 | a cold build's wall with a warm `sccache` | §3.4 | wire it; never export `CARGO_TARGET_DIR` | orchestrator, §6 |
| 8 | a round's price is requests × context; the fixed reading is ~10 % of it | §5 | caps on output, fewer requests | Q877, §6 |
| 9 | peak memory of one cold workspace build: 44–51 GiB of a 66 GiB `MemoryHigh` | §3.5 | serialise cold builds across lanes | orchestrator, §6 |
| 10 | incremental compilation: no SIGSEGV in 8 builds on either compiler | §3.6 | incremental for a round's own rebuilds, not for gates | orchestrator, §6 |
| 11 | the release link's random LTO failure | §3.7 | no setting; keep the retries | none |

## 3. The build

### 3.1 Debug information (findings 1 and 2)

A cold workspace test build is **75 553 MiB**: `debug/deps` 57 185, `debug/examples` 17 865; 825
executables in `deps` are 53 255 MiB and 240 examples the rest, while the 522 `.rlib`s are 1 870 MiB.
`readelf` over the executables: `.debug_info` 23.5 GB, `.debug_str` 23.0, `.debug_line` 11.8,
`.debug_ranges` 11.0, `.text` 7.1 — each program carries the whole dependency graph's line tables.

Backtrace check, a crate that panics through an `#[inline(never)]` frame: `unpacked` and `zlib` keep
`at ./src/lib.rs:2:43`; `zstd` and `debug = 0` lose it (the standard library's symboliser reads no
zstd). `-p pdf-model`, cold, one sample each (load 34–175): as before 30 205 MiB / 211 s; zlib 12 611 /
238 s; zlib + unpacked 8 872 / 195 s; `debug = 0` 6 313 / 150 s.

Built: `split-debuginfo = "unpacked"` (ADR 2912). Whole workspace, cold: **45 149 MiB** (−40 %),
examples 9 646 MiB, **12 974 processor-seconds against 16 043 (−19 %)**, 486 s wall at load 50–125.
The workspace nextest under it: 9 970 passed, 73 skipped, 582 s.

Proposed (§6): `-C link-arg=-Wl,--compress-debug-sections=zlib` in the user-level Cargo configuration's
`rustflags`, beside `-fuse-ld=mold`. It is a link argument, so it is the machine's, not the manifest's.
Expected on top of the built change, by `pdf-model`'s ratio: the workspace's debug directory from ~45
to ~20 GiB (**hypothesis** for the workspace; measured for `pdf-model`). Cost: one cold build; link time
within the noise of one sample; `bench`'s profiles carry compressed sections, which `perf` and
`valgrind` read (**hypothesis**, check one callgrind run).

### 3.2 Examples in a round's build (finding 5)

`cargo test` and `cargo nextest` build every example of every selected package, and nextest runs none
of them; `pdf-model` has 142. Its tests built cold, one sample each: all targets 17 124 MiB, 3 318
processor-seconds, 152 s (load 63); `--lib --bins --tests` 12 408 MiB, 2 671 s, 138 s (load 83) —
**−28 % disk, −20 % processor time**. `clippy --all-targets` still type-checks every example, and the
train's workspace nextest still links them; an example's link error would surface at the merge. Asked
as Q876 because it changes what a gate builds.

### 3.3 One test binary per crate (finding 6)

584 files under `crates/*/tests/` and `tools/conformance/tests/` are each a binary that links the
crate's whole graph: `pdf-model` 239, `viewer-core` 70, `render-cpu` 44, conformance 42. Merging each
crate's into one binary (`tests/it/main.rs` with modules) removes most of those links and most of what
§3.1 still leaves. **Hypothesis** for its size: on the order of the executables' share, which §3.1 put
at 94 % of a directory. Cost: a large mechanical change; every `--test <name>` in `tools/state.sh`,
`doc/todo/02` and the briefs becomes a filter; a test's process is shared with its neighbours (globals,
`set_var`, working directory, panics that abort); and the 108 tests that find a program from their own
path (ADR 2910) move with it — so it belongs beside the Cargo 1.100 layout repair, not before it.

### 3.4 `sccache` (finding 7)

A private cache (`SCCACHE_DIR`, own port), the workspace build twice with the target directory named
on the command line: fill **539 s** (load 20→110), then a deleted target directory and a full cache
**351 s** (load 87→86), **−35 %**. The cache is **641 MiB**. 547 of 548 cacheable compilations hit;
**1 213 of 1 775 calls are not cacheable** (`crate-type`: every test harness, binary and proc-macro),
which is why it cannot do more: the links and the harnesses are most of the work (§3.1). Costs: it is
keyed on every `CARGO_*` variable, so the post-merge train's exported `CARGO_TARGET_DIR` gives it a
namespace no round reads (`doc/environment.md` has the rule); and a cache would keep an artefact a
faulty compile wrote (§3.7) until evicted.

### 3.5 Memory (finding 9)

The build's scope peaked at **44 104 MiB** (base) and **51 269 MiB** (unpacked) and **47 356 MiB**
(sccache fill) at Cargo's default 32 jobs. Two lanes' cold builds together pass the 66 GiB `MemoryHigh`
with no swap. Nothing measured here says which setting is safe; the rule that follows is to start a
lane's cold build only when no other cold build runs (**hypothesis** for the threshold; the peaks are
measured).

### 3.6 Incremental compilation under 1.100.0 (finding 10)

`-p pdf-model`'s tests, a cold build and then three rebuilds each after appending a public function to
`pdf-syntax/src/lib.rs` (so every dependent recompiles), under the tree's profile:

| | cold | rebuilds | `incremental/` | SIGSEGV |
|---|---|---|---|---|
| 1.97.1, `CARGO_INCREMENTAL=1` | 177 s | 112, 94, 115 s | 5.7 GiB | 0 of 4 |
| 1.100.0, `CARGO_INCREMENTAL=1` | 213 s | 95, 90, 81 s | 2.6 GiB | 0 of 4 |
| 1.97.1, `CARGO_INCREMENTAL=0` | 166 s | 160 s | — | 0 of 2 |

Load 63–219 throughout. No crash in eight incremental builds, on either compiler; the SIGSEGV that made
`CARGO_INCREMENTAL=0` a habit is recorded in no ADR, trap or habit file of the tree, only in briefs, and
memory `release-lto-random-corruption` puts the libLLVM SIGSEGVs of 2026-11-04/05 beside an in-process
memory fault. So the habit rests on evidence this round cannot reproduce. Incremental makes a round's
own edit-and-rebuild loop **~35 % faster** after a deep edit, for 2.6–5.7 GiB on this one crate; a gate
gains nothing (its build is cold). Proposed (§6): gates and the train keep `CARGO_INCREMENTAL=0`; a
round's own loop may use incremental, and a SIGSEGV seen there is recorded as a trap with its compiler
version and command.

### 3.7 The release link's random LTO failure (finding 11)

The memory `release-lto-random-corruption` records the evidence: the same rlib linked in one process and
failed in its sibling within one cargo run, a different crate each time, beside rustc SIGSEGVs in
libLLVM. That is a fault in one process's memory, not a property of fat LTO, so no profile setting
removes it; thin LTO would shorten the exposure and costs `callgrind_open` +12.30 % (ADR 0666). The
kernel log of this boot holds no machine check and no rustc fault (it holds Q874's loader faults). The
train already retries the install three times. No change.

### 3.8 Two toolchains in one build directory

Round 2047's build directory reached 200 GiB because 1.97.1 and 1.100.0 artefacts shared it; nothing
prunes by toolchain. The bump round should delete `debug/` when it changes the pin, and a lane's
directory past 100 GiB is already pruned between rounds; no tool change is needed for a one-off.

## 4. The gates

### 4.1 The post-merge train's worktree had no submodules (finding 3)

The train's first run (20:00) created its worktree with `git worktree add` alone. `pdf-spec`'s build
script then panicked ("the Arlington PDF Model is missing"), so `tests`, `corpus`, `golden`, `selection`
and `accessibility` each exited 101 after 22–87 s. The orchestrator saw it and restarted the train at
20:30 with the gitignored inputs and the submodules linked from the main checkout, as `tools/batch.sh
open` does for a round. It still exports `CARGO_TARGET_DIR` although its `.cargo/config.toml` now names
the directory; that matters only if `sccache` is wired (§3.4).

### 4.2 Where the walk lock's hours went (finding 4)

Holds since 2026-11-05 that exited 0, by command: rustdoc over the workspace **112 holds, mean 2 079 s,
232 876 s in all**; `script_corpus` 21 × 3 549 s; `archive_corpus` (three forms) 94 holds, 83 958 s;
`awkward_classes` 21 × 757 s; the two viewer censuses ~300 s each. Rustdoc is 47 % of the 492 228 s
the lock held in all. Its `-j 2` is there because rustdoc's memory crossed the large lane's 12 GiB at four jobs
(ADR 2190); but at `-j 2` it also compiles every dependency's metadata two at a time. Proposed (§6): in
the train, warm the check-mode dependencies first with `cargo check --workspace` at the full job count,
then run the bounded `cargo doc -j 2`, which then documents only (**hypothesis**: that Cargo shares the
dependency units between the two commands, to verify with one run).

## 5. The agents' tokens

`tools/round-tokens.py` over 21 transcripts: **$416 at list rates, 3 630 requests**, a mean of $19.8.
Cache reads are 85–90 % of each round's price, so a round costs its request count times its mean
context. Decomposed for round 2047 (301 requests, final context 335 565 tokens): first request 23 316
(system prompt, tool schemas, `CLAUDE.md` 32 kB, the memory index 11.6 kB), tool results ~100k, the
model's own output (commands, edits and thinking) 152 884.

- **The required documents are not where it goes.** Over all 21: `state-of-play.md` was read 17 times
  for 13 kB in all, `environment.md` 84 kB, `todo/02` 205 kB, the two every-round files 256 kB — the
  reading list is ~30 kB, ~8k tokens, a round. `CLAUDE.md` plus the memory index ride in every request:
  ~11k tokens × ~170 requests ≈ $0.95 a round (5 %).
- **Large outputs are.** Results ≥ 4 000 characters, ADR 2372's limit: **488 of 3 730, 54 % of all
  result characters**; `sed -n` 2.96 M characters, `grep` 1.18 M, `cat` 0.69 M; `ledger.toml` alone
  355 kB in 143 calls. Cargo's own output (nextest, clippy, build) is about 1 %, so a summarising
  wrapper around cargo would save almost nothing — none was built.
- **Polls**: 228 requests (6.3 %, **7.2 % of the price**) were a `tools/wait.sh` that came back
  unfinished, nearly all waiting for a tier-2 walk; the train moved those walks out of rounds on
  2026-11-16, so most of them go with it. And the harness now moves a foreground command to the
  background after 180 s, while `tools/wait.sh`'s default wall is 240 s (seen twice this round).
- **The brief's habits list** is ~40 lines, ~4.5 kB, ~1.2k tokens: about $0.12 a round. Its cost is
  attention and staleness rather than tokens.

## 6. To the orchestrator

1. **The train** (§4.1): its submodule links are made only when the worktree is created; a submodule
   whose pin moves on `main` is then read at the main checkout's commit, which is `main` itself, so
   that holds. Drop its exported `CARGO_TARGET_DIR` before wiring `sccache`.
2. **Link-time compression** (§3.1): append `"-C", "link-arg=-Wl,--compress-debug-sections=zlib"` to
   the user-level Cargo configuration's `rustflags`. One cold build; then measure the workspace once.
3. **`sccache`** (§3.4): `build.rustc-wrapper = "sccache"` in that configuration, and drop the
   train's exported `CARGO_TARGET_DIR` (its `.cargo/config.toml` can name the directory, as a round's
   does). Expected −35 % of a cold round build's wall, for 0.6 GiB of cache.
4. **Rustdoc in the train** (§4.2): `cargo check --workspace` first, then the bounded `cargo doc`.
5. **Cold builds one at a time** across lanes (§3.5).
6. **The brief**: carry only what is new. The durable habits of the 2047 brief (the conformance file
   habit, one fixed `-p` set, `CARGO_INCREMENTAL=0`, fuzz check on public types, the audit-test rule,
   the GPU and memory notes) belong in `doc/habits/every-round.md`, which every round already reads; the
   tier-2 and `state.sh docs` ordering habits are void now that the train runs those. Target: the
   template's 60 + 25 lines (review 1401).
7. **`tools/wait.sh --wall 160`** in every brief, or a default under the harness's 180 s.
8. **Incremental in a round's own loop** (§3.6), gates and train unchanged; a SIGSEGV becomes a trap.
9. **Measure each change with `tools/round-tokens.py`** over the next rounds' transcripts rather than
   from this table.
