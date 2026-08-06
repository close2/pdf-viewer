# ADR 0201 — The host keeps the place, and not the text

Status: accepted, 2026-08-06 (session 349).

## Context

This program has been able to fill in a form field since the hundred-and-thirty-fifth session, and
**no window could do it.** `ViewState::set_field` is what a person typing means, `Edit::SetField` is
the message that carries it, and the only consumer either ever had was
`viewer-core/tests/headless.rs`. `viewer-ui` sent neither.

ADR 0197 made that gap visible from the other side. §12.7.5.3's `DoNotScroll` binds a program that
accepts text, so `set_field` now takes only as much of a value as fits the widget's rectangle — and
that ADR's own consequences section says what it left owed:

> **A host cannot yet read back what was accepted.** `Query::FieldAt` answers with §14.9.3's two
> names and no value, so a host with a text box of its own would drift from the field after a
> truncation.

Two owed things, one round: a host that types, and the answer it needs in order to.

## Decision

### The host keeps the point, not the text

A host typing into a field could hold a buffer of what it had typed and send the whole thing on
every keystroke. That host would be **wrong on the first character past the edge of a
`DoNotScroll` field**: the core would truncate, the buffer would not, and every subsequent
keystroke would re-send a value the field had already refused.

So `viewer-ui` holds `typing: Option<(f32, f32)>` — **a point on the page and nothing else.** Every
key press re-asks `Query::FieldAt` for what the field says *now*, appends one character to that,
and sends it back. Divergence is not unlikely; it is impossible, because the host has nothing of
its own to diverge.

The point is the right thing to keep because a field does not move. The alternative — keeping the
qualified name — would need a second query to reach the value by name, and the name is already in
the answer.

It costs one query per keystroke, which is a walk of one page's annotation array. A person types at
ten characters a second.

### `Answer::Field` carries the value, and an empty field is not an absent one

`Query::FieldAt` answered with two names. It now answers with two names and `value: Option<String>`,
which is the ADR 0166 and 0167 shape for the third time: *where a host needs a thing a variant does
not carry, the variant changes and every consumer fails to compile*.

**The distinction that made this work is `None` against `Some("")`.** `None` is a field whose value
is not text at all — §12.7.5.2's buttons select an appearance, §12.7.5.5's signatures hold a
dictionary, §12.7.5.4's list box states which items are selected — and `Some("")` is a text field
with nothing in it, which is what 147 of the corpus's first-page widgets are.

That is exactly what a host deciding where to send the keyboard needs, and it is §12.7.4.3's own
line: the clause lays text out for `Tx` and for a combo `Ch`, and for nothing else. Folding an empty
text field into the absent answer would have forced the host to ask a second question, or to guess.

The first draft did guess, and it was worth throwing away: it probed by sending
`Edit::SetField { value: None }` and watching whether anything took it. That writes to the edit log
to answer a question about the document — `CLAUDE.md` rule 1's spirit in a host — and the answer it
computed was meaningless anyway.

`ViewState::field_value` is the model half, and it resolves the same four statements about a value
in the same order `ViewState::annotation` already does: what a person typed, §12.7.8's import,
§12.7.6.3's reset, Table 226's `/V`. A password field answers with Table 231 bit 14's bullets: a
host is allowed to draw them and not to know them.

### The keyboard goes to the field, and Escape is what leaves

While a field has the keyboard, the host's other bindings do not fire — `+` is a plus sign there
and a magnification everywhere else. **Escape leaves the field rather than the program**, which is
the one binding this state changes the meaning of and the one a person expects it to. A press
anywhere the query answers no field puts the keyboard back on the page.

**Enter offers a newline and the field decides.** §12.7.5.3's Table 231 bit 13 is what makes a value
lay out on two lines, and `variable_text::wrap` is where it is read — so the host hands over the
character and does not decide whether a return means a line or the end of typing. A single-line
field simply lays the newline out as nothing.

## Consequences

- **A person can fill in a form field in a window**, which the handover has been describing since
  the hundred-and-thirty-fifth session on the strength of the headless host alone.
- **§12.7.5.3's `DoNotScroll` has a witness a person can see.** 260 corpus widgets over 8 documents
  set it, and until this round nothing in a window could reach one.
- **The caret is still `doc/todo/33`'s**, and this round makes the case for it concrete rather than
  theoretical: a person typing into a field sees the text but not where the next character goes.
  The text layer already has the geometry.
- **What is still not offered**: selecting inside a field, arrow keys within a value, and typing
  into a field reached by the tab key rather than by a click. §12.5.1's focus ring already marks
  which annotation the keyboard walk is on; joining that to this is one query away and is a
  separate decision, because a focus ring on a *button* means something else.
