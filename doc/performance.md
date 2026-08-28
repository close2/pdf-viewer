# Performance: what has been measured, and what each measurement is of

Status: **record and standing** — every number here was printed by an instrument named beside it.
Read by: whoever is about to optimise something, or to quote a number. `doc/todo/40`–`43` and `45`
hold the items still open; `doc/RENDER_LIBRARY.md` holds what a rendering library would have to be.

`doc/HANDOVER.md`'s reader table points a round that measures anything here. The rule that governs all of it
is `CLAUDE.md`'s: an optimisation is justified by a benchmark and explained by a comment, and a
number is quoted against a number taken the same afternoon.

## The launch path

**Page one goes to the graphics device by the project owner's decision**, so GPU bring-up is on the
critical path and what it costs is a number to keep rather than a cost to hide —
`doc/state-of-play.md` states the decision and this is the timeline under it.

**Since the two-hundred-and-seventy-fourth session `--trace` prints the whole
launch as a timeline** — one `Instant` taken at `main`'s first statement, one mark per milestone,
printed when the first frame lands. It was **145 ms from process start to the first frame** on this
machine's software adapter under `Xvfb`, for a 5-page document *and* for ISO 32000-2's 1023 pages,
and is **98.8 to 119** after two rounds of taking things off it. Nothing the window needs has ever
looked at a PDF, so the document opens on a thread of its own (ADR 0182); and a `wgpu::Instance`
needs no window either, so **it is made on a second thread** and handed to quorra, which added the
entry point for it after `doc/QUORRA_FEEDBACK.md` §8 asked with a measurement (ADR 0185). Of what
is left: `EventLoop::new` 20 to 45 ms, the first present 48 to 53, the device **13 to 19**, the
instance 0.006 to 2.6 and the document's join 5. **Re-run in the four-hundred-and-forty-fifth on the
same software adapter**, ISO 32000-2, one launch: **122.6 ms** to the first present — arguments 0.017,
chrome fonts 2.2, event loop 30.6, window 30.7, graphics instance 42.8, graphics device 68.3, document
joined 72.9 — with five arrow keys turning to page 6 presenting in **9.2 to 16.1 ms**. That is one
sample on a machine with a fuzz corpus merge on it, so it is a check that the shape holds rather than
a new bound; the step that is furthest from the range above is `graphics device` at +25.4 ms against
13 to 19, which is where a loaded machine would show first.
**And the table itself ends with the first frame's own phases since the five-hundred-and-eighty-eighth
session** (ADR 0423), which is ADR 0332's lesson one round of optimisation later: `first scene built`
and `first present` are *one frame*, and on a large drawing they are half the launch under two names
that say nothing about what happened inside them. `FrameCost::scene` gained `handover` — the part of
the translation spent inside quorra's own `upload_*` calls, with `outline_segments` as its
denominator — and the launch table prints scene, handover, encode, transfer, execute, elsewhere and
present under the rows they are the inside of. The device's three phases were previously legible only
from the summary at exit, as medians over a whole run, which cannot answer a question about the one
frame a person waited for. What the division found on the owner's own document is
[todo 44](todo/44-a-draft-that-takes-ten-seconds.md) §6's, and the shape of it is worth having here:
**an upload is not a copy** — quorra converts an outline's cubics to quadratics inside
`upload_outline`, four fifths of a cold frame's `scene` phase, for a representation only its GPU
coverage lane reads and no launch of this viewer takes.

**Two things on it broke a rule `CLAUDE.md` states and both are closed**, in
[todo 42](todo/42-the-launch-path.md): `Document::open` cost 12 to 22 ms on 101 318 objects
against 0.20 on a small file, where the rule is "a 500-page document must open no slower than a
5-page one", and `Outline::read` 3 to 7 ms for a panel nobody had opened. **41% of the first went
in the two-hundred-and-seventy-sixth** — 40% of it was §7.5.6's "most recent copy" rule being
re-decided once per cross-reference entry instead of once per file (ADR 0180, 130.7 M instructions
to 76.6 M) — and the rest of both went *beside* the window rather than in front of it. Measured in
the two-hundred-and-eighty-ninth: **1023 pages and 5 pages now cost the launch the same**, 5 ms of
join either way. The rule read as a statement about `Document::open` itself is still 10 to 13 ms
against 0.2, and todo 42 keeps that question open as a question about the function. **A third cost on
that path turned out to be a clause nobody had read**: §12.8's signature walk spent 1.681 ms
finding nothing on a document whose form could have said so in one integer, and §12.7.3's Table
225 exists precisely so that a processor need not scan — 0.017 ms now, with the ledger row that
called the entry "signature behaviour" corrected (ADR 0181). **Adapter selection is the largest part of bring-up and the backend set is not the
lever**: `examples/bring_up` shows Vulkan-only moving the cost out of instance creation and into
`request_adapter` with the total unchanged (ADR 0179). **On the machine's real adapter, headless,
the first frame costs 18.2 ms and the tenth 4.1** — and a one-second sleep before it changes
nothing, so the ~12 ms difference is first-use allocation rather than warmth. `CLAUDE.md`'s ban on
waiting for warmth therefore costs nothing measurable here; `examples/first_frame` is the
instrument and `doc/QUORRA_FEEDBACK.md` §9 is the ask. The same arithmetic puts a launch on the
real GPU at **75 to 90 ms** against `lavapipe`'s 145, and nobody has run that — the window half of
it is the user's to measure. The CPU backend keeps its other two jobs:
the correctness oracle, and the frame the device refuses.

### 3b. The quorra backend, and what a corpus-scale comparison found in it

**A second GPU backend arrived in the hundred-and-eighty-sixth to -eighth sessions**, written
against `doc/RENDER_LIBRARY.md`'s brief in its own tree and adapted here by `render-quorra`; the
window presents through it. It came with eleven cross-backend scenes and four real pages, which
is a better suite than the Vello backend ever had — and trap 12b is about exactly that gap.
**`render-quorra/tests/corpus.rs` closes it** (ADR 0156): every one of the 974 documents' first
pages, both backends handed the *same display list*, so a difference is two rasterisers
disagreeing and a refusal is a hole in the new one. Three ratchets held by name — refused,
differing at the edges (similarity above 0.99), differing in shape — and both renders of every
differing page written to `target/tmp/quorra/<stem>/`.

**The first run was 900 agree, 50 differ, 7 refused, and every finding in it has been answered.**
The three were written up for the library's team in `doc/QUORRA_FEEDBACK.md`, with the command
that reproduces each, and the same document now carries what closed them:

- **§10.7.4's rule was not asked for** — trap 2's shape. `issue4260_reduced.pdf` drew an empty
  box at similarity **0.49** because the rule lives in `pdf_render::collapsed` so that both
  backends inherit it (ADR 0154) and the new one did not ask. It asks now: **0.9938**.
- **The caches never evicted**, which only a long run can see: 533 of 952 pages refused at 4×
  with the 512 MB budget full, and a page refused in the full run passing on its own. Entries
  carry recency now and the device releases down to half its budget after every frame —
  **zero** resource refusals at 4×, where 413 pages agreed before and 918 do now.
  **That zero decayed to one and nothing was watching**, which the four-hundred-and-sixty-second
  session noticed while measuring something else and wrote up further down this file.
  `22060_A1_01_Plans.pdf` — 72 sampled images — would hold 548 104 348 resource bytes against the
  536 870 912 `max_resource_bytes` default. It is a ratchet now rather than a sentence
  (`corpus.rs::REFUSED_BY_THE_DEVICE_AT_FOUR`), and that is the answer to a claim of this kind rather than a
  better paragraph: nobody knows how many rounds passed between the zero becoming a one and
  somebody noticing, and a test is the only thing that would have.
  **And the ratchet is back to zero resource refusals since the five-hundred-and-thirty-ninth
  session, on a change that is not the backend's at all**: those 72 uploads were **8 distinct
  rasters**, because `image::decode_parts` ran at every `Do` and each one allocated its own copy.
  A cache of the decoded raster (ADR 0374) leaves the page uploading what it actually holds. The
  refusal was arithmetic against a byte budget, and the question such a refusal asks is *who is
  spending the bytes* — this time the answer was two crates upstream.
- **Six refusal messages named a byte count under the budget they exceeded.** They add up now,
  and the one that replaced the mis-stated limit says what it is.

**And the shape list bought something nobody asked for**: reading it found strokes under an
*anisotropic* transform being given one scalar device width, which is exact for a similarity and
exactly wrong for a shear. Four documents left the list.

