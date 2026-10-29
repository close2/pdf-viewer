# 2586 — A glyph the evaluator refuses is kept in the outline cache and named in the page's report

Status: implemented. Session 1876, slot 6 of batch 128, a parsers round (robustness). ADR 2587 and
Q752 are unused. I worked in `scratchpad/r1876/` and one export of this tree built there.

## Context

Principle 1 forbids a silent error. `LoadedFont::build_outline` (`pdf-font/src/loading/drawing.rs`)
ran every glyph program through `.ok()?` — `cff::draw`, `type1::Program::draw` and `skrifa`'s unhinted
draw — so a glyph the evaluator stopped on was cached as `None`, the answer for a glyph with nothing to
draw. `account_for_a_missing_mark` (`pdf-model/src/content/text/show.rs`) then counted it as a glyph
the program describes empty, and the page said either nothing (the font drew something else) or
"program has no outline for any of the code(s)", which is untrue of such a program. Since ADR 2572
the commonest such stop is `read-fonts`' operation budget, which a hostile fan-out of subroutines
reaches; a malformed charstring is the other.

## Decision

1. **The cache keeps the refusal.** Its value is `Result<Option<Arc<Path>>, OutlineRefused>`:
   `Ok(None)` is still a glyph with nothing to draw (a class face or glyph the program does not hold,
   or empty contours), and `Err` is the evaluator's error for this glyph, its `Display` text kept as
   the reason. `outline` answers `None` for both, so nothing drawn changes; `outline_refusal(code)`
   reads the same entry. The hinted draw's fall-back to the unhinted skeleton (ADR 0727) is unchanged.
2. **One report site, the fonts' own.** `account_for_a_missing_mark` asks `outline_refusal` after
   `glyph_lost_to_repair`, whose reason is the narrower, and before the program's own empty answer.
   `Coverage` counts the codes and keeps the first refusal; `complete` notes one
   `Unsupported::Font` per font: "font /F1: the evaluator refuses the outline of glyph 2 (…), and 1 of
   the code(s) the page shows through it reach a refused glyph and are not drawn". A font whose every
   empty code was lost to a repair or refused does not also get the no-outline report.
3. **The corpus gate classes it** as `Whose::NeitherOne`: the reason says whether the file is
   malformed or the budget was reached, and neither is a clause this tree leaves undone.

## Cost and bounds

The new allocation is one `OutlineRefused` per distinct refused glyph of a loaded font, bounded by
the program's glyph count (a `u16`), and one per font in a page's `Coverage`. `getrusage`'s
high-water for the refused page is 11.9 MB against 12.5 MB for the same glyph drawn. What this does
not change: a refused glyph still costs the evaluator up to `cff::CHARSTRING_OPERATIONS` operations
(about 17 ms) once per distinct glyph before it is refused. A code inside a Type 3 glyph description
reaches no readback and so not this report, and `character_glyph`'s interface text still drops a
refusal into `None`.
