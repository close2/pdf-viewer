# More corpora, and the one that cannot be a submodule

Status: **standing**, and it became standing rather than one-off in the four-hundred-and-twenty-second
session, which built the three pieces `doc/test-docs.md` asked for. What is left is the *taking*:
a chunk a round, the way `doc/todo/00` takes a page off the ambiguous ranking.
Priority: 03 — the standing band, deliberately.
Corpus: 974 pdf.js documents **plus 108 in three submodules** and whatever `tools/safedocs` has
been asked for — **65 944 as of the four-hundred-and-thirty-third**, 145 archives, 93 GB, the
manifest below
Code: `tools/safedocs`, `doc/corpora/*`, `doc/oracle-and-corpus.md` §2

## What exists now (ADR 0258)

- **`doc/corpora/pdf20examples`** — 7 files, CC BY-SA 4.0. The *coverage* denominator.
- **`doc/corpora/pdf-differences`** — 37 files, Apache-2.0. The oracle's food.
- **`doc/corpora/pdfbox`** — 64 files, Apache-2.0, a **partial sparse** checkout of
  `pdfbox/src/test/resources/input`. The recipe is in `doc/oracle-and-corpus.md` §2 because
  `.gitmodules` cannot hold it.
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
  4.1–53.6 M operators, and `MAX_STATE_DEPTH`'s single witness wants 337 where §C.2's Table C.1
  prints 28 as the depth a writer could rely on.
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

### 2. `openpreserve/format-corpus` is owed a licence reading, not a decision

Examined in the four-hundred-and-twenty-second and declined: GitHub detects no licence, the
`README.md` says "[a]ll items are CC0 licenced **unless otherwise stated**", and per-file `.md`
sidecars are where such a statement would be. Two facts for whoever reads it:

- **the `/pdf/` directory `doc/test-docs.md` names does not exist.** Its PDF material is
  `pdfCabinetOfHorrors` (9.6 MB, 30 files), `pdf-handbuilt-test-corpus` (0.1 MB, 90 files),
  `govdocs1-error-pdfs` (62.7 MB, 55 files) and `fully-featured-pdf` (22.8 MB, 9 files);
- a sparse checkout of the first two costs **9.7 MB**, in the same shape `pdfbox` uses.

The reading is: open the sidecars for the directories being taken and check that none states
other terms. That is an hour, not a round.

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

- **The oracle over the new corpora.** `tests/oracle.rs` compares against poppler, mupdf and
  ghostscript over the 974; `pdf-differences` exists *because* readers diverge on its files, so
  it is the population where a reference comparison should be most informative and where the
  three references are least likely to form a consensus. That is a session of its own and it
  needs a decision first: an oracle run over files chosen for disagreement will produce
  `ambiguous` almost everywhere, so the verdict vocabulary may not fit.
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
