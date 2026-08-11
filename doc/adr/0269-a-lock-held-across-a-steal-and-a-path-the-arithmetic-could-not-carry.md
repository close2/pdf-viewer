# ADR 0269 — A lock held across a steal, and a path the arithmetic could not carry

Date: 2026-08-11 (session 433)
Status: accepted

## Context

The project owner moved to a fast connection and asked for it to be used. **93 GB is on disk:
65 944 documents in 145 archives of `CC-MAIN-2021-31`**, up from 5944 in 85 — eleven times the
largest population this project has ever had, and a sample of the actual web, because ADR 0261
verified that an archive is a hash bucket (the whole crawl sorted by SHA-256, agreeing with the
digest to 2.6 × 10⁻⁴).

Every survey so far has paid: session 425 found §11.4.7's page-group blending space at 3.5% of the
web against 0.7% of the pdf.js corpus, and the first crasher; session 430 found a three-component
APP14 JPEG asked for four channels and a `/Length 0` stream reported as lost drawing. This round
surveyed **all of it**, twice.

**First, a correction to the number.** `manifest.tsv` holds 65 968 rows and the cache holds
**65 944 documents**: archive `0050` was fetched twice, once by session 425's stride and once whole,
so its 24 members are recorded twice. Every count below is over the 65 944 distinct files.

## Decision 1 — survey the whole population, one archive per process

**One `safedocs survey --dir <archive>` per archive, 145 of them, sequential.** Not because
65 944 documents in one process would be too many — they would not — but because the survey
rasterises with `render-cpu` under `[profile.release]`, where `panic = "abort"`, and rayon over one
`par_iter` means **one document's abort loses every other verdict in the process**. An archive is
455 documents on average, so a shard that dies names where to look and costs at most that many.

It paid on the first run: **five of the 145 archives did not report at all** — two aborted and three
were killed at the driver's 600 s timeout. Both defects below are those five archives, and neither
would have had a name if the population had been surveyed as one process.

**What the first pass cost: 2894 s of wall clock**, of which 1800 s is the three hangs sitting at
the timeout and **1072.7 s is the survey itself** over the 140 archives that reported — 63 872
documents. **The second pass, after both fixes, is the baseline**: 145 archives, 0 failures.

## Decision 2 — a `OnceLock` initialiser may not call rayon

**Three archives hung: `3050`, `3250`, `7350`.** Not slowly — *stopped*: 25 threads all parked in
`futex_do_wait`, 2.26 s of processor time spent in five minutes, and a `pdf-sandbox-worker` child
idle on its own pipe. A hang is the shape of finding this population is for, and the cause is one
function.

`colour::ink_table` — §11.7.2's conversion into a group's blending space, built in the
four-hundred-and-twenty-seventh session (ADR 0263) — was

```rust
TABLE.get_or_init(|| (0..4913).into_par_iter().map(search_ink).collect())
```

**`OnceLock::get_or_init` blocks every other caller until the closure returns, and rayon's
`collect` runs other jobs while it waits.** So the initialising thread can be handed a job that
calls `ink_table` again; that call finds the `OnceLock` in progress and waits — on the same thread,
for itself. Nothing can make progress and nothing times out.

**Reduced to twenty lines it is not this tree's bug at all**, which is what makes it worth writing
down: a `OnceLock` whose initialiser is a rayon `collect`, called from a rayon parallel iterator,
**hung 10 runs out of 10**. The same reduction with the grid computed *before* the lock is taken
finished **10 out of 10**.

**The fix is that, plus one branch.** `build_ink_table` is called first and `get_or_init` is handed
a value rather than a closure, so the lock is held across a move and nothing else. And the build is
**parallel off a rayon worker and serial on one**: the parallelism exists for the launch path —
61.7 ms serially against 7.5–10.0 ms across 24 threads, on the way to page one — and a host's own
thread is never a worker of the pool. Inside a worker, `collect` would still run other jobs while it
waits, and the function would nest once per job that wants the grid; the depth of that is a property
of the caller's queue rather than of anything here, so the branch declines it. A second caller
arriving mid-build computes its own grid and throws it away, which costs one grid and waits on
nothing.

**Verified by running it**: the three archives that were killed at 600 s now take **0.72, 0.89 and
0.25 s**, three times each, and the whole 145-archive pass has no failure of any kind.

**What it says about the gates** is worth more than the fix. The corpus gate rasterises its 974
documents through the same `par_iter`, and 0.6% of them composite in ink, so it has been able to
hang for six sessions and has not — the window is the 8 ms the grid takes to build. A defect that
needs two documents to want one lazily-built table in the same eight milliseconds is not a defect a
974-document gate finds; it is one a 65 944-document population finds three times in an hour.

