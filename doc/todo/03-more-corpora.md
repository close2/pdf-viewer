# More corpora, and the one that cannot be a submodule

Status: **standing**, and it became standing rather than one-off in the four-hundred-and-twenty-second
session, which built the three pieces `doc/test-docs.md` asked for. What is left is the *taking*:
a chunk a round, the way `doc/todo/00` takes a page off the ambiguous ranking.
Priority: 03 — the standing band, deliberately.
Corpus: 974 pdf.js documents **plus 108 in three submodules** and whatever `tools/safedocs` has
been asked for — **1944 as of the four-hundred-and-twenty-fifth**, 81 archives, the manifest below
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

**Taken so far: 81 archives, 1944 documents, 2.68 GiB transferred — 5.4% of the 50 GB.**
`0000` and `3500` in the four-hundred-and-twenty-second and -third, 24 members apiece; then in
the four-hundred-and-twenty-fifth, **`50 + 100k` for k = 0 … 78** — `0050`, `0150`, … `7850` —
24 members from the head of each, 2731.0 MiB of member ranges and 14.1 MiB of central
directories, 79 fetches, 0 failures, every CRC-32 matched. **That stride is spent**; the next
round wants a window the manifest does not hold, and by decision 2 of ADR 0261 it may be any
shape at all.

What a chunk owes when it is taken:

- the survey's line, recorded — it is a baseline for that chunk, never a ratchet;
- every report read against the population it belongs to, because a second witness for a
  population `doc/todo/21` or `23` already names is worth nothing;
- a **promotion** only where the problem is new and is named in the commit. The budget below.

**The four-hundred-and-twenty-fifth's 1896 new documents, as the baseline to beat:** *1896
documents in 42.1 s: 4 unopenable, 1 locked, 0 encrypted beyond us, 3 pageless, 86 incomplete,
0 slow*, with 862 codes reaching no glyph in silence over 12 documents. **85 incomplete after
that session's own fix.** Two things it is worth knowing before reading the next one:

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

- **A zero-byte stream that names a filter.** `4150022.pdf` — archive `4150`, SHA-256
  `85ea41fdedc0a195deacd9aedf88df9a0e002bd05015940d1e2351c55a1b9c29` — has a six-part
  `/Contents` whose fourth object is
  `<< /Filter /FlateDecode /Length 0 >>`, and `flate` refuses an empty input, so the part is
  reported `Undecodable`. The report claims the page is missing drawing and an empty part cannot
  be — but "decode zero bytes to zero bytes" also silences a stream truncated to nothing, and
  choosing between those two needs the argument rather than the patch. §7.3.8.1 permits "zero or
  more bytes" between the keywords, so the *stream* is valid and only the filter's input is empty.
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
