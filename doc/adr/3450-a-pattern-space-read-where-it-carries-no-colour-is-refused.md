# 3450 — A `Pattern` space read where it carries no colour is refused, and said

Session 2187. Status: **accepted and built.** In `pdf-colour`: `shading::kind_of` and `parsing::parse_indexed`; in
`pdf-model`: `Interpreter::set_colour` and `Unsupported::PatternOperands`, with its sentence in `viewer-core`.
Finding 2184-B04. Clauses: ISO 32000-2 §8.7.1, Table 77, §8.6.6.3, §8.6.8.

## Context

§8.7.1: a `Pattern` space uses "pattern objects as the equivalent of colour values instead of the numeric component
values used with other spaces". Three routes read numbers in one anyway, and `ColourSpace::to_rgb` answers a
`Pattern` space with no underlying space with opaque black, so each drew black and the page reported itself
complete: a shading whose `/ColorSpace` is a `Pattern` (Table 77: "any colour space except a Pattern space"), an
`Indexed` space over a `Pattern` base (§8.6.6.3: the base "shall not be a Pattern space or another Indexed space"),
and `scn` given numbers and no name in a `Pattern` space (§8.6.8: "name shall be the name of an entry in the Pattern
subdictionary").

## Decision

- **The shading is refused**: `kind_of` answers `ShadingError::Malformed` naming Table 77, and the caller reports it
  as it reports every shading it cannot build.
- **The `Indexed` space is unsupported**: `parse_indexed` answers `None` for a `Pattern` base, and `cs` reports the
  space where it was selected, as it does any space it cannot build. The colour then drawn is the interpreter's one
  fallback for an unbuildable space, which is unchanged here.
- **Numbers with no name leave the colour in force**, whatever the `Pattern` space's underlying space: those numbers
  are the colour an uncoloured pattern is poured through *beside* its name, and alone they select no pattern. This is
  `Unsupported::ColourOperands`'s answer to operands that state no colour (ADR 2759), raised as its own variant,
  `PatternOperands`, because "takes N" says nothing true of a space whose operand is a name.

## Consequences

- `PatternOperands` is a new public variant. `pdf-transform`'s archive photograph lists the stream's own faults by
  variant (`is_a_fault_of_the_documents_own_stream`); it does not list this one yet, which is the conservative answer
  there, and adding it is that crate's lane's.
- An `Indexed` space over another `Indexed` space, which the same sentence excludes, is not refused here: its table
  bytes are still numbers in a space that has them, and no finding names it.
