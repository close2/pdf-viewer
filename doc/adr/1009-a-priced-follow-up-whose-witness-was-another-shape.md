# ADR 1009 — A priced follow-up whose witness was another shape, and a report about a mode nothing carried

Status: accepted, 2026-09-12. Session 988.

## Context

Clause 11, continued from ADR 1000. That round built §11.7.4.4's three constructions for the
*implicit* knockout groups — a `B`'s two portions, a text object's glyphs — and in its §7 priced
one follow-up without taking it:

> a *form* non-isolated knockout group whose elements share an affine mode (`issue18032.pdf`)
> could take route 2 and return to three backends.

It also left four `partial` rows of the clause read and ranked, each with zero corpus witnesses:
§11.4.4, §11.3.7.2, §11.7.5.3 and §11.7.2. This round was given the follow-up and three
questions — where a single alpha per pixel is the wrong answer (§11.3.7.2), and *when* a group's
colour conversion happens (§11.7.2, §11.7.5.3).

## 1. The witness was not the shape the pricing named

ADR 1000's route 2 moves a blend mode from a knockout group's elements to the group's `Do`, and
its derivation (`blend_at_the_do`) holds where the mode is affine in its source or every element
is one colour. The pricing said `issue18032.pdf` was such a group. Opening the file (trap 1
applies to a *display list* as much as to a raster): the group is form 101, `/I false /K true`,
whose two elements are

- `/Fm0`, a nested transparency group holding one `sh` shading, invoked under `/BM /Color`;
- `/Fm1`, a nested transparency group holding one magenta fill, invoked under `ca 0`.

`Color` is §11.3.5.3's, non-separable and not affine; the shading is not one colour; both
elements are nested groups, which `blend_at_the_do` refused outright ("[e]very element has to be
elementary"). By the condition as written the page could not take route 2 at all. What the page
draws is the shading blended with the page under `Color` and, inside the magenta fill's outline, a
hole showing the page — the second element's *shape* is 1 inside its path and its opacity 0, and
§11.4.6's NOTE 5 says what that does: "[a] shape value of 1.0 (inside) yields the colour and
opacity that result from compositing the object with the initial backdrop", which at opacity 0 is
the backdrop itself.

## 2. The derivation read for what it needs

The equality route 2 rests on is, per pixel,

```text
α_K × M(C_b, K ⁄ α_K) = Σ wᵢ × M(C_b, Cᵢ),        K = Σ wᵢ Cᵢ,  α_K = Σ wᵢ
```

with `wᵢ` the weight §11.4.6's second stage leaves element `i`. ADR 1000 held it by two
sufficient conditions on the *elements*. It holds under a weaker one on the *terms*:

- **A term with `wᵢ = 0` everywhere is not in the sum.** §11.6.4.4's constant is a factor of the
  opacity (or, under `/AIS true`, of the shape), so an element whose constant is 0 contributes no
  colour whatever its mode; under the opacity reading it still knocks out, because its shape is
  its geometry. `carries_colour` is that test: a solid paint's alpha, an image's or a nested
  group's constant, each a *constant* of zero; a soft mask is not (it is zero only where it is
  zero) and a shading's constant is folded into its colours and is read as carrying colour.
- **With one term left, `K ⁄ α_K` is that element's colour at every pixel**, and the equality is
  an identity for every `M`, §11.3.5.3's non-separable modes included — `M` sees exactly `C₁`.
- **A nested group may be a term where it is isolated.** Its elements draw on §11.4.5's
  transparency under either construction, so its colour at a pixel is the same `Cᵢ` in both, and
  the mode this move takes off it is the one at its own `Do`. A non-isolated one is refused: under
  §11.4.6's own backdrop it is seeded from that backdrop and on transparency from nothing.

