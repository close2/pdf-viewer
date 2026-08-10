# More corpora, and the one that cannot be a submodule

Status: **standing**, and it became standing rather than one-off in the four-hundred-and-twenty-second
session, which built the three pieces `doc/test-docs.md` asked for. What is left is the *taking*:
a chunk a round, the way `doc/todo/00` takes a page off the ambiguous ranking.
Priority: 03 — the standing band, deliberately.
Corpus: 974 pdf.js documents **plus 108 in three submodules** and whatever `tools/safedocs` has
been asked for
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

**None of it is in `doc/todo/02` §2's sequence** and none of it should be until it has earned a
place. `safedocs survey` is a command a round runs on purpose.

## What is left, in the order its value is clearest

### 1. Take a chunk a round

The instrument works and the first 24 documents produced one finding. `--archive` names any of
7 933 archives and `--from` any window inside one, so a round's chunk should be **somewhere
nobody has been**: the manifest at `corpus-cache/safedocs/manifest.tsv` is the record of where
that is.

What a chunk owes when it is taken:

- the survey's line, recorded — it is a baseline for that chunk, never a ratchet;
- every report read against the population it belongs to, because a second witness for a
  population `doc/todo/21` or `23` already names is worth nothing;
- a **promotion** only where the problem is new and is named in the commit. The budget below.

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
  None has been found yet.

### 4. Two more instruments the new populations make possible and nobody has built

- **The oracle over the new corpora.** `tests/oracle.rs` compares against poppler, mupdf and
  ghostscript over the 974; `pdf-differences` exists *because* readers diverge on its files, so
  it is the population where a reference comparison should be most informative and where the
  three references are least likely to form a consensus. That is a session of its own and it
  needs a decision first: an oracle run over files chosen for disagreement will produce
  `ambiguous` almost everywhere, so the verdict vocabulary may not fit.
- **`pdfbox`'s own expected text.** Its `input/` directory carries `*.pdf.txt` and
  `*.pdf-sorted.txt` beside 64 of the PDFs — somebody else's extraction, checked in as a
  fixture. `tests/text_extraction.rs` measures this tree against `pdftotext` at run time; a
  second, *frozen* opinion is a different instrument and it is already on disk.

## What not to do

- **Do not start a multi-gigabyte download without asking**, and on a metered connection do not
  start one at all. The tool cannot do it by accident and can do it on purpose; which of those a
  session is doing is a question for whoever is paying for the bytes.
- **Do not add a corpus to the default gate sequence.** Session 385 took that sequence from
  608 s to 268 s.
- **Do not commit a SafeDocs file** without reading `doc/third-party-data.md`'s entry for it.
