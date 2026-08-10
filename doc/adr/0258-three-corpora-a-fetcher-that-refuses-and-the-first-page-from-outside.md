# ADR 0258 — Three corpora, a fetcher that refuses, and the first page from outside

Date: 2026-08-10 (session 422)
Status: accepted

## Context

`CLAUDE.md`'s "Two questions, two denominators" says robustness is measured against **the
world** — "what share of the files that actually exist render correctly" — and that the corpus
and the oracle are *the only instrument* for that question. This tree's world had been 974
documents chosen by one browser's bug history over a decade. That is a good corpus and it is
not the world, and the same file says a corpus "declares success the moment the last file goes
green".

The project owner asked for repositories of test documents, as shallow as possible, and for a
tool that downloads the DARPA SafeDocs corpus in chunks and tests against it — with one hard
constraint, stated on a mobile connection: **do not start the complete download; short tests
are fine.** The owner clarified it later in the same session: *"I am currently on a mobile
connection, but am regularily on a fiber connection, without any limits. I can then of course
make big download!"* So the requirement is not a small tool; it is a tool on which the size is
**known and asked for**, and which cannot spend a gigabyte that nobody typed.

## Decision 1 — three submodules, and one corpus examined and deferred

Under `doc/corpora/`, all three `shallow = true` in `.gitmodules`:

| submodule | licence | size | what it is for |
|---|---|---|---|
| `pdf20examples` | **CC BY-SA 4.0** (`LICENSE.md`, verbatim) | 96 KB, 7 PDFs | the **coverage** denominator: clean PDF 2.0 files exercising clauses by name |
| `pdf-differences` | **Apache-2.0** (`LICENSE`, the standard text) | 4.3 MB, 37 PDFs | the oracle's food: files built to make readers diverge |
| `pdfbox` | **Apache-2.0** (`LICENSE.txt`, `NOTICE.txt` both present) | 12 MB, 64 PDFs | a decade of real-world parsing failures from user reports |

`pdfbox` is a **partial, sparse** checkout and not a `--depth 1` clone, because the repository
is 118 MB of Java source and the wanted directory is 5.7 MB of it:

```sh
git clone --depth 1 --filter=blob:none --sparse https://github.com/apache/pdfbox.git \
          doc/corpora/pdfbox
git -C doc/corpora/pdfbox sparse-checkout set pdfbox/src/test/resources/input
```

1.9 s for the clone and 2.6 s for the checkout, measured. `.gitmodules` cannot express a sparse
checkout, so that recipe is written down in `doc/oracle-and-corpus.md` beside the submodule
list; a plain `git submodule update --init` still works and costs the full 118 MB, which is a
documented cost rather than a trap.

**`openpreserve/format-corpus` was examined and is not added**, and the reason is the licence
rather than the size. GitHub detects no licence on it; its `README.md` says "[a]ll items are
CC0 licenced **unless otherwise stated**", and the repository carries per-file `.md` metadata
sidecars that are where such a statement would be. A grant with an escape clause is not a
grant this project can rely on without reading 765 MB of sidecars, and ADR 0187's discipline —
which took the specification PDFs out of all 436 commits of this history — is what says so.
Two further facts belong in the record: the `/pdf/` directory `doc/test-docs.md` names **does
not exist** in that repository (its PDF material is `pdfCabinetOfHorrors` 9.6 MB,
`pdf-handbuilt-test-corpus` 0.1 MB, `govdocs1-error-pdfs` 62.7 MB and `fully-featured-pdf`
22.8 MB), and a sparse checkout of the first two would cost 9.7 MB whenever the licence
question is answered. That is what `doc/todo/03` now owes.

## Decision 2 — `tools/safedocs`, and what it cannot do *by accident*

SafeDocs cannot be a submodule. `CC-MAIN-2021-31-PDF-UNTRUNCATED` is **7 933 ZIP archives**
of 1.0 to 3.7 GB each, 7 932 878 PDF files, close to 8 TB uncompressed, served as plain
objects from `s3://digitalcorpora`. So a lazy fetcher — and the constraint it is built to is
**"impossible by accident", not "impossible"**. The owner clarified that during the session:
the mobile connection is the exception and a fibre connection with no limit is the rule, on
which a big download is perfectly fine. A tool that made one unreachable would have solved the
wrong problem, so what this one guarantees is that the number is *known and asked for*:

