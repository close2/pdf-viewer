# 3802 — A form drawn in two colour spaces is refused by the class's own sentence

Session 2275. Status: **accepted and built** in `pdf-transform` (`archive/base_standard.rs`'s
sentence for `model/colour-operands-do-not-fit-the-space`). Follows ADR 3697 and ADR 3679.
Clauses: ISO 32000-2 §8.10.1, §8.6.8, §8.4.1.

## Context

Record 2258 found a probe form painted first in `DeviceGray` and then in `Lab`
(`q /Fm0 Do Q /CS0 cs /Fm0 Do`) refused after the write's proof, and did not follow it. §8.10.1
starts each `Do` of the form in the state in force there, so its `1 0 0 scn` is three operands that
do not fit `DeviceGray` — §8.6.8's repair, which this reader draws as `rg` — and three that fit
`Lab`. ADR 3697 judges a stream begun in two states in neither (`Spaces::UNCOUNTED`).

## Cause

No defect: the survey records both openings, the walk starts the form in `UNCOUNTED` and respells
nothing, and no write remains for the class, so it stays refused. What the probe met was the
sentence: `in_content_operator!`'s text said the operator "is written out", which is not this class's
write (ADR 3679 respells it), and named nothing of the two spaces.

## Decision

**Refused by name, with the reason.** One stream drawn under two spaces is drawn two ways, and any
write of that stream changes one of them: written as `rg`, the `Lab` occurrence would draw RGB;
left, the `DeviceGray` occurrence keeps the repair. Splitting the form into one copy per space is a
change of structure no clause asks for, and is not built. The class's sentence now says what is
written (the device operator, operands kept, ADR 3679), when (content-as-drawn granted, the stream
drawn in one space wherever it is drawn, the page drawn the same), and that a form painted under two
spaces has no such write. `as-if-printed` photographs the page.

## Consequences

`a_form_drawn_in_two_colour_spaces_is_refused_by_name` fixes both outcomes. A document holding the
class elsewhere reads the new sentence; no answer changes.
