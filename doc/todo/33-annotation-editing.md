# Annotation editing, a caret, and logical-order selection

Status: **markup landed in the three-hundred-and-twenty-first session** (ADR 0196), **a window
types into a field since the three-hundred-and-forty-ninth** (ADR 0201), and **a caret says where
the next character goes since the three-hundred-and-seventy-first** (ADR 0211). What is left is
free text, placing the caret by a click *inside* a value, a selection within one, and a host that
sends the markup command from a drag.
Priority: 33
Clauses: §12.5.6.6, §12.5.6.10, §12.7.4.3, §7.5.6, §14.8.2.5
Code: `crates/viewer-core/src/command.rs` (`Edit`), `crates/pdf-model/src/view.rs`,
`crates/pdf-syntax/src/write.rs`

`CLAUDE.md`'s exclusion list permits exactly this: what a *user* does to a document already open
is not authoring, and §7.5.6's incremental update is how it is written back — the producer's
bytes stay in the file, byte for byte, under whatever the user added.

## 1. ~~Markup annotations~~ — **done in the three-hundred-and-twenty-first session**

`Edit::Markup { kind, colour }` marks up what is selected in one of §12.5.6.10's four ways;
`ViewState::add_markup` builds the annotation; §7.5.6's update writes it and appends the reference
to the page's `/Annots`. ADR 0196 has the argument. Three decisions worth keeping here:

- **`Page` gained its own identity**, which is what the three-hundredth session found in the way
  and what this round added because it now has a consumer. `Pages::get` sets it from the `/Kids`
  entry that named the node; the interpreter needs no page-tree walk.
- **The log records what was *done*, not what was asked for.** `Edit::Markup` names its target as
  "what is selected", which is a fact about the moment the command arrived, so `Open::resolve`
  fixes the page and the quadrilaterals before anything reaches the log — undo and redo are a
  replay, and a replay has to reproduce what happened.
- **`/QuadPoints` is in default user space** and everything this crate answers with is in the
  display list's, so `content::page_transform` is public and its inverse is what an author
  composes.

### What is still owed here: free text

§12.5.6.6's free-text annotation is a *different* shape: it has no selection to take its geometry
from and it carries text this program would have to lay out (§12.7.4.3's `/DA` machinery, one
clause over). Nothing in the corpus asks for it, and a host has nowhere to type it yet — the
caret below is the same missing piece.

## 2. ~~A caret~~ — **done in the three-hundred-and-seventy-first session**

`Query::Caret { at, offset }` answers with the segment the next character will be drawn against, in
device pixels of the viewport, and `viewer-ui` draws it: the arrow keys, Home and End move it,
Backspace and Delete take out the character on either side of it, and the tab key aims the keyboard
at whatever §12.5.1's walk lands on when that is a field with text in it. ADR 0211 has the argument.
Three things worth keeping here:

- **The place comes from §12.7.4.3's layout and not from the text layer.** An empty field has no
  glyphs, and 147 of the corpus's first-page widgets are empty text fields — so `Interpretation::text_layer`,
  which this file used to point at, cannot answer the commonest case at all. `variable_text::lay_out`
  computes the caret in the same walk that writes the stream.
- **The standard states no caret**, and §12.5.6.11's caret *annotation* is a different object. What
  it looks like is the host's, exactly as §12.5.1's focus ring is.
- **The host keeps a byte offset and clamps it** to the value the field accepted, because
  §12.7.5.3's `DoNotScroll` truncates — the same reason ADR 0201 has it keep the point and not the
  text.

### What is still owed here: a point turned into an offset

Two things want the same missing piece, which is this query's *inverse*:

- **A click inside a value places the caret.** Today a click aims the keyboard at the field and puts
  the caret at the end of its value, wherever inside the field it landed; the arrow keys reach the
  rest, so nothing is unreachable and everything is one press further away than it should be.
- **A selection inside a value**, which is a drag between two such offsets — and then a decision
  about what copy, cut and paste mean inside a field, which is a vocabulary question and not only a
  geometry one.

Neither is a clause: the standard says nothing about a text cursor, so both are choices to be made
and written down the way ADR 0211's were.

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
