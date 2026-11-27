# 3801 — A move by the leading is composed as a `Td` is

Session 2275. Status: **accepted and built** in `pdf-transform` (`archive/as_drawn.rs`: `Leading`,
`Matrices::after`, `Matrices::moved_by`). Extends ADR 3736, which left `T*`, `'` and `"` unstated.
Clauses: ISO 32000-2 §9.3.1 Table 103, §9.3.5, §9.4.2 Table 106, §9.4.3 Table 107, §8.4.2, §8.10.1.

## Context

ADR 3661 encloses text operators this reader ran outside a text object in one it adds, opening with
the matrix they were drawn by; ADR 3736 states that matrix across a `Td` or `TD`. `T*`, `'` and `"`
move by the leading, which the walk did not follow, so a show after a cut that followed one of them
was refused (or photographed) where it draws the same enclosed.

## Decision

1. **The walk follows the leading.** Table 103 gives `TL` "Initial value: 0", so a page's own content
   starts at 0; a form, a glyph description or a pattern's cell inherits the state of its invocation
   (§8.10.1), which the walk does not state, and starts with none. `TL` sets it; Table 106's `TD`
   "shall have the same effect as this code: - t y TL t x t y Td" sets it to `-ty`; a `q` saves it
   with the graphics state and a `Q` restores it (§8.4.2), a `Q` with nothing saved restoring nothing
   as the reader ignores it. Each operand is narrowed to single precision as the interpreter narrows
   it. An operator this reader may not have run as written (any repair but §9.4.1's and §7.8.2's
   operands left over) leaves the leading unstated, and so does a stack deeper than the reader's own
   bound of 256 saved states.
2. **`T*` is `0 -Tl TD`**, Table 106's own words, and `'` and `"` begin with it (Table 107): each is a
   move by `(0, -Tl)` composed onto the line matrix held, as ADR 3736 composes a `Td`; `'` and `"`
   then show a glyph, so the text matrix moves past what the walk measures and the line matrix is
   the one held.
3. **`"`'s `aw` and `ac` need no restating.** They set `Tw` and `Tc`, text state that lives in the
   graphics state and that a `BT` does not reset (§9.3.1), so the object added draws with the values
   the reader held. Only the position needs the walk.
4. ADR 3558's proof decides, as before.

## Consequences

`14 TL 10 100 Td T* (A) Tj q Q 0 -20 Td (B) Tj` writes `BT 1 0 0 1 10 86 Tm 0 -20 Td (B) Tj ET` after
the `q Q`, and the same after `'` and `"`; a `TD` and a restored leading are tested. A form's moves by
the leading stay unstated; following the leading into a form through the survey is not built.
