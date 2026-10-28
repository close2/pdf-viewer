# 2572 — A charstring is evaluated under `read-fonts`' operation budget, and this crate's own walks spend the same

Status: implemented. Session 1869, slot 9 of batch 127, a parsers round (security). ADR 2573 is the
upstream issue's text; Q735 is unused. I worked in `scratchpad/r1869/`.

## Context

Principle 3 asks for an explicit budget on every loop an input reaches. Adobe Technical Note #5177's
Appendix B bounds subroutine nesting at ten levels, and `read-fonts` 0.42.1 enforced that and nothing
else, so a subroutine that calls the next one *k* times is entered about *k*⁹ times: measured under
the bound on the evaluator alone (a bare CFF of ten global subroutines), 0.65 ms at *k* = 3, 12 ms at
*k* = 4, 75 to 117 ms at *k* = 5 and 4.4 s at *k* = 8, with `getrusage`'s high-water flat at 12 MB —
the cost is time, and a 65 535-byte charstring has room for *k* in the tens of thousands. Every glyph
this tree draws or measures from a CFF, OpenType-CFF or Type 1 program goes through that evaluator
(`cff::draw`, `cff::advances`, `type1::Program::{draw, advance}`, `skrifa`'s outline for an `sfnt`'s
`CFF ` table in `loading/drawing.rs`, `embed/accented.rs`, `restate.rs`), and two walks of this crate's
own follow calls the same way: `cff::calls_local_subr` and `embed::reach::Reached::walk`.

The brief proposed a static pre-scan in this tree. The premise did not hold: `read-fonts` 0.44.0, the
version `skrifa` 0.47.0 depends on, counts every operand and operator a glyph's evaluation reads,
inside its subroutines, and stops the glyph past `MAX_OPERATIONS = 1_000_000` — the bound the
pre-scan would have computed, held by the evaluator itself and exact by construction.

## Decision

1. A stack decision, taken deliberately for this upstream limit: `skrifa` 0.45.1 → 0.47.0 and `read-fonts`
   0.42.1 → 0.44.0, in step as `Cargo.toml` requires, which moves `font-types` 0.12.2 → 0.12.6 with them;
   `fuzz/Cargo.lock` takes the same three pins (`fuzz_workspace`), and `vello`'s own `skrifa` 0.42.1 and
   `read-fonts` 0.39.2 are untouched because no glyph of a document reaches them. The API change
   is one: an INDEX's `get` and a charset's `glyph_id` return `Option`, at eight sites in `pdf-font` and
   `pdf-transform`. Every evaluator call site above is bounded by that one change.
2. `cff::CHARSTRING_OPERATIONS` restates the count, and `calls_local_subr` and `Reached::walk` spend it
   per glyph, one per operand or operator. A walk that spends it answers on its own safe side: the
   repair counts the glyph as calling a local subroutine, and the subset writer keeps every subroutine.
3. No static pre-scan is written. It would be a second Type 2 and Type 1 interpreter in a crate that
   reads charstrings through `read-fonts` precisely to avoid one (ADR 0006), and it would have to agree
   with the evaluator on stack heights, `hintmask` lengths and Type 1's `callothersubr` to be sound.

## Cost

A refused glyph costs up to a million operations before it is refused, 8 to 17 ms here, where the
pre-scan would have refused it before any. Each glyph is evaluated once per loaded font (the outline
cache keeps its answer), so a hostile font costs at most its glyph count times that. The evaluator
names the stop `CharstringNestingDepthLimitExceeded`, which is ADR 2573's issue. The refusal is not
yet reported by name: `loading/drawing.rs` answers `None` for an evaluator error, and `show.rs` counts
such a glyph as one the program describes empty.

## Tests

`cff_budget_tests.rs`: a glyph reading exactly the budget is drawn with its line and one operation more
is refused by `draw` and `advances`; a fan-out of five at ten levels is refused by both and by
`calls_local_subr`, which spends the budget to zero; three a level is drawn. `type1.rs` and
`embed/reach.rs` hold the same pair for a Type 1 program and the subset walk.