**Where it stands is what the gate prints**, and it is not written here: `tools/state.sh` runs it
and `crates/render-quorra/tests/corpus.rs`'s `REFUSED_BEFORE_THE_SCENE` and `REFUSED_BY_THE_DEVICE`
hold the refusals to equality — split since the five-hundred-and-seventy-eighth along the *stage*
the refusal happens at, so that a name leaving one of them means one thing — so the count
in a paragraph is always the one a round did not run. **What is worth stating is the *shape* of the
refusal list, because that is an argument rather than a number**: at the page's own scale one page
refuses and its reason is a *device capability* — `bug1721218_reduced.pdf`'s rasterised coverage
outgrows the 16 384 × 16 384 texture this adapter allows. Every refusal that was a **clause** the
backend could not state has been closed (§11.4.4's non-isolated group, §11.4.7's four-component
page, §11.4.6's knockout pair — ADRs 0237, 0262, 0291), and since the four-hundred-and-seventy-eighth
session every refusal that was **arithmetic against a byte budget** has been closed too, upstream
and at every scale (`QUORRA_FEEDBACK.md` §22). This paragraph used to carry three successive counts
and each was stale before it was read.

**Where it stood when this section was written: 914 agree, 42 differ, 1 refused** — and 27 of the 42 are the glyph
antialiasing floor, which shrinks as the page grows (17 pages differ at 2×, 16 at 4×). It was
913/43 until the three-hundred-and-sixty-eighth session: `issue4260_reduced.pdf` returned to
`agrees` the moment a §10.7.4 mark became a whole device pixel row instead of a band at the
shape's fractional position, because a band is exactly what two rasterisers distribute in their
own ways and a whole row is not (ADR 0208).

**Performance, offscreen, readback included** (AMD 890M, RADV, release): at the page's own scale
the CPU backend takes **2.55 s** over 956 pages against quorra's 6.26 s, median page **2.05×**;
at a window's 2× it is 5.21 s against 10.16 s, median **2.87×**; at 4× — comparable for the first
time now that the eviction fix draws 934 of 952 pages — 11.34 s against 24.13 s, median 3.24×.
The totals ratio *improves* with scale while the median *worsens*, and both are true — our CPU cost grows with the pixels
while the median page is dominated by a per-frame floor that does not. Quote the total against
the median and say which. The window is not measured here and this gate is deliberately not the
place to: a presented frame pays no readback.

### 3c. What interpreting one very large page costs, and why the attribution keeps expiring

**The launch path's largest single item on the owner's document is one page's interpretation**,
and `doc/todo/44` is the item. What belongs here is not its number — `examples/callgrind_interpret`
prints that — but the thing three rounds of measuring it have established, which no command prints:

**an attribution of this page has expired every time it has been acted on.** ADR 0332 measured
the lexer at 63.6% and named two candidates under it; ADR 0341 took both and the lexer became
40.9%. ADR 0365 then changed the window the stream arrives through, which added per-token cost
*outside* the lexer. By the time ADR 0370 retook the profile, the function ADR 0332 ranked fourth
— marshalling a path operator's operands — was second at 18.16%, and it had been there all along
under a lexer twice its size. **The rule that comes out of it**: a profile is a statement about a
tree, it decays at the rate that tree changes, and a round that optimises from a table it did not
print is optimising the wrong function. Retaking it costs one callgrind run.

**A fourth retake, in the five-hundred-and-eighty-eighth**, and it is the rule working rather than
failing: ADR 0370's three levers are gone from the profile, and what the interpretation of that page
is now made of is `Lexer::next_token` at 40.2% and §7.4's inflation through ADR 0365's window at
23.1%. **A quarter of the launch's largest step is decompression**, which no table in this tree had
said and which no launch measurement could have told anybody — the instrument is
`examples/callgrind_interpret`, and the run this came off was `examples/zoom_frame` under callgrind,
which interprets the page on its way to drawing it.

**And the five-hundred-and-eighty-ninth divided both halves** (ADR 0424), which is what the rule above
asks a round to do rather than optimise from a share. Three things belong here because no command
prints them.

- **The lexer's 40% is numbers.** That page states 20 831 607 tokens and 17.65 million of them are
  §7.3.3 numbers over 104.5 million digits — a drawing's coordinates — so the lexer's cost is the
  *number* path and not the token dispatch. It walked each of them twice, once to find the run
  §7.2.3 bounds and once to read the value, and fusing the two is worth 5.4% of the page. What the
  fused loop is *spelled* as turned out to matter more than the fusion: `for &byte in body` with a
  `read += 1` beside it measured **750 M worse** than `while let Some(&byte) = body.get(read)` — the
  same function, the same answers, 6.5% of the page in code with no line to attribute it to. ADR
  0370 met that from the other side and this is the second instance; **on a per-byte loop, ask what
  the spelling costs before believing the algorithm.**
- **The inflation is one inflation and there is nothing to memoise.** All of it is inside
  `Window::refill`, 2 409 calls of about 60 KiB producing the stream's 147 972 263 bytes once, at
  16.6 instructions per output byte on a stream that compresses only 2.96:1 and is therefore mostly
  literals. `doc/todo/41`'s cache and `doc/todo/14`'s window were both read and neither applies.
- **What is ours around it costs 613 M**, and is on the launch path rather than on an error path,
  because Table 5 makes `/Length` an indirect reference for a producer that streams its own output:
  §7.3.8.2's `endstream` search scanned 89 MB nine bytes at a time (446 M, now 253 M on a first-byte
  search), and the encoded stream is copied **twice** — once by the parser's guess and once by
  `Document::with_stated_length`'s correction, 99 MB of `memcpy` for a 49.7 MB stream. The second is
  `doc/todo/10` §3's residue, priced and not taken. `memchr` was declined for the search on principle
  3: hand-written `unsafe` SIMD fed a hostile file's bytes in the crate that most wants
  `#![forbid(unsafe_code)]`, for 2.3% of one document's launch.

**And the second rule is about reading a share rather than a number.** ADR 0370 found
`Lexer::next_token` within 0.07% of where ADR 0341 had left it while its *share* had fallen two
points — the denominator had grown. A share that moves because the denominator moved says nothing
about the function, and the only way to tell the two apart is to keep the absolute figure. Both
ADRs' tables carry it for that reason.

### 3d. What a *zoom step* costs, and the two rows of it that were never what they looked like

**The frame a person waits for on a large drawing is a magnification against warm caches**, and
`crates/render-quorra/examples/zoom_frame.rs` is the instrument that draws exactly that: two frames
on one device, the second placing the same display list at a new scale. It reports what nothing else
here reported — quorra's `bytes_uploaded`, its `Timings::readback`, and the named spans of
`Timings::phases` — and **it runs on the real adapter without a window**, which is the correction
`doc/environment.md` now carries.

What belongs here is not the numbers (the example prints them) but two shapes that cost this project
rounds of misreading, both found in the five-hundred-and-fifty-second session (ADR 0387):

- **A count beside a duration will be read as its denominator, whether or not it is one.** The frame
  line printed *resource* uploads next to `transfer`, and the two are unrelated: the frame that
  hands over 58 029 resources moves fewer bytes than the frame that hands over 40. The fix is not a
  faster phase, it is printing the quantity the duration is actually about — and the number had been
  crossing the boundary, unread, since the instrument was built.
- **A phase one side measures and the other drops gets attributed to something else.** `elsewhere`
  was documented here as the acquire, the present and the readback; measured, the first two are
  three hundredths of one per cent of it and the third was a phase this tree was not carrying at all.
  Its real contents are host time inside the library's own `render` call — one span it measures and
  discards, one it never times.

**And the row that decides where every remaining lever is**: on the owner's own adapter, `execute` —
the graphics device's own timestamps — is **0.07 %** of a zoom frame of that document. A viewer whose
slow frame is 99.9 % host thread has no lever on the device at all.

## What a soft mask cost, and what naming one constant took off it

**Instrument: `crates/pdf-model/examples/open_one`**, which opens, interprets and rasterises one
document in a process that can be killed, with per-stage timings — plus scratch counters through
`render-cpu` for the round that took the measurement (ADR 0271, not committed).

**Where the two slowest documents of a 65 944-document sample of the web spend their time.** Both
complete; neither reports anything; neither is slow to *parse*:

| | `Document::open` | page one | `interpret` | rasterise | total |
|---|---|---|---|---|---|
| `0423548.pdf` (1843 × 5103) | 6.8 ms | 7.8 ms | 2.10 s | **23.3 s** | 25.4 s |
| `6081357.pdf` (2552 × 1693) | 3.8 ms | 4.5 ms | 0.27 s | **52.3 s** | 52.6 s |

**And inside the raster, in one line.** `build_soft_mask` drew each mask group into a buffer the
size of the *target* and converted all of it — `Pixmap::take_demultiplied` allocating a second
target-sized buffer and dividing by the alpha per pixel, then `SoftMask::values` deriving §11.5.3's
luminosity in floating point per pixel:

| | distinct soft masks | builds | the value pass | pixels | of which wholly transparent |
|---|---|---|---|---|---|
| `0423548.pdf` | 89 | 456 | **22.4 s of 25.4** | 1 686 997 422 | 1 662 342 098 — **98.54%** |
| `6081357.pdf` | **912** | 895 | **51.0 s of 52.6** | 3 866 879 720 | 3 865 478 534 — **99.96%** |

A 4.3-million-pixel page with 912 distinct masks does 912 times the page's own work, and 99.96% of
it derives the same constant: a mask group covers its `/BBox` and the rest of the buffer is the
transparency it was allocated as, for which §11.6.5.1's answer is one number. `SoftMask::outside()`
is that number, computed once. **The two arms of the branch call the same function, so this is
exact rather than approximate** — and no page of the oracle's 1794 or quorra's 957 moved.

**What it is worth**, three samples each, `[profile.release]`, machine otherwise idle:

| document | before | after | |
|---|---|---|---|
| `0423548.pdf` | 25.70 / 25.78 / 25.80 s | **6.61 / 6.70 / 7.26 s** | 3.7× |
| `6081357.pdf` | 53.61 / 53.75 / 53.78 s | **3.72 / 3.92 / 4.02 s** | **13.6×** |

Spreads of 0.10 and 0.17 s before, 0.65 and 0.30 s after; both differences are two orders of
magnitude larger than either.

**And the survey's `slow` count is not how to state that**, which three passes over all 145
archives established: before **2**, after **0**, and a third pass on an idle machine **1** — a
different document, `1284722.pdf`, which takes **11.13 / 11.14 / 11.23 s alone** and crossed the
30 s budget only because the survey rasterises 24 documents at once. A per-document wall-clock
budget measured under a 24-way parallel load is a property of the machine as much as of the file.
Every other line of the survey is identical before and after: 65 944 documents, 1138 incomplete,
173 unopenable, 52 pageless, 45 locked, 23 encrypted, budget refusals 48 / 31 / 4 / 1.

**What is left on those two pages, and it is not this.** `0423548.pdf`'s remaining 6.6 s is 2.1 s of
interpretation and **2.85 s of `initial_backdrop`**, which copies the whole *surface* into a fresh
buffer for each of its 136 groups, 132 of them non-isolated — **4.3 GB** of buffer allocated and
copied where the groups' own bands are **82 MB**, a factor of 52. (The surface is a strip rather
than the whole page here, because the page is drawn in strips; the 4.3 GB is the counter's own
sum rather than 136 × the page.)
`6081357.pdf` allocates 31.6 GB of target-sized group buffers for 487 MB of band. The buffer is
target-sized on purpose (one coordinate system rather than two that have to agree), and taking it
to the group's own band was `doc/todo/40`'s item. **The copying half of it was taken in the
four-hundred-and-ninety-third session** (ADR 0328): the backdrop copy, the mask conversion and the
mask storage are banded — byte-identically, because none of the three is drawing arithmetic — while
the drawing buffers stay target-sized on ADR 0219's argument. The numbers are that ADR's; the chain
half of the item stays in the todo file.

**What the four resource bounds cost, over the same 65 944 documents** (ADR 0271, and
`doc/todo/49` has the reasoning). Every one of the 83 documents they refuse was opened with its
bound lifted, one process apiece:

