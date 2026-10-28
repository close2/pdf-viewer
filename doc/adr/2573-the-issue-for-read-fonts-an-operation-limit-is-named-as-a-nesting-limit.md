# 2573 — The issue for `read-fonts`: an operation limit is reported as a nesting limit

Status: accepted. Session 1869, slot 9 of batch 127. The decision: this tree reports nothing upstream
itself, and the text below is the issue the orchestrator files at `googlefonts/fontations` (A681 item 3
leaves pushing and filing to it); nothing was filed from this round. ADR 2572 is the decision it follows.

## The issue's text

**Title**: `ps::cs`: exceeding `MAX_OPERATIONS` returns `CharstringNestingDepthLimitExceeded`

`read-fonts` 0.44.0 stops a charstring's evaluation once `ops_done > MAX_OPERATIONS` (1 000 000), which
is what keeps a subroutine fan-out — ten nested levels of *k* calls each, about *k*⁹ entries — from
running for seconds or hours; 0.42.1 had only the nesting limit and took 4.4 s on *k* = 8. The stop
returns `Error::CharstringNestingDepthLimitExceeded`, whose message names the ten-level nesting limit,
so a caller that reports why a glyph was not drawn tells its user that the font nests subroutines too
deeply when it does not. A distinct variant — for example `CharstringOperationLimitExceeded` — would let
a caller say which bound a font reached; `MAX_OPERATIONS` being public, or the limit configurable per
evaluation, would let a caller that walks the same charstrings itself spend the same budget without
restating the constant.

Reproduction: ten global subroutines, subroutine *i* holding *k* copies of `<i + 1 − 107> callgsubr`
and then `return`, the last only `return`, and a glyph `<−107> callgsubr endchar`. At *k* = 5 the glyph
is refused after about 17 ms with the nesting error, though it nests exactly ten deep.
