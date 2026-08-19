# More corpora, and the one that cannot be a submodule

Status: **standing**, and it became standing rather than one-off in the four-hundred-and-twenty-second
session, which built the three pieces `doc/test-docs.md` asked for. What is left is the *taking*:
a chunk a round, the way `doc/todo/00` takes a page off the ambiguous ranking.
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

- ~~**The oracle over the new corpora.**~~ **Decided in the five-hundred-and-fifty-eighth and the
  answer is no** (ADR 0393). This bullet said `pdf-differences` "exists *because* readers diverge
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
