# Transparency departures

Status: each reported where it can change a pixel. **§11.5.3's population is closed** — its device
branch was taken in the three-hundred-and-eightieth session (ADR 0217) and both residues in the
three-hundred-and-eighty-third (ADR 0220) — **§11.4.6's shape is closed in the
three-hundred-and-ninety-seventh** (ADR 0234), and **§11.4.4's non-isolated group is drawn since
the four-hundredth** (ADR 0237). One population stands, plus the residues inside the closed ones.
Priority: 23
Corpus: 6 documents
Clauses: §11.4.4, §11.4.6, §11.6.6
Code: `crates/pdf-model/src/content.rs`, `crates/pdf-render/src/display_list.rs`,
`crates/render-cpu/src/blend.rs`

| | corpus | what it is |
|---|---|---|
| a blending space that is not the device's three components, for a **painted** group | 4 | all `/DeviceCMYK`, and **still owed** |
| ~~a non-isolated group NOTE 5 cannot flatten~~ | ~~6~~ → 3 | **the non-knockout ones closed** in the 400th, ADR 0237; what is left is knockout, below |
| ~~a soft-mask group with such a space~~ | ~~7~~ → 0 | **closed** in the 380th and 383rd, ADRs 0217 and 0220 |
| ~~a knockout element whose shape is not its coverage~~ | ~~5~~ → 0 | **closed** in the 397th, ADR 0234 |

Each remaining one is refused *by name* rather than approximated, which is the rule that keeps the
corpus count honest. The six documents are `bug1721218_reduced`, `issue14200`, `issue18032` and
`bug1755507` for the blending space, and `issue18032`, `knockout_blend_multiply` and
`knockout_inner_backdrop` for what §11.4.4 still refuses — two lists overlapping on one file.

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

## The one that stands

**There is a precedent for the display-list question**, and there are now four: `ImageSource`
carries a raster the list *names* (ADR 0210), a mask group is painted in the one quantity §11.5.3
composites (ADR 0220), a knockout element states its shape beside its colour (ADR 0234), and a
group names the backdrop its elements composite onto (ADR 0237). What the four have in common is
that the missing quantity turned out to be sayable in a command — three times as a second command
or a second raster, and once as a flag over an identity nobody had derived. The one below is not.

- **§11.6.6's blending space for a painted group** (4 documents) wants the group's raster in its
  own components: a painted group's result is three components at every pixel, so the linearity
  that made a mask group one channel is a property of the *reduction* to luminosity and a painted
  group is not reduced. A second raster format, in three backends, with images and shadings inside
  it. ADRs 0217 and 0220 both say so and neither bears on it.

**And the lesson ADR 0237 leaves for it is a warning rather than a method.** "A raster cannot hold
this" survived three hundred sessions and two ADRs as a *claim about the clause* that nobody had
checked by writing the clause's formulas down and running them. Before this population is priced
again, transcribe §11.6.6 and §11.3.3 the same way and find out what actually cancels.
