# ADR 0271 — A page that derives one constant four billion times, and four bounds nobody had opened

Date: 2026-08-11 (session 435)
Status: accepted. **Corrected in one unit by ADR 0306**, which does not disturb a measurement here.

> **Everywhere below that this ADR says a document "wants *n* operators", it means *n* lexer
> tokens.** `MAX_OPERATIONS` counted the loop's turns rather than the operators, and §7.8.2 puts
> an operator after its operands, so `x1 y1 x2 y2 x3 y3 c` is seven of the first and one of the
> second. The populations, the wall clocks and the peaks below were all measured through that
> counter and are exactly as recorded; what was wrong is the word. Read "4.1 to 53.6 million
> operators" as "4.1 to 53.6 million tokens", and note that the conclusion this ADR draws from
> them — that the population is maps and plans rather than bombs, and that a count is not a cost —
> is unaffected, because a document that terminates with the bound lifted terminates whichever
> quantity the bound was counting. Session 471 re-measured in operators: of 926 680 pages over
> 65 967 crawled documents, 48 pass four million tokens and **8** pass four million operators.


## Context

Session 433 surveyed the whole of `CC-MAIN-2021-31` — 65 944 documents in 145 archives, 93 GB
on disk — and left two populations named and undiagnosed (ADR 0269):

- **Two documents are slow**, 32.9 s and 68.0 s, and **both complete**. Nothing was refused, so
  the time went somewhere and nobody had looked.
- **84 refusals are a resource budget**, and the rate is stable at three sample sizes (0.105% of
  1896, 0.2% of 4000, 0.127% of 65 944), so it is a fact about the web. `doc/todo/49` had kept the
  bounds where they were with the honest note that what was still owed was "one of the 84 read
  with the bound lifted in a scratch build, to find out whether the constant costs a mark or
  stops a bomb."

Two of 65 944 is a tail, and a tail is where a denial of service lives.

**First, a correction to the second number.** It is **84 refusals over 83 documents**, not 84
documents: `7680183.pdf` reports `MAX_OPERATIONS` and `MAX_TILES` both. The rate is unchanged.

## What the round inherited, and the hunk that had to go

This round was started by another agent and interrupted. What was in the working tree was
scratch instrumentation — atomic counters in `render-cpu`, a debug dump in `open_one`, and the
four bounds in `content.rs` rewritten as functions reading an environment variable **on every
call**, which puts a `getenv` inside the interpreter's per-operator loop. Every one of those
files is labelled *Not for commit* in its own comment, and none of it is in this commit.

**And the diff edited `CLAUDE.md`.** Principle 2 said

> a parallel path that improves throughput while worsening time-to-first-page is a regression

and the diff made it "while **noticeably** worsening". That hunk is reverted. `CLAUDE.md` is the
project owner's statement, its own opening requires a conflict to be said out loud "rather than
quietly compromising", and it requires an exclusion to be revisited "by argument, never by
attrition" — so a round that softens the rule it is about to fall foul of has done the thing the
file forbids. **The rule turned out not to bind anyway**, which is the part worth recording: the
change below adds no parallelism and no startup cost, because it removes work rather than
spreading it.

## Decision 1 — the two slow documents are this tree's fault, and it is one line

**Neither is slow to parse.** `open_cost`'s first two steps are milliseconds:

| | `Document::open` | page one | `interpret` | rasterise | total |
|---|---|---|---|---|---|
| `0423548.pdf` | 6.8 ms | 7.8 ms | 2.10 s | **23.3 s** | 25.4 s |
| `6081357.pdf` | 3.8 ms | 4.5 ms | 0.27 s | **52.3 s** | 52.6 s |

So the time is in `render-cpu`, and inside it, in one place. Counters through `CpuRasterizer`:

| | distinct soft masks | soft-mask builds | **the value pass** | pixels it ran over | of which `[0,0,0,0]` |
|---|---|---|---|---|---|
| `0423548.pdf` | 89 | 456 | **22.4 s** | 1 686 997 422 | 1 662 342 098 — **98.54%** |
| `6081357.pdf` | **912** | 895 | **51.0 s** | 3 866 879 720 | 3 865 478 534 — **99.96%** |

