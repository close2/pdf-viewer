# 466 — A figure with no place, and the rectangle that gives it one

**Finding.** This tree's only `silent` ledger row is closed. An element whose content drew no
*text* — a `Figure`, a cell holding an image — crossed to an assistive technology with no bounds at
all, because `AccessibilityNode::quads` is built from the text layer; on a real AT-SPI bus such a
node implemented no `Component` interface, so a client asking where it is got an error. Table 379's
`/BBox` is read now, mapped from default user space into the viewport's pixels through §7.7.3.3's
rotation, and **intersected with the page** — §14.8.5.4.3's rectangle encloses the element's
*visible* content and §14.11.2.1 makes the crop box what can be seen, which matters because
`doc/PDF20_AN001-BPC.pdf` states `[-32768 -32768 32767 32767]` for one figure and unclipped that
reached the bus as a node 55 045 pixels square.

**Date.** 2026-08-13.
**ADR.** [0301](../adr/0301-a-figure-with-no-place-and-the-rectangle-that-gives-it-one.md).
**Touched.** `crates/pdf-model/src/structure.rs` (`Tree::bounds`, `normalised_rectangle`, one test),
`crates/pdf-model/examples/element_bounds_census.rs` (new),
`crates/viewer-core/src/accessibility.rs` (`AccessibilityNode::bounds` and the gather),
`crates/viewer-core/src/viewer.rs` (`Viewer::device_rect`),
`crates/viewer-core/examples/accessibility_cost.rs` (new),
`crates/viewer-core/tests/headless.rs` (a two-page figure fixture and its test),
`crates/viewer-confined/src/protocol{.rs,/panels.rs}` (the rectangle on the pipe, and a `Figure`
carrying one in the round trip), `crates/viewer-accessibility/src/tree.rs` (`place`) and
`tests/tree.rs`, `doc/conformance/ledger.toml` (§14.8.5.4.3, §14.8.5.4.6, §7.9.5),
`doc/todo/31-accessibility-host.md`, `doc/todo/README.md`, `doc/verify.md`,
`doc/adr/0301-*`, this file.

## The population, counted first

`element_bounds_census` over 1096 files (1080 opened, 117 tagged, 133 114 elements): **28 documents
state a Table 379 `/BBox` and 132 elements do** — `Figure` 77, `Table` 51, `P` 3, `Formula` 1 —
while **1700 elements have content items that produced no text**, of which **61 state a `/BBox`**
and 60 of those 61 are `Figure`s. That last line is §14.8.5.4.3's own NOTE 1 coming back as a
measurement: the clause says the attribute "should be present" for content that does not reflow and
names `Figure` and `Formula` as the examples.

The census was wrong twice before it was right, both trap 11's shape and both worth the retelling:
it interpreted only the documents that stated a `/BBox`, which counted every element of every other
tagged document as marking no text (4163 rather than 1700); and `document.get_key(element, "Pg")`
*resolves*, so `as_reference()` on it is always `None` and the "reaches outside its page" count was
0 until the raw entry was read. The right answer is 8 of 132.

## The gates

The whole of `doc/todo/02` §2 ran after the last edit. Nothing moved but the test count: the corpus
gate's incomplete list is unchanged, the oracle's verdicts are unchanged, both text gates are
unchanged, quorra is unchanged, and the conformance checker passes with the new quotations verified
against `doc/md/`.

Both new tests were checked by deleting the code they guard. Without `page_space_at`, the rotated
page's figure comes back at the unrotated rectangle; without the intersection, the ±32768 figure
comes back as `[-49027, -49152, 49275.5, 49150.5]`; with `finish` answering `None`, the whole
assertion fails at the first figure.

## What the round did not do

Prefer the stated rectangle over the shapes that were drawn. `tree::place` uses the quads where an
element has both, on the conservative reading, and the case where the two disagree — a `Figure`
holding a caption *and* a picture — is unmeasured and is written into `doc/todo/31` as such rather
than settled by taste.
