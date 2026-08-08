# ADR 0234 — A shape is not an alpha, and one group can tell

Status: accepted, 2026-08-08 (session 397).

## Context

`doc/todo/23` named three transparency departures left after ADRs 0217 and 0220 closed §11.5.3's,
each refused *by name* and each with a corpus count:

| | corpus | what it is |
|---|---|---|
| a knockout element whose shape is not its coverage | 5 | §11.4.6 composites an element with the group's *initial* backdrop; a shape that is not the element's coverage cannot be expressed as one alpha channel |
| a non-isolated group NOTE 5 cannot flatten | 6 | §11.4.4's NOTE 5 makes grouping equivalent to not grouping only where no element blends with the backdrop it excludes; these blend |
| a blending space that is not the device's three components, for a **painted** group | 4 | all `/DeviceCMYK`; a painted group's result is three components, so it wants the group's raster in its own components |

**The first was chosen, and the reason is that its answer is a display list and the other two are
a raster format.** §11.4.4's NOTE 5 residue needs the group's elements composited onto the *page's*
own colour with the group alpha accumulated separately — NOTE 4 says why one raster cannot do it —
which means a buffer whose colour is the backdrop while its alpha is zero, and neither `tiny-skia`
nor Vello has one: both store premultiplied samples, where a colour at alpha zero does not exist.
§11.6.6's painted group needs four components per pixel through a whole group and a conversion at
the end, which is a second raster format in three backends and a decision about images and shadings
inside it. §11.4.6's needs one more thing for a display list to *say*, which is the shape it has
been ADR 0210's and ADR 0220's precedent to add.

## Decision

### The display list states the shape beside the object, and only where they differ

`pdf_render::Command::Shaped { object, shape }`. The object is what any other command is; the
shape is the same object with every source of *opacity* removed, so the alpha a backend draws it
with is §11.6.4.2's shape. §11.6.4.2 gives the shape from geometry alone — for a path "the shape
shall always be 1.0 inside and 0.0 outside the path" — and §11.6.4.3's soft mask and §11.6.4.4's
constant are opacity. So:

- a **fill or stroke** in a uniform colour: the same command with an opaque paint, no mask and
  the blend mode dropped;
- an **image** whose samples are opaque: the same command at alpha 1 with no mask;
- a **group**: the same group whose elements are their own shapes, at alpha 1 with no mask. Its
  shape is the union of its elements', which is what drawing those shapes onto transparency
  accumulates — and **knockout or not makes no difference to a shape**, which is arithmetic rather
  than a simplification: §11.4.6 accumulates `(1 − f) × F + f` and §11.4.4 accumulates
  `Union(F, f) = F + f − F × f`, and the two expressions are equal.

Where the shape *is* the coverage — an opaque mark with no mask, which is every glyph of §9.3.8's
text objects and most of everything else — nothing is wrapped and nothing changes. That is not an
optimisation dressed as a rule: the one-draw form is the same arithmetic, and doubling the marks of
a knockout text object to restate what a rasteriser already knows would be a cost with no answer
attached.

### Two Porter-Duff operators, and the second one is the clause

§11.4.6's two stages are (a) composite the object with the group's initial backdrop "disregarding
the object's shape and using a source shape value of 1.0 everywhere", then (b) a "weighted average
of this result with the object's immediate backdrop, using the source shape as the weighting
factor". On the transparent initial backdrop of §11.4.5's isolated group — which is what both
backends build — the clause's `αt = Union(αg0, qs)` and `Ct = qs × Cs` collapse, and the pair
becomes one line per pixel in premultiplied form. With `P` the accumulated group result, `f` the
shape and `S` the object's premultiplied colour (already carrying `f × opacity`):

```text
P' = (1 − f) × P + S
```

which a backend draws as **Destination-Out with the shape, then Plus with the object**.

**Ordinary source-over in the second step is not the clause**, and this is the part worth the
paragraph. It weights the backdrop a second time, by `1 − f × opacity`, where §11.4.6 weights it by
`1 − f` alone. The two agree wherever the object is opaque or the shape is 0 or 1 — which is why a
scene of rectangles cannot see it — and at a half-covered pixel under a half-opaque mark they are
**32 of 255** apart on two channels. `the_object_is_added_to_the_backdrop_the_shape_left_behind`
is that pixel, and it fails with `(191, 96, 160)` against the clause's `(191, 64, 128)` when
`Compose::Add` is changed to over.