`6081357.pdf` is a 2552 × 1693 page — 4.3 million pixels — and it evaluates **912 distinct soft
masks**, so the raster work is 912 times the page's own size. That is not eviction thrash: the
display list holds 912 of them and each is built about once. `0423548.pdf`'s 456 builds over 89
masks is the per-strip rebuild a parallel render pays, which is a different thing and is correct.

**Why the whole page, for every mask.** `build_soft_mask` draws a mask group into a buffer the
size of the *target* and then converts all of it: `Pixmap::take_demultiplied` allocates a second
target-sized buffer and divides three channels by the alpha in every pixel, and
`SoftMask::values` then derives §11.5.3's luminosity in floating point in every pixel. A mask
group covers its own `/BBox`; the rest of the buffer is the transparency it was allocated as.

**And for the transparent pixel both answers are constants.** The division gives back the zero it
started from, and the derivation gives one number for the whole raster — the transfer function of
0.0 for `/Alpha`, the backdrop's luminosity for `/Luminosity`, which is exactly §11.6.5.1's rule
for a pixel outside the group's box. `SoftMask::values`' own doc comment has said so since the
hundred-and-seventy-fifth session; nothing had read it as a *quantity*.

**So it is named and computed once.** `SoftMask::outside()` is `self.value([0, 0, 0, 0])`, and
both `SoftMask::values` and `render-cpu`'s `build_soft_mask` take it for a pixel that is wholly
transparent. `render-cpu` additionally stops calling `take_demultiplied`, demultiplying per pixel
instead and only where the group actually marked — which removes a target-sized allocation per
mask as well as the arithmetic.

**It is exact, not an approximation, and that is the whole warrant.** The branch's two arms call
the same function on the same pixel. Two tests state the two halves that could make it false:
`the_outside_value_is_the_transparent_pixels_own` walks every mask kind and both transfer cases,
and `the_transparent_pixels_shortcut_is_the_derivation` asserts the thing this tree does not own
— that `tiny-skia`'s `demultiply` sends `[0,0,0,0]` to `[0,0,0,0]`, which it does because the
division by zero is a `NaN` that Rust's saturating cast lands on 0. A third,
`the_per_pixel_route_agrees_with_the_whole_buffers`, compares the new route against
`take_demultiplied` over a 16 × 16 buffer of every alpha.

**What it is worth**, three samples each, `[profile.release]`, machine otherwise idle:

| document | before | after | |
|---|---|---|---|
| `0423548.pdf` | 25.70 / 25.78 / 25.80 s | **6.61 / 6.70 / 7.26 s** | 3.7× |
| `6081357.pdf` | 53.61 / 53.75 / 53.78 s | **3.72 / 3.92 / 4.02 s** | **13.6×** |

The spreads are 0.10 s and 0.17 s before, 0.65 s and 0.30 s after; both differences are two
orders of magnitude larger.

**Both documents are now far inside the survey's 30 s budget — and the survey's `slow` count is
not the way to say so.** Three passes over all 145 archives: before, **2 slow**; after, **0**; and
a third pass on an idle machine, **1** — a *third* document, `1284722.pdf`, that neither earlier
pass had named. Alone it takes **11.13 / 11.14 / 11.23 s** (spread 0.10 s), nowhere near the
budget; it crossed 30 s in that one pass because the survey rasterises 24 documents at once. The
claim this round makes is therefore about the two documents measured alone, not about the count.
What the third pass is worth is the correction it forces: **a per-document wall-clock budget taken
under a 24-way parallel load measures the machine as much as the file**, and a round that had run
the survey once would have reported "0 slow" and been wrong about why. `1284722.pdf` is not a
soft-mask document either — 11.1 of its 11.2 s is `interpret`, for 94 596 commands — so nothing
here touches it, and it is the next candidate this population offers.

