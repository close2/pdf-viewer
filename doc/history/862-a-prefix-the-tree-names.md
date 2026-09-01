# 862 — A prefix the tree names, and a producer's `shall` a reader can still say out loud

2026-09-01. Argued in [ADR 0786](../adr/0786-a-prefix-the-tree-names.md).

Touched: `crates/pdf-model/src/page.rs`, `crates/pdf-model/src/content.rs`,
`crates/pdf-model/src/structure.rs`, `crates/pdf-model/tests/damaged_page_dictionaries.rs`,
`crates/pdf-model/examples/absence_audit.rs`, `crates/viewer-core/src/notes.rs`,
`doc/conformance/ledger.toml`, `doc/checks/fixed-documents.toml`,
`doc/traps/parsers-and-streams.md`, `doc/todo/03-more-corpora.md`,
`doc/todo/48-the-specification-we-check-against.md`, `doc/todo/README.md`,
`doc/errata-read.md`, `doc/adr/0786-a-prefix-the-tree-names.md`.

## 1. The tree-named prefix — `doc/todo/03` §34's first door, taken

`Pages::new`'s recovery ran from the scan, so a damaged child that the `/Kids` array *names* was
never offered to `Document::damaged_dictionary` on any terms but its own `/Type /Page`. It now runs
from the tree as well: `tree_named` descends from the catalogue's `/Pages` collecting the object
numbers the `/Kids` arrays state, and `scan_for_pages` takes a prefix that does not declare itself
where the tree names it **and** it holds one of Table 31's page-only entries.

§34's argument is taken as written and the round found the sentence that finishes it — the one
after Table 30, which §34 cited only by subclause: a page tree node "may contain further entries
defining inherited attributes for the page objects that are its descendants", so a node's
legitimate keys are Table 30's four and §7.7.3.4's four and **no others**. That closure is what
makes `/Contents` or `/Annots` in a prefix a statement that the object is a page.

Three things came out of building it that §34 did not have:

- **The discriminator is a positive list of Table 31's keys, not the complement of Table 30's.**
  `poppler-355-0.pdf`'s prefix holds `/WinAnsiEncope`, a key in neither table: under the complement
  it would be recovered, under the list it stays refused — which is what §34 argued for and what
  the complement would have quietly undone. §7.3.7 is the reason: a subset says what the producer
  wrote and nothing about what it did not.
- **The tree is walked from the catalogue's `/Pages`, not from every `/Kids` in the file.** The
  census may do the latter; a reader doing it would collect form-field and name-tree kids. And the
  obvious tightening — follow a node stating `/Type /Pages` — **loses three of the five witnesses**,
  whose roots state no `/Type` at all. Table 29 makes the catalogue's entry the root by
  declaration, which is what makes the walk sound anyway.
- **The report names which door the page came through.** `DictionaryDamage::identification` is a
  two-variant enum and `Unsupported::PageDictionary` says which, because Table 31's `/Type` off the
  producer's bytes and this reader's inference from §7.7.3.2 are two different claims about a file.

### What it recovered, per witness

All five of §34's, and nothing else on this disk. `standing_count_census` over `batch1`, `batch2`,
`batch3` and `batch6` falls from 12 to 7.

| document | prefix | outcome |
|---|---|---|
| `GHOSTSCRIPT-699521-0` | 4, incl. `/Contents` | **draws** — 795 × 842, `Hello world` in outlined 30pt Helvetica, 10 commands, one report |
| `GHOSTSCRIPT-701846-0` | 4, incl. `/Annots` | the producer's own 500 × 500, blank; `/Contents` is past the damage |
| `GHOSTSCRIPT-698991-0` | 2, incl. `/Contents` | this reader's sheet, blank, with the `/Contents` object reported unreachable — three sentences about a 282-byte file |
| `GHOSTSCRIPT-699018-0` | 1 — `/Annots` alone | this reader's sheet, blank; the one entry is the whole evidence |
| `poppler-192-0` | 2, incl. `/Contents` | this reader's sheet, with `/SH0 is not in /Shading` said by name |

