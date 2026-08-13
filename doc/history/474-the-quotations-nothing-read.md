# 474 — The quotations nothing read

**Finding.** `doc/todo/48` item 3a named a sixth population of quotation — every quotation of the
standard in this project's own Markdown documents — and said no instrument compared a word of it.
`conformance::prose` is that instrument. Its first run produced **thirteen corrections**, three of
them sentences quoted as ISO 32000-2's that ISO 32000-2 does not contain, two of those also standing
in `crates/` in prose the gate does not read, and one a wrong *table* number. Two of its remaining
suspects turned out to be `doc/md/` losing text the PDF has, which is the first evidence for a
caveat that file has carried as advice.

**Date.** 2026-08-13.
**ADR.** [0309](../adr/0309-the-sixth-population-of-quotation-and-what-it-accused-the-conversion-of.md).
**Touched.** `tools/conformance/src/prose.rs` (new), `tools/conformance/src/bin/quotations.rs`
(new), `tools/conformance/src/lib.rs` (one line), `tools/spec-errata/src/lib.rs`
(`document_landings`, `Quoted::Document`, `Error::Documents`), `tools/spec-errata/src/main.rs`,
`crates/pdf-model/src/content.rs`, `crates/pdf-model/src/appearance.rs`,
`crates/pdf-model/tests/corpus.rs`, `crates/pdf-syntax/src/document.rs`,
`doc/conformance/ledger.toml` (§7.5.5's note, one word), ADRs 0009, 0030, 0071, 0080, 0089, 0092,
0097, 0122, 0130, 0145, 0192, 0204, `doc/todo/13-the-transfer-function.md`,
`doc/todo/48-the-specification-we-check-against.md`, `doc/todo/02-every-round.md`,
`doc/adr/0309-*`, this file.

## What was built

`cargo run --release -p conformance --bin quotations`. Every Markdown document under `doc/` that
this project wrote — not `doc/md/`, which is the standard itself, not the third-party checkouts,
not the errata file `spec-errata` generates — judged against all fourteen specifications, with ADR
0249's discriminator: five words and half the quotation matched, then a divergence.

Two things beyond the ledger's sweep, and both earned their place on the first run:

1. **The standard's continuation is printed under each divergence**, read back through an index from
   the folded text to the spaced one. A finding stops being "this is wrong" and becomes the
   correction, written out.
2. **Two more foldings**, each of which removed a class of noise rather than a case: the two
   quotation marks (a quotation delimited by `"` cannot contain one, and every cross-reference in
   the standard carries one), and the Mathematical Alphanumeric Symbols block, because ISO sets
   every variable in a formula in math italic and a document types the letters on the keyboard.

## The corrections

| what it was | where | the standard |
|---|---|---|
| `/TR2` "shall be used in preference to `TR`" | ADR 0204, `content.rs` | Table 57: "[i]f both TR and TR2 are present … TR2 shall take precedence". The invented sentence is §8.11.2.2's, about `/VE` |
| "the first and last vertex shall be implicitly connected" | ADR 0192, `appearance.rs` | §12.5.6.9 says the opposite sentence about the *other* subtype. The inference was right; the quotation was manufactured from it |
| §7.5.5's `/Size` "one greater than the highest object number used in the file" | ADR 0130 | half of Table 15 and half of Table 17, two subclauses apart |
| `/Root` "[t]he catalog dictionary for the PDF **document**" | ADR 0097, `document.rs`, `corpus.rs`, ledger §7.5.5 | Table 15's word is **file** |
| Table 31 NOTE 2 with a word inserted | ADR 0080 | "the catalog dictionary", not "the document catalog dictionary" |
| `/H` attributed to **Table 192** | ADR 0122 | `/H` is Table 191's; Table 192 is the appearance characteristics dictionary one row below |
| four unmarked elisions | ADRs 0009, 0030, 0071, 0092 | a parenthetical, two cross-references, and two sentences with two more between them |
| a full stop closing a sentence the standard continues | `doc/todo/13`, ADR 0122 | §10.1 continues "(for example to simulate …)" |
| a bracketed substitution inside the quotation marks | ADR 0089 | `[Table 255]` moved outside them |
| a sentence Errata Collection 3 struck out | ADR 0145 | Table 33 still contains it; the citation moved there |

## Why it is not a gate

ADR 0249's argument, one population over, and the ADR has it. What is worth carrying here is the
number that decides it: of the quotations read, more than half share too little with any
specification to be a quotation of one — these documents quote `CLAUDE.md`, the project owner,
another renderer's output, a test's name, and their own retired wording. The last is not noise but
*correct writing*: `doc/todo/01`'s fourth sweep is "this row said *X*", and eight of the twelve
surviving suspects are exactly that, including two in ADR 0249 itself. **This round's own ADR adds
five more of them**, because naming a wrong sentence means quoting it.

## Suspect the conversion first, now with witnesses

`doc/todo/48` has warned that a miss may be `doc/md/`'s fault. Two of the twelve are, and both were
settled with `pdftotext -layout` over the PDF in `doc/` before anything was edited:

- **Table 29's `/OpenAction` row is truncated in the conversion.** The standard ends it "the document
  shall be opened to the top of the first page at the default magnification factor"; `doc/md/` stops
  the row at "an array defining a destination". ADR 0054's quotation was right and the file it was
  checked against was missing the sentence.
- **Table 179's columns are shifted through the middle of a description.** `doc/md/` writes "…to form
  an open | | ClosedArrow | arrowhead"; the PDF sets `OpenArrow`'s description whole across two
  lines. ADR 0192's quotation was right.

## The errata question over the same population

`spec_errata::document_landings`, so that ADR 0254's question — does a quotation quote a sentence
Errata Collection 3 struck out? — reaches the new population too. Its landings in the cited clause
are almost all correct writing by construction, because `doc/errata-read.md`'s entire subject is the
struck text and five more are ADR 0092 on §8.9.5.4, which `doc/todo/48` item 1 records as the one
clause this tree knowingly implements a retired version of. Two were worth an edit and are in the
table above.

## Discrimination

A checker nobody has seen fail is a checker nobody has tested. One word of a §7.7.3.3 quotation was
changed — `clipped` to `cropped` — planted in `doc/todo/00` and swept; it was reported at 11 of 12
words with the standard's own sentence printed under it, and the plant was removed. The same case
and one for each of the four earlier foldings are unit tests, so a coarsening added later cannot
quietly swallow a finding.

## Gates

`fmt`, `clippy --workspace --all-targets` (silent), `nextest --workspace` **1708 passed, 11
skipped**, `cargo test --workspace --doc`, and `cargo test -p conformance` (**6959 citations, 686
blockquotes, 875 ledger rows, 0 unreviewed, 0 file-only evidence**). No page-drawing gate was owed:
every change in `crates/` is a comment.

## Two things the next round should know

1. **The shared build directory is not safe when five worktrees run at once.** `cargo clippy
   --workspace` failed here with `unresolved import conformance::prose` — an error about a module
   that exists — while `cargo clippy -p conformance` passed, repeatedly and in both directions. The
   same commands under `CARGO_TARGET_DIR` pointing somewhere private were clean every time. The
   worktrees share `/home/AI/cargo-target/pdf-viewer/` and a parallel round's `conformance` build is
   in there beside this one's. **A gate failure naming something you just added is worth re-running
   in a private target directory before believing it.**
2. **A `§` in a doc comment is a citation wherever it is.** The sentence "the nearest `§` on or
   before the quotation's line" failed `every_citation_names_a_clause_that_exists` — the checker
   reads a bare section sign as a malformed citation, which is exactly what it is for. Write "the
   nearest clause citation" instead.
