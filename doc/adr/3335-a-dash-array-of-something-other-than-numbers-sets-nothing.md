# 3335 — A dash array of something other than numbers sets nothing, and is reported

Session 2158. Status: accepted. Context: audit 2155's finding B04, ADRs 0018, 2759.

## The clause

§8.4.3.6: "The dash array's elements shall be numbers that specify the lengths of alternating dashes and
gaps; the numbers shall be nonnegative and not all zero. The dash phase shall be a number". §8.4.1:
"Most parameters shall be of the correct type or have values that fall within a certain range."

## What was wrong

`set_dash` and `/D` each kept an array's numbers and dropped the rest, so `[3 (x) 1]` became `[3 1]` — a
pattern the file never stated, in which the `1` that was a dash is now a gap. `apply_dash` draws an
array outside the clause's *range* solid; one outside its *type* became another pattern.

## Decision

One reading, `read_dash` in `pdf-model`'s `content.rs`, for `d` and `/D`: an element or a present phase
that is not a number makes the operands no pattern. §8.4.1's clipping is for a number out of range,
and a string has no range to be clipped into, so the operator **sets nothing** — the pattern in force
stays, the answer `Unsupported::ColourOperands` gives a colour operator whose operands state no colour
(ADR 2759) — and the page reports it as `StreamSyntaxFault::DashNotNumbers`, an `Unsupported::Operator`
sentence naming `d` or `/D`. An absent phase still reads as zero.