And for a *form's* group the `Do` carries a constant alpha `w_g` and a soft mask that an implicit
group never has. ADR 1000 §7 said the collapse survives them — `(1 − w_g α_K) B + w_g × (…)` on
both sides — and `the_mode_at_the_do_survives_a_constant_alpha_at_the_do` holds the two
constructions to one raster at `ca ½` under `Difference`, with the three levels computed from the
clause: `(255, 0, 128)` where the coloured part is alone, `(255, 0, 0)` in the hole, `(255, 0, 64)`
on the half-covered column. The mode at that `Do` has to be Normal, as it already had to be for
ADR 0327's construction, because the cancellation is the Normal blend function's.

`issue18032.pdf`'s group takes the construction on exactly the first two bullets — one coloured
element, so `Color` moves — and the third admits its nested groups.

## 3. The decision

`transparency::knockout_construction` is §11.4.6's three constructions for a form `XObject`'s
knockout group, extracted from `run_transparency_group` and tried in the order
`implicit_knockout_group` tries them: transparency; the mode at the `Do`; the group's own
backdrop. The second is tried before the third because every backend draws it and two refuse the
third. `blend_at_the_do`, `without_blend` and `one_solid_colour` are widened as §2 says, and the
widening reaches the implicit callers too, harmlessly: a `B` never has a zero-alpha part (its
caller requires both parts to mark) and a text object's glyphs are one colour.

One condition the pricing did not name is on the *report* rather than the construction: the mode
moves only where nothing in the stripped tree still blends, because `note_group_structure` reads a
group's whole tree and would name an inner mode as blending with the backdrop this group excludes.
A nested isolated group's inner elements blend against its own transparency under either
construction, so this costs a construction nothing and keeps the report exact.

## 4. What it measures

- `issue18032.pdf` at scale 2, on the oracle: **0 of 1 938 816 pixels changed** — the two
  constructions are one picture, which is what §2's equality says. The page's display list holds
  `Group { blend: Color, isolated: true, knockout: true }` where it held `isolated: false`.
- Every backend draws that command. `render-raster`'s corpus gate holds the names its translation
  refuses to equality, and `issue18032.pdf` is on it for the construction it no longer takes; the
  list is `crates/render-raster/tests/corpus.rs::REFUSED_BEFORE_THE_SCENE`, that crate is the colour
  round's file this session, and the edit owed is the one name and its count. The gate's run:
  `958 pages compared: 931 agree, 21 differ, 6 refused` — the page **agrees with the CPU oracle**
  through raster, and the gate is red only because the name is still on the list.
- Corpus gate: `61 incomplete`, unchanged; oracle: `agrees 990, contradicted 62, ambiguous 835`,
  unchanged, `issue18032.pdf` listed nowhere. The lines are in `doc/history/988-*.md`.

## 5. A report about a mode nothing carried

With the form taking route 2, `issue18032.pdf` reported "non-isolated, and an element blends with
the backdrop it excludes" — a group whose every element had just been stripped of its mode.
`command_blends` matched `Fill`, `Stroke`, `Image` and `Group` and fell to `_ => true` for
everything else, and a `Command::Shaped` is everything else. The arm was written when `Shaped` did
not exist, with the honest reason that an unknown command should count as blending because the
callers decide whether to *report*. `Shaped` arrived (ADR 0234) and the arm never learned it, so a
knockout group drawn on transparency with a stated element was reported as blending whenever
Table 145 said `/I false`. No corpus page carried the report — every corpus knockout group with a
stated element is `/I true` — and the run of the transparency fixtures either side shows nothing
moving. A stated element blends as its object does now.

## 6. §11.3.7.2: where the product is wrong, and that nothing reachable is drawn with it there

