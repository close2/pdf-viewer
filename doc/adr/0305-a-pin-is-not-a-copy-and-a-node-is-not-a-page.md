# ADR 0305 — A pin is not a copy, and a node is not a page

Status: accepted, 2026-08-13. Session 470. Reverses the licence caution ADR 0258 recorded and ADR
0302 left standing; amends §7.7.3.2's and §7.7.3.3's ledger rows and `MAX_PAGELESS`. Changes
nothing about what may be *committed*, which ADR 0258's promotion budget still governs.

## Two decisions, and the first is the project owner's

### 1. Which corpora may be pinned

ADR 0258 examined `openpreserve/format-corpus` and declined it "on licence rather than on size":
its `README.md` grants CC0 "unless otherwise stated", and a grant with an escape clause is not one
this project relies on without reading the escape. Session 467 did the reading — every per-directory
sidecar, tabulated in `doc/oracle-and-corpus.md` §2c — and put the resulting question to the project
owner. They did not answer the question. They replaced the rule that produced it:

> Add as many submodules as you want unless we can clearly deduce from their licence that we are
> not allowed to do so. We don't even republish, so most licence notes don't even apply to us!
> (We should still mention them as a courtesy.)

**The middle clause is the substance and it is a fact about `git`, not a concession.** A submodule
is a URL and a commit identifier in `.gitmodules` and the index. This repository stores none of the
other project's bytes, transfers none of them, and hands a person who clones it nothing but the
address to fetch from — which they then fetch from *that* project's server, under *that* project's
terms, exactly as if they had typed the URL themselves. Every obligation in the table
`doc/third-party-data.md` carries — reproduce the notice, keep the licence with the copy, carry
`NOTICE.txt` — is a condition on **redistribution**, and pinning is not one. What remains is the
courtesy, and the courtesy is met in `doc/third-party-data.md` and in `/NOTICE` §3.

So the test becomes: *does anything clearly forbid this?* Applied to the five PDF directories, with
each sidecar quoted in `doc/third-party-data.md`:

| directory | taken | why |
|---|---|---|
| `pdf-handbuilt-test-corpus` | **yes** | 360 KB; nothing forbids and the root default is CC0; it is the instrument this round's second half rests on |
| `pdfCabinetOfHorrors` | **yes** | 9.7 MB; CC0 stated in the directory itself |
| `govdocs1-error-pdfs` | **yes** | 63 MB; Govdocs1's own terms *permit* — "freely available for research and may be (to the best of our knowledge) freely redistributed" — and ask for a citation, which `/NOTICE` now carries |
| `jhove-errors` | no | 275 MB of published journal articles under **no** grant anybody could make; that is an absent grant rather than a prohibition, so the owner's rule would admit it, and it is left on size and on value — surveying it produced two ordinary reports |
| `fully-featured-pdf` | no | one file, 23 MB, already complete, and what distinguishes it is Clause 13, which `CLAUDE.md` excludes; its unlicensed embedded media is the second reason and not the first |

**Saying why each was left matters as much as the licence reading**, because the failure this rule
was made to prevent is a directory quietly declined as "unclear" when what was meant was "large".
Both refusals above are on size and value, stated as such.

`doc/corpora/format-corpus` is therefore the fourth submodule, pinned at `366f068c`, sparse-checked
out to those three directories: 73 MB of files and about 58 MB of pack, against `doc/pdf.js`'s
350 MB. **No gate in `doc/todo/02` §2 depends on it**; the one test that names a path in it skips
where the submodule is absent, which is `contents_entry.rs`'s pattern for `pdf-differences`.

### 2. What a page tree node is, when it says it has no children

Pinning it is bookkeeping until something runs. `tools/safedocs survey --dir` over the three
directories, and the ink assertion `doc/oracle-and-corpus.md` §2b describes over the 89 handbuilt
files, leave exactly one file blank *in silence*:
`T02-02_005_page-tree-no-kids.pdf`, whose defect is one deleted entry:

```text
1 0 obj
<< /Type /Pages /Count 1 >>
endobj
2 0 obj
<< /Parent 1 0 R /MediaBox [0 0 612 792] /Resources 3 0 R /Type /Page /Contents [4 0 R] >>
endobj
```

`Pages::get(0)` handed back **object 1**. It has no `/Contents`, so the page drew nothing, and
because a page with no `/Contents` is a conforming empty page (Table 31) nothing was reported. The
survey called the document complete.

#### What the standard states

§7.7.3.2 Table 30, on a page tree node:

> ( Required ) An array of indirect references to the immediate children of this node. The children
> shall only be page objects or other page tree nodes.