**And no page moved**, which is what makes the claim of exactness a measurement rather than an
argument: the oracle's 1794 pages are 905 / 68 / 786 with 1/0, 2/2, 14/9, 18/0 and an empty
undiagnosed list, quorra's 957 are 911 / 35 / 11 / 17, and the corpus's 974 have 68 incomplete —
every figure identical to session 433's.

**What this does not do** is stop the buffer being target-sized. A mask group's own device band
would take the *encode* and the allocation down as well, and it is a change to what `Built` means
because a soft mask's outside value need not be zero — `doc/todo/40` and `doc/todo/49` carry it.
`0423548.pdf`'s remaining 6.6 s is mostly elsewhere anyway: 2.1 s of interpretation and 2.85 s of
`initial_backdrop`, which copies the whole page into a new buffer for each of its 132
non-isolated groups.

## Decision 2 — the four bounds stay, and now three of them have a clause and all four a census

Each of the 83 was opened with its bound lifted in a scratch build, one process apiece, killed at
120 s. **None of the four bounds moved**, and the reason differs for each — which is the point of
having looked.

**Where the standard is.** §C.1: "In general, this PDF standard does not restrict the size or
quantity of things described in the PDF file format". §C.2's Table C.1 has a *Nested objects* row
that anticipates a bound like these —

> However PDF processors may implement recursive algorithms which may cause issues for
> excessively nested constructs.

— and its NOTE prints the one figure ISO 32000-2 gives for any of them:

> In previous versions of PDF, a maximum depth of graphics state nesting by q and Q operators
> was 28.

Annex C is informative, so neither sentence binds. What they settle is whether these numbers are
mean, and they are not.

| bound | of 65 944 | what the documents are, with the bound lifted | what it prevents | and the 4 GiB ceiling? |
|---|---|---|---|---|
| **`MAX_FORM_DEPTH` 16** | 4 (0.006%) | **all four are cycles** — lifted sixteenfold to 256, all four reach 256 | unbounded recursion | **no.** A cycle exhausts the *stack*, and Rust's guard page makes that an abort rather than a report. Nothing else catches this |
| **`MAX_TILES` 4096** | 48 (0.073%) | all terminate, 0.06–14.2 s, wanting 4104–895 500 tiles; **14 of 48 want under twice the bound** | a loop whose trip count the file states and whose body may cost nothing | **no.** 1 000 000 *empty* tiles interpret in 889 ms reporting nothing — `MAX_OPERATIONS` never moves, because an empty cell executes no operator |
| **`MAX_OPERATIONS` 4 M** | 31 (0.047%) | all terminate, wanting 4.1–53.6 M **tokens** (see the correction above), 0.27–49.9 s; the worst peaks at 1.57 GB for 495 marks | slowness | **yes**, and a cancel already exists at 0.83–1.97 ms (ADR 0241) |
| **`MAX_STATE_DEPTH` 256** | 1 (0.0015%) | wants **337**, draws in 0.05 s | memory per saved state | **yes** |

**`MAX_FORM_DEPTH` is the clean answer: this is the attack the bound exists for.** Four documents
of 65 944, and every one of them is a form that reaches itself. The bound costs the web nothing
and is the only thing standing between a cycle and an aborted process.

**`MAX_TILES` is load-bearing for a reason that had not been written down.** A cell's content runs
through the same interpreter, so a pattern's operators count against `MAX_OPERATIONS` — but an
*empty* cell executes none, and then nothing at all bounds `columns × rows` except this. Measured
with the bound lifted to 4 194 304: 1 000 000 empty tiles in 889 ms, 0.89 µs apiece, and a
`/XStep` of 0.001 over a 600-unit fill states 3.6 × 10¹¹ of them — about four days. The 48 real
documents are legitimate hatching, and 14 of them want fewer than twice the bound, so this is a
bound sitting where documents fall. **It is not raised anyway, because the count is the wrong
quantity**: `7680183.pdf` wants 42 282 tiles and takes 14.2 s while `2760154.pdf` wants 765 440
and takes 8.7, so a larger count buys no safety and refuses a different arbitrary set. What it
should become is a bound on the *work*, and `doc/todo/49` carries that as the item.