| bound | documents | with the bound lifted |
|---|---|---|
| `MAX_TILES` 4096 | 48 | all terminate, 0.06–14.2 s, wanting 4104–895 500 tiles |
| `MAX_OPERATIONS` 4 M | 31 | all terminate, wanting 4.1–53.6 M **lexer tokens**, 0.27–49.9 s |
| `MAX_FORM_DEPTH` 16 | 4 | **all four are cycles**: lifted to 256, all four reach 256 |
| `MAX_STATE_DEPTH` 256 | 1 | wants 337, draws in 0.05 s |

and one number that is a property of the loop rather than of any document: with `MAX_TILES` lifted,
**1 000 000 empty tiles interpret in 889 ms reporting nothing** — 0.89 µs apiece, and `/XStep 0.001`
over a 600-unit fill states 3.6 × 10¹¹ of them, about four days. None of the four bounds moved.

**And the second column of that table was in the wrong unit for the whole of the bound's life**
(ADR 0306). `MAX_OPERATIONS` said *operators* and counted lexer tokens, which §7.8.2 makes seven of
for one `c`; the counter now counts operators and the value is unchanged at four million. Measured
in the right unit over 926 680 pages of 65 967 crawled documents, 48 pages pass four million tokens
and **8** pass four million operators — so the row above is a population of *tokens* and a round
re-running it should expect a smaller number. The instrument is
`cargo run --release -p pdf-model --example content_budget_census -- <dir>…`, which counts both in
one pass and prints the largest decoded stream and the largest page `/Contents` total beside them.

## What a document-wide search costs, and what a bound on memory bought

**The instrument is `viewer-core/examples/find_cost`**, which drives `Command::Find`'s pump with no
host at all — nothing rendered, nothing presented — because the first measurement of this feature
measured a host: 19.25 s of which 13 s was `viewer-ui` putting a whole window on the screen per step
(ADR 0250). A needle the document does not contain is the worst case and the only one whose cost is
a property of the document rather than of where the word sits.

**Where a cold search's time goes**, ISO 32000-2's 1023 pages, `--profile gates`, medians of three:
§7.7.3.2's page tree walked from the root by `Pages::get`, once per page, **1.12 s (19.6%)**; the
content stream through `interpret_with`, **4.59 s (80.4%)**; building the index itself,
`Pages::new` × 1023, **2.8 ms**. The readback of the whole document is **2 658 697 bytes**.

**What the readback cache bought**, four-hundred-and-twentieth session, ADR 0256 — medians of seven,
against the same binary with the budget set to zero so the two differ in the cache and nothing else:

| | before | after |
|---|---|---|
| first (cold) sweep, 1024 steps | 5.51 s (5.50 – 5.66) | 5.61 s (5.42 – 5.74) — **unchanged**, inside the spread |
| repeated sweep | 5.45 s (5.37 – 5.50) | **7.27 ms** (6.13 – 8.16) |
| peak RSS, one sweep | 209.6 MB | 211.9 MB |
| peak RSS, two sweeps | 234.0 MB | 211.9 MB |
| readback held | — | 2 658 697 of 4 194 304 bytes, 0 evicted |

The two RSS rows are worth reading together: a cache costs 2.3 MB on a single sweep and *saves* 22
on a second, because the second sweep no longer interprets a thousand pages. The budget is 4 MiB per
open document, `viewer_core::readback::BUDGET`, and `Viewer::readback_cache` is how a person reads
what it holds — `pdf-viewer --trace=search` prints it when a search ends.

**And in the window**, `Xvfb` at 1100×1200 with lavapipe, timed from the key press to the line the
search prints, medians of three:

| | `SEARCH_REDRAW` = 16 steps | `SEARCH_PROGRESS` = 100 ms |
|---|---|---|
| first search | 4.94 s | **4.79 s** (4.756 – 4.789) |
| repeated search | 0.51 s | **0.021 s** (0.018 – 0.026) |

A repeated search costing 7.27 ms inside `viewer-core` cost **0.51 s** in the window, and all of the
difference was the progress count: 1024 ÷ 16 = 64 whole windows presented at about 13 ms each to
move a digit. That constant was right when ADR 0250 measured it, because a step then cost 5.7 ms —
**a step count is a proxy for time, calibrated against one step cost**, and when the step cost fell
by three orders of magnitude the proxy stopped tracking what it stood for. It is a clock now, in the
host, where `doc/ui-boundary.md`'s rule 3 permits one.

**What was left of a cold sweep after that, and what took it, is three ADRs and not this
paragraph** — §7.4's filter chain memoised (ADR 0317, −36.5% of a hundred-page sweep), §7.7.3.2's
page tree walked without copying (ADR 0330, −12.1% of the whole), and the key a dictionary lookup
allocated (ADR 0335, **−1.92%**: 37 642 044 068 → 36 920 639 974 instructions, with `malloc` down
19.1% and `free` down 15.7% because `Dictionary::get` stopped building an `Arc<[u8]>` on each of
3 278 302 lookups). All three are callgrind, two binaries from one tree, and all three produce a
byte-identical readback. **What is left is `interpret_with` at 93.5% of the sweep**, which is the
page being read: the only way to make it smaller is to read fewer pages, and both ways of doing
that are refused with numbers in ADRs 0250 and 0260.

## What the object cache does, and what making it `Sync` cost

**Counted rather than guessed, in the four-hundred-and-twenty-fourth session** (ADR 0260), with a
temporary counter build — one `AtomicU64` per call site inside `pdf_syntax::Document`, not in the
tree, because a counter on `Document::get` is a counter on the hottest path in the program. ISO
32000-2, 1023 pages, 101 318 objects:

| | `get` | hits | `load` | `expand` | `resolve` | `get_key` | `decode` |
|---|---|---|---|---|---|---|---|
| a page turn (page 2) | 133 | 80 | 53 | 41 | 3 881 | 813 | 17 |
| a whole cold sweep, per page | **829.3** | **768.9** | 60.4 | 55.8 | 8 710.6 | 4 275.3 | 12.4 |
| the same sweep again, per page | 829.3 | **829.3** | **0** | **0** | 8 708.4 | 4 272.4 | 11.7 |

Three readings, and two of them were not what the tree assumed. **The object cache answers 92.7% of
a cold sweep and 100% of a repeat, and that is worth 5.5% of the wall clock** — 6.15 s against
5.81 s, medians of three — so a sweep's seconds are not in `Document::get`. **`resolve` is called
ten times as often as `get` and mostly touches no cache**, because most objects are not references;
only `get` takes a borrow, which puts the borrow count at about **1 070 a page for 6 ms of work**.
And **`decoded_stream_data` was not memoised at all**: 12 717 calls over one sweep and **11 975 over
the second sweep of the same document**, which is a filter chain re-run. `document.rs`'s module
comment claimed otherwise and was corrected; the section below is what the four-hundred-and-eighty-
second session found when it priced those calls.

**What `RefCell` → `RwLock` cost**, which is what made a `&Document` shareable between threads:

| | `RefCell` | `RwLock` | |
|---|---|---|---|
| `callgrind_interpret`, instructions | 2 208 807 721 | 2 209 269 060 | **+0.021%** |
| `callgrind_open`, instructions | 78 464 732 | 78 357 201 | **−0.14%** |
| `find_cost` cold sweep, medians of seven interleaved | 5.69 s (5.61 – 5.91) | 5.78 s (5.54 – 5.90) | inside the spread |
| launch, `document joined`, twelve samples interleaved | 5.46 ms | 4.95 ms | inside the spread |

The launch's *whole* figure is deliberately not quoted against itself: `EventLoop::new` ran 28 to
55 ms across those twenty-four launches and **which end of that range a run landed on depended on
whether it was the first launch after the X server went idle**, not on the binary — reversing the
order reversed the apparent difference.

## What §7.4's filter chain costs when a document is read end to end

**Counted before anything was built**, four-hundred-and-eighty-second session (ADR 0317), with a
temporary counter build in `Document::decoded_stream_data_reported` — the call sequence keyed by the
address and length of the encoded bytes, so a repeat is a repeat of the same allocation. One sweep
of `interpret_with` over ISO 32000-2's 1023 pages, `--profile gates`:

| | |
|---|---|
| calls, of which filtered | 12 734 / 12 586, over **3 936 distinct streams** |
| calls that decode something already decoded | **8 798** |
| of the sweep's wall clock, inside `decoded_stream_data` | **24.6%** |
| of the sweep's wall clock, decoding *again* | **23.4%** |
| decoded bytes | 877 MB, of which **830 MB is re-inflation of 46 MB** |

Three streams are 3.2 s of the 3.9 — 193 KB inflated 1993 times, 136 KB 1486 times, 96 KB 808 times.
They are the document's font programs, and a font program is decoded once per *use*. **This is the
same measurement `doc/todo/41` took at 0.7% and it is not a contradiction**: that one walked a corpus
one page per document, where nothing can repeat. The counts are deterministic; the two *shares* are
one run's ratio, taken inside the process so both halves meet the same machine — a quieter run of
the same build put them at 29.5% and 28.1%.

**What the memo buys, in instructions**, because the machine carried a load average of 20 to 30 that
session and a wall clock on it is not evidence — seven interleaved samples an arm gave medians of
9.75 s against 6.73 s with ranges of 9 s and 8 s, which is the right direction and no number. Two
binaries from one tree, `DECODED_BUDGET` set to 0 for the off arm, under callgrind:

| | instructions | |
|---|---|---|
| ISO 32000-2, 100 pages, no cache | 4 933 481 135 | |
| ISO 32000-2, 100 pages, 4 MiB | **3 133 405 696** | **−36.5%** |
| the same again | 3 132 945 345 | 0.015% apart |
| `issue6961.pdf`, 2 pages, 276 decodes of which 2 repeat, no cache | 942 918 105 | |
| the same, 4 MiB | **920 371 576** | **−2.4%** |

The readback both arms produce is byte-identical, which is the search path's standing gate. Peak resident over a
full sweep is **213.6 / 214.0 MB with the cache against 211.6 / 209.6 without** — the budget, and
nothing else. A least-recently-used replay of the recorded sequence says what the bound gives up:
1 MiB saves 21.4% of the sweep, 4 MiB saves 23.1%, and an unbounded cache holding the whole 46.6 MB
working set saves 23.4%.

### And the arm where the decode produces nothing, which was left out until ADR 0437

A `FilterRefusal::TooLarge` costs up to `Limits::max_stream_len` of inflation to reach and was
answered again on every read, so a document naming one refused stream from every page paid a
gibibyte per page. The memo keeps the refusal now, under the bound it was reached under. Twenty
pages of one hex-wrapped Bomb B in a form `XObject` — the chain that defeats §7.8.2's window, which
is what the shape had become since ADRs 0427–0430 — one cold `find_cost` sweep, three runs an arm,
alternating:

