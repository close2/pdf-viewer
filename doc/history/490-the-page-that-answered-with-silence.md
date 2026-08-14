# 490 — The page that answered with silence

**Finding.** `doc/todo/31` had recorded, without looking into it, that any page but the first few
of a large tagged document answers `Query::AccessibilityTree` with nothing. The cause was not the
bound it named: `viewer_core::accessibility::nodes` reached a page's elements by walking the
**document's** structure tree from the root and pruning afterwards, so the bound written for one
page's *answer* was being spent on the whole document's *tree* and ran out among the first hundred
pages' elements. Everything past them was never gathered, and an empty list is exactly the answer
an untagged page gives — so a screen reader on a thousand-page tagged document was told the
document says nothing about itself, with no report, no count and no gate able to see it. §14.7.5.4
states the route that has no such shape, and it exists for precisely this reason: a content stream
cannot refer back to its structure elements, so the structural parent tree does it, keyed by the
page's own `/StructParents`. The page's elements are now found through that tree and placed by
Table 355's `/P`; the walk still descends from the root, because the order §14.8.2.5 defines is the
tree's and the parent tree's own order is the content stream's.

**Date.** 2026-08-14.
**ADR.** [0325](../adr/0325-the-page-that-answered-with-silence.md).
**Touched.** `crates/pdf-model/src/structure.rs` (`Tree::elements_on_page`, `Tree::ancestry`,
`identified_children` made public, one unit test), `crates/pdf-syntax/src/tree.rs`
(`lookup_unresolved`, and `lookup` rebuilt on it), `crates/viewer-core/src/accessibility.rs` (the
pruned walk, the `Gathering` it answers with, the three fallbacks),
`crates/viewer-core/tests/headless.rs` (one test),
`doc/conformance/ledger.toml` (§14.7, §14.7.5.3, §14.7.5.4), `doc/todo/31-accessibility-host.md`,
`doc/adr/0325-*` (new), this file.

## What the corpus A/B was worth, which is the part to carry forward

The change was verified by running the new route and the old one **side by side** on every page
sampled from every document in `doc/pdf.js/test/pdfs` and `doc/` — six pages per document, 978
documents, 1274 pages — and requiring the whole gathered value to be equal: the same elements, in
the same order, with the same parents, identifiers, phrases, languages, header scopes, bounds and
header associations. Pages whose old walk was itself cut short by the bound are skipped, because
there the two *should* differ and the difference is the defect.

**It failed twice, and both failures were real defects in the new code that no test in this tree
would have caught:**

- The first version built the page's element set from whatever §14.7.5.4 answered, including the
  case where the page states **no `/StructParents`** and only its annotations are keyed. On a
  document whose pages carry widget annotations and no `/StructParents`, that produced a
  complete-looking answer with every paragraph missing. `Tree::elements_on_page` now answers
  `Option`, and `None` — the file has not said — falls back to the whole-tree walk.
- The second version guarded its own fallback with the length of the **pruned** answer, which is
  zero exactly when the walk was bounded out and kept nothing. The bound is now read before the
  pruning and carried on `Gathering::bounded`.

The A/B itself is twelve lines inside `nodes` behind an environment variable plus a throwaway
example, and it is **not committed** — a permanent one would run the walk this round removed. ADR
0325 says how to rebuild it, and it took under two minutes of wall clock to run.

## And it was checked where the defect lived

`doc/verify.md`'s AT-SPI recipe, with `--page 400` on the specification's own 1023 pages. A real
client walking `org.a11y.atspi.Accessible` from the registry root reaches `main` → `pdf-viewer` →
the frame → the document → `page 385 (400 of 1023)` — §12.4.2's label — and then the page's own
paragraphs and a `Figure` announced with §14.9.3's `/Alt`. That subtree was empty before. The
recipe's two documented traps both bit again and both are already written down: `IsEnabled` must be
set on the session bus inside `dbus-run-session`, and the registry needs a `DISPLAY` of its own.

## What the next round should know

- **The cost item on `doc/todo/31` is now about something else.** The document-sized walk is gone;
  what is left is the ancestors' `/K` arrays, whose every child is resolved to find out whether it
  is one of the page's, and §14.7.5.3's role map per element. **This round measured none of it**
  and deliberately quotes no wall clock: it ran beside nine others, and ADR 0312 already paid for
  that lesson.
- **The obvious next optimisation is rejected in writing**, so that it is not re-derived: skipping a
  child by its reference alone, before resolving it, is unsound because §14.7.5.1.1's content items
  may themselves be indirect objects.
- No raster can change — nothing on the drawing path was touched, and `Query::AccessibilityTree` is
  read by no rasteriser — so the corpus and oracle gates were not owed by this change.