Neither operator can do what `Compose::Copy` did to `render-gpu` in the seventy-first session: at
zero contribution Destination-Out leaves the destination exactly and Plus adds nothing, so a layer
whose shape is the whole target erases nothing outside the mark. And the sum is bounded by 1 at
every pixel — the backdrop keeps `1 − f`, the object brings `f × opacity` — so `Plus`'s saturation
never engages and the pair is arithmetic rather than an approximation of it.

`render-cpu` says both as per-draw blend modes (`DestinationOut`, `Plus`). `render-gpu` has neither
as a parameter and reaches them through two layers, which is also where `open_layer` came from:
Vello needs a whole-target layer for every operator beyond source-over, and this backend now has
three of them (the mask's `DestIn` and these two) saying so in one place.

### `render-quorra` refuses it, by name, and that is the gain

`quorra_scene::Compose` has source-over and `Src`, "Porter-Duff Source, **modulated by coverage**"
— which is exactly the assumption a shaped element exists to contradict. So the backend refuses,
naming the clause and the two operators it would need, and `doc/QUORRA_FEEDBACK.md` §14 is the
request.

**Four corpus pages moved from `agree` to `refused` in that gate, and every one of them is a page
quorra used to draw wrongly.** The agreement was two backends making the same wrong assumption
about the same display list; the display list now says what the clause says, and one of the two
backends cannot draw it. A refusal that replaces an agreement about a wrong picture is not a
regression, and the ratchet is re-set with that argument rather than around it.

### And §11.6.4.3's `/AIS`, which this round could not leave alone

Table 57's alpha source flag decides whether the mask and the two constants are shape or opacity.
Alpha is their product (§11.3.7.1), so the flag changes nothing anywhere the product is all that is
used — and the ledger's row said exactly that, twice, in two different wordings.

**The second wording was already false when it was written.** From the seventy-first session a
knockout group drawn by modulating Porter-Duff Source with coverage reads the constant alpha as
*opacity*; under `/AIS true` it is shape, and the picture is different. So the one place in this
renderer where the flag can decide a pixel is the one place that had stopped reporting. This round
would have made it worse — `stated_shape` builds a shape by removing precisely the two quantities
`/AIS` reclassifies.

The entry is therefore read, and **every knockout group is refused by name while it is set**. The
flag is monotone within a page on purpose: what matters is whether any element of a knockout group
was painted under it, which is a question about the graphics state's history rather than its value,
and the over-approximation costs nothing measurable — the corpus's incomplete list is the same 67
with the flag read and without it.

**And "no corpus document writes `/AIS`", which the row asserted, is false**: 53 of the 974 state
the entry and **nine state it true** — `bug1703683_page2_reduced`, `bug1755507`,
`issue12798_page1_reduced`, `issue14297`, `issue18032`, `issue18956`, `issue19360`, `issue7891_bc0`
and `issue7891_bc1`. That is trap 8's shape from the other side: a census nobody had run, standing
in for one nobody could.

## Consequences

### The measurement, against the clause's own arithmetic

Three fixtures, each derived from §11.4.6 rather than from another renderer, and each checked by
putting the old route back.

| fixture | the clause | what the old route drew |
|---|---|---|
| a soft-masked blue over an opaque red, in a knockout group | shape 1 knocks the red out whole; blue at ½ over white, **(127, 127, 255)** | **(127, 0, 127)**, a purple band |
| a nested group at `ca 0.5` over an opaque red | the group's shape is where it marks, so again **(127, 127, 255)** | the group was not built at all — the whole page was reported |
| a half-covered pixel under a half-opaque mark | `P' = ½ × (1,0,0;1) + ¼ × (0,0,1;1)` over white, **(191, 64, 128)** | **(191, 96, 160)** with source-over in stage b) |

The third is the one the other two cannot see, and it is why it exists: every fixture whose shape
is 1 in the overlap gives the same answer under both compositing rules.

