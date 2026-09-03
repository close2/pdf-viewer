# More corpora, and the one that cannot be a submodule

Status: **standing**, and it became standing rather than one-off in the four-hundred-and-twenty-second
session, which built the three pieces `doc/test-docs.md` asked for. What is left is the *taking*:
a chunk a round, the way `doc/todo/00` takes a page off the ambiguous ranking.

> **The SafeDocs crawl is finished.** All **65 944** documents of `CC-MAIN-2021-31` are ranked
> against `pdftoppm`, `mutool` and `gs`, over nine chunks from the six-hundred-and-third session to
> the six-hundred-and-forty-seventh; §27 took the last 3944 and says what the whole population
> turns out to be. **No later round need re-derive that**, and a round looking for a chunk should
> take a *different* corpus rather than more of this one — §1's note about which kind is worth a
> round is the argument.
>
> **The issue-tracker set is no longer "nobody has fetched it", and it is no longer at the address
> five sections of this file sent a round to.** §29 has where it went, the route that still works,
> what it holds and what the first chunk found. Every sentence below that calls it "SafeDocs' 31 GB
> issue-tracker corpus, six archives" is a standing offer that has been taken; the size and the
> count are right and the *host* is gone.
Priority: 03 — the standing band, deliberately.
Corpus: 974 pdf.js documents **plus 275 in four submodules** — of which
`pdfCabinetOfHorrors` and `govdocs1-error-pdfs` were taken as a chunk in the
five-hundred-and-fifth, §8 below — and whatever `tools/safedocs` has
been asked for — **65 944 as of the four-hundred-and-thirty-third**, 145 archives, 93 GB, the
manifest below. The fourth submodule is three directories of `openpreserve/format-corpus`, taken in
the four-hundred-and-seventieth (`doc/oracle-and-corpus.md` §2b, §2c; ADR 0305)
Code: `tools/safedocs`, `doc/corpora/*`, `doc/oracle-and-corpus.md` §2

## What exists now (ADR 0258)

- **`doc/corpora/pdf20examples`** — 7 files, CC BY-SA 4.0. The *coverage* denominator.
- **`doc/corpora/pdf-differences`** — 37 files, Apache-2.0. The oracle's food.
- **`doc/corpora/pdfbox`** — 64 files, Apache-2.0, a **partial sparse** checkout of
  `pdfbox/src/test/resources/input`. The recipe is in `doc/oracle-and-corpus.md` §2 because
  `.gitmodules` cannot hold it.
- **`doc/corpora/format-corpus`** — 167 files in three directories, partial sparse again, added in
  the four-hundred-and-seventieth on the owner's rule that a corpus is added unless its licence
  clearly forbids it (ADR 0305). `pdf-handbuilt-test-corpus` is the instrument; the other two are
  populations. `doc/third-party-data.md` has each directory's terms and the two that were left.
- **`tools/safedocs`** — `corpora`, `plan`, `fetch --download`, `list`, `survey [--dir …]`. It
  reads a chunk out of a gigabyte archive through the archive's own central directory and one
  contiguous byte range, refuses a plan over 32 MiB **while naming the `--budget-mb` that would
  admit it**, and verifies every member against the CRC-32 the archive records. **The bound is on
  accident, not on the person**: the budget has no ceiling and `--all` takes every member, so
  `safedocs fetch --archive 0042 --all --budget-mb 2048 --download` is the whole archive and is
  the right command on an unmetered connection.

**None of it was in `doc/todo/02` §2's sequence and one thing now is**, having earned the place
rather than taken it: the `pdfbox` comparison in item 4, at **0.4 s** and inside a line that
already ran every ignored test of that binary. `safedocs survey` is still a command a round runs
on purpose, and a *corpus* still earns its place before it takes one.

## The data budget, stated by the owner on 2026-08-10

**Up to 50 GB may be downloaded.** The owner checked their plan and said so outright, which
replaces the "on mobile, short tests only" constraint the four-hundred-and-twenty-second session
worked under. Disk is not the limit either: 1.1 TB free on this machine.

**That makes *how* to spend it the question rather than *whether*.** One fact decides it, and it
is **not** the one this section carried until the four-hundred-and-twenty-fifth session.

**This file used to say that "[f]iles inside one archive are correlated — they come from one
neighbourhood of one crawl", and prescribed a stratified sample on the strength of it. That is
false, and the four-hundred-and-twenty-fifth session's stratified sample is what disproved it**
(ADR 0261). The corpus is the whole crawl **sorted by SHA-256** and cut into 7933 equal pieces: for
every one of the 1944 members now cached, the file's own number — its rank among all 7 932 878 —
and its digest read as a fraction of 2²⁵⁶ agree to within **2.6 × 10⁻⁴**, which is the fluctuation
7 932 878 uniform order statistics have by construction and nothing more. Nothing about a document
can correlate with its content digest, so:

- **An archive is a hash bucket**, and a gigabyte from one archive is the same sample as a gigabyte
  spread over fifty. Going deep costs nothing in representativeness.
- **`--from M` addresses an unbiased window of the whole corpus**, not of one crawl neighbourhood.
- **Spread costs 182 KiB of central directory per archive.** The 79-archive slice paid 14.1 MiB of
  it, which bought the disproof and would buy nothing a second time.

So the plan a round should follow is **whatever is cheapest among what nobody has taken**:
`corpus-cache/safedocs/manifest.tsv` is the record of where that is, and one archive fetched whole
is 1000 documents for one directory read where forty windows cost forty. **SafeDocs' issue-tracker
corpus is 31 GB in six archives** and fits inside the budget with room to spare.

**What the budget does not change**: the promotion rule (20 MB of committed witnesses, then names
and digests), that these files are crawled web pages under no grant anybody made
(`doc/third-party-data.md`), and that a new corpus stays out of `doc/todo/02` §2's default sequence
until it has earned a place.

## What is left, in the order its value is clearest

### 1. Take a chunk a round

The instrument works and every chunk so far has produced a finding. A round's chunk should be
**somewhere nobody has been**, and `corpus-cache/safedocs/manifest.tsv` is the record of where
that is.

