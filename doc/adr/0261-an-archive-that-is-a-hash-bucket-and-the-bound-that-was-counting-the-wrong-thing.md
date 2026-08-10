# ADR 0261 — An archive that is a hash bucket, and a bound that was counting the wrong thing

Date: 2026-08-10 (session 425)
Status: accepted

## Context

ADR 0258 built `tools/safedocs` under a stated constraint — a mobile connection, short tests
only — and the project owner has since lifted it: **up to 50 GB may be downloaded**, with
1.1 TB free on this machine. `doc/todo/03` recorded what replaces the constraint, and it was a
*strategy*: spend the budget **stratified** rather than deep, because "[f]iles inside one
archive are correlated — they come from one neighbourhood of one crawl — so a gigabyte spread
across fifty archives is worth more defects than a gigabyte taken from one."

This session took the first stratified slice. Two things came out of it: the strategy's premise
is false and the reason is checkable, and the sample found a defect and a crasher in one clause.

## Decision 1 — the sampling rule, stated so that it can be repeated

**Archive `50 + 100k` for k = 0 … 78 — `0050`, `0150`, … `7850` — and the first 24 members of
each.** Three properties, each the reason for a part of it:

- **A fixed stride across the whole range**, so the sample is a *rule* rather than a choice, and
  a later round can extend it by changing the offset rather than by re-deciding.
- **Offset by 50**, which misses the two archives ADR 0258 and session 423 already took (`0000`
  and `3500`) and misses every thousand-boundary, where the corpus changes directory.
- **The head of each archive**, because `--from 0` needs no extra directory read and — for the
  reason decision 2 gives — a window anywhere in an archive is the same sample as a window
  anywhere else.

**What it cost: 2731.0 MiB of member byte ranges and 14.1 MiB of central directories, 2.68 GiB
in total, for 1896 documents.** Of the 50 GB that is **5.4%**. 79 fetches, 0 failures, every
member verified against the CRC-32 its archive records. Nothing was committed.

## Decision 2 — the archives are hash buckets, and `doc/todo/03`'s premise is withdrawn

The corpus's `README.md` says its file numbering derives from each file's SHA-256, and ADR 0258
noticed the first two digits matching in one archive. The stratified sample makes the stronger
statement checkable over the whole range, and it holds:

> For every one of the 1944 members now in the cache, the file's own number — its rank among all
> 7 932 878 — and its SHA-256 read as a fraction of 2²⁵⁶ agree to within **2.6 × 10⁻⁴**.

That is not "roughly sorted". 2.6 × 10⁻⁴ is the size of the fluctuation the *k*-th order
statistic of 7 932 878 uniform draws has by construction — √(p(1−p)/N) is 1.8 × 10⁻⁴ at the
midpoint — so the residual is sampling noise and nothing else. The corpus is the whole crawl
sorted by content digest and cut into 7933 equal pieces.

**So an archive is a hash bucket rather than a crawl neighbourhood**, and every consequence
`doc/todo/03` drew from the opposite belief goes with it:

- A gigabyte from one archive and a gigabyte spread over fifty are **the same sample**. Nothing
  about a document — its producer, its site, its language, its age — can correlate with its
  SHA-256.
- `--from M` therefore addresses an unbiased window of the *whole* corpus, not of one crawl
  neighbourhood, which makes going deep cheap: one archive fetched whole is 1000 documents for
  one directory read instead of forty.
- The stratification this round paid for was **14.1 MiB of central directories**, and it bought
  the disproof rather than the spread. That is the right trade for the first slice and it is not
  the right trade for the second.

The rule that replaces it is in `doc/todo/03`: sample by *manifest*, not by archive — take what
nobody has taken, in whatever shape is cheapest, because the population does not care.

## Decision 3 — §7.10.4's k is not a component count, and the bound now says which it is

`crates/pdf-model/src/function.rs` had one constant, `MAX_VALUES = 64`, documented as bounding
"what a single evaluation can allocate" and justified by "colour spaces top out at a handful of
components". It was applied to five arrays that scale with a function's *dimensionality* — and
to two that do not.

§7.10.4's Table 41 makes `/Functions` "( Required ) An array of k , 1-input functions that shall
make up the stitching function", `/Bounds` "An array of 𝑘 - 1 numbers" and `/Encode` "An array of
2 × 𝑘 numbers", and bounds k **nowhere**; the only value the clause singles out is the small one,
"The value of k may be 1". The same subclause settles the other quantity outright: for a type 3
function "Domain shall be of size 2 (that is, 𝑚  =  1 )". So one of the two numbers is fixed at
one by the standard and the other is free, and a single constant could not be about both.

A 256-stop gradient is written as k = 255. **`2750009.pdf` in this round's sample — archive
`2750`, SHA-256 `58b30fccc67721da1d51cec8c5572c15b577fd909b022c63ea04b66492791e6a` — is four
shadings of exactly that shape, and all four were refused whole.** A second consequence sat
behind the first and was worse, because it was silent: `/Encode` is read through the same array
reader, whose ceiling was 256 numbers, so a *present* 510-number `/Encode` was dropped and
replaced by the identity. On this witness the identity happens to be what the file states; on the
next one it would not be, and nothing would have said so.

**The fix is two bounds where there was one, and each is named at the call site.** `numbers` and
`pairs` take the limit as an argument, so `/Domain`, `/Range`, `/C0`, `/C1`, `/Size` and
`/Decode` are bounded by `MAX_VALUES` and §7.10.4's `/Bounds` and `/Encode` by `MAX_FUNCTIONS`.
Reading one array with the other constant is now a thing somebody would have to type.

## Decision 4 — the bound on k is a budget over the tree, because breadth alone is not one