All five pinned in `doc/checks/fixed-documents.toml`. **One band is narrower than that file's
convention and the file now says why**: `699521`'s marks move the mean by 0.307 of a level, so ±1.0
would admit a blank page and pin nothing but the report beside it.

Door 2 — resynchronising past an unreadable value — is **not** taken and its argument is unchanged.

### Fixtures

Six new pairs in `damaged_page_dictionaries.rs`, on trap 28's discipline, including the two the
trap asks for: the same body under a `/Kids` that names the object and one that does not (one
character apart), and **a tree that still reaches a page with a tree-named damaged object carrying
`/Contents` beside it** — the rightness condition true and the guard false.

## 2. §14.8.6.3's enclosure `shall`, reported

The reading ADR 0375 made in the five-hundred-and-fortieth stands on *whose* `shall` it is: the
sentence opens "[w]hen including mathematics structured as MathML", so it is a producer's. **What
decayed is the step after it** — `CLAUDE.md`'s authoring exclusion says this tree does not *write*
such a tagging and says nothing about reading one, and §14.8.6.2's own file-addressed `shall`
became a report **one round ago** (ADR 0785), one subclause away, on reasoning that transfers word
for word. Four documents had carried "what stays owed is a validator's report" since.

So it is a report. `Tree::mathml_outside_a_formula` counts an element ending at the lowercase
`math` type **in the MathML namespace** with no `Formula` anywhere above it;
`viewer_core::notes::about` says it once when the document opens, beside §14.8.6.2's. Three narrow
readings inside that, each deliberate: the namespace is part of the type (both ways — a foreign
`Formula` is not §14.8.4's either, so the ancestor test goes through `standard_role`); *under* is
read as *anywhere under*, because the sentence does not say *immediately*; and `Math` is not `math`.

**The erratum's second sentence is declined for a firmer reason than the exclusion** — all MathML
types and their attributes having the namespace explicitly defined quantifies over MathML's own
vocabulary, which ISO 32000-2 states nowhere.

Calibrated by three plants, each failing exactly one of six fixtures and none failing a test about
another sentence. **No witness** in `doc/pdf.js`, the four `doc/corpora/` submodules, this
project's fixtures or the 65 944-document `CC-MAIN-2021-31` crawl, measured by
`examples/absence_audit` with the reader that decides the report.

## 3. Ledger and documents

`§7.7.3.2`, `§7.7.3.4` and `§14.8.6.3` each gained the round's reading; all three were and remain
`implemented`, and `§14.8.6.3`'s row loses the validator sentence. Trap 5's §7.3.7 paragraph now
says there are two doors and why the evidence has to be a Table 31 entry *present*.
`doc/todo/03` gains §35; `doc/todo/48`'s item 2 is closed and `doc/todo/README.md`'s line for it
was three closures stale.

## 4. Gates

The full §2 sequence, plus `cargo test -p conformance` and the `fixed_documents` line.

**The `page` fuzz target was run twice and only the second run says anything**, which is trap 24
one step along. The first was the standing recipe — fork mode over the whole 784 MB corpus, under
`timeout 900` — and the timeout killed it inside libFuzzer's own corpus merge, so its exit status
is the timeout's and its output was nothing. **A run that did not finish its merge has not fuzzed
anything, and its silence is not a pass.** The second was aimed instead of broad: a corpus of the
fifteen documents this recovery is about — §34's five witnesses, its six refusals, and ADR 0784's
four — with `-max_total_time=300` and no fork, so the run prints its own count. **242 868 runs in
301 seconds, no crash, no leak, no timeout, and no new artefact under `fuzz/artifacts/page`.** The
2926 inputs it found were folded back into `fuzz/corpus/page`, which is machine-local and outside
the history.

`absence_audit` was run over all three of its populations for the new claim; `--bin pointers`,
`--bin quotations`, `--bin overstated`, `--bin parts` and `--bin undenominated` printed nothing this
round added.
