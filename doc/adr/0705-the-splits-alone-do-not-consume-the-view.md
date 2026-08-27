# 0705 — The splits alone do not consume the view

**Status.** Accepted — the narrowing of ADR 0702's stroke ledger that its own
wording asked for ("not worth the audit yet"), audited now because the round's goal
is scenes that survive, and a dashed table border was costing every page that has one
its scene.

## The narrowing

`render-quorra`'s stroke encode marked the scene view-consuming whenever
§8.5.3.2's degenerate split or the dash cut *ran*. The audit says only their **marks**
read the view:

- Which subpaths are degenerate is the path's own geometry; the split's surviving
  remainder is path-space work, true anywhere.
- The dash pattern cuts in path space with no width in the arithmetic
  (`pdf_render::dashes_showing_direction` reads pattern and cap; `kurbo::dash` reads
  pattern and phase), and the zero-length-dash partition is a path-space length
  test against `ZERO_DASH`.
- The stroke that survives both carries its scene-space width to quorra, which
  resolves §8.4.3.2's zero and §10.7.5's adjustment per placement (ADR 0701).

What does read the view is the marks: dots and zero-length-dash caps are sized by the
resolved width and pass through §10.7.4's substitution, whose *decision* consults the
placement even when it changes nothing. So the condition is now exactly that: marks
made, or a substituted coverage — `!dots.is_empty() || coverage < 1.0` — and a dashed
stroke that made none keeps its scene across every zoom.

The full move of §8.5.3.2's dots into quorra's encode — the disc per placement, the
§10.7.4 point-mark substitution mirrored — remains deferred, now with its cost
stated: it duplicates `pdf_render::sub_pixel`'s construction for a shape the corpus
produces rarely, and the narrowing above already frees the common dashed page. It is
a deferral by argument, revisitable the day a corpus page shows dots on its zoom
path.

## Held by

The sixty `render-quorra` tests and the full workspace, unmodified: the narrowing
changes when a scene is *reused*, never what a build draws, and the reuse claim rests
on the walk producing identical commands at every viewport for the freed cases —
argued above clause by clause.