**Taken so far: 145 archives and 65 944 documents, 93 GB on disk.** The four-hundred-and-thirty-third
session surveyed **all of it** and this section's job has changed with it: the fetching rule below is
spent, because there is no longer anywhere nobody has been in this corpus. What a round takes next is
either a *different* corpus (SafeDocs' issue-tracker set, 31 GB in six archives) or a *population*
out of the one on disk.

**The four-hundred-and-sixty-seventh took a different corpus rather than more of this one, and the
lesson is about which kind is worth a round.** `openpreserve/format-corpus`'s
`pdf-handbuilt-test-corpus` is 89 files that each carry **one** deliberate structural defect and all
draw the same *Hello PDF-world!*, so a blank render *is* the finding and no reference is needed to
say so — 0.1 MB of files against 93 GB of crawl. **It produced three defects over three consecutive
rounds** (ADRs 0302, 0303, 0305), and the third is the sharpest statement of what such a corpus is
for: a census of the 65 703 crawled documents that open finds **not one** with the construct that
defect was in. **A corpus built to be diagnostic outranks a corpus built to be large**, when what a
round wants is a defect rather than a rate. `doc/oracle-and-corpus.md` §2b has the survey and the
one-line ink assertion that reads it, and §7 below says why the instrument is now spent.

**`manifest.tsv` holds 65 968 rows and the cache holds 65 944 documents**: archive `0050` was fetched
twice, once by session 425's stride and once whole, so its 24 members are recorded twice. Every count
here is over the distinct files.

The record of how it was taken, for reproducibility:
`0000` and `3500` in the four-hundred-and-twenty-second and -third, 24 members apiece; then in
the four-hundred-and-twenty-fifth, **`50 + 100k` for k = 0 … 78** — `0050`, `0150`, … `7850` —
24 members from the head of each, 2731.0 MiB of member ranges and 14.1 MiB of central
directories, 79 fetches, 0 failures, every CRC-32 matched. **That stride is spent.**

**And in the four-hundred-and-thirtieth, `0100 + 2000k` for k = 0 … 3** — `0100`, `2100`, `4100`,
`6100`, **every member of each**, which is 4000 documents for **four** directory reads where the
stride paid 79 for 1896 (ADR 0266). 5409.6 MiB of member ranges and 728.7 KiB of directories,
5.04 GiB in 438 s, 4 fetches, 0 failures, every CRC-32 matched. The offsets are ≡ 100 (mod 1000),
which is disjoint from the stride and off the thousand-boundaries where the corpus changes
directory. *Which* archives is immaterial and that was the point: an archive is a hash bucket, so
the rule existed to stop two rounds fetching one archive twice and for no other reason. **The
four-hundred-and-thirty-third then took the remaining 60 whole archives** — the corpus on disk is
145 of the 7933 — **and the rule is spent with it**: there is nothing left to schedule inside
`CC-MAIN-2021-31` that a round would want before it wants a different corpus.

What a chunk owes when it is taken:

- the survey's line, recorded — it is a baseline for that chunk, never a ratchet;
- every report read against the population it belongs to, because a second witness for a
  population `doc/todo/21` or `23` already names is worth nothing;
- a **promotion** only where the problem is new and is named in the commit. The budget below.

**The whole population, surveyed in the four-hundred-and-thirty-third, and it is a baseline rather
than a ratchet** (ADR 0269): *65 944 documents in 1139.3 s: 173 unopenable, 45 locked, 23 encrypted
beyond us, 52 pageless, **1144 incomplete**, 2 slow*, with 51 272 codes reaching no glyph in silence
over 635 documents. **1148 s of wall clock**, 145 archives, one process each, 0 failures.

**One process per archive is the method and it is not an implementation detail.** `render-cpu`
rasterises under `[profile.release]`'s `panic = "abort"`, and the survey runs documents through one
rayon `par_iter`, so one document's abort takes every other verdict in the process. Five of the 145
archives produced no report at all on the first pass — two aborted and three sat at the driver's
600 s timeout — and **both of this round's defects are those five**. Surveying 65 944 documents in
one process would have produced one traceback and no numbers.

**And every process goes through `tools/bounded.sh`, at most four side by side, since the
eight-hundred-and-sixty-sixth.** Eight shards of one directory, each with a rayon pool of one thread
per core, is what took the machine into a soft lockup on 2026-09-01 (ADR 0798): a shard's cost is
not the documents it walks but the documents in flight, one fuzzed page can cost 11 GB on its own,
and eight pools of 24 threads had 192 in flight. The wrapper divides the walk's 32 GiB and the
machine's cores between the shards — `tools/bounded.sh --shards 4 -- <build>/safedocs survey --dir
<shard>` — and its last line says what the shard cost or that the bound stopped it, which is a
different report from the traceback above and must be read as one: a shard stopped by the bound
has surveyed nothing, and is re-run with more shards rather than recorded.

**The three-way rate comparison, which is what a population this size is for:**

| sample | documents | incomplete | rate |
|---|---|---|---|
| session 425, 79 archives × 24 | 1896 | 86 | 4.54% |
| session 430, 4 whole archives | 4000 | 70 | 1.75% |
| **session 433, all 145** | **65 944** | **1144** | **1.735%** |
| the pdf.js gate | 974 | 68 | **6.98%** |

The first number moved because sessions 426 and 427 built §11.4.7's conversion. The second and third
differ by 0.015 points over a sixteen-fold increase in sample size, so **1.7% is a fact about the web**
rather than about a sample — and the 974 being four times that is what a corpus assembled from bug
reports is *for*.

**The residue over 65 944, ranked** — a document reporting two things is in two rows:

| population | documents | of 65 944 | named by |
|---|---|---|---|
| a group's blending colour space (§11.4.7, §11.6.6) | **398** | 0.60% | `doc/todo/23` |
| a font with no outline for any code the page shows | **261** | 0.40% | `doc/todo/21` §3 |
| an image | **152** | 0.23% | five things, ADR 0266 |
| §11.4.4's non-isolated group | **129** | 0.20% | `doc/todo/23` |
| **a budget stopped interpretation** | **84** | 0.13% | `doc/todo/49` |
| text-showing operators skipped | 60 | 0.09% | with the font rows |
| a font program that would not parse | 35 | 0.05% | — |
| a `/Contents` part that would not decode | 32 | 0.05% | — |
| a `/Font` or other resource the file never defines (§7.8.3) | 25 | 0.04% | ADR 0255 |
| §11.6.4.3's knockout group | 23 | 0.03% | `doc/todo/23` |
| an operator / a shading / an annotation | 22 / 20 / 11 | — | — |
| §11.6.2's object in parts / §9.3.8's text knockout | 9 / 4 | — | — |

**The first row's five conditions, read off the reports** — this is what `doc/todo/23` prices one row
apiece, with a real number at last:

| condition | documents |
|---|---|
| the document names the press its `DeviceCMYK` is (§8.6.5.6, §14.11.5) | **151** |
| the page group's four components are not `/DeviceCMYK` | **106** |
| a group inside the page composites in a different space (§11.6.6) | **78** |
| a group inside the page *introduces* the space (§11.6.6, not the page group) | **30** |
| a non-separable blend mode (§11.3.5.3) | **27** |
| an `/ExtGState` states Table 57's `/BG` or `/UCR` (§11.7.5.3) | **7** |

Four things to take away:

- **The budgets are 0.127% of the web** — `MAX_TILES` 48, `MAX_OPERATIONS` 31, `MAX_FORM_DEPTH` 4,
  `MAX_STATE_DEPTH` 1 — against 0.2% of 4000 and 0.105% of 1896, so the rate is stable at three
  sizes, and **none of the 84 is one of the two slow documents**. That is session 430's finding
  confirmed with ten times the evidence: the bound stops the work *inside* the per-document budget.
  `doc/todo/49` keeps the constants where they are. **All 83 were opened in the
  four-hundred-and-thirty-fifth** (ADR 0271) — 84 refusals over 83 documents, because `7680183.pdf`
  reports two — with each bound lifted in a scratch build, one process apiece. None of the four
  moved, and the reason differs for each: `MAX_FORM_DEPTH`'s four documents are **all cycles**,
  `MAX_TILES` is the only bound on a loop an *empty* cell makes invisible to `MAX_OPERATIONS`
  (1 000 000 empty tiles in 889 ms reporting nothing), `MAX_OPERATIONS`' 31 all terminate wanting
  4.1–53.6 M **lexer tokens** — the word here was *operators* and the counter was counting tokens
  until ADR 0306 corrected the unit without moving the value — and `MAX_STATE_DEPTH`'s single
  witness wants 337 where §C.2's Table C.1 prints 28 as the depth a writer could rely on. **The
  `MAX_OPERATIONS` figure is therefore a population of the old unit**: re-measured over 926 680
  pages of 65 967 crawled documents, 48 pass four million tokens and 8 pass four million operators.
- **Two documents are slow, which this population had never produced, and both are *complete*.**
  `0423548.pdf` (archive `0423`, 9 933 485 bytes, SHA-256
  `0db5152253cc8483dad26ae0c27cba5e54c88e6a941603ca17b27b8a4d487c85`) at **32.9 s** and
  `6081357.pdf` (archive `6081`, 4 390 859 bytes, SHA-256
  `c43ac28fd21d5d13201849d641346b9269582670c5b3ecdc0879228ec1964ab8`) at **68.0 s**, both measured
  again on their own rather than under 24 threads. They report nothing; they simply take that long
  to draw page one — and this population had never produced a "slow" at all: sessions 425 and 430
  both printed 0, over 1896 and 4000 documents.
  ~~**Undiagnosed, and it is the next thing this file owes**: a profile of one of the two says whether
  it is one construct or the size of the file.~~ **Diagnosed in the four-hundred-and-thirty-fifth,
  and it is one construct** (ADR 0271). Neither is slow to parse — `Document::open` is 6.8 ms and
  3.8 ms — and both spend the time in `render-cpu`'s `build_soft_mask`, which drew every mask group
  into a buffer the size of the *target* and then demultiplied and derived a luminosity for all of
  it. `6081357.pdf` states **912 distinct soft masks** on a 4.3-million-pixel page, so it ran that
  pass over 3.87 **billion** pixels of which **99.96% were wholly transparent** — the case
  §11.6.5.1 answers with one constant. Naming that constant took the two to **6.6–7.3 s** and
  **3.7–4.0 s**, three samples each, with no page of the oracle's 1794 or quorra's 957 moving.
  **The survey's own `slow` count is not the way to say so, and three passes are why**: before 2,
  after 0, and a third pass on an idle machine 1 — a *third* document, `1284722.pdf`, which takes
  **11.13 / 11.14 / 11.23 s alone** and crosses the 30 s budget only under the survey's 24-way
  load. Every other line of the survey is identical. ~~`1284722.pdf` is 11.1 s of `interpret` for
  94 596 commands and is the next candidate this population offers.~~ **Taken (ADR 0287): 11 011 ms
  → 142 ms.** Its `/Resources /ExtGState` is an *indirect* dictionary with 26 414 entries and the
  page states 26 414 `gs` operators, and `Document::get` hands back an owned object — so the
  interpreter copied that whole map once per operator, which was 57% of 108 G instructions.
  `Interpreter::resource_tables` remembers an indirect category table by its `ObjectId`, and a
  direct one is read in place. `6081357.pdf` went 267 → 207 ms with it; `0423548.pdf` did not move
  at all, because its seconds are `initial_backdrop`'s and belong to `doc/todo/40`. What is still owed on those pages is the *band*:
  `0423548.pdf`'s remaining seconds are 2.1 of interpretation and 2.85 of `initial_backdrop`,
  which allocates and copies a whole surface per group — **4.3 GB** across its 136 groups where
  their bands are **82 MB**. `doc/todo/40`.
- **Re-surveyed whole in the round that took ADR 0288** — 145 archives, one process each, 65 944
  documents, 1188 s, 0 failures: *173 unopenable, 45 locked, 23 encrypted beyond us, 52 pageless,
  **823 incomplete**, 2 slow*. 1144 → 823 over the eleven rounds since ADR 0269, and **the two slow
  documents are not the two ADR 0271 diagnosed**: those were fixed, and these two were under the
  30 s threshold before the survey's 24-way load put them over it.
  - **`3129278.pdf`** — 5 874 commands, **34 450 ms**, and 95% of its 380 G instructions inside
    `ColourSpace::parse_at`: 1053 distinct axial shadings, each preceded by its own `cs` naming one
    `[/ICCBased 15 0 R]`, so the profile was inflated and read 1053 times. **Taken: about 1 550 ms**
    (ADR 0288), and it re-prices `doc/todo/41` on a second population.
  - **`3990833.pdf`** — 279 commands, **24 948 ms → about 19 500**, and **what is left is the next
    candidate this population offers**: 38 images on one page, and `callgrind` over its 233 G
    instructions says `image::convert_channels` 22.2%, `zune_jpeg` about 30%, `colour::press_at`
    9.8%, `zlib` 9.3%. A per-sample press conversion with a memo (`image::Conversion`) that a
    photograph's colours defeat. Trap 6 binds anything done about it: `ColourSpace::to_rgb` is the
    only place a colour becomes RGB.

- **Nothing failed to open for a reason that is this tree's, for the third sample running.** 163 of
  the 225 unusable documents have no `%PDF-` header in their first kilobyte, 52 have no first page
  and 5 have an unusable cross-reference table — and all five of those were opened by hand: three
  have had their `<<` and `>>` replaced by `&gt;` in transit, two are truncated to about a hundred
  bytes. Four pageless ones were opened too and two state a linearised `/L` in the millions while
  being 968 and 1431 bytes long.
- **20 of the 23 encryption refusals are `/R` 5**, the deprecated proprietary extension the standard
  states no algorithm for (§7.6.4.2, Table 21) — 0.03% of the web, which is the number that says
  whether it would ever be worth implementing.

**The four-hundred-and-thirtieth's 4000 new documents, for comparison:** *4000 documents in 53.3 s: 6 unopenable, 3 locked, 2 encrypted beyond us, 2 pageless,
**70 incomplete**, 0 slow*, with 1161 codes reaching no glyph in silence over 33 documents — **64
incomplete after that session's own two fixes** (ADR 0266). The rate is what moved: 86 of 1896 was
4.5% and 70 of 4000 is **1.75%**, because sessions 426 and 427 closed §11.4.7's conversion into the
page group's blending space and its population fell 67 → 24.

**The residue, ranked by document count** — a document reporting two things is in two rows:

| population | documents | named by |
|---|---|---|
| §11.4.7's page-group blending space | **24** | `doc/todo/23`, and see below |
| a font with no outline for any code the page shows | **14** | `doc/todo/21` §3 |
| an image | **11** → 7 | five things; 4 were the JPEG fix |
| **a budget stopped interpretation** | **8** | nothing until now — `MAX_TILES` 4, `MAX_OPERATIONS` 4 |
| §11.6.6's group in a space of its own | 4 | `doc/todo/23` |
| §11.4.4, and text-showing operators skipped | 3 apiece | `doc/todo/23`, `doc/todo/21` |
| a `/Contents` part that would not decode | 3 → 2 | 1 was the empty-stream fix, 2 are truncations |
| six singletons | 1 apiece | §7.8.3, §11.4.6, §9.3.8, a shading, an operator, an annotation |

Two rows are worth taking away from this:

- **All 14 array-formed page groups are four-component `ICCBased` spaces**, checked with
  `examples/group_space_census`. `doc/todo/23` prices that row at 0 corpus documents and 1 web
  witness; it is **14 of 4000, 0.35%**, the largest single *named* residue in the sample, and it
  wants an ICC `B2A` transform rather than sixteen corners.
- **The budget row is a population nobody had named**: 0.2% of the web reaches `MAX_TILES` or
  `MAX_OPERATIONS`, stable across two samples, and **not one of the eight was slow** — the bound
  stops the work inside the per-document budget rather than after it. `doc/todo/49` keeps both
  constants under "not negotiable" and this changes none of that; what is owed is reading *one* of
  the eight pages to find out whether the bound cost it a mark.

**The four-hundred-and-twenty-fifth's 1896, for comparison:** *1896 documents in 42.1 s: 4
unopenable, 1 locked, 0 encrypted beyond us, 3 pageless, 86 incomplete, 0 slow*, with 862 codes
reaching no glyph in silence over 12 documents; 85 after that session's own fix. Two things it is
worth knowing before reading the next one:

- **Nothing failed to open for a reason that is this tree's.** All seven unusable documents are
  crawl artefacts, opened by hand: four are HTML saved under a `.pdf` name, three are PDFs the
  origin server truncated at about a kilobyte with a Baidu link-submission script where the body
  should be.
- **The failure modes are overwhelmingly already-named ones**, and the number is the result:
  **67 of the 86 are §11.4.7's page-group blending space**, which is `doc/todo/23` and ADR 0251.
  That is **3.5% of the web's PDFs against 0.7% of the 974**, and it makes that item the single
  largest correctness gap this tree has against real files by a factor of six over everything
  else together. 7 more are `doc/todo/21` §3's font-with-no-outline and 4 are §11.4.4's
  non-isolated group. The remaining 8 are singletons and are listed in ADR 0261.

### 2. ~~`openpreserve/format-corpus` is owed a licence reading, not a decision~~ — **read in the four-hundred-and-sixty-seventh and decided in the four-hundred-and-seventieth**

**Done.** The reading is `doc/oracle-and-corpus.md` §2c and the decision is the project owner's,
quoted in ADR 0305: add a corpus unless its licence *clearly* forbids it, because a submodule is a
pin rather than a copy and pinning republishes nothing — and mention them all as a courtesy anyway.
Three of the five directories are `doc/corpora/format-corpus`; `jhove-errors` and
`fully-featured-pdf` were left on size and on value, which is stated as such rather than dressed as
a licence.

**What this changes for a later round** is the rule and not the corpus. A candidate corpus is now
taken unless somebody can point at the sentence that forbids it, and the questions worth asking of
one are the other two: will anybody run it, and what does a fresh clone pay for it. Both were
answered here — 73 MB, no gate depending on it, and a defect fixed the day it arrived.

### 3. The promotion budget, and why it is mostly moot

The owner's rule: PDFs exposing a problem come into the test set; below **20 MB** of such files
they are committed and above it only their names, with enough to fetch each again exactly.

Applied here it splits, and the split is the useful half:

- **A file from a submodule needs no promotion.** The submodule is pinned by commit, so the file
  is already exactly reproducible and carries **zero bytes** into this history. A test names the
  path and skips where the submodule is absent — `crates/pdf-model/tests/contents_entry.rs` is
  the pattern. **Running total against the 20 MB budget: 0 MB.**
- **The budget binds SafeDocs alone**, where no submodule is possible and the files are crawled
  web pages under no grant anybody made (`doc/third-party-data.md`). A names-only entry is the
  archive, the member name and the SHA-256, which is what `manifest.tsv` records.
- **A crasher is a different thing** — small, always committable, and `CLAUDE.md` requires one.
  **The first one arrived in the four-hundred-and-twenty-fifth**, and it cost the budget nothing
  either: §7.10.4's `/Functions` naming its own object overflows the stack, and the **696-byte**
  witness is *generated* by `crates/pdf-model/tests/hostile_functions.rs` rather than committed —
  720 until the four-hundred-and-twenty-eighth asked the fixture's own construction how long it is.
  A crasher a test can write is better than a crasher a test has to store.

### 4. Two instruments the new populations make possible — one built, one not

> **The heading's "one not" is spent, and the bullet below is about one corpus of the four.**
> The six-hundred-and-ninety-second session put the oracle in front of all four and gated two of
> them — `pdf20examples` and `pdfbox` — on the rule that a vote is evidence only where there is a
> clause the references are both reading (ADR 0541, `doc/oracle-and-corpus.md` §2e).
> `format-corpus` joins `pdf-differences` outside the vote, and for a different reason: every file
> in its three pinned directories is deliberately damaged, and `CLAUDE.md` says the standard
> "describes *valid* files and says nothing about the rest". Both exclusions are censused rather
> than assumed. **What that round found is that "ranked" and "voted" are different things** — §§8,
> 12, 13 and 14 below each say a population was put in front of the three references, and each
> means the ink ranking, which reaches no verdict and holds no page by name.

- ~~**The oracle over the new corpora.**~~ **Decided in the five-hundred-and-fifty-eighth and the
  answer is no** — **for `pdf-differences`**, which is the corpus this bullet is about; the other
  three were decided in the six-hundred-and-ninety-second (ADR 0541), two of them yes. This bullet
  said `pdf-differences` "exists *because* readers diverge
  on its files, so it is the population where a reference comparison should be most informative",
  and asked for a decision about the verdict vocabulary before anybody ran it. **Both halves were
  wrong, and reading the corpus is what showed it.** Sixteen of its eighteen test cases quote a
  normative sentence of ISO 32000-2 and then state which rendering is correct — the repository's
  README says so as a convention, "Correct renderings are always the _last_ image in the MarkDown"
  — so the corpus is about *implementations* differing and not about the standard permitting them
  to. On such a population the references are the **subject under test**, and voting them reads the
  answer off the programs the corpus was assembled to catch out. §14 has the numbers that make that
  concrete: on six of the eighteen cases at least one reference is wrong against the clause, and on
  one of them two of the three are.

  **The verdict vocabulary does not change either**, and the argument is in ADR 0393 §2: every
  outcome `pdfref` has is a function of the rasters, "the standard permits this" is a function of a
  clause, and a term the instrument cannot compute becomes the bucket every unexplained page goes
  into. A permitted difference is `ambiguous` — or `contradicted`, and it stays `contradicted` —
  held by a **named group quoting the permission**, which is the mechanism `oracle.rs` already has.
  What this corpus is for instead is a reading list today and a per-case gate later, wherever the
  clause supplies the expected value with no reference in it.
- **`pdfbox`'s own expected text — done in the four-hundred-and-twenty-third** (ADR 0259).
  `text_extraction.rs::the_text_we_draw_agrees_with_pdfboxs_frozen_extraction`, **40** documents
  (that is how many of the 64 carry a `.pdf.txt`, not 64), whole documents rather than page one,
  both of PDFBox's orders read and the stream-ordered one gating. **0.4 s and no new line in
  `doc/todo/02` §2**, which already runs every ignored test in that binary. Its first run found
  five below the floor, three of which were one defect — §9.10.2's permission declined for every
  `Identity-H` composite font — and the pdf.js gate moved 23987 → 24003 words and 25 → 23 named
  documents on the fix. The four that remain are named with their reading in `PDFBOX_BELOW_FLOOR`.
  **What it does not yet do is the other direction**: it measures recall, so a rule that *invented*
  text would not move it. `examples/readback.rs` is what a person reads for that, and pairing the
  two automatically is the next thing this instrument is owed.

### 5. Two things this round diagnosed and did not take

- ~~**A zero-byte stream that names a filter.**~~ **Done in the four-hundred-and-thirtieth**
  (ADR 0266), and the argument it was waiting for is §7.3.8.2 rather than §7.3.8.1. §7.3.8.1 makes
  "zero or more bytes" conforming and Table 5 has `/Filter` name what "shall be applied in
  processing the stream data found between the keywords stream and endstream ", so an empty stream
  decodes to nothing. What decides the objection — that a stream *truncated* to nothing arrives
  holding no bytes just the same — is §7.3.8.2: `/Length` "indicates how many bytes of the PDF file
  are used for the stream's data" and "[a]ll of these constraints shall be consistent", so a
  truncation states a number the bytes do not support and only a stated zero the bytes agree with
  is silence the producer asked for. `Document::states_no_data` is both halves, and deliberately
  not on the image path, where §7.3.8.2's own "many objects from whose attributes a length can be
  inferred" makes a stated zero a contradiction. Two of 5944 members do it and two more are
  truncations.
- ~~**No fuzz target reaches `pdf-model`'s interpreter.**~~ **Done in the
  four-hundred-and-twenty-eighth** (ADR 0264), and both halves of the note above it were wrong.
  **`cargo-fuzz` was installed all along** — `~/.cargo/bin/cargo-fuzz` 0.13.2, dated 26 July,
  a fortnight before the round that said otherwise; it is simply not on `PATH`, so `which` reports
  a false negative and both this note and ADR 0261 were written from one. And **`confined_wire`
  does not reach `pdf_model::interpret`** either: `nm` finds the symbol in exactly one of the
  thirteen binaries, `variable_text`, whose page has no `/Resources` — so it was one target rather
  than two, and twelve binaries did not contain the interpreter at all. `page` is the fourteenth
  target: a whole document through `interpret`, seeded from the 1944 SafeDocs documents, the 108
  in `doc/corpora` and the pdf.js submodule's 974 by `fuzz/seed_page.py`.

### 6. What the four-hundred-and-thirtieth diagnosed and did not take

- **Eight documents reach a budget.** `MAX_TILES` on `0100935`, `2100091`, `4100668`, `4100929`
  and `MAX_OPERATIONS` on `0100034`, `2100236`, `2100253`, `6100352` — 0.2% of the web, and the
  same rate session 425 saw at one apiece in 1896. **This is a finding about the budget and not a
  reason to raise one.** What is owed is one page: interpret it with the bound lifted *in a
  scratch build*, compare the raster, and find out whether the constant costs a mark or stops a
  bomb. Until somebody does that, neither answer is known and the constants stay where
  `doc/todo/49` puts them.
- **Four documents carry an `/SMask` with a `/Matte`** (§11.6.5.3), which is the second-largest
  image row and has no todo of its own. `0100547`, `2100434`, `4100238`, `6100743`.
- **The array-formed page groups are all four-component `ICCBased`**, which is `doc/todo/23`'s row
  and now has a number: 14 of 4000.

### 7. What the four-hundred-and-sixty-seventh diagnosed, both of which are now taken

- ~~**A page tree node with no `/Kids` becomes a leaf, and the blank page it yields is silent.**~~
  **Taken in the four-hundred-and-seventieth** (ADR 0305), and the population this file said it
  wanted was measured first: `examples/kidless_node_census` walks the tree with `pdf_syntax` alone
  and finds **0 of the 65 703** SafeDocs documents that open with such a node, 1 of the 1025 pdf.js
  and specification documents, and 1 of the 165 in `format-corpus`. Zero on the web is what decided
  the shape: the construct is unreachable by any corpus, so the rule is pinned by five pairs of
  hand-built fixtures in `crates/pdf-model/tests/page_tree_nodes.rs`. The reading is Table 31's —
  `/Type` is required of a page object and names `Page` or `Template`, so a dictionary saying
  `Pages` is not one — and where that empties the tree the recovery scan finds the producer's page,
  which is what makes `T02-02_005_page-tree-no-kids.pdf` draw its *Hello PDF-world!*. One document
  of the 974 moved with it and `MAX_PAGELESS` went 5 → 6, argued at the constant.
- ~~**A `Tf` whose size operand is a lone `.`**~~ — **read and taken in the
  four-hundred-and-sixty-eighth** (ADR 0303). §7.3.3 writes both numeric forms as "one or more
  decimal digits", so a run holding none is no number at all; the lexer returns the `Keyword` it
  lexically is, and everything downstream was already written — the parser refuses a keyword where an
  object belongs, and the interpreter reports one it does not recognise. The population was counted
  on both sides over all four corpora on this disk, 67 293 documents: **twelve** change a report and
  nine of those were already reporting; **three** display lists of 11 349 move and only one of the
  three changes a pixel — `issue9252.pdf`, whose producer wrote `. .59 .84 rg` meaning zero and whose
  word is now black rather than teal, because a guess that happened to be right is still a guess.

**And the instrument is now spent, which is worth knowing before spending a round on it.** All five
of the handbuilt corpus's silent blanks are accounted for: two are the blank the standard asks for
and three were defects, taken in three consecutive rounds. Thirteen of its 89 files draw nothing
today and every one of them either reports or is right. A round wanting a finding from this
direction should point the same ink assertion at `pdfCabinetOfHorrors` or `govdocs1-error-pdfs`,
where the files do *not* share a page and the method therefore needs a reference again.

### 8. Both of those were taken in the five-hundred-and-fifth, and the method transfers

**Taken: `pdfCabinetOfHorrors` (24) and `govdocs1-error-pdfs` (54), 78 documents**, with page one
rendered against `pdftoppm`, `mutool` and `gs` at 72 dpi — every one of them told to use the crop
box, which is trap 3 and not optional — and ranked by **our ink minus the lightest live
reference's**, which is `doc/todo/00` step 7's number applied to a population that has no
ambiguous bucket. Both survey lines reproduce the four-hundred-and-sixty-seventh's exactly.

**The ranking separated one row from the rest by two orders of magnitude**, which is what makes
the method worth repeating on a corpus whose files share nothing: `veraPDFHiResChangedHeight.pdf`
at −94.953 against a next-largest of −1.719, and everything below −0.65 a document this tree
already reports on. The two largest *silent* negatives, `032270` and `427330` at −0.643 and
−0.623, were opened side by side against two references and are the same page three times —
which is the second half of what the number is for: it says where to look **and** where not to.

The defect is ADR 0340's and the reading is §7.4.8's. Two things it leaves for a later round:

- ~~**A corrupt flate content stream, and whether a truncated recovery is ever right.**~~
  **Settled in the five-hundred-and-eighth, and the question rested on a false premise about this
  tree** (ADR 0343). It was posed as *should this tree start keeping the prefix?*, and **this tree
  already kept it** — `FilterRefusal::Corrupt`'s own documentation said so and
  `stream_length_bound.rs` had a test asserting it since ADR 0306. So the decision was never
  whether to recover; it was whether a recovery that says nothing is one the principles permit,
  and it is not. §7.4.1's sentence has two halves — a reader "shall invoke the corresponding
  decoding filter or filters to convert the information back to its original form" — and a damaged
  stream is a decode that did the first and could not finish the second. Both halves are stated
  now: the prefix is drawn, and `ContentIssue::Damaged` says it is a prefix.

  **Two defects were under the question, both silent, and neither was the one this entry
  predicted.** The recovery said nothing at all, so a page cut short was indistinguishable from a
  page meant to be sparse; and it was *unreliable*, because `read_to_end` discards whatever the
  erroring `read` call produced, so the prefix survived only to the last whole call — which is
  why `498264.pdf` reported `Undecodable` while the module promised a prefix. `flate` is driven
  through `flate2::Decompress` now, which also tells RFC 1951's final block from an input that
  merely ran out; **a truncated stream had been indistinguishable from a whole one since the first
  parser**.

  **The population was measured rather than guessed, and it is the reason this was worth a
  round**: `examples/damaged_stream_census`, over all 65 944 crawled documents. The entry above
  was right that the population matters more than the witness and wrong about which population —
  the undecodable parts are the *small* half.

- **`498264.pdf` itself buys a report and no mark, which is the honest outcome.** Its 18 recovered
  bytes are `q\n30 31.16 552 729` and yield **0 drawing commands**. `poppler`'s three lines are
  *past* the invalid distance code, so recovering them means resynchronising a broken deflate
  stream — a guess about bits nobody wrote — and principle 5 makes agreement with one renderer no
  target at all. The clause was the target and it bought a sentence.
- **`pdfCabinetOfHorrors` has one report left and it is the file's own defect**, a `/Im0` naming
  an object that is not a stream, which every reference draws blank too. So this corpus is spent
  in the same sense the handbuilt one is, and `govdocs1-error-pdfs`' six reports are all named
  populations — four unparsable font programs, one truncated `head`, one undecodable
  `/Contents`. **What is not spent is the instrument**: the ink ranking against three references
  found a whole-page defect in 78 files, and no survey line moved when it was fixed.

### 9. The chunk the five-hundred-and-eighth took: damaged streams, over every corpus on this disk

**Every figure in this section and the next was measured before ADR 0366**, which found that
§7.3.8.2's indirect `/Length` was costing every affected stream its last byte and making it read as
damaged. The arguments below stand; their denominators are roughly a third smaller now, and the
census is what prints today's. §11 has the correction.

**The population §8 named, measured whole.** `cargo run --release -p pdf-model --example
damaged_stream_census -- <dir>` asks two questions of every document — what page one's
`/Contents` kept and whether the prefix draws anything, and how many stream objects *anywhere*
in the file decoded only part-way. One process per archive, the surveys' own method and for
their reason. The run is a baseline for this population, never a ratchet.

Three things it says, and the second is the finding:

- **A damaged `/Contents` is rare and it draws.** Ninety documents of the crawl have one, and
  **eighty-five of the ninety put at least one drawing command on the page from the recovered
  prefix** — so the rule §8 asked about had been buying real marks all along, on 0.14% of the
  web, while saying nothing about any of them. Twenty-four more keep nothing at all and stay
  `Undecodable`; that is ADR 0269's row, which this file recorded as 32 before the intervening
  fixes. Truncation outnumbers corruption roughly three to one.
- **The wider silence is the number to take away**: **2260 damaged streams over 726 documents**,
  against 90 whose `/Contents` is the damaged one. So the route this round made loud is about
  4% of the population, and the rest reach a font program, an ICC profile, an image or a
  function through `Document::decoded_stream_data`, which drops the damage by design and keeps
  sixty-one call sites off the change. **`pdf_font::program` is the one other route now closed**
  and it was closed because a corpus document forced it, not on principle — see below.
- **`govdocs1-error-pdfs` is where the witnesses are readable**, 29 of its 54 documents holding
  one: `507676.pdf` is the sharpest, 67 923 recovered bytes and **33 854 commands** this tree has
  been drawing from a corrupt content stream in silence, and it is the document session 505's ink
  ranking put at −1.719 without anyone asking why.

**What is left, and it is a real item rather than a formality.** The other 96% of damaged streams
are still silent — **and that fraction was wrong, which §10 corrects with the same census broken
down by consumer**: 841 of the 2260 are a page's `/Contents`, not 90, because 90 counts only the
documents whose *page one* holds one. Whether each of the rest deserves a report is a *separate*
question per consumer and must be argued per consumer, because the answer is not uniform — which
is the round's own lesson:

- **A font program is refused**, and the witness made the argument rather than the other way
  round. `issue13316_reduced.pdf`'s corrupt `/FontFile2` yields 863 bytes that parse and draw
  **`A C E F`** where `pdftoppm` draws the file's six CJK glyphs. §7.8.2 makes a content stream
  "a sequence of instructions", so a prefix of one is a shorter sequence of the same kind; a font
  program is a table directory whose offsets point forward, so a prefix is a directory describing
  bytes that are not there. Trap 5's substitutive test (ADR 0106) then decides it: the wrong
  glyphs stand *in place of* the right ones. **Trap 1 in one page — the command count rose and
  the picture got worse.**
- ~~**An image, an ICC profile and a function are unexamined**~~, and each wants the same two
  questions: is a prefix of this a smaller one of the same kind, and are the marks it makes
  additive or substitutive? A round taking this should point the census at
  `govdocs1-error-pdfs` first, where 29 of 54 documents carry one and every file is small enough
  to open by hand. **All three answered in the five-hundred-and-twenty-first** (§10 and ADR 0356),
  and the *other* four content streams in the five-hundred-and-twenty-fourth (ADR 0359). What the
  method left standing is the two questions themselves, which are now the ones §10's last paragraph
  asks of the roles nobody has read.

### 10. The chunk the five-hundred-and-twenty-first took: the same population, by consumer

**§9's own instruction, followed**: point the census at `govdocs1-error-pdfs` first, where 29 of 54
documents carry a damaged stream and every file is small enough to open by hand. Taken as the
first step of a chunk that then ran over `format-corpus`'s 167, the 974 and all 65 944 crawled
documents — one process per archive, 145 archives, 0 failures. `examples/damaged_stream_census`
gained a **role** per damaged stream, read off the entry the standard makes required of that
object, and the two *extent* arithmetics §7.3.8.2 and §7.10.2 state. Its three older lines
reproduce the five-hundred-and-eighth's numbers to the digit, which is what says the instrument
did not move under this round's changes. A baseline for this population, never a ratchet.

**The 2260 damaged streams, by the consumer that reads them** (ADR 0356 has the table with what
each does today): `/Contents` **841**, an image **529**, a font program **371**, unclassified 296,
an object stream 144, a form `XObject` **46**, an ICC profile 19, a cross-reference stream 10, a
metadata stream 2, a function 2.

**And the population that matters more than damage, which is why §9's question had a better
predicate**: a stream short of the extent the file's own attributes infer is wrong whether a filter
failed or the producer simply wrote too little. **54 images in 8 of the 65 944**, 51 in 2 of the
167, **0 of the 974**, and **not one** short sampled function anywhere on this disk.

Three answers, one per consumer §9 named, each in ADR 0356:

- **An image** drew its missing samples as **zero** — black rows, and a *marked* page for an
  `/ImageMask`, whose §8.9.6.2 default paints where the sample is 0. `178360.pdf` is the witness
  and it was already in this file: session 505's ink ranking put it in the positive tail as "ours
  40.7, `poppler` 26.2" without anyone asking why, and the why is a 133 × 2944 stencil corrupt 359
  bytes into the 50 048 its grid needs, **99.3% of it marked in the fill colour**. What the stream
  carries is drawn where it belongs, the rest of the grid is left unpainted, and
  `image::short_of_its_grid` reports it beside the drawing.
- **A sampled function** is refused, on §7.10.2's own sentence. A prefix of one is not a smaller
  function: its missing samples are *values* of a mapping evaluated over the whole domain, read as
  0 and interpolated into the real samples beside them. No corpus on this disk reaches it, so it is
  pinned by a hand-built pair (`tests/sampled_function_extent.rs`).
- **An ICC profile** needed no report: Table 65 states the whole recovery for a profile a reader
  cannot use, and `colour.rs` already took it. What it needed was not to be *parsed* — a tag table's
  prefix is a directory describing bytes that are not there, and a missing `rTRC` reads as no curve
  at all rather than as a failure.

~~**What this chunk leaves, and it is one report rather than a reading.**~~ **Taken in the
five-hundred-and-twenty-fourth** (ADR 0359). A damaged **form `XObject`, tiling pattern, appearance
stream or Type 3 glyph description** was still silent, and §7.8.2's argument for a page's
`/Contents` covers all of them word for word — they are content streams and this tree already draws
their prefixes. 46 of the crawl's damaged streams and 7 of the pdf.js corpus's 57 are form
`XObject`s, and `comments.pdf` and `highlights.pdf` were the witnesses.

**Two corrections that entry earns for the next round, both about the list rather than the rule.**
It named `appearance.rs` and `type3.rs`, and neither is where the stream is read: the appearance is
decoded in `annotation.rs` (the drawn copy passes through `content/annotations.rs`, and §12.7.4.3's
regeneration *splices* the bytes, so a report at the draw would go quiet for exactly the fields a
reader has typed in), and the glyph description is decoded in `content/text.rs`. `type3.rs`'s
`decoded_stream_data` is the font's **`/ToUnicode` CMap**, which is not a content stream at all,
costs no mark and is correctly silent — its missing codes are already counted in
`codes_without_a_character`. And the count of kinds is **five**, not four, because §11.6.5.1's `/G`
is a form too and is the one of them whose answer had to be derived rather than carried over.

### 11. ~~What the same census still leaves, one round later~~ — **taken in the five-hundred-and-thirty-first** (ADR 0366)

All three roles are answered, and the round that answered them found that most of the *population*
sections 9 and 10 measured was this reader's own defect. What is left standing here is the method
and the correction; the numbers are `examples/damaged_stream_census`'s to print.

- **unclassified was not a role and is now split.** `who_names_what` classifies a stream by the
  entry that names it, because the standard makes that entry the statement of the role — which is
  what a page's `/Contents` had always been doing and nothing else was. The answer is that the
  bucket was mostly already decided: its majority is `/ToUnicode` CMaps, whose silence session 524
  argued for on §9.10.3, and Type 3 glyph descriptions, which ADR 0359 made loud. The `form
  XObject` row split the same way, into forms and annotation appearances.
- **an object stream, §7.5.7, is a prefix rule the clause states outright** — NOTE 7's "ends prior
  to the byte offset of the next object or when the end of stream is encountered" — so every object
  but the last has a stated end and the last one has none under damage. That one is now refused
  rather than parsed from bytes that stop early, and `Document::objects_lost_to_damage` says what
  went.
- **a cross-reference stream, §7.5.8, is short when its own `/Index` and `/W` say so**, which is
  the arithmetic rather than the damage flag, and the entries it loses are *unknown* rather than
  deleted — the one place in this reader where those two differ. **The condition has no members on
  this disk**, so it is pinned by a hand-built pair, as the object-stream rule is.

**The correction, and it is the thing to carry forward.** Reading the first of those roles produced
a corpus witness that disproved the reader instead: §7.3.8.2's `/Length` may be an indirect
reference, a parser cannot follow one, and the scan it falls back to was taking a byte off every
stream whose producer wrote no end-of-line before `endstream`. For a `FlateDecode` stream that byte
is RFC 1951's final block, so the stream read as *truncated* while being whole. **Sections 9 and 10
above quote populations measured with that defect in place** — they are this reader's damage flag
rather than the files' damage, and the census after ADR 0366 prints numbers roughly a third of them,
with the *corrupt* counts unchanged. The arguments in those sections are untouched; only their
denominators moved.

**What a round taking this population next should know**: the damaged-stream census is a
measurement of this reader as much as of the files (trap 8's fourth shape), and the way that was
found was reading a single witness by hand rather than trusting a count.

### 12. The chunk the five-hundred-and-forty-fourth took: the rest of `format-corpus`

§11 named no successor, so §1's rule decided it: **the 126 documents of
`openpreserve/format-corpus` that the submodule's sparse checkout leaves behind** —
`jhove-errors` (99), `office-examples` (13), `ebooks` (8), `variations` (4),
`fully-featured-pdf` (1), `desktop-publishing` (1), 261 MB, no network. Chosen over §1's other
offer, SafeDocs' 31 GB issue-tracker corpus, on §1's own finding that a diagnostic corpus outranks
a large one when a round wants a defect rather than a rate — and `jhove-errors` is one directory
per JHOVE error code. **The last 26 of those documents had never been in any survey at all**: §2c's
five-directory question was about the PDF-corpus directories, and `ebooks`, `office-examples`,
`variations` and `desktop-publishing` are not among them.

The survey line, a baseline for this population and never a ratchet: **117 complete, 5 locked, 2
incomplete, 1 pageless, 1 unopenable**, the two incomplete being a `/Font` and an `/ExtGState` a
page names and the file never defines (§7.8.3, ADR 0255) and the five locked wanting an open
password. `tools/safedocs survey --dir` prints today's.

- **The pageless one was the finding, and it is ADR 0379's.** `PDF-HUL-138/6.2017-0960.pdf` is a
  21-page paper three references draw and this tree drew no page of: it is one complete document
  with a *truncated copy of itself* appended, so its correct `startxref` sits eight megabytes from
  the end and `find_startxref`'s 2048-byte window missed it — after which `rebuild` took the
  truncated copy's objects and emptied the page tree. §7.5.5's "PDF processors should read a PDF
  file from its end" is now read past the window, ahead of a rebuild §C.4 licenses only for a
  table that is "damaged or missing". **0.64% of the crawl misses the window and 189 documents are
  reached by the wider search**: 10 pageless and 10 incomplete become 5 and 8, nothing moves the
  other way, and the digest over the 974 is byte-identical.
- **Nothing failed to open for a reason that is this tree's, for the sixth population running.**
  The one unopenable file is an AppleDouble sidecar — 213 bytes of `Mac OS X` attributes and a
  `com.apple.quarantine` string under a `.pdf` name.
- **The ink ranking found no second whole-page row**, which is the instrument saying so rather
  than a round saying nothing: page one at 72 dpi against `pdftoppm`, `mutool` and `gs`, every one
  explicit about the page box, and the entire negative tail is **−0.744 and shallower** — glyph
  weight, with the largest opened side by side. Session 505's ranking separated its defect by two
  orders of magnitude; this one separates nothing.
- **Two documents render here and in no reference at all** — `PDF-HUL-29`'s pair, where poppler,
  mupdf and ghostscript each refuse the page tree for a `/Kids` entry that is not an indirect
  reference and this tree draws a complete journal page and a complete book title page. A ranking
  cannot see this direction: they sit in the positive tail with nothing to subtract.

**What this chunk leaves.** `jhove-errors` stays out of the sparse checkout, and the record of why
changes rather than the pin: `doc/third-party-data.md` left it on size **and** on value, and the
value half is disproved — it is left on size alone, with the whole five-directory corpus still
fetchable into `corpus-cache/` for a round that wants the population. What is unranked on this disk
after this chunk is `pdfbox`'s 64 and `pdf-differences`' 37; the second of those is §4's first
bullet and still wants its decision about the verdict vocabulary before anybody runs it.

### 13. The chunk the five-hundred-and-fifty-fourth took: `pdfbox`'s 64

§12 names its own successor and there was nothing to choose: **`doc/corpora/pdfbox`'s 64
documents**, the larger of the two it leaves unranked and the one that needs no decision first.
It is the right *kind* of chunk by §1's rule as well — `pdfbox/src/test/resources/input` is another
PDF library's regression corpus, so every file in it is there because it broke something, which is
the diagnostic property §1 says outranks size. This tree already read it for §2a's frozen
`PDFTextStripper` comparison and had never pointed a raster at it.

The survey line, a baseline for this population and never a ratchet: `tools/safedocs survey --dir`
reproduces `doc/oracle-and-corpus.md` §2's row for this corpus exactly, and the one incomplete is
the `MAX_FORM_DEPTH` that table has recorded since the corpus arrived.

- **The ranking separated nothing, and the finding is in the column beside it.** Page one at
  72 dpi against `pdftoppm`, `mutool` and `gs`, every invocation explicit about the page box
  (trap 3), ranked by our ink minus the lightest live reference's: the whole negative tail is
  **−0.410 and shallower** — glyph weight, and the three deepest were opened side by side — and
  the largest positive is `gs` drawing one notice 20% light. What the script also prints is each
  panel's **raster size**, because trap 3's tell is a dimension rather than a difference, and one
  row of the 64 has ours at 596 × 842 where all three references say 612 × 792. Its *gap* is
  `none`: every renderer draws that page blank, because its two `Text` annotations set Table 167's
  Hidden bit. **A page everybody draws blank at two different sizes is invisible to a ranking by
  ink**, and the audit column found it anyway. That is the transferable half of this chunk.
- **The defect is a page with no `/MediaBox` anywhere in its ancestry**, which §7.7.3.3 makes
  required and §7.7.3.4 requires of the page or of an ancestor. The standard states no recovery,
  so `Page::DEFAULT_MEDIA_BOX` stands in — **and it stays A4**, because moving a constant to match
  three references on a question the standard answers nowhere is curve-fitting, and the three do
  not even agree under perturbation: on a `/MediaBox [0 612 792]` they answer 612 × 792, 792 × 612
  and nothing at all. What the disagreement earns is a *report*, which is ADR 0389 and the
  eleventh place this program reports while drawing.
- **The population, measured with `examples/media_box_census` over every corpus on this disk** —
  one process per archive for the crawl, the surveys' own method, and a baseline rather than a
  ratchet: **4 of the 974, 1 of `pdfbox`'s 64, 4 of `format-corpus`' 167, 0 of `pdf20examples` and
  `pdf-differences`, and 1 document of the 65 703 crawled that open** — 22 pages, every one with
  `/Contents`, an arithmetic worksheet this tree draws 50 points low and 16 units narrow. That
  crawl rate, 1 in 65 703, is the rarest this file has recorded. **Not one page of the whole
  population states another of §14.11.2's boxes**, which is what says the substitution discards
  nothing the file wrote.
- **The handbuilt corpus was called spent in §7 and was spent only for the question it was asked.**
  Two of `format-corpus`' four witnesses are
  `T02-03_008_page-object-mediabox-missing.pdf` and `T02-03_009_page-object-mediabox-not-rectangle.pdf`,
  one per branch of the substitution, built to carry this defect and nothing else. §7's
  one-line ink assertion could not see them because **they never drew blank — they drew the wrong
  page**, and it took a different question, arriving from a different corpus, to reach them. A
  diagnostic corpus is spent per *predicate*, not per corpus.
- **The report cost zero judged pages**, which is trap 11's question answered rather than waved at.
  Of the four in the 974, one is already unusable, two are already incomplete for other reasons,
  and the fourth — `issue15590.pdf` — is `not comparable` because all three references refuse it.
  The corpus gate goes 64 → 65 incomplete and the oracle's `not comparable` moves 8 → 7 *on pages
  we call complete* with every other line identical; the display-list digest over all 974 first
  pages is byte-identical, which is why no quorra lane and no ink sweep were run.

**What this chunk leaves.** `pdf-differences`' 37 is now the only unranked population on this disk,
and it is still §4's first bullet: an oracle run over files chosen for disagreement will produce
`ambiguous` almost everywhere, so the verdict vocabulary wants deciding before anybody runs it.
The other offer §1 still holds open is SafeDocs' 31 GB issue-tracker corpus.

### 14. The chunk the five-hundred-and-fifty-eighth took: `pdf-differences`' 37, and the decision §4 held them behind

§13 named its own successor and §4 named the decision it wanted first, so this chunk is both:
**`doc/corpora/pdf-differences`, 18 test cases in 37 documents, CC BY 4.0**, and the verdict-vocabulary
question. ADR 0393 has the argument; §4's bullet above carries the outcome. Three things belong here.

- **Read the corpus before deciding what it is.** The round's premise — "files chosen for
  disagreement", so `ambiguous` everywhere — described a bag of ambiguities, and the corpus is the
  opposite: sixteen of eighteen cases quote a clause and publish the correct picture. **Exactly two
  differences in the whole corpus are the standard's own permission**, §8.4.3.4's zero-length dash
  at a zero-length subpath segment and §9.5 NOTE 5's substitution, and both say so in the
  standard's words rather than in the corpus's. A decision taken on the premise would have been a
  decision about a corpus that does not exist.
- **The survey line**, a baseline for this population and never a ratchet: 37 documents, 0
  unopenable, 0 locked, 0 encrypted beyond us, 0 pageless, 0 slow. It is one report above
  `doc/oracle-and-corpus.md` §2's row and both moves are new reports on purpose (ADRs 0356, 0359).
  **Run it with `PDF_SANDBOX_WORKER` pointing at a built worker**, or two JPEG 2000 images are
  refused and the line reads two higher — the confinement working, not the files.
- **The ranking's head is real for the first time since session 505.** Page one at 72 dpi against
  `pdftoppm`, `mutool` and `gs`, every invocation explicit about the page box, ranked by our ink
  minus the lightest live reference's, with session 554's size column beside it:
  `OverlappingGlyphClipping.pdf` at **−8.989** against a next-largest of −1.237, and the three
  references agreeing with each other to 0.32. And the size column found its second row in two
  chunks — `LineCap-Degenerate.pdf` is 4000 × 4000 here, in `mutool` and in `gs`, and 400 × 400 in
  `poppler`, which is Table 31's `/UserUnit 10`.

Four findings, three of them left with their witnesses:

- **§9.6.4's two colours — fixed.** A `d1` glyph description collapsed the stroking colour into the
  non-stroking one, so `Type3Test.pdf`'s dashed squares came out blue where the clause draws them
  red. §9.6.4's own EXAMPLE, its NOTE 2's plural "current colours" and the list of parameters a
  stroking description must set for itself all refute the reading, and `poppler` and `ghostscript`
  share it while `mupdf` does not. ADR 0393.
- **§8.4.3.5's mitre — `doc/todo/11` §6.** `LargeMitreLimit.pdf` sets `333 M` on a 10-unit line and
  this tree draws no mitre at all where `mutool` and `gs` put the tip exactly where
  `w/(2·sin(φ/2))` says. The cause is `tiny-skia`'s `AngleType::Nearly180` shortcut, which bevels a
  join sharper than about 1.27° whatever the limit says — a ratio cutoff near 90 hiding inside an
  angle test.
- **§9.3.6's non-zero winding over two substituted faces — `doc/todo/21` §6, and fixed in the
  five-hundred-and-sixty-first (ADR 0396).** The ranking's head. Our compiled-in Helvetica is an
  `sfnt` and our Times was a bare CFF wound the other way, and a text clip that overlaps a glyph of
  each cancelled where every reference unions. The permission is §9.5 NOTE 5's; the bad choice
  inside it was ours, and it was stateable with no reference at all — two substitutes for two of
  §9.6.2.2's "14 Type 1 fonts" wind the same way now, which a test asserts over all fourteen. The
  page goes from −8.989 of 255 to −1.116, so this corpus's ranking has no head standing out from
  its body any more.
- **A rebuild that missed every compressed object — taken, ADR 0395.** `UnknownFilter-Linearized.pdf`
  is documented as fully processable and lost its text here: the scan `xref::rebuild` falls back to
  finds `N G obj` headers only, so the font inside an object stream was invisible. §7.5.7 states the
  recovery itself, and a rebuild now reads each object stream's own header; the item's file is gone
  and its population is in the ADR.

**What this chunk leaves, and for the first time it is not a successor.** Every population on this
disk is ranked. §1's other standing offer is SafeDocs' 31 GB issue-tracker corpus; the cheaper item
is the one §4 now names — the per-case gates this corpus makes possible, one clause and one
hand-built witness apiece, each with its expected value derived rather than voted.

### 15. The chunk the five-hundred-and-eighty-first took: the survey itself

**Not a population but the instrument that measures them**, and it is the one chunk in this file
whose subject is a previous chunk's tool. Session 580 reported that re-running `safedocs survey`
moved about ten documents in and out of §11.4.7's report and left it; this section is what that
was. ADR 0416 has the reproduction, the attribution and the three roads; three things belong here.

- **It was nondeterminism, not load sensitivity, and the two have different fixes.** Three quiet
  runs of one unchanged binary over one unchanged directory of 287 documents printed 30, 36 and 33
  press refusals; a fourth under twelve spinning cores printed 35, inside that range. What load
  changes is the interleaving, which is the mechanism rather than the cause. The `slow` count *is*
  load-sensitive and §1 already said so — **that is the entry a round should read first before
  reading anything else in this file as unstable**, because the two look alike in a diff and only
  one of them is about the documents.
- **Every survey line in this file that counts an incomplete over the crawl was measured with it.**
  The instrument shared `colour::MAX_PRESSES` between the documents it was judging: a table of
  eight, `static`, never evicted, so the ninth distinct press a process meets is refused and which
  documents those are is the scheduler's answer. `doc/traps/parsers-and-streams.md` trap 8's third entry is the
  general form — a measurement taken with the instrument under test is not independent of it — and
  this is the shape one step further out, where the instrument is not under test and is not
  independent of *itself*. The numbers in §1's tables are not retracted: the affected population is
  0.44% of the crawl (287 of 65 703). **The instrument is deterministic again since ADR 0417 made
  the budget the interpretation's**, so a round re-taking that survey should expect its incomplete
  count to fall and to hold still: over the same 287 documents it is 19 on three runs with every
  verdict line byte-identical, where it was 45, 46 and 47 with the lines differing.
- **The population is re-established with a census instead**, because a census shares nothing
  between documents and a survey shares a process. `cargo run --release -p pdf-model --example
  press_census -- <files>`, one process per archive, run twice and byte-identical over all 145:
  **2296 of the 65 703 crawled documents that open state §11.4.7's condition** — a page group whose
  blending colour space is not the device's, 3.49% — of which **287** name their press through a
  four-component ICC profile this tree evaluates, and those name **28 distinct presses**. The union
  over archives is what the census prints one line apiece for: `grep -h '^  press ' | sort -u |
  wc -l`. The 974 name **0** and the four submodule corpora none, which is why no gate has ever
  moved for this.

**What this chunk left** was the bound itself, and it was **taken in the five-hundred-and-eighty-second**
(ADR 0417): the press budget is per interpretation, which is what every other budget in this tree
already is, over a bounded process-wide cache of the *sampling*, which is a cache and may be shared
because it changes how fast an answer is reached and never what it is. `doc/todo/49`'s third-bound
section has the measurement that forced that split and the one number it leaves open.

### 16. The chunk the six-hundred-and-third took: the crawl, in front of a reference for the first time

§14 said "[e]very population on this disk is ranked" and it was true of the *curated* corpora only.
**The crawl had never been put beside another renderer at all** — 145 archives of surveys, which
ask whether this reader reports anything and never whether the page is right, so 64 507 documents
called *complete* were a claim nothing had tested. That is `CLAUDE.md`'s two questions with the
second one unanswered on the population it exists for.

**Taken: two whole archives, `0100` and `7680`, 2000 documents**, page one at 72 dpi against
`pdftoppm`, `mutool` and `gs`, every invocation explicit about the page box (trap 3), ranked by our
ink minus the lightest live reference's with each panel's raster size beside it (§13). About 0.33 s
a document at sixteen workers, so 2000 documents is eleven minutes — which is the number that says
what the rest of this population would cost.

- **The ranking's head separates by three orders of magnitude, and it is a defect of this tree.**
  `0100223.pdf` at **−225.633 of 255** against a next-largest of −5.040: a full-page scan the three
  references draw (their inks agree to 0.93) and this tree drew as a blank sheet, reporting an
  `XObject` the file does not define. **The file defines it.** Its name holds the byte 0xF4 — a
  Windows path with an *ô* in it — and the interpreter carried a resource name as a `String` built
  with `from_utf8_lossy`, so the probe was U+FFFD where the key was 0xF4 and §7.3.5's "exact binary
  match" could never happen. ADR 0438; the reverse direction, two names differing only outside UTF-8
  colliding onto one, had no witness anywhere on this disk and would have drawn the *wrong* object
  in silence.
- **The fix moved one row of the 1000 and nothing else**, re-ranked whole and diffed panel by
  panel, with every reference panel byte-identical. **No gate moved**: no document of the 974 states
  such a name, so this was invisible to every ratchet — the same statement §7 makes about a corpus
  measuring a construct's absence, from the other side.
- **The rate this chunk can claim is 1 in 2000, measured.** A whole-crawl census wants a walk that
  classifies a name by the entry that names it — `damaged_stream_census`'s `who_names_what` shape —
  and is a round of its own.

**What this chunk leaves**, and it is a successor with a number attached: **63 944 crawled documents
unranked**, about six hours of wall clock at this round's settings, in archive-sized pieces a round
can take one or two of. What is *not* answered is §4's question one population further out — the
**oracle** proper over the crawl, with its structural similarity, its consensus vote and its seven
verdicts. That is not a cost question but a meaning one, in ADR 0393's shape: on a population where
nothing supplies an expected value and no file was chosen for anything, what does a verdict assert?
Until somebody answers it, the ink ranking is what this corpus gets, and it has now produced a
whole-page defect on its first 2000 documents.

### 17. The chunk the six-hundred-and-thirteenth took: five more archives, and the tail 603 did not read

**Taken: `0300`, `1653`, `3252`, `4851` and `6327`, whole, 5000 documents**, on §16's instrument
unchanged — page one at 72 dpi against `pdftoppm`, `mutool` and `gs`, every invocation explicit
about the page box, ranked by our ink minus the lightest live reference's. Two minutes an archive
here.

**The instrument was re-checked before it was trusted, because 612 had moved something under it.**
Session 612 made this tree apply §14.11.2.1's crop on every target; archive `0100` re-ranked whole
against 603's own artefacts differs in **exactly one row of 1000**, which is 603's own fix. A
page-sized target *is* the crop box, so that clause could not reach this measurement — trap 14 from
the other side.

- **Both ends of the head are defects of this tree, and the positive end had never been read.**
  `4851434.pdf` at **−20.341** is a bilevel scan whose `RunLengthDecode` data decodes to exactly the
  266 456 bytes its dictionary describes and then carries one truncated run header; the filter threw
  the whole prefix away. `6327194.pdf` at **+244.885** is a **solid black page** — one command, no
  report — where a greyscale JPEG under `[/Indexed /DeviceRGB 255 …]` had every sample divided by
  255 before the lookup, so a 256-entry grey ramp was addressed only at its two darkest entries.
  §7.4.5 and §8.6.6.3; ADR 0448.
- **The reach is measured rather than argued**: all seven ranked archives re-ranked whole after the
  fixes move **5 rows of 7000**, three of them in 603's archives, and every other panel — ours and
  each reference's — is identical to the thousandth.
- **The positive tail above +10 is otherwise one reference.** 22 documents of the 5000 are pages
  where `poppler` alone draws almost nothing while this tree, `mupdf` and `ghostscript` agree; the
  witness opened side by side draws its rules and its header and none of its body font. A ranking
  against the *lightest* live reference is by construction sensitive to that, which is worth knowing
  before reading a positive gap as ink of ours.
- **What the head still holds, each with its evidence, in ADR 0448**: a 4256×6258 scan refused by
  `hayro-jbig2` 0.3.0's flat 10 000-instance cap where the page declares 13 264 and upstream has
  already replaced the cap; an aerial photograph drawn as 1700 `DCTDecode` images one sample tall,
  which is `doc/todo/11`'s subject on a real document; and a `/DefaultCMYK` `ICCBased` conversion,
  which is trap 9's family.

**What this chunk leaves: 58 944 crawled documents unranked**, in archive-sized pieces at two
minutes each. Both of this chunk's defects were invisible to every gate — no document of the 974
states either construct — which is the second round running that the crawl has answered a question
no curated corpus can.

### 18. The chunk the six-hundred-and-fifteenth took: seven archives, and a head that was ours from end to end

**Taken: `0423`, `1161`, `2268`, `3375`, `4482`, `5589` and `6696`, whole, 7000 documents**, on
§16's instrument unchanged and *reused* rather than rewritten — page one at 72 dpi against
`pdftoppm`, `mutool` and `gs`, every invocation explicit about the page box, ranked by our ink
minus the lightest live reference's. It was checked against 613's own three documents before it
was trusted, and reproduces them to the thousandth.

- **The six deepest positive rows are three different defects of this tree**, each a page all
  three references agree on and this one buried under two hundred levels of ink. Five are
  mixed-raster scans whose `/Mask` stencil is finer than the ceiling on the grid the two are
  combined on — *reported*, and what a refusal there draws is the base image unmasked, which is a
  solid black page. The sixth is a floor plan drawn as a photographic negative because black point
  compensation took an additive ICC profile's **white** point for its black, silently.
- **The deepest negative row is a third**, and also silent: `0423269.pdf`'s two coloured
  backgrounds are `ShadingType 4` meshes painted by `sh` inside a tiling pattern's cell, and the
  page-sized rectangle this tree stands in for §8.7.4.2's absent path is displaced by the lattice
  while the shading is displaced with it — so the site whose shading lands on the page is the site
  whose rectangle has left it, and no site paints. §11.6.4.2 says the surface is "the bounds of
  the shading's painti ng geometry", which is a property of the shading and therefore travels with
  it.
- **The reach is measured rather than argued**: all fourteen ranked archives re-ranked whole after
  the fixes, and the rows that move are the documents the three fixes are about and nothing else.
  ADR 0451; §8.7.4.2's, §11.6.4.2's, §8.9.6.3's and §8.6.5.9's rows.
- **Below +20 the positive head is 613's finding and not ours** — `poppler` alone drawing almost
  nothing — which is now a note in `doc/traps/oracle-and-references.md` and was read there instead
  of being derived again. That is what a trap entry is for.
- **What the head still held, and what the six-hundred-and-twenty-first session found there**: two
  silent rows below −8, called trap 9's family on sight. **One of the two was, and one was not, and
  the one that was not was a defect of this tree.** `6696954.pdf` is trap 9 exactly — a probe page
  of colour patches through the document's own embedded CMYK profile has `poppler`, `mupdf` and
  `ghostscript` agreeing to four levels because all three are Little CMS at its default
  *perceptual* intent, while Table 51, §8.6.5.8 and §11.4.7 each say the default is
  RelativeColorimetric; the page stays contradicted with the evidence beside it. `5589519.pdf`,
  filed as "`/DeviceCMYK` JPEGs" and so as 613's `6327765.pdf` again, is **not** that: a probe of
  plain `DeviceCMYK` patches puts this tree and `poppler` on the same values, `hayro` agrees with
  the other three about the page, and the disagreement is one shading pattern inside a soft mask
  landing in the page's default space instead of the mask's (§8.7.2, ADR 0456). −8.212 → +0.713.
  **A diagnosis written from a document's dictionary rather than from a measurement is a guess**,
  and the cost of this one was a silent defect carried for six rounds.
- **And `doc/todo/_image-codecs-and-the-sandbox.md` §7's `hayro-jbig2` bound is gone**, taken as
  the commit because no release carries it: `1653119.pdf` −35.695 → +0.012, `3375154.pdf`
  −16.417 → +0.032, `3252105.pdf` −6.390 → −0.215.

**What this chunk leaves: 51 944 crawled documents unranked**, in archive-sized pieces. Three
rounds running, the crawl's head has been a defect no curated corpus states — and this is the
first of the three where *every* row of the head was ours.

### 19. The chunk the six-hundred-and-nineteenth took: eight archives, and four ceilings that were not ours

**Taken: `0546`, `1284`, `2022`, `2760`, `3498`, `4236`, `4974` and `5712`, whole, 8000
documents**, on §16's instrument unchanged and *reused* — page one at 72 dpi against `pdftoppm`,
`mutool` and `gs`, every invocation explicit about the page box, ranked by our ink minus the
lightest live reference's.

- **Check the instrument, and check what the instrument needs.** Sixteen documents named by ADRs
  0438, 0448 and 0451 were re-measured before anything else, and seven came back *worse than
  before their fix* — `3252105.pdf` at −156.436 against 615's −6.390. Nothing had regressed:
  **`pdf-sandbox-worker` must be built into the same target directory as the example**, or every
  codec behind the sandbox refuses and the ranking measures a tree with no bilevel decoder. Two
  commands, not one: `cargo build --release -p pdf-model --example render_at` **and**
  `cargo build --release -p pdf-sandbox --bins`. With it built all sixteen reproduce to the
  thousandth.
- **Four defects of this tree, and each is a bound belonging to something else** — ADR 0454.
  `2022009.pdf` at **−84.152** is a blank sheet because `zune-jpeg`'s default 16384-row limit
  fired before this crate's own `MAX_SAMPLES`, on a scan of 28341 rows (§7.4.8 puts no ceiling
  anywhere). `3498294.pdf` at **−26.015** lost 63% of an inline image and tokenised 1.4 MB of its
  samples as operators, because a derived length the 64 KiB window was too short to *check* was
  dropped for the forward search (§8.9.7, §7.3.8.2). `4236390.pdf` at **−15.235** and
  `2022430.pdf` at **−12.618** were refused for a `/Columns` of 872 against a `/Width` of 869 —
  which is Table 11's own "the filter shall adjust the width … to the next multiple of 8".
  `0546308.pdf` at **−6.785** and `3498231.pdf` at **−7.131** lost every glyph of a font whose
  `/FontFile3` `OpenType` program has no `head`, which Table 124 exempts in as many words.
- **The reach is measured rather than argued**: all twenty-two ranked archives re-ranked whole
  after the fixes, and the rows that move are the documents the four fixes are about and nothing
  else. §7.4.6's, §7.4.8's, §8.9.7's and §9.9's rows.
- **The positive head is 613's finding and not ours** — `poppler` alone drawing almost nothing —
  read out of `doc/traps/oracle-and-references.md` rather than derived again. Two rows above it
  are the *other* half of that note: `1284136.pdf` +47.956 is `ghostscript` light, and
  `1284295.pdf` +28.502 is `ghostscript` rendering a different page box, which is trap 3 arriving
  as a size rather than as ink.

**What the head still holds**, each named so the next round does not re-derive it:

- **`hayro-jbig2` 0.3.0's flat 10 000-instance cap now has five documents of 22 000 waiting on
  it** — `0546561.pdf` −30.018 and `4974796.pdf` −15.417 join 613's `1653119.pdf` and 615's
  `3375154.pdf` and `3252105.pdf`. `doc/todo/_image-codecs-and-the-sandbox.md` §7.
- **Four silent rows this round diagnosed and did not take.** `2022794.pdf` −12.743 states
  **1451 `DCTDecode` images**, one of them 1400×2, which is `doc/todo/11`'s subject and 613's
  aerial photograph again. `4236552.pdf` −10.930 is **one command** — a single `DCTDecode` under
  an `ICCBased` space with an `/SMask` — at 182.784 against 193.714 / 195.641 / 194.921, which is
  trap 9's family. `4236836.pdf` −10.001 is a **text-only page** of five Type 1 subsets at 20.414
  against 30.415 to 35.758, with nothing reported, so it is glyph weight or a glyph absent
  silently — the one row of the head that is not about an image. `2022216.pdf` **+20.141** is
  twenty `/SMask`s and is the only positive row where the three references agree within 7 and we
  are 13 above the heaviest.
- **615's two remain**: `6696954.pdf` −10.252 and `5589519.pdf` −8.212.
- **44 rows of the 8000 produce no number**, the same three shapes 613 and 615 opened by hand.

**What this chunk leaves: 43 944 crawled documents unranked**, in archive-sized pieces. Four
rounds running, the crawl's head has been a defect no curated corpus states.

### 20. The check a chunk leaves behind, which is a file rather than a memory

**A fix found by ranking this crawl is measured once**, by the round that makes it, in a tree that
does not yet hold its neighbours' work. The corpus, oracle and quorra gates walk `doc/pdf.js` and
name none of these documents, so two branches touching no common line can defeat each other with
every gate green — which is what the six-hundred-and-twenty-third session found and the
six-hundred-and-twenty-fourth attributed (ADR 0458). Two rules follow, and **they are a program
rather than a habit, because the round that first stated them recorded them in its own history
file and in no other document, which is exactly the failure they are about**:

- **A round that fixes a document no gate covers appends a row to
  [`doc/checks/fixed-documents.toml`](../checks/fixed-documents.toml)** — the path, the page, the
  reports the page must and must not carry, an ink band, and the defect in one line with its ADR
  and clause. The file's own header says what each field means.
- **The merge round runs it**, because the merge is the only place the combination exists. It is a
  line in `doc/todo/02` §2, two commands, and about half a minute over the whole file:

  ```sh
  cargo build --profile gates -p pdf-sandbox --bins   # trap 10: nothing else builds it
  cargo test  --profile gates -p pdf-model --test fixed_documents -- --ignored --nocapture
  ```

**It is a regression check and not a second ranking.** A row goes in once its defect has been read
against a clause; a document at the head of a chunk with no diagnosis belongs in a section above,
not here. And the check needs no reference renderer at all — reports and this tree's own ink — so
it costs seconds and cannot drift under `poppler` having a bad day.

It is seeded with the documents sessions 603, 613, 615, 619 and 621 fixed, each *re-measured*
rather than copied out of a history file.
### 21. The chunk the six-hundred-and-twenty-fifth took: ten archives, and two extents

**Taken: `0669`, `0915`, `1530`, `2391`, `3129`, `4113`, `5220`, `6204`, `7311` and `7926`, whole,
10 000 documents**, on §16's instrument unchanged and *reused* — page one at 72 dpi against
`pdftoppm`, `mutool` and `gs`, every invocation explicit about the page box, ranked by our ink
minus the lightest live reference's. Both binaries built first, which is 619's note taken rather
than re-learnt, and all **26** documents named by ADRs 0438, 0448, 0451, 0454 and 0456 reproduce
to the thousandth before anything else was read.

- **Two defects of this tree, and both are about an *extent*** — where a thing ends, and which
  thing owns the answer. ADR 0459. `0669424.pdf` at **−7.223** refused three `/FontFile2` programs
  for a Flate stream that ends without RFC 1951's final block, each of which decodes to exactly
  its `/Length1`; §9.9's Table 125 states that length in **decoded** bytes, so the program is
  whole and what stopped short is the filter's marker. `4113230.pdf` at **−112.626** — the deepest
  row of the ten thousand, and silent — fills one path with two tiling patterns in turn and drew
  only the first, because `DisplayList::add_clip` interns and the second cell's box arrived
  carrying the *first* cell's identifier, which the copier read as a clip already in force.
- **The length is not the whole of the first condition, and the corpus said so before any gate
  did.** `issue13316_reduced.pdf` decodes to 168 808 bytes under a `/Length1` of 168 808 and draws
  **A C E F** where six CJK glyphs belong: its damage is `Corrupt` rather than `Truncated`, and
  only the second means every byte produced is the producer's. Both conditions are now asked.
- **The population of the first is a census over the whole crawl with an instrument that is not
  this tree's** (trap 8): **140 embedded font streams in 8 documents** end before the final block,
  the 138 `/FontFile2`s all reach their `/Length1`, and the two `/FontFile`s fall short of the sum
  of the three lengths — so `7557616.pdf` stays refused and the Type 1 arm has a real negative
  witness.
- **The reach is measured rather than argued**: all thirty-two ranked archives ranked whole before
  and after, **39 rows of 32 000 move**, and **two of the six documents the font fix moves are in
  archives an earlier chunk took** — `6696243.pdf` in 615's `6696` and `7680832.pdf` in 603's
  `7680`. That is the fourth round running that a fix has reached back. Four of the 39 are the head
  documents; **23 are tiling-pattern pages moving by at most 1.34**, eighteen toward agreement and
  five away by at most 0.64, every one silent and every one carrying more than one `PatternType 1`;
  and the other twelve are the instrument — nine with our own panel identical and a *reference*
  panel differing between runs, three with a panel absent from the earlier run.
- **The positive head is 613's finding and not ours** — `poppler` alone drawing almost nothing —
  read out of `doc/traps/oracle-and-references.md` rather than derived again; eight of the ten
  deepest positive rows are that shape, and `2391466.pdf` **+23.749** is the note's other half,
  `ghostscript` on a different page box (612 × 792 against 504 × 360).

**What the head still holds**, each named so the next round does not re-derive it:

- ~~**`7926872.pdf` −41.731 is a round of its own, and the clause has an answer nobody has used.**~~
  **Taken in the six-hundred-and-thirty-third — §22 below, ADR 0466.** The paragraph is kept because
  its reading is what the fix rests on.
  Its inline image is `/W 1200 /H 1790 /CS /RGB /BPC 8 /F /FlateDecode` with no `/L`, so
  `inline_image`'s answer 3 runs — the forward search the module's own comment calls "the one
  guess" — and the first `EI` token stands 24 822 bytes into 2.9 MB of Flate. 477 217 samples of
  6 444 000 are drawn and 1.4 MB of the photograph is tokenised as operators. §8.9.7 makes the
  bytes "a stream object's data" and every filter it admits states its own end-of-data, so a
  **filtered** extent is derivable rather than searchable. `pdf_syntax::Pump` already counts
  consumed input on its Flate engine and does not expose it; `DCTDecode` and `CCITTFaxDecode` do
  not go through it at all.
- **Five silent rows diagnosed no further than their numbers.** `6204475.pdf` −12.710 (209
  commands), `5220184.pdf` −8.911 (685), `3129942.pdf` −6.879 (8189), `0915159.pdf` −4.244, and
  two positives where all three references agree within 1.2 and we are far above them:
  `1530098.pdf` **+47.699** (ours 100.877 against 54.365 / 53.178 / 53.806) and `7926547.pdf`
  **+44.797**. None is called a family here: a structure count is evidence about where to look and
  never about who is right (trap 9).
- **`1530064.pdf` −15.950 is `doc/todo/49`'s** — `MAX_TILES` reached, and a stroke whose colour is
  a tiling pattern, which §8.7.3's ledger row already prices.
- **619's four and 615's two are still open**, and **65 rows of the 10 000 produce no number**, the
  same three shapes 613, 615 and 619 opened by hand.

**The re-runnable check for each document this chunk fixed**, because a fix on a document no gate
covers is invisible to a merge (623). `$R` is
`cargo build --release -p pdf-model --example render_at` plus
`cargo build --release -p pdf-sandbox --bins`, and `$C` is
`corpus-cache/safedocs/cc-main-2021-31`:

```sh
cargo run --release -q -p pdf-model --example open_one -- $C/0669/0669424.pdf 1
# expect: 26 commands, unsupported []          (was: 941 text operations lost to three fonts)
cargo run --release -q -p pdf-model --example open_one -- $C/6942/6942406.pdf 1
cargo run --release -q -p pdf-model --example open_one -- $C/6696/6696243.pdf 1
cargo run --release -q -p pdf-model --example open_one -- $C/4100/4100967.pdf 1
cargo run --release -q -p pdf-model --example open_one -- $C/7680/7680832.pdf 1
cargo run --release -q -p pdf-model --example open_one -- $C/3990/3990014.pdf 1
# expect for all five: unsupported []          (was: /FontFile2 decoded only as far as its damage)
$R $C/4113/4113230.pdf 1 1.0 /tmp/a.png && magick /tmp/a.png -alpha off -colorspace Gray \
  -format "%[fx:(1-mean)*255]" info:
# expect: 157.2 (three references 157.601 / 157.338 / 157.821); was 44.7
$R $C/0669/0669424.pdf 1 1.0 /tmp/a.png && magick /tmp/a.png -alpha off -colorspace Gray \
  -format "%[fx:(1-mean)*255]" info:
# expect: 10.1 (three references 14.217 / 9.997 / 14.160); was 2.774
```

And the negative twin, which is the one a merge is likeliest to lose because it is a page that
must **stay** blank — it is a gate, and named here so the pair is read together:
`cargo test -p pdf-model --test silent_fonts` covers `issue13316_reduced.pdf`.

**What this chunk leaves: 33 944 crawled documents unranked**, in archive-sized pieces. Five rounds
running, the crawl's head has been a defect no curated corpus states.


### 22. What the six-hundred-and-thirty-third took: §21's head, and the half of the population it leaves

**`7926872.pdf` and `4605499.pdf`**, both fixed, and the defect is §21's diagnosis carried out:
an inline image whose data is *filtered* and which states no `/L` had its end **searched for**
rather than **derived**, and a byte pair inside the compressed data that reads as a
white-space-delimited `EI` ended the image there. §8.9.7 makes the bytes "a stream object's data"
and §7.3.8.2 makes such data self-limiting, so the filter's own end-of-data marker is the answer.
ADR 0466; §8.9.7's and §7.3.8.2's ledger rows.

- **The population was measured before the change** (trap 11), by `examples/token_window_census`,
  which gained the comparison and keeps it: of **2 672 062** filtered inline images with no `/L`
  over 65 967 crawled documents, **17 in 5 documents end early**, costing 13.45 MiB of encoded data
  taken for content operators. **The curated corpora carry none at all**, so no gate in this tree
  could see it — which is why the two rows below exist.
- **`4605499.pdf` was not in §21's head**: its archive is in none of the ten that chunk took, and
  at ours 8.848 against 72.062 / 72.409 / 72.682 it is deeper than the row this round was sent
  after. Both now report nothing: 44.516 against 44.647 / 45.020 / 45.233, and 71.775 against
  72.062 / 72.409 / 72.682.
- ~~**Half the population is untouched and the size of it is the finding.**~~ — **taken in the
  six-hundred-and-thirty-fifth, §24 below.** The first filter of those 2 672 062 images is
  `CCITTFaxDecode` **1 272 430** times against `FlateDecode`'s 1 367 073, with `ASCII85Decode`
  23 018, `ASCIIHexDecode` 4 104, `DCTDecode` 3 778 and `RunLengthDecode` 1 655 behind them — and
  only `FlateDecode` and `LZWDecode` have a resumable decoder in this tree, so everything else still
  falls to the search. This bullet said three of the five need no decoder at all (`>`, `~>`, a
  length byte of 128); **none of the remaining three does**, and §24 has which clause says so for
  each. **A successor with a number attached**, and the number is above.
- **The two rows are in `doc/checks/fixed-documents.toml`**, which is §20's rule and the only gate
  that sees either document.
### 23. The chunk the six-hundred-and-thirty-first took: ten archives, and three ranges

**Taken: `0792`, `1038`, `1776`, `2145`, `2883`, `3621`, `4359`, `5097`, `5835` and `6573`, whole,
10 000 documents**, on §16's instrument unchanged and *reused* — page one at 72 dpi against
`pdftoppm`, `mutool` and `gs`, every invocation explicit about the page box, ranked by our ink
minus the lightest live reference's. **Fifteen minutes** for the ten thousand at sixteen workers.
Both binaries built first and §20's check run before anything was ranked (**25 checked, 0 absent,
green**), with 625's three recorded documents reproduced to the thousandth.

- **Three defects of this tree, and all three are one question: what range does a sample run
  over?** ADR 0464. None of them is about decoding; every one is about the arithmetic between a
  decoded byte and a colour or an extent.
  - **`5097148.pdf` −43.503**, the deepest row of the ten thousand, is 625's leftover with a
    sharper clause than that note expected. Its inline image is `/F [/A85 /Fl]` with no `/L`, so
    the forward search ran and stopped at the first `EI` the base-85 spells, 69 598 bytes into
    1.29 MB: **one command drawn** at 0.092 against three references agreeing on 43.9, with 1.4 MB
    of encoded samples reported as operators. §8.9.7 makes the bytes "a stream object's data" and
    §7.4.2 and §7.4.3 give two of Table 92's filters an end-of-data marker *in* the data, so the
    extent is derived rather than searched for — which is what the clause's own EXAMPLE writes.
    → −0.323, 328 commands.
  - **`4359750.pdf` +32.097** is a **positive** head row and the first chunk where the sharpest
    finding was on that side. A `/DCTDecode` photograph in a `/Lab` space, drawn as a solid black
    rectangle in silence: `convert_three` divided every sample by 255, where §8.9.5.2's map and
    Table 88's default for that space give a lightness in 0 to **100**. ADR 0448 fixed the
    `Indexed` half of the same hole one arm along; the other two arms had no witness until now.
    → +0.307.
  - **`0792405.pdf` −8.329** loses both its photographs to "the colour space takes 4 components
    but the codestream has 3". The codestream has four: it is a **bare** codestream, so the codec
    synthesises a three-channel space for it and calls the fourth channel opacity — a reading
    §7.4.9 sets aside where the dictionary states the space, and Table 87 makes moot where
    `/SMaskInData` is 0. → +0.576.
- **`spec-errata emit` over the clause family is what made the second of those a rule.** Errata
  Collection 3's Issue #293 adds a whole sentence to §7.4.3 that `check` had never named and could
  not: it compares the tree's *quotations* against struck passages, and this is a pure addition
  over text nobody had quoted. `doc/errata-read.md` carries the row.
- **The reach is measured over our own panel rather than by a whole re-ranking**, because that is
  exactly what a change to this tree can move and a reference's panel is not: **10 rows of 42 000
  move, six of them the fixes and four the instrument** — four renders that lost their budget while
  three other rounds were compiling, byte-identical under both binaries on a quiet machine.
  **Three of the six are in archives an earlier chunk took** (`4482` and `0423` are 615's, `7311`
  is 625's), which is the sixth round running that a fix has reached back, and all three move
  toward agreement — `4482885.pdf` **+11.288 → −0.840**.
- **The first fix's population is one document of 65 944**, measured with a walk that reads each
  codestream's own SIZ marker (trap 8). The row says so; what makes it worth having is that the
  clause is not that narrow.
- **What the head still holds**, named so the next round does not re-derive it: `5835546.pdf`
  −13.310 is `MAX_OPERATIONS`, `doc/todo/49`'s; `2883767.pdf` −7.159 reports §11.4.4's non-isolated
  group, `doc/todo/23`'s; **three silent rows** are `2883994.pdf` −10.134 (5236 commands),
  `1776488.pdf` −5.965 (198) and `5835193.pdf` −4.976 (120 513). Below +20 the positive head is
  613's `poppler`-draws-nothing note, and `5097568.pdf` +26.596 and `4359131.pdf` +20.057 are that
  note with **`mutool`** as the light one instead — ours within a level of the other two on both.
  **61 rows of the 10 000 produce no number.**
- **`7926872.pdf` −41.731 is still open and is now a *different* question from `5097148.pdf`.** Its
  filter is `/FlateDecode`, which states no marker in its data, so a textual end-of-data cannot
  reach it and what it needs is still `pdf_syntax::Pump`'s consumed-input count exposed. 625's five
  silent rows and `1530064.pdf` remain as §21 records them.

**What this chunk leaves: 23 944 crawled documents unranked**, in archive-sized pieces. Six rounds
running, the crawl's head has been a defect no curated corpus states.


### 24. What the six-hundred-and-thirty-fifth took: §22's other half, and the three that needed no decoder

**§8.9.7's filtered inline image now has a derived extent for every filter Table 92 admits.** §22
left `CCITTFaxDecode`, `DCTDecode` and `RunLengthDecode` on the forward search for a token-delimited
`EI`; each of the three states its end in its own framing and none of them needs a decoder to find
it. ADR 0467; the ledger rows for §7.4.5, §7.4.6, §7.4.8, §7.3.8.2 and §8.9.7.

- **The population was measured before the change and after** (trap 11), by
  `examples/token_window_census`, which now builds its **own** `Delimiting` from each image's
  dictionary rather than asking the code under test (trap 8) and reports per filter. Over 66 960
  documents that open of 67 213 and 926 308 pages: of **2 672 351** filtered inline images with no
  `/L`, 2 671 901 were answerable, **one ended early** and **1229 over-ran by exactly one byte**.
  Afterwards, **0 and 0 of 2 672 332**.
- **`CCITTFaxDecode` is the negative result and it is worth as much as the fix.** All **1 272 438**
  of them carry Table 11's end-of-block pattern, and all 1 272 438 already agreed with the search:
  nearly half the population was being guessed at correctly. No page moves; what changes is that
  the answer is derived.
- **The one early image is `7311536.pdf` page 9**, a `DCTDecode` inline image 163 bytes short of
  ISO/IEC 10918-1's EOI. **1.617 of 255 with eight array-operand reports → 54.406 with none**,
  against `pdftoppm` 57.99, `mutool` 57.61 and `gs` 62.44. A row in
  `doc/checks/fixed-documents.toml`, which is §20's rule.
- **The 1229 over-runs are one byte each**, which is §8.9.7 taking one white-space character off the
  data where a producer wrote two. Not a hazard, and counted apart from the early one so that a
  wide one would be legible if it ever appeared.
- **A number two roads use moved**: the census's largest single lexical object over 8.88 billion
  content tokens fell from **798.20 KiB to 390.16 KiB**, because the old figure was a string begun
  by the 163 bytes of JPEG that were being lexed. `doc/todo/14`'s road D sizes a reader's window
  against that.
- **What is left to the search is now the standard's own boundary**: §7.4.6's `/EndOfBlock false`,
  where the clause puts the end outside the data; a fax stream whose producer wrote no end-of-block
  pattern (the crawl has none); and `FlateDecode` data corrupt before its marker (446).
### 25. The chunk the six-hundred-and-thirty-sixth took: ten archives, and two entries a dictionary states

**Taken: `1407`, `1899`, `2514`, `2637`, `3006`, `3744`, `3867`, `4728`, `5343` and `5958`, whole,
10 000 documents**, on §16's instrument unchanged and *reused* — page one at 72 dpi against
`pdftoppm`, `mutool` and `gs`, every invocation explicit about the page box, ranked by our ink
minus the lightest live reference's. **13 minutes 28 seconds** at fourteen workers, at a load
average of 23 to 33 on a 24-thread machine three other rounds were also using; 9966 of the 10 000
produce a number and 34 do not. Both binaries built first and §20's check run before anything was
ranked (**29 checked, 0 absent, green**).

- **The negative head is the shallowest any chunk has produced** — deepest row −10.174, against
  613's −20.3, 619's −84.2, 625's −112.6 and 631's −43.5 — and the sharpest row of the ten
  thousand is on the *positive* side for the second round running. **Two defects of this tree, and
  both are the same sentence in two clauses: a dictionary states something about its own data and
  this tree would not read it.** ADR 0468.
  - **`3867366.pdf` +77.113**, the top row, silent, 135 commands: a product catalogue's cover drawn
    as its own complement — a green photographic background as dark purple, a black textured header
    as beige — at 146.044 against three references inside 0.75 of each other. Its two photographs
    are `/JPXDecode` CMYK images under `/ColorSpace [/ICCBased …]` with `/Decode [1 0 1 0 1 0 1 0]`,
    and the JPEG 2000 route consulted no `/Decode` array at all. §7.4.9's bullet is "[i]f
    ColorSpace is absent, then the Decode array shall be ignored unless ImageMask is true" and
    Table 87's own `/Decode` row states it the same way round: **the condition is `/ColorSpace`'s
    absence, not the filter.** The route now goes through §8.9.5.2's map like every other, which
    also gives it a space's own units — an `Indexed` index passed through, a `/Lab` lightness to
    100, which is ADR 0464's finding one route along. → **−0.449**.
  - **`3867363.pdf` −6.915**, *reported*: a full-page statistics report drawn as a blank sheet at
    0.225 against 7.278 / 7.139 / 8.299, with "font /F1's program has no outline for any of the
    3938 code(s)". Its `/FontFile2` is whole — it decompresses to exactly its `/Length1` — and its
    table directory says `loca` is 6510 bytes where 3255 long offsets need 13 020, while `hmtx`'s
    record carries the length `loca` should have had. The bytes at `loca`'s offset are a whole
    table, ascending and ending exactly at `glyf`'s length. `skrifa` finds a `loca` too short for
    `numGlyphs` and produces **no outline for any glyph at all**. `sfnt.rs` gains a third repair
    beside its two, on the same derivation: a file that states one fact twice can check itself.
    → **+0.059**.
- **The reach is measured over our own panel** (631's rule), before and after, over all **52
  archives any chunk has ranked plus the 243 documents the two censuses name — 52 043 documents**.
  **Nine rows move and every one moves toward agreement**, put in front of the three references
  afterwards. **Five are in archives an earlier chunk took** — `1530` and `3129` are 625's, `1038`
  is 631's, `6696` and `3375` are 615's — which is the seventh round running that a fix has reached
  back, and one more is in an archive no chunk has ranked at all. **One of the five is §21's own
  open lead**: `1530098.pdf`, listed there as a silent row "diagnosed no further than [its]
  numbers" at +47.699, is the `/Decode` defect and now sits at +0.487.
- **Both populations were measured before the change** (trap 11), with instruments that are not
  this tree's (trap 8). Of **99 031** `JPXDecode` image dictionaries over the 65 944 crawled
  documents, 98 490 state a `/ColorSpace` and **2298 state a `/Decode` array** — every one of those
  beside a `/ColorSpace` — over **200 documents**, of which 92 arrays invert; the curated corpora
  carry none. The `loca` walk inflates every `/FontFile2` in the crawl *and* in `doc/corpora` and
  `doc/pdf.js`: **62 short records over 6 documents of 66 920**, 59 of them whole tables, and the
  two curated documents are the *negative* cases — `bug868745.pdf` descends and `issue14618.pdf`
  runs outside the program, so the repair declines both and no gate can move.
- **`spec-errata emit` found nothing on this family**, which is worth recording because four of the
  last six chunk rounds found something. §7.4.9 carries one annotation group (Issue #29, "except
  for" → "excluding"), §8.9.5.2 none at all, and Table 87's three live errata on that page
  (#366, #215) touch `/BitsPerComponent` and its neighbours rather than `/Decode`.

**What the head still holds**, named so the next round does not re-derive it:

- **`1407194.pdf` −6.304 is an annotation question rather than an image one**, and the only row of
  this head that is neither. Seven commands, silent: a `/Text` annotation with `/Rect [0 542 400
  792]` and no `/AP`, whose synthesised icon this tree draws at the whole 400×250 rectangle while
  all three references draw a small fixed-size note at its corner. §12.5.6.4 says text annotations
  "shall not scale and rotate with the page; they shall behave as if the NoZoom and NoRotate
  annotation flags … were always set", and §12.5.3's NoZoom is about *magnification*; what a
  synthesised icon does with an oversized `/Rect` is the question, and ADR 0109 is where this tree's
  icon artwork was decided. Not settled here.
- **Seven silent rows below −4**, none of them called a family (trap 9): `2637357.pdf` −10.174
  (1485 commands, and the three references spread from 20.0 to 30.9), `1899493.pdf` −9.049 (218 182
  commands and 75 045 distinct clips), `1407825.pdf` −8.296, `1407822.pdf` −7.856 (95 commands, and
  the three references agree within 0.8), `3006401.pdf` −6.640 (1909 commands, references within
  1.1), `5958599.pdf` −5.078 and `3006323.pdf` −4.073.
- **`1899774.pdf` −4.904 is `doc/todo/49`'s** — `MAX_TILES` reached — as `1530064.pdf` was for 625.
- **The positive head above +40 is three rows and none is ours.** `2637210.pdf` **+94.713** has
  `poppler` and this tree together at 130.8 and 127.8 while `mutool` sits at 33.1 and `gs` at 45.1,
  which is 613's note with the light reference on the other side; `1899170.pdf` **+53.494** has all
  four renderers disagreeing (7.4 / 60.9 / 144.3 / 190.1); `2514746.pdf` **+42.812** the same
  (9.0 / 19.3 / 28.4 against ours at 51.8). Below +25 the head is 613's `poppler`-draws-nothing
  note verbatim — a band of pages where `poppler` is at 1 to 3 while `mutool`, `gs` and this tree
  agree within 3 — read out of `doc/traps/oracle-and-references.md` rather than derived again.
- **619's four, 615's two and 625's remaining silent rows are still open** as those sections record
  them.

**What this chunk leaves: 13 944 crawled documents unranked**, in archive-sized pieces. Seven
rounds running, the crawl's head has been a defect no curated corpus states.


### 26. The chunk the six-hundred-and-fortieth took: ten archives, and a condition nothing states

**Taken: `2100`, `3990`, `4100`, `4605`, `6081`, `6100`, `6942`, `7065`, `7188` and `7434`, whole,
10 000 documents**, on §16's instrument unchanged and *reused* — page one at 72 dpi against
`pdftoppm`, `mutool` and `gs`, every invocation explicit about the page box, ranked by our ink minus
the lightest live reference's. **14 minutes 12 seconds** at fourteen workers; 9957 of the 10 000
produce a number and 43 do not. Both binaries built first and §20's check run before anything was
ranked (**31 checked, 0 absent, green**), with six documents ADRs 0459, 0464 and 0468 name
reproduced to the thousandth. These are every remaining thousand-document archive but two.

- **The negative head is the shallowest any chunk has produced, for the second round running, and
  that is now a sequence** — deepest row **−8.860**, against 636's −10.174, 631's −43.503, 625's
  −112.626, 619's −84.152 and 613's −20.341. 25 rows of the 9957 are below −3 and 48 below −2.
  **What sits at the top of it is this tree's own scan conversion, documented and priced**, which is
  the first chunk of eight where that is true rather than a misread clause. ADR 0471.
- **One defect of this tree, and it was a *condition* rather than a reading.** `1407194.pdf`
  **−6.304**, silent, §25's own open lead: a book cover with a pale yellow sticky note **250 units
  square** over its top-left quarter, from a `/Text` annotation with `/Rect [0 542 400 792]` and no
  `/AP`. `annotation::anchored_icon` had made the right derivation in the two-hundred-and-sixty-fifth
  session — its own comment says "a fixed size, which is by definition not `/Rect`'s" — and then
  written it under `if subtype != b"Text" || !is_empty(rect)`. §12.5.6.4's "attached to a point" and
  its "shall behave as if the NoZoom and NoRotate annotation flags … were always set", §12.5.3's
  "shall always maintain the same fixed size on the screen" and Table 166's "defining the location of
  the annotation on the page in default user space units" say nothing about the rectangle's area;
  §12.5.5's algorithm, which does turn `/Rect` into a size, maps a **stored** appearance's `/BBox`
  and has nothing to map here. → **+0.032**. §12.5.6.15's and §12.5.6.16's icons are untouched
  because neither clause states either sentence.
- **The reach is measured over our own panel** (631's rule), before and after, over all **62
  archives any chunk has ranked plus every document `doc/checks/fixed-documents.toml` and the
  annotation census name — 62 009 documents**. **Four rows differ and no row differs for any other
  reason.** Three of the four are in archives an earlier chunk took — `1407` is 636's, `6573` and
  `2145` are 631's — the **eighth round running** that a fix has reached back. The sharper of the two
  visible ones is on the **positive** side: `6573247.pdf` **+8.264 → −0.172**, the same producer's
  note over a nearly blank page where the icon was most of the ink, against three references agreeing
  within 0.04. `7557734.pdf` +0.604 → +0.025; `2145632.pdf` moves by nine ten-thousandths, which is
  what the change costs where twenty-seven rectangles were already about the icon's size.
- **The population was measured before the change** (trap 11) with an instrument that is not this
  tree's (trap 8), and **it was wrong twice before it was believed** — a regular expression that
  `rc_annotation.pdf`'s `<p>Hello World!</p>` defeated, then a stream-blanking pattern that also
  matched the `stream` inside `endstream`. Over **67 193 files**: **185 `/Text` annotation records in
  67 documents, 80 of them in 18 documents with no `/AP`, and 7 in 6 documents with no `/AP` and a
  side over twenty units.** **The curated corpora carry not one of the seven**, so no gate could show
  this and none moves for the fix.
- **`spec-errata emit` found a live erratum, on the third clause of the family.** §12.5.6.4 carries
  no annotation and §12.5.5 none; §12.5.3 carries **Issue #34**, whose second half had never been
  read because the first already had a verdict — a pure addition, invisible to `check`: "When an
  appearance dictionary is not present, the rendered appearance will be implementation dependent."
  It is the standard saying that everything this tree constructs for an annotation with no `/AP` is
  the processor's. `doc/errata-read.md`.

**A second instrument, because ink had gone quiet**: `examples/open_one` over all ten thousand,
asking what this tree *reports* rather than what it draws. **101 documents of 10 000 report anything
at all on page one**, and two hold nine tenths of the reports — both damaged files where the
references do worse than we do. Worth keeping as a habit; the two questions have different blind
spots, and this one produced the successor below.

**What the head still holds**, named so the next round does not re-derive it:

- **`4605705.pdf` is `doc/todo/11` §7, which is new**: eight `/Contents` parts that decode cleanly
  and then into garbage, among which one `cm` has no inverse — and `render-cpu` refuses the **whole
  raster** for it, losing the 293 commands that did draw. The question is `Rasterizer`'s contract,
  not this document, whose content is noise (`poppler` refuses it; `mutool` and `gs` disagree by 78
  levels about what it says).
- **`6942935.pdf` −8.691 is `doc/todo/11` item 5 with a witness nobody had to construct**: a hymn
  sheet whose rules the producer draws as twelve abutting `.06 w` strokes, which §11.3.7.3's union
  composites to 0.52 of a pixel where their area is 0.72.
- **`7434231.pdf` −2.271, with the three references inside 0.018 of each other**, is §10.7.4's
  anti-aliasing departure on a TeX double box thinner than a pixel; **`6081615.pdf` −4.127** and
  **`4100532.pdf` +7.452** are `Image::area_averaged` against a decimating filter, one in each
  direction.
- **`4100873.pdf` −7.922 and `7188835.pdf` −5.129 are trap 9's family with the evidence beside
  them** — four-component `ICCBased` `DCTDecode` photographs, three references inside 0.3 and one
  colour library between them, and the files stating `/Intent /RelativeColorimetric`, which is Table
  51's default and ours. Not called a diagnosis (trap 9's last bullet); what would settle it is that
  bullet's probe.
- **`6100352.pdf` −8.860, the deepest row, is `doc/todo/49`'s** — `MAX_OPERATIONS` at 345 032
  commands — as `1899774.pdf` was for 636.
- **`7188579.pdf` +19.856 and `7188417.pdf` are not defects but the opposite**: truncated linearised
  files that all three references refuse or reduce to a 1×1 raster, where this tree draws what the
  bytes carry and reports the shortfall.
- **Below +16 the positive head is 613's `poppler`-draws-nothing note verbatim** — 39 of the 49 rows
  above +10 have `poppler` under a third of the heaviest reference. **43 rows of the 10 000 produce
  no number.**
- **619's four, 615's two, 625's and 631's remaining silent rows and 636's seven are still open** as
  those sections record them.

**What this chunk leaves: 3944 crawled documents unranked** — two thousand-document archives
(`7557`, `7803`) and eighty-one of twenty-four apiece. Eight rounds running, a fix has reached back
into an earlier chunk; this is the first where the crawl's head was *not* a defect no curated corpus
states.

**Those 3944 were taken in the six-hundred-and-forty-seventh and the crawl is finished**; §27.


### 27. The chunk the six-hundred-and-forty-seventh took: the rest of it, and what the whole crawl says

**Taken: everything that was left — archives `7557` and `7803` whole and all eighty-one
twenty-four-member archives, 3944 documents**, on §16's instrument unchanged and reused: page one
at 72 dpi against `pdftoppm`, `mutool` and `gs`, every invocation explicit about the page box,
ranked by our ink minus the lightest live reference's. **5 minutes 10 seconds** at twelve workers
on a machine whose load average was between 5 and 13; 3924 of the 3944 produce a number and 20 do
not. Both binaries built first and §20's check run before anything was ranked (**33 checked, 0
absent, green**), with the four documents ADRs 0438 and 0471 name reproduced against the
four-renderer instrument to the ten-thousandth before anything was read.

**With this the crawl is 65 944 of 65 944 ranked**, over nine chunks. There is no successor of the
form "n crawled documents unranked" and this file's header says so.

- **The negative head is deeper than the last two chunks' and is still made of this tree's own
  departures** — deepest row **−12.251**, against 640's −8.860, 636's −10.174, 631's −43.503,
  625's −112.626, 619's −84.152 and 613's −20.341. 7 rows of the 3924 are below −3 and 17 below −2.
  **Seven of the first eleven rows were read to a cause and four were placed but not settled**, and
  every cause found is a departure this project has already argued rather than a misread clause —
  which makes it two chunks running, and this one over the population's remainder rather than over a
  sample of it. The four unsettled are named at the end of this section so the next round does not
  re-derive where they sit.
- **One defect of this tree, and it was the *shape* of a refusal rather than its value.**
  `7803372.pdf` **−12.251**, the deepest row, reports `MAX_TILES` and nothing else: a school-canteen
  menu whose *Jeudi* and *Vendredi* columns are hatched by a `/BBox [0 0 1.6 1.6]` cell holding one
  8 × 8 one-bit image, wanting some twenty thousand sites apiece against a bound of 4096. The bound
  is ADR 0271's and its value is not what moved. What moved is that the check sat in **front** of
  the cell's interpretation, so a page that could afford four thousand sites was given none.
  §8.7.3.1 puts the requirement on the processor — "[w]hen performing painting operations such as S
  (stroke) or f (fill), the PDF processor shall paint the cell on the current page as many times as
  necessary to fill an area" — so a budget decides how many and not whether, and the affordable
  prefix is drawn now with the shortfall reported by name exactly as before. → **9.083 → 11.096**
  against references between 21.3 and 22.4. The other four fifths are `doc/todo/49`'s and stay
  there. ADR 0477.
- **The asymmetry was legible in the ledger the whole time.** §8.7.3.1's own row already records
  §7.8.2's prefix rule for the *cell's content stream* (ADR 0359) — a cell that decodes part-way is
  replicated as far as it got — while the *lattice* threw its prefix away. Two things make a tiling
  and the rule had reached one of them.
- **The rest of the head, each with its evidence**, and none of it new:
  - **ADR 0308's abutting marks, on a witness far stronger than 640's hymn sheet.**
    `7803184.pdf` **−6.381** and `7803350.pdf` **−6.639** are pages a producer states as thousands
    of thin image strips — **2217 `Do`s on the first, 1882 of them 0.96 units tall and stepped
    alternately 0.96 and 0.72**, each a 627-wide, 1- or 2-row `DCTDecode` band. Every strip covers a fraction of a device pixel row, §11.3.7.3 composites the
    fractions as a union — `1 − (1−a)(1−b)` — and the page comes out 25% of the way to white along
    a third of its rows, in seams a reference does not have because none of the three anti-aliases
    an axis-aligned image edge. **Measured rather than asserted**: the gap against `pdftoppm` is
    −6.381, −2.601, −1.011, −0.501 at scale 1, 2, 4 and 8, halving per doubling, which is ADR
    0308's boundary-over-area signature and nothing else's. `doc/todo/11` item 5.
  - **Trap 9's colour family, with the references disagreeing among themselves.** `7650021.pdf`
    −5.951 is a press-ready restaurant menu with registration marks and colour bars whose dark
    ground reads (69, 69, 71) here, (65, 64, 66) in `mutool` and (57, 53, 54) in `pdftoppm` — three
    answers, not two. `7557305.pdf` −3.988 fills its page from a `/Separation` whose alternate
    resolves to (15, 83, 143) on §10.4.2.5's own arithmetic against `poppler`'s (0, 75, 152) and
    `mutool`'s (0, 82, 139), which are thirteen levels apart from each other. Neither gap moves
    with resolution, so neither is scan conversion. Not called a diagnosis, on trap 9's last
    bullet.
  - **`Image::area_averaged` against a decimating filter**: `7557123.pdf` −2.526, a single 1200 ×
    1800 `DeviceCMYK` photograph under a Flate `/SMask` drawn into 288 × 432.
  - **A second `MAX_TILES` page**, `4650000.pdf` −2.456, which the fix above also moves.
- **The positive head is 613's `poppler`-draws-nothing note and almost nothing else**: 18 rows are
  above +10 and **10 of them have `poppler` under a third of the heaviest reference** while
  `mutool`, `gs` and this tree agree — `6150016.pdf` +20.192 with `poppler` at 3.148 against our
  23.339 is the shape. The exceptions are the other direction of the same question: `7557508.pdf`
  +16.686 and `7557287.pdf` +10.999 have **`mutool`** light while `poppler`, `gs` and this tree
  agree within 0.3. Read a positive gap as a question about which reference is light before reading
  it as ink of ours.
- **The four rows this entry used to leave "placed but not settled" are settled** — taken in the
  seven-hundred-and-ninety-second session with the instruments this bullet had named, and one of
  the four was a defect of this tree that is now fixed (ADR 0727 holds all four diagnoses with
  their probes):
  - **`7803013.pdf` −2.606** was ours: an embedded DFKai-SB subset whose glyph shapes are
    *computed by its TrueType instruction programs*, drawn from the uninstructed skeleton because
    every sfnt outline was drawn unhinted. The persistent −2.25 at 8× that would not converge was
    the shape difference. Fixed — the hint-reliant family draws through `skrifa`'s interpreter at
    one pixel per design unit — and the witness now agrees with both references to 0.02 ink at 8×;
    its row is in `doc/checks/fixed-documents.toml`.
  - **`7557015.pdf` −3.241** is not resampling: its photos agree with `pdftoppm` to 0.33, and the
    gap is a band of thousands of 0.227 pt white *stroked* diamond outlines — §10.7.4's
    anti-aliasing departure plus ADR 0308's composited boundaries (`doc/todo/11` item 5), already
    argued.
  - **`7557305.pdf` −3.988** is §11.4.7, not the colour conversion: on a probe built from the
    page's own `/Separation`-over-Lab, we, `poppler` and `gs` agree byte for byte. The page's
    *page-level group* states `/CS /DeviceCMYK`; that one entry reproduces our page colour
    exactly, while `poppler` and `mutool` do not composite in the stated group space and `gs`
    returns a fourth answer through its SWOP tables.
  - **`7557122.pdf` −6.295** is ADR 0510's darkest-few-percent ICC finding on the document's own
    FOGRA-family profile: near-black `ICCBased` fills where the three references — one Little CMS
    between them, trap 9 — sit at (9, 0, 0) and we at (19, 14, 13), while on plain `DeviceCMYK`
    patches of the same values we and `poppler` agree byte for byte. An argued position
    (ADRs 0456, 0484, 0510).

- **The reach is bounded by the code and confirmed by measurement**, and the two are worth keeping
  apart. The diff is entirely inside the `total > MAX_TILES` branch, which is the branch that raises
  the report, so a page that can move is a page that reports `MAX_TILES` — and `examples/open_one`
  over all **65 944** says that is **48 documents** over 35 archives. The confirming run is 631's
  rule over **8011** documents rather than the whole crawl, for that reason: this chunk's 3944, the
  four previously-ranked archives that hold such documents (`0100`, `1530`, `6204`, `7188`), all 48,
  and every row of `doc/checks/fixed-documents.toml`. **42 rows move and every one reports
  `MAX_TILES`**; a forty-third moved because a 30 MB document lost the harness's wall-clock budget
  at a load average over 100 and is identical to four decimals when re-measured alone, which is
  626's lesson on our own instrument instead of a reference's. ADR 0477 and
  `doc/history/647-*.md`.

### 42. What the eight-hundred-and-eightieth took: `batch5/FOP`, and a CID-keyed CFF whose Font DICTs are nowhere

**The directory, surveyed whole under the four rules** — twelve rayon threads, `--data 8
--tree 12`, 2.5 s, 0.99 GiB peak. The line, a baseline for this directory and never a ratchet:

| directory | documents | line |
|---|---|---|
| `batch5/FOP` | 808 | 2 unopenable, 0 locked, 1 encrypted beyond us, 0 pageless, 34 incomplete, 0 slow |

**The rate is the tracker's shape, and it is the lowest so far.** 34 of 808 is **4.2%** incomplete
— below `MOZILLA`'s 2.47% only, and below the pdf.js gate's 6.98% — because an Apache FOP issue
attachment is most often a document FOP *produced* rather than one it could not read: the two
unusable files are two members of one `.zip` with no usable cross-reference table and no object
header anywhere, and the one encrypted file is `/R 5`, which §7.6.4.2's Table 21 states no
algorithm for. Ranked by report, one document counted once per kind: 12 `Font`, 9 `Operator`, 5
`Annotation`, 5 `Text`, 4 `Image`, 2 `Shading`, one each of `NoninvertibleMatrix`,
`MissingResource` and `TransparencyGroup`. Six of the nine `Operator` documents are one producer's
habit — FOP 0.20.3rc writing `Tc` inside a `TJ` array, which §7.3.6 admits only objects into — and
the five `Annotation` documents are one file's five revisions, a `Widget` whose `/DA` names a font
its `/DR` does not define.

**Ranked by ink** — ours flattened on white against `pdftoppm -cropbox` and `mutool draw` at
72 dpi over all 34 incomplete pages, round 876's script — **the head is at the light end with both
references agreeing to a hundredth of a level**: `FOP-2736-4.pdf`, ours **0** against `poppler`
4.35 and `mupdf` 4.36, reporting that `/F20`'s program had no outline for any of the 1816 codes the
page shows. Then `FOP-304-2.pdf` at the dark end (ours 6.96, `poppler` 7.18, `mupdf` 2.65 — the
`Tc`-in-array file, where `mupdf` abandons the stream and `poppler` and this tree draw on), and
`FOP-2699-7.pdf` (ours 0, `poppler` 7.30, `mupdf` 2.05: a CFF whose `CharStrings` are missing,
which the three renderers do three things with). Fourteen rows are within a tenth of a level of
both references.

**The head is a font whose FDArray offset lands inside its own FDSelect.** The page is FOP's glyph
sheet of a Japanese OpenType font, embedded as a bare CID-keyed CFF by Apache FOP 2.3.0-SNAPSHOT's
subsetter; its charset is the identity over 7925 intact charstrings, and its Top DICT puts the
`FDArray` at offset 8001 where the format-0 `FDSelect` occupies 208 to 8133 — so every glyph
selects a Font DICT no reader can find, and Table 124's "shall conform to Adobe Technical Note
#5176" is the sentence the file breaks. ADR 0808: `pdf_font::cff::readable_font_dicts` re-encodes
the Top DICT with fixed-width offsets and appends a fresh `FDArray` — every Font DICT that reads
kept, every one a glyph selects that does not an empty Private DICT — and names the glyphs under a
replaced DICT that call a local subroutine it cannot hold, the one way a Type 2 outline depends
on its Private DICT; a page that shows such a code says so once per font, and a page that shows
none has lost nothing. The sheet draws at 4.38 between the two references, looked at glyph for
glyph; `FOP-2491-1.pdf`, the same subsetter with 9 of 13 glyphs calling local subroutines, went
from nothing drawn to four glyphs and *12 of the codes the page shows* said; the fixed-documents
gate holds the head at its own reading; and `doc/pdf.js`'s `issue9278.pdf`, whose two fonts hold
four Font DICTs with no Private DICT beside fifteen that read, is why the readable ones are kept —
the first draft replaced all of them and the corpus gate refused the regression. `batch5/FOP` is
**33 incomplete** after it.

**What is left here**: `batch5`'s other twenty trackers, `PDFIUM` (379) the largest; `batch4` once
its pieces land; [`40`](40-mask-chain-crop.md)'s clip cost; and
[`10`](10-bounds-that-cap-size.md)'s file-backed reader. In this tracker: `FOP-2751-3.pdf`,
`-7` and `-11` are one name-keyed CFF (`FuturaStd-Book`, Type1C) whose Private DICT's `Subrs`
INDEX is at a wrong offset — `read-fonts` answers `InvalidIndexOffsetSize(101)` at draw, and
nothing can be replaced there because the subroutines the charstrings call are what is unreadable
— which `mupdf` and `poppler` draw at 0.6 to 1.1 levels of ink through FreeType's tolerance and
this tree reports; and `FOP-2699-7.pdf`'s `MissingCharstrings`, a one-reference page.

### 43. What the eight-hundred-and-eighty-second took: `batch5/PDFIUM`, and a count that was never a cost

**The directory, surveyed whole under the four rules** — `--data 8 --tree 12`, 3.0 s, 1.30 GiB
peak, on the tree with round 879 merged; **10.0 s and 1.36 GiB after this round's change**, which
is what drawing the hatching costs and is recorded because a survey line that moved for a reason
is worth more than one that did not move. The line, a baseline for this directory and never a
ratchet:

| directory | documents | line |
|---|---|---|
| `batch5/PDFIUM` | 379 | 7 unopenable, 2 locked, 2 encrypted beyond us, 19 pageless, 66 incomplete, 0 slow |

**The rate is the tracker's shape, and it is the highest so far.** 66 of 379 is **17.4%**
incomplete — above `poppler`'s 11.5%, and for the same reason one step further: a pdfium issue
attachment is a fuzzer's output far more often than a document anybody wrote. The seven unusable
are one issue's seven files (`PDFIUM-325-0` to `-6`), each with no usable cross-reference table
and no object header anywhere; the two locked want a password; the two encrypted are one `/R 6`
whose crypt filter method §7.6.4.1 does not pair with and one `/R 5` (§7.6.4.2 Table 21 states no
algorithm for it); and the **19 pageless are sixteen files numbered `PDFIUM-1205-0` to `-1256-0`, one
issue after another, plus `-1023-0`, `-360-1` and `-512-0`**, every one of them "no first page" —
section 34's population again and not chased here. Ranked by report, one document counted once per kind: 17 `Content`, 16 `Font`, 15
`Text`, 11 `MediaBox`, 8 `Image`, 8 `Annotation`, 7 `Operator`, 5 `CompositedInParts`, 3
`Shading`, 3 `MissingResource`, 2 `TextKnockout`, 2 `LimitReached` (both `MAX_TILES`), one each of
`TransparencyGroup`, `PageDictionary` and `DamagedContentStream`.

**Ranked by ink** — ours flattened on white against `pdftoppm -cropbox` and `mutool draw` at
72 dpi over all 66 incomplete pages, round 876's script — **the head is at the light end with both
references agreeing to a hundredth of a level**: `PDFIUM-1122-0.pdf`, ours **70.92** against
`poppler` 80.51 and `mupdf` 80.52, reporting `MAX_TILES` and nothing else. Then `PDFIUM-407-0.pdf`
(ours 4.15, `poppler` 7.85, `mupdf` 7.48: a JPEG whose samples stop 561 bytes short and a
`/FontFile2` decoded only as far as its damage), `PDFIUM-1497-2.pdf` (ours 5.79, `poppler` 10.15,
`mupdf` 7.59: the second `MAX_TILES` page), and the three revisions of `PDFIUM-1475` (ours 9.01
against 9.91 and 9.95: a JPEG that begins with `ADBE`, and a CID font under `Identity-H` with no
program). `PDFIUM-1236-1.pdf` is ours 0 against 0.73 and 0.73 — a JBIG2 stream that ends early,
which both references draw the same prefix of. At the dark end, one-reference evidence only:
`PDFIUM-466-0.pdf` (ours 12.55, `poppler` 0, `mupdf` 76.5) and `PDFIUM-366-0.pdf` (ours 10.81,
`poppler` 0, `mupdf` 10.78). Seven rows have a missing panel.

**The head is a fill of 4480 sites under a bound of 4096.** A 10-unit cell over an A4 sheet's
frame is 4480 sites of a two-command cell, and `MAX_TILES` afforded every column and the whole
rows from the bottom that 4096 buys, so the top of the sheet was white. ADR 0271 had already concluded the count was the wrong
quantity — all 48 crawled documents that reached it terminate with it lifted — and kept it for one
measurement: an *empty* cell executes no operator and copies no command, so its loop ran the trip
count the file states, 3.6 × 10¹¹ for `/XStep 0.001` over 600 units. ADR 0810, three things and
none a count of sites: a cell with no marks replicated any number of times is no marks, so
`repeat_cell` does not enter the loop for one; a site is only copied where the fill's interior can
reach its cell — `pattern/reach.rs` scans the path onto the lattice a row at a time, and
`7680183.pdf`'s 249 hatched polygons fell from 539 729 sites to 112 499; and every other site is a
copy charged before it is made to the page's budget and to the tiling's own, `MAX_TILE_COPIES` of
65 536 *commands* — the cost in its unit, sixteen times the count at a one-command cell, chosen
after the alternative was measured: charged to the page's budget alone, `PDFIUM-1497-2.pdf`'s two
largest tilings took the whole four million and the frame and title block after them, eleven
seconds where the count took two. The sheet draws at 80.52 by the ranking's instrument, square for
square against `mupdf` at four times magnification; the fixed-documents gate reads 161.052 and that
is the band. ADR 0477's two crawl rows moved with it — `7803372.pdf`'s hatched columns from 11.1 to
18.9 by the gate's instrument, `4650000.pdf` from 49.2 to 57.3, inside its references' 43.7 to 62.9.
What still wants more than the copies budget is `2760154.pdf`'s 762 930 sites and
`PDFIUM-1497-2.pdf`'s 448 632, and what they want is a cell rendered once and replicated by the
rasteriser, which [`49`](49-restrictions-worth-re-examining.md) carries with both as witnesses.
`batch5/PDFIUM` is **65 incomplete** after it, `PDFIUM-1497-2.pdf` being the one page that still
reports a budget and reporting `MAX_TILE_COPIES` for it.

**What is left here**: `batch5`'s other nineteen trackers; `batch4` once its pieces land;
[`40`](40-mask-chain-crop.md)'s clip cost; [`10`](10-bounds-that-cap-size.md)'s file-backed
reader; and [`49`](49-restrictions-worth-re-examining.md)'s budget on commands that prices them.
In this tracker: `PDFIUM-1497-2.pdf`, the other `MAX_TILES` page, now `MAX_TILE_COPIES` on its two
largest tilings and [`49`](49-restrictions-worth-re-examining.md)'s witness; `PDFIUM-407-0.pdf`, a short JPEG and a damaged
`/FontFile2` where the two references disagree with each other by a third of a level; and
`PDFIUM-1236-1.pdf`, whose truncated JBIG2 both references draw a prefix of and this tree reports —
the same question ADR 0794 answered for CCITT, one filter over.


### 44. What the eight-hundred-and-ninetieth took: `batch5/sumatrapdf`, and a rectangle of no area that stopped marks it does not place

**The directory, surveyed whole under the four rules** — twelve rayon threads, `--data 8
--tree 12`, 3.1 s, 1.83 GiB peak. The line, a baseline for this directory and never a ratchet:

| directory | documents | line |
|---|---|---|
| `batch5/sumatrapdf` | 320 | 2 unopenable, 2 locked, 0 encrypted beyond us, 3 pageless, 17 incomplete, 0 slow |

**The rate is the tracker's shape, and it is the second lowest so far.** 17 of 320 is **5.31%**
incomplete — between `MOZILLA`'s 2.47% and `REDHAT`'s 6.07%, below the pdf.js gate's 6.98% and far
below `PDFIUM`'s 17.4% — because a SumatraPDF issue attachment is most often a document somebody
could not read rather than a fuzzer's output. The two unusable are a file with no `%PDF-` header in
its first 537 bytes and one with a table that is unusable and no object header anywhere; the two
locked want a password; the three pageless are section 34's population again and not chased here.
Ranked by report, one document counted once per kind: 10 `Font`, 6 `Text`, 2 `Operator`, 2 `Image`,
2 `Annotation`, one `TransparencyGroup`, one `Content`.

**Ranked by ink** — ours flattened on white against `pdftoppm -cropbox` and `mutool draw` at
72 dpi over all 17 incomplete pages, round 876's script — **the head is two rows within a hundredth
of each other, and the first of them was already answered**: `sumatrapdf-378-0.pdf`, ours **0**
against `poppler` 3.298 and `mupdf` 3.310, a page of 339 text operations whose `/FontFile2` is
`Corrupt` after 409 275 bytes. That is ADR 0459's decision exactly — Table 125's stated extent is
asked *and* the damage must be a truncation, because a `Corrupt` stream is the input violating the
filter's grammar and `issue13316_reduced.pdf` reaches its extent to the byte and draws **A C E F**
where six CJK glyphs belong — so it was **held by name**, with `-854-0` and `-1505-4` in the same
tracker being the same shape. The second row is the one this round took. Below them:
`sumatrapdf-1005-0.pdf` (ours 0, 1.06, 0.57: a JPEG whose headers begin `2020`),
`sumatrapdf-839-0.pdf` (ours 1.89, 3.60, 1.95: `BT` without `ET`, where the references disagree
between themselves), and `sumatrapdf-LINK-1532-1.pdf`, ours **61.3** against two blank references —
a `/Font` entry that resolves to §7.3.10's null. Eleven of the seventeen are within a tenth of a
level of one reference or the other.

**The head is a bounding box the writer got wrong, and a rule applied one branch too wide.**
`sumatrapdf-LINK-1618-0.pdf` is `pdfcomment.sty`'s demonstration sheet — ours 4.19 against 7.48 and
7.93 — and what the *page* showed, which the report did not, is that a line with arrowheads, a
strikeout, six underlines, two squiggly underlines and two highlights were drawn nowhere and named
nowhere. Every one of the thirteen states no `/AP` at all and a `/Rect` written as a point, with
its own geometry whole: `/QuadPoints`, `/L`, `/Vertices`. `annotation::decide` dropped them on
§12.5.5's arithmetic — the algorithm that scales a *stored* stream's `/BBox` onto `/Rect`, where a
scale onto no extent leaves no mark — applied to annotations that go through none of it, because a
construction is written in the page's own default user space and placed with the identity. ADR
0825: `Constructed::bounded` becomes `appearance::bounded_by_rect`, one list asked twice instead of
two answers to one question, and the six subtypes whose clause states their marks "in default user
space" — §12.5.6.7's `/L`, §12.5.6.9's `/Vertices`, §12.5.6.10's `/QuadPoints`, §12.5.6.13's
`/InkList`, §12.5.6.6's `/CL` — are no longer stopped by a rectangle that does not place them.
That the silence is a silence is checkable rather than assumed: §12.5.6.5's Table 176 gives a
*link's* `/QuadPoints` a fallback to `/Rect` in as many words and Table 182 gives a mark's none,
and Table 166's `/AP` row frees a writer from an appearance dictionary for exactly this `/Rect`, so
the standard anticipates the file. The page draws twelve of the thirteen and **names the
thirteenth** where it was silent (a cloudy `/BE` on the `PolyLine`, §12.5.4 and ADR 0106), moving
from 8.385 to 9.078 by the fixed-documents gate's instrument; it is a row in
`doc/checks/fixed-documents.toml` with a band of a tenth, because the twelve marks are worth 0.693
of a level on an A4 sheet. `examples/point_rectangle_census` is the population: over `batch5`'s
6119 files, 6074 open, **122 documents state an annotation whose `/Rect` covers no area, 1237 of
those annotations state no `/AP` at all, and 57 of those — over six documents — still state the
entry their own clause puts in default user space.** The other 1180 are `Link`, `Widget`, `Text`
and one `RichMedia`: subtypes `/Rect` places. `batch5/sumatrapdf` is **17 incomplete** after it,
the head still reporting its three held decisions.

**What is left here**: `batch5`'s other twenty trackers, `DSS` (243) and `ocrmypdf` (205) the
largest; `batch4` once its pieces land; [`40`](40-mask-chain-crop.md)'s clip cost;
[`10`](10-bounds-that-cap-size.md)'s file-backed reader; and
[`49`](49-restrictions-worth-re-examining.md)'s cell rendered once and replicated by the
rasteriser, which §8.7.3.1 NOTE 2 anticipates. In this tracker: `sumatrapdf-LINK-1532-1.pdf`, the
one row at the dark end with two blank references, whose `/Font` entry `F1` is a reference to an
object the file does not define — ADR 0789's side of §7.3.10, and this round did not open it;
`sumatrapdf-404-0.pdf`, a colour-key `/Mask` on a `DCTDecode` image, which is a named refusal and
0.36 of a level from the lighter reference; and `sumatrapdf-1550-0.pdf`, where the substituted face
this machine offers draws none of the 148 characters asked of it, which is
[`21`](21-font-substitution.md)'s question and not this file's.

## What the whole crawl says, now that all of it has been ranked

The paragraph this file has never been able to write, and every figure in it is this round's own
run rather than an earlier chunk's record.

**65 944 documents. 65 703 open** — 241 do not, which is 0.37% — and **65 659 have a first page**,
the other 44 opening onto a page tree that yields none. Over the 65 659, `examples/open_one` says
**720 report anything at all about page one** and **64 939 report nothing**: **98.9%** of this
crawl's first pages are drawn with no shortfall to name. That is a statement about what this tree
*reports*, not about whether the pixels are right — the ink ranking is the instrument for the
second question and the two have different blind spots (640's rule) — and it is the first time the
report instrument has been run over the population rather than over a chunk of it.

**What the 720 are made of**, by the first thing each names: 250 a font whose program has no
outline for the codes a page shows through it, 157 a transparency group this tree will not
composite as stated (128 of them the non-isolated-and-blending case alone), 103 an image — two
thirds of those a `/SMask` with a `/Matte` that cannot be un-pre-blended — 55 a resource budget, 51
text, 44 a `/Contents` part that decoded part-way or not at all, 19 a stroke whose colour is a
tiling pattern, 10 an annotation with no appearance and no geometry in its clause. **138 of the 720
report more than one thing.** The budgets are 48 `MAX_TILES`, 5 `MAX_FORM_DEPTH`, 4
`MAX_OPERATIONS`, one `MAX_STATE_DEPTH` and one `MAX_OPERANDS`, and **no page in 65 944 exhausts
the clip or soft-mask tables**.

**The ink distribution is this chunk's 3924 comparable rows and not the population's**, because no
round has held all nine chunks' rankings at once and this one did not re-rank the 62 000 against
three references to get one: **3637 rows inside ±1 of the lightest reference (92.7%)**, 34 between
−1 and −2, 17 below −2, 7 below −3, and 18 above +10 of which 10 are a light `poppler`. What *can*
be said across the nine is the shape of the head, because each chunk's is recorded in its own
section above: it has stopped being defects and become **decisions this project has already argued
and priced** — ADR 0308's conflation of abutting marks, §10.7.4's anti-aliasing departure,
`Image::area_averaged`, trap 9's colour question, and the resource budgets — and every one of those
has a `doc/todo` entry with a cost written down.

**So what this corpus is for has changed.** It was opened to find defects no curated corpus states
and the sections above record what each chunk found; what it produces now is *witnesses* for
departures that were already known — `7803184.pdf` is the strongest statement of ADR 0308 anybody has been able to
make, and nobody had to construct it. A round that wants a defect should take a corpus built to be
diagnostic (§1's argument); a round that wants to close a departure should come here for the page
that shows what closing it would buy.


### 28. The chunk the eight-hundred-and-thirty-fifth took: `pdf-differences` read as clauses

**Not a new population but the one §14 left owed**, and §4's own words for it: *the per-case gates
this corpus makes possible, one clause and one hand-built witness apiece, each with its expected
value derived rather than voted.* There was nothing else to take — the crawl is 65 944 of 65 944
ranked, the four submodule corpora are ranked, the oracle's seven verdicts are each held by name
and its undiagnosed queue is empty, so the round's fallback had no unexplained head either. What is
left in this file for a *population* is the 31 GB issue-tracker corpus and nothing else.

Every one of the 37 documents was rendered and read against the clause its own README quotes,
never against the picture beside it. **Fourteen cases agree** — ISO 32000-2's corrected ColorBurn
and ColorDodge edge cases (`cb` = 1, `cs` = 0 and their mirror), §8.6.6.3's out-of-range indices,
§8.4.3.6's negative dash phase, §8.5.3.2's degenerate line caps, §8.9.7's inline-image
abbreviations, §11.7.4.4's atomic fill-and-stroke, §9.4.4's negative font size, and the default
colour spaces both ways — and the four this file already records are as their ADRs left them.

- **The negative dash phase was measured rather than eyeballed**, because the corpus's own
  "incorrect" picture differs from the correct one only in which way a slope leans:
  `[ 20 0 0 10 10 ]` at phases 0 to −7 puts the leading edge of the first dash at device columns
  200, 202, 204 … 214, one unit right per unit of negative phase, which is `(−p) mod 2Σ` and
  nothing else. `[ 10 10 ] −1 d` moves one unit the same way and the empty array with a negative
  phase draws solid.
- **One case is a defect of this tree, on all three rasterisers, and it is fixed** (ADR 0762).
  `DegenerateDashing.pdf`'s two rectangle heights state §8.4.3.4's "ends *within* an on-dash" and
  §8.4.3.6's "coincides *exactly* with a join point" in one file: 200 × 45 has a perimeter of 490
  and finishes an on-dash at the lower-left corner, 200 × 44 has 488 and stops eight units inside
  one. Every dasher this tree draws through merged the first and last dash of a closed contour
  whenever both were on, so both drew the document's round join where one of them wants two end
  caps. `pdf_render::opened_where_a_dash_ends_at_the_close` is the rule and
  `render-quorra/tests/dashed_close.rs` holds all three backends to it.
- **`VerticalText.pdf` was the case left standing, and the eight-hundred-and-thirty-sixth session
  took it** (ADR 0763). `/Encoding /Identity-V` over a non-embedded `CIDFontType0` of
  `Adobe-Japan1`: the producer had already chosen the vertical-form CIDs, and a substitute reached
  through Unicode drew the horizontal brackets and the centred punctuation the corpus publishes as
  wrong. The displacement was right all along — `/DW2 [ 880 −1600 ]` puts the columns where the
  references do — so what was missing was a *form* rather than anything in clause 9's vertical
  metrics. §9.5 NOTE 5 leaves the substitute's shapes open, so the two halves of the route are
  published tables read for what they say rather than a picture matched: the collection's own
  Unicode `CMap` pair, and the face's `vert`/`vrt2` feature. `doc/todo/21` §7 carries what is left.
- **`TextClippingModeChanges.pdf` and `PageLabels-UX` are not raster questions** in the way the
  rest are: the first draws what §9.3.6's two paragraphs ask for as far as three read-throughs can
  establish, and the second is about what a *user interface* shows for §12.4.2's labels.

**What this chunk left was one gate rather than a population, and the eight-hundred-and-thirty-sixth
session added both.** `crates/pdf-model/tests/indexed_out_of_range.rs` holds §8.6.6.3's row of
patches and `inline_image_abbreviations.rs` gained a second test for §8.9.7's eight images. Two
things were learned in the writing and neither was in the sentence this paragraph replaces.

- **The indexed file's two rows do not match exactly and its README says they do.** The reference
  row writes `0.95 0.5 1 rg` for a palette entry of `F380FF`, and 0.95 × 255 is 242.25 against
  0xF3's 243, so three of the eleven patches are a level apart in any renderer that rounds. The gate
  therefore reads the **upper** row against the file's own lookup string and the clause, which is
  what "derived rather than voted" means here — comparing the rows would have gated a decimal
  literal. Trap 8's shape: a corpus states an invariant about itself and the invariant is not quite
  true.
- **`InlineAbbreviations.pdf` is not the copy `doc/pdf.js` already carries.** The two files are the
  same 15 125 bytes and differ in seventeen of them, every one inside a `/L` or a `/Length`: the
  corpus copy states 1276 and 201 where `issue14256.pdf` states 1240 and 197, and 1276 is where the
  `EI` actually is. So the two take *different routes* through `inline_image::data_extent` — one
  answered by §8.9.7's stated length, the other falling through to the first filter's own
  end-of-data — and the second gate is a second witness rather than a duplicate.

### 29. The chunk the eight-hundred-and-fifty-fifth took: the issue-tracker corpus, at last

**§28 said it in as many words — "[w]hat is left in this file for a *population* is the 31 GB
issue-tracker corpus and nothing else" — and five sections before it held the same offer open.
Taking it found first that the corpus is not where this file says it is.**

**Where it went.** The set was published by the DARPA SafeDocs programme in November 2020 as bug
attachments from 35 issue trackers of 32 PDF technologies, hosted on the Apache Tika regression
server at `corpora.tika.apache.org/base/packaged/pdfs/pdfs_202011/`. Apache received takedown
requests, disabled the server, and closed the question as `LEGAL-696` (resolved 2025-04-10); the
name is **NXDOMAIN** today, which is a fact rather than an inference — Google's resolver returns
`Status: 3` with `apache.org`'s SOA in the authority section. So the URL in `doc/test-docs.md` and
every offer in this file names a host that does not exist.

**The route that does work, and why it is trustworthy.** The Internet Archive holds the six
tarballs whole and serves byte ranges of them, and **each one is checked against the SHA-512
Apache published beside it**, which is what makes a copy from a third party evidence rather than a
guess: `batch6.tgz` came back 1 303 317 582 bytes matching
`bc90ca4ab204b8ea…d140a70c70d3306c` exactly, and `batch1.tgz` matched its own. The `if_` suffix is
what asks the Archive for the original bytes rather than for a rewritten page.

```sh
base=https://corpora.tika.apache.org/base/packaged/pdfs/pdfs_202011
curl -L "http://web.archive.org/web/2021id_/$base/batch1.tgz.sha512" -o batch1.tgz.sha512
curl -L "http://web.archive.org/web/2021id_/$base/batch1.tgz"        -o batch1.tgz
sha512sum -c <(printf '%s  batch1.tgz\n' "$(cut -c1-128 batch1.tgz.sha512)")
```

**Do not resume onto a failed attempt.** The Archive answers a share of requests with an nginx
`504` page, and `curl -C -` appends the next attempt's bytes to that page's, so the digest can
never converge; one attempt in this round spent eight retries on a 4.2 GB file that was wrong from
its first 160 bytes. Each attempt starts from nothing.

**What the six hold**, off the Archive's own listing of the directory: `batch1` 1.8G, `batch2`
4.2G, `batch3` 3.4G, `batch4` 5.0G, `batch5` 3.9G, `batch6` 1.2G — **19.5 GB compressed**, which is
where this file's "31 GB" figure comes from uncompressed. One directory per tracker inside each.

**Fetched and verified in the eight-hundred-and-fifty-fifth: `batch6` and `batch1`, 3.27 GB
compressed and 4.2 GB on disk.** `batch2` was attempted eleven times there and never matched its
digest; `batch3`, `batch4` and `batch5` were untried.

**Retried in the eight-hundred-and-fifty-seventh: `batch3` landed, the other three did not.** Six
attempts apiece, each starting from nothing. `batch3.tgz` came back on its fifth attempt at
3 606 571 824 bytes matching its published digest exactly; `batch5`'s six attempts were all the
Archive's 160-byte nginx `504` page; `batch2` and `batch4` each got one transfer that started and
stopped short, the rest error pages.

**The suspicion that the Archive's copy is *short* is not supported and should not be carried
forward** — `batch2`'s two truncated attempts stopped at two *different* lengths (4 201 315 360 and
4 201 109 128), which is a connection dropping rather than a file ending, and `batch3` came back
byte-exact through the same throttling. `HEAD` on `batch4` answers 302 naming its capture, so all
six captures exist.

**What to test next, stated as a hypothesis because n is 2.** The only two transfers that ever
started on a file larger than `batch3` stopped near the *same* size — 4 201 109 128 and
4 201 227 576, against stated sizes of 4.2 and 5.4 GB — while the one that completed is 3.6 GB. If
that is a ceiling on this route rather than a coincidence, then `batch2`, `batch4` and `batch5`
want HTTP `Range` requests in pieces rather than one `GET`; the Archive serves byte ranges, which
is how this recipe found the corpus at all. Try that before spending another six attempts.

**The population, in one line each** — a baseline for these directories, never a ratchet, and one
process per directory for the reason §1 gives:

| directory | documents | line |
|---|---|---|
| `batch6/cairo-gitlab` | 29 | 0 unopenable, 0 locked, 2 pageless, 2 incomplete, 0 slow |
| `batch6/evince` | 241 | 0 unopenable, 1 locked, 1 encrypted beyond us, 0 pageless, 14 incomplete, 0 slow |
| `batch6/poppler-gitlab` | 463 | 5 unopenable, 8 locked, 9 pageless, 54 incomplete, 3 slow |
| `batch1/PDFBOX`, eight shards | 3792 | 3 unopenable, 12 locked, 14 encrypted beyond us, 9 pageless, 275 incomplete, 2 slow |

**The rate is the finding about the population.** 275 of 3792 is **7.25%** incomplete and the
poppler tracker's 54 of 463 is **11.7%**, against **6.98%** for the pdf.js corpus and **1.735%**
for the whole 65 944-document crawl. A corpus assembled from bug reports is four to seven times
harder than the web, which is what §1 says such a corpus is for — and the two figures are the two
denominators `CLAUDE.md` separates, not one number twice.

- **One document does not finish, it is the head of both trackers, and it is the same file twice.**
  `poppler-gitlab/poppler-978-0.pdf` and `PDFBOX/PDFBOX-3688-0.pdf` are byte-identical —
  2 051 350 bytes, SHA-256 `3030bb601a45da4c426d9a77b166a77991ea230c9c0f478508bf516ffcda7618` —
  filed against two readers by two people. It **opens in 1.6 ms** and **interprets in 2.5 s into
  298 379 commands with nothing reported**; what does not finish is the rasterisation, and ten
  sampled stacks put nine of ten inside one call: `draw_group` → `PixmapMut::draw_pixmap`. Its page
  one states **73 047 transparency groups**, every one of them isolated, unclipped, carrying a soft
  mask and holding nine images, on a 1701 × 2409 target — so each group blits 4.1 M pixels and the
  page asks for some 300 **billion** of them. Measured: 115 groups a second, which is about
  **640 s** for the page, and both surveys' "slow" line is that document at 616 s and 602 s.
- **The two other "slow" documents in that survey are not slow**, and this is a note about the
  instrument rather than about them: `poppler-267-0.pdf` and `poppler-459-0.zip-1.pdf` take
  **260 ms and 68 ms** alone against 602 s and 601 s under the survey's 24-way load beside the
  document above. §1's own warning about the `slow` count, met again.
- **The obvious fix was built and disproved by measurement, which is why it is written down here
  rather than shipped.** `render-cpu`'s `marked_rows` already answers "which rows of the surface
  can this command list mark", and `build_soft_mask` already uses it; narrowing a *group's* band by
  it is three lines and is exact — outside its own marks an isolated group's buffer is the
  transparency it was allocated as, and all three operators a group is blitted under (a separable
  blend under source-over, `DestinationOut`, `Plus`) leave a destination alone under a transparent
  source. It buys **nothing here**, because every one of the 73 047 groups' nine images spans the
  page's full height: `marked` comes back as the whole 2409 rows, 4497 times out of 4546 sampled.
  And it buys nothing on the two documents `doc/todo/40` prices for exactly this shape either —
  `0423548.pdf` 2.35 → 2.37 s and `3990833.pdf` 2.78 → 2.79 s, three samples each, which is noise
  in the wrong direction. Reverted on `CLAUDE.md`'s own rule that an optimisation is justified by a
  benchmark.
- ~~**So what this page needs is a bound, and this round did not invent one.**~~ **Taken in the
  eight-hundred-and-fifty-sixth** (ADR 0780), in the order this paragraph asked for. The measure is
  cumulative **group blit pixels** — `pdf_render::group_blit_demand`, read off the display list and
  drawing nothing — and the constant is `MAX_GROUP_BLIT_PIXELS`, sized by
  `examples/group_blit_census` over the three populations the way `MASK_BUDGET` was sized, with
  `BackendError::GroupsTooCostly` as `GroupsTooDeep`'s sibling and all three backends asking at
  five call sites. Two things the census settled that the paragraph could not: the bound has to be
  **absolute** rather than a ratio to the target, because wall clock tracks the product and a page
  demanding 660 repaints of a small sheet draws in 0.2 s while one demanding 301 of a large one
  takes 11.2; and the population's tail is a cliff rather than a slope — the heaviest first page
  that is *not* this one demands 23.08 G pixels against its 299.3 G. This document is a row in
  `doc/checks/fixed-documents.toml` now, on that file's new third form: `ink = refused: <words>`.
- **The report that misstated the file, which is the defect this chunk yielded and it is fixed.**
  §7.8.3's *no `/Font` resource named `/f-0-0`* was printed for a page whose `/Font` names all six
  of its fonts and whose objects the bug report's reduction removed — §7.3.10's null, a different
  clause and a different producer's mistake. ADR 0779 has the split and its three populations; the
  974 do not move and twenty-two reports in this corpus do.
- ~~**The claim that has held for six populations breaks here, and it breaks softly.**~~ **Read in
  the eight-hundred-and-fifty-seventh, and both of the numbers above were wrong.** The population
  is **28** documents rather than sixteen — the four directories' own survey lines add to it — and
  **13** of them get a page count out of `pdfinfo` rather than eight. What matters more is that the
  count was the wrong instrument: **`pdfinfo`'s `Pages:` is the page tree's `/Count`, not a page**,
  and this tree prints the *same* number for ten of the thirteen, because `Pages::len` reads
  `/Count` too. Asked to draw page one at 36 dpi instead, poppler answers a 1×1 image or a blank
  US-Letter sheet for eleven of the thirteen — the `/Kids` those trees name are objects the bug
  reports' reductions removed, so neither reader has a page and neither is wrong. **Two carry ink,
  and those two were ours**: `PDFBOX-4777-0.pdf` and `PDFBOX-4777-1.pdf`, encrypted files written
  entirely with cross-reference streams whose `startxref` is nineteen bytes short, where the
  rebuild searched for a `trailer` keyword §7.5.8.1 forbids such a file to carry, lost `/Encrypt`
  with the rest of Table 15, and handed back ciphertext with nothing reported. Fixed and pinned
  (ADR 0781). Of the remaining eleven, three disagree with `pdfinfo` on the count and none of the
  three is ours: `cairo-274-0.pdf` writes its catalogue's header as `1 0 Ybj`, `cairo-51-0.pdf`
  writes `/Type` as `\x1dType` inside it, and `poppler-732-0.pdf` has no `%PDF-` header anywhere.
  **So the sentence was right to be withdrawn and is true again now**: two documents did fail to
  open for a reason that was this tree's, they are fixed, and the rest are the files' own. What
  the episode is really about is the instrument — a page *count* cannot answer "did the other
  reader open this", and asking it to is trap 9's shape. Ask for a raster.

**What this corpus is for, now that a chunk of it has been taken.** It is the *diagnostic* kind by
§1's ranking — every file is an attachment somebody filed because a reader got it wrong — and its
directories are named for the reader that failed, which the crawl's hash buckets are not. A round
wanting a defect should take another tracker; a round wanting a rate should not, because a bug
tracker's rate is a fact about bug reports.

### 30. The chunk the eight-hundred-and-fifty-seventh took: `batch3`, which is one tracker

**`batch3.tgz` landed and verified**, 3 606 571 824 bytes against Apache's published SHA-512
`a00bbb1e97f101db…c15cbd1b`, on the fifth attempt of a day whose first four came back as the
Archive's 160-byte `504` page. It holds **one directory**, `MOZILLA` — the Firefox bug tracker —
with **6835 documents**, more than `batch1`'s whole `PDFBOX` shard set.

Surveyed as eight shards, one process each, §1's method. **The shard directories are symlink
farms and they are deleted after the walk**, which is not tidiness: `--bin undenominated` counts
the PDFs on this disk, and a farm left behind by `batch1`'s walk was making the issue-tracker
corpus report 21 987 documents where it holds 11 360. Re-making one is a shell loop.


| directory | documents | line |
|---|---|---|
| `batch3/MOZILLA` | 6835 | 5 unopenable, 1 locked, 3 encrypted beyond us, 17 pageless, 169 incomplete, 36 slow |

**The rate is the finding, and it goes the other way from §29's.** 169 of 6835 is **2.47%**
incomplete, against **7.25%** for `PDFBOX`, **11.7%** for `poppler-gitlab`, **6.98%** for the
pdf.js corpus and **1.735%** for the whole 65 944-document crawl. So "a corpus of bug attachments
is four to seven times harder than the web" is a statement about *which* tracker: PDFBOX's and
poppler's attachments are reduced and fuzzed test cases somebody built to break a parser, and
Firefox's are documents a person found in the wild and could not read. **The Mozilla directory is
nearer the web than it is to its own corpus's other three directories.**

- **The claim §29 could not repeat holds here without qualification.** Of the 22 documents this
  tree cannot open or finds no page in, **not one gets a page count out of `pdfinfo`** — checked
  on every one. Fourteen of the seventeen pageless are one zip attachment,
  `MOZILLA-552567-0.zip-*`, so the seventeen are five bug reports.
- **The bound `MAX_FORM_DEPTH` is reached by seven documents, and all seven are cycles.** ADR
  0271 established that over the crawl by lifting the bound sixteenfold to 256 and finding all
  four of its witnesses still reached it; the same experiment over these seven gives the same
  answer, so the claim now rests on eleven documents in two corpora and neither of them holds a
  legitimate one. The constant's own comment says so, with both denominators named.
- **`MAX_TILES` is reached by five, which is the departure ADR 0271 already documented**: the
  population is legitimate hatching, the count is the wrong quantity, and `affordable_span` spends
  the sites it affords rather than discarding them. More witnesses, no new question.
- **The 169 reports are the ones this project already owns.** 85 are a glyph a document's own
  embedded subset does not contain, closed by decision; 16 are `Identity-H` over a descendant
  with no embedded program, which is [`21`](21-font-substitution.md); 58 are §11.4.7's
  non-isolated and knockout groups, which is [`23`](23-transparency-departures.md); 17 are a
  resource name the dictionary does not define. The 181 `Operator` reports are one garbled token
  each, in files whose content streams are damaged.
- **The `slow` count is again mostly the instrument**, and this is the third chunk to say so:
  four of the 36 were timed alone and two are not slow at all — `MOZILLA-1296262-0.pdf` 1.6 s and
  `MOZILLA-1120249-0.pdf` 0.5 s against 45.0 s and 43.2 s under eight-way load.
- **Two are genuinely slow, and the first is the best witness [`40`](40-mask-chain-crop.md) has
  ever had.** `MOZILLA-831621-14.pdf` opens in 2.1 ms and interprets in 414 ms into **3166
  commands referencing 3149 distinct clips** — very nearly one clip apiece — and then spends
  **41 seconds** rasterising them onto a 1280 × 800 target, reporting nothing. That is the chain
  arithmetic that item prices, on a page where the chain is the whole page.
  `MOZILLA-892314-0.pdf` is the other shape: **162** commands, 83 clips, and an 8646 × 3544
  target, 32 s. Neither is diagnosed further here; the first is handed to `40` and the second is
  a size rather than a structure.

### 31. ~~What the eight-hundred-and-fifty-seventh leaves: a recovery whose condition is not its comment~~ — **taken in the eight-hundred-and-fifty-eighth** (ADR 0782)

**The `len()`-versus-`/Count` question is settled in writing and the code matches the reading.**
Table 30's own cell makes `/Count` "redundant" and the `Kids` arrays and their descendants what
"definitively determines the number of descendant pages", and the `shall` keeping the two
consistent is the **writer's** — so a node with no reachable descendants has no pages, whatever
integer the entry holds. `Pages::new` now probes the tree for a page (`reaches_a_page`, which is
`count_leaves`'s walk stopped at the first leaf) before believing a `/Count` it has not walked, with
the two conditions ordered by cost so a well-formed document pays one leftmost-spine descent — the
descent `get(0)` performs anyway. Measured either side under callgrind on `pdf-retrieve document`:
**+5120 instructions on a 14-page document and −75 323 on ISO 32000-2's 1023 pages**, and three
`--trace` launches either side inside one instrument's spread.

**Two halves of the reading are worth carrying forward, because both were nearly got wrong.** The
recovery runs where the tree yields *no* page and never where it yields fewer than `/Count` claims:
a tree that produced one page of five has stated an order and a set, and a scan's ascending object
numbers would substitute an invented order for a stated one (trap 5's additive-or-substitutive
test). And `len()` moves only where the recovery *found* pages: where it finds none, nothing has
been examined, the number of descendants is unknown rather than nought, `/Count` stands as the one
statement in evidence, and the page is asked for and refused **out loud** — which is louder than a
document reporting no pages, because such a document has nowhere to put a report at all. The round
wrote the other rule first and `viewer-core`'s
`objects_lost_inside_a_damaged_object_stream_are_said_out_loud` caught it.

**One of the four witnesses drew, and the other three are a different defect — which is a finding
about the instrument that named them.** §31's table above was built from a byte search for
`/Type /Page`, and a byte search cannot tell a page object *the tree cannot reach* from a page
object *nothing can parse*:

| document | what it is | after |
|---|---|---|
| `batch1/PDFBOX/PDFBOX-4623-1.pdf` | object 2 is its own `/Kids` entry; object 3 is a whole page | **draws** — *Hello World* at 48 pt, ink 1.315, pinned in `doc/checks/fixed-documents.toml` |
| `batch1/PDFBOX/PDFBOX-4339-0.pdf` | object 3's body opens `\xbc` where `<<` belongs | **still refused**, and now for a stated reason: `\xbc` is a regular character, so §7.2.3's keyword run is `obj\xbc` and §7.3.10's header does not lex — there is no object to take a prefix of (ADR 0784) |
| `batch6/poppler-gitlab/poppler-742-0.pdf` | object 8's `/TrimBox` array never closes and runs into the stream after it | **draws**: seven entries whole, the producer's own sheet, blank because `/Contents` is past the damage, reported (ADR 0784) |
| `batch6/poppler-gitlab/poppler-750-0.tgz-0.pdf` | object 14's `/ProcSets` array is unterminated, and it writes `/Con\x91ents` and `e@dobj` | **draws**: one entry whole, so this reader's default sheet with nothing on it and both sentences said (ADR 0784) |

**The rule that separates them is `doc/habits.md`'s and it is cheap**: run the reader and the grep,
and know that the reader is the instrument under test. Running it turned a claim about four
documents into a claim about one, and the other three are a *page object* defect rather than a page
*tree* defect — a different item, unread, and the question it would ask is whether §7.3.7's
dictionary has a prefix worth drawing, which is trap 5's test again on a third population.

### 32. What the eight-hundred-and-fifty-eighth leaves: three batches, and a route that works

**§29's hypothesis is confirmed: HTTP `Range` requests get past whatever stops a whole-file
transfer near 4.2 GB.** `batch2`'s pieces come back as clean `206`s at exactly the length asked
for, 512 MiB at a time, against a `GET` of the whole file that stopped short eleven times across
two rounds. The Archive's throttling is unchanged and is the whole cost: a piece takes between one
and a dozen attempts, every failure being the 160-byte nginx `504` page or a 107-byte `502`, and a
successful piece is discarded and re-fetched rather than resumed onto (§29's rule, unchanged).

The recipe, which is the thing to keep:

```sh
base=https://corpora.tika.apache.org/base/packaged/pdfs/pdfs_202011
capture=$(curl -sI "http://web.archive.org/web/2021id_/$base/batch2.tgz" \
          | grep -i '^location:' | tr -d '\r' | awk '{print $2}')
total=$(curl -sI -r 0-0 "$capture" | tr -d '\r' | grep -i '^content-range:' | sed 's|.*/||')
# then 512 MiB pieces: curl -r "$start-$end" -o piece --max-time 1800 \
#      --speed-limit 8192 --speed-time 120, retried from nothing until the length is exact,
# then cat the pieces and check the published SHA-512.
```

**Resolve the capture URL once and range-request *that*.** `web/2021id_/` answers a `302` naming
the capture; a `Range` on the redirect works but pays the redirect on every piece.

**What landed, and what is left, is `ls corpus-cache/tika-issue-tracker/` rather than a sentence
here** — the round's own history file records what this one got. What is worth carrying is that the
route is proven and the remaining cost is patience: about 512 MiB of the Archive's throttling per
successful piece, and `batch2`, `batch4` and `batch5` are 9, 11 and 8 pieces.

### 33. What the eight-hundred-and-sixtieth took: `batch2`, and the page object that will not parse

**`batch2` was fetched by the round before and extracted by this one**, verified against Apache's
published SHA-512. `ls corpus-cache/tika-issue-tracker/` is what says which batches are on this
disk; `batch4` was still being fetched while this round ran and was left alone.

**§31's remaining three witnesses are two defects, and one of them is closed.** The question §31
handed on — whether §7.3.7's dictionary has a prefix worth drawing — is answered in ADR 0784, and
the answer is neither *draw it* nor *refuse it*: the clause states no extent for a dictionary
beyond its closing `>>` and states that the written order is not information, so the entries whole
before the damage are a **subset** of the producer's rather than the dictionary. `Document::get`
still refuses the object outright; a second door hands the subset to one consumer, which takes it
only where those entries themselves state Table 31's `/Type /Page`.

**The population was measured first and it is several defects rather than one.**
`crates/pdf-model/examples/standing_count_census` over `batch1`, `batch2`, `batch3` and `batch6`
prints it; what is worth carrying here is the *shape* rather than the counts, which that command
prints: a small fraction of a per cent of documents state a page count this reader produces no page
for, most of them because **no object in the file declares `/Type /Page` at all** — for which the
standing `/Count` and a refusal out loud is the right answer and there is nothing to recover from —
and a minority because a page object's dictionary opens and then stops. All of the second kind now
draw. Three are pinned in `doc/checks/fixed-documents.toml`.

**One thing this found that was not the subject**: `xref::scan_for_objects` keeps an offset only
where the whole object parsed, so in a rebuilt file a damaged object is not merely unparsed but
*unnamed* — invisible to `object_numbers()` and to `object_headers()` alike. Anything else that
wants to ask a question about an object that will not parse has the same problem, and
`Document::damaged_dictionaries` is the only answer to it in the tree.

### 34. What the eight-hundred-and-sixty-first read: the eleven that declare `/Type /Page` nowhere

**§33 left a majority and nobody had asked what was in it.** `examples/standing_count_census` split
the standing-count population by cause and the largest cause was *no object whose bytes declare
`/Type /Page`* — which ADR 0782's standing count and a refusal out loud already answers correctly,
so nothing had opened the files. Eleven documents, and reading each against §7.5 and §7.7.3 turns
one cause into five.

**The census prints the account now rather than this file stating it.** A byte scan for
`/Type /Page` can only ask about an object's *own* declaration, and every one of these eleven has
stopped making one; the question that separates them is what the page tree's own `/Kids` names,
which is the file's statement about an object made in a place the object's damage cannot reach —
§7.7.3.2, "[t]he children shall only be page objects or other page tree nodes". So the census
follows every `/Kids` array of every object that parses, and prints, for each named object that
does not resolve, how far it reads and the keys of the entries read whole before the damage.

```sh
cargo run --profile gates -p pdf-model --example standing_count_census -- corpus-cache/tika-issue-tracker/batch{1,2,3,6}
```

The five causes, and what a conforming reader can honestly do with each:

- **The page objects are not in the file at all** — `PDFBOX/PDFBOX-3870-0.pdf`, whose `/Kids` names
  objects 6 to 10 and whose whole body between them is missing, and `PDFBOX/PDFBOX-3894-0.pdf`,
  whose `/Kids` names 8 and 14. Both are reductions rather than corruptions: the cross-reference
  table's own offsets run past the end of the file (42 461 into 37 934 bytes; 10 109 into 2316).
  **Beyond any additive recovery**, and the word *additive* is the whole of it — there are no bytes
  to read, so every entry of every page would be this reader's invention. ADR 0782's standing
  `/Count` with a refusal out loud is already the right answer, and it is the right answer for the
  same reason the recovery exists: the number of pages is unknown rather than nought.
- **No object header lexes where the page should be** — `PDFBOX/PDFBOX-4452-0.pdf`, whose object 3
  opens `3 0 obj5<`. §7.2.3 makes `5` a regular character, so the keyword run is `obj5` and
  §7.3.10's header does not lex; the single `<` after it opens a hexadecimal string. **Beyond
  additive recovery**, and for exactly the reason ADR 0784 refused `PDFBOX-4339-0.pdf`: reading an
  object there means deciding that `5<` was meant to be `<<`.
- **The dictionary opens and the first entry's value is already unreadable** —
  `GHOSTSCRIPT/GHOSTSCRIPT-698887-0.pdf` (`/Pare R`, where `R` is a keyword and no valid object
  begins with one) and `GHOSTSCRIPT/GHOSTSCRIPT-699695-1.pdf` (the `/Type` key *and* its value
  overwritten with `0xFF` bytes). The prefix is **zero entries**, so §7.3.7's subset is empty and
  says nothing about the object at all. **Beyond additive recovery as the prefix rule stands** —
  and 698887 is the file that names the next question, because its `/Type /Page` is in the file,
  in plain sight, four bytes past the damage. Taking it would mean resynchronising to the next
  `/Name` after an unreadable value, which is a *guess about the value's extent* rather than a
  prefix; the argument for and against is [below](#the-question-these-eleven-hand-on).
- **The tree names the object, its dictionary is damaged, and the prefix holds an entry only a
  page object states** — five documents, and this is the finding.
  `GHOSTSCRIPT-698991-0.pdf` reads `[Resources Contents]`, `GHOSTSCRIPT-699018-0.pdf` `[Annots]`,
  `GHOSTSCRIPT-699521-0.pdf` `[MediaBox Parent Contents Resources]`,
  `GHOSTSCRIPT-701846-0.pdf` `[Annots CropBox MediaBox Parent]` and
  `poppler-gitlab/poppler-192-0.pdf` `[Contents CropBo\x8c]`. **A conforming reader can produce
  something honest here, and the argument is the standard's own, twice**: §7.7.3.2's `/Kids` says
  the object is a page object or a page tree node, and §7.7.3.4 says which entries a node may
  legitimately carry beyond Table 30's four — `Resources`, `MediaBox`, `CropBox` and `Rotate`,
  because those are inheritable — so a prefix stating `/Contents` or `/Annots` was written by a
  producer describing a **page**. That is a different door from ADR 0784's, which takes a prefix
  only where the prefix *itself* declares `/Type /Page`, and it is a different door on purpose:
  0784's consumer finds its candidates by scanning the whole file, where an object that says
  nothing about itself could be anything, and this one is handed its candidate by the tree.
  `GHOSTSCRIPT-699521-0.pdf` is the best of them — 795 × 842, `/Resources` with Helvetica, and a
  `/Contents` that is `ASCIIHexDecode` of `BT /F1 30 Tf 350 750 Td 20 TL 5 Tr (Hello world) Tj ET`
  — and its damage is a *second* `/MediaBox` whose value is the bare keyword `e`.
- **The tree names the object, its dictionary is damaged, and the prefix discriminates nothing** —
  `poppler-gitlab/poppler-355-0.pdf`, whose prefix is a garbled key, `WinAnsiEncope`, `Parent` and
  `CropBox`. `Parent` is Table 30's and `CropBox` is inheritable, so nothing in the subset says
  page rather than node, and the object's own `/Type` reads `/PagP`. **Beyond additive recovery**:
  taking it would mean deciding it is a page on the strength of it not looking like a node, which
  is the substitutive direction trap 5's test forbids.

#### The question these eleven hand on

Two doors, and each is worth arguing rather than assuming:

1. **A prefix the tree names.** Five documents, the argument above, and one honest risk: §7.3.7's
   subset can only say what the producer *did* write, so "this dictionary states no `/Kids`" is not
   knowable from it — the evidence runs the other way, from an entry Table 30 does not define and
   §7.7.3.4 does not make inheritable being present. It needs the recovery to run from the *tree*
   rather than from the scan, which `Pages::new` does not do today: `Document::get` answers `Null`
   for the damaged child and the walk stops there.
2. **Resynchronising past an unreadable value.** Two documents, and it is a bigger claim than the
   prefix rule: §7.3.7 states no extent for an entry's value, so a reader that skips to the next
   `/Name` has guessed where the bad value ended. The one thing in its favour is that no valid
   object begins with the keyword the guess steps over — but that is an argument about *these*
   files rather than about the clause, which is where it should stop until somebody reads §7.3
   properly for it. **[§36](#36-what-the-eight-hundred-and-sixty-third-read-73-for-door-2-which-is-now-closed)
   is that reading and the door is closed**; the objection in this bullet turns out to be
   answerable and a different clause refuses it. Do not re-open this from here.

Neither was taken in the eight-hundred-and-sixty-first. What it took was the account: **the eleven
are five defects, two of them with a route the standard supplies and three with none**, and the
census prints which is which rather than this file asserting it.

### 35. What the eight-hundred-and-sixty-second took: the first of those two doors

**Door 1 is built and all five of §34's witnesses draw.** `Pages::new`'s recovery now runs the
tree's `/Kids` beside the scan: `tree_named` descends from the catalogue's `/Pages` collecting the
object numbers the arrays state, and a damaged prefix that does not declare its own `/Type /Page` is
taken where the tree names it **and** it holds one of Table 31's page-only entries. ADR 0786 has the
argument; the sentence that closes it is the one after Table 30, which §34 cited by its subclause and
which is worth having verbatim: a page tree node "may contain further entries defining inherited
attributes for the page objects that are its descendants", so a node's legitimate keys are Table 30's
four and §7.7.3.4's four and no others.

Three things that reading changed from §34's sketch, each worth carrying:

- **The discriminator is a positive list of Table 31's keys, not the complement of Table 30's.**
  `poppler-355-0.pdf`'s prefix holds `/WinAnsiEncope`, a key in neither table — under the complement
  it is evidence and the file is taken, under the list it is evidence of nothing and the file stays
  refused, which is what §34 argued for and what the complement would have quietly undone.
- **The tree is walked from the catalogue's `/Pages`, not from every `/Kids` in the file.** The census
  reads every array called `/Kids` and can, because it is a census; a *reader* that did would collect
  §12.7.4.2's field kids and §7.9.6's name-tree kids, which §7.7.3.2 says nothing about. Table 29
  makes the catalogue's entry the root "page tree node" by declaration, which matters: three of the
  five witnesses have a root that states no `/Type` at all, so a rule keyed on `/Type /Pages` would
  have lost them.
- **The report says which door the page came through**, because they are two different claims about
  the file — Table 31's `/Type` off the producer's bytes, against this reader's inference from
  §7.7.3.2 — and `Unsupported::PageDictionary` now names the evidence and the entry.

Door 2, resynchronising past an unreadable value, is **not** taken and the argument against it is
unchanged: §7.3.7 states no extent for an entry's value, so skipping to the next `/Name` guesses
where the bad value ended. (**§36 closed the door and this paragraph's reason is not the one that
closed it** — §7.2.3 does state the extent; what refuses the door is the continuity that sentence
is bounded by. ADR 0787.) `GHOSTSCRIPT-698887-0.pdf` and `GHOSTSCRIPT-699695-1.pdf` are still its
only witnesses, and `poppler-355-0.pdf`, `PDFBOX-3870-0`, `PDFBOX-3894-0` and `PDFBOX-4452-0` are
beyond any additive recovery for the reasons §34 states. `standing_count_census` prints where the
population now stands.

### 36. What the eight-hundred-and-sixty-third read: §7.3 for door 2, which is now closed

**§34 said the argument "should stop until somebody reads §7.3 properly for it".** This is that
reading, and its verdict is **no**: door 2 is refused for good, and ADR 0787 has it in full.

**The surprise is that §34's own objection is answerable.** It said a reader skipping to the next
name "has guessed where the bad value ended", and three clauses say otherwise:

- **§7.2.1 puts tokens below objects.** Bytes "can be grouped into tokens according to the syntax
  rules described in subclauses 7.2.2 … through 7.2.4", and "[o]ne or more tokens are assembled to
  form higher-level syntactic entities, principally objects". So a token's extent is decided
  without reference to any object.
- **§7.2.3 states that extent.** "A sequence of consecutive regular characters comprises a single
  token", and "[a]ny of these delimiters terminates the entity preceding it and is not included in
  the entity."
- **§7.3.1's list of nine types is closed, and each subclause names its introducer** — `true`,
  `false`, `null`, §7.3.3's digit forms, `(`, `<`, `/`, `[`, `<<`, and §7.3.10's two integers then
  `R`. A run of regular characters outside that set begins no object of any type.

So at a failing value the file states **no value at all**, and the run's extent is the standard's
rather than a reader's. Both witnesses fail exactly there — `R` in `GHOSTSCRIPT-698887-0.pdf`,
`\xff` in `GHOSTSCRIPT-699695-1.pdf`.

**What closes the door is the sentence that bounds §7.2.3**: its rules "apply to all characters in
the file except within strings, streams, and comments". A reader knows it is outside those three
only by having tokenised continuously from the object's `<<` — and **continuity is what ADR 0784's
subset argument rests on**, which is easy to miss because 0784's sentence is about *order*. The
entries are a subset not because they came first but because every one of them is the producer's
own, which byte continuity from a known position is the whole of the proof for. Resynchronisation
is the deliberate surrender of that continuity.

**The counterexample is one byte wide**, and it is pinned as three files in
`crates/pdf-model/tests/damaged_page_dictionaries.rs::the_third_door`:

```
A   2 0 obj << /Note (junk /Contents 9 0 R more) /Rotate [0 >] >> endobj   refused, rightly
B   2 0 obj << /Note Zjunk /Contents 9 0 R more) /Rotate [0 >] >> endobj   refused today
```

`B` is `A` with the string's `(` replaced by a regular character. Under door 2 the prefix becomes
`{/Contents 9 0 R}`, ADR 0786's door fires on it, and object 2 becomes a page drawing object 9 —
bytes the producer wrote inside a string. The manufactured entry is not noise a recovery tolerates;
it is **the discriminator the recovery acts on**, so one byte decides both that the object is a page
and what it draws.

Three ways of saving it were weighed and each fails. Requiring the resumed reading to reach `>>`
makes it worse — an object assembled across a gap that closes cleanly stops being a
`DamagedDictionary` and reaches every reader through `Document::get` rather than the one consumer
that asks for a prefix by name. Refusing on an unmatched `)` fails because the same damage eats the
`)`: `699695-1`'s corruption is *runs of `0xFF` over arbitrary bytes*, which is precisely the
mechanism that would destroy a `(`, so **the witness cannot distinguish its own case from the
counterexample**. And taking the witnesses' corroboration — that their object 4 really is a content
stream — is the argument about *these files* §34 already refused.

**So the eleven of §34 are settled**: five draw through door 1 (§35), and the other six are beyond
any additive recovery, with `GHOSTSCRIPT-698887-0.pdf` and `GHOSTSCRIPT-699695-1.pdf` moving from
*not yet argued* to *refused with the argument written down*. ADR 0782's standing `/Count` and a
refusal out loud is the answer for all six.

**The general form outlives the clause**, and it is the third sentence in this family after ADRs
0343 and 0784: ask what a prefix of the thing *is*, ask whether the thing's parts are *ordered*, and
now **ask what made the prefix the producer's**. Where the answer is byte continuity from a known
position, no recovery may skip bytes and keep the guarantee.

### 37. What the eight-hundred-and-sixty-fourth took: `batch5`, the GHOSTSCRIPT tracker, and a silence that was the reader's

**`batch5` landed and is on disk.** The fetch shepherd of the round before verified it against
Apache's published SHA-512 and this round verified it again before extracting; `ls
corpus-cache/tika-issue-tracker/` is what says which batches are here, and `batch4` was still
being fetched piece by piece while this round ran and was left alone. `batch5` is the widest of
the six by directory count — two dozen trackers rather than one or four — and `ls
corpus-cache/tika-issue-tracker/batch5/*/*.pdf | wc -l` counts what it holds.

**The chunk is `batch2`'s GHOSTSCRIPT directory, walked as eight shards, one process each**, plus
the TIKA directory beside it. §1's method, and the shard directories are symlink farms outside the
tree, deleted after the walk for `--bin undenominated`'s sake (§30's rule).

| directory | documents | line |
|---|---|---|
| `batch2/GHOSTSCRIPT`, eight shards | 5442 | 15 unopenable, 47 locked, 19 encrypted beyond us, 52 pageless, 472 incomplete, 85 slow |
| `batch2/TIKA` | 154 | 0 unopenable, 4 locked, 0 encrypted beyond us, 0 pageless, 7 incomplete, 0 slow |

The line is the tree with this round's own fix in it; the walk before it read 473 incomplete and
76 slow, and the whole of the first difference is one document named below. The second is load.

**The rate places the tracker rather than the corpus, which is §30's finding confirmed on a fifth
directory.** 472 of 5442 is **8.67%** incomplete, between `PDFBOX`'s 7.25% and `poppler-gitlab`'s
11.7% and nowhere near `MOZILLA`'s 2.47% — and TIKA's 7 of 154 is **4.55%**, which is a corpus of
documents somebody could not *extract text* from rather than one somebody could not parse. The
web's is 1.735% and the pdf.js gate's 6.98%. A bug tracker's rate is a fact about the tracker.

- **The directory holds 16 `.ai` files the survey never sees**, because the walker takes `.pdf`.
  Illustrator has written PDF since version 9, so those are documents; nothing here depends on
  them and the count above is stated over the 5442 the instrument read.
- **The reports are the ones this project already owns**, ranked by report rather than by
  document: 5473 `Operator` and 3845 `MissingResource` — a fuzzed content stream's garbled tokens
  and the resources its reduction removed — then 296 `Font`, 142 `Shading`, 124 `Image`, 103
  `Text`. Of the fonts, 67 are a glyph the document's own subset does not contain (closed by
  decision), 35 a font program decoded only as far as its damage, 17 `Identity-H` over a
  descendant with no embedded program ([`21`](21-font-substitution.md)) and 11 a `/Font` the
  dictionary does not define. The 26 `TransparencyGroup` are
  [`23`](23-transparency-departures.md)'s.
- **The bounds are tripped 40 times: `MAX_TILES` 21, `MAX_FORM_DEPTH` 16, `MAX_OPERATIONS` 2,
  `MAX_STATE_DEPTH` 1.** That is by far the widest `MAX_FORM_DEPTH` population this project has
  met — ADR 0271 established over the crawl and §30 over `MOZILLA` that every witness is a cycle,
  on eleven documents in two corpora, and these sixteen have not had that experiment run on them.
  **It is what this chunk leaves owed**: lift the bound sixteenfold in a scratch build, one
  process apiece, and see whether any of the sixteen stops reaching it. A single legitimate one
  would move `doc/todo/49`'s constant. **Run in section 39**: two of the sixteen stop.
- **The `slow` count is again mostly the instrument, and this is the fourth chunk to say so.**
  Four of the 76 were timed alone against 8-way load: 66.6 s against 408.6, 86.3 against 350.8,
  **2.4 against 130.4** and 9.7 against 106.6. Two are genuinely slow, one is a 5700 × 3504 target
  — a size rather than a structure — and one is not slow at all.
- **The slowest is the best witness [`40`](40-mask-chain-crop.md) has ever had, and it is not
  where that item is looking.** `GHOSTSCRIPT-692419-3.bz2-0.pdf` spends **66.5 s of its 66.6 in
  `interpret`**, producing 25 125 commands that reference **24 743 distinct clips** on a 596 × 842
  page, and reports nothing. §30 handed `40` a document at 3166 commands and 3149 clips whose 41 s
  were the *rasteriser's*; this one is eight times the clips with the time on the other side of
  the boundary, so the two together say the chain costs at both ends. `GHOSTSCRIPT-688696-0.pdf`
  is the ordinary shape beside it — 2.45 s of interpretation and 83.8 s of rasterising 88 077
  commands over 649 clips.
- **The 130.4-second one is the file this corpus has now filed three times.**
  `GHOSTSCRIPT-692158-0.pdf` is byte-identical to §29's `poppler-978-0.pdf` and
  `PDFBOX-3688-0.pdf` — the same SHA-256, filed against three readers by three people — and it
  draws in **2.4 s** because ADR 0780's `MAX_GROUP_BLIT_PIXELS` refuses its 73 047 groups. A third
  copy is a third confirmation that the bound is where the document is.

**The defect this chunk yielded is not in the GHOSTSCRIPT directory at all**, and finding it is an
argument for running the settled census over a *new* population rather than over the one being
walked:

```sh
cargo run --profile gates -p pdf-model --example standing_count_census -- corpus-cache/tika-issue-tracker/batch{1,2,3,5,6}
```

Over five batches it prints every document that states a page count and produces no page, split by
cause, and exactly one of them fell in the class the census's own doc comment calls "a defect of
the scan rather than of the file": an object that **parses whole**, declares `/Type /Page`, is
named by the tree's `/Kids`, and still yields nothing. **That class is empty now** — the census's
`have one that parses whole` column reads 0 — which is the mechanical form of this round's fix. That is `batch5/cairo/cairo-85141-3.pdf`, and the cause is a cross-reference subsection
this reader abandoned at its fourth entry because a corrupter wrote `000/0` where a generation
belongs. Everything after it — twenty-five entries, the page among them — became a number with no
entry, which everywhere else in this reader means *deleted*. ADR 0789 separates the two conditions
out of §7.5.4's own subsection header and the page draws; the other five are §34's and §36's
already-argued refusals.

**And the same defect has a witness inside the walked chunk after all, which only the re-run
found.** `GHOSTSCRIPT-692248-0.pdf` reported §7.3.10's null for its own `/Contents` — object 4,
one of the numbers its subsection declared and never described — and drew a blank 240 × 240 sheet.
It draws with ink and reports nothing now, and it is the difference between the two survey lines
above. Both documents are rows in `doc/checks/fixed-documents.toml`; the second is the one with
marks on it, which is why it is worth having beside the first.

**What is left here, in the order its value is clearest**: `batch4`, once its pieces land;
`batch5`'s two dozen trackers, none of them walked; the `MAX_FORM_DEPTH` sixteen above; and the
interpretation-side clip cost, which is [`40`](40-mask-chain-crop.md)'s to price with a witness it
did not have.

### 38. What the eight-hundred-and-sixty-ninth re-walked: the second half of ADR 0798's GHOSTSCRIPT slice, after `doc/todo/17`

The slice ADR 0798 measured is the first 680 of `batch2/GHOSTSCRIPT` in sorted order, and its
second half is the 340 that held the ten-gibibyte document. Re-walked here after the raster
cache stopped holding the resource dictionary (ADR 0791), through `tools/bounded.sh`, one walk
on the machine at a time, in the two forms that ADR used:

| run | documents | threads | peak resident | round |
|---|---|---|---|---|
| second half, one survey shard | 340 | 24 | 12.58 GiB | 866 |
| second half, one survey shard | 340 | **24** | **1.55 GiB** | 869 |
| second half, one document at a time under `--data 2` | 340 | 24 | none ran out; worst 0.03 GiB | 869 |

The survey's own line for the shard, this round: **0 unopenable, 6 locked, 0 encrypted beyond us,
1 pageless, 24 incomplete, 0 slow** — 340 documents in 2.2 s of a 3 s wrapper run, the witness
`complete` as before. The per-document walk's seven non-zero exits are `render_at`'s own `expect`s
on the six locked and the one pageless document, which the wrapper reports as a panic and not as
the bound. The shard was a symlink farm in the round's scratchpad, deleted afterwards (§30's rule),
and C-locale `sort` is the order — `ls` under a UTF-8 locale collates a few of these names
differently, so a round that wants the same 340 sorts the same way.

**What is left here is unchanged from §37**: `batch4` once its pieces land, `batch5`'s two dozen
trackers, the `MAX_FORM_DEPTH` sixteen, and `40`'s clip cost.

## What not to do

- **Do not start a multi-gigabyte download without asking**, and on a metered connection do not
  start one at all. The tool cannot do it by accident and can do it on purpose; which of those a
  session is doing is a question for whoever is paying for the bytes.
- **Do not add a corpus to the default gate sequence** on the strength of it being interesting.
  Session 385 took that sequence from 608 s to 268 s. The `pdfbox` comparison is in it because it
  costs 0.4 s on a binary already being built and needs no external process; an instrument that
  runs `pdftotext`, `pdftoppm` or `gs` per document is a different proposition and the timing is
  the argument either way.
- **Do not commit a SafeDocs file** without reading `doc/third-party-data.md`'s entry for it.

### 39. What the eight-hundred-and-seventy-first ran: ADR 0271's experiment on section 37's sixteen, and two of them are not cycles

Section 37 left one thing owed by name: sixteen documents in `batch2/GHOSTSCRIPT` reach
`MAX_FORM_DEPTH`, and ADR 0271's experiment — lift the bound sixteenfold in a scratch build, one
process apiece, and see which still reach it — had been run on every earlier witness (four in the
crawl, seven in `MOZILLA`) and on none of these. Every earlier witness was a cycle, and the ADR
argued from that: "this is the attack the bound exists for".

**Method.** The survey was re-run over the directory to name the sixteen —
`tools/bounded.sh --data 32 -- safedocs survey --dir corpus-cache/tika-issue-tracker/batch2/batch2/GHOSTSCRIPT`,
one process at 24 threads, 172 s, 13.5 GiB peak, and its line is *5442 documents: 15 unopenable,
47 locked, 19 encrypted beyond us, 52 pageless, 471 incomplete, 9 slow* against section 37's 472
and 85 — the same sixteen `MAX_FORM_DEPTH` documents by name. Then `MAX_FORM_DEPTH` was set to
256, 32 and 64 in turn in a scratch build of `examples/open_one` under its own `--target-dir`,
the constant put back after each build, and each of the sixteen run through
`tools/bounded.sh --data 2 -- timeout 120 open_one <document> 0.1`, reading the `unsupported`
list `open_one` prints after interpretation. No run needed the timeout; the slowest interpreted
in 97 ms at 256.

**The split: fourteen cycles, two legitimate nestings.**

| at 256 | documents |
|---|---|
| still reach the bound — a form that reaches itself | `689439-0`, `690847-0`, `690847-1`, `690847-2`, `691755-1`, `692217-0`, `692368-0`, `692733-0`, `693033-1`, `693134-0`, `693144-0`, `694531-0`, `698226-0`, `700301-0` (all `GHOSTSCRIPT-…pdf`) |
| **stop** — finite nesting deeper than 16 | `GHOSTSCRIPT-697655-0.pdf` (stops at **32**; pdftk 2.02 over iText), `GHOSTSCRIPT-695948-0.zip-0.pdf` (stops at **64**, not at 32; Aspose.Pdf for .NET 9.3.0) |

**Both of the two are real documents by real producers, and both draw a blank page at 16.**
`697655` is a French company's articles of association — a coloured banner, headings, body text —
which `mupdf` draws whole and this tree draws as nothing, reporting the bound; pdftk's stamp and
background operations wrap the previous page content in a form each time they are applied, which
is how a page comes to be seventeen or more forms deep with no cycle in it. `695948` is a boxed
paragraph of regulation text with one image inside it, nested between thirty-three and sixty-four
forms deep, and again `mupdf` draws it and this tree draws nothing. Rendered and looked at (trap
1), both.

**So ADR 0271's argument for `MAX_FORM_DEPTH` no longer holds as stated**, and this section says
so rather than moving the constant: "every one of them is a form that reaches itself" was true of
eleven witnesses and is false of two in sixteen more. The bound is still what stands between a
cycle and a stack exhaustion, and the fourteen cycles here reach 256 as fast as they reach 16 —
the whole population of sixteen interprets in under a tenth of a second apiece at 256 — so the
cost of a higher bound is not time. What a higher bound costs is stack per level, which is the
figure a round that raises it owes: the interpreter's frame per nested form, times the bound,
against the thread's stack. **`doc/todo/49` carries the item**: the constant is a decision, the two
documents are its witnesses, 64 draws both, and the crawl's four and `MOZILLA`'s seven should be
run at the candidate bound in the same sitting so that the claim about cycles is re-made over all
twenty-seven rather than inherited.

**And the fourteen were not cycles either, bar two** — the eight-hundred-and-seventy-fourth
found the experiment above measured the instrument: a tiling cell was run at `MAX_FORM_DEPTH - 1`,
so a cell holding two levels of forms reached the bound at 256 as surely as at 16. With the bound
asked once in `Interpreter::run` and set to 64 from a stack measurement, twenty-five of the
twenty-seven witnesses across the three corpora draw whole reporting nothing, and only
`GHOSTSCRIPT-698226-0.pdf` and `GHOSTSCRIPT-700301-0.pdf` report it at 64 and at 256 alike. ADR
0793, and `doc/traps/instruments-and-reports.md`'s trap 29.

**Two things this section does not claim.** The survey line's 471 against 472 is one document
this tree now draws that it did not in section 37, and it was not chased — the round's own
change (ADR 0792, calibrated one-component blending spaces) is the likely cause and the survey
is a baseline rather than a ratchet. And a "legitimate" nesting is one that terminates; whether
the sixty-four-deep Aspose page is a producer's intent or a producer's accident is not a question
the bound can answer, and the page it holds is a page.

### 40. What the eight-hundred-and-seventy-sixth took: `batch5`'s REDHAT tracker, and a fax drawn to the row it breaks on

**The first of `batch5`'s two dozen trackers, walked under the four rules of 2026-09-02**: one
walk on the machine at a time, `tools/bounded.sh --data 8 --tree 12`, twelve rayon threads, and
the directory taken whole rather than as shards — 1712 documents in 22 s at a 2.4 GiB peak. The
line, a baseline for this directory and never a ratchet:

| directory | documents | line |
|---|---|---|
| `batch5/REDHAT` | 1712 | 2 unopenable, 2 locked, 2 encrypted beyond us, 4 pageless, 104 incomplete, 0 slow |

**The rate places the tracker where §30 would have put it.** 104 of 1712 is **6.07%** incomplete —
between `MOZILLA`'s 2.47% and `PDFBOX`'s 7.25%, and about the pdf.js gate's 6.98%. Red Hat's
attachments are documents evince could not show a person, which is the Mozilla shape with a
reader in front of it that is closer to this tree's own references.

- **The four pageless are nobody's**: 175, 255, 871 and 28 768 bytes, and neither `pdfinfo` nor
  `mutool` gets a page count out of any of them. §29's claim holds here without qualification.
- **The reports are the ones this project owns**, ranked by report: 97 `Operator` (garbled tokens
  in damaged content streams, three of them `BT without ET`, which the interpreter notes and draws
  through), 94 `Font` (77 a glyph the document's own program has no outline for, closed by
  decision; two `Identity-H` over a descendant with no program, which is [`21`](21-font-substitution.md);
  five a `/Font` resource the dictionary does not define; two an entry that is not a font
  dictionary under §7.3.10), 16 `TransparencyGroup` ([`23`](23-transparency-departures.md)), 16
  `Text`, 9 `Content` (three `/Contents` streams that resolve to §7.3.10's null, four corrupt
  Flate parts drawn to the damage, two undecodable), 5 `Image`, 4 `MissingResource`, one
  `MAX_TILES`, one `/MediaBox` inherited from nowhere.
- **Ranked by ink** — ours, flattened on white, against `pdftoppm -cropbox` and `mutool draw` at
  72 dpi over all 104 incomplete pages, the same instrument as every chunk above and with the
  same two traps sprung before it was believed (`-cropbox`, and an alpha channel that returns
  half the ink) — **the head is one finding by a factor of three**: `REDHAT-229174-0.pdf` and its
  byte-identical `REDHAT-493442-0.pdf`, ours **0** against `poppler` 8.86 and `mupdf` 74.31, on the
  report `I1: CCITTFaxDecode: arithmetic overflow in position calculation`. Below it the head is
  the known populations: `1532381-0` (a resource dictionary nobody resolves, `poppler` blank
  too), `652152-0` (`MAX_TILES`, −2.2), `1575851-0` and `443362-1` (under two levels), and the
  rest inside ±0.6.

**The finding is a decision this tree had made and the standard had not.** The stream is a
Photoshop 4.0 Group 4 scan of a textbook page whose data breaks at scan line 756 of 2244 — a
probe against the pinned `hayro-ccitt` decodes 756 whole lines and stops inside the 757th, which
is exactly where `poppler`'s text stops; `poppler`'s black band below it and `mupdf`'s two further
lines of text are what each does *after* the damage. §7.4.6 says the filter "shall not perform any
error correction or resynchronization", and Table 11's `/DamagedRowsBeforeError` defaults to zero —
so the decoder is right to stop, `mupdf`'s continuation is what the sentence forbids, and the only
question was this tree's: `pdf_sandbox::decode::ccitt` refused the whole picture on any decoder
error, on a doc comment arguing that drawing the rows before it "would be a page that is silently
missing its bottom half". The word was *silently*. ADR 0794: the rows the filter delivered are
drawn, the rest are left **unpainted** — not the filter's white, which under `/BlackIs1 true` with no
`/Decode` is the page's black, and the first build of the change drew two thirds of the page solid
before the page was looked at — and the shortfall is reported beside the drawing, through the
raster cache, for the page's image, a `/Mask`, an `/SMask` and the transform's `images` verb. The
page draws what `poppler` draws above row 756 and is a row in `doc/checks/fixed-documents.toml`.
The file's `stream\r` — §7.3.8.1's forbidden CARRIAGE RETURN alone — was checked first, because an
off-by-one at the stream's start produces the same sentence; the parser already tolerates it.

**`batch5/poppler` was walked twice and surveyed neither time.** At twelve threads and again at
four, one document asks the allocator for **6 001 925 632 bytes** in one allocation and the
survey dies under `--data 8`, which is the whole directory's verdict lost to one file — §1's
one-process-per-archive argument met inside one directory. Which document it is was not chased:
finding it is a per-document walk under `tools/bounded.sh --data 2`, one process apiece, and that
walk is what the next round of this chunk starts with, because a six-gigabyte allocation that
passes every budget this tree has is [`10`](10-bounds-that-cap-size.md)'s question before it is
this file's.

**What is left here**: `batch5/poppler`'s six-gigabyte document and then its 1586; `batch5`'s other
twenty-two trackers, `FOP` (808) and `PDFIUM` (379) the largest; `batch4` once its pieces land;
and [`40`](40-mask-chain-crop.md)'s clip cost. The REDHAT ranking's second and third rows are a
dictionary nobody resolves and a `MAX_TILES` refusal, both diagnosed populations, and the two
`Content { Unreachable }` documents whose `/Contents` is §7.3.10's null are worth a look from
ADR 0789's side — the survey names them and this round did not open them.

### 41. What the eight-hundred-and-seventy-eighth took: `batch5/poppler`, the six-gigabyte document, and a JPEG whose lines are stated after its data

**Section 40's allocation was the file.** The directory walked one document per process under
`tools/bounded.sh --data 2 --tree 4`, four lanes side by side, exited 0 on every one of its 1586
documents — because the failing allocation is not a decode, a raster or a table sized from a stated
dimension but `Arc<[u8]>: From<Vec<u8>>`'s copy of the whole file in `Document::open`.
`poppler-44085-1.xz-0.pdf` is **6 001 925 614 bytes**, an honest `%PDF-1.5` with a ten-digit
`startxref`, and 6 001 925 614 plus an `Arc`'s header rounded to eight is the 6 001 925 632 the
survey died asking for: `std::fs::read`'s own `try_reserve_exact` had already held the first copy
and the second was the one allocation on the open path that could not fail gracefully. ADR 0795:
`pdf_syntax::FileBytes` holds the vector where it was, `pdf_syntax::read_file` asks for the whole
length before the first byte is read and refuses by name (`NoRoom { length }`, under
`io::ErrorKind::OutOfMemory`) with **deliberately no number of this program's own** — the bound is
the process's limit, which is [`10`](10-bounds-that-cap-size.md)'s brief — and the document opens
and draws page one in 3.2 s at a 5.58 GiB peak, `complete`. The same round read every allocation
site sized by an expression across the six crates that touch a document's bytes and closed the one
other member of the class it found: §7.4.4.4's predictor sized two row buffers to `/Columns` before
reading a byte, so six bytes stating `/Columns 1099511627776` asked for two terabytes; they are
sized to the data now, byte-identical output, and both tests abort against the old sizing.

**Then the directory, surveyed whole under the four rules** — twelve rayon threads, `--data 8
--tree 12`, the six-gigabyte document surveyed on its own beside it. The line, a baseline for this
directory and never a ratchet:

| directory | documents | line |
|---|---|---|
| `batch5/poppler` | 1586 | 6 unopenable, 3 locked, 2 encrypted beyond us, 27 pageless, 183 incomplete, 0 slow |

**The rate is the tracker's shape.** 183 of 1586 is **11.5%** incomplete — above every tracker
before it (`PDFBOX` 7.25%, `REDHAT` 6.07%, `MOZILLA` 2.47%) and above the pdf.js gate's 6.98% — and
the reason is what a poppler bug attachment is: a reduced or fuzzed file that crashed a renderer,
far more often than a document a person could not read. The unopenable are three files with no
header, two with a table that is unusable and no object header anywhere, and one that is `.xz`
in name only and opens; the locked are three. **The 27 pageless are not section 29's** — `pdfinfo`
counts pages in eighteen of them and `mutool` in four, and every one is a hand-mangled tree (a
`/Kids 3 0 R` naming an object the file does not hold, a `/Type` whose name is corrupt bytes
before `/Page`, `3 32767 obj` headers with no table): section 34's population again, the doors of
sections 35 and 36 already read, and not chased here. Ranked by report, one document counted once
per kind: 52 `Font`, 42 `Text`, 34 `Content`, 30 `Image`, 26 `Operator`, 23 `MediaBox`, 20
`LimitReached` (17 `MAX_TILES`, 2 `MAX_OPERATIONS`, one `MAX_OPERANDS`, one `MAX_FORM_DEPTH`), 19
`MissingResource`, 17 `TransparencyGroup`, 16 `Shading`, 15 `Annotation`, 6
`DamagedContentStream`, 5 `UndefinedCurrentPoint`, 4 `NoninvertibleMatrix`, 3 `PageDictionary`,
one `CompositedInParts`.

**Ranked by ink** — ours flattened on white against `pdftoppm -cropbox` and `mutool draw` at
72 dpi over all 183 incomplete pages, the same instrument as section 40 with its two traps sprung
— **the head is at the *dark* end and both references agree on it**: `poppler-61994-0.pdf`, ours
**60.4** against `poppler` 5.38 and `mupdf` 5.03, a scanned letter drawn as the top five per cent
of a grey page. The light end is one-reference evidence throughout: `poppler-103116-0.pdf` (ours 0,
`poppler` 32.8, `mupdf` 0: garbled operators, an unreadable font, a JPEG with no headers),
`poppler-26280-0.pdf` (ours 0, `poppler` 0, `mupdf` 30.5: a JPEG with no quantisation table for
Cb, which `mupdf`'s decoder tolerates), `poppler-6688-0.pdf` (ours 0, `poppler` 27.6, `mupdf` 0: a
corrupt Flate content stream of which 202 bytes decode). Fifty-six rows have no reference ink at
all.

**The head is a number the encoded data states after its data.** The letter's `SOF0` says
`Y = 65535`; a `DNL` marker after the scan says 3486 lines; the dictionary says 3473. ISO/IEC
10918-1 section B.2.5 makes the `DNL` define or redefine `Y`, §7.4.8 puts the dimensions in the encoded
data, and `zune-jpeg` reads the header alone and pads to it. ADR 0799: `image::frame_as_defined`
walks the markers, writes the `DNL`'s count into `Y` and takes the segment out before the decoder
sees the bytes; the page draws the letter at 5.24 between the two references; the fixture pins all
three of 10918-1's cases and is a row in `doc/checks/fixed-documents.toml`.

**What is left here**: `batch5`'s other twenty-one trackers, `FOP` (808) and `PDFIUM` (379) the
largest; `batch4` once its pieces land; [`40`](40-mask-chain-crop.md)'s clip cost; and a
file-backed reader for a document the size of section 40's, which [`10`](10-bounds-that-cap-size.md)
now carries as the question a 5.6 GiB resident peak leaves open. The poppler ranking's second and
third rows are one-reference pages of damaged JPEGs, which `mupdf` and `poppler` disagree about
between themselves; and the 27 pageless files are section 34's population and worth a census
before any of them is opened by hand.