| | cold sweep, 20 pages |
|---|---|
| without | 5.92 / 6.12 / 5.92 s |
| with | **2.76 / 3.23 / 2.89 ms** |

An ordinary 25-page document is inside the noise either way (47–51 ms against 36–55). What the memo
still declines is a refusal whose *encoded* bytes do not fit the budget, because the entry pins them.

## What a strip costs a scanned page, and why the planner could not see it

**`render-cpu` cuts a target into strips and replays the display list into each**, and
`pdf_render::replay_ratio` bounds that replay by the **rows** a command covers. That is exact for a
fill and blind to `Image::area_averaged`, whose cost is per *source* sample and does not shrink with
the band — so a page of one deep reduction reads as a replay of 1.00 and reduces the same samples
once per strip.

`issue12963.pdf` page 1 — a 2480×3506 `JBIG2Decode` scan on 596×842 — under
`examples/callgrind_rasterise`, two draws, both arms one sitting, pinned to four cores so that the
ratio is legible:

| 4 strips, 2 draws | before | after |
|---|---|---|
| whole program | 3 884 597 422 | **2 248 310 893** |
| `Image::area_averaged` | 2 178 913 072 (56.1%) | **544 728 268** (24.2%) |

Exactly fourfold on the reduction, which is the strip count. Unpinned, the machine grants thirteen
strips and the before arm spends **75.74%** of the whole program in that one closure.

**`examples/strip_spans`**, medians of five interleaved runs an arm on a quiet machine, each figure
already the fastest of five renders — before, the page got *slower* the more strips it was granted:

| strips | 1 | 2 | 4 | 8 | 16 |
|---|---|---|---|---|---|
| before | 17.0 ms | 14.0 | 13.3 | 20.2 | 30.9 |
| after | 14.0 ms | **10.8** | **7.8** | **6.7** | **6.7** |

The one-strip row is the control, where there is one reduction either way; a page with nothing to
share is unmoved too
(`scan-bad.pdf`, one command, one strip: `Image::area_averaged` at 527 379 600 in both arms).
`render_cpu::images` reduces each image once on the thread that plans the strips and holds it on the
rasteriser, on ADR 0297's key, so a host that redraws one page keeps it as well — 957 rasterised
first pages byte-identical. **The memo may not block**: a lock held across `area_averaged` is
re-entered by its own thread, because rayon can steal another strip's job onto the stack that is
waiting inside `par_chunks`. Both blocking forms were written and both deadlocked the corpus gate.
ADR 0731.

## What cross-page parallelism buys, and at what memory

**`pdf-model/examples/parallel_sweep`** reads every page three ways — one thread; N threads over one
`&Document`; N documents opened from the same bytes, one per worker — inside a pool built with
exactly N, because `interpret` bands §8.9.5's colour conversion across
`rayon::current_num_threads()` of its own. All three read the same 2 658 697 bytes at every count.
Medians of three, 12 cores / 24 threads, background load average about 4 (ADR 0260):

| threads | one | shared | per-thread |
|---|---|---|---|
| 1 | 5.97 s | 6.02 s | 6.10 s |
| 4 | 6.01 s | 1.93 s | 1.98 s |
| 8 | 6.08 s | 1.59 s | 1.50 s |
| 24 | 6.11 s | 1.61 s | **1.18 s** |

Repeated on warm caches at 24 threads the order reverses — **shared 1.11 s, per-thread 1.22 s** —
because a cold shared sweep takes 61 836 exclusive locks and a warm one takes none, while the
per-thread arrangement re-opens 24 documents. Peak resident, `VmHWM`, two sweeps each: one thread
**225 MB**, shared **398 MB** at 8 and **625 MB** at 24, per-thread **488 MB** at 8 and **966 MB**
at 24.

So **cross-page parallelism was never blocked by `!Sync`** — N documents in N threads needs nothing
from this crate and is the faster of the two here — and a 4× first-search speedup costs 2.8× to
4.3× the peak memory, against an owner's stated "1 GB is definitely too much". ADR 0260 declines it
for now and says what would change the answer.

## What the window itself has been measured at

**This was `doc/HANDOVER.md`'s gate table's `window` row**, moved here whole in the three-hundred-and-ninety-fifth rather than deleted with the rest of that table's narrative: it is not a gate — `Xvfb` and `xdotool` are not build dependencies and a test that skipped silently would be worse than none (`doc/environment.md` has the recipe) — but every figure in it was taken by running the program, and the loop it exercises is the one no gate touches.

