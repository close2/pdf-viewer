# Transparency departures

Status: each reported where it can change a pixel. **§11.5.3's population is closed** — its device
branch was taken in the three-hundred-and-eightieth session (ADR 0217) and both residues in the
three-hundred-and-eighty-third (ADR 0220) — **§11.4.6's shape is closed in the
three-hundred-and-ninety-seventh** (ADR 0234), and **§11.4.4's non-isolated group is drawn since
the four-hundredth** (ADR 0237). The four-hundred-and-fifteenth found the standing population was
the wrong one and priced what is left of it (ADR 0251); **the four-hundred-and-twenty-sixth built
what it priced and found the price was for the wrong half** (ADR 0262).
Priority: 23
Corpus: 8 documents
Clauses: §11.4.4, §11.4.7, §11.6.6, §11.7.2, §11.7.5.3
Code: `crates/pdf-model/src/content.rs`, `crates/pdf-model/src/colour.rs`,
`crates/pdf-render/src/blending.rs`, `crates/render-cpu/src/lib.rs`

| | corpus | of 69 web witnesses | what it is |
|---|---|---|---|
| **a conversion *into* the blending space** (§11.7.2) | 5 | 61 | **the standing item.** §10.4.2.4 is not the inverse of this tree's conversion out |
| a four-component `ICCBased` blending space | 0 | 6 | its conversion out is a profile rather than sixteen corners |
| a group inside the page composites in a different space (§11.6.6) | 1 | 3 | needs a conversion between two spaces at the `Do` |
| a non-separable blend mode on such a page (§11.3.5.3) | 1 | 2 | the black component has a rule of its own |
| an `/ExtGState` states `/BG`, `/BG2`, `/UCR` or `/UCR2` (§11.7.5.3) | 0 | 1 | **was silent until the 426th** |
| ~~the four components themselves~~ | — | — | **closed in the 426th, ADR 0262: two rasters, no new format** |
| ~~a non-isolated group NOTE 5 cannot flatten~~ | ~~6~~ → 3 | | **the non-knockout ones closed** in the 400th, ADR 0237; what is left is knockout, below |
| ~~a soft-mask group with such a space~~ | ~~7~~ → 0 | | **closed** in the 380th and 383rd, ADRs 0217 and 0220 |
| ~~a knockout element whose shape is not its coverage~~ | ~~5~~ → 0 | | **closed** in the 397th, ADR 0234 |

Each remaining one is refused *by name* rather than approximated, and since the four-hundred-and-
twenty-sixth the name says **which** of the five conditions fired. The eight documents are
`bug1703683_page2_reduced`, `bug1721218_reduced`, `bug1755507`, `issue12798_page1_reduced`,
`issue13520` and `issue18032` for the blending space, and `issue18032`,
`knockout_blend_multiply` and `knockout_inner_backdrop` for what §11.4.4 still refuses — two lists
overlapping on one file. `personwithdog.pdf` left in the four-hundred-and-twenty-sixth, which drew
it.

## The four components were two rasters, and that is closed

**§11.3.4 applies the compositing formula per component** — "[t]he i th component of the result
colour 𝐶𝑟 shall be obtained by applying the compositing formula to the i th components of the
constituent colours" — so a rasteriser with three channels composites four by drawing the page
twice with a different three loaded. `Compositing::Subtractive(Half)` is which three; both halves
carry §11.3.4's additive complements, so the blend functions see what that clause requires without
anything being complemented around them; `pdf_render::BlendingSpace` carries the conversion out as
the ink cube's sixteen corners and `blending::resolve` applies it where §11.4.7 does, before the
medium. `render-gpu` and `render-quorra` refuse the list (`QUORRA_FEEDBACK.md` section 17).

**ADR 0251's "second raster format" is therefore withdrawn as a requirement.** It was a true
statement about arithmetic — the ink cube is affine on no face of the cube, 48 of 255 at worst —
attached to a wrong statement about what carrying four components costs.

## What actually blocks the population, and it is a colour conversion

§11.7.2's second sentence, which this tree had read only for its inheritance rule:

> If the colour space of a graphics object within the group is not equivalent to the group's
> blending colour space, then it shall be converted to the group's colour space , and all blending
> and compositing computations shall be done in that space

§11.7.5.3 names §10.4.2.4 as that conversion, and §10.4.2.1 packages §10.4.2.2 through §10.4.2.5 as
what a processor uses **instead of** §10.3 — the branch ADRs 0009 and 0042 put this tree's
conversion *out* of `DeviceCMYK` on. The standard's own pair round-trips exactly; one branch
composed with the other does not, and ADR 0262 has the picture: a webinar flyer whose green panel
comes back grey-green and whose `0 g` text comes back the process black `#231F20`, neither of them
a mark that composites with anything.

