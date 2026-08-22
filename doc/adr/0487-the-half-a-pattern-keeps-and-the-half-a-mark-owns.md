# ADR 0487 — The half a pattern keeps, and the half a mark owns

Status: accepted, 2026-08-22. Session 660. Completes ADR 0483, which read the three clauses that
were said to meet at a shading pattern's `scn`, implemented two of the three answers and priced the
third with a warning attached. Amends §10.5's, §11.6.7's, §11.4.7's and §11.7.5.2's ledger rows;
**removes the condition ADR 0483 added to `group_press`**, exactly as that ADR said would happen.
Changes nothing ADR 0479 decided about *where* §10.5 is applied to a shading's colours.

## The decision

**A shading pattern's colours are built again at the mark that paints them**, through
`shading::Cache`, wherever the mark asks for a colouring the `scn` did not build under.

**The two quantities a mark may supply are §10.5's transfer function and §11.7.2's compositing
target, and nothing else.** §11.6.7's black point compensation, rendering intent and smoothness stay
the definition's, and that is enforced by the *shape of two function signatures* rather than by a
rule: `Interpreter::mark_colouring` and `Interpreter::build_shading` take a `&ShadingDefinition` and
have no `&GraphicsState` parameter at all.

**`group_press`'s fifth condition comes off**, because the compositing target is now answered by
building rather than by refusing.

## The clause, in two sentences that sit four lines apart

§11.6.7 is usually quoted for its first sentence about a pattern's definition, and this round's
work is in what the same subclause says next. The definition first:

> The definition shall not inherit the current values of the graphics state parameters at the time
> it is evaluated; those parameters shall take effect only when the resulting pattern is later used
> to paint an object.

> Any parameters that are not so specified shall be inherited from the graphics state that was in
> effect at the beginning of the content stream in which the shading pattern is set to be the
> current colour in the graphics state or in which the sh operator is used.

and then, in the paragraph immediately after the four bullets, the painting:

> When the pattern is later used to paint a graphics object, the colour, shape, and opacity values
> resulting from the evaluation of the pattern definition shall be used as the object's source
> colour ( 𝐶𝑠 ), object shape ( f j ), and object opacity ( qi ) in the transparency compositing
> formulas. This painting operation is subject to the values of the graphics state parameters in
> effect at the time, just as in painting an object with a constant colour.

**Two moments, named in one place, and the code had only ever had one of them.** Which parameters
fall on which side is not this ADR's judgement — the clauses say:

- **§10.5's transfer function belongs to the painting.** §11.7.5.2 puts it at "the last (topmost)
  elementary graphics object enclosing that point", and §11.7.5.3's NOTE takes it out of the group
  evaluation altogether: "[t]his differs from the current halftone and transfer function, whose
  values are used only when all colour compositing has been completed and rasterization is being
  performed."
- **§11.7.2's compositing space belongs to the painting.** §11.6.7 makes the pattern's definition an
  implicitly enclosed *non-isolated* group, and §11.7.2 says "[n]on-isolated groups shall inherit
  their colour space from the nearest ancestor isolated parent group" — which for a pattern painted
  inside a group is that group, whichever content stream the `scn` stood in.
- **§8.6.5.9's black point, §8.6.5.8's intent and §10.7.3's smoothness belong to the definition**,
  by §11.6.7's own third bullet, which names two of the three outright. ADR 0483 is the reading and
  `content::PatternInitial` is the implementation.

`cargo run --release -p spec-errata -- emit doc/*.pdf` over all fourteen documents, clauses 10 and
11 read first: **no annotation falls in §10.5, §11.6.7, §11.7.2, §11.7.5.2, §11.7.5.3 or any of
§8.7.4.1–§8.7.4.3.** The nearest are §10.7.2's pair, which amend Table 57's *flatness* and not
§10.7.3's smoothness; §11.6.6's three, on pages 436 and 437, whose texts are Table 145's two
deprecations and "of the transparency group XObject", none of which strays into §11.6.7; and
§11.4.8's four `a`→`α` corrections inside its own formulas. Confirmed rather than assumed, which is
what a tool that files an annotation by the page a heading opens on requires of its reader.

## The price, re-derived before the code

`doc/habits.md` now holds that a price is a claim that decays, and ADR 0483's was "a hundred lines
and a fixture". **The cheapest re-derivation is asking what the layers already contain**, and here
five of the six pieces already existed:

| the piece | already there |
|---|---|
| a table keyed by object, resolution and conversion, so a second build is a lookup | `shading::Cache`, ADR 0069 |
| §11.6.7's three parameters, scoped per content stream | `content::PatternInitial`, ADR 0483 |
| a conversion built from a black point that is *not* the state's | `Interpreter::conversion_under`, split out by ADR 0483 for this caller |
| §8.7.2's matrix mapped to the parent content stream's default space | `Interpreter::base`, since the fifty-second session |
| the three colouring inputs as one argument | `shading::Colouring`, ADR 0479 |
| **the definition itself, kept past the `scn`** | *nothing* — `pattern()` dropped it |

So the change is one struct that keeps what was being thrown away, one comparison, and moving two
methods off `GraphicsState`. The last of those is where the extra lines went: `fill_paint` and
`stroke_paint` had no document and no cache, so they became `Interpreter` methods and the graphics
state kept `solid_fill`, `solid_stroke` and two new predicates. **The count came out at 358 lines
added and 173 removed across six source files — a net 185**, against the hundred claimed, and most
of the addition is the four doc comments that carry the two clauses. Near enough that the estimate
was sound; and the re-derivation was still worth its minutes, because it is what showed that
`shading::Cache` already makes a rebuild-per-mark a lookup, and therefore that this design needs no
memo of its own to be affordable.

