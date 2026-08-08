# Transparency departures

Status: each reported where it can change a pixel. **§11.5.3's population is closed** — its device
branch was taken in the three-hundred-and-eightieth session (ADR 0217) and both residues in the
three-hundred-and-eighty-third (ADR 0220) — and **§11.4.6's shape is closed in the
three-hundred-and-ninety-seventh** (ADR 0234). Two populations stand, plus two residues inside the
closed one.
Priority: 23
Corpus: 8 documents
Clauses: §11.4.4, §11.6.6
Code: `crates/pdf-model/src/content.rs`, `crates/pdf-render/src/display_list.rs`

| | corpus | what it is |
|---|---|---|
| a non-isolated group NOTE 5 cannot flatten | 6 | §11.4.4's NOTE 5 makes grouping equivalent to not grouping only where no element blends with the backdrop it excludes; these blend |
| a blending space that is not the device's three components, for a **painted** group | 4 | all `/DeviceCMYK`, and **still owed** |
| ~~a soft-mask group with such a space~~ | ~~7~~ → 0 | **closed** in the 380th and 383rd, ADRs 0217 and 0220 |
| ~~a knockout element whose shape is not its coverage~~ | ~~5~~ → 0 | **closed** in the 397th, ADR 0234 — below |

Each remaining one is refused *by name* rather than approximated, which is the rule that keeps the
corpus count honest. The two lists overlap on `issue18032.pdf` and `bug1755507.pdf`, so the eight
documents are: `knockout_blend_multiply`, `knockout_inner_backdrop`, `issue12798_page1_reduced`,
`issue13520`, `issue18032` and `bug1755507` for the first, and `bug1721218_reduced`, `issue14200`,
`issue18032` and `bug1755507` for the second.

## The knockout shape, and why it fell

**A shape is a second quantity a *command* can state, where the other two are second quantities a
*buffer* has to hold.** §11.6.4.2 gives an object's shape from its geometry alone; §11.6.4.3's soft
mask and §11.6.4.4's constant are opacity. So `pdf_render::Command::Shaped` carries the object
beside a second command — the object with those two removed — whose drawn alpha *is* the shape, and
a group's shape is the union of its elements'. §11.4.6's two stages then come to
`P' = (1 − f) × P + S` in premultiplied form, which both backends draw as Destination-Out with the
shape and then **Plus** with the object. ADR 0234, and its third fixture is the one that pins the
Plus: source-over there is 32 of 255 out at a half-covered pixel under a half-opaque mark.

All five corpus witnesses lost this report. Three of them — `knockout_nested.pdf`,
`knockout_nested_group_alpha.pdf` and `knockout_smask.pdf` — now draw with **nothing** reported;
`knockout_inner_backdrop.pdf` and `issue18032.pdf` keep the first row of the table above, which is
what actually blocks their groups. The oracle's contradicted count fell **74 → 69** and its
agreeing count rose **899 → 904**: those three, plus `knockout_groups_test.pdf` pages 2 and 3,
stopped being contradicted and now agree with the reference consensus.

### What that left behind, each reported by name and each with no corpus witness

1. **An element whose one alpha carries both quantities in a raster.** An image's samples may be
   §8.9.6.2's stencil (shape) or §11.6.5.2's `/SMask` (opacity), and a shading's colours already
   carry §11.6.4.4's constant, so neither can be un-multiplied after the fact. An `ImageSource`
   that keeps the two apart would answer both, and it is a smaller construction than either
   population above.
2. **§11.6.4.3's `/AIS`.** It inverts which of the two the mask and the constants are, so the
   shape a `Shaped` states is exactly wrong while it is set — and a knockout group drawn by
   modulating Source with coverage was *already* wrong under it, silently, since the seventy-first
   session. The entry is read now and every knockout group is refused while it is set. **Nine
   corpus documents state it true** (the ledger row said none did), and none of their knockout
   groups is drawn today. Honouring it means composing the mask and the constants into the shape
   instead of into the object, which is a second `stated_shape` rather than a new vocabulary.
3. **`render-quorra` refuses a `Shaped` element outright**, because `quorra_scene::Compose` has
   source-over and coverage-modulated source and neither writes `(1 − f) × P + S`.
   `doc/QUORRA_FEEDBACK.md` §14 asks for Destination-Out and Plus. Four corpus pages moved from
   `agree` to `refused` in that gate and every one of them is a page quorra used to draw wrongly.
4. **`render-gpu`'s coverage path keeps its documented residue**: where the shape *is* the coverage
   it still draws the element with source-over after the Destination-Out, which weights the
   backdrop by `1 − f × opacity` a second time. Bounded and stated in `knock_out`'s own comment
   since the seventy-first session. Removing it means a Plus layer per element and the elements are
   §9.3.8's glyphs, so it wants a measurement before it is paid.

## The two that stand

**There is a precedent for the display-list question**, and there are now three: `ImageSource`
carries a raster the list *names* (ADR 0210), a mask group is painted in the one quantity §11.5.3
composites (ADR 0220), and a knockout element states its shape beside its colour (ADR 0234). What
the three have in common is that the missing quantity turned out to be sayable in a command. The
two below are not.

- **§11.4.4's NOTE 5 residue** (6 documents) is a non-isolated group whose own `Do` states an
  alpha, a blend mode or a soft mask *and* which holds a blending element. Its elements have to be
  composited onto the page's own colour and the backdrop's contribution removed afterwards —
  `C = Cn + (Cn − C0) × (α0/αgn − α0)` — and NOTE 4 says why one raster cannot: Table 140's group
  alpha has to be accumulated *apart* from the composite alpha, and an opaque backdrop destroys the
  difference. That is a buffer whose colour is the backdrop while its alpha is zero, which neither
  `tiny-skia` nor Vello has, because both store premultiplied samples and a colour at alpha zero
  does not exist in one.
- **§11.6.6's blending space for a painted group** (4 documents) wants the group's raster in its
  own components: a painted group's result is three components at every pixel, so the linearity
  that made a mask group one channel is a property of the *reduction* to luminosity and a painted
  group is not reduced. A second raster format, in three backends, with images and shadings inside
  it. ADRs 0217 and 0220 both say so and neither bears on it.
