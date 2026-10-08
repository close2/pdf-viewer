# 1734 — A neighbour stretches a walk by a fifth at most, and the golden draws by the cores it is given

Session 1449. Status: **accepted**. Takes the figure ADR 1706 section 5 named and the log could not
give: two walks, each timed alone and beside a declared small walk in one sitting. Decides nothing
new in `tools/bounded.sh`: the second lane stays, and nothing is handed to the instruments round.
Records a finding about `raster_golden` that is not this round's to build (section 4).
Supersedes nothing.
Context: ADRs 0219, 1577, 1684, 1706; habit 36 of `doc/habits/measuring.md`; traps 13, 50.
Instrument: `scratchpad/r1449/` (`one.sh`, `sitting.sh`, `neighbour-alone.sh`), kept at
`/home/AI/lock-stretch-1449/` with every run's output and the binaries' `md5sum`.

## 1. The method

- **The binaries are copies**: `raster_golden`, the Tier 1 column (`pdf-script --features engine
  --test script_corpus`), `pdf-model --test corpus` and `pdf-sandbox-worker`, built once under
  `--profile gates` from the worktree at 11:52 and copied out (trap 50), so a sibling's later edit
  cannot reach an arm and cargo's freshness check is not inside a figure.
- **Every run is one `--clock` hold of both lanes**, so no other walk on the lock ran during any
  figure. Beside: inside the same hold the neighbour starts first, `pdf-model --test corpus` looped
  under `tools/bounded.sh --tree 6` with `RAYON_NUM_THREADS=4` and unpinned, as a round's small walk
  runs; three seconds later the measured walk starts, and the neighbour stops after it.
- **The measured walk is pinned as ADR 1577 pins `turn_path`'s child**: `taskset -c` on the
  fastest class derived from `cpuinfo_max_freq` (CPUs 0–3 and 12–15 here), `RAYON_NUM_THREADS` the
  count of that list (8), and those cores kept busy for 30 ms just before it. The figure is the
  measured process's own wall clock, `bash`'s `time`, with its user and system time beside it.
- **Interleaved** A B, B A, A B within each walk, `raster_golden` first and the column second,
  12:21 to 12:37, one-minute load 2.7 to 5.9 at every start.

## 2. The figures, seconds of wall

| walk | alone | beside | min / min | median / median | pairs in sitting order |
|---|---|---|---|---|---|
| `raster_golden` (small) | 18.79, 19.90, 18.79 | 21.01, 21.01, 19.91 | 1.060 | 1.118 | 1.118, 1.056, 1.060 |
| the Tier 1 column (large) | 122.61, 102.76, 104.96 | 120.46, 123.77, 116.03 | 1.129 | 1.148 | 0.982, 1.204, 1.105 |

- **The worst beside against the best alone is 1.118 and 1.204.** The column's own alone arm spread
  1.193 between its first run and its second, so its stretch is the size of its drift, as ADR 1706
  section 3's control found for the merge's gates (0.83 to 1.20).
- **The processor time moved with it**: the column's user plus system time was a median 200.8 s
  alone and 222.1 s beside (1.106), `raster_golden`'s 59.7 and 61.0 s (1.022). The measured walk
  queued for no core it was pinned to; it did more processor-seconds of the same work, which is
  habit 36's sharing — an SMT sibling, the cache, a package clock — and not a wait to subtract.
- **The neighbour stretched too.** Alone, after the sitting and in a `--clock` hold of its own:
  6.64, 7.74, 7.75 s. Beside the column, 37 iterations: median 9.10 s (1.175 of the alone median),
  mean 9.39 (1.213); beside `raster_golden`, 6: median 8.83 (1.141). **The worst single iteration
  was 12.94 s, 1.672×**, the first full one beside each column run (12.94, 10.91 and 12.28 s), so a
  short small walk that starts while the column is opening its documents can take two thirds as long
  again. Over any whole hold of the column the neighbour's mean is 1.21×.

## 3. Decision

- **No walk timed here stretched a whole hold by 1.5×.** The worst is 1.204 for a large walk and
  1.213 for a small one over a hold, against ADR 1706 section 3's break-even of 1.5 to 3 for taking
  the lane out. **The second lane stays**, and the section 5 re-ask is answered: a neighbour stretches
  each walk by 1.06 to 1.21× when the two are measured in one sitting, the size of the drift a walk
  shows alone. Batch sixty-nine's own lines replay, through `/home/AI/lock-replay-1435/replay.py`,
  at 27 457.4 s on one lane against 4 655.6 s under the built rule (22 801.8 s apart, against ADR
  1684's 3 600 s), so neither reopening condition holds.
- **The single-iteration figure is not a reason to reopen.** A hold is what the replay's factor
  multiplies, and 1.67× held for one 13 s iteration of a walk whose mean over the hold was 1.21×.
  It is a reason for a round timing a short walk to take `--clock`, which the rule line already says.
- **The re-ask is reopened only by the conditions ADR 1706 section 5 states**, and a later round
  that repeats this sitting runs `scratchpad/r1449/sitting.sh` from the kept directory.

## 4. Found on the way: `raster_golden`'s digests are a function of the process's core count

Every pinned run of `raster_golden` exited 101 with **21 first pages "raster only"**, the list and
the reports unmoved. The same binary passed unpinned with `RAYON_NUM_THREADS` 4 and with 8, and
failed pinned with 4: the rayon pool is not it. `render_cpu::plan_strips` asks
`std::thread::available_parallelism` for the strip count where the caller names none, and ADR 0219
measured that a page's pixels depend on its division into strips. `raster_golden.rs` draws with
`CpuRasterizer::new()`, so its held digests are those of a process that may run on 24 CPUs: on a
machine, a cgroup or an affinity mask with fewer, pages fail with nothing changed — 21 of 967
first pages under an eight-CPU mask.
The figures in section 2 are unaffected — both arms drew in the same eight strips — but the gate is
held on a quantity the tree does not decide. The construction that removes it is the one
`shadings.rs` and `pdf-transform`'s renderer already use, a strip count stated with `with_strips`
and the file regenerated under it; that is `pdf-model`'s and the pixels round's to build, and this
round proposes it in its report rather than editing a sibling's file.