## Why a type and not a rule

ADR 0483 left an explicit warning: a rebuild at the mark "has to keep reading `PatternInitial` for
the black point, the intent and the smoothness, or it will trade one departure for another". That
is a rule somebody has to keep, and this tree's precedent — `HeldContent` in the five-hundred-and-
ninety-second session, `Ungrounded` in the six-hundred-and-twenty-eighth — is that a type is worth
more than a comment.

So the two functions on the rebuild path take a `&ShadingDefinition`, which carries the `/Shading`
object unresolved, the resource dictionary, the transform and `PatternInitial`, and **neither takes
a `&GraphicsState`**. §10.5's transfer arrives as the one explicit argument, from a caller that has
read `GraphicsState::transfer`; §11.7.2's target arrives through `self.compositing`, which is the
raster being painted rather than the state. There is no parameter through which a later round could
pass the state's own smoothness, and the wrong version of this code does not compile rather than
failing a test that might not exist.

`MarkColouring` is the pair those two supply, and its equality is by hand: `Conversion` compares by
value and the transfer by `Arc::ptr_eq`. That is an over-approximation in the direction that costs
a *rebuild* rather than a wrong colour — a stream restating one `/ExtGState` twice parses two
`Transfer`s that are equal and not identical, and rebuilding colours that were already right is a
lookup in a table keyed by everything else.

## What fell out, and what deliberately did not

**`group_press`'s fifth condition is gone.** ADR 0483 refused a subtractive pair for a group a
shading pattern was carried into, because "a colour resolved outside cannot be reinterpreted as ink
inside", and wrote that the refusal would end with this rebuild. It has: the mark inside the group
rebuilds the pattern's colours in whichever half of the press the run is compositing into, like any
other colour there. The test that held the refusal now holds the opposite and one thing more — the
same page with the `scn` outside the `Do` and inside it draws the same pixel, which is §11.6.7's
split stated as an assertion.

**§8.6.8's uncoloured condition stays, and the difference is which way the colour travels.** An
uncoloured tiling cell "shall not specify any colour", so its colour comes from the `scn` outside as
a resolved `pdf_render::Color`; there is no definition inside the group to rebuild from, and
reinterpreting that value as ink would state an ink nobody did. §11.7.5.3's black-generation
condition stays for its own unrelated reason. Both were re-read rather than assumed still right.

**`Painted::Shading`'s `stale` flag is gone**, with the `Unsupported::TransferFunction` report that
§10.5 raised through it. There is no stale state left to name: every mark's transfer function is the
one its own graphics state states. §11.7.5.2's *other* report — the per-region model — is untouched
and its condition is now simply `state.transfer.is_some()`.

**What is not in scope here and is unchanged**: this tree applies a transfer function per object
before compositing, where §11.7.5.2 applies the topmost object's to the composited colour. A shading
pattern rebuilt inside a press is transferred the same way every other colour in that press already
was. That departure is §11.7.5.2's row and `doc/todo/13`'s remaining section, and this change
neither widens nor narrows it.

## The population, and what moved

`examples/pattern_state_census` gained the fourth condition this change turns on: a document holding
both a `/PatternType 2` object and a Table 57 `/TR` or `/TR2` stating a real function — neither
`/Identity` nor `/Default`, which state none. It gained a witness list too, one name per line, so
that a digest can be run over exactly what a count matched.

- **`doc/pdf.js`**: 964 open, 38 hold a Type 2 pattern (601 of them), 0 state Table 75's
  `/ExtGState`, 0 can see the black point move, 2 hold a four-component group `/CS`, **0 state a
  real transfer function**.
- **The crawl**: 65 703 open, 1504 hold one (36 527 patterns), 42 state an `/ExtGState`, 0 can see
  the black point move, 211 hold a four-component group `/CS`, **11 state a real transfer
  function**. Every figure but the last is 655's, re-derived and identical.

`examples/raster_digest` on both arms — the tree as it ships and `HEAD` — over the 974 corpus first
pages and over the 221 crawled documents the census named: **byte-identical on both**. So the
argument for this change is the clause, and the corpus is evidence that nothing else moved rather
than evidence that anything was broken. The honest limit is the instrument's: `raster_digest` draws
*first pages*, so a witness whose pattern is on page nine is outside what this measurement covers.

## The tests, each run against the defect it guards

- **`a_pattern_is_painted_under_the_transfer_function_the_mark_states`** reads the ramp both ways
  round — selected under an inverting function and painted with none, and the reverse — and asserts
  the mark's answer. With `shading_paint` made never to rebuild, it fails.
- **`a_rebuilt_patterns_black_point_is_still_its_definitions`** is the warning's test. It forces a
  rebuild without changing a colour by any other route: the `gs` at the mark states §7.10.3's
  identity *written as a function*, which Table 57 makes a stated function rather than the
  `/Identity` name that clears one, while the pattern's own `/ExtGState` says `/UseBlackPtComp
  /OFF`. With only the rebuild's colouring mutated to read a compensating default, it fails — and
  so do ADR 0483's two, which is the right shape: the three are one question asked at three moments.
- **`a_shading_pattern_carried_into_a_press_is_rebuilt_in_the_groups_space`** fails with ADR 0483's
  condition put back on `group_press`.

## A finding about the instruments, not about the code

`cargo nextest run -p pdf-model` alone fails six CCITT tests on `HEAD` with black and white
exchanged; the same tests pass under `cargo nextest run --workspace`, which is what
`doc/todo/02` §2 runs and what CI runs. It is Cargo feature unification: the package-scoped build
resolves `hayro-ccitt`'s features differently from the workspace-scoped one. Recorded because a
round that narrows the test command to save time will meet six red tests that are not red, and lose
an hour deciding whether it broke them.
