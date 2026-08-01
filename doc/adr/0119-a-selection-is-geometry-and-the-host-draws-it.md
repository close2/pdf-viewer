# ADR 0119 — A selection is geometry, and the host draws it

Status: accepted, 2026-08-01.

## What this decides

Text can be selected. A drag across a page picks out what it crossed; `Command::Select` takes the
page or clears it; `Query::Selection` answers with the text and the shapes covering it, and the
window draws the shapes in its own colour.

**Nothing here is in ISO 32000-2.** The standard says where a glyph is drawn (§9.4.4) and what
character it stands for (§9.10.2), and it says nothing whatever about what a person means by
dragging across a page. Every rule below is therefore a choice, and each one is written down as
one — which is the same discipline §12.5.6.4's icon artwork got, for the same reason.

## The choices

**A point selects the nearer edge of the nearest glyph.** In the left half of a glyph means the
position before it, in the right half means after. That is what makes a drag across a word select
the whole word rather than all but its last letter. "Nearer" is measured along the glyph's own
advance rather than along x, so rotated and mirrored text behave the same way.

**A point that is nowhere near any glyph still answers.** Dragging below the last line selects to
the end of it, which is what a person dragging off the bottom of a paragraph means. Only a page
with no text at all answers nothing.

**A code that reads back as nothing is still selectable.** A glyph no `/ToUnicode`, glyph name or
`cmap` could name has an empty span, and it is included in the shapes when the selection runs
across its position: it is ink a person dragged over, and a hole in the highlight where it sits
would say something false about what is selected. 41 corpus documents have such fonts.

**Shapes are merged per run of a line.** A highlight drawn as three hundred abutting rectangles
under one alpha shows a seam at every edge, and a host that was handed one shape per glyph would
have to do the merge itself. Two glyphs join when their boxes share both baseline corners' y and
the gap between them is less than a line's height — so the space between two words stays
highlighted, which is what a drag across them meant.

**A drag that selected something does not follow the link it started on.** §12.5.5 describes
appearances, not activation; this is the rule every viewer has, and without it dragging across a
paragraph of links would jump to whichever one the press landed on.

**A page turn forgets the selection.** It is a range of *this page's* readback, and carrying it
would leave it pointing into text that is no longer there.

## Geometry, not pixels

`Query::Selection` returns quadrilaterals in **device pixels of the viewport**, and the host draws
them. Three consequences, and they are the reason this crate's chrome crosses as shapes:

- **A selection does not re-render the page.** Dragging emits `Damage` and no `NeedsRender`, which
  `a_drag_across_a_line_selects_what_it_crossed` asserts directly. At pointer speed that is the
  difference between a smooth drag and a 2 000 M instruction interpretation per frame.
- **A native host draws it natively** — macOS's selection colour, KDE's accent, the Windows
  highlight brush, its own caret blink and focus ring. None of that is reachable if the core hands
  over finished pixels.
- **Device pixels rather than the page's own units**, because a host has no transform of its own,
  and asking it to compose one would be asking it to re-derive the magnification, the centring and
  the y flip. That is exactly the arithmetic ADR 0118 found wrong, one session ago, in the one
  place it existed.

`viewer-ui` draws the shapes as filled paths with a hard-coded blue, and says in a comment that a
native host asks its platform for that colour and this one has nobody to ask.

## What this does not do, and the clause behind it

**The selection is in content order, not §14.8.2.5's logical order.** The clause defines both —
"the sequencing of graphics objects within a page's content stream" and "a depth-first traversal
of the document's logical structure hierarchy" — and says they *should* coincide;
`tests/logical_order.rs` measures how often they do not. A selection taken in content order on a
page whose producer wrote its columns out of order gives text in that wrong order.

Closing it is not hard and is not this session's: `Interpretation::marked` already carries the
`/MCID` spans and `Tree::logical_text` already produces the logical string, so what is missing is
the map between the two orders' offsets. Recorded here rather than left as a surprise.

Also absent: a caret, a word or paragraph selection (a host sends the double click; the rule for
what it selects has not been chosen), and selection across pages, which needs a view that shows
more than one.

## Consequences

Tests 916 → 919. The three are the drag, select-all-and-clear, and the page turn that forgets.
`viewer-core` gains `select.rs`, `Command::Select`, `PointerAction::Dragged` and
`Query::Selection`; `Interpreted` now keeps the page's readback and text layer, because a drag
asks about them sixty times a second and re-interpreting to answer would be absurd.

The four gates are unmoved.
