# Annotation editing, a caret, and logical-order selection

Status: the log, the writer and the constructions all exist; **two** things sit on top of them — the third went in the two-hundred-and-ninety-sixth session.
Priority: 33
Clauses: §12.5.6.6, §12.5.6.10, §7.5.6, §14.8.2.5
Code: `crates/viewer-core/src/command.rs` (`Edit`), `crates/pdf-model/src/view.rs`,
`crates/pdf-syntax/src/write.rs`

`CLAUDE.md`'s exclusion list permits exactly this: what a *user* does to a document already open
is not authoring, and §7.5.6's incremental update is how it is written back — the producer's
bytes stay in the file, byte for byte, under whatever the user added.

## 1. Markup and free-text annotations

The constructions exist (`appearance.rs` draws §12.5.6.6's markup and §12.5.6.10's four text
markups) and `pdf_syntax::write` puts an object into a file. What is new:

- authoring `/QuadPoints` and `/Rect` from a drag — geometry from the pointer, in the page's
  space, which `Query::Selection` already produces for text;
- **an `Edit` variant that carries a new object rather than a field's value.** `Update` in
  `view.rs` already allocates object numbers for it, and the writer already *adds* an object
  (that is the half of §7.5.6's "changed, replaced, or deleted" that widgets with no `/AP`
  needed).

The number to start from is the larger of `/Size` and the highest the cross-reference table
holds, because 68 corpus documents understate the first.

**And one thing the three-hundredth session found by scoping the work rather than doing it**: a
new annotation has to be attached to a *page*, and nothing on the interpretation path knows which
page it is looking at. `interpret(&document, &page)` takes a `Page`, which carries `dict`,
`resources` and five boundaries and **no `ObjectId`** — so `ViewState` has no key to file an added
annotation under and `draw_annotations` has nothing to look it up by. `Pages::indices` inverts the
tree for exactly this question in two other places already (`viewer_core`'s accessibility tree and
its logical selection), and doing it a third time inside the interpreter would put a page-tree
walk on the render path.

So the first commit of this item is `Page` gaining its own identity, set by `Pages::get`, and it
is written here rather than done because a field with no consumer is speculative infrastructure —
the round that adds the `Edit` variant is the round that adds the field.

## 2. A caret

Form-field editing landed in the hundred-and-thirty-fifth session and saving in the
hundred-and-thirty-sixth, and a host still sends **whole values**. Nothing lays out a cursor
between two characters. The text layer has the geometry: `Interpretation::text_layer` is one
`Placed` per character code with the quadrilateral its glyph occupies.

## 3. ~~§14.8.2.5's logical order~~ — **done in the two-hundred-and-ninety-sixth session**

`Tree::logical_range` is the map between the two orders' offsets and
`viewer_core::Query::LogicalSelection` is what asks for it. Two decisions worth keeping:

- **It refuses rather than shortens.** A marked-content sequence the structure tree does not
  reach is not part of the logical content order, and leaving one out of a *page's* reading is
  the clause's own position — but leaving one out of what a person dragged over would be a copy
  that silently lost a paragraph. So the range form answers `Some` only where the tree reaches
  every byte, which makes what comes back a rearrangement of exactly the same characters.
- **A second query, not a second field.** `Query::Selection` is asked sixty times a second during
  a drag and this walks the structure tree.

`31-accessibility-host.md` needs the same map and now has it.

## Far, and deliberately so

Editing the page's own *text* is out of scope until the two above exist: it means re-laying-out
content streams whose producer's intent is recorded nowhere.
