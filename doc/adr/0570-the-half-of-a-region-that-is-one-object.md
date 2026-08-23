# ADR 0570 — The half of a region that is one object

Status: accepted, 2026-08-24. Session 706. Amends §11.7.5.2's ledger row from `reported` to
`partial`, and §11.7's and §11.7.5's with it. Continues ADR 0469, which built the report and priced
the model behind it; **corrects that price**, which had two things inside it at one number. Changes
nothing ADR 0479 or ADR 0487 decided about *where* §10.5's function is applied to a colour, and
nothing ADR 0505 decided about which entry it is read from.

## The decision

**A mark ISO 32000-2 §11.7.5.2 does not call fully opaque is painted with the page's default
transfer function**, which on this device is the identity — so its own colour, its samples and its
shading's ramp all reach the backend unmapped. The same holds for every mark inside a **soft mask's**
group.

**One function decides it**, `Interpreter::transfer_for_mark`, and it is the only reader of the
graphics state's transfer parameter left in the tree. `TransferState::in_force` — the accessor that
handed a `&Transfer` to any caller that wanted to apply one — is gone, because every one of its
callers was a place making §11.7.5.2's choice by omission.

**What remains reported is one overlap and it needs a point**: a fully opaque mark carrying a
function, seen through a *later* mark that is not fully opaque. The report's condition narrowed to
exactly that.

## The clause, and the deduction the price had missed

§11.7.5.2 names two candidates for the function at a point and nothing else:

> The halftone and transfer function to be used at any given point on the page shall be those in
> effect at the time of painting the last (topmost) elementary graphics object enclosing that
> point, but only if the object is fully opaque.

> For portions of the page whose topmost object is not fully opaque or that are never painted at
> all, the default halftone and transfer function for the page shall be used

Take an object *O* that fails any of the clause's six conditions, and any point *p* it encloses.
Either *O* is the topmost object at *p*, in which case the first sentence withholds *O*'s function
and the second supplies the page's default; or something else is topmost at *p*, in which case that
object's function or the default is chosen. **Neither branch can choose *O*'s.** So *O*'s function
is used nowhere on the page — which is a statement about one object, decided from one graphics
state at the moment it is painted, and it needs nothing per-point to implement.

That is exact rather than an approximation, and it is worth being clear about why the *second*
branch costs nothing. Where the topmost object at *p* is fully opaque, the clause's own six
conditions "ensure that only the object itself shall contribute to the colour at the given point,
completely obscuring the backdrop" — so whatever *O* contributed there is invisible, and what was
done to *O*'s colour cannot be seen either.

**The page's default is a documented choice.** Table 52 says only "a PDF reader shall initialise
this to a suitable device dependent value", and for this device it is the identity: the same table
describes the parameter as a function that "adjusts device gray or colour component levels to
compensate for nonlinear response in a particular output device", and nothing this program puts
between a `Command::Fill`'s colour and the raster is such a response. Recorded as a choice, in the
sense principle 5 asks for, rather than left as a silence.

## The soft mask, which is the same deduction one step earlier

§11.7.5.2 is addressed throughout to "any given point on the page". §11.5.3 makes a mask group's
result "the luminosity of the resulting colour" — a one-component mask rather than ink at a point —
so no mark inside one is ever "the topmost elementary object in the entire page stack" anywhere on
the page, and the deduction above applies to it without needing the opacity conditions at all.

§11.5.3 says it a second time from the other direction, of the conversion itself: for a device
colour space the luminosity is taken "with no compensation for gamma or other colour calibration",
and gamma compensation is precisely what Table 52 calls a transfer function. §11.7.5.3's NOTE says
it a third time, of the moment rather than the place — the halftone and transfer function are the
parameters "whose values are used only when all colour compositing has been completed and
rasterization is being performed".

Before this round a mark inside a mask group had its colour mapped, and that colour became the
mask's luminosity. Nothing measured said so, because the only thing that ever asked about a transfer
inside a mask was the report, and the report was scoped away there.

## What was wrong with the price, and what the new one is

`doc/todo/13` and §11.7.5.2's row both priced the model as **a per-pixel transfer identity beside
the colour plus a matching pass in all three backends**. That price is right for what is left and
was wrong for the whole, because it was quoted for two different questions at once:

| the clause asks | what it needs | price |
|---|---|---|
| which marks may use their own function | one graphics state, at the mark | a function in `pdf-model` |
| what happens where a translucent mark covers a transferred opaque one | which objects cover which points | a channel and three backend passes |

`doc/habits.md`'s standing rule is that a price is a claim that decays; this one did not decay so
much as arrive conflated. **The tell, in hindsight, is that the clause's own text separates the two
and the price did not**: the first sentence is about "the last (topmost) elementary graphics object
enclosing that point", and the words that made half of it cheap — "but only if the object is fully
opaque" — attach to the object rather than to the point.

The remaining half is still owed and still priced as a rasteriser change, with one design question
recorded beside it in `doc/todo/13`: the clause's topmost object is the one with a "nonzero object
shape value ( f j)" at the point, so a pixel an antialiased edge covers *partially* is inside that
object and takes its function although its colour is a blend. Annex N.3 names the artefact that
follows — "a fringe using an unexpected halftone" — while being informative and, by its own §N.1,
conditional on "an output device that requires halftoned output", so it licenses nothing here and
warns about the right thing.

## Trap 2, and why the decision is in `pdf-model`

A device decision either backend can make alone is a decision neither has made. This one is not a
backend decision at all today — the transfer is applied while the display list is built — but the
shape it leaves behind matters for the half that is owed: **the meaning of a transfer identity, and
what the page's default is, belong to `pdf-render` and not to a rasteriser's pass.** Stated here so
that the round taking it does not have to rediscover it.

Within `pdf-model` the same rule applied one level down, and it is what the diff mostly is. There
were five places that reached for `state.transfer` and applied it: the solid fill, the solid stroke,
an image's samples, a shading pattern's rebuild and the `sh` operator's `Colouring`. Each of them
was answering §11.7.5.2 by not asking it. They now take the answer as an argument, and the
graphics state's parameter has exactly one reader.

## What it costs, and what it moved

Nothing on the corpus, and that is derived rather than hoped for.
`examples/transfer_function_census` finds one document in `doc/pdf.js` stating a transfer function
that is not `/Identity` or `/Default`; that document draws one image, fully opaque, under the Normal
blend mode with no mask; the corpus gate raises no `TransferFunction` report for any page. So the
only marks whose colours could move are marks no corpus document makes. The oracle's verdict counts
and every ratchet in the sequence are unchanged, and the witness page — `issue6931_reduced.pdf`,
whose text says *The color should be red* — still draws a red heart on white.

Four fixtures defend it, each with its mutation, and all four were run against the tree before the
change and fail there with the exact wrong colour (trap 13): a `ca` below 1.0, a group invoked
translucently, a soft mask's group, and the overlap that is still reported.

## What this does not decide

- **Overprinting.** §11.7.5.2's last paragraph makes the whole determination per-component where
  the applicable overprint parameter is true, on the condition that "overprinting yields the source
  colour (not the backdrop colour) for that component". Nothing here reads it, and the row does not
  claim to.
- **Halftones.** §10.6's screens stay inapplicable on §10.1's own condition. Only the
  `TransferFunction` entry of a halftone dictionary is read, which is ADR 0505's.
- **Where the function is applied**, which is ADR 0479's answer and unchanged: inside a shading's
  sampling rather than over a finished ramp.
