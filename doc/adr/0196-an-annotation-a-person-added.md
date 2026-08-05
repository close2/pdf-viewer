# ADR 0196 — An annotation a person added, and the page it belongs to

Status: accepted, 2026-08-05 (session 321).

## Context

`CLAUDE.md`'s exclusion list was amended in the hundred-and-thirtieth session by argument rather
than by attrition: **what a *user* does to an open document is not authoring.** A field filled in
landed in the hundred-and-thirty-fifth and its save in the hundred-and-thirty-sixth. What was left
of that sentence is the other half — an annotation *added* — and `doc/todo/33` had been carrying it
with the pieces already in place: `appearance.rs` constructs §12.5.6.10's four text markups,
`pdf_syntax::write` appends §7.5.6's incremental update, `Query::Selection` produces the geometry.

The three-hundredth session scoped it and found the thing in the way, which is why this ADR opens
with a field rather than with a clause:

> A new annotation has to be attached to a *page*, and nothing on the interpretation path knows
> which page it is looking at.

## Decision

### A page knows which object it is

`Page` gains `id: Option<ObjectId>`, set by `Pages::get` on both of its paths — the tree walk,
where the identity comes from the `/Kids` entry that named the node, and the recovery scan, where
it is the object that declared itself a page. It is `None` for `Pages::detached`, which builds
§12.7.7's template pages from a dictionary outside the tree.

**Why a field rather than a lookup.** `Pages::indices` inverts the tree for exactly this question
in two other places, and asking it a third time *inside the interpreter* would put a page-tree walk
on the render path. The identity is free where it is taken: `find_leaf` already holds the reference
it resolved.

`doc/todo/33` said this field would be added by the round that gives it a consumer rather than
before, because a field with no consumer is speculative infrastructure. This is that round.

### The log records what was done, not what was asked for

`Edit::Markup { kind, colour }` is what a host sends and it means **mark up what is selected**.
That is a fact about the moment the command arrives: a replay after the selection moved would mark
up something else, and undo and redo in this crate *are* a replay of the log's surviving prefix
(ADR 0120).

So there are two types. `Edit` is the request; `open::Done` is the record, and `Open::resolve` is
the one place they meet — it fixes the page, the four corners of every run, and the colour before
anything reaches the log. A `SetField` passes through unchanged, because a field's name and value
are already the whole of what happened.

### `/QuadPoints` is in default user space, and the display list is not

Everything this crate answers with is in the display list's coordinates — `Placed::quad`, a
selection's shapes, a focus ring — because that is what a host needs to draw over a frame (ADR
0118). Table 179's `/QuadPoints` is in default user space. `content::page_transform` is
`base_transform` under a name a caller outside the module can say, and `Open::resolve` composes its
inverse.

**The corner order does not matter**, and that is worth stating because it usually does:
`appearance.rs`'s `Quad::read` sorts the four vertices by where they fall along the text's own
direction, on the argument that the clause's "counterclockwise" has two readings and producers use
both. A writer that had to pick one would be picking a side of that argument.

### What is written, and what is deliberately not

Table 166's `/Subtype`, `/Rect`, `/C`, `/QuadPoints`, and `/F 4`.

- **`/Rect` is the quadrilaterals' bounding box**, because §12.5.2 positions an annotation by it
  and every reader clips the appearance to it.
- **`/F 4` is Table 167's `Print` bit and it is a choice.** The flag's own sentence is "[i]f
  clear, never print the annotation, regardless of whether it is rendered on the screen", and a
  person who marks up a document to send it on means the mark to survive printing.
- **`/M` is not written**, because `CLAUDE.md`'s rule 3 gives `viewer-core` no clock. A host with
  one may add it; inventing a timestamp from nothing would be worse than omitting an optional
  entry.
- **`/T` is not written**, because a person's name is not something any part of this program
  knows.
- **`/P` is written at save time rather than when the annotation is added**, because Table 166
  calls it "an indirect reference to the page object with which this annotation is associated" —
  a statement about the *file*, which belongs to the writing.

### The object number is allocated when the annotation is added

Not when it is saved. The number is the annotation's identity for as long as the document is open,
which is what the pointer, a later edit and the writer all need to name it by — and it follows
`Update::beside`'s own rule, the larger of the highest cross-reference entry and `/Size`, because
68 corpus documents understate the second. The writer then `reserve`s each of them so that an
appearance stream cannot be allocated on top of one.

### `/Annots` is rewritten where it is

§12.5.2 puts an annotation on a page through the page's `/Annots`, and the entry may be an inline
array or a reference to an array object. **Both are ordinary, so both are handled where they are**:
an inline array is rewritten inside the page dictionary, a referenced one in the object it names.
Inlining a referenced array would leave the original object in the file saying something else, and
§7.5.6's "most recent copy" rule would then have to arbitrate for no reason.

**Appended rather than inserted**, because the array's order is the drawing order the same clause
states, and a mark a person made last belongs on top of what was there.

### The appearance stream is written too

ADR 0130's argument, one clause over: **this program can produce the bytes, so a file that carried
the annotation without them would be asking the next reader to do work this one has already done**
— and a reader that constructs nothing would show the page unmarked. `crate::appearance::construct`
is the same function the drawing path calls, so what a saved file shows is what this program shows.

`/BBox` is the annotation's own `/Rect`, and that is not a coincidence to be tidied away.
§12.5.6.10's `/QuadPoints` is "in default user space", so the constructed marks are already in the
page's coordinates; §12.5.5's algorithm maps a form's `/BBox` onto its `/Rect`, and giving it the
same rectangle twice makes that map the identity. Any other box would move the marks off the words
they are over.

**Measured on the two readers this project compares against**, over a five-page note whose cover
was marked up and saved by the viewer itself:

| | yellow pixels at 72 dpi |
|---|---|
| ours | 32 423 |
| `poppler` | 35 654 |
| `mupdf` | 35 050 |

Without the `/AP` the same two were 23 060 and 27 241 — eighteen percent apart, because each was
constructing its own picture of the same quadrilaterals. With it they are within 1.7% of each
other and of ours, which is the difference between a file that *states* its marks and one that
asks.

## Consequences

- A markup is drawn by the *same three functions* the file's own annotations take:
  `draw_annotation` is `draw_annotations`' body with the dictionary and the identity passed in, so
  an annotation this program constructed is not a second kind of annotation and cannot drift into
  one.
- `viewer-core/tests/headless.rs` states the end a person sees — select, mark up, and the wash is
  on the page in the colour asked for; undo and it is gone — and `pdf-model/tests/saving.rs`
  states what neither end can see: which objects the update contains, that `/P` is a *reference*,
  and that the page's array grew by one at its end.
- **Nothing about `interpret` changed shape.** It is still a pure function of the document and the
  view state, which is what the oracle's comparison of 1685 pages rests on: the annotation lives
  in `ViewState`, like every other edit, and the document is still immutable.
- What is left of `doc/todo/33` is the caret, and authoring from a *drag* in a host — this round
  gives a host one command and `viewer-ui` does not yet send it.
