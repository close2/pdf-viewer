# 0871 — The read side has a corpus walk, and it is the confined transport that walks

Session 914. Status: **accepted**. The second of this round's two records: the instrument, what
shape it has, and why two of its choices are not obvious.

## Context

`doc/todo/58` §5 has carried this since the core landed, and session 911 turned it from a gap into
an argument. That round mounted the face by hand and found ten defects; **the three deepest were
reads**, and none of them was reachable by `crates/pdf-vfs/tests/a_face.rs`, which carries four
committed documents:

- `ls images/0060/` killed the confined worker — one page in one document with two images;
- `ls -l pages/` on a 1023-page document produced 1674 `EIO`s, over an ordering defect in §14.7's
  structure carry that no smaller document showed;
- a second `ls -l` cost more than the first.

Meanwhile the *write* side has had a 974-document walk since session 909 (ADR 0860), and RFC 0002's
five other verbs have six between them. The asymmetry was the finding: the side of this crate that
a file manager actually exercises — every `ls`, every `stat`, every `cat` — was measured against
four documents.

## Decision

`crates/pdf-vfs/tests/read_corpus.rs`, in `write_corpus.rs`'s shape and under `doc/todo/02` §2's
`pdf-vfs` row. For every corpus document, over one mount: **the whole layout listed, every entry
`stat`ed, every file read, and then the listings and the files read again.**

### 1. Each file is held against the generator the layout table names, computed here

RFC 0003 §7 forbids this crate a second implementation of anything, and `crate::layout::Generator`
names the delegate of every row. So the walk does not *describe* what a file should be; it runs the
delegate:

| the tree's file | is held to |
|---|---|
| `pages/NNNN.pdf` | `Plan::Split`, `Pieces::EachPage`, that page — byte for byte |
| `renders/DPIdpi/NNNN.png` | `Plan::Render` at that resolution — byte for byte |
| `images/NNNN/NAME` | one output of `Plan::Images` for that page, under that output's own name |
| `text/NNNN.txt` | `pdf_model::interpret`'s readback — byte for byte |
| `text/document.txt` | those joined by a form feed |
| `attachments/NAME` | what `Plan::Attachments`'s `Save` writes for the name the *document* files it by |
| `meta/info.json` | `pdf_model::metadata::Information`, entry by entry, through `pdf_transform::json`'s own escaping |
| `meta/xmp.xml` | the catalog's `/Metadata` stream, decoded, byte for byte |

The listings are held to the layout's own spelling — RFC §4's "zero-padded ordinal; width from
page count" with four the floor — in **order**, so a dense sequence is a property rather than a
count. `images/NNNN`'s listing is held to the extraction's whole set of output names, which is the
departure `crate::layout` records: a listing and a read there are one call and cannot disagree.

The refusals are counted by reason rather than failed on (trap 11), and the *pairing* is what is
asserted: a file the tree produced where the delegate refused is bytes this crate invented, and a
file the tree refused where the delegate produced some is an answer the tree is hiding. Both are
failures; a refusal on both sides is a column.

### 2. The transport under test is the confined one

`doc/todo/58` §4 says no face ships before the confined worker exists, because a mount is entered
by anything that touches a folder. The walk therefore mounts on `ConfinedWorkers` over a
`FileBacking` — the posture a face has, the document crossing as a descriptor (ADR 0812) — while
every expectation above is computed **in this process** by the same `pdf_transform` plan
`worker::InProcess` runs.

That makes each comparison two things at once: the delegation check RFC §7 asks for, and the
two-transport comparison `tests/confined.rs` makes on four documents, here made over the corpus.
It cost one walk rather than two, and **it is what found ADR 0870's defect on the first sixty
documents**: four of them killed the worker outright, and every in-process test in this tree passed
while they did.

The three `meta/` files have no plan to compare against — their JSON is composed inside
`crate::worker` — so those are additionally compared against a second tree over the same file
mounted in process. Three small files a document, and it is the only place the two transports are
asked the same question directly.

### 3. The walk puts itself in the worker's font posture, and that is a *finding* rather than a convenience

ADR 0870's consequence: a confined worker cannot read the machine's fonts and says so before it is
confined, so a document naming an uninstalled face is substituted from the compiled-in faces there
and from the machine's here. Comparing those two would be comparing two machines, and would file a
*fidelity* difference in the column meant for a *transport* difference. So the walk calls
`pdf_font::substitute::no_machine_fonts()` in its own process before it reads anything.

**This is the one place the walk is weaker than it looks, and it is written down rather than
absorbed**: it measures the transport exactly, and it cannot measure the substitution gap ADR 0870
opened. What closes that gap is the broker supplying the face; `doc/todo/58` carries it, and this
walk is the instrument that will say when it is closed — the line comes out, and every page has to
agree again.

### 4. What repetition is held to, and why it is a count rather than a clock

Round 911 found a second `ls -l` costing more than the first and measured it with a stopwatch; ADR
0865 §3 fixed it by keeping sizes in the cache past the eviction of their bytes, and its own test
"failed for real on its first run" only once it counted generations instead of comparing sizes — a
size is the same number whether it was remembered or recomputed.

So this walk asserts two things and neither is a clock:

- **A second `stat` of a file this generation has already produced generates nothing.**
  `Vfs::generated()` is read before and after the second pass, and any increase fails the run. A
  corpus is where the document too large for the cache's byte budget is found, which is exactly
  the population whose `stat` is expensive.
- **A second reading is the first reading.** RFC §5.4's generation key is asked before every
  answer, so an unchanged file answers the same bytes; the walk digests every file of the first
  pass and re-reads all of them.

A wall clock would have been the obvious instrument and is the wrong one on a machine running
three rounds (`doc/todo/02` §2's own warning), and it would have passed on any tree where the
second pass was merely *fast*.

### 5. One ceiling, and its figure is the corpus's

`PAGES_READ` bounds the pages whose files are read and `stat`ed; the listings are always whole,
whatever the page count. The corpus is 1747 pages over 973 documents, of which one holds 352 and
two more hold 55 and 23 — so the ceiling costs a few per cent of the pages and takes the longest
document's serial run, which is one rayon task, from 352 pages to 16. Every page of the great
majority of documents is still read at both resolutions.

The alternative was no ceiling, and it is worth saying why that is not obviously better: one
document would then decide the walk's wall clock, and a gate whose duration is one file's is a gate
people stop running.

## Consequences

- `doc/todo/02` §2 gains the line, beside the write side's, and the `--bins` build above it already
  covers this one (trap 10, same binary).
- `doc/todo/58` §5's read-side shortfall is closed and replaced by what the walk *did not* measure:
  the substitution gap of ADR 0870, and the documents the ceiling's tail leaves at 16 pages.
- The walk is where the next read-side defect will be found, which is the reason to keep its
  columns per layout row rather than as one total: a count that falls tells you which generator.