§7.7.3.3 Table 31, on a page object:

> ( Required ) The type of PDF object that this dictionary describes; shall be Page for a page
> object or Template for an invisible Template page (see 12.7.7, "Named pages").

Object 1 says `Pages`. By Table 31 it is therefore neither a page object nor a template, in its own
words; by Table 30 it is a node whose required children are missing. Both readings agree: **it has
no children and it is not one**. Drawing it as a page was this reader contradicting the file.

#### Why the walk did not see that, and why the reason is still right

`page.rs` decides a leaf by the *absence* of `/Kids`, with the comment "[t]rusting `/Type` instead
would drop pages from files that omit it" — and that is correct about the files it was written for.
The fix keeps it and adds the one asymmetric case: a dictionary with no `/Kids` and no `/Type` stays
a leaf; a dictionary with no `/Kids` and `/Type /Pages` is the file answering the question itself.
`declares_a_node` is that one predicate, asked at all four walk sites and once more in `Pages::new`,
where `/Count` was authoritative over a root that has no children for it to count.

#### The population was measured before the reading was applied

`doc/todo/03` §7 said this wanted its population first, and it did. `examples/kidless_node_census`
walks the tree with `pdf_syntax` alone — never through `Pages`, because a census whose predicate is
the code under test measures the code (trap 8) — and over **65 703** of the SafeDocs `CC-MAIN-2021-31`
documents that open, 1025 pdf.js and specification documents, and the 165 in `format-corpus`:

| population | documents | with a `/Type /Pages` node stating no `/Kids` |
|---|---|---|
| SafeDocs, all 145 archives | 65 703 | **0** |
| pdf.js corpus + the other three corpora + `doc/` | 1025 | **1** |
| `format-corpus`, three directories | 165 | **1** |

**Zero in 65 703** is the number that decides the shape of the fix. The construct does not occur on
the web at all, so no corpus this project can grow will ever exercise it, which is why the test is
five pairs of hand-built fixtures differing in one entry apiece (`tests/page_tree_nodes.rs`). It
also means the change cannot regress a real document: there is none to regress.

#### The one document that moved besides the witness

`poppler-937-0-fuzzed.pdf`, in the pdf.js corpus. Its node states `/Type /Pages` and a `/Kids`
whose `[` was fuzzed into a NUL — §7.2.3 white space — so the entry resolves to a bare dictionary
instead of Table 30's array. It used to be read as a leaf and drawn: a blank page, silently. It now
has **no first page**, and `MAX_PAGELESS` goes 5 → 6.

That ratchet moves in the loose direction and the argument for it is that the page was never ours
to count. All three references, each having read the file independently, reach the same place:
`poppler` prints *Kids object (page 1) is wrong type (dictionary)* and writes one 1×1 pixel,
`mutool` refuses with *invalid page number: -1*, and `ghostscript` says *Requested FirstPage is
greater than the number of pages in the file: 0*. Principle 5's direction of inference holds — the
clause decided it and the agreement is evidence that the clause was read right — and the oracle
loses no verdict, because that document has none.

## What was considered and not done

**A `/Kids` that resolves to a dictionary rather than an array could be read as naming one child**,
which would give `poppler-937-0-fuzzed.pdf` its real content stream instead of no page at all. It
is left, and the reason is that its only witness is a fuzzer artefact every reference refuses:
nothing establishes that the recovery is right, and a recovery rule with no sound witness is a guess
that happens to render something. If a real file ever states one, the census above is the instrument
that will find it.

**No report was added for the recovered page.** Where the tree empties and the scan finds the
producer's page, the document draws correctly and completely; a report there would take a page off
the oracle's judged set for nothing, which is trap 11's whole subject. Where the scan finds nothing,
"no first page" is already loud in every layer that prints a document by name.

## Consequences

- `doc/corpora/format-corpus` is a submodule; `doc/third-party-data.md` and `/NOTICE` §3 carry the
  attributions, including the citation Govdocs1 asks for.
- `T02-02_005_page-tree-no-kids.pdf` draws *Hello PDF-world!* at ink 0.807367, which is the intact
  `hello_world.pdf`'s figure exactly. The handbuilt corpus now has **no file that is blank in
  silence**: of the thirteen that draw nothing, eleven say why and two are right to be blank.
- The pdf.js corpus gate is unchanged but for `MAX_PAGELESS`: 974 documents, 67 incomplete, 6
  pageless. Oracle, quorra, text extraction, dates, XMP and JPEG 2000 gates all unmoved.
- `examples/kidless_node_census` is in the tree, so the population is re-measurable rather than
  recalled.
