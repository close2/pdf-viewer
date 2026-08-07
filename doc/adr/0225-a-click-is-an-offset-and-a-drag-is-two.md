# ADR 0225 — A click is an offset, a drag is two, and the standard states neither

Status: accepted, 2026-08-07 (session 388).

## Context

ADR 0211 gave a form field a caret and named what it left owed in the same breath:

> **What is still owed** is in `doc/todo/33`: a click *inside* the value to place the caret, and a
> selection within a value. Both need the same missing piece — the inverse of this query, a point
> turned into an offset — and both are a decision about what copying out of a field means.

Until this round a click aimed the keyboard at a field and put the caret at the **end** of its
value, wherever inside the field it landed. Nothing was unreachable — the arrow keys, Home and End
reach the rest — and everything was one press further away than it should be. A person correcting
the third character of a value had to walk there.

**The standard states nothing about any of this.** ISO 32000-2 says where a *glyph* goes (§9.4.4's
text rendering matrix), how a field's value becomes glyphs (§12.7.4.3) and what a text field is
(§12.7.5.3). It describes no text cursor, no click that places one, no selection inside a value, and
no clipboard. §12.5.6.11's caret *annotation* is a different object and is not touched here. So
everything below is a **choice**, recorded as one in the sense `CLAUDE.md` principle 5 requires —
the same footing ADR 0211's caret and §12.5.1's focus ring stand on.

What *is* the standard's is where the glyphs are, and that is the whole reason these questions
belong in `pdf-model` rather than in a host.

## Decision

### Two new questions, and the second is not the first twice

```
Query::Caret         { at, offset }   → Answer::Caret { from, to }      (ADR 0211)
Query::Offset        { at, point }    → Answer::Offset(usize)
Query::FieldSelection{ at, from, to } → Answer::FieldSelection(Vec<[f32; 8]>)
```

**`Query::Offset` is `Query::Caret` run backwards**: that one takes an offset and answers a place,
this takes a place and answers an offset, and an answer from either, handed to the other, comes back
unchanged. `viewer-core/tests/headless.rs` asserts exactly that round trip at every byte of a value,
and `pdf-model`'s does it against the ink.

**It takes two points, not one.** `at` names the field — the point a host kept from the press that
aimed the keyboard (ADR 0201) — and `point` is the place inside it to measure. They are the same
point on a click and different ones on every move of a drag, and the second case is why one point
would not do: **a drag that leaves the widget's rectangle is still a drag inside its value**, and a
single point that had wandered off the widget would name no field at all. `at` keeps the one meaning
it has everywhere else in this vocabulary: the point that names the thing.

**A point outside every glyph answers the nearest boundary rather than refusing.** A press a host
has already decided belongs to a field has to leave the cursor somewhere; a click past the end of a
line answers that line's end, one below the last line answers the last, and an empty field answers
zero. Refusing would hand a host a press it could do nothing with.

**`Query::FieldSelection` is a third message rather than a range on the caret**, and the case that
settles it is §12.7.5.3's Table 231 bit 13. A host holding both ends of a selection could join two
carets itself on a single-line field. On a multiline one it cannot: `variable_text::wrap` breaks the
value into lines by an algorithm §12.7.4.3 hands to the processor, so **the lines between the two
ends are this crate's to name**, and a host that guessed would draw a highlight across the middle of
a box that has three lines in it. One rule for both shapes is what keeps a host from needing two.

Folding it into `Query::Caret` as a range was the alternative and it costs the caret's own property:
ADR 0211 answers two points *because a caret has no width*, and a selection has area. One question
whose answer changed shape depending on whether its two offsets were equal would be two questions
wearing one name.

### The selected *text* is not in the answer, and copy, cut and paste needed no message at all

`Answer::FieldSelection` carries shapes and nothing else. The characters between the two offsets are
a slice of a string the host is already holding: ADR 0201 has it read the field's value back through
`Query::FieldAt` after every keystroke, and the offsets are into that same string. Putting the text
in the answer would be a *second way to say something a host can say itself*, which is the one thing
this vocabulary has refused to grow since it was written.

That decides the vocabulary question the todo file raised, and the answer is that there is no new
vocabulary:

- **Copy** is `value[from..to]`, in the host.
- **Cut** is that slice, plus the `Edit::SetField` a keystroke already sends, with the range spliced
  out.
- **Paste** is the same `Edit::SetField` with the range replaced.

Three verbs, no messages. `viewer-ui` binds them to Ctrl + C, X and V and keeps **its own**
clipboard: reaching the system's is a platform's business — X11's `CLIPBOARD` selection, Wayland's
data device, `NSPasteboard`, `OpenClipboard` — and a native host embedding `viewer-core` owns that
end exactly as it owns the colour a highlight is drawn in.

