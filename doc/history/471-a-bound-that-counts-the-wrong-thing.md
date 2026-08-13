# 471 — A bound that counts the wrong thing, and a clamp that said nothing

**Finding.** `doc/todo/10` §3's three defects, all of them, and they turned out to be one sentence:
*a bound must count what its name says, and a bound that refuses must say so.* `MAX_OPERATIONS`
said "operators" and incremented once per **lexer token**, so §7.8.2's rule that an operator follows
its operands made a `c` cost seven and the project owner's 50 MB Bézier drawing was truncated at 19%
while stating 814 705 *fewer* operators than the bound. `flate` and `lzw` answered a decompression
bomb with the prefix they had inflated and no report, because `io::Take` yields end-of-file at its
limit and `read_to_end` calls that `Ok`. And `max_stream_len` was two gibibytes against the confined
worker's four-gibibyte ceiling, over a decode that costs about twice its output, with no bound at all
on the *total* of a page's `/Contents` parts. **The witness draws whole now and the constant did not
move; the unit did.**

**Date.** 2026-08-13.
**ADR.** [0306](../adr/0306-a-bound-that-counts-the-wrong-thing-and-a-clamp-that-said-nothing.md).
**Touched.** `crates/pdf-model/src/content.rs` (the increment site and `MAX_OPERATIONS`' comment),
`crates/pdf-model/src/page.rs` (`content_with_report`'s aggregate, `ContentIssue::TooLarge`),
`crates/pdf-syntax/src/filter.rs` (`FilterRefusal`, the four filters, `ascii85`'s `z` hole),
`crates/pdf-syntax/src/document.rs` (`StreamRefusal`, `decoded_stream_data_reported`),
`crates/pdf-syntax/src/parser.rs` (`max_stream_len`), `crates/pdf-syntax/src/lib.rs`,
`crates/pdf-syntax/tests/stream_length_bound.rs` (new),
`crates/pdf-model/tests/hostile_budgets.rs` (the zero-operand fixture and three new cases),
`crates/pdf-model/examples/content_budget_census.rs` (new),
`doc/conformance/ledger.toml` (§7.4, §7.4.3, §7.4.4.1, §7.4.4.2, §7.7.3.3, §7.8.2),
`doc/adr/0271-*` (the unit, corrected in place), `doc/performance.md`, `doc/todo/03`,
`doc/todo/10` (§1, §2, §3 and the header), `doc/todo/49`, `doc/todo/README.md`, `doc/verify.md`,
`doc/adr/0306-*`, this file.

## The witness, before and after

`tmp/Entwurf.pdf` is the owner's, is not in this repository and is named in no test.

| | before | after |
|---|---|---|
| `pdf-retrieve page … 0` | `LimitReached { limit: "MAX_OPERATIONS" }`, 0.54 s, 380 MB | `complete: true`, `unsupported: []`, 1.36 s, 380 MB |
| `render_at … 1 1.0` | 0.62 s, 381 MB, **7.99%** of the raster inked | 1.54–1.59 s over five samples, 381 MB, **34.64%** inked |
| `mutool draw -r 72` | 2.08–2.31 s, 97 MB, 34.88% inked | |
| `pdftoppm -r 72` | 3.38–3.53 s, 19–20 MB, 34.38% inked | |

All three at 1667×474 and all three agreeing about the page's ink to within a quarter of a
percentage point, which is what says it draws *whole* rather than merely more. **We are the fastest
and the least frugal by a factor of four to twenty**, and `doc/todo/10` §1 keeps that as residue
rather than this file.

## The bombs, rebuilt from §2's description

They came out 389 317 and 1 847 467 bytes, both 1029:1 — the sizes §2 records, to the byte — which
is what makes this a measurement rather than a memory.

| | before | after |
|---|---|---|
| **Bomb A**, 0.39 MB → 400 MB, 200 M `n` | 0.81 s, **831 MB**, `MAX_OPERATIONS` | 0.71 s, **831 MB**, `MAX_OPERATIONS` |
| **Bomb B**, 1.85 MB → 1.9 GB | 3.26 s, **3694 MB**, `MAX_OPERATIONS` | 1.18 s, **1095 MB**, `TooLarge { part: Some(0), limit: 1073741824 }` |

Bomb A is unchanged and should be. Bomb B loses 70% of its peak and gains a report that names what
happened. It is still a gibibyte commanded by 1.85 MB of file, and only §5's road D takes that back.

## What the corpora said

**The census is the A/B**, because it counts both quantities in one pass over the same documents —
`content_budget_census`, new, with the operator rule written out again so that it is not the code
under test:

| population | documents | pages | streams | largest stream | largest `/Contents` | past 4 M tokens | past 4 M operators | ratio |
|---|---|---|---|---|---|---|---|---|
| SafeDocs `CC-MAIN-2021-31` | 65 967 | 926 680 | 5 047 187 | 483.84 MiB | 483.84 MiB | 48 | **8** | 3.76 |
| pdf.js + four submodule corpora + `doc/` | 2492 | 10 809 | 56 021 | 202.07 MiB | 163.89 MiB | 0 | 0 | 3.08 |

**The web survey confirms it document by document**: 65 944 documents in 542.1 s — 173 unopenable,
45 locked, 23 encrypted beyond us, 52 pageless, 869 incomplete, 5 slow — with the budget refusals

| bound | session 435 | now |
|---|---|---|
| `MAX_OPERATIONS` | 31 | **8** |
| `MAX_TILES` | 48 | 48 |
| `MAX_FORM_DEPTH` | 4 | **5** |
| `MAX_STATE_DEPTH` | 1 | 1 |
| `ContentIssue::TooLarge` | — | **0** |

Three of the four documents `doc/todo/03` names as `MAX_OPERATIONS` witnesses — `0100034.pdf`,
`2100236.pdf`, `2100253.pdf` — now report **complete**. The zero in the last row is the other half of
the measurement: a gibibyte refuses nothing the web contains.

**The fifth `MAX_FORM_DEPTH` is this round's and it is the best thing in it.** `5712394.pdf` was one
of the 31; with the counter corrected it runs past four million tokens, reaches a form that draws
itself, and is refused by the bound that is genuinely load-bearing — *and it now extracts the title
its producer wrote*. **A bound counting the wrong thing was hiding a real cycle behind a false
refusal.** Confirmed by reverting the one-line change in a scratch build and running that document
alone: `MAX_OPERATIONS` before, `MAX_FORM_DEPTH` after.

`5 slow` is the machine and not the tree — the survey rasterises twenty-four documents at once under
a thirty-second per-document budget, and this run shared the machine with a release build. ADR 0271
recorded the same instrument moving 2 → 0 → 1 over three quiet passes.

## Nothing that is drawn moved, and it is an artefact rather than an argument

`display_list_digest` over the pdf.js corpus, the four submodule corpora and `doc/` — **1139
documents, identical line for line** between a build of `39056dd` and this one. The census says why
it must be: the largest page in those corpora states 547 411 operators and the largest decoded stream
is 202.07 MiB, so neither the operator bound nor the gibibyte can fire. `doc/todo/00` step 7's ink
sweep is therefore not owed.

## Gates

`fmt`, `clippy --workspace --all-targets` silent, `nextest run --workspace` (1695 passed, 11
skipped), `--doc`, corpus (974 documents, 67 incomplete, 6 pageless, 0 slow), oracle (905 agrees, 68
contradicted, 786 ambiguous, 1 our geometry, 2 reference geometry, 13 not comparable, 19 no render,
undiagnosed list empty, 99.8% cache hit rate), text extraction (99.3%, 24 010 of 24 189 words, 22
below 90%) and the frozen PDFBox comparison beside it, dates, XMP, JPEG 2000 (14 codestreams
byte-identical to OpenJPEG), quorra corpus (956 pages: 918 agree, 37 differ, 1 refused, 18 not
comparable), conformance (875 rows — 417 implemented, 240 partial — 6924 citations, 683 quotations).
Every one green and nothing moved but the counts this round added.

`cargo test -p conformance` earned its place again: the Table 31 blockquote in the new
`/Contents`-total test cited "Table 31" and no clause, and an unattributed quotation is one nothing
can check. It is `ISO 32000-2 §7.7.3.3`'s.

**Both new filter tests were confirmed to fail with the defect put back**, which is the only thing
that establishes a test guards what it claims — and the third hole was found the same way: asserting
that all four stream filters answer a bound with the same variant failed on `ascii85`, whose `z` arm
reached the length check by way of a `continue`.

## What the next round should know

- **`doc/todo/10` §5 is untouched and the choice is still the owner's.** No deadline, no callback, no
  streaming lexer, no boundary message, and the confinement is not shipped. Each road was checked
  against what landed and none is foreclosed; road **D** arrives with the distinction its own caveat
  demanded already made, because `FilterRefusal::TooLarge` is exactly the report a window-fed decoder
  owes when its consumer stops.
- **Three residues are written into §3** rather than left implied: a decode still costs about twice
  its output, `Document::image_stream` still drops the *reason* a filter refused, and a ceiling
  breach in the confined worker is still indistinguishable from a crash.
- **The build directory passed a hundred gigabytes** (106 G), so `doc/todo/02` §5a's sweep is due —
  never `target/tmp/`, which holds the 1.6 G reference-render cache.