- **The archive is addressed a member at a time, never as an object.** A plan is resolved
  through the archive's own end-of-central-directory record and central directory — a `HEAD`
  and one 64 KiB range request, about 182 KiB against a 1.2 GiB object — and a fetch then asks
  for **one contiguous byte range** covering the members the chunk names. `doc/todo/03` asked
  for "one archive, or a byte range, or an explicit file list"; the byte range turned out to be
  the only one worth building, because a ZIP lays its members out consecutively, so a window
  of the listing *is* a window of the object — and `--all` is that window widened to every
  member, which *is* the whole-archive download and is still one argument somebody typed.
- **Nothing transfers without `--download`.** `safedocs fetch` without it prints the plan and
  says `nothing was transferred`.
- **A plan over the budget is refused**, in bytes **and in the `--budget-mb` that would admit
  it**, whether or not `--download` is present. `Budget::DEFAULT` is 32 MiB, chosen against the
  corpus: its median member is a little over a megabyte, so the default is a few dozen
  documents. **The budget has no ceiling**, and the refusal is written to be *acted on* rather
  than obeyed — `safedocs plan --archive 0042 --all` prints
  `would transfer 1.6 GiB … --budget-mb 1610`, and repeating the command with that number and
  `--download` fetches the archive. A wall would have been easier to write and would have been
  the wrong answer.
- **`curl --max-filesize` is the load-bearing guard**, not the length check beside it. A
  server that ignores `Range` answers 200 with the whole object, and against a 1.2 GiB archive
  the length check would notice only after the gigabyte had crossed the wire.
- **What arrives is verified against what the archive says** — the uncompressed length and the
  CRC-32 the central directory records — before it reaches the cache. That is what makes a
  byte-range fetch as trustworthy as a whole-object one, and it needs no digest anybody
  publishes separately.

Two dependency decisions, both **no**. The transport is `curl` as a subprocess, so this
workspace gains no HTTP client, no TLS stack and no certificate store — `tools/pdfref` set
that precedent for `pdftoppm`, `mutool` and `gs`. And the ZIP reading is 200 lines in tree
rather than a crate, because every ZIP crate reads `Read + Seek`, which over HTTP means either
downloading the object or writing the range-issuing adapter that is most of what is here
anyway. **No package was added to `Cargo.lock`**: `flate2` supplies both the raw-deflate
decoder and the CRC-32, and `sha2` was already here for §7.6.

The cache is `corpus-cache/safedocs/`, already covered by `.gitignore`, with `manifest.tsv`
recording corpus, archive, member, size and SHA-256 — which is what makes a names-only entry
reproducible rather than a memory.

`safedocs survey` runs this tree's five corpus questions over a directory and reports the way
`tests/corpus.rs` does. It is a **copy of that file's shape and not a call into it**, because
that file is a ratchet whose every constant is an argument about the 974 pdf.js documents;
pointing it at a new population would move every constant at once. `--dir` makes it work over
the three new submodules as well, and `doc/todo/03`'s rule holds: none of this is in
`doc/todo/02` §2's default sequence.

## Decision 3 — the promotion budget, and why it is mostly moot

The owner's rule is that PDFs exposing a problem come into the test set, and that below 20 MB
of such files they are committed and above it only their names are. Applied here it splits in
two, and the split is worth writing down:

- **A file from a submodule needs no promotion at all.** `doc/corpora/pdf-differences` is
  pinned by commit, so the witness below is already exactly reproducible and carries **zero
  bytes** into this history. A test names the path and skips where the submodule is absent,
  which is `tests/corpus.rs`'s own pattern. Running total against the 20 MB budget: **0 MB.**
- **The budget therefore binds SafeDocs alone**, where no submodule is possible. Nothing was
  promoted from it this session: the two reports its 24 documents produced are known
  populations with existing witnesses (`doc/todo/23`), and a second witness for a population
  already named buys nothing.

A crasher remains a different thing, small and always committable, and there was none.

## The short test, run

