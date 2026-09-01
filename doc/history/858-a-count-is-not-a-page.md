# 858 — A `/Count` is not a page

2026-09-01. Argued in [ADR 0782](../adr/0782-a-count-is-not-a-page.md).

**The finding**: `Pages::new`'s recovery scan was guarded on `/Count`'s *claim* while its own
comment stated the right rule — "[i]t runs only where the tree produced nothing" — so a root
stating a positive count over children that are not in the file produced no page, ran no scan and
reported nothing, and `len()` went on repeating the number. Settled against Table 30's own cell,
fixed, measured on the launch path, and pinned.

Touched: `crates/pdf-model/src/page.rs`, `crates/pdf-model/src/measurement.rs`,
`crates/pdf-model/tests/page_tree_nodes.rs`, `crates/viewer-core/tests/headless.rs`,
`doc/conformance/ledger.toml`, `doc/checks/fixed-documents.toml`, `doc/todo/03-more-corpora.md`,
`doc/adr/0782-a-count-is-not-a-page.md`.

## The reading, which the round was told to settle first

ADR 0782 has it in full. In one paragraph: Table 30 makes `/Count` "redundant" and the `Kids`
arrays and their descendants what "definitively determines the number of descendant pages", and the
`shall` keeping the two consistent is the **writer's** — so a node with no reachable descendants has
no pages, whatever the entry holds. §7.7.3.1's `shall` about "any form of tree structure built of
such nodes" is about the shapes a *valid* tree takes and does not reach a `/Kids` naming an object
that is not in the file, so the recovery is a documented choice rather than a derivation, exactly as
it has been since the hundred-and-seventh session.

Two lines the round had to draw, and it drew the second one wrong first:

- **The recovery runs where the tree yields no page, never where it yields fewer than `/Count`
  claims.** A tree that produced one page of five has stated an order and a set; a scan's ascending
  object numbers would substitute an invented order for a stated one. Trap 5's
  additive-or-substitutive test, applied to an order rather than to a mark.
- **`len()` moves only where the recovery *found* pages.** The first draft made a document whose
  tree and scan both yield nothing report zero pages, and
  `viewer-core::objects_lost_inside_a_damaged_object_stream_are_said_out_loud` failed on it: a
  document with no page on the screen has nowhere to put a page report, so the sentence naming the
  objects a damaged §7.5.7 stream lost had nothing to attach to. The correction is in the cell
  already quoted — the determination is made "by examining the tree itself", and a reader that
  *refused* a `/Kids` entry has examined nothing, so the number is unknown rather than nought.
  `/Count` stands as the one statement in evidence and each page refuses out loud.

## What it costs, which is why §5 ran first

`reaches_a_page` is `count_leaves`'s walk stopped at the first leaf, and it builds no page. The two
conditions are ordered by cost: `count == 0` short-circuits a pageless file straight to the scan,
and the probe runs only where `/Count` was believed without a walk.

| instrument | before | after |
|---|---|---|
| callgrind, `pdf-retrieve document`, `tracemonkey_annotation_on_page_8.pdf` (14 pages) | 1 704 468 | 1 709 588 (**+5120**, +0.30%) |
| callgrind, same, `ISO_32000-2_sponsored_EC3.pdf` (1023 pages) | 172 809 735 | 172 734 412 (**−75 323**, −0.044%) |
| `--trace` first present, ISO 32000-2, `Xvfb` + `lavapipe`, three launches | 137.2 / 159.7 / 132.3 ms | 135.7 / 151.6 / 137.6 ms |
| `--trace` document joined, same runs | 80.5 / 97.5 / 83.0 ms | 71.0 / 93.7 / 89.3 ms |

The launch timeline is one instrument's spread either side and says nothing; the callgrind pair is
the attribution. The large document comes out **ahead**, because the object the probe resolves is
page one's own and it is still warm when `get(0)` asks for it — which is also why the probe is a
descent rather than a `find_leaf` that would have built the page twice.

## Why step 7 is not owed, argued rather than assumed

`find_leaf`'s `/Count` subtree skip requires `count <= *remaining`, and `remaining` is 0 while
looking for page one — so no skip can happen there, and `find_leaf(remaining = 0)` descends exactly
as `reaches_a_page` does. Wherever the probe says *no page*, `get(0)` was already `None`; `get`
takes the recovered list only when it is non-empty, and `count` moves only then. So a page can
appear where there was none and no page that drew before can draw differently.