**So a page is drawn in its blending space only where every colour painted into it is already in
that space**, and 61 of the 69 web witnesses are not. What closes them is a conversion into
`DeviceCMYK` on §10.3's branch — a right inverse of the press ADR 0009 assumes, with gamut mapping
where no preimage exists, which is a colour-management round of its own and not a transparency one.
Two measurements to start it from, both taken in the four-hundred-and-twenty-sixth: exempting
`DeviceGray` alone would take the 62 remaining reports to **43**, and taking §10.4.2.4 as written
would take them to **10** at the cost of every non-`DeviceCMYK` colour on those pages.

## How the population was found: the blending space was the wrong four documents

**This file said "4 documents, all `/DeviceCMYK`" for eighteen sessions and three of the four were
reported for the wrong reason or for no reason at all.** §11.6.6 gives a group's `/CS` effect "[f]or
isolated groups" and then hands every other case to the parent — "[f]or non-isolated groups, or if
no group colour space is specified, the group colour space shall be inherited from the parent group
or page" — and §11.4.7 puts the *page group* under that inheritance: its `/CS` "shall serve as the
default blending colour space for each page", and "[a]ll page-level compositing shall be done in the
default blending colour space of the page".

So `issue14200.pdf` was reported for a `/DeviceCMYK` on a group that states no `/I`, on a page that
states no `/Group` at all — nothing on it composites anywhere but the device's components, and the
report has gone. And five documents were departing in silence, because nothing in this tree read
§11.4.7's entry: `bug1365930`, `bug1703683_page2_reduced`, `issue12798_page1_reduced`, `issue13520`
and `personwithdog` all state a page group of `/DeviceCMYK`, so **every mark on those pages
composites in ink**. Four of them report it now; `bug1365930` does not, because nothing on its first
page composites and the space cannot change a pixel there.

`crates/pdf-model/examples/group_space_census.rs` is what says this, and the thing that made it say
anything is printing the *effective* space beside the declared one. 115 of the 974 documents state a
page group `/CS`; 7 of those name a space that is not the device's three components; 71 group
dictionaries declare `/DeviceCMYK` and 96 groups actually composite in it.

## How it was priced in the four-hundred-and-fifteenth, and what survived that pricing

**A second raster format is genuinely required, and ADR 0217 gave the wrong reason for it.** The
reason was "a painted group's result is three components"; the number of components has nothing to
do with it. §11.3.3 under `Normal` is a weighted average — §11.3.6: "the compositing formula
collapses to a simple weighted average of the backdrop and source colours" — and a convex
combination passes through an **affine** map unchanged. So the only question is whether the
conversion out of the blending space is affine over the colours the group composites.

Measured per component over 200 000–300 000 random pairs (ADR 0251):

| the conversion | worst gap between the two orders of operation |
|---|---|
| §10.4.2.5's classic `1 − min(1, c + k)`, no channel over one unit of ink | **3.3 × 10⁻¹⁶** |
| the same, with the clamp reached | 117 of 255 |
| the same, with the clamp **deferred** onto three unclamped components | **3.4 × 10⁻¹⁶** |
| **this tree's multilinear interpolation of the ink cube** (ADRs 0009, 0042) | **48 of 255** |

Under the standard's own classic formula the collapse is exact, and the clamp is deferrable by
ADR 0220's trick one clause over. Under the conversion this project chose it does not collapse at
all, because multilinear interpolation carries products of the four inks. Half of registration black
over paper is `[76.0, 66.1, 63.9]` in `DeviceCMYK` against `[127.5, 127.5, 127.5]` on the device —
**51.5 of 255**, and `compositing_in_cmyk_is_not_compositing_in_the_device_and_this_is_the_gap` pins
it and its control.

**ADR 0251 concluded from this that a four-component raster was owed**, and the arithmetic above
is right while that conclusion is not: §11.3.4's per-component formula makes four components two
rasters, which the four-hundred-and-twenty-sixth built. What the arithmetic still decides is that
compositing in ink is a *different picture* and worth having — 51.5 of 255 at the fixture, +0.100
of 255 over the whole of `personwithdog.pdf` — and that §10.4.2.5's classic conversion is not the
way to get it, because it is 115 of 255 out at the cube's corners.

## The non-isolated group, and why it fell

**A second accumulator the clause divides out again.** §11.4.4's NOTE 4 advises keeping Table
140's group alpha apart from the composite alpha, because NOTE 3's backdrop removal divides by
the first; this file, ADR 0234 and the ledger all concluded that one premultiplied raster
therefore cannot do it. The quantity the removal divides out is **multiplied straight back in**
when §11.3.3 composites the group's result onto the same backdrop, so with the Normal blend
function at the `Do` the pair collapses — exactly, for every backdrop alpha and every blend mode
inside the group — to

