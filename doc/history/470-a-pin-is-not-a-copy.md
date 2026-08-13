# 470 — A pin is not a copy, and a node is not a page

**Finding.** The project owner reversed the licence caution three rounds had been holding —
*add as many submodules as you want unless we can clearly deduce from their licence that we are not
allowed to, we don't even republish* — so `openpreserve/format-corpus` is the fourth submodule under
`doc/corpora/`, three of its five PDF directories, pinned at `366f068c`. Running this tree over it
left exactly one file blank *in silence*, which is the one `doc/todo/03` §7 had diagnosed and not
taken: `T02-02_005_page-tree-no-kids.pdf` states `/Type /Pages /Count 1` with no `/Kids`, and this
reader drew that dictionary as a page. Table 31 makes `/Type` required of a page object and names
`Page` or `Template`; Table 30 makes `/Kids` required of a node. Both readings agree that object 1
is a node with no children rather than a page, and with the tree emptied the recovery scan finds the
page its producer wrote. The file now draws *Hello PDF-world!* at the intact ink, 0.807367.

**Date.** 2026-08-13.
**ADR.** [0305](../adr/0305-a-pin-is-not-a-copy-and-a-node-is-not-a-page.md).
**Touched.** `.gitmodules` and `doc/corpora/format-corpus` (new submodule),
`crates/pdf-model/src/page.rs` (`declares_a_node`, four walk sites and `Pages::new`'s `/Count`),
`crates/pdf-model/tests/page_tree_nodes.rs` (new), `crates/pdf-model/tests/corpus.rs`
(`MAX_PAGELESS` 5 → 6, argued), `crates/pdf-model/examples/kidless_node_census.rs` (new),
`doc/conformance/ledger.toml` (§7.7.3.2, §7.7.3.3), `doc/third-party-data.md`, `NOTICE` §3,
`doc/oracle-and-corpus.md` (§2, §2b, §2c), `doc/todo/03-more-corpora.md` (§1, §2, §7),
`doc/todo/README.md`, `doc/environment.md`, `doc/adr/0305-*`, this file.

## What was taken and what was left, and why the second half matters

| directory | files | checked out | taken |
|---|---|---|---|
| `pdf-handbuilt-test-corpus` | 89 | 360 KB | yes — the instrument |
| `pdfCabinetOfHorrors` | 24 | 9.7 MB | yes — CC0 in the directory itself |
| `govdocs1-error-pdfs` | 54 | 63 MB | yes — Govdocs1's own terms permit and ask for a citation |
| `jhove-errors` | 99 | 275 MB | no — size and value; an *absent* grant, not a prohibition |
| `fully-featured-pdf` | 1 | 23 MB | no — one complete file whose distinguishing half is Clause 13 |

Both refusals are stated as size-and-value refusals rather than dressed as licence findings, which
is the failure the owner's rule exists to prevent. 73 MB checked out, about 58 MB of pack, and **no
gate depends on it**: the one test naming a path inside prints that the submodule is absent and
passes, which is `contents_entry.rs`'s pattern.

## The population, before the reading was applied

`doc/todo/03` §7 said this wanted its population counted first, and it did.
`examples/kidless_node_census` walks the page tree with `pdf_syntax` alone — never through `Pages`,
because a census whose predicate is the code under test measures the code — and 2 m 43 s over
everything on this disk:

| population | opened | a `/Type /Pages` node stating no `/Kids` |
|---|---|---|
| SafeDocs `CC-MAIN-2021-31`, all 145 archives | 65 703 | **0** |
| pdf.js corpus + the other three corpora + `doc/` | 1025 | **1** |
| `format-corpus`, three directories | 165 | **1** |

**Zero on the web** is what decided the shape of the fix rather than merely permitting it: the
construct is unreachable by any corpus this project can grow, so the rule is pinned by five pairs
of hand-built fixtures differing in one entry apiece, and the change cannot regress a real document
because there is none to regress.

## The one document besides the witness, and the ratchet it moved

`poppler-937-0-fuzzed.pdf`, whose `/Kids`'s `[` was fuzzed into a NUL — §7.2.3 white space — so the
entry resolves to a bare dictionary rather than an array. It used to be a leaf and drew a blank
page in silence; it now has no first page, and `MAX_PAGELESS` goes 5 → 6.

The ratchet moves loose and the argument is that the page was never ours to count. All three
references reach the same place independently: `poppler` prints *Kids object (page 1) is wrong type
(dictionary)* and writes one 1×1 pixel, `mutool` refuses with *invalid page number: -1*, and
`ghostscript` says *Requested FirstPage is greater than the number of pages in the file: 0*. The
oracle loses no verdict — that document has none — and the corpus gate's incomplete count does not
move at all.

## Gates

`fmt`, `clippy --workspace --all-targets`, `nextest run --workspace` (1685 passed, 11 skipped),
`--doc`, corpus (974 documents, 67 incomplete, 6 pageless), oracle (68 contradicted, 786 ambiguous,
19 no render), text extraction (99.8%, 4 below the floor; pdf.js gate unmoved), `pdfbox` frozen
extraction, dates, XMP, JPEG 2000, quorra corpus, conformance. Every one green, and only
`MAX_PAGELESS` moved.

`cargo test -p conformance` caught three blockquotes written from memory rather than from
`doc/md/` — the checker earning its place again — and the correction is not cosmetic: Table 31's
`/Type` says "shall be Page for a page object **or Template for an invisible Template page**", and
the sentence this round's rule rests on is stronger for naming both, since a node saying `Pages` is
neither.

## What the next round should know

The handbuilt corpus is **spent as an instrument**. All five of its silent blanks are accounted
for: two are the blank the standard asks for, three were defects taken in three consecutive rounds
(ADRs 0302, 0303, 0305). Thirteen of its 89 files draw nothing today and every one of them either
reports or is right to be blank. The same ink assertion pointed at `pdfCabinetOfHorrors` or
`govdocs1-error-pdfs` needs a reference again, because those files do not share a page.

One recovery was considered and declined, and it is written down in ADR 0305 so that a later round
does not re-derive it: a `/Kids` that resolves to a *dictionary* could be read as naming one child,
which would give `poppler-937-0-fuzzed.pdf` its content stream. Its only witness is a fuzzer
artefact every reference refuses, so nothing establishes the recovery is right. The census is the
instrument that would find a real one.