## Decision 3 — a path outside the scan converter's arithmetic loses its anti-aliasing

**Two archives aborted: `0300` and `4605`**, both with

```
tiny-skia-0.12.0/src/alpha_runs.rs:170: called `Option::unwrap()` on a `None` value
```

under `render_cpu::CpuRasterizer::draw` → `PixmapMut::fill_path` → `scan::path_aa::fill_path` →
`SuperBlitter::blit_h` → `AlphaRuns::break_run`. One document apiece:

| document | archive | SHA-256 |
|---|---|---|
| `0300856.pdf` | `0300` | `09bafa06b40c6cfab5bfca03ac5e819efab3adec6d8ebdc52408b9cca23a7b1b` |
| `4605705.pdf` | `4605` | `94a3b4969b1f16611801bc27c03ceb3ace5b73bfa1a18f9e0f061a4e5681f5c0` |

**What they state.** Both are 368 × 542 pages whose content streams are damaged, and both fill
paths whose device-space bounds run past 10²⁵ — read off the display list rather than inferred.
The fill that aborts was found by printing every path handed to `fill_path`: on `0300856.pdf` it is
the third one, bounds (−11 310 159, −236 519) to (89 152 340, 2 034 299 800) under a transform that
only translates and flips y.

**Where the standard is on this.** §10.7 leaves scan conversion to the device and bounds no
coordinate, and §7.3.3 hands the range of a number to the implementation outright:

> The range and precision of numbers may be limited by the internal representations used in the
> computer on which the PDF processor is running; Annex C, "Advice on maximising portability",
> gives these limits for typical implementations.

Annex C is *informative* and gives no figure for a coordinate — its row for real numbers says only
that computers "often" use IEEE 754. So the magnitude a page may state is this processor's to
decide about, and deciding is what was missing.

**Where the dependency is on it.** `tiny-skia`'s anti-aliased scan converter supersamples by four
into 16.16 fixed point, and it says the consequence itself above `DrawTiler`: its fixed-point types
are limited by 8192 and 32768, so it cannot render a path larger than 8192 onto a pixmap. What it
bounds by that number is the **pixmap**, which it tiles; the **path** runs through the same
arithmetic unbounded. `SuperBlitter::blit_h` carries a comment calling its own left-hand guard a
hack "until I figure out why my cubics (I think) go beyond the bounds", and there is no right-hand
one at all.

**The remedy is the library's own, applied where its test does not reach.** `path_aa::fill_path`
already falls back to the aliased converter when the clipped bounds would overflow the shift. So
`crates/render-cpu/src/scan.rs` is now the single place a path reaches either converter — eleven
call sites across `lib.rs` and `shading.rs` go through its four functions — and a path whose device
bounds leave ±8191 is drawn **without anti-aliasing** rather than with it.

**Why not a refusal.** `CpuRasterError` stops the whole raster, so refusing one damaged path would
take every mark after it off the page, on the backend that is the correctness oracle. And the
aliased converter is demonstrably able: over the reduction, magnitudes from 10³ to 10³⁰ all return,
and where the anti-aliased one also returns the two agree to within the anti-aliasing of the edges —
**13 250 820 against 13 252 311 of ink, 0.011%**.

**Why 8191 is not a fitted constant.** It is the dependency's own number for its own stated reason,
and the failure is *not* monotone in magnitude — the reduction survives at 10⁵, aborts at 3 × 10⁶
and 10⁷, survives from 3 × 10⁷ to 10⁹ and aborts at 3 × 10⁹ — so no threshold read off the failures
would be anything but curve-fitting. The bound comes from the arithmetic instead.

**The regression test is generated, not committed.** `crates/pdf-model/tests/hostile_geometry.rs`
writes a 368 × 542 page whose content stream is `10 10 m 10000000 2000000000 l 30 10 l f`, and three
more for the same spike stroked, used as a clip, and a control that is an ordinary square. With
`SUPERSAMPLED_LIMIT` lifted the three hostile tests fail and the control passes; with it in place all
four pass. `doc/todo/03`'s promotion budget is untouched at **0 MB**.

## Decision 4 — the baseline, and the three-way rate comparison

**The whole population, after both fixes, and a baseline for this population rather than a
ratchet**: **65 944 documents in 1139.3 s** — 1148 s of wall clock over the 145 archives, no
failure of any kind — **173 unopenable, 45 locked, 23 encrypted beyond us, 52 pageless, 1144
incomplete, 2 slow**, with 51 272 codes reaching no glyph in silence over 635 documents.

**The rate is the result, and it is stable across two samples of very different sizes:**