**The external evidence is the oracle, and it is the strongest number in the round.** Four
documents stopped being reported and were therefore judged for the first time: the corpus's
contradicted count fell **74 → 69** and its agreeing count rose **899 → 904**. Nothing was tuned
toward a reference — the fixtures above are arithmetic — and four pages this tree had drawn as
ordinary groups now agree with a consensus of poppler, mupdf and ghostscript.

### What moved on the gates

- **corpus 70 → 67 incomplete**, and nothing joined. `knockout_nested.pdf`,
  `knockout_nested_group_alpha.pdf` and `knockout_smask.pdf` lost their only report;
  `knockout_inner_backdrop.pdf` lost its knockout report and keeps §11.4.4's; `issue18032.pdf`
  keeps all three of its, and its knockout one is now attributed to the non-isolated condition
  rather than to a shape.
- **oracle 899 → 904 agreeing and 74 → 69 contradicted**, complete-page agrees 859 → 862.
- **quorra 920 agree / 36 differ / 1 refused → 916 / 36 / 5**, with the argument above.
- **text, dates, XMP and JPEG 2000 unmoved**; the text gate's ungated-incomplete count falls
  67 → 64 as the three documents become gated, on the same word denominator.

### `doc/todo/00`'s step 7, run before and after, and byte-identical

Our ink minus the lightest reference's, over all **786** ambiguous pages, from the artefacts each
oracle run leaves on disk — the *before* half taken by stashing the round and re-running the gate,
because the artefacts are overwritten. **Every line is identical, including the labels**: twenty
names at or past −1 and sixteen of them documents this tree calls incomplete, head
`issue16038.pdf` −5.758 then `issue12295.pdf` −1.712, `checkbox_no_appearance.pdf` −1.200,
`issue14297.pdf` −1.146 and `issue7821.pdf` −1.000.

**That is the expected result here and it is worth saying why, because "nothing moved" is
otherwise indistinguishable from "the sweep did not run".** The sweep's population is the
*ambiguous* bucket, and every page this round changed was **contradicted** before it and agrees
after it. A page in neither state can move without the sweep seeing it; a page moving between those
two cannot be seen by it at all. So the sweep says what it is for — nothing stopped being drawn —
and the movement is in the oracle's own verdicts.

(The head reads −5.642 in ADR 0220's record and −5.758 in both halves of this round's, on the
`issue16038.pdf` page `doc/todo/00` already names as having drifted once between sessions. It did
not drift in this one.)

### What the other two populations now owe

- **§11.4.4's NOTE 5 residue** (6 corpus documents, now including the whole of what blocks
  `issue18032.pdf`'s knockout group) needs Table 140's group alpha accumulated *apart* from the
  composite alpha, which NOTE 4 says an opaque backdrop destroys. Nothing here helps: a shape is a
  second quantity a *command* can state, and this is a second quantity a *buffer* has to hold.
- **§11.6.6's blending space for a painted group** (4 documents, all `/DeviceCMYK`) is unmoved and
  ADR 0217's paragraph about it still stands.
- **Inside §11.4.6**, two elements keep the report and the report now names which: an image whose
  samples may be §8.9.6.2's stencil or §11.6.5.2's `/SMask`, and a shading whose colours already
  carry §11.6.4.4's constant. Both are the same shape of problem — one alpha carrying two
  quantities in a *raster* — and both would be answered by an `ImageSource` that keeps them apart,
  which is a smaller construction than either population above.
- **`/AIS` is read and not honoured.** Honouring it means composing the mask and the constants into
  the *shape* instead of into the object, which is a second `stated_shape` rather than a new
  vocabulary. Nine corpus documents state it; none of their knockout groups is drawn today.
- **`render-gpu`'s coverage path keeps its documented residue.** Where the shape is the coverage it
  still draws the element straight into the scene with source-over after the Destination-Out, which
  is the `(1 − f)(1 − f × opacity)` above; `knock_out`'s comment has carried the bound since the
  seventy-first session. Removing it means a Plus layer per element, and the elements are §9.3.8's
  glyphs — a cost worth measuring before it is paid.