`MAX_FUNCTIONS = 4096`, and it is spent by *every* function one call into the parser builds
rather than checked per array. The reason is arithmetic: a subfunction may itself be a stitching
function, so a per-array bound of b and a depth bound of d leaves bᵈ reachable, and 4096⁸ is not
a bound. A `Function` is 120 bytes on this target — asserted by the test rather than recalled —
so the ceiling is 480 KiB of functions for one shading, and the witness's k = 255 sits sixteen
times below it.

**And a `/Functions` array naming its own object was a stack overflow.** Nothing in ISO 32000-2
forbids it: a subfunction may be a stitching function, and §7.3.10 makes a reference something a
reader follows. A **720-byte** document doing it aborted every program in this tree —

```
thread 'main' has overflowed its stack
fatal runtime error: stack overflow, aborting
```

— and not only in a test harness: `target/pdf-retrieve`, the release binary session 424 installed
for a person to run, dies the same way on `pdf-retrieve page <file> 0`. Anything that interprets a
page in process is reachable, which is `pdf-viewer`, both native hosts and the C ABI.
`MAX_STITCH_DEPTH = 8` is the guard, and it is *the same constant*
`Function::breakpoints` walks to, so the two cannot disagree; that comment claimed "`parse`
already bounds the nesting it will build" and had been wrong since it was written.

`CLAUDE.md` requires a crasher to become a permanent regression test, and
`crates/pdf-model/tests/hostile_functions.rs` is it — six tests, the fixtures **generated**
rather than committed, so the promotion budget is untouched.

**Thirteen fuzz targets and not one of them could have found it.** Only two reach
`pdf_model::interpret`: `confined_wire`, whose input is a *process* rather than a document, and
`variable_text`, which varies a widget's `/DA` and `/V` inside a page it holds fixed — so no
fuzzer in this tree has ever constructed a shading, let alone a function that names itself. A
corpus of real files found it in one round. That gap is written down in `doc/todo/03` rather than
closed here, because `cargo-fuzz` is not installed on this machine and a target nobody has run is
not a target.

## What the sample said about the population, which is most of what it said

**1896 documents in 42.1 s: 4 unopenable, 1 locked, 0 encrypted beyond us, 3 pageless, 86
incomplete, 0 slow** — 85 after decision 3 — with 862 codes reaching no glyph in silence over 12
documents. A baseline for this chunk, never a ratchet.

**Nothing failed to open for a reason that is this tree's.** All seven unusable documents are
artefacts of the crawl, opened and read by hand: four are HTML — Cloudflare interstitials and a
`<!DOCTYPE html>` error page saved under a `.pdf` name — and three are PDFs truncated by the
origin server with the same Baidu link-submission script appended where the body should be. Two
of those three say `/Linearized 1`, so their first-page objects are at the front of the file and
would have been recoverable; the truncation is at about a kilobyte and the first page's xref
points past 90 000, so there is nothing there.

**The failure modes are, overwhelmingly, ones this tree has already named.** Of the 86 reports:

| what | documents | already named by |
|---|---|---|
| §11.4.7's page-group blending space | **67** (59 `/DeviceCMYK`, 6 array-formed, 2 with §11.4.4 beside them) | `doc/todo/23`, ADR 0251 |
| a font whose program has no outline for any code the page shows | 7 | `doc/todo/21` §3 |
| §11.4.4's non-isolated group with an element that blends | 4 | `doc/todo/23`, ADR 0234 |
| everything else | 8 | one apiece |

The first row is the round's number: **3.5% of the web's PDFs composite their page in a space
this renderer does not composite in**, against 0.7% of the 974 pdf.js documents. `doc/todo/23`
prices that at a four-component raster and ADR 0251 argues it; what this sample adds is that it is
not a corner — it is the single largest correctness gap this tree has against real files, by a
factor of six over everything else put together. The six array-formed ones were checked with
`group_space_census` and are 4-component ICCBased page groups, so they are the same population
and not a second one.

The eight singletons are worth their names, because a population of one is where the next round
looks: `MAX_OPERATIONS` and `MAX_TILES` each reached once, a JBIG2 with too many symbol
instances, a `/DCTDecode` stream whose first two bytes are `89 50` — a PNG — a JPEG the file
declares `/DeviceCMYK` and whose data is RGB, a `CCITTFaxDecode` that produced 4204 of the 5196
rows the dictionary states, an `/SMask` with a `/Matte`, and a `FreeText` annotation with no
appearance stream.

**One of the eight was diagnosed and deliberately not taken.** `4150022.pdf`'s page one has a
six-part `/Contents` whose fourth object is `<< /Filter /FlateDecode /Length 0 >>` — a stream of
zero bytes naming a filter — and `flate` refuses an empty input, so the part is reported
`Undecodable`. The report claims the page is missing drawing and an empty part cannot be; but
"decode zero bytes to zero bytes" also silences a stream that was truncated to nothing, and
choosing between those two is an argument this round did not have room to make properly.
`doc/todo/03` carries it with the diagnosis.

## Consequences

- The 50 GB budget is **5.4% spent** and the promotion budget is **0 MB spent**, unchanged: the
  crasher is generated in a test and the k = 255 witness is named by archive and digest.
- `doc/todo/03`'s stratification rule is replaced by a manifest rule, and future rounds may go
  deep without paying for spread.
- §7.10.4's ledger row keeps `implemented` and gains the reading; no other row moved.
- Every gate reproduced except the three this round's own work moved: tests 1544 → **1550**,
  citations 6223 → **6240**, distinct tables cited 217 → **218**. The 974 corpus's 70 incomplete,
  the oracle's 1794 pages with 905/68/786 and the undiagnosed list empty, quorra's 912/36/9/17,
  both text gates, the dates, the XMP and the JPEG 2000 lines are unmoved, and the ledger is 875
  rows with the same six counts.
