# 1739 — A note's caret in `quorra` is placed by the window's own layout, and a rich note keeps it at the end

Session 1451. Status: **accepted**. Takes back ADR 1726's cost for plain notes; keeps it, by
argument, for one case. Context: ISO 32000-2 §12.5.6.14 (the popup "shall be used for editing the
parent's text"), §12.5.6.2's Table 172 `/RC`; ADR 1726.

## 1. Decision

**`quorra` places a note's caret where its window draws the text: a press puts it at the nearest
place, the four arrows, Home and End move it, and a character, Backspace or Delete edits at it.**

- The places are `chrome::popup_caret` and `chrome::popup_offset`, both read out of one function,
  `plain_carets`, that walks the layout `draw_plain` draws — paragraphs split at a carriage return
  or a line feed, each collapsed and wrapped by `Wrapped`, one place per character boundary of each
  line above the window's bottom edge. One layout for the picture and the caret is the whole point:
  ADR 1726 kept the caret at the end because a second layout would put it where the next character
  does not go.
- **The caret is drawn over the popup windows**, in an overlay of its own after them: the field
  caret's overlay is under the windows with the page's, where a note's caret is covered by the paper
  it stands on — which the drive's first passing run showed only on its photograph.
- **A white-space run the window draws as one space has one place**, at that space. The text
  keeps every character typed; a second space typed in a row is in `/Contents` and not on the
  screen, which is the window's existing collapse and not a new one.
- **A window drawn from `/RC` keeps the caret at the note's end, ringed.** `rich::draw` lays its
  runs out one style at a time and answers no point for an offset, and a retyping keeps the `/RC`
  only where the characters still agree (ADR 1721) — so the plain layout would place a caret over
  runs it did not draw. Building the rich layout's places is the remaining cost, and it is named
  here rather than discovered.

## 2. Consequences

- Tests: `viewer-ui/tests/panel.rs` round-trips every place of a two-paragraph note with a double
  space through a press, and a rich window gives none (the control).
- The two toolkit windows retype in their own text widgets and are unchanged.