| sample | documents | incomplete | rate |
|---|---|---|---|
| session 425, 79 archives × 24 | 1896 | 86 | 4.54% |
| session 430, 4 whole archives | 4000 | 70 | **1.75%** |
| **session 433, all 145 archives** | **65 944** | **1144** | **1.735%** |

The first number moved because sessions 426 and 427 built §11.4.7's conversion; the second and
third differ by 0.015 points over a sixteen-fold increase in sample size, which makes **1.7% a fact
about the web** rather than about anybody's sample. The 974 pdf.js documents sit at 68 of 974,
**7.0%** — four times the web's rate, which is what a corpus assembled from bug reports is *for*
and is the sharpest statement of "two questions, two denominators" this project has had.

**The reports, ranked by document count** — a document reporting two things is in two rows:

| population | documents | of 65 944 | named by |
|---|---|---|---|
| a group's blending colour space (§11.4.7, §11.6.6) | **398** | 0.60% | `doc/todo/23` |
| a font with no outline for any code the page shows | **261** | 0.40% | `doc/todo/21` §3 |
| an image | **152** | 0.23% | five things, ADR 0266 |
| §11.4.4's non-isolated group with an element that blends | **129** | 0.20% | `doc/todo/23` |
| a budget stopped interpretation | **84** | 0.13% | `doc/todo/49`, and see below |
| text-showing operators skipped | 60 | 0.09% | with the font rows |
| a font program that would not parse | 35 | 0.05% | — |
| a `/Contents` part that would not decode | 32 | 0.05% | — |
| a `/Font` or other resource the file never defines (§7.8.3) | 25 | 0.04% | ADR 0255 |
| §11.6.4.3's knockout group | 23 | 0.03% | `doc/todo/23` |
| an operator, a shading, an annotation | 22, 20, 11 | — | — |
| §11.6.2's object painted in parts, §9.3.8's text knockout | 9, 4 | — | — |

**The first row splits into five named conditions**, which is what `doc/todo/23` prices one row
apiece — counted as *distinct documents* off the reports rather than assumed, which is why they sum
to 399 against the row's 398:

| condition | documents |
|---|---|
| the document names the press its `DeviceCMYK` is (§8.6.5.6, §14.11.5) | **151** |
| the page group's four components are not `/DeviceCMYK` (an array-formed space, or `/DeviceGray`) | **106** |
| a group inside the page composites in a different space (§11.6.6) | **78** |
| a group inside the page *introduces* the space (§11.6.6, not the page group) | **30** |
| a non-separable blend mode gives black a rule of its own (§11.3.5.3) | **27** |
| an `/ExtGState` states Table 57's `/BG` or `/UCR` (§11.7.5.3) | **7** |

Two of those numbers are worth saying out loud. The four-component page group `doc/todo/23` priced
at "0 corpus documents and 1 web witness" until ADR 0266 made it 14 of 4000 is **106 of 65 944**,
and the press-naming row above it is **151** — so the largest transparency gap this tree has is a
conversion into somebody else's four components, twice over. And §11.7.5.3's row, which read "1 of
1896, 0 of 4000" and could have been mistaken for noise, is **7**.

**The budgets are 84 of 65 944, and the rate is stable at three sizes**: `MAX_TILES` 48,
`MAX_OPERATIONS` 31, `MAX_FORM_DEPTH` 4, `MAX_STATE_DEPTH` 1 — 0.127% against session 430's 0.2% of
4000 and session 425's 0.105% of 1896. Neither of the two slow documents is one of them, which is
the same finding session 430 made with eight documents and this one makes with 84: **the bound
stops the work inside the per-document budget rather than after it.** `doc/todo/49` keeps them where
they are.

**Nothing failed to open for a reason that is this tree's, for the third sample running.** The 225
unusable documents are 163 with no `%PDF-` header in their first kilobyte (HTML saved under a `.pdf`
name), 52 with no first page and 5 whose cross-reference table is unusable — and all five of those
were opened by hand: three have had their `<<` and `>>` replaced by `&gt;` somewhere in transit and
two are truncated to 119 and 131 bytes. Four of the pageless ones were opened too: two state a
linearised `/L` in the hundreds of thousands or millions and are 968 and 1431 bytes long. The 23
encryption refusals are 20 `/R` 5, 2 `/Encrypt`s that do not resolve to a dictionary (§7.6.1) and
one `/Adobe.PubSec` (§7.6.4).

**And this population produced its first *slow* documents**, both measured again on their own
rather than under the survey's 24 threads and **both complete** — they report nothing, they simply
take that long to draw page one:

