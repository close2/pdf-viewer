# Annotation editing, a caret, and logical-order selection

Status: **markup landed in the three-hundred-and-twenty-first session** (ADR 0196) and **a window
types into a field since the three-hundred-and-forty-ninth** (ADR 0201). What is left is a caret,
and a host that sends the markup command from a drag.
Priority: 33
Clauses: §12.5.6.6, §12.5.6.10, §7.5.6, §14.8.2.5
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

## 2. A caret

Form-field editing landed in the hundred-and-thirty-fifth session and saving in the
hundred-and-thirty-sixth, and until the three-hundred-and-forty-ninth **no window could reach
either**: `viewer-ui` sent no `Edit::SetField` at all, and the only consumer the message ever had
was the headless host.

**It types now** (ADR 0201). A click inside a text field aims the keyboard at it, characters and
Backspace go to the field, Escape gives the keyboard back, and the host keeps the *point* it
clicked rather than the text it typed — so §12.7.5.3's `DoNotScroll` truncating a value is
something it reads back rather than something it has to predict. `Answer::Field` carries the value
for that, with `None` for a field whose value is not text and `Some("")` for an empty one.

**So the caret is now the thing a person actually misses**, which is a better place to argue from
than a list. A person typing sees the text and not where the next character goes; a person who
wants to correct the middle of a value has to delete back to it. The text layer has the geometry —
`Interpretation::text_layer` is one `Placed` per character code with the quadrilateral its glyph
occupies — and what is missing is a *query* for the caret's quadrilateral given a field and an
offset, plus the arrow keys and a selection inside a value.

One more thing this round left: **typing into a field reached by the tab key**. §12.5.1's focus
ring already marks which annotation the keyboard walk is on, and joining that to the typing state
is one query away — but a focus ring on a *button* means something else, so it is a decision rather
than a wire.

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
