# Transparency departures

Status: each reported where it can change a pixel. **§11.5.3's population is closed**: its device
branch was taken in the three-hundred-and-eightieth session (ADR 0217) and both residues that left
behind were paid in the three-hundred-and-eighty-third (ADR 0220). Three populations stand.
Priority: 23
Corpus: 11 documents
Clauses: §11.4, §11.6.6
Code: `crates/pdf-model/src/content.rs`, `crates/pdf-model/src/colour.rs`

Four populations, and the count beside each is what it costs on the corpus's first pages:

| | corpus | what it is |
|---|---|---|
| a knockout element whose shape is not its coverage | 5 | §11.4.6 composites an element with the group's *initial* backdrop; a shape that is not the element's coverage cannot be expressed as one alpha channel |
| a non-isolated group NOTE 5 cannot flatten | 6 | §11.4.4's NOTE 5 makes grouping equivalent to not grouping only where no element blends with the backdrop it excludes; these blend |
| a blending space that is not the device's three components | 4 | all `/DeviceCMYK`, and **still owed** |
| ~~a soft-mask group with such a space~~ | ~~7~~ → 0 | **closed**, below |

Each is refused *by name* rather than approximated, which is the rule that keeps the corpus count
honest.

## The fourth one, and why it fell

**A mask group's result is one number and a painted group's is three.** §11.5.3 reduces the mask
group to a luminosity, §10.4.2.3 states that reduction for a subtractive space, and its conversion
is *linear in the components* except for one `min` — so the group is painted in that one number,
on the grey channel a rasteriser already has, and the mask reads it back. ADR 0217.

**And the `min` waits for the compositing, which is what the second round added.** The clause
applies it *after* the group has been composited with its backdrop, and a rendered channel holds
`0..=1` where registration black weighs 2.0. `InkScale` is the divisor that makes it fit — one unit
for a `DeviceGray` group, because §11.6.6's conversion *into* that space is itself the `min`, and
two for a `DeviceCMYK` one, because four clamped components weigh `0.3 + 0.59 + 0.11 + 1.0`. What
is left of §10.4.2.3 is composed into the mask's transfer table, where both backends already apply
one. ADR 0220.

**The same round carried an image's samples and a shading's ramp into that quantity**, which is
what the scaled channel needed to be sound: `crate::colour::Compositing` is threaded through
`crate::image`, `crate::shading` and `crate::mesh`, so a `DeviceCMYK` raster inside a mask group is
converted by §10.4.2.3 rather than to RGB first. Three documents lost their reports outright —
`issue14297.pdf`, `issue9017_reduced.pdf` and `bug1703683_page2_reduced.pdf` — and the corpus's
incomplete count went 73 to 70 with nothing joining.

**The census is why the population was smaller than the report.** `examples/luminosity_mask_census`
reaches 90 mask groups over 964 documents: 39 blend in `/DeviceCMYK`, 36 in `/DeviceGray`, and
**not one sets a `k` colour**. So the departure lived in the backdrop and in the rasters, not in
the artwork, and the old condition fired three steps from it.

### What replaced it, and both have no corpus member

Two reports remain inside §11.5.3, each kept because the alternative is a silence:

1. **A `Lab` mask group.** Its three components are not a linear map of the device's, so *neither*
   of the clause's branches is this tree's. No corpus document states one.
2. **A blend mode inside a `/DeviceCMYK` mask group.** §11.3.5.2 applies a separable function to
   each component "expressed in additive form", and this composites one weighted average of the
   four — exact for `Normal`, which is affine, and for nothing else. **This was silent for three
   sessions**: ADR 0217 removed the sentence that had covered it without naming it, and ADR 0220
   is where it was re-derived from the clause. No corpus document states one either, and a
   `DeviceGray` group is exact for every blend mode because its channel *is* its one component.

## The three that stand

**There is a precedent for the display-list question**, from the three-hundred-and-seventieth
session: `pdf_render::ImageSource` carries a raster the display list *names* rather than holds, and
a backend produces it at `Grid::for_placement` (ADR 0210). ADR 0220 is the second instance and the
more useful one here — a display list can carry a *quantity* rather than a colour, and the two
backends stay in agreement because the arithmetic between the raster and the mask is one 256-entry
table both are handed.

- **§11.6.6's blending space for a painted group** (4 documents) is the one neither round answers,
  and the reason is stated in both: a painted group's result is three components at every pixel, so
  the linearity that makes a mask one channel is a property of the *reduction* to luminosity and a
  painted group is not reduced. This wants the group's raster in its own components, which is a
  second raster format.
- **§11.4.6's knockout whose shape is not its coverage** (5) and **§11.4.4's NOTE 5 non-isolated
  group** (6) are about a group's *shape* and its backdrop rather than its colour space, and
  nothing in either ADR bears on them. §11.4.6's own sentence is the statement of what is missing:
  "[t]he existence of the knockout feature is the main reason for maintaining a separate shape
  value rather than only a single alpha".
