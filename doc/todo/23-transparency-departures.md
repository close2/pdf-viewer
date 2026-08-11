# Transparency departures

Status: each reported where it can change a pixel. **§11.5.3's population is closed** — its device
branch was taken in the three-hundred-and-eightieth session (ADR 0217) and both residues in the
three-hundred-and-eighty-third (ADR 0220) — **§11.4.6's shape is closed in the
three-hundred-and-ninety-seventh** (ADR 0234), and **§11.4.4's non-isolated group is drawn since
the four-hundredth** (ADR 0237). The four-hundred-and-fifteenth found the standing population was
the wrong one and priced what is left of it (ADR 0251); **the four-hundred-and-twenty-sixth built
what it priced and found the price was for the wrong half** (ADR 0262); **the
four-hundred-and-twenty-seventh built that other half and closed the standing item** (ADR 0263);
**the four-hundred-and-thirty-sixth made the press the document's** and closed the largest
condition the web has (ADR 0272). **The four-hundred-and-thirty-eighth took the first of the two
backend rows off this file** (ADR 0274): `render-quorra` draws §11.4.4's non-isolated group, and
the two rows still on it — §11.4.6's stated shape and §11.4.7's two rasters — stopped being
requests to somebody else and became work here, because quorra answered both asks at `89d7dd77`.
Priority: 23
Corpus: 6 documents
Clauses: §11.3.5.3, §11.4.4, §11.6.6, §11.7.5.3, §8.6.5.5, §8.6.5.6, §8.6.5.7, §11.7.2, §14.11.5
Code: `crates/pdf-model/src/content.rs`, `crates/pdf-model/src/colour.rs`,
`crates/pdf-render/src/blending.rs`, `crates/render-cpu/src/lib.rs`

| | corpus | web witnesses | what it is |
|---|---|---|---|
| a group inside the page composites in a different space (§11.6.6) | 3 | 3 of 1896, 4 of 4000, 78 → **85 of 65 944** | **the standing item now.** Needs a conversion between two spaces at the `Do`. A further **30** are a group that *introduces* the space rather than the page group doing so |
| a page group whose components are not four this tree can sample | 0 | 14 of 4000, 106 → **5 of 65 944** | what is left after ADR 0272: a `/DeviceGray` or `Lab` page group, or four components with no profile behind them, so §11.3.4 has no formula to apply and no conversion out |
| a non-separable blend mode on such a page (§11.3.5.3) | 1 | 1 of 1896, 2 of 4000, 27 → **28 of 65 944** | the black component has a rule of its own |
| an `/ExtGState` states `/BG`, `/BG2`, `/UCR` or `/UCR2` (§11.7.5.3) | 0 | 1 of 1896, 0 of 4000, 7 → **9 of 65 944** | **was silent until the 426th**, and 0 of 4000 could have been read as noise |
| ~~the document names the press its `DeviceCMYK` is~~ | ~~0~~ | ~~151~~ → **0** | **closed in the 436th, ADR 0272: the press is a value, and `CMYK_CORNERS` is one of them** |
| ~~a conversion *into* the blending space~~ | ~~5~~ → 0 | ~~61~~ → 0 | **closed in the 427th, ADR 0263: a right inverse of the ink cube** |
| ~~the four components themselves~~ | — | — | **closed in the 426th, ADR 0262: two rasters, no new format** |
| ~~a non-isolated group NOTE 5 cannot flatten~~ | ~~6~~ → 3 | | **the non-knockout ones closed** in the 400th, ADR 0237; what is left is knockout, below |
| ~~a soft-mask group with such a space~~ | ~~7~~ → 0 | | **closed** in the 380th and 383rd, ADRs 0217 and 0220 |
| ~~a knockout element whose shape is not its coverage~~ | ~~5~~ → 0 | | **closed** in the 397th, ADR 0234 |

The three rows that grew by one or two are documents that were reported for the press and are now
reported for the next condition they meet — the population narrowing honestly rather than a
condition being narrowed (trap 5).

