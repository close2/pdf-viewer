# Transparency departures

Status: each reported where it can change a pixel. **§11.5.3's device branch was taken in the
three-hundred-and-eightieth session** (ADR 0217); the other three populations stand.
Priority: 23
Corpus: 14 documents
Clauses: §11.4, §11.5.3, §11.6.6
Code: `crates/pdf-model/src/content.rs`, `crates/pdf-model/src/soft_mask.rs`,
`crates/pdf-model/src/colour.rs`

Four populations, and the count beside each is what it costs on the corpus's first pages:

| | corpus | what it is |
|---|---|---|
| a knockout element whose shape is not its coverage | 5 | §11.4.6 composites an element with the group's *initial* backdrop; a shape that is not the element's coverage cannot be expressed as one alpha channel |
| a non-isolated group NOTE 5 cannot flatten | 6 | §11.4.4's NOTE 5 makes grouping equivalent to not grouping only where no element blends with the backdrop it excludes; these blend |
| a blending space that is not the device's three components | 4 | all `/DeviceCMYK`, and **still owed** |
| ~~a soft-mask group with such a space~~ | ~~7~~ → 7, narrowed | **taken in the three-hundred-and-eightieth**; what is left are two named residues, below |

Each is refused *by name* rather than approximated, which is the rule that keeps the corpus count
honest.

## What the fourth one turned out to be

**A mask group's result is one number and a painted group's is three**, which is why the fourth
population fell and the third did not. §11.5.3 reduces the mask group to a luminosity, §10.4.2.3
states that reduction for a subtractive space, and its conversion is *linear in the components*
except for one `min` — so the group is painted in that one number, on the grey channel a
rasteriser already has, and the mask reads it back unchanged. No second raster format, and the
only display-list change is what `SoftMaskKind::Luminosity`'s backdrop is measured in. ADR 0217.

**The condition the old report fired on was three steps from the departure**, which
`examples/luminosity_mask_census` found: of the 90 luminosity mask groups the census reaches, 39 blend in
`/DeviceCMYK` and 36 in `/DeviceGray`, and **not one of them sets a `k` colour** — so the
departure lived in the backdrop, not in the artwork.

### What that population still owes

Two residues, each reported by name and neither with a corpus witness that is not already here:

1. **A colour of more than one unit of ink.** §11.5.3 puts §10.4.2.3's `min` after the
   compositing and a rendered channel holds `0..=1`; `/BC [1 1 1 1]` weighs 2.0. The excess is
   clamped early, and the cost is a closed form — for artwork of ink `s` at coverage `α` over a
   backdrop of ink `1 + e`, at most `(1 − α)·e`, so it lives at the partly covered pixels of the
   group's own marks and nowhere else. **5 documents**: `issue18032.pdf`, `bug1755507.pdf`,
   `issue12798_page1_reduced.pdf`, `issue14297.pdf`, `issue9017_reduced.pdf`.

   Two constructions were tried and withdrawn, and ADR 0217 says why each failed: an unclamped
   backdrop, which comes to a negative grey on the graphics device and `quorra-scene` refuses;
   and scaling the group's channel with the inverse folded into the transfer table, which needs
   every colour in the group scaled and an image's samples are not colours the interpreter sees.
   **A round taking this needs the second one plus an image decoded into the group's own
   components** — which is residue 2, so the two are one piece of work.

2. **Colour that arrives already rasterised.** An image's samples and a shading's ramp are RGB
   before a display list can carry them, so a subtractive space is lost. **3 documents**:
   `bug1755507.pdf` (a `/Separation` image), `issue13520.pdf` and `bug1703683_page2_reduced.pdf`
   (`/DeviceN` shadings). The last is the round's own finding and the corpus's only witness for
   a subtractive raster in a `/DeviceGray` mask group.

## The three that stand

**There is a precedent for the display-list question**, from the three-hundred-and-seventieth
session: `pdf_render::ImageSource` carries a raster the display list *names* rather than holds,
and a backend produces it at `Grid::for_placement` (ADR 0210). What ADR 0210 settles is that
adding a second thing for a display list to say is not blocked by the interpreter's ignorance of
the device; what it does not settle is what the second thing should be, which is a compositing
question rather than a resolution one.

- **§11.6.6's blending space for a painted group** — the one ADR 0217 did *not* answer, and the
  reason is stated there: a painted group's result is three components, so the linearity that
  makes a mask one channel does not apply. This wants the group's raster in its own components,
  which is a second raster format.
- **§11.4.6's knockout whose shape is not its coverage** and **§11.4.4's NOTE 5 non-isolated
  group** are about a group's *shape* and its backdrop rather than its colour space, and nothing
  in ADR 0217 bears on either. §11.4.6's own sentence is the statement of what is missing: "[t]he
  existence of the knockout feature is the main reason for maintaining a separate shape value
  rather than only a single alpha".
