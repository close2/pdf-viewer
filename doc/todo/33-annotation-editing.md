# Annotation editing, a caret, and logical-order selection

Status: the log, the writer and the constructions all exist; three things sit on top of them.
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

## 2. A caret

Form-field editing landed in the hundred-and-thirty-fifth session and saving in the
hundred-and-thirty-sixth, and a host still sends **whole values**. Nothing lays out a cursor
between two characters. The text layer has the geometry: `Interpretation::text_layer` is one
`Placed` per character code with the quadrilateral its glyph occupies.

## 3. §14.8.2.5's logical order

A selection is taken in *content* order, so a page whose producer wrote its columns out of order
gives its text in that order. `Interpretation::marked` carries the `/MCID` spans and
`Tree::logical_text` produces the logical string; **what is missing is the map between the two
orders' offsets.** The same map `31-accessibility-host.md` needs.

## Far, and deliberately so

Editing the page's own *text* is out of scope until the three above exist: it means re-laying-out
content streams whose producer's intent is recorded nowhere.
