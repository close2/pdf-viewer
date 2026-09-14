# 1056 — A group's result leaves into what its parent composites in

Session 1039. Status: **accepted**. One decision about §11.6.6's final compositing where the
parent itself composites in a space of its own, and the shape it takes in the display list.

`§N` is ISO 32000-2 and nothing else.

## 1. The clause, and what the tree did

§11.6.6's last action at a `Do`:

> If colour conversion needs to take place in order to composite the group into its parent,
> the rendering intent and black point compensation from the graphics state at the point of
> invocation of the Do operator shall be used for the conversion.

and §11.7.2's second sentence about the same moment:

> The resulting colours shall then be interpreted in the group's colour space when the group is
> subsequently composited with its backdrop.

This tree drew a group in a space of its own — a press, one component, three CIE-based ones —
only where the parent composited on the device, because the group's conversion out lands on
the device and that is where such a parent composites. Where the parent composited in a space
of its own, a group that changed the space was *recorded* (`nested_space_departed`,
`blending_changed`) and the whole page fell back to the device with a report: the parent's
compositing was given up so that the child's could be named. `group_space_census`'s new
parent -> group table counts that population at 31 of the 88 890 documents on this disk, and
`doc/pdf.js` holds none of it.

## 2. The decision: compose the conversions, and hand the backend one

The conversion the clause asks for is a function of the group's result: the group's own
conversion out, then the parent's conversion in. Both are known at the `Do`, and every backend
already resolves one grid, curve or cube per group before painting it onto the parent. So the
two are **composed in `pdf-model`** — `transparency::composed_into_parent` — and
`pdf_render::GroupBlending` carries the composition: a press's grid resampled into the parent's
channels, a curve's samples converted, a cube resampled, and for a group that composites on the
device's own three components the parent's conversion in as a cube on its own. No backend
changes, no protocol change, and the two backends that refuse a group in its own space keep
refusing it by the same name.

**The parent's conversion in is the one every mark inside the parent takes** — `Compositing::
paint` asked of a device colour — which is trap 6's one-route rule applied to a group's result.
The one exception is the clause's rather than a convenience: a CIE-based parent of three
components lets a `DeviceRGB` *graphics object* keep its components "for compositing purposes
only" (§11.7.2), and a group's result is not such an object — it is "interpreted in the group's
colour space", which differs from the parent's or nothing would be converted — so it goes in
from its XYZ (`RgbRoute::components_of_srgb`, §10.3.1), as every colour in a space of its own
does.

**What it costs is stated.** The composition is sampled with identity curves at
`INTO_PARENT_SIDE` (33 an axis, the side a table profile's cube already takes) and
`COMPOSED_PRESS_SIDE` (17, a profile press's own side; the assumed inks' two-sample grid is
multilinear and a composition with a non-linear conversion is not, so it cannot stay at two).
That is one interpolation's precision on top of the conversion out's — §11.7.2's NOTE 5 — and
it is memoised per interpretation on `(group space, parent space, rendering)` so that a page
drawing one shape of group a hundred times samples it once.

**A change nothing can see no longer costs the page its space.** §11.3.4 tells two spaces apart
only where something composites, so a group of three or four components with nothing
compositing inside it is run in the parent's space as before, and neither record is set. Both
records are set only where a change *is* visible and the group could not be drawn in its own
space — which after this round is the one shape below.

## 3. What is drawn now, and what is not

Every pairing of the four spaces this tree draws, in either direction, including a press inside
a press (two presses are two conversions, via the device) and a group inside a soft mask's group
— which used to inherit the mask's channel in silence, since the record was scoped out at the
mask and nothing read it. The tests hold each shape against arithmetic derived from the clause
(a Multiply's product per component in the group's own space, §11.3.5.2; §10.4.2.3's greys;
§8.6.5.3's gamma), and all six fail when the nested branch of `group_compositing` is planted
back to `None`.

**Not drawn, and reported by name:** an isolated knockout group naming a four-component space.
§11.4.6's staged rewrite edits one element list after the runs, and a pair is two; applying it to
both halves is a later round's work, and no corpus document states the combination.

## 4. What moved

`raster_golden` holds 971 of 974 first pages unchanged in pixels **and** list; the three that
moved are list-only. Two — `bug1703683_page2_reduced.pdf`, `personwithdog.pdf` — are last-ulp
differences in shading corner colours from a sibling round's in-flight `mesh.rs`/`shading.rs`,
confirmed by planting only this change onto the pre-round tree and watching their lists stay
byte-identical. One — `bug1721218_reduced.pdf` — is this round's: its luminosity masks hold
isolated `/DeviceCMYK` groups that blend, which now composite in ink inside the mask's group and
leave through a composed grid (1 -> 11 `FourComponents`), and the page's pixels do not move.
