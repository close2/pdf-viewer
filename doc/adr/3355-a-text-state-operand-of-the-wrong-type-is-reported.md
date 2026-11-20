# 3355 — A text state operand of the wrong type is reported, and a `Tf` leaves unset what it does not state

Session 2163. Status: accepted. Context: audit 2161's finding B03; ADRs 0303, 3335.

## The clauses

Table 103: `Tc`, `Tw`, `Tz`, `TL` and `Ts` take a number; `Tf`'s "size shall be a number"; `Tr`'s
"render ... shall be an integer". §9.3.1: "There is no initial value for either font or size; they shall
be specified explicitly by using Tf before any text is shown." §8.4.1: "Most parameters shall be of the
correct type".

## Decision

An operand present and of the wrong type is reported as `StreamSyntaxFault::TextStateOperandOfTheWrongType`,
the `Unsupported::Operator` shape ADR 3335 gave `d`. For the five number operators and for `Tr` the
parameter keeps its value, the answer `-1 Tr` already had: a string has no range to clip into. `Tr`
reports `1.5` and `/X` as it reports `-1`.

**`Tf` states a pair, and leaves unset what it does not state.** A name that is not a name leaves no
font, and a size that is not a number leaves the size unset — nought, which Table 103's "[z]ero sized
text shall not mark or clip any pixels" makes the one value that draws nothing — rather than the earlier
`Tf`'s size under the new font, which nothing in the file states. The same holds for a `Tf` short of an
operand, which the loop reports as a shortfall; there the type is not judged, because which operand the
one present was meant to be is not in the file. ADR 0303's sentence holds: a `Tf` that states no size
leaves the show undrawable, and drawing nothing is said.