**A password field is the case that proves the division is right.** §12.7.5.3's Table 231 bit 14 has
its characters "echoed in some unreadable form", and `Answer::Field` hands a host bullets rather than
the value. So a host copying out of one copies bullets — it can draw what it may not know, and it
cannot leak what it was never given. An answer that carried the *text* of a selection would have had
to make that decision again, in a second place.

### The place is computed inside §12.7.4.3's own walk, for ADR 0211's reason

`variable_text::Asked` is what a question wants — a caret's offset, a point, a range — and
`LaidOut` carries the three answers back out. All of it is computed in the loop that writes the
stream, from the same numbers that place the glyphs: the x a line is positioned at by Table 228's
`/Q`, the widths measured by the formula that draws, the ascent and descent that put the baseline
where it is. A second implementation in a host would sit a cursor beside the text the first time an
auto-sized field, a `Tz` or a wrapped line disagreed.

Three things fell out of putting it there:

- **A comb field's inverse is a cell, exactly as its caret is.** Table 231 bit 25 divides the box
  into `/MaxLen` positions and puts one character in each, so a click names the cell it landed in
  and a selection covers whole cells.
- **`Encoded` gained the byte offset each code came from**, which is the mapping the caret already
  ran the other way — and it is built **only** when a point was asked about, so nothing that draws
  pays for the vector.
- **A range covering no glyph has no shape**, which two equal offsets are and which a range holding
  nothing but a line break also is. A caret is what a host draws there, and it draws it as a line.

### A caret is a collapsed selection in the host and two questions in the core

`viewer-ui` holds one pair of offsets and calls them equal when nothing is selected, which is what
makes Backspace with a selection take out the selection and typing replace it — one splice rather
than two behaviours. The core keeps the two questions apart because their *answers* are two shapes:
a segment with no width, and boxes with area. Modelling them as one message in the middle would have
forced one of the two ends to pretend.

Shift extends and a plain arrow collapses to the edge of what is selected, both this host's
conventions and neither anyone's clause.

## A third key-order defect, in the same shape as the first two

ADR 0211 found Escape exiting the program while a field had the keyboard, because the window's event
handler answered it three branches before `typed` was asked. **The same branch did it to `o` and
`?`**: an `o` typed into a field toggled the sidebar and a `?` opened the About card, for the same
reason and in the same place, and that survived the round that fixed Escape. The fix is one move —
`typed` is now asked before any of this host's own bindings — and the window run below presses `o`
at a field and reads the value back with an `o` in it.

## Consequences

- **A person clicks into a word and types where they clicked.** In the window: `160F-2019.pdf` under
  `Xvfb`, a click at (430, 174) into `A.NOM` and `t y p e d`; the caret at **x = 381–383** over rows
  169–179. A second click at **x = 368** moves it to **x = 369–371** — the only pixels in the frame
  that changed — and one `x` there sends `SetField { value: "typxed" }`, which is the letter landing
  at offset 3 because that is where the click was.
- **A drag selects part of a value and costs the document nothing.** Press at 358, three moves,
  release at 382: the highlight covers **x = 357–382** over the same eleven rows, in the selection's
  own blue — the pixel at x = 370 goes from (216, 226, 237) to (119, 160, 237), and nothing at 350
  or 390 changes. **No command is sent at all**: three moves, three presents at 37.1, 27.2 and
  25.0 ms, `pointer Dragged` absent from the trace and the page's own selection still empty. A
  selection inside a field is chrome, which is the same statement ADR 0211's arrow keys made.
- **Copy, cut and paste work and the boundary did not move.** Ctrl + C says *copied 5 bytes out of
  the field*; Ctrl + X sends `SetField { value: "d" }`, which is `typxed` with `typxe` taken out;
  Ctrl + V sends `SetField { value: "typxed" }` and the value is back, byte for byte. Each is one
  `Edit::SetField` and nothing else.
- **§12.7.4.3's fuzz target now asks the three questions**, because none of them draws and so nothing
  else reaches them: 50 000 runs clean with the offsets and the point taken from the fuzzer's own
  bytes, against a widget whose `/DA`, `/V` and comb count it also chooses.
- **Free text was re-read and is genuinely still more than a round.** `doc/todo/33` blocked it on
  three things and two have expired: the text has been laid out by §12.7.4.3 since the twenty-third
  session, and a host has had somewhere to type since the three-hundred-and-seventy-first. What is
  left is real — a geometry that comes from a drag rather than from a selection, an `Edit` verb and
  a `ViewState::add_free_text` beside §12.5.6.10's markup, and a caret path that works on an
  **annotation**: `appearance::caret` and both of its new companions begin by reading a `Field`, and
  a free text annotation is not one. The todo file says that now instead of the expired half.
- **What is still owed here** is a host that sends the markup command from a drag, and free text
  above. Nothing in this round changed a page any corpus document draws: all eight gates are
  unmoved, and the two new questions are asked by nothing that renders.