**Tab draws a focus ring** — `160F-2019.pdf` under `Xvfb`, two presses, the ring round its first widget, captured with `xwd` (ADR 0126's recipe, and `xdotool windowfocus` before `key --window` is what makes the press arrive). page one of ISO 32000-2 drawn in a real window on `Xvfb`, presented in **22.4 ms**, and five arrow keys turn to page 6 presenting in **9.5 to 15.9 ms** with nothing refused — re-run in the three-hundred-and-ninth at 9.6 to 22.7 ms over the same five keys, with the whole launch **125.4 ms** on a machine running the corpus gates beside it (`arguments` 0.016, `chrome fonts` 3.4, `event loop` 28.9, `window` 29.2, `graphics instance` 33.2, `graphics device` 55.0, `document joined` 58.9, first present 125.4). Measured undisturbed in the two-hundred-and-ninety-second, where the whole launch is **98.8 ms** (ADR 0179's timeline, ADR 0185's threads), on `lavapipe` through quorra and so not comparable with a real adapter, where `examples/first_frame` puts the first frame at 18.2 ms and the tenth at 4.1; the shape is what the row is for. **The sidebar opens by itself** — that document states `/PageMode /UseOutlines` — and its title bar reads *ISO 32000-2:2020 (PDF 2.0) including Errata Collection 3* rather than the file name, because it also sets `/DisplayDocTitle` — and since the two-hundred-and-ninety-fourth that string comes from the `dc:title` Table 147 actually names rather than from `/Info /Title`, which on this document happens to say the same thing **Re-run whole in the three-hundred-and-thirty-fifth**: five arrow keys turn ISO 32000-2 to page 6 under `Xvfb`, presenting in **8.7 to 15.6 ms**, with the launch at **98.5 ms** to the first present and the sidebar open on §12.3.3's outline — and the outline's rows carry the placeholder boxes of ADR 0195, which is two features meeting in one screenshot **And a click places the caret and a drag selects inside a value, in the three-hundred-and-eighty-eighth**: the same `160F-2019.pdf` under `Xvfb`, a click at (430, 174) into `A.NOM` and `t y p e d` leaving the caret at **x = 381–383** over rows 169–179; a second click at **x = 368** moves it to **x = 369–371** — the only pixels in the whole frame that changed — and one `x` there sends `SetField { value: "typxed" }`, the letter landing at offset 3 because that is where the click was. A drag from 358 to 382 highlights **x = 357–382** in the selection's own blue (the pixel at x = 370 goes from (216, 226, 237) to (119, 160, 237), and 350 and 390 do not move) and **sends no command at all** — three moves, three presents at 37.1, 27.2 and 25.0 ms, `pointer Dragged` absent from the trace. Ctrl + C says *copied 5 bytes out of the field*, Ctrl + X sends `SetField { value: "d" }` and Ctrl + V puts `typxed` back byte for byte, each one `Edit::SetField` and no new message. And `o` typed at the field reaches the field — `typxeod` — with **zero columns changed left of x = 300**, which is the sidebar not toggling (ADR 0225). | ADR 0126's recipe, session 335 | **And the owner's own report closed in the window, in the three-hundred-and-fortieth**: `doc/PDF20_AN001-BPC.pdf` page 3 under `Xvfb`, twenty presses of `+` to the 6400% clamp and six of `-` back, with §12.3.3's outline panel's ink flat at **19.82, 19.82, 19.89, 19.89, 19.89** — the sidebar that used to be its background rectangle alone above ~2000%, drawn whole at every rung (ADR 0198). **And a person fills in a form field in it, in the three-hundred-and-forty-ninth**: `160F-2019.pdf` under `Xvfb`, a click at (430, 174) landing on the field `A.NOM` — which the program says out loud — then `t y p e d` and one Backspace, with the field's own row reading **23.84, 28.79, 27.55** of 255 and the word *typed* becoming *type* in the picture (ADR 0201). **And it shows a caret, in the three-hundred-and-seventy-first**: the same click into `A.NOM` puts a two-pixel line at **x = 356–357** of the empty field — column ink 2656 and 2361 of a possible 4080 over the sixteen rows the caret spans — `typed` moves it to **381–382**, two `Left` presses to **370–371**, and a Backspace *there* sends `SetField { value: "tyed" }` and leaves it at **364–365**, which is the middle of a value being corrected rather than deleted back to. Escape takes it off the screen (the darkest columns in that band are the glyphs' own, 358, 361, 366 and 375) **and the program is still running**, which it was not before this round. An arrow key sends no command at all and presents in **20.6 and 18.4 ms**; a character costs an edit of 5.1 to 8.5 ms and a render beside it. And the tab key aims the keyboard: one press, no click, and `x` reaches the field `F.1` with the caret at **x = 394–395** (column ink 3702 and 3611) — §12.5.1's ring and §12.7.4.3's caret in one screenshot (ADR 0211). **And it declines an operation on the document's instructions, in the three-hundred-and-seventy-third**: a hand-built form whose author certified it `/P 1` under `Xvfb`, a click at (300, 480) landing on the field `name` and one `t` — and the window prints *this document's author certified it as final (§12.8.2.2's /P 1), so it permits no change at all — filling in a form field was not done*, then *this reader is obeying that; --ignore-restrictions turns it off*. Started again **with that flag**, the same click and `t y p e d` are accepted, `s` writes 1598 bytes, and the update's own bytes carry `typed` above the producer's 1004 unchanged byte for byte with the `/DocMDP` still in them — which is the two halves of ADR 0212 in one host: turning the restriction off is the reader's, and nothing about the file was made to lie.

## What the rendering library's feedback loop has closed

**And a fourth, closed in three rounds from the three-hundred-and-thirty-sixth, which is the
`doc/QUORRA_FEEDBACK.md` loop working twice.** Two symptoms at high magnification — quorra's GPU
coverage lane drawing the **wrong glyph** after a larger frame, and this host's **sidebar** losing
its rows above about 2000% — turned out to be one defect: the winding texture is kept between
frames and grown to the tallest sheet any frame has needed, and what leaked across frames was its
*size*, so a shorter frame's geometry was stretched by `held ÷ sheet` and every tile resolved what
the stretch had put under it. Reported with a two-frame reproduction, **answered upstream at
`52b07f29`**, and verified here the day it was published: every rung of `zoom_ladder`'s descent
equals its ascent, `chrome_ladder`'s one-device pass equals its device-per-rung pass, and §0's
corpus gate is unmoved at **914 / 42 / 1 / 17** — which the feedback document recorded as 913 / 43
for sixteen sessions and the three-hundred-and-eighty-fourth corrected. What this tree owed and now
has is the *instrument*: no gate here rasterised more than one display list, and a window draws
several into one scene.

**And a fifth, which was a request rather than a defect: §12, a backend a caller can name.**
Answered at `2531f447`, which is what `Cargo.lock` pinned from the three-hundred-and-eighty-fourth
session until the four-hundred-and-thirty-eighth moved it to `89d7dd77` for §11.4.4's flag (ADR 0274) — `create_instance_with(backends)` exactly as asked, plus `Device::adapter_names_on`,
which closes the trap the parameter would otherwise have opened, and a decision *not* to read
`WGPU_BACKEND`. `pdf-viewer --backend` is what this tree does with it (ADR 0221). The same pull
carries `7cbf6e8`: a `Device` now joins its warm-up thread in `Drop`, because a device dropped
before it was warm could reach `exit()` with a thread still inside `vkCreateGraphicsPipelines`
while Mesa tore the driver down under it — a crash *after* a test suite reports success, which is
the shape nobody attributes. Both ladders and all eight gates were re-run on the new revision and
none moved.

**And a third, closed on the other side of a boundary.** A frame the graphics device refused left
the window blocking a second a present, for ever, and the mechanism was in the rendering library:
the swapchain texture was acquired *before* the frame budget was checked, and `Timeout` was the one
swapchain state that did not ask for a reconfigure. Written up in `doc/QUORRA_FEEDBACK.md` §7 and
**answered at `4aab7e2` in all three shapes it asked for**, including a `Device::invalidate_surface`
for a host that needs to say so itself. Re-measured by restoring this tree's own defect locally and
running the original report's recipe: a refused present costs **6 ms instead of 1.008 s**, nothing
reports `Timeout`, and the drag keeps updating. A page the device refuses for the other reason —
`bug1721218_reduced.pdf`, whose coverage outgrows a 16384 × 16384 scratch image — came back on the
processor in 1.68 s and the window zoomed, scrolled and opened its sidebar afterwards. That page's
rasterisation halved in the three-hundred-and-ninety-ninth (ADR 0236) and the 1.68 s has not been
re-taken; the refusal is quorra's and is unchanged.

### 4. Performance

**One fair comparison exists, and a second Rust renderer was tried and is not one.** Every other
renderer here is C; `hayro` is Rust, forbids unsafe, and rasterises on the CPU single-threaded as
we do. `rasterrocket` is Rust too and fails all three of the other conditions — 335 `unsafe` sites,
CUDA and Vulkan, and no way to start the clock inside its process. ADR 0136, and the summary is
below.

| | 139th | 125th | 119th | 106th | 99th | 73rd | 65th | 58th |
|---|---|---|---|---|---|---|---|---|
| total, ours | **6.91 s** / 863 complete pages | 6.04 / 865 | 7.08 / 864 | 6.99 / 862 | 7.08 / 859 | 6.91 / 858 | 6.20 / 852 | 7.13 / 852 |
| total, `hayro` | 47.89 s | 41.02 | 41.28 | 39.59 | 49.03 | 41.87 | 34.93 | 39.03 |
| **median page** | **2.15×** slower | 2.16 | 2.13 | 2.14 | 2.15 | 2.14 | 2.15 | 2.29 |

**Quote a total against a total taken the same afternoon.** Both totals rose ~15% between the
hundred-and-twenty-fifth session and the hundred-and-thirty-ninth — ours 6.04 → 6.91, `hayro`'s
41.02 → 47.89 — while the median ratio moved 2.16 → 2.15. Two independent programs do not slow
down together; the machine did. **The ratio is the measurement and the totals are the machine**,
which is also why the 6.04 s was never an improvement on the 7.08 s beside it. In aggregate we are 6.9× faster than `hayro` on the complete pages and 14× over every page both
render, because their distribution has a long tail and ours no longer does; the totals and the
median answer different questions and only quoting both is honest. We are faster on 103 of the
863.

**`rasterrocket` compared, in the hundred-and-fifty-fourth session, and the whole of it is ADR
0136.** It is an OCR front-end rather than a viewer, so most of what differs is two programs asked
different questions. Three things survive that:

- **It is slower, and its own shape says why.** Measured over 91 of a 98-document sample through
  `hayro-speed --per-document` joined to its CLI's timings, on the pages where its work clears its
  own 7.35 ms process floor: at 72 dpi **4.52× the median** and 859 ms against our 147; at 150 dpi
  **1.70×** and 1118 ms against our 462. The ratio halves because **our page-one time grows with
  the pixels and theirs does not move at all** — `tracemonkey.pdf` costs it 106.4 ms at 72 dpi and
  106.4 ms at 150, against our 18.6 and 34.6. A viewer is asked for the same page at many
  resolutions, so that is the axis the comparison actually measured.
- **It draws no path fills in this build and says nothing.** `alphatrans.pdf` loses three
  rectangles and a shading; a four-object hand-made document that `pdftoppm` marks 3267 sampled
  pixels of comes back blank. Exit 0 both times. Its golden-image harness has an empty case list,
  which is how 1330 passing tests coexist with that. **This is the strongest external evidence
  this project has that the corpus and the oracle are what make a correctness claim mean
  anything.**
- **It is not going in the oracle**, on four grounds in ADR 0136, three of which are traps already
  in this file: a reference that draws nothing votes for nothing (trap 9), its font module says it
  mirrors poppler's `getFTLoadFlags` exactly so on text it is a fourth vote from a reference we
  have (trap 9 again), and it is not in this repository so a gate on it would skip silently.

**What the comparison names for us is parallel rasterisation, and the first number is in.** Session
153 measured a dense text page spending four to six times as long being drawn as being read; their
fixed per-page cost against our resolution-proportional one says the same thing from outside.
`render-cpu` draws single-threaded, `Band` (ADR 0010) already has the geometry, and rayon is
already here. **The cost of the naive form is measured**: page 101's rasterisation re-measured at
**4 993 M** (session 153's 4 990 M reproduces), of which `CpuRasterizer::draw` is 4 104 M, and
inside it `render_cpu::convert::path` 405 M, `Rect::from_points` 218 M and
`RasterPipelineBlitter::new` 164 M are **787 M — 19% of the render — of per-command work that does
not shrink with the band**, and that a strip replay repeats once per strip a command touches.

**That composition is spent, and the six hundred rounds it took to say so is the lesson rather than
the numbers.** The *total* above has been re-taken in the hundred-and-sixty-second,
-third, -seventy-fifth, -eighty-fifth and -ninety-fifth sessions and again in ADR 0677's table; the
*breakdown* was last taken with it in the hundred-and-sixty-third. Re-run in the
seven-hundred-and-sixty-second (ADR 0687), two of the three items above are gone —
`convert::path` 405 M → 45 M, `RasterPipelineBlitter::new` 164 M → 24.5 M — `Rect::from_points` is
where it was at 227.9 M, and what had taken their place was **`pdf_render::strips::segments` at
373.7 M self and 448.3 M inclusive**, a function no document in this tree had ever named. A total
is what a later round re-takes and a breakdown is not, so **a composition decays faster than the
sum it adds up to**; `doc/habits.md`'s measuring section carries the rule. Retaking it is one
`valgrind --tool=callgrind` over `examples/callgrind_rasterise` and one `callgrind_annotate
--tree=both`, which is a minute — so the ranking above is never a thing to read when it can be
printed.

**And the counter that decides it is written and run: `examples/strip_spans`, ADR 0137.** At eight
strips a command touches **1.01 to 1.13** of them on four pages, so the 19% is multiplied by 1.13
and not by 8 — a **2.5%** penalty, and duplication is not the problem. **Imbalance is, and a prefix
sum removes it.** The worst strip's share of the estimated cost, against a 12.5% ideal:

| page | equal heights | equal cost |
|---|---|---|
| ISO 32000-2 p. 101 | 15.9% | **12.9%** |
| ISO 32000-2 p. 6 | 15.8% | **13.0%** |
| `tracemonkey.pdf` | 22.3% | **12.6%** |
| `bug1721218_reduced.pdf` | **72.0%** | **12.8%** |

Equal heights give the project's worst page a 1.4× ceiling on eight threads; equal cost gives
every page tested within 4% of perfect. **So strips are chosen by cost, and equal heights are not
a simpler first version — they are the version that fails on the page that most needs it.** The
mask worry is settled too: a clip chain touches 1.06 strips of eight on the 3608-chain page, and
the chains that span many are page-wide ones whose masks are band-tall anyway.

**It was then built, and the oracle refused it. ADR 0138.** The driver worked — strips borrowing
disjoint rows of the pixmap, a `MaskCache` each, rayon — and it changed the picture: **four pages
newly contradicted** (`bug1811694`, `dates`, `issue14705`, `issue15597`). The cause was isolated to
one line of geometry. A curve **clipped by a strip's edge is re-parameterised**, so the clipped
curve is not the `f32` control points the unclipped one was, and its edge coverage differs by up to
64 of 255. A probe pins it exactly: split a page where the shape lies wholly inside one piece and
the result is **bit-identical**; split it where the shape crosses and it is not. The same run with
the strip count forced to one — same planner, same skip test, same refactor — is clean at 836
agreeing pages, so **the strips are the cost and not anything else in the change**.

Everything but the planner was reverted, because `CLAUDE.md` forbids shipping a path nobody takes.

**The probe it named was run in the hundred-and-fifty-fifth session, and the parallel rasteriser
ships. ADR 0139.** The answer is a table rather than a yes — filling one shape into a pixmap and
into two pieces of it, an axis-aligned rectangle crossing the cut differs in **0** bytes, an
oblique straight edge in 292–528 (worst 32), a cubic in 2480–2744 (worst 64). So ADR 0138's
proposed rule was **too weak** (a clipped line keeps its geometry and loses its endpoints) and
**too strong** (page 6's page-wide clip is a rectangle, and rectangles survive).
`pdf_render::unsplittable_rows` marks the rows a re-stated segment spans; `strip_boundaries_avoiding`
minimises the worst strip among the rows left, by binary search on that maximum, because ADR
0137's prefix sum snapped to the nearest legal row gives 24.5% against a 12.5% ideal. **Every
oracle verdict, corpus count and text percentage is unchanged**, which is what says the strips are
exact, and `with_strips` plus `strip_parallelism.rs` is the standing guard.

| page, at the scale a laptop window asks for | serial | split | strips |
|---|---|---|---|
| ISO 32000-2 p. 6 at 2× (1192×1684) | 20.8 ms | **7.9 ms** | 16 |
| ISO 32000-2 p. 101 at 2× | 27.0 ms | **10.9 ms** | 16 |
| `tracemonkey.pdf` at 2× | 33.5 ms | **15.8 ms** | 4 |
| `bug1721218_reduced.pdf` | 105 ms | 105 ms | 1 — no legal cut |

The last row is a **session 155 measurement of a page that halved in the three-hundred-and-ninety-ninth** (ADR 0236); what it still says is the thing it was taken for, that this page grants no legal cut and is therefore drawn serially — which is why its own before/after is quoted as a counter and not as a clock.

**Two things nobody planned were most of the work, and both are traps one level up.** A serial
per-pixel pass bounds a parallel render: `impose_on_medium` was **7.8 ms of a 17 ms page**, all of
it eight integer divisions per transparent pixel, and §11.4.7's isolated page group makes most of a
page transparent — a `[0,0,0,0]` pixel is exactly the medium, so that case is a copy and the pass is
1.7 ms before it is split at all. And a planner on the drawing path is not a planner in an example:
`command_extents` rebuilt every command's clip chain from the leaf, **606 ms** on
`bug1721218_reduced.pdf`, six times that page's whole rasterisation, correct and unmeasured for two
sessions because only an example called it.

**The planner's *other* half was never priced at all, and it was half the critical path** (ADR
0687). `unsplittable_rows` walks every fill's, every clip's and every soft mask's path to name the
rows a cut may not fall on, and on page 101 of ISO 32000-2 that was 76 991 `mark` calls per render,
**448 M inclusive — 8.28% of the page** — with the whole planning prologue at 494 M, 9.13%. Nine of
those ten marks changed nothing: a dense text page is thousands of glyph fills over a few hundred
rows, and once a line of text has marked its own rows every later glyph on it re-marks them.
Skipping a path whose every reachable row is already forbidden — exact, because `Path::bounds` is
the control hull `oblique_spans` reports y ranges of — takes the page down **7.08%**, `tracemonkey`
5.81%, page 6 3.19%.

**Nine per cent of the total understated it, and the arithmetic that says so is worth keeping**:
the prologue is serial and everything under it is not. `examples/strip_spans` says page 101 is
granted eleven strips whose slowest holds 10.6% of the estimated cost, so of the ~258 M a render
costs, the drawing contributes about 233 M × 10.6% ≈ 24.7 M to the critical path — and the planner
was 24.7 M of it. **On this page the plan cost as much as the drawing did.** It is now 5.1 M.
A share of a *total* and a share of a *critical path* are different questions on any parallel path,
and this file had only ever asked the first.

**And ADR 0137's touch ratio was right about four pages and wrong as a property of pages.** The
oracle's first parallel run kept every verdict and went **37.0 s → 59.1 s**; five pages held most of
it and `issue12841_reduced.pdf` is their shape — **two commands, each covering the page**, replayed
sixteen times, 105 ms serial against 166 ms split. `pdf_render::replay_ratio` computes that number
per page and `plan_strips` refuses a division costing more than **1.25** of the list, after which the
oracle is **37.0 s, the serial figure exactly**, at +17% processor time. **This is the first change
in the tree where latency and throughput point in opposite directions**, and `CLAUDE.md` ranks them.

**Interpretation, by callgrind on `examples/callgrind_interpret`**: **2 137.7 M** in session 153, of which
the text layer is 35.8 M (session 133's A/B, below). The six sessions from the hundred-and-thirty-
third cost **140 097 instructions, +0.0065%** — selection, editing, saving and §12.5.6.19's `/H`
are all paths a page render does not take. Session
124 rebuilt `0723cda` in the same sitting and got **2 119 519 869** against the 2 119.5 M session
119 recorded for the same commit — a *repeat*, not a drift. **So the 0.42% "drift floor" this file
used to quote is drift between machines and builds; an A/B in one sitting resolves far below it.**
Four sessions of change cost 3.3 M (+0.16%), of which §11.7.4.4's per-glyph bookkeeping is 1.1 M
(0.05%), measured by stubbing exactly that out.

Per-feature interpretation costs, measured the same way and kept because they are the only honest
scale for "what a feature costs": text rendering modes +0.46%, composite fonts +0.44%,
constructed appearances +0.34%, variable text +0.31%, **the text layer +1.69%** (2 114.8 M →
2 150.7 M, an A/B in one sitting), masking +0.12%, soft masks +0.05%, §14.7's
parent tree **+4.5%** (object streams the drawing path never touches; 885 of 974 documents pay one
dictionary lookup), and §8.4/§8.5's path rules **−0.21%** — collapsing consecutive `m` operators
leaves fewer commands to build than the rules cost to apply.

**Where interpretation goes on the median page** (session 58, the specification's own page):
`zlib_rs::inflate` **28.0%**, `Interpreter::show_text` 6.5%, `Lexer::next_token` 5.1%,
`inflate_table` 4.0%, AGL name lookup 3.2% — the last was ours and avoidable, and a cache took
interpretation 2 013.8 M → 1 989.1 M.

**Re-taken in the seven-hundred-and-sixty-sixth, and the ranking is inverted** (ADR 0694). The
same instrument on the same page: `inflate` **3.11%**, `show_text` **13.83%**, `next_token`
7.69%, `inflate_table` 0.54%, the Adobe Glyph List **6.52%**. The largest item in the profile by
a factor of four is now the fifth, because ADRs 0317, 0365 and 0429 happened to §7.4; the
smallest had doubled, because nothing had happened to §9.10.2's list. **This is the same lesson
`callgrind_rasterise`'s composition taught four rounds earlier** (ADR 0687) and the pair is worth
keeping: the *total* was re-taken in ADR 0677 and this session reproduced it to nine digits
(1 278 428 629 against 1 278 427 485, a tree measured twice), while the breakdown beside it was
seven hundred rounds old. **A total is what a later round re-takes and a breakdown is not.**
Read the absolute figures, not the shares — interpretation of this page is 1 278.4 M against
session 195's 2 184.4 M, so every share above moved for two reasons at once (ADR 0370's rule).

What the retake found is two costs neither the profile's shares nor any document named, both
taken in the same session for **−7.52%** with the display list identical command for command:
§9.3.1's `Tf` resolved and **copied the font dictionary twice** before asking the cache that
answered 273 of its 280 statements on that page; and `agl_by_code`'s laziness was the *table*'s
rather than the *entry*'s, so the first character extracted from a font resolved all 256 of its
codes through the list where the page shows a few dozen — 67 200 `text_for` calls against the
8 850 the page asks for. ADR 0694 has both, and the arm where the second one's own extra cell
loses (+0.22%) is measured there rather than assumed.

**And the question 766 left — whether a font cache can outlive an interpretation — was taken in
the seven-hundred-and-seventieth** (ADR 0710). What repeats is the same font *object* across the
pages of one document — 62.4% of all font loads in the corpus's multi-page documents are re-loads
— so `viewer_core::Open` now holds a `pdf_model::FontCache` beside its document and passes it to
`interpret_with_fonts`; every other caller computes through a fresh cache per call and the answer
is unchanged by construction and by test. The instrument had to be built, because fifty
repetitions of one page *contain* the repetition a cross-page cache removes:
`examples/callgrind_pages` walks distinct pages with both arms in one binary. Twenty distinct
pages of ISO 32000-2 interpret at **−14.86%**, the page-101-fifty-times population `Open::stale`
actually re-interprets at **−31.32%**, and the keep-nothing arm the oracle and gates run in pays
**+0.536%** for the `Send` conversion that made a `LoadedFont` shareable. The budget is 2 MiB of
font program, least-recently-used, both halves derived in `FONT_BUDGET`'s own doc comment with
`examples/font_cache_budget` printing peak resident memory beside each row — and the reason it is
not 4 MiB is the accounting's honesty rather than the arithmetic: above 2 MiB the uncharged
tables beside the programs overtake the charge.

**The decompression item is priced and it is small** (session 128). Over one interpretation of
every corpus page: 6220 inflations of 38.08 MB; among the streams above 4 KB — 722 calls, 35.0 MB,
92% of the bytes — **35 are repeats costing 925 KB, 2.6%**, so a decoded-data cache is worth about
**0.7% of interpretation**, against a real memory cost, a bound to argue for and a liveness
invariant to write down. Below 4 KB the count is worthless: an address freed with one document is
handed to the next. **The benchmark page is not representative** — one 88 KB font program inflated
twice is 58% of *that* page's inflation and 2.6% of the corpus's. **Price an item on the corpus,
not on the page the profiler happens to open.**

**The worst page**, `bug1721218_reduced.pdf`: 144.05 G instructions → 54.05 G when a ramp stopped
carrying 256 stops for a linear function (ADR 0068) → 43.13 G when the built shading was cached per
object (ADR 0069) → **20.03 G** in the three-hundred-and-ninety-ninth session, when a rectangular
fill stopped being drawn wider than its mask can mark (ADR 0236). Twenty renders through
`examples/callgrind_rasterise`, A/B in one sitting: **38 453.3 M → 20 030.7 M**, the page's own PNG
byte-identical and the ink sum of twenty rasters equal to the digit.

**What that page is made of is what nobody had asked for a hundred sessions.** `pdfimages -list`
names no image; the content stream holds **3490 `sh` operators** against 479 `f`, each a whole-page
rectangle under a clip that admits about 24 pixels — §8.7.4.2's Table 76 bounds the operator by the
clipping path and by nothing else, and a rasteriser shades the *path's* spans and multiplies the
mask in afterwards. So 10.4 M pixels were shaded per render to keep 85 608, and
`tiny_skia::pipeline::lowp::gradient` went **15 783.8 M (41.05%) → 578.9 M (2.89%)**. The two
reference pages measure unchanged in the same sitting: ISO 32000-2 page 6 4 004.7 M → 4 007.7 M and
page 101 5 531.7 M → 5 523.1 M. It is not one document's shape: over the corpus's first pages, 49
carry a shading fill and **99.7% of their shaded pixels lie outside the clip**.

**What a *sampled* shading costs, which is a different page and a different lever** (session 529,
ADR 0364). A `ShadingType 1` is the one kind that cannot be reduced to a ramp: since ADR 0339 the
function is evaluated **once per device pixel the domain covers**, so a full-page one at a window
is hundreds of thousands of evaluations and a *page-turn-sized* cost hiding behind a single
display-list command. The project owner's `tmp/pi.pdf` is one — 2580 bytes of type 4 program over a
400×400 page — and it presented in **1079–1202 ms of which 1059–1175 was `scene`**, against
`mutool draw -r 96`'s 15–16 ms. Two things were wrong with it and neither was the grid:

- **A cell allocated three or four times.** The clipped inputs, the operand stack (`inputs.to_vec()`
  per evaluation), the outputs, the group's concatenation, and `copy`'s own `to_vec` — about a
  million heap round trips a frame. Writing into a caller's buffer, with the type 4 program running
  *on* that buffer because §7.10.5 leaves its outputs on the operand stack, is worth **6.1%** of
  `pi.pdf`, **11.9%** of `type4_pi.pdf` and **30.0%** of `function_based_shading.pdf` in
  instructions, and **420 instructions in a billion** on a page with no shading on it.
- **The grid was a serial loop.** Divided across rows, five renders of `pi.pdf` go **1.37 s → 0.19 s**
  and the owner's trace line **1059–1175 ms of `scene` → 105–284**.

**The threshold below which it is not divided is 4096 cells, and it was chosen against the clock
rather than with it.** The division wins in wall clock at every grid size measured, down to 400
cells — but at 400 it costs **5.6× the processor time** for that clock, and re-run on a loaded
machine the 4096-cell arm read 9.1 s serial against 11.9 s divided. A page may hold many small
shadings where the measurement holds one. **`rows_in_parallel` also declines on a rayon worker**,
which is `render-cpu` calling it from inside its own strips: dividing what is already divided asks
a busy pool for a job per strip.

**And the cost `render-cpu` still pays is per *strip*.** It rebuilds the shading's pattern inside
every strip, so a page in sixteen strips resolves the whole grid sixteen times. Invisible on the
path the owner reported — quorra builds the scene once, on one thread — and named in ADR 0364 for
whoever measures that backend next.

**Then session 572 asked why the work was being done at all, and most of it was not work** (ADR
0405). The owner reported that zooming that same `pi.pdf` drops frames; the attribution was a
`callgrind` profile rather than an inference — **94.09%** of the page in `Function::eval_into`,
15 240 instructions per device pixel — and the two obvious levers were eliminated by measurement:
the grid is already divided (781.3 ms serial against 93.5 on 24 threads at load 17, A/B in one
sitting) and the interpreter has no fat left after ADRs 0364 and 0371. What the *program* had was a
constant: the BBP series for π it evaluates per pixel is built entirely from literals.
`fold_constants` computes it once at compile time, and the consequence is not mainly a processor
saving — **9.6%**, 2.591 G instructions to 2.343 G, because the folded instructions are the cheap
ones — but that the program handed to the device no longer contains an operator WGSL 15.7.4.1 gives
an error budget, so quorra evaluates the shading per fragment under its unweakened agreement rule:

| frame | pixels | total | scene | bytes uploaded |
|---|---|---:|---:|---:|
| before, 1× | 400×400 | 24.6 → **19.8** | 21.1 → **0.2** | 640 032 → 32 |
| zoom, 2× | 800×800 | 96.1 → **3.5** | 93.5 → **0.1** | 2 560 032 → 32 |

`examples/zoom_frame`, real Radeon 890M, minima of five rounds, the *after* arm at the higher load
average of the two. **The lesson is the one about which number to quote**: 96.1 → 3.5 ms is a change
of path and 9.6% is the change to the path a still-refused page takes, and adding them together
would describe neither.

What is left, in order, on the halved page: `build_soft_mask` 17.1%, `Mask::intersect_path` 8.3%,
`fill_path_impl` 7.7%, `calloc` 4.5%, `gradient` 2.9%. **The two mask lines are one item**:
`MaskCache::get` is now **41.5%** of the page — 3554 chains built, no eviction and no duplication
worth removing (ADR 0103) — and is its largest cost. [todo 40](todo/40-mask-chain-crop.md) is that
item and the same round re-derived it: worth **42% of that function** rather than most of it
(3551 leaves through 7066 distinct nodes, so the sharing is 1.99 to a leaf), **not blocked on
memory** (the peak is 12.31 MB against `MASK_BUDGET`'s 32, where the file said 27.9), and resting
on a claim ADR 0219 refuted — that a mask value does not depend on which band holds its row.

**A page turn on the largest document was 380 ms and is 9 ms** (session 141, ADR 0124). §12.3.3's
`section_at` resolved every outline item's destination with `Pages::index_of`, which is a *search*
of the page tree — 988 items over 1023 pages, `O(items × pages)`, on the path of an arrow key.
`Pages::indices` gathers the whole map in one walk. Two more walks went with it: `Query::LinkAt`
6.05 ms → 52 µs and `Query::PageGeometry` 3.06 ms → 832 ns, both asked at pointer speed and both
`Pages::get` looking up the page already on the screen. **The gates cannot see any of this** — no
gate turns a page in a viewer and the specification's own PDF is in none of them — and the two
regression tests are *ratios* against a walk the test performs itself.

**The GPU backend's own question is open and has a plan** (ADR 0128, session 143), **and step 2
of that plan has been executed as a measurement and the answer is no** (ADR 0131, session 146).
Page 6 is 5933 fills of **107 distinct outlines**, and Vello re-flattens all 5933 every frame —
but the outlines are what is shared and the *coverage* is not. `examples/glyph_reuse` counts it:
an exactly-correct coverage cache hits **116 times out of 5933** on that page and **not once** on
`tracemonkey.pdf`, because a glyph's sub-pixel phase is an arbitrary float. The cache pays only
with the phase quantised, which is a positional departure, and **the oracle contradicts a page at
1/8 of a pixel and is clean at 1/16** — measured by applying it. At 1/16 the reuse is 5.0× on
page 6, 1.3× on `tracemonkey.pdf`, worth at most 39% and 11% of those pages' rasterisation before
the cost of a blitter `tiny-skia` does not provide. Refused, with the numbers.

**What the whole of that argument became is a document.** `doc/RENDER_LIBRARY.md` (session 165)
specifies the library this viewer would want from a team writing one: the input model, the API,
the clause-11 obligations an SVG-shaped model cannot express, the failure contract Vello breaks,
and a performance section built on the measurement `doc/gpu.txt` said nobody had taken.
**`examples/frame_split` is that measurement** — fastest of ten offscreen frames on the 890M —
and it reverses the obvious plan. Scene encoding is **1.1 to 1.6 ms and flat across a sixteenfold
range of pixels**, so a retained scene is worth 4% of a frame at 4× and 22% at a thumbnail's size.
What dominates is the per-pixel floor: the same viewport drawn from a list of **one rectangle**
costs 3.48 ms at 596×842, 8.77 at 1191×1684 and 26.73 at 2382×3368, which is **55% to 92% of the
frame before any of the page is drawn**, and most of it scales with bytes at about 1.2 GB/s —
consistent with the readback a windowed host would not pay. Page 6's 5 933 glyph fills cost 2.4 to
3.3 ms on top of that floor and **do not grow with resolution**.

**That changes the case for our own backend rather than removing it.** The atlas was ADR 0128's
headline and a GPU atlas quantises the same phase for the same reuse, so what it buys over Vello
is 1/16-pixel reuse and no more. The other four items — damage rendering, persistent geometry,
progressive rendering, clause 11 conformance — are untouched and are now the whole of the
argument. The rest of the plan stands: stale-frame zoom (perceived latency, host-side, judged
*ugly but acceptable for now* by the owner), a moving window of interpreted pages, then a spike
against Vello and `vello_hybrid`. A whole document cannot be resident: 70 MB of draw records is
affordable, the **4.0 s** to interpret 1023 pages is not, and the startup rule decides it.

**The item that profile found instead was taken the next session and it was the largest single
win this project has had on a text page** (ADR 0132). `calloc` was 18.3% of page 6's
rasterisation, *all* of it under `tiny_skia::Mask::new`, and `glyph_reuse` said why: **303
distinct `ClipId`s of one distinct region.** The producer wraps each of 303 text runs in `q … W n
… Q` with the same rectangle, and `add_clip` gave each an identifier of its own, so both backends
did the per-chain work 303 times. `DisplayList::add_clip` now returns an existing identifier for
an identical region — exact comparison, no tolerance. Rasterisation of the specification's pages
5, 6 and 101: **4.81×, 4.66× and 2.89×**. Interpretation pays +1.22% for hashing every clip. Every
oracle verdict, corpus count and text percentage is unchanged.

**And it dissolved ADR 0127's cliff on the page that motivated it.** Page 6 no longer overflows
Vello's buffers at 1.9008, or at 5.0 — the black page a person reported was, in substantial part,
this tree handing the device 303 layers for one region. The banding stays (Vello's constants are
fixed and another scene can still exceed them) and its witness is now synthetic:
`a_scene_too_large_for_one_pass_is_banded` gives each of page 6's fills a clip nudged so no two
are equal, which is what a producer with per-run rounding emits and what dedup cannot collapse.
**That test failed on its own guard rather than on a pixel**, which is why the change was noticed
at all.

**Rasterisation is now measured too, and on a text page it is the larger half.** Twenty renders
of the specification's own pages through `examples/callgrind_rasterise`: page 6 **3 601 M** and
page 101 **4 990 M** in session 153, against 16 771 M and 14 406 M before the clip deduplication of
session 147. Per page that is ~180 M and ~250 M of rasterisation against ~43 M of interpretation —
**a dense text page spends four to six times as long being drawn as being read**, which is not the
proportion this file's performance section has historically implied.

**Session 162 re-measured both after the strips landed, and the counter says what the clock does
not.** Interpretation is **2 139.4 M**, a repeat of 153's 2 137.7 M. Rasterisation was page 6
4 543 M and page 101 6 691 M — *up* 26% and 34% on the serial 3 598 M and 5 065 M measured in
the same sitting, against a wall clock that halved (ADR 0139's table). Callgrind counts every
thread, so a parallel render's instruction count is the serial one plus the replay and the
planning; the two numbers are measuring different things and both are true. **Quote the clock for
a parallel change and the counter for a serial one**, and say which — a session that reported
only the counter here would report a 34% regression on a change that made the page appear twice
as fast.

**And then the counter said where the overhead was, in one line of `callgrind_annotate
--tree=caller`.** `Path::bounds` was **17.6% of page 101's parallel render**, from 541 300 calls
over twenty renders — 3007 commands × 9 strips × 20 — because the strip driver asks every
command whether it misses the strip and the answer walked forty control points every time. A
`Path` now keeps its own untransformed hull in a `OnceLock` and maps it, which is **exact**
wherever the transform keeps the axes: `a·x + e` is monotone in `x`, so the same control point
attains the same extreme through the same arithmetic. A shear takes the walk.

| | serial | strips, before | strips, after |
|---|---|---|---|
| page 6 | 3 598 M | 4 543 M (+26%) | **4 035 M (+12%)** |
| page 101 | 5 065 M | 6 691 M (+32%) | **5 565 M (+10%)** |

Wall clock at a window's scale went with it: page 6 at 2× is **19.7 → 5.9 ms** and page 101
**33.7 → 10.1**, both 3.3× where ADR 0139 measured 2.6×. Interpretation pays **+0.8%**
(2 139.4 M → 2 156.9 M) for the branch and the `OnceLock`, which is the honest price of the
memo and is written here rather than left out of the comparison.

**Session 175 re-measured all three after ten sessions of change and none of them moved**:
interpretation **2 156.9 M**, the same figure to the digit; rasterisation of the specification's
page 6 **4 031 M** against 163's 4 035 M and page 101 **5 550 M** against 5 565 M. Nine sessions
of panels, extraction, trigger events and a parallel colour conversion cost the drawing path
nothing, which is what the numbers are for — and the one place they *did* move is priced beside
the change that moved it (ADR 0147).

**Session 195 re-measured after the ten from the hundred-and-eighty-sixth, and the total is
+0.41%.** Interpretation **2 184.4 M** against 185's 2 175.5 M; rasterisation of the
specification's page 6 **4 056.6 M** against 4 023.7 M and page 101 **5 566.4 M** against
5 566.0 M — the second is the same figure to four digits after two sessions that changed how
every fill is drawn, which is what says §10.7.4's rule and the tight stroke bound cost the
rasteriser nothing. **The interpretation figure was 2 211.1 M when it was first taken, and the
27 M between the two was one line.** ADR 0157's per-font tally used `BTreeMap::entry`, which
takes the resource name *by value* — an allocation per show string whether or not the font was
already in the map, and a page names three fonts and shows thousands of strings through them.
Stubbing the tally out measured 2 155.7 M, so the counter cost **2.2%**; hoisting it out of the
per-glyph loop and looking up before allocating gave back 27 M of it. **A counter is not free
where its key is a `String`**, and the way to find that out is to remove it and measure rather
than to read the profile.

**Session 185 re-measured after the ten from the hundred-and-seventy-sixth, and interpretation is
the one that moved: 2 156.9 M → 2 175.5 M, +0.86%.** Rasterisation did not — page 6 **4 023.7 M**
and page 101 **5 566.0 M**, both within the repeat noise of 175's figures — which is the right
shape, since nothing in those ten sessions touched the rasteriser. The 18.6 M is three things and
each is priced where it was spent: a type 1 shading's domain is hashed into a clip at every `sh`
and every shading-pattern fill (ADR 0150), a `Tf` now copies its resource name into the text state
so a report can say which font it means (ADR 0152), and every show operation tallies whether a
substituted face drew (ADR 0152). None is on a path a well-formed Latin page takes more than once
per operator, and all three buy a report or a mark that was missing.

**Colour-managing an image in parallel was taken in the hundred-and-seventy-first, and the item
named the wrong loop** (ADR 0147). `image::unpack` is the obvious target and is not the one a
JPEG takes: `zune-jpeg` writes components into the raster and `convert_channels` converts it in
place afterwards, which is where callgrind puts 27.6% of `issue19971.pdf` plus the 26.2% of
`libm` under it. Parallelising `unpack` measured as noise and was reverted; parallelising
`convert_three` **halves the page**, 110 ms → 57 ms of interpretation, at 1 085 M → 1 365 M
instructions. Eight bands rather than one per core: same clock, two thirds of the extra processor
time, because each band allocates a `Conversion` table and one sized to a twenty-fourth of the
image collides no less than one sized to an eighth. The split is exact because the memo is a memo
of a *pure function of one pixel's samples* — which is precisely what ADR 0138's strips were not.

**Where a page turn goes, measured for the first time in the three-hundred-and-ninetieth session
and acted on in the three-hundred-and-ninety-first** (ADRs 0227, 0228). The witness is the project
owner's own `NorthAmerican.30MB.pdf` — 65 pages, 30 MB, a scanned document — driven through 38 page
turns under `Xvfb` on `llvmpipe`, which is **not** the owner's Intel UHD through DX12, so the ratios
are shape and the numbers are this machine's. Sums over 39 frames, three runs of each:

| | before | after |
|---|---|---|
| frame | 1203.4 / 1242.3 / 1225.7 | **1071.6 / 1097.7 / 1074.0** |
| scene | 208.4 / 219.2 / 210.1 | **71.4 / 71.4 / 68.3** |
| device | 994.3 / 1022.4 / 1014.9 | 999.5 / 1025.7 / 1005.0 |
| attend | 92.8 / 94.1 / 94.3 | **10.3 / 12.4 / 10.7** |

**Two findings, and both had the wrong suspect.** The display-list-to-scene walk was bimodal —
a 388-command page cost sixteen times a 3675-command one — and the whole of it was
`Image::area_averaged`, which is paid **per source sample** and is therefore invisible to every
instrument that counts commands. The column bands are computed once per image rather than once per
output cell and the output rows are divided across rayon above a measured floor of 65 536 source
samples: a 2700×3450 image reduced threefold goes **22.4 ms → 2.9**, byte-identical
(`pdf-render/examples/area_bench` asserts it on every size it times). And the accessibility
publication's 2 ms was not §14.7's tree at all — that query is 0.13 to 0.25 ms — but
`App::place_window`, two synchronous X11 round trips for a window position a page turn cannot
change; it is asked at bridge-up, `Moved` and `Resized` now.

**And what a page turn cannot see is a *redraw*, which is the four-hundred-and-sixty-second
session's** (ADR 0297). A page turn draws each page once, so the reduction above was paid once a page
and looked free; a scroll draws the same page again. Same document under `Xvfb` at 1200×1500, an
800×1000 window, two `+` to put the page past the window and then twenty `Down` — twenty redraws of
one page at one scale — with the two release binaries built from the same tree and run alternately,
three runs an arm:

| over 23 frames | before | after |
|---|---|---|
| median frame | 15.2 / 15.0 / 15.0 ms | **4.7 / 4.8 / 4.8 ms** |
| median scene | 8.9 / 8.9 / 8.9 ms | **0.0 / 0.0 / 0.0 ms** |
| sum, scene | 197.9 / 197.3 / 203.6 ms | **16.9 / 15.7 / 16.3 ms** |
| sum, frame | 359.6 / 358.4 / 369.3 ms | **155.3 / 159.4 / 158.9 ms** |
| resource uploads | 23 | **2** |

A scratch build of two `Instant`s attributed the 8.9 ms exactly: `area_averaged` on the page's one
2700×3450 photograph, 8.5 to 9.8 ms, against an `upload_image` of **0.002** and a `transfer` median
of 0.8 — the cost is one pass over the *source* samples on the host and nothing else. It is kept
now, in `render-quorra`'s resource cache, keyed by the source's `Arc` identity **and the reduction
factors**, which `pdf_render::Image::reduction` answers without producing the raster. Two uploads
rather than twenty-three is the reduction being produced once per magnification instead of once per
frame. Every gate is unmoved and the 4× coverage lane is byte-identical, refusal for refusal — which
is the run that would show a retained raster crowding the budget, and it shows nothing. **What the
same run says about a claim in this file**: there is one resource refusal at 4× today
(`22060_A1_01_Plans.pdf`), where section 3b records zero; it predates this change by both arms of the
A/B, and it is what a ratcheted count looks like when nothing is ratcheting it. (That one is gone
since the five-hundred-and-thirty-ninth, on the interpreter's side of the boundary — ADR 0374, and
section 3b carries the argument.)
**Something is ratcheting it since the four-hundred-and-seventy-eighth**:
`corpus.rs::REFUSED_BY_THE_DEVICE_AT_FOUR`
holds that lane's refusals to equality by name, which the sentence above is the argument for.

**What is left is quorra's and is reported rather than changed**: `encode` is 45% of a page turn,
is host processor time, and is the only phase of `Device::render` that tracks the scene's size —
**3.86 µs a command plus 3.84 ms** by least squares over 38 frames spanning 388 to 3675 commands.
`doc/QUORRA_FEEDBACK.md` §13 asks for a subdivision of it, an instrument before an optimisation.
The same section retracts something this side printed: the trace's `elsewhere` row subtracts a
timestamp-query duration from a host wall clock, so it is a bound rather than a duration, and the
summary now says so.

**Still open, each priced and each with a file** — the first of the three this list used to
carry, an image *and its sampling intent* to the backends, closed across ADRs 0210, 0321 and
0339 and its file is gone: a clip chain as one crop and one intersect on the
corpus's worst page ([todo 40](todo/40-mask-chain-crop.md)), which the
three-hundred-and-ninety-ninth session unblocked and re-priced rather than took — and whose
*copying* half the four-hundred-and-ninety-third then took byte-identically (ADR 0328), leaving
the chain itself; and a
decoded-stream cache, whose 0.7% was a corpus walked one page a document and which the
population a *reader* is in has since paid for twice over — the bytes in ADR 0317 and the refusals
in ADR 0437 ([todo 41](todo/41-decoded-stream-cache.md)).

Two fixes worth carrying as patterns: unpacking JPEG output cost 6.89 G until two paired
`chunks_exact` iterators took it to 1.25 G — **the safety habits this project enforces everywhere
are expensive in a loop that runs per pixel** — and a mesh's triangles stopping at the device
pixel took `personwithdog.pdf` from 17.3 s to 1.06 s *while* moving every mesh page closer to the
references. (That rule was `Triangle::is_subpixel`, a bound on a *subdivision* that
`pdf_render::MeshRaster` replaced in the forty-third session; the method survived it as dead code
until ADR 0292 removed it, and what it bought is why the sentence stays.)