Each remaining one is refused *by name* rather than approximated, and since the four-hundred-and-
twenty-sixth the name says **which** of the conditions fired. The corpus documents are
`bug1703683_page2_reduced`, `bug1755507` and `issue13520` for §11.6.6, `issue18032` for the
non-separable blend, and `issue18032`, `knockout_blend_multiply` and `knockout_inner_backdrop` for
what §11.4.4 still refuses. `personwithdog.pdf` left in the four-hundred-and-twenty-sixth and
`issue12798_page1_reduced.pdf` and `bug1365930.pdf` in the four-hundred-and-twenty-seventh, which
drew them.

## The press is the document's, and no ICC dependency was needed

**292 of the 65 703 web documents that open name a press**, and 286 of those name four components:
186 by a page group `/CS` that is a four-component `ICCBased` space (§11.7.2), 94 by §14.11.5's
output intent, 6 by §8.6.5.6's `/DefaultCMYK`. **Every one of those 286 profiles parses with the
`A2B` evaluator ADR 0009 wrote in this tree**, so the round that was set up as a dependency
question was a reading one. `crates/pdf-model/examples/press_census.rs` is the instrument.

Two clauses decide the direction. §8.6.5.5 requires the *file* to carry `B2A` for a blending-space
profile — and all 286 do — but places no requirement on the processor; §14.11.5's Table 401 names
`A2B` for this device outright: "the 'to CIE' (AToB) information may optionally be used to remap
source colour values to some other destination colour space, such as for screen preview or
hardcopy proofing". A screen is what this processor has. The conversion *into* the press is then
ADR 0263's right inverse of the same sampling, so a page has one colour model and no boundary.

**The residue this created is a number and it is the thing to watch.** A backend interpolates a
table, so a press is sampled onto a grid of seventeen per axis; over the 286 profiles that grid
departs from evaluating the profile by a median 5.99 and at most 14.52 of 255. No feasible side
reaches half a level — a v2 CMYK profile puts a steep sampled curve on each ink *before* its own
table, and sampling in linear light is worse rather than better. What closes it is per-axis input
curves beside the grid, which is what an ICC `A2B` tag is, and which the backend would have to be
taught. Against it stands the 48 to 51 of 255 that compositing in *somebody else's* four components
costs (ADR 0251), which is what the round removed.

## The four components were two rasters, and that is closed

**§11.3.4 applies the compositing formula per component** — "[t]he i th component of the result
colour 𝐶𝑟 shall be obtained by applying the compositing formula to the i th components of the
constituent colours" — so a rasteriser with three channels composites four by drawing the page
twice with a different three loaded. `Compositing::Subtractive(Half)` is which three; both halves
carry §11.3.4's additive complements, so the blend functions see what that clause requires without
anything being complemented around them; `pdf_render::BlendingSpace` carries the conversion out as
the ink cube's sixteen corners and `blending::resolve` applies it where §11.4.7 does, before the
medium. `render-gpu` and `render-quorra` refuse the list. **`QUORRA_FEEDBACK.md` §17 is answered**
— two `Target::Readback` renders against one quorra device were always supported, share their
uploaded resources and cost the second pass no geometry at all, and `89d7dd77` added the test that
keeps it so. **So `render-quorra`'s refusal is now work rather than a request**: two
`Rasterizer::rasterize` calls against one `QuorraRasterizer` and `pdf_render::blending`'s
recombination, which `render-cpu` already does. Three corpus pages — `personwithdog.pdf`,
`issue12798_page1_reduced.pdf` and `bug1365930.pdf` — and the 3.5% of the web §17 measured are what
it is worth.

**ADR 0251's "second raster format" is therefore withdrawn as a requirement.** It was a true
statement about arithmetic — the ink cube is affine on no face of the cube, 48 of 255 at worst —
attached to a wrong statement about what carrying four components costs.

## What used to block the population, and what it turned out to be

This section carried §11.7.2's second sentence as the standing blocker for one session:

> If the colour space of a graphics object within the group is not equivalent to the group's
> blending colour space, then it shall be converted to the group's colour space , and all blending
> and compositing computations shall be done in that space

