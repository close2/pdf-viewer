# 3828 — The survey does not hold the leading a form is invoked with, so a form's leading stays unstated

Session 2281. Status: **accepted**. This is a reading, and its construction is named but not built.
Context: ADR 3801, which starts the leading of a stream other than a page's own content as unknown,
and ADR 3697, which carries the colour spaces in force at a `Do` through the survey. Clauses:
ISO 32000-2 §8.10.1, §9.3.5, Table 103.

## Reading

Record 2275 asked whether the leading in force at a form's `Do` can be carried into the walk through
the survey, as the colour spaces are. **It cannot, as the survey stands.** `pdf_archive::survey`
classes `TL` as `Operator::Other` and `TD` as `Operator::Position`. It keeps an operator's names, its
integer and whether any number is non-zero, and drops the values. Its `State` has no leading, and
`OpenedStream` records `began_in` (the spaces) and nothing about the text state.

## The construction, when it is built

- `State` gains `leading: Option<f32>`, which is 0 at a page's start (Table 103). `TL` sets it to its
  operand and `TD` to `-ty`, each narrowed to single precision as the interpreter narrows it. `q`
  and `Q` carry it with the rest of the state.
- `OpenedStream` gains the leading at the `Do`. `streams_began_in`'s rule applies: a form invoked
  under two leadings is started with none.
- `as_drawn`'s `Leading` starts a form's walk from that value and not from `None`.
- **What blocks it from being a small change** is ADR 3801's condition that an operator the reader
  may not have run as written leaves the leading unstated. The survey does not see the content
  reader's repairs. Either the survey learns which operators were repaired, or a repaired `TL` or
  `TD` anywhere before the `Do` makes the invoking leading unknown.

ADR 3558's proof by drawing both files still decides whatever the walk states. So a wrong leading
would be refused there and never written. The cost of leaving it unstated is that a show after a cut
inside a form, which follows a `T*`, `'` or `"`, is refused or photographed where it would draw the
same enclosed. Record 2275 names no document that needs it.
