# Annotation editing, a caret, and logical-order selection

Status: **markup landed in the three-hundred-and-twenty-first session** (ADR 0196), **a window
types into a field since the three-hundred-and-forty-ninth** (ADR 0201), **a caret says where the
next character goes since the three-hundred-and-seventy-first** (ADR 0211), **a click places it
and a drag selects since the three-hundred-and-eighty-eighth** (ADR 0225), and **§12.5.6.6's free
text since the four-hundred-and-first** (ADR 0238). What is left is one annotation this program
still will not touch — the *file's* own — and a callout line no clause states a colour for.
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

## 1a. ~~Free text~~ — **done in the four-hundred-and-first session**

`Edit::FreeText { from, to, colour }` puts §12.5.6.6's annotation over a rectangle a person
**drew**, `Edit::SetFreeText { annotation, text }` says what it says, and `Query::FreeTextAt { at }`
is how a host learns which annotation the drag made — asked at a point inside it, because a host
that can ask needs no event. `viewer-ui` binds `f`, `viewer-confined` carries all three, and ADR
0238 has the argument. Four things worth keeping here:

- **A drag, because this subtype has no text to be over.** §12.5.6.10's four "appear as
  highlights, underlines, strikeouts … in the text of a document" and need only a selection;
  §12.5.6.6 "displays text directly on the page" and has to be given a box. The two corners cross
  in device pixels like every other point in the vocabulary and are mapped to default user space
  by `Viewer` — the map needs the viewport and the scale, which are the window's — before
  `Open::resolve` puts the rectangle in the log.
- **Two verbs rather than one carrying both.** One verb with a rectangle *and* a string would add
  an annotation per keystroke on replay, because nothing in the log would say the second entry was
  about the first one's object. The object is stable across a replay, which is what makes naming
  one in the log sound.
- **The caret needed no new arithmetic.** `appearance::caret`, `offset_at` and `selection` are
  unchanged; `frame` and `laid_out_in` one level below them dispatch on `/Subtype`. §12.5.6.6 sends
  this subtype to §12.7.4.3 itself, so the layout underneath is one layout and only the way in
  differs — which is what this file predicted in one line.
- **The `/DA` this program writes obeys §12.7.4.3's `/DR` `shall`.** A save states `/Helv` in Table
  224's `/DR`, creating the interactive form dictionary where the document has none. This tree
  reads six corpus documents that break that sentence; writing a seventh would be a different
  thing.

### What is still owed on this subtype, and neither is a capability

- **Editing a free text annotation the *file* states.** `ViewState::set_free_text` takes only an
  annotation this session added and `free_text_at` answers `None` for the producer's own, which is
  deliberate: appending an object is the writing `CLAUDE.md` permits, and replacing one the
  producer wrote is a decision nobody has made. What it would cost is a second map in `ViewState`
  — annotation to text — read where `appearance::free_text` takes `/Contents`, plus a replacement
  object at save time. Table 167's `Locked` and `LockedContents` become reachable the same day and
  the §12.5.3 ledger row says so.
- **Table 177's `/CL` and `/LE` drawn rather than reported.** The geometry is stated and the colour
  is not — Table 166's `/C` is an icon's background, a popup's title bar and a link's border, none
  of which this subtype has — so drawing it means inventing one, exactly as `/BS`'s border does.
  Reported by name since this round. No corpus first page states one.

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

### ~~A point turned into an offset~~ — **done in the three-hundred-and-eighty-eighth session**

`Query::Offset { at, point }` is the caret's inverse and `Query::FieldSelection { at, from, to }` is
the shapes over a range of a value; `viewer-ui` places the caret where a click landed, selects with a
drag, and copies, cuts and pastes inside a field. ADR 0225 has the argument. Four things worth
keeping here:

- **The inverse takes two points and not one.** `at` names the field and `point` is measured inside
  it, because a drag that leaves the widget's rectangle is still a drag inside its value.
- **A selection is a third question rather than two carets.** Table 231 bit 13 lets the layout break
  a value into lines a host cannot see, so the lines *between* the two ends are `pdf-model`'s to
  name. On a single-line field a host could have joined two carets itself; one rule for both shapes
  is what keeps it from needing two.
- **Copy, cut and paste needed no message.** The offsets are into the value a host has already read
  back, so the characters are a slice it holds and the edit is the `Edit::SetField` a keystroke
  already sends. The clipboard is the host's, and the system's is a platform's.
- **A point outside every glyph answers the nearest boundary** rather than refusing, which is a
  choice: a press a host has already decided belongs to a field has to leave the cursor somewhere.

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
