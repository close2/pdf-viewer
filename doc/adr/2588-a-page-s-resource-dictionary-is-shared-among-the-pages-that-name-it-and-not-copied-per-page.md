# 2588 — A page's resource dictionary is shared among the pages that name it, and not copied per page

Status: implemented. Session 1877, slot 7 of batch 128, a parsers round (robustness). ADR 2589 and
Q753 are unused. I worked in `scratchpad/r1877/`.

## Context

Principle 3 asks for explicit memory budgets. `pdf_model::Page::resources` was an owned
`Dictionary`, and `Document::get` can hand an object out of its cache only as a copy of the whole of
it (`pdf-syntax/src/document/objects.rs`, `get`). So every page that named one resource dictionary
held its own deep copy, and `Pages::every` (ADR 1953) multiplied the dictionary by the pages: a
document of N pages over one W-entry dictionary cost N × W entries. ISO 32000-2 §7.8.3 describes
exactly that document as the producer's choice to make:

> If the content streams of multiple pages require exactly the same set of resources, a single
> Resources dictionary may be shared between them by using indirect references.

and §7.7.3.4's inheritance (Table 31's `/Resources` is "( Required; inheritable )") is the second
route to one dictionary for many pages.

## Decision

1. **`Page::resources` is an `Arc<Dictionary>`.** Every reader that only reads takes `&Dictionary`
   through deref and is unchanged; the interpreter's page resources (`content.rs`) and a form's
   restore (`content/xobject.rs`) now take a second handle where they took a deep copy per
   interpretation, and `pdf-archive`'s survey holds the page's handle rather than a copy.
2. **Sharing follows the reference, never the content.** A walk keeps one `Arc` per object a
   `/Resources` entry names (`Walk::stated_resources`, `page.rs`), read through the new
   `Document::stated_key_of`, which is `get_key_of` before §7.3.10's chain is followed
   (`get_key_of` is now that plus `resolve`, so its answers are unchanged). Two equal dictionaries
   written separately stay two. An inherited dictionary is the ancestor's handle.
3. **Bounded by the walk.** The memo holds at most one entry per node visited, so
   `MAX_NODES_VISITED` bounds it, and it is dropped with the walk. `Pages::get` makes a fresh one
   per call, so a caller looking pages up one at a time holds no more than before.
4. **The document stays immutable.** Nothing in `pdf_syntax::Document` changes; a caller that
   edits a page's resources does it through `Arc::make_mut`, which copies the dictionary the other
   pages still share (`tests/shared_resources.rs`).

## Measurement

`quorra-transform archive <in> --to 4 -o <out>`, release builds of HEAD and of this change, each in
its own export; children's `getrusage` high-water, under `tools/bounded.sh --tree 6`. Inputs: N
pages under one node, one `/ExtGState` of W = 20 000 small graphics states, every page naming it as
`3 0 R` ("ref") or inheriting it from the node ("inherit"). Wall seconds were taken at load
averages above 30 and are not clock figures.

| N | shape | HEAD | this change |
|---|---|---|---|
| 100 | ref | exit 0, 2 629 548 KiB, 78.0 s | exit 0, 93 408 KiB, 5.2 s |
| 100 | inherit | exit 4, 2 641 004 KiB | exit 4, 93 832 KiB |
| 1 000 | ref | exit 2, 4 165 308 KiB (worker out of memory) | exit 0, 97 468 KiB, 32.8 s |
| 1 000 | inherit | exit 2, 4 166 712 KiB (worker out of memory) | exit 4, 97 680 KiB |
| 2 000 | ref | exit 2, 4 167 132 KiB (worker out of memory) | exit 0, 101 372 KiB, 50.3 s |
| 2 000 | inherit | exit 2, 4 166 948 KiB (worker out of memory) | exit 4, 102 264 KiB |

Exit 4 is a refusal that HEAD makes too and that does not depend on width: from 33 pages that
inherit their resources the output still fails `graphics/content-streams-have-an-explicit-resources-dictionary`
(32 pages convert, 33 do not, at W = 4), the shape of trap 243 — `pdf-archive/src/finding.rs`'s
`MAX_KEPT` is 32 and `pdf-transform/src/archive/sites.rs` reads the findings as the population of
pages to give a dictionary. Named, not fixed here.

## The page turn moves, downward

The interpreter took a deep copy of the page's resources at every interpretation and now takes a
handle, so `turn_path` can move. One `--clock --nice 0` hold ran HEAD's export and this one's
alternately, twice each (load 5.7 falling to 3.9). This change passed both runs (exit 0, 2 passed);
HEAD failed both on bands it left (issue12841's turn and step, a Type 3 document's turn and step,
bug1721218's step). The interpretation-heavy pages are lower in both pairs: issue12841's turn
50.64 and 39.19 ms at HEAD against 34.16 and 33.34; bug1721218's turn 152.57 and 155.55 against
135.57 and 138.15, its step 101.43 and 101.83 against 86.75 and 87.27; bug1743245's turn 45.24
and 49.38 against 39.58 and 40.03. The pages whose time is the encoder's are level. No band is
narrowed here; that is the merge's ratchet.

## Copies left, named

Each is a copy per call that this change keeps byte for byte, because its consumer wants an owned
`Dictionary`: `pdf-model/src/colourants.rs` 422, `named_page.rs` 348, `view/pages.rs` 534;
`pdf-transform/src/images.rs` 855, `redact.rs` 340 and 1740, `redact/frames.rs` 153;
`pdf-archive/src/survey/fonts.rs` 168 and `survey/xobjects.rs` 89 (a form or Type 3 font stating
no resources of its own). The root is `Document::get`'s copy out of a cache of owned objects; a
cache of shared objects is a `pdf-syntax`-wide change and its own decision.
