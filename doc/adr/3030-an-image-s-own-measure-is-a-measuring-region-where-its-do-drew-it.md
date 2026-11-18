# 3030 — An image's or a form's own `/Measure` is a measuring region where its `Do` drew it

Session 2084. Status: **implemented.** Context: review 2038 (finding B73), ADR 1191, ADR 1593, ADR 1678. Clauses:
ISO 32000-2 §8.9.4, §8.9.5.1 Table 87, §8.10.1, §8.10.2 Table 95, §12.9.1 Tables 265 and 266, §12.10.2 Table 269.

Table 87 gives an image XObject an optional `/Measure`, "a measure dictionary … that specifies the scale and units
which shall apply to the image", and Table 95 gives a form the same entry for "the form". Table 269's `/LPTS` says
which square their registration points are stated in: "[t]he unit square is mapped to the rectangular bounds of the
Viewport, image XObject, or forms XObject that contains the measure dictionary". The measurement query read only
the page's `/VP`, so a map drawn as an image with its own georeference measured nothing.

## Decision

1. **The region is the object's bounds where its `Do` put them.** An image's bounds are the unit square §8.9.4
   draws it into, mapped by the CTM at its `Do`; a form's are its `/BBox` (Table 93), mapped by its `/Matrix` and
   that CTM (§8.10.1). Either map may rotate or shear the bounds. The interpreter reports each image or form it draws
   that states a `/Measure` (`MeasuredXObject`: the map into the display list's space, the bounds, the parsed
   measure) in `Interpretation::measured_xobjects`. The report is carried through the checkpoint and the divided read
   like the page's other per-object reports, and its count is bounded by the page's `/VP` bound. For an image the
   base image's entry is read, because a §8.9.5.4 alternate is another rendition of that image.
2. **A measured object is a `Viewport` with a placement.** `Viewport::placement` holds the map from the unit square
   into default user space (the bounds' corners carried through `content::user_space_at`) and the object's own
   bounds. `contains`, `unit_square` and `from_unit_square` use the placement where it is set. `/BBox` stays the
   `/VP` entry's geometry, so every existing caller of `Viewports` is unchanged.
3. **Objects are searched before viewports, the last drawn first. This is a choice.** ISO 32000-2 orders only the
   `/VP` array (§12.9.1's reverse search). Table 265 gives a viewport's measure to "the contents within the
   viewport" and Table 87 gives an image's "to the image". Where both claim a point, the object's is the narrower
   statement, about the marks under the point itself. `Viewports::on_page` puts objects after every `/VP` entry in
   drawing order, so `Viewports::at`'s reverse search meets the topmost object first, which carries §12.9.1's own
   rule for overlapping viewports over to objects.
4. **`/PCSM`'s "XObject position coordinates" are the object's own space.** For a `/VP` entry they are default user
   space, where its `/BBox` is stated (ADR 1678). For an image they are the unit square, the user space its `Do`
   drew it in (§8.9.4), and for a form its form space. `Viewport::object_position` answers all three,
   `located::locate` hands it to `geographic_position`, and `page_position` runs it backwards.

## Cost

One `get_key` per image `Do` and per form with a `/BBox`, and a vector that is empty on every page whose objects
state no measure. A measurement on a page not yet interpreted sees its `/VP` only, because nothing has been drawn
there. A form drawn as an annotation's appearance is not a `Do` on the page and is not reported.

Tests: `viewer-core` `image_xobject_measurement_spec_audit` (two, the audit's), `viewer-core`
`form_xobject_measurement` (one), `pdf-model`
`measurement::tests::an_image_s_region_is_its_unit_square_wherever_the_ctm_carried_it`.