**`MAX_OPERATIONS` is the one whose population is mostly legitimate, and the honest reading is
that it is the wrong instrument.** Thirty of the 31 are maps, plans and charts that draw in under
14 s. The thirty-first, `7926547.pdf`, wants 53.6 million tokens, takes 49.9 s, peaks at
1.57 GB and produces **495 display commands** — so the bound is doing real work for one document
in 65 944. It stays at four million because **a count is not a cost**: one `sh` paints the whole
page, so no number here bounds the time. What bounds the time is the confined worker's cancel,
which is a kill and which ADR 0241 measured at 0.83–1.97 ms; this bound is the cheap
approximation that runs where the worker does not.

**`MAX_STATE_DEPTH` is the one that looked most like a bound set too low, and the standard says
it is not.** One document of 65 944, wanting 337 where the bound is 256 — a factor of 1.3, and
the kind of margin that invites a quiet raise. Table C.1's figure is **28**. This tree's 256 is
nine times it and the document wants twelve times it, so admitting `0546285.pdf` would mean
setting the bound by the worst file seen rather than by anything. It stays.

**Nothing here is a silent raise and nothing here is a silent refusal**: `MAX_TILES`,
`MAX_OPERATIONS`, `MAX_FORM_DEPTH` and `MAX_STATE_DEPTH` each report by name, which is how the
survey could count them at all, and `crates/pdf-model/tests/hostile_budgets.rs` now holds six
generated fixtures — one hostile and one control for the state stack and for tiling, the
self-referential form, and a four-million-operator stream — so that a change which turned any of
them into a silence would fail. `doc/todo/03`'s promotion budget is untouched at **0 MB**.

## Consequences

- **`doc/todo/49`'s "keep" list is answered.** What it said was owed — one of the 84 read with the
  bound lifted — is done for all 83, and the four entries are rewritten with their populations.
  A new item is added in its place: `MAX_TILES` and `MAX_OPERATIONS` bound counts where they mean
  to bound work, and the two documents that show it are named.
- **The web survey is unchanged in every line but `slow`**, before and after, over all 145
  archives: 65 944 documents, **1138 incomplete**, 173 unopenable, 52 pageless, 45 locked, 23
  encrypted beyond us, and the budget refusals still 48 / 31 / 4 / 1. `slow` goes 2 → 0 → 1 over
  three passes and the 1 is a different document at the threshold, which is decision 1's last
  paragraph.
- **Four ledger notes gain their census**: §8.4.2 (Annex C's 28 against this tree's 256 and the
  web's one witness at 337), §7.8.2 (`MAX_OPERATIONS`' 31, all terminating), §8.7.3.1 (`MAX_TILES`'
  48 and the empty-cell measurement), and §11.6.5.1 (the outside value as a named quantity). No
  status moved: **875 rows, 403 implemented / 250 partial / 18 reported / 83 inapplicable /
  8 writer-side / 113 out-of-scope**.
- **The gates**, all fourteen of `doc/todo/02` §2, run last of all: tests **1584 → 1594** (11 skipped),
  citations **6399 → 6407**, quotations **597 → 601**, and every other line identical — corpus 974 with
  68 incomplete, oracle 1794 at 905 / 68 / 786 with 1/0, 2/2, 14/9, 18/0 and an empty undiagnosed list,
  quorra 911 / 35 / 11 / 17, text 99.2% (24003/24187) and 99.8% (14257/14281), dates 1514 of 1545, XMP
  318 of 319. **The test count was written as 1588 in three files first and corrected in all three**:
  it was copied from a run taken before this round's last six tests existed, which is `doc/todo/02` §6's
  rule needing one more word — run the gate *last*, not merely run it.
- **`CLAUDE.md` is unchanged in this commit**, which is the thing to check rather than to assert:
  `git show --stat` names four source files, one test file, the ledger and the documents.
- **Two instruments are still owed and neither was built here**: a soft mask evaluated over its own
  band rather than the target's, and a work-based bound for a tiling pattern. Both are priced above
  and both are in `doc/todo`.