The empirical half, with its corpus named: of the **974** documents in `doc/pdf.js`, **not one**
claims a positive page count and fails to produce page one — so nothing in the population every
ratcheted gate walks is in the category this change reaches.

## The witnesses, and the instrument that named them

`doc/todo/03` §31 named four documents. **One draws.**
`batch1/PDFBOX/PDFBOX-4623-1.pdf` — root object 2 states `/Count 1 /Kids [2 0 R]`, its own kid —
now renders object 3's *Hello World* at 48 pt on the 595 × 842 sheet object 2 states, taking its
`/MediaBox` and its `/F1` by §7.7.3.4 inheritance up its own `/Parent`. Ink 1.315, pinned in
`doc/checks/fixed-documents.toml` (46 rows, 0 absent).

**All three references fail on it.** `poppler` prints *Syntax Error: Loop in Pages tree* and writes
a 1 × 1 image; `mupdf` prints *format error: cycle in page tree*, then *Page tree load failed.
Falling back to slow lookup*, then the same error again, and writes an empty file; `ghostscript`
prints *Couldn't get page info* and *page not found*. Every mark this tree draws is the file's own
declaration and nothing is tuned to any of them, which is what principle 5 asks of a disagreement.
`mupdf`'s middle line is the one worth keeping: it has a recovery, tries it, and does not reach the
page either.

**The other three are a different defect**, and §31's table could not tell them apart because it was
built from a byte search for `/Type /Page`. Each one's page object is damaged *in its own
dictionary*: `PDFBOX-4339-0.pdf`'s object 3 opens `\xbc` where `<<` belongs; `poppler-742-0.pdf`'s
object 8 writes a `/TrimBox` array that never closes and runs into the stream after it;
`poppler-750-0.tgz-0.pdf`'s object 14 leaves `/ProcSets` unterminated and writes `/Con\x91ents` and
`e@dobj`. No reader gets a dictionary, so neither the tree nor the scan can reach a page — and their
`/Count` stands, which is the loudness half above. `doc/habits.md`'s rule is what separated them:
run the reader and the grep, and the reader is the instrument under test.

## Second track: §12.9.1's other selection rule

`--bin owed`'s reading list opens with §12.9.1, and reading the clause found a `shall` this tree
stated and executed by nobody: "[a]ny measurement that potentially involves multiple viewports, such
as one specifying the distance between two points, shall use the information specified in the
viewport of the first point." `measurement.rs`'s module comment quoted it and left it to "callers
with two points", of which there were none.

`Viewports::distance` carries it out, together with Table 267's `/D` cell, which states the order
the arithmetic goes in — the scale factors "shall be used to convert from default user space to the
appropriate units **before** applying the distance function" — so each axis is converted by its own
array's first factor, `/CYX` brings y into x's units, and the distance is taken there. `/O` and the
`/BBox` corner order take no part, because a distance is invariant under a translation and a
rotation. A `/Y` with no `/CYX` answers with nothing, which is Table 267 refusing the calculation
itself. Two fixtures, both the clause's own numbers (trap 8: the corpora hold one viewport and it is
`GEO`), and the ordering test is built so no single factor applied after a hypotenuse can reproduce
its answer.

§12.9.1 stays `partial` for one named reason: nothing supplies the two points, and no crate under
`viewer-*` names `Viewports`, `measurement` or `distance`.

## The batches

§29's hypothesis is **confirmed**: HTTP `Range` requests get past whatever stops a whole-file
transfer near 4.2 GB. `batch2`'s 512 MiB pieces come back as clean `206`s at exactly the length
asked for, against eleven short or failed whole-file `GET`s across two earlier rounds. The Archive's
throttling is the whole remaining cost — one to a dozen attempts per piece, every failure the
160-byte nginx `504` page or a 107-byte `502`, and a failed piece discarded rather than resumed onto.
The recipe is `doc/todo/03` §32. What landed by the end of this round is
`ls corpus-cache/tika-issue-tracker/`.

## Gates

`doc/todo/02` §2's whole sequence, because `pdf-model` is under every gate. 2877 tests where 2872
ran before — three pairs in `page_tree_nodes.rs` and two in `measurement.rs`. The corpus gate:
974 documents, 0 unopenable, 8 locked, 2 encrypted beyond us, 6 pageless, 63 incomplete, 0 slow. The
oracle took 100% of its reference rasters from the cache, so the background fetch could not have
distorted a reference's wall clock.