```text
result = (1 − w) × backdrop + w × (elements composited onto the backdrop)
```

with `w` the group's constant alpha times its soft mask. `w = 1` is NOTE 5's flattening. So the
display list gained one flag (`Command::Group`'s `isolated`), `render-cpu` seeds the group's
buffer from the surface and writes that line in one pass, and two corpus documents lost their
only report. ADR 0237 has the derivation, the 200 000-case check against the clause's own
formulas, and the three fixtures.

### What that left behind

1. **A knockout group whose elements blend** — the same sentence one clause over. §11.4.6
   composites each element with the group's *initial* backdrop, which for a non-isolated knockout
   group is the page, so the two stages are not the pair `Command::Shaped` states. Three corpus
   documents: `issue18032.pdf` and `knockout_blend_multiply.pdf` state one, and
   `knockout_inner_backdrop.pdf` is a non-isolated group *inside* a knockout one, which is a
   third condition rather than the second.
2. **A blend mode at the `Do`**, where the collapse genuinely fails and NOTE 4's second
   accumulator would genuinely be needed — 0.601 of full scale wrong if it is assumed anyway. No
   corpus document states one.
3. **`render-gpu` and `render-quorra` refuse the command**, because a Vello layer and a
   `quorra_scene::GroupSpec` both begin transparent and neither can be seeded from the surface.
   Four corpus pages moved from `agree` to `refused` in the quorra gate and every one of them is a
   page quorra used to draw with the wrong initial backdrop; `doc/QUORRA_FEEDBACK.md` §16 asks
   for the flag. On the GPU side the frame goes to the CPU backend, which is what `CLAUDE.md`
   keeps that backend for.

## The knockout shape, and why it fell

**A shape is a second quantity a *command* can state, where the other two are second quantities a
*buffer* has to hold.** §11.6.4.2 gives an object's shape from its geometry alone; §11.6.4.3's soft
mask and §11.6.4.4's constant are opacity. So `pdf_render::Command::Shaped` carries the object
beside a second command — the object with those two removed — whose drawn alpha *is* the shape, and
a group's shape is the union of its elements'. §11.4.6's two stages then come to
`P' = (1 − f) × P + S` in premultiplied form, which both backends draw as Destination-Out with the
shape and then **Plus** with the object. ADR 0234, and its third fixture is the one that pins the
Plus: source-over there is 32 of 255 out at a half-covered pixel under a half-opaque mark.

### What that left behind, each reported by name and each with no corpus witness

1. **An element whose one alpha carries both quantities in a raster.** An image's samples may be
   §8.9.6.2's stencil (shape) or §11.6.5.2's `/SMask` (opacity), and a shading's colours already
   carry §11.6.4.4's constant, so neither can be un-multiplied after the fact. An `ImageSource`
   that keeps the two apart would answer both, and it is a smaller construction than the
   population below.
2. **§11.6.4.3's `/AIS`.** It inverts which of the two the mask and the constants are, so the
   shape a `Shaped` states is exactly wrong while it is set — and a knockout group drawn by
   modulating Source with coverage was *already* wrong under it, silently, since the seventy-first
   session. The entry is read now and every knockout group is refused while it is set. **Nine
   corpus documents state it true** (the ledger row said none did), and none of their knockout
   groups is drawn today. Honouring it means composing the mask and the constants into the shape
   instead of into the object, which is a second `stated_shape` rather than a new vocabulary.
3. **`render-quorra` refuses a `Shaped` element outright**, because `quorra_scene::Compose` has
   source-over and coverage-modulated source and neither writes `(1 − f) × P + S`.
   `doc/QUORRA_FEEDBACK.md` §14 asks for Destination-Out and Plus, and it is **still open**.
4. **`render-gpu`'s coverage path keeps its documented residue**: where the shape *is* the coverage
   it still draws the element with source-over after the Destination-Out, which weights the
   backdrop by `1 − f × opacity` a second time. Bounded and stated in `knock_out`'s own comment
   since the seventy-first session. Removing it means a Plus layer per element and the elements are
   §9.3.8's glyphs, so it wants a measurement before it is paid.

## What the four precedents have in common, and why this one is not among them

`ImageSource` carries a raster the list *names* (ADR 0210), a mask group is painted in the one
quantity §11.5.3 composites (ADR 0220), a knockout element states its shape beside its colour
(ADR 0234), and a group names the backdrop its elements composite onto (ADR 0237). In each the
missing quantity turned out to be sayable in a command — three times as a second command or a second
raster, once as a flag over an identity nobody had derived. **This one is among them after all**: a
page's four components are a second *list*, which is the same shape a fifth time, and the paragraph
that used to stand here said they were a format and were not sayable. What is not sayable in a
command is the one thing left — a colour conversion, which is not a quantity at all.
