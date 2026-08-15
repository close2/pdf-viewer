# ADR 0211 — A caret is where the next glyph goes, and the standard states nothing else about it

Status: accepted, 2026-08-06 (session 371).

## Context

A person has been able to type into a form field in a window since the three-hundred-and-forty-ninth
session (ADR 0201), and has been shown no indication of where the next character goes. The
consequence is not cosmetic: with no caret there is nowhere to *move*, so correcting the middle of a
value means pressing Backspace until the mistake is gone and typing the rest again. That ADR's own
consequences section named it:

> **The caret is still `doc/todo/33`'s**, and this round makes the case for it concrete rather than
> theoretical: a person typing into a field sees the text but not where the next character goes.

**The standard states no caret at all.** ISO 32000-2 describes where a *glyph* is placed (§9.4.4's
text rendering matrix), how a field's value becomes glyphs (§12.7.4.3), and what a text field is
(§12.7.5.3). It says nothing about a text cursor: not its shape, not its width, not its colour, not
whether it blinks. §12.5.6.11's **caret annotation** is a different object entirely — a mark left
*in a document* to record that text was edited there, with its own `/Rect` and its own appearance —
and this ADR does not touch it or its `reported` ledger row.

So this is a *choice*, in the sense `CLAUDE.md` principle 5 requires one to be recorded: the clause
is silent, something has to be decided, and what is decided is written down as a decision rather
than presented as a reading.

## Decision

### The core answers where, the host answers what it looks like

`Query::Caret { at, offset }` → `Answer::Caret { from, to }`, in device pixels of the viewport,
beside `Query::Focus` and `Query::Selection` and for the same reason: **interactive chrome crosses
as geometry**. A native host draws the insertion point in its platform's colour, at its platform's
width, with its platform's blink; `viewer-ui` draws a steady black line two pixels wide, which is
this host's choice exactly as §12.5.1's focus ring being blue is (the clause states no ring either).

Two points rather than a rectangle. A caret has no width — how thick one is drawn is a convention of
the platform, not a fact about the document — and a widget's Table 192 `/R`, its appearance's
`/Matrix` or its `/DA`'s `Tm` can each turn it off the page's axes, which a rectangle could not
describe.

Nothing on the caret's path re-interprets the page. An arrow key changes an offset the host holds
and asks one query; the trace of the window run below shows two `ArrowLeft` presses producing a
present and **no command at all**.

### The place comes from §12.7.4.3's own layout, not from the text layer

The obvious route was `Interpretation::text_layer`, which `doc/todo/33` pointed at: one `Placed` per
character code with the quadrilateral its glyph occupies, derived from §9.4.4's text rendering
matrix and Table 120's metrics (ADR 0118). It is the wrong instrument here, and the case that
settles it is the commonest one in the corpus:

**an empty field has no glyphs.** 147 of the corpus's first-page widgets are empty text fields
waiting for a person, and the text layer can say nothing whatever about where their first character
will land. Neither can it say where a *value's* offset is, because what it indexes is the page's
readback — one string for the whole page, with §14.8.2.6.2's inferred separators in it — and not the
field's value.

What does know is `variable_text::lay_out`: the x a line is positioned at by Table 228's `/Q`, plus
the width of the value's own prefix measured with the same formula the glyphs are placed by, between
the ascent and the descent that put the baseline where it is. So the caret is computed *in the walk
that writes the stream*, from the same numbers, and `LaidOut::caret` carries it out. A caret measured
by a second implementation of that layout — in the host, or in a second function here — would sit
beside the text rather than in it the first time an auto-sized field, a `Tz`, or a wrapped line
disagreed.

Three consequences fell out of putting it there rather than beside it:

- **`wrap` returns ranges of the code vector** instead of copies of it, because an offset has to be
  locatable in a line. Every caller reads the same slice through one accessor, and no line is copied
  any more.
- **`encode` turns the byte offset into a code index**, in the one loop that knows how many codes a
  character produced: none for a character the font cannot spell, one for each of the line breaks
  §12.7.5.3's Multiline value carries. A caret cannot stand beside a glyph that is not drawn.
- **An empty value is laid out when, and only when, a caret was asked for.** The drawing path still
  returns "nothing to draw" for an empty field; the query path lays out the empty string, because
  somewhere for the first character to go is the one thing an empty field can still be asked.

### Which space the value is laid out in is the question `decide` already answers

