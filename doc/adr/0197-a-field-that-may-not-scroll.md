# ADR 0197 — A field that may not scroll, and the axis its clause names

Status: accepted, 2026-08-05 (session 338).

## Context

§12.7.5.3's Table 231 bit 24:

> If set, the field shall not scroll (horizontally for single-line fields, vertically for
> multiple-line fields) to accommodate more text than fits within its annotation rectangle. Once
> the field is full, no further text shall be accepted for interactive form filling; for
> non-interactive form filling, the filler should take care not to add more character than will
> visibly fit in the defined area.

Two sentences and only the second binds a *reader*: a `shall` about **accepting** text. It was
found by `doc/todo/01`'s second sweep in the three-hundred-and-fourth session, in §12.7.5.3's own
"Not read:" list, behind the reason *"constrains typing"* — true when it was written and false
since the hundred-and-thirty-fifth session, which is when `ViewState::set_field` made this a
program a person types into. **A flag that constrains typing binds a program that types**, and it
had been binding for two hundred sessions.

The flag is not obscure. `examples/field_flag_census`, written for this round, reads Table 227's
`/Ff` through §12.7.4.1's inheritance for every widget in the corpus:

| | widgets | documents |
|---|---|---|
| `DoNotScroll` (bit 24) | **260** | 8 |
| `DoNotSpellCheck` (bit 23) | 101 | 9 |
| `NoToggleToOff` / `Radio` (15, 16) | 54 | 17 |
| `Comb` (bit 25) | 47 | 6 |
| `Pushbutton` (bit 17) | 42 | 23 |
| `Combo` (bit 18) | 26 | 17 |
| `ReadOnly` (bit 1) | 20 | 10 |
| `Multiline` (bit 13) | 15 | 11 |
| `NoExport`, `Sort`, `FileSelect`, `RadiosInUnison`/`RichText` | **0** | 0 |

833 widgets over 103 documents with an `/AcroForm`. `DoNotScroll` is the most-set of every
type-specific flag the four tables state, by a factor of two and a half — and four of the twenty
have no witness at all, which is the census's other half and worth keeping.

## Decision

### The question belongs to the layout, and the answer is one bit

`variable_text::LaidOut` gains `overflows: bool` — whether the value needs more room than the box
gives it. Not a byte offset, which is what `doc/todo/22` had scoped: the offset would have to be
threaded through `encode`, `wrap` and `write_lines`, all three of which work in the font's
*character codes* and have thrown the value's byte positions away by the time a line ends. A
predicate costs six lines and puts the measurement where the placement already is.

**The axis is one per shape, because that is what the parenthesis states.** A single-line field
overflows when its line is wider than the box; a multiline field when its lines are taller than the
box; a comb field when there are more characters than Table 232's `/MaxLen` cells — the third case
the sentence does not spell out and Table 231's own rule for bit 25 settles, since there the room
is a count rather than a length. A line wider than a *multiline* field's box is not overflow under
this clause: `wrap` has already decided where that line ends.

Each of the three is measured with the formula that *places* the text — `Measure::width` for the
line, `write_lines`' own `leading·(n−1) + ascent − descent` for the column. A question answered
with a second formula would say a value fits and then clip it.

### Accepting is where the `shall` lands, so `set_field` is where it is obeyed

`appearance::accepted_prefix` answers, for one widget, the byte length of the longest prefix of a
candidate value that does not overflow — and `None` where the flag is clear, the field is not a
text field, or nothing about the widget can be laid out at all. **That last one is deliberate**:
refusing a person's text on the strength of a layout this crate could not build would be a guess,
and the report `field_text` already raises is the honest answer there.

The search is a **bisection over the value's character boundaries**, on the property that a longer
value never fits where a shorter one does not: `wrap` is greedy left to right, so its state after
*n* codes does not depend on what follows, and auto-sizing only shrinks as the value grows. The
property is not assumed — the prefix returned is one this function measured — so a font with
negative advances costs an early cut rather than a wrong one.

### One field, one value, so the shortest prefix wins

§12.7.4.1 shares a field's value across all of its widgets and Table 231's flag belongs to the
*field*. A value that overflowed one widget's rectangle while fitting another's would make the
field full and not full at once, so `set_field` takes the shortest prefix any of the widgets that
are taking the value will accept. That is the reading under which the clause's sentence stays true
of the field rather than of one rectangle.

### A password field is measured as it is drawn

Table 231 bit 14 has a password field's characters "echoed in some unreadable form", and
`field_text` echoes them as bullets. So `accepted_prefix` measures bullets too. A bullet is wider
than an `i` and narrower than an `m`, and a field that accepted text by the width of characters it
does not show would be full at a different place from where it looks full.

### The truncated value is what is logged, and that is not ADR 0196's exception

`viewer-core`'s edit log records `Edit::SetField { field, value }` as the host sent it, and undo is
a replay. That is safe here where it was not for §12.5.6.10's markup: truncation is a pure function
of the document and the value, so a replay against the same document cuts in the same place, while
"mark up what is selected" was a fact about the moment the command arrived. **The distinction is
whether the ask depends on state the replay will not reproduce.**

## Consequences

- 260 widgets over 8 corpus documents now stop accepting text where the clause says they must.
  Nothing drawn changes: the corpus, oracle, text, quorra and every other gate reports the same
  numbers to the digit, because the flag reaches only what a *person* does.
- `examples/field_flag_census` stays, because the four flags with no witness are a fact about the
  corpus that will be wanted again — trap 8 in its usable form.
- **A host cannot yet read back what was accepted.** `Query::FieldAt` answers with §14.9.3's two
  names and no value, so a host with a text box of its own would drift from the field after a
  truncation. `viewer-ui` sends no `Edit::SetField` at all today, so nothing is wrong yet;
  `doc/todo/22` carries it beside the caret, which is the other half of what turns this from a
  truncation into the behaviour the clause describes — a person typing into a full field sees
  nothing happen.
