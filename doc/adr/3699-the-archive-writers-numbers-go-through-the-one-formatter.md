# 3699 — The archive writers' numbers go through the one formatter, and a non-finite one refuses its stream

Session 2249. Status: **accepted and built** in `pdf-transform` (`archive/mod.rs`: `operand`,
`widened_operand`, `NUMBER_NOT_FINITE`). Completes ADR 3686's "Left". Clauses: ISO 32000-2 §7.3.3.

## Context

ADR 3686 put every writer of a computed number outside `archive/` onto `pdf_syntax::write`'s
`real_number`, which writes §7.3.3's real and answers an infinity or a NaN with `NonFinite`. Three
writers in `archive/` were another lane's that round: `derived.rs`'s wrapper matrix (`{value}`, which
writes `inf` and `NaN` as names), `preserve.rs`'s `number` (two places, so a line's baseline moved by
up to 0.005 of a point) and `matrix_operand` (through `write::object`, which writes `0` for a
non-finite `Object::Real`), and `fonts/photographed.rs`'s `number` (four places, a negative zero
taken off).

## Decision

- `archive::operand(f32)` is `real_number_f32`: the shortest decimal that reads back as the same
  `f32`, an integral value as an integer. It writes `derived.rs`'s matrix, `preserve.rs`'s text page
  (`Tf`, `Td`) and the photograph's placement.
- `archive::widened_operand(f32)` is `real_number` of the value widened, for §12.5.5's matrix that
  `preserve.rs` writes so that the reader's own `f64` recovers it exactly (ADR 2090).
- Either refuses with `Because::NotBuiltYet(NUMBER_NOT_FINITE)`; the stream is not written and the
  conversion says why. The functions that compose a stream return the refusal beside their old
  `None`, which still means a stream that would not encode.

## Consequences

The derived page's matrix bytes are unchanged for every finite value. The text page's and the
photograph's operands now state the value computed rather than a rounding of it; both are this
program's own marks on a page this program composes (the appended page of preserved text, ADR 3557's
photograph), so no producer's mark moves.