A widget's value is laid out either in the page's own space (`construct`, where the file carries no
appearance stream) or in the stored stream's `/BBox` (`regenerate`, §12.7.4.3's splice). The caret
has to be stated in default user space, so it needs the second one's map — which is §12.5.5's
placement algorithm, already written. `annotation::stored_frame` hands it over: a stored appearance's
box, and `AA`.

**The caret is answered from the stored stream's box whether or not that stream has been rewritten
yet**, and that is deliberate: the next character typed rewrites it, by the clause's own splice, so
the space the caret must be right about is the space the *next* character will be drawn in.

### The host keeps the offset, and clamps it to what the field accepted

ADR 0201's rule — the host keeps the place it clicked and never the text — is unchanged and extended
by one number. The caret is a byte offset into the value the core answers with, and §12.7.5.3's
`DoNotScroll` is why it must be *clamped* rather than counted: a widget takes only the prefix that
fits, so after every keystroke the host reads the value back and puts the caret at the nearest
character boundary of what was accepted. Nothing is buffered, and nothing can diverge.

Two things this host chose, each one press wide:

- **The caret starts at the end of the value on a click**, wherever inside the field the click
  landed. Placing it *where* the click was means turning a point into an offset, which is the
  inverse of this query and does not exist yet; the arrow keys, Home and End reach the rest of the
  value, so nothing is unreachable. `doc/todo/33` carries it.
- **A comb field's caret is a cell, not a gap between glyphs.** Table 231 bit 25 divides the box
  into `/MaxLen` positions and puts one character in each, so the place the next character goes is
  the next position — the caret stands at that cell's left edge.

### The tab key aims the keyboard, and it needed no new message

`doc/todo/33` left this as a decision rather than a wire, on the grounds that "a focus ring on a
*button* means something else". It does, and the distinction was already answered: `Answer::Field`'s
value is `Some` only for a field §12.7.4.3 lays text out for, which is what a click already uses to
decide where the keyboard goes. What was missing was only a *point*, and `Query::Focus` answers with
the focused annotation's quadrilateral in the same device pixels `Query::FieldAt` takes. So a walk
that lands on a text field types into it, and a walk that lands on a button or a link takes the
keyboard back to the page.

## Two defects found by reading, both in the round's own subject

- **Escape exited the program while a field had the keyboard.** ADR 0201 decided that "Escape leaves
  the field rather than the program", `typed` has an arm for it, and the window event handler
  answered Escape three branches earlier by calling `event_loop.exit()`. The arm was dead code from
  the round that wrote it. No gate could see it: nothing in this tree presses two keys in one window
  and looks at what happened.
- **`Query::Focus`'s ring and `Query::Popups`' windows were placed without the page transform.** The
  inverse map (`user_space`) undoes §7.7.3.3's `/Rotate` and the crop box's origin; the forward one
  did not, so a rectangle the document states was placed as though every page were unrotated with
  its crop box at the origin. Not one of the 974 corpus documents states a rotated page *and* a
  widget, which is why no picture could have shown it — the fix is one composition, in the one
  function every shape now goes through, and it changes nothing on any page the corpus has.

## Consequences

- **A person can see where they are typing, and correct the middle of a value.** In the window:
  `160F-2019.pdf` under `Xvfb`, a click at (430, 174) into `A.NOM`, the caret at x = 356–357 in the
  empty field; `typed` moves it to 381–382; two `Left` presses put it at 370–371; a Backspace there
  sends `SetField { value: "tyed" }` — the middle character, which is the whole point — and leaves
  the caret at 364–365; Escape takes it off the screen and the program is still running. Two `Left` presses send no command at all and present in 20.6 and 18.4 ms; a character costs an edit of 5.1 to 8.5 ms and a render beside it. One Tab press, no click, puts the caret in `F.1` at x = 394–395 and `x` reaches the field.
- **A third defect never shipped, and the test that caught it is one line long.** `caret_boundary` asked `is_char_boundary` before clamping, and that predicate answers *false* past the end of a string — so an offset outside a value §12.7.5.3 had truncated landed on the last character's boundary rather than on the end, one character short every time. It is the case the whole function exists for.
- **The layout is measured once and used twice**, which is what keeps the caret honest as
  §12.7.4.3's rules grow: anything that changes where a glyph goes moves the caret with it.
- **What is still owed** is in `doc/todo/33`: a click *inside* the value to place the caret, and a
  selection within a value. Both need the same missing piece — the inverse of this query, a point
  turned into an offset — and both are a decision about what copying out of a field means.
