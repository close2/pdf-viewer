# 1742 — A pixel gate states its strip count: one where it holds a digest, sixteen where it holds a clock

Session 1453. Status: **accepted** and **built**. Builds what ADR 1734 section 4 found and proposed:
`raster_golden`'s digests were a function of the CPUs its process was given. Supersedes nothing.
Context: ADRs 0218, 0219, 1016, 1443, 1734; traps 1, 13, 66, 134. Code:
`crates/pdf-model/tests/raster_golden.rs` (`STRIPS`) and `raster_golden.tsv`;
`crates/pdf-model/tests/oracle.rs` (`STRIPS`); `crates/render-raster/tests/corpus.rs`
(`ORACLE_STRIPS`); `crates/render-cpu/src/lib.rs` (`MAX_STRIPS`, now public);
`crates/pdf-model/examples/raster_digest.rs` and `own_ink.rs` (`STRIPS`). Instrument:
`scratchpad/r1453/` (`strips_pair.sh`, `compare.py`, `golden-three.sh`, `arms.sh`, `chain.sh`).

## 1. The dependence

`CpuRasterizer::new()` leaves the strip count to `plan_strips`, which asks
`available_parallelism` — the CPUs the process's affinity mask and cgroup allow, capped at
`MAX_STRIPS` (16) and at a page's rows over 64. ADR 0219 measured that a page drawn in strips is the
page drawn whole only up to `tiny-skia`'s arithmetic at a shifted origin. So every instrument that
compares pixels exactly and leaves the count to the machine holds a fact about that machine: on this
24-CPU laptop it drew in sixteen strips and under ADR 1734's eight-CPU mask in eight.

## 2. How much the division moves, over the population the golden holds

Regenerating `raster_golden.tsv` under one strip moved **150 of 967** first pages, every one
"raster only": no display list and no report moved. Drawn again with `examples/render_at` at one
CPU (`taskset -c 0`, one strip) and unpinned (sixteen) and compared byte for byte, the 149 that
open without a password (`issue21579.pdf` asks for one) differ on every page and nowhere much:
**7 042 pixels** in all, **worst 2 levels of 255** (5 pages; 144 at 1), the largest share
**0.36%** of a page (`issue1350.pdf`, 1 078 of 301 104 pixels on five rows). Looked at side by
side with the moved pixels painted red (trap 1), that page is the page and every red pixel is on a
horizontal edge of its form's three boxes: an edge on a sample row, ADR 0219's mechanism.

**Three pages exceed the bound `strip_parallelism.rs` asserts** — one pixel in a thousand
(`issue1350.pdf`, `issue7020.pdf`, `pdfjs_wikipedia.pdf`) — and five exceed its one level
(`blendmode.pdf`, `comments.pdf`, `highlights.pdf`, `issue12810.pdf`, `issue7014.pdf`). That gate
holds three pages; the corpus holds more than its constants admit. It is trap 66's shape, and it is
not decided here (section 5).

## 3. Decision: which count, gate by gate

- **`raster_golden`: one.** The gate compares digests, so the division is what it holds, and ADR
  0219 has already decided which division is the page: the one that exists when there is no
  division. The pages are drawn in parallel across the corpus, so a page's own strips bought the
  gate nothing visible: 18.61 s unpinned at four rayon threads, beside ADR 1734's 18.79 to 19.90 s
  at eight strips under the eight-CPU mask (not an A/B: another sitting, another load). Regenerated
  once, the 150 moved and nothing else; then
  **held 974, moved 0** unpinned, pinned to the eight fast CPUs at eight and at four rayon threads,
  and pinned to two CPUs (22.50, 23.50 and 33.61 s). Calibrated in both directions: the same
  binary at one strip fails HEAD's file on exactly the 150, and HEAD's binary at the machine's count
  failed 21 under the eight-CPU mask (ADR 1734).
- **`examples/raster_digest` and `examples/own_ink`: one**, for the golden's reason: each is a
  change detector whose two arms are compared exactly, and two arms taken under different masks
  would otherwise differ where nothing changed.
- **The oracle's own render (`tests/oracle.rs`): one.** It is the render the references are
  compared with, and ADR 0219 names the undivided page as the page; the outer `par_iter` already
  spreads the pages over every core. Run once at one strip from an export of HEAD with this
  patch, against the shared reference cache (6 796 hits, 0 produced): 1 968 pages, every held list
  held, exit 0, 56.4 s, 33 s of processor time ours.
- **`render-raster`'s corpus gate, the six arms' oracle: `render_cpu::MAX_STRIPS`, sixteen.** Its
  pages are drawn one after another and the CPU backend's clock is half of the survey — the totals,
  the median ratio quoted in `doc/performance.md` §3b and in the rendering library's own notes, and
  the second column of every `PDFVIEWER_RASTER_TIMES` file. Sixteen is what the CPU backend asks
  for on any machine of at least sixteen CPUs, the one every figure was taken on, so on this
  machine the stated count is the old one bit for bit and on a smaller one the pixels no longer
  follow the mask. One strip would turn that clock into a serial one that no window runs. The six
  arms of the same export against `/home/AI/arms-1450/`: 5 795 page lines, **0 digests and 0 means
  moved**, the same differing, refused and not-comparable pages in every arm; only the clocks
  differ. So no page is named here, because none moved.

## 4. Read and left as they are

- **`turn_path` holds no digest** — the brief's premise did not hold. Its witness is each row's
  command count, a property of the display list, and its frames are drawn by raster's lanes on the
  device with the encode threads its own child is pinned to.
- **The arms' frame digest is raster's frame, not the oracle's.** The corpus gate draws every page
  again at one encode thread and fails on any byte the count moves (`OneThread`, trap 66), so the
  fourth column was never the machine's; the fifth, the mean against the oracle, was, and is
  stated by section 3.
- **The censuses that measure one property in one run** (`border_overhang_census`,
  `crop_box_census`, `uncovered_share`, `tiling_type_census`) and the fixture tests that call
  `CpuRasterizer::new()` keep the machine's count: none compares two runs, and a fixture of a few
  hundred rows is cut into the same few strips by any machine of more than a handful of CPUs.

## 5. Cost, and what is left

- Nothing a window draws changed: `MAX_STRIPS` became public, and every other edit is in a gate.
- `strip_parallelism.rs`'s two bounds are narrower than the corpus (section 2). Whether its
  constants are wrong or eight pages carry a second mechanism is a pixels round's question, with
  `scratchpad/r1453/compare.txt` as its list.
