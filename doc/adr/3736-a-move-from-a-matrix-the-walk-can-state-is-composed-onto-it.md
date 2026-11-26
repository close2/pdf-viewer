# 3736 — A move from a matrix the walk can state is composed onto it

Session 2258. Status: **accepted and built** in `pdf-transform` (`archive/as_drawn.rs`:
`Matrices::after`, `StatedMatrix`, `Moved`). Extends ADR 3661, whose walk gave up at every `Td`.
Clauses: ISO 32000-2 §9.4.2, Table 106.

## Decision

Table 106's `Td` sets the text matrix and the line matrix to a translation by its operands times
the line matrix held; `TD` does the same and sets the leading. Where the walk can state the line
matrix — the identity, or a matrix a `Tm` or an earlier move stated — and the operator ran as
written, the new matrix is computed as this reader computes it (single precision, the operands
narrowed as the interpreter narrows them, `pdf_render::geom::Transform::then`) and written, when a
text object added must restate it, as the shortest decimals that read back as the same numbers
(ADR 3699's `operand`). `T*`, `'` and `"` move by the leading, which the walk does not follow, and
still leave the matrix unstated. ADR 3558's proof decides, as before.

## Consequences

A show after a show across a cut — `Td (A) Tj q Q 0 -20 Td (B) Tj`, ADR 3661's left case — is
enclosed in a text object opening with the `Tm` the first move stated.
