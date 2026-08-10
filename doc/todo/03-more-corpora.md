# More corpora, and a fetcher for the one that cannot be a submodule

Status: **asked for by the project owner on 2026-08-10**, from `doc/test-docs.md`: add repositories
(as shallow as possible) for test documents that seem useful; build a tool that downloads the
SafeDocs stressful PDF corpus, in chunks, and tests against it. **The owner is on a mobile
connection — do not start a large download.** Short tests are fine.
Priority: 03 — it sits in the **standing** band deliberately, because once the fetcher exists this
becomes "take some of this every round", exactly like `00`'s bucket. Until then, item 1 below is a
one-off.
Corpus: 974 pdf.js documents today; the candidates below are tens of thousands
Code: `doc/pdf.js` (the existing shallow submodule), a new `tools/` fetcher,
`crates/pdf-model/tests/corpus.rs`

## Why this is worth a round, in `CLAUDE.md`'s own terms

`CLAUDE.md`'s "Two questions, two denominators" says coverage is measured against the specification
and **robustness against the world** — "what share of the files that actually exist render
correctly" — and that the corpus and oracle are *the only instrument* for the second. This tree's
world is currently 974 files chosen by one browser's bug history over a decade. That is a good
corpus and it is not the world, and its silence is not evidence: the same file records that a corpus
"declares success the moment the last file goes green".

## 1. Repositories, shallow, and what each is for

`doc/pdf.js` is already a submodule and is the pattern (`--depth 1`). Candidates from
`doc/test-docs.md`, in the order their value is clearest:

- **`pdf-association/pdf20examples`** — clean, compliant, human-readable PDF 2.0 files illustrating
  ISO 32000-2 features properly. **The most valuable of the four for this project**, because it is
  the *coverage* denominator rather than the robustness one: files that exercise clauses the pdf.js
  corpus never reaches. Small.
- **`pdf-association/pdf-differences`** — files that highlight non-conforming behaviour and
  ambiguous edge cases that make readers diverge. This is the oracle's natural food: a page five
  renderers disagree on is exactly what `tests/oracle.rs` is built to judge.
- **`openpreserve/format-corpus`** (`/pdf/`) — PDF/A, metadata, legacy encodings. Reaches §14.11 and
  the archival clauses nothing else here does.
- **`apache/pdfbox`** (`/pdfbox/src/test/resources/input/`) — real-world parsing failures from a
  decade of user reports, heavy on text extraction and font mapping. The repository is large because
  of its Java source; a **sparse checkout** of that one directory is the way, not a full clone.

**What to check before adding any of them**, and it is not optional: **the licence**. This tree
already carries ADR 0187's discipline because the specification PDFs could not be redistributed, and
`doc/third-party-data.md` is where such a position is recorded. A submodule does not copy the files
into our history, which is most of the answer, but the fetched corpus still has terms.

## 2. The SafeDocs fetcher

SafeDocs is **not a git repository and cannot be one**: the issue-tracker corpus alone is over 31 GB
in six compressed archives, and the untruncated Common Crawl set is 24 TB across nearly eight
million files. It is distributed through AWS Open Data and Digital Corpora as plain archives.

So: a lazy fetcher, in `tools/`, that

- takes a **chunk** — one archive, or a byte range, or an explicit file list — never "everything";
- caches into a **gitignored** directory and re-uses it;
- verifies what it downloaded (size and digest) rather than trusting a transfer;
- **prints what it would fetch and how large before fetching**, so a person on a mobile connection
  finds out first. This is the owner's constraint and it should be the default, with the download
  behind an explicit argument.

Then a mode that runs the corpus gate over a fetched chunk and reports the same way
`tests/corpus.rs` does — opened, reached page one, drew with nothing reported, reported something,
slower than 30 s.

## 3. What comes back into the tree, and the owner's rule

The owner's rule, verbatim in intent: **copy PDFs that expose problems into our test set, unless
they become too many. Below 20 MB of such files for the whole test set, commit them; above it,
commit only their names.**

So the fetcher's output is a *candidate list*, and promotion is a decision with a budget:

- a file is promoted only when it **exposes a problem** — a refusal, a report, a panic, a
  disagreement with the references, a timeout — and the problem is named in the commit;
- the running total is checked against 20 MB before committing, and stated;
- past the budget, the **name and its source URL** go in, plus enough to fetch it again exactly —
  which is what makes a names-only entry reproducible rather than a memory.

**Two hazards worth stating now.** A corpus this size will find crashers, and `CLAUDE.md` says every
crasher found becomes a permanent regression test — that is a *fuzz corpus* entry, which is small
and always committable, and it is a different thing from a rendering witness. And the gates are
ratcheted: a large new population will move every count at once, so the first run is a **baseline to
record**, not a regression to chase.

## What not to do

- **Do not start a multi-gigabyte download.** The owner said so; the tool should make it hard to do
  by accident.
- **Do not add a corpus to the default gate sequence.** `doc/todo/02` §2 is what every round runs and
  session 385 spent a whole round getting it from 608 s to 268 s. A new corpus is an `--ignored`
  test or a separate command until it has earned a place.
- **Do not commit binaries the licence does not permit**, and record the position in
  `doc/third-party-data.md` either way.