`safedocs fetch --archive 0000 --count 24 --download`: **19.9 MiB transferred in 13 s**,
inflating to 22.8 MiB, 24 documents, every CRC-32 matched. The corpus's own `README.md` claims
its file numbering is derived from each file's SHA-256, and the manifest confirms it —
`0000000.pdf` hashes to `00000836…`, `0000023.pdf` to `00003d58…`.

**24 documents in 0.4 s: 0 unopenable, 0 locked, 0 encrypted beyond us, 0 pageless, 2
incomplete, 0 slow.** Both reports are §11's transparency, both are populations `doc/todo/23`
already names: `0000004.pdf`'s page group composites in `/DeviceCMYK` (§11.4.7, the population
ADR 0251 found) and `0000023.pdf` is a non-isolated group with an element that blends (§11.4.4).
One document shows 51 codes that reach no glyph while reporting nothing — `doc/todo/21`'s
measurement, from the web rather than from pdf.js for the first time.

Over the three new submodules, and this is the round's baseline to record rather than a gate:

| corpus | documents | complete | reported |
|---|---|---|---|
| `pdf20examples` | 7 | **7** | — |
| `pdf-differences` | 37 | 30 | 7 |
| `pdfbox` | 64 | 63 | 1 |
| SafeDocs `0000`, first 24 | 24 | 22 | 2 |

Five of `pdf-differences`' seven are the point of the file — the `UnknownFilter` set encodes
one stream apiece with a fake `/XXXDecode` and its `README.md` says which of them a reader
should survive. `pdfbox`'s one is `PDFBOX-4372-…-p4_reduced.pdf` reaching `MAX_FORM_DEPTH`.

## The finding, and it is the one that matters

`UnknownFilter-PageContentStream.pdf` came back **complete**: 0 commands, `unsupported []`. A
page that draws nothing and says nothing is the silence `CLAUDE.md` spends its rounds removing,
and it took one run of a corpus this tree had never seen.

The mechanism, checked rather than guessed. Object 10 is the page's content stream and its
dictionary ends with **one `>`** where §7.3.7 requires two, so it does not parse. §7.3.10 then
applies — "[a]n indirect reference to an undefined object shall not be considered an error by a
PDF processor; it shall be treated as a reference to the null object" — and §7.3.9 finishes the
job: "Specifying the null object as the value of a dictionary entry … shall be equivalent to
omitting the entry entirely." Table 31 says an absent `/Contents` means "the page shall be
empty". So the blank page is *conforming*, and `Page::content_with_report` had one line
implementing exactly that: `if !matches!(part, Object::Null)`.

**The silence is not conforming, because it is not the standard's question.** A page whose
producer named a content stream and got a blank page is not a page whose producer stated none,
and `Unsupported::Content`'s own doc comment had said so since it was written: "[w]ithout this
report a page compressed with a filter we do not implement is indistinguishable from a page the
producer meant to leave sparse." This file is that sentence exactly. `poppler` prints *Syntax
Error: Illegal character '>'*; the same file with `>>` written in reports
`Content { Undecodable { filters: ["XXXDecode"] } }` from this tree, which is what proves the
report path was intact and the reference never reached it.

So `content_with_report` keeps each `/Contents` entry **as written** beside the resolved one,
and a `Object::Reference` that resolves to `Object::Null` becomes `ContentIssue::Unreachable`.
A literal `null` and an absent entry stay silent, because those are the file stating nothing.
This is ADR 0255's argument one clause over — a name the file never defines, said out loud —
three sessions later.

**It costs the pdf.js corpus nothing**: 70 incomplete before and 70 after, so no document in
974 does this, which is the sharpest thing the round has to say about why a second corpus was
worth a session. Six tests in `crates/pdf-model/tests/contents_entry.rs`, one of them the real
witness. Ledger rows §7.3.9, §7.3.10 and §7.7.3.3 updated.

## Consequences

- The world this tree is measured against is 974 + 108 documents and a command that fetches
  more without being able to fetch everything.
- A round may now take a chunk from `safedocs` the way `doc/todo/00` takes a page from the
  ambiguous bucket. `doc/todo/03` is rewritten to that.
- `format-corpus` is owed a licence reading, not a decision.
- The gates did not move except where the round's own work moved them: tests 1525 → 1539,
  citations 6190 → 6203, quotations 582 → 583, and every corpus-scale count identical.