and recorded that §11.7.5.3 "names §10.4.2.4 as that conversion". **It does not.** The bullets that
name the black-generation and undercolour-removal functions are §10.4.2's side of §10.4.2.1's fork;
the paragraph above them chooses a *target* and leaves the algorithm to whichever branch the
processor is on. Reading that paragraph is the whole of the four-hundred-and-twenty-seventh session,
and what it licensed was the third of the three routes this file listed — a right inverse of the
press, with gamut mapping where no preimage exists. The two measurements this section recorded as
"where the next round will be tempted" were both offers to take a shortcut without a clause, and
neither was taken: exempting `DeviceGray` would have put black text at `#231F20`, and taking
§10.4.2.4 as written would have moved every `DeviceCMYK` pixel on every page.

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
3. **`render-gpu` refuses the command**, because a Vello layer begins transparent and cannot be
   seeded from the surface; the frame goes to the CPU backend, which is what `CLAUDE.md` keeps
   that backend for. **`render-quorra` draws it since the four-hundred-and-thirty-eighth** (ADR
   0274): `quorra_scene::GroupSpec` gained Table 145's `/I` at `89d7dd77`, which is exactly what
   `doc/QUORRA_FEEDBACK.md` §16 asked for, and the flag passes straight through. Three of the four
   corpus pages that had moved from `agree` to `refused` went back to `agree` — this time about
   the picture the clause states rather than about the one both backends were substituting — and
   the fourth turned out to be §11.4.7's, which is the row above.

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
3. **`render-quorra` refuses a `Shaped` element outright, and the reason is now this side's.**
   `doc/QUORRA_FEEDBACK.md` §14 asked for Destination-Out and Plus; **both arrived at `89d7dd77`**
   as `Compose::DestOut` and `Compose::Plus` (quorra's ADR 0025), weighted by shape rather than by
   the paint's alpha, which is what a `Shaped` command's second member already carries. What is
   unwritten is the translation in `scene.rs`: two marks per command, the first drawn with every
   source of opacity removed. **It must be written as a pair or not at all** — `Plus` alone
   saturates a premultiplied channel past its alpha, and the library states that as the caller's
   obligation because one mark cannot tell it whether the other is coming. Four corpus pages
   (`knockout_smask`, `knockout_nested`, `knockout_nested_group_alpha`, `knockout_inner_backdrop`)
   and the two positions the builder refuses — a mark carrying a blend mode, a mark inside a
   knockout group — are what a round doing this has to size.
4. **`render-gpu`'s coverage path keeps its documented residue**: where the shape *is* the coverage
   it still draws the element with source-over after the Destination-Out, which weights the
   backdrop by `1 − f × opacity` a second time. Bounded and stated in `knock_out`'s own comment
   since the seventy-first session. Removing it means a Plus layer per element and the elements are
   §9.3.8's glyphs, so it wants a measurement before it is paid.

## What the five precedents have in common, and what the sixth was instead

`ImageSource` carries a raster the list *names* (ADR 0210), a mask group is painted in the one
quantity §11.5.3 composites (ADR 0220), a knockout element states its shape beside its colour
(ADR 0234), a group names the backdrop its elements composite onto (ADR 0237), and a page's four
components are a second *list* beside the first (ADR 0262). In each the missing quantity turned out
to be sayable in a command — three times as a second command or a second raster, twice as a flag or
a field over an identity nobody had derived.

**The sixth was not a quantity at all**, and that is why this paragraph used to end by saying it was
unsayable: a conversion *into* the blending space is a function rather than a value, and no display
list can carry one. What it needed was not a command but a **branch decision** — §11.7.5.3 names the
conversion's target and §10.4.2.1 ranks the algorithms, so the conversion in belongs on whichever
branch the conversion out is on, and a right inverse of the ink cube is what that branch means here
(ADR 0263). The residues left in the table above are the same shape one space over: each of them
needs a *second* colour space this tree does not model, which is again not a quantity a command
could name.
