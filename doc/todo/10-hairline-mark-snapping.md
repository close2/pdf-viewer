# A §10.7.4 mark that moves with sub-pixel placement

Status: **diagnosed, proposal written, not implemented.** The project owner has asked for it in
one of the next rounds.
Priority: 10 — visibly wrong pixels, on a page this tree fixed two weeks ago
Corpus: 1 (`issue4260_reduced.pdf`); the shape is general
Clauses: §10.7.4, with §10.7.5 as the boundary — see `_scan-conversion.md`
Code: `crates/pdf-render/src/collapsed.rs`, and both backends through it
Proposal: `doc/QUORRA_HAIRLINE_MARKS.md`, written from the quorra side

## What is wrong

`split_collapsed_fill` gives a degenerate fill one device pixel of ink **at the shape's own
fractional position**. A rule at y = 1085.0 comes out crisp; one at y = 1085.3 is an
anti-aliased band split half and half across two rows. A page ruled with both — which is what a
grid is — renders as a mix of crisp lines and fuzzy grey double ones.

Measured beside Okular on a high-DPI screen, by a vertical cut through the horizontal rules:

| | rows per line | ink per line |
|---|---|---|
| this viewer | **2** | **1.00**, split roughly half and half |
| Okular | **1** | **0.13** |

Two renderers, two different halves of the right answer: ours is the right amount of ink in the
wrong place, Okular's is the right place with an eighth of the ink.

`cargo run --release -p render-quorra --example mark_width` is the instrument — it renders the
page through both backends at 1×, 2× and 4× and prints each line's starting row, rows touched
and total ink.

## What the clause determines

§10.7.4 paints *any pixel whose half-open square region intersects the shape*. A zero-height
line at y = 1085.3 intersects exactly one pixel row, and the clause's own rendering of it is
that row, painted — crisp like Okular's and full-ink like ours. Today's band is an
approximation of the clause, not the clause.

**And the strongest argument is one the proposal does not make.** The sentence's stated purpose
is that no shape disappears "as a result of unfavourable placement relative to the device pixel
grid". A mark whose appearance depends on where it falls between two pixel centres is *exactly*
placement-dependent. Snapping is what the sentence is for.

## The proposal

In `pdf_render::split_collapsed_fill`, where every backend inherits the same answer (trap 2):

- the helper gains the placement transform (it already takes `thinnest`, which the caller
  derives from that transform);
- under an **axis-preserving** transform — which these marks essentially always ride — each
  collapsed subpath's mark becomes the run of whole device pixels its zero-extent axis
  intersects, mapped back to path space, so the ordinary fill machinery paints it at full
  coverage;
- under a rotation or a shear, pixel alignment has no meaning along the mark's axis and today's
  band stays, as the stated fallback.

Roughly half a day including tests, on the quorra side's offer to implement.

## Two things to settle before the code moves

**1. Does the hairline *stroke* snap with it?** `render-cpu/tests/zero_area_fill.rs` asserts the
mark is byte-identical to a `0 w` hairline stroke, and that stroke splits across two rows for
the same reason. Snapping one and not the other breaks an identity two constructions currently
have. But snapping the stroke would be §10.7.5's grid-fitting applied without `/SA`, which is
precisely what `AMBIGUOUS_STROKE_ADJUSTMENT`'s reading of `bug1743245.pdf` rests on this tree
*not* doing.

The reading that lets them diverge, and it should be written into the ADR rather than assumed: a
stroke has a **stated width**, and §10.7.5 governs whether its coordinates are adjusted,
conditionally; a degenerate fill has **no width at all**, and its mark is wholly this
processor's construction under §10.7.4, where no condition applies. If that is accepted, the
byte-identity test becomes an ink test and says why.

**2. It moves the oracle.** `render-cpu` is the correctness oracle, so every corpus and oracle
ratchet touching a page with degenerate fills shifts. The expectation is that they shift
*toward* the references — Adobe and poppler both snap hairlines — but an expectation is not a
measurement, and re-ratcheting is this tree's decision rather than a backend's. Re-run the
corpus and the oracle at 1× and 4× and bring the before/after numbers into the ADR.