| document | archive | bytes | alone | SHA-256 |
|---|---|---|---|---|
| `0423548.pdf` | `0423` | 9 933 485 | **32.9 s** | `0db5152253cc8483dad26ae0c27cba5e54c88e6a941603ca17b27b8a4d487c85` |
| `6081357.pdf` | `6081` | 4 390 859 | **68.0 s** | `c43ac28fd21d5d13201849d641346b9269582670c5b3ecdc0879228ec1964ab8` |

2 of 65 944 is 0.003%, and it is the first time this population has produced a "slow" at all:
sessions 425 and 430 both printed **0 slow**, over 1896 and 4000 documents. Not
diagnosed here; `doc/todo/03` carries it, and it is the clearest thing this population has left for
the next round.

## The fuzzer over the new seeds, and what 65 944 documents cost it

`fuzz/seed_page.py` over every document on disk — the 65 944 `SafeDocs` members, `doc/corpora`'s 108
and the pdf.js submodule's 974 — writes **32 040** of the 67 026 into `fuzz/corpus/page` and skips
**34 986** as larger than the target's 256 KiB ceiling. The corpus goes **10 048 → 38 331 seeds**,
against session 430's 8572 and session 428's 1882, and what they state is printed rather than
assumed: 19 997 with an embedded font, 16 225 with an image XObject, 14 447 with a `/Group`, 11 985
with `/Annots`, 6513 with an `/SMask`, 3585 form XObjects, 1280 `/Function`, 588 `/OCProperties`,
587 `/Shading`, 441 `/Pattern`.

**Coverage at seeding is 33 625 edges** — 28 535 when session 428 built the target, 32 671 after
session 430's 51 324 iterations — over a corpus libFuzzer reduced to **8703** inputs of distinct
coverage. **33 217 units later it is 34 119**, with the features 181 907 → 186 146 and the corpus
8703 → 9384, and **0 crashes, 0 out-of-memory and 0 timeouts**.

**And the cost is the finding.** libFuzzer's fork mode merges the corpus before it fuzzes, one
execution per seed, and 38 331 seeds took **48 minutes** of that against a few minutes for session
430's 8572; the run that follows it executes at 3–11 per second rather than 41, because each job
restart re-reads 8703 inputs. `doc/todo/02` §2 carries that so the next round budgets for it or
`cmin`s first — this one was stopped at 33 217 of its 50 000 rather than paying for the rest.

**Nineteen new `slow-unit-` artefacts and ADR 0264's rule says read them in a release binary before
believing them.** The five largest run in **1.27, 1.73, 1.78, 1.82 and 2.06 s** through
`target/pdf-retrieve`, so they are the sanitiser's slowness and not the product's, exactly as
sessions 428's five and 430's two were — slower than those, because the seeds are now drawn from a
population whose documents are larger. Nothing was promoted and no budget was touched; the one
`timeout-` artefact in that directory predates this round.

## Consequences

- **Neither fix moves the 974, and that is checked rather than hoped.** The whole of
  `doc/todo/02` §2 ran and every line reproduced except three counts — tests 1572 → **1580**,
  citations 6373 → **6378**, quotations 596 → **597**. The corpus's 974 with **68 incomplete**, the
  oracle's 1794 pages (1690 complete, 104 incomplete) at **905 / 68 / 786** with 1/0, 2/2, 14/9,
  18/0 and the undiagnosed list empty, quorra's **911 / 35 / 11 / 17**, the text gates' 99.2%
  (24003/24187) and 99.8% (14257/14281), the dates' 1514 of 1545, the XMP's 318 of 319 and the
  JPEG 2000 line are what `doc/HANDOVER.md` says. **`doc/todo/00`'s step 7 is not owed**: no
  document of the 974 states a path outside ±8191 device units, which is what those identical
  oracle *and* quorra counts mean — quorra compares this backend's raster page by page, so a
  changed CPU raster would move its differing list.
- **`doc/todo/23`'s five rows have web numbers from a population of 65 944** rather than from 4000,
  and `doc/todo/49`'s budget row and `doc/todo/21` §3's silence have one for the first time.
- §10.7 and §10.7.1 gain the reading; §11.7.2 gains the note about how its table is built. No
  ledger status moved: **875 rows, 403 implemented / 250 partial / 18 reported / 83 inapplicable /
  8 writer-side / 113 out-of-scope**.
- **Two instruments are owed and neither was built here**: a survey that keeps going past a crash
  (a worker process per shard, or `panic = "unwind"` for the survey binary), and the general path
  clipper that would let a fill outside the range keep its anti-aliasing on the part that shows.
- **And the fetching rule in `doc/todo/03` is spent**, because there is nowhere in this corpus
  nobody has been. What a round takes next is a *different* corpus or a *population* out of this
  one — the 106 four-component page groups, the 151 that name a press, the two slow documents.