The row asked for the reachable case where one alpha per pixel is the wrong answer, or a proof
there is none. The proof is short and it is the clause's own. §11.3.7.1 makes alpha the product of
shape and opacity; every formula of §11.3.3 and §11.3.6 reads `α_s` alone; §11.3.7.3's result
opacity `q_r = α_r ⁄ f_r` re-enters the next composite as `α_r` unless that composite is §11.4.6's;
§11.5.3 composites a mask's group by those formulas and reads a luminosity of the result. The one
reader of a shape apart from an opacity is §11.4.6 — "[t]he existence of the knockout feature is the
main reason for maintaining a separate shape value rather than only a single alpha that combines
shape and opacity" — and the fixture that shows the product wrong is built from it and has been in
the tree since ADR 0234: a masked element over an opaque one inside a knockout group knocks the lower
one out entirely, `(127, 127, 255)`, where the product read as shape composites over it at a half,
`(127, 0, 127)`. There this tree does not carry the product: the shape is stated beside the element
or the element is reported by name. So the row's "shape channel every command carries" is not owed
by this clause — nothing but §11.4.6 would read it, and §11.4.6 reads a stated one — and what keeps
the row `partial` is the two elements reported rather than drawn: an image whose samples may be a
stencil (shape) or an `/SMask` (opacity), and a shading whose colours carry the constant. Both need
the *kind* of the alpha carried beside its value in `crate::image` and `crate::shading`, which are
the colour round's files; no corpus page states either inside a knockout group.

## 7. §11.7.2 and §11.7.5.3: the moments are the clause's, the parameter at one of them is not

The clause fixes two moments. The conversion *in* is at the painting operation ("all painting
operators shall convert source colours … to the group colour space before compositing objects into
the group"), and `Interpreter::colour` performs it there, per stated colour, under that operation's
graphics state. The conversion *out* is at the group's `Do`, after every composite inside ("[t]he
resulting colours shall then be interpreted in the group's colour space when the group is
subsequently composited with its backdrop"), and every backend that draws a `GroupBlending` resolves
it over the group's finished raster before painting it onto the parent.
`a_group_that_introduces_a_press_composites_in_it` is the test of the moment: paper and
registration black at `ca ½` composite to `(76.0, 66.1, 63.9)`, and converting each colour first
gives 127.5. A non-isolated group has no moment of its own and is inherited past.

What is not read at the second moment is the *parameter* §11.7.5.3's second bullet names, not
the moment: a press's conversion out is sampled once from the profile's `A2B1`-else-`A2B0` with
black point compensation on (`colour::sample_press`), so the `/RI` and `/UseBlackPtComp` in force
at the `Do` do not select it — the same standing `A2B` has everywhere in this tree, recorded in
§8.6.5.8's row — and a colour that went in by the inverse of the same grid comes back whatever
they say. That is `crate::colour`'s. And one group has no moment where it should: a knockout group
naming a four-component space keeps its report (`group_press`'s last condition), because §11.4.6's
rewrite of the elements would have to be applied to both halves of the pair; the halves are
`paired` and the rewrite is a pure function of the list, so it can be, and no corpus document
states the combination.

## 8. What this leaves

- `render-raster`'s `REFUSED_BEFORE_THE_SCENE`: remove `issue18032.pdf`, `[&str; 4]`, and the
  doc comment's sentence that names it for ADR 0327's construction. The colour round's file.
- The own-backdrop construction on two backends, for coloured elements under two modes or under
  one that is neither affine nor applied to one colour. Unchanged from ADR 1000 §7.
- A knockout group with a four-component `/CS` of its own: apply `knockout_construction` to both
  halves of the pair in `run_transparency_group`, and drop `!group.knockout` from `group_press`.
- The kind of an image's and a shading's alpha, one bit beside the value, in the colour round's
  files; it closes §11.3.7.2's last two reports and §11.4.6's "an image whose samples state either
  shape or opacity".

## 9. Two habits, for the files that hold them

**A priced follow-up names a population by the shape of its argument, and the witness can be
another shape.** ADR 1000 §7 wrote "whose elements share an affine mode" because that was route
2's condition, and the one page it named shared no affine mode and had no elementary elements.
Before taking a priced follow-up, open the witness's display list and check it against the
condition — the derivation may hold for a reason the pricing did not state, and it may not hold at
all. The cost of not looking would have been a construction taken on the strength of a page it
cannot reach.

**A non-exhaustive `_ => true` arm is a report that fires on every variant added after it.**
`command_blends`'s fallback was right about unknown commands and wrong about a known one that
arrived later, and nothing in the tree could see it: the report it raised was a true sentence about
a mode that was not there. When a `Command` variant is added, grep the `match`es on `Command` for
`_ =>` arms and read what each one now says about the new variant.
