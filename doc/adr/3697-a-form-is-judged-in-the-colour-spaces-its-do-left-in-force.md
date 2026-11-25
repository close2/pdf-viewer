# 3697 — A form is judged in the colour spaces its `Do` left in force, and a mask group's repair is its own

Session 2249. Status: **accepted and built** in `pdf-model` (`content::grammar::Spaces`,
`Grammar::starting_in`; `soft_mask::SoftMaskRequest::group_id`), `pdf-archive` (`survey`'s `State::spaces`
and `began_in`, `OpenedStream::began_in`) and `pdf-transform` (`archive/as_drawn.rs`: `streams_began_in`).
Extends ADR 3541 (the grammar), ADR 3679 (the colour write). Context: the `MICOM` poster, refused under
`as-shown` by `model/colour-operands-do-not-fit-the-space` after round 2244.
Clauses: ISO 32000-2 §8.4.1, §8.6.8, §8.7.3.1, §8.10.1, §9.6.4, §11.6.5.1.

## The clauses

§8.10.1, after the five steps `Do` performs on a form, none of which touches a colour space:

> Except as described above, the initial graphics state for the form shall be inherited from the
> graphics state that is in effect at the time Do is invoked.

§9.6.4 has a Type 3 glyph description inherit the graphics state "at the point of invocation of the
text-showing operator", the CTM aside. §8.7.3.1's step b) installs for a tiling pattern's cell "the
graphics state that was in effect at the beginning of the pattern's parent content stream". A page's
content starts in §8.4.1's initial state, `DeviceGray` for both colours.

## What was wrong

The grammar started every stream in `DeviceGray`, so the survey and `as_drawn`'s walk judged three
`scn` operands in a form painted under a page's three-component `ICCBased` space as not fitting. The
interpreter does not: it runs a form in a clone of the invoking state (`content::xobject`). The
poster's form was refused for a repair no reader made, and the write `as_drawn` made of it (`rg`)
drew differently, so the proof refused it.

The poster's soft-mask group, which §11.6.5.1 makes a form `XObject`, holds `0 0 0 scn` in the
initial state the interpreter gives a mask (`GraphicsState::initial`): a real repair, written as
`rg` under ADR 3679. The interpreter logged it at the *page's* content site, because a
`SoftMaskRequest` carried the group resolved and not its object, so the page's own stream — which
no write touches — stayed refused.

## Decision

1. **The grammar starts where the interpreter starts.** `Grammar::starting_in(Spaces)`: a page and an
   annotation's appearance in `Spaces::INITIAL`, a form in the spaces at its `Do`, a glyph description in
   those at the operator that reached it, a pattern's cell in those its parent stream began in. The
   survey's `State` carries the spaces, set from the grammar before each operator is applied, and each
   `OpenedStream` records the spaces its walk began in.
2. **`as_drawn`'s walk begins each stream where the survey began it.** A stream the survey opened on
   several pages in different spaces starts in `Spaces::UNCOUNTED`, where no colour operator is
   judged, so none is respelled; its class, if the survey found one, stays refused by its sentence.
3. **A repair inside a soft-mask group is logged at the group's object**, as one inside a form is
   (`Interpreter::stream`), or at no holder where `/G` is a direct stream.

## Consequences

The poster converts under `as-shown`: its form is no longer refused, and its mask group's `scn` is
written as `rg` at the loss `content-as-drawn`. A form drawn in the same space as its `Do` no longer
reports a repair to the validator. Nothing is respelled in a form whose starting space depends on
the page.
