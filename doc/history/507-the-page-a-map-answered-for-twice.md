# 507 — The accessibility ratchet, and the page a map answered for twice

**Finding.** ADR 0323's third instrument is built, and on its first run it found that **page one of
ten tagged documents answered a screen reader with nothing at all** — ISO 14289-1, which is PDF/UA
itself, ISO/TS 32001, 32002 and 32005, Well-Tagged PDF, the Tagged PDF Best Practice Guide, PDF
Declarations and three PDF Association application notes. `viewer-core` joined a page *index* to
the page **object** Table 355's `/Pg` names by inverting `pdf_model::Pages::indices`, and that map
deliberately holds an entry for an intermediate `/Pages` node as well as for each page — so the
scan for the first entry with the wanted index handed back a node whenever the node's object number
was the lower one. Every `/Pg` comparison then failed, every element was pruned as belonging to
another page, and §14.7's whole answer was the silence an untagged page gives. `Page::id` — "which
object the page *is*" — is the join now, at all three call sites that had it: the accessibility
tree, §14.8.2.5's logical selection and §12.5.1's tab order.

**Date.** 2026-08-14.
**ADR.** [0342](../adr/0342-the-page-a-map-answered-for-twice.md).
**Touched.** `crates/viewer-core/tests/accessibility_census.rs` (new — the instrument),
`crates/viewer-core/src/viewer.rs` (`page_object`, and the three call sites),
`crates/viewer-core/tests/headless.rs` (one regression test),
`crates/viewer-core/Cargo.toml` (rayon, dev only), `tools/state.sh` (the `accessibility` section),
`doc/conformance/ledger.toml` (§14.7, §14.7.5.2), `doc/todo/05-an-instrument-for-the-interactive-surface.md`,
`doc/todo/31-accessibility-host.md`, `doc/habits.md`, `doc/verify.md`,
`doc/adr/0342-*` (new), this file.

## The counts on the first sound run, which are what a first run is for

`tools/state.sh accessibility`, over 988 documents — the 974 of `doc/pdf.js/test/pdfs` and the 14
specifications in `doc/` — under the gates profile:

```
988 documents in 28.9s: page one of each, and every page of every document with structure
  refused open: 2
structure (§14.7.2's /StructTreeRoot): 103 documents
  tagged (§14.8.1's /MarkInfo /Marked): 90 documents
  one predicate and not the other: 19
pages that answer at all: 1501 of 1557 pages of documents with structure
  the file names elements for the page and the answer is empty: 0
  no /StructParents, and the whole-tree fallback answered nothing: 56
  the file names no elements for the page, which is the honest case: 0
  answers reaching viewer-core's node bound, which nothing says out loud: 0
elements reached: 102849
  §14.9.3's /Alt or §14.9.5's /E carried: 664
  placed by Table 379's /BBox or §12.5.2's /Rect: 7538
  cells with §14.8.4.8.3's headers resolved: 16617 (27273 associations)
  §12.7.5's controls behind §14.7.5.3's object references: 272
untagged pages answering the honest empty tree: 877 of 877
  an untagged page answered with structure it does not state: 0
```

The run **before** the fix, for the record and because it is the measurement of the defect: 1487
of 1557 pages answering, 10 in the "names elements and answers empty" class, 102 572 elements,
635 `/Alt`. The two documents that refuse to open are `issue21579.pdf` (`/R 5`, a proprietary
extension the standard states no algorithm for) and `PDFBOX-4352-0.pdf` (an `/Encrypt` that does
not resolve to a dictionary) — the same two the encryption gates name, and refusals rather than
gaps.

Nothing here is ratcheted, which is ADR 0323's own rule: a number enters `doc/todo/02` §2 after it
has held across rounds and not before. Two *decisions* are asserted from this run — no panic, and
no untagged page given a structure it does not state (ADR 0214).

## What the two residues on `doc/todo/31` turned out to be worth

Both had been recorded without a number beside either.

- **The whole-tree fallback.** 56 pages take it and answer empty; the census prints each
  document's tree size beside the page, and the largest is 393 elements against a bound of 8192.
  So every one of the 56 is the *file* naming nothing on that page. `comments.pdf` is the shape of
  it: a whole structure tree of one `Figure` whose only content item is an annotation on page six,
  so its other thirteen pages have nothing to say and say so. **ADR 0325's residue has no witness
  in this population** — which is a fact about the population as much as about the code, and the
  census will name one the day a document exhibits it.
- **An answer cut at the bound.** No page's answer reaches 8192 nodes. The shortfall stands — a
  `Vec` cannot say it was truncated, which is trap 5 inside a bound — but it binds nothing today.

## What the next round should know

- **The instrument's predicate is not independent of its answer, and it says so.** Both sides read
  the same file with the same crate; what makes it worth anything is that they read the document's
  *two different statements about itself* — §14.7.5.4's parent tree against §14.7.2's `/K` walked
  down from the root. A census that asked one chain twice would have found nothing, and this one
  found ten pages. Trap 8 is the rule and this is the honest version of it.
- **`doc/`'s specifications are in the denominator on purpose.** The pdf.js corpus's tagged
  documents are 17 pages at their largest; every one of the ten defective pages was in `doc/`. A
  population that cannot exhibit the defect reports success.
- **The census ran in 65.0 s before the fix and 28.9 to 42.0 s after it**, which is consistent with
  `page_object` replacing a whole-tree walk per query with one descent to a leaf — but the runs were
  minutes apart on a machine doing other things, and the post-fix figures vary by half themselves,
  so it is an observation and not an A/B. ADR 0312 already paid for that lesson;
  `viewer-core --example accessibility_cost` under callgrind is what would settle it.
- **No raster can change.** Nothing on the drawing path was touched, and no rasteriser reads
  `Query::AccessibilityTree`, `Query::LogicalSelection` or the focus order — so the corpus, oracle
  and quorra gates were not owed by this change and were not run.
