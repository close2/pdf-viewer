# 3665 — A measurement is taken along the measuring axes its viewport or object orders

Session 2241. Status: accepted. Context: review 2239 (B03), ISO 32000-2 §12.9.1 Tables 265, 267
and 269, ADR 3030.

## Decision

`Viewport::along_measuring_axes` turns a point of default user space into coordinates along the
measuring axes, still in user space units (which is what Table 267's `/X` converts from), and
`Viewports::distance` and `Viewports::traced` pass every point through it before `Measure`
computes anything. `Measure`'s methods are unchanged and take points already so oriented.

- **A `/VP` entry**: Table 265's `/BBox` corner order "shall determine the orientation of the
  measuring coordinate system (that is, the direction of the positive x and y axes) in this
  viewport", and Table 267's `/X` repeats it ("[t]hese directions shall be determined by the
  `BBox` entry of the containing viewport"). A corner order can only reverse an axis, so each
  coordinate is negated along an axis stated high first.
- **An image's or a form's own `/Measure`** (Tables 87 and 95) has no `/BBox` to order. Its axes
  are the edges of the unit square Table 269 maps to the object's "rectangular bounds", as the
  `Do` drew it (`Placement::square`), each normalised to unit length. **This is a reading**: the
  standard names the containing viewport's `/BBox` for direction and says nothing more for an
  object; the unit square is the one rectangle §12.10 already treats as the object's, its
  lower-left corner is where a `/BBox` would start, and normalising keeps `/X`'s "from default
  user space units" true.

## Which quantities can see it

The slope (and Table 267's change in x or y, which nothing shows yet) is signed: a reversal of one
axis negates it, of both leaves it. Distance and length (a `hypot`), area (the shoelace sum's
magnitude) and angle (the non-reflex angle, from the cross product's magnitude) are sign-free, so
a `/VP` entry's corner order changes none of them. A placed object can also rotate or shear its
axes, which every quantity sees where `/X` and `/Y` scale differently. §12.10's geographic reading
keeps the page's points, because it maps them through the unit square itself.

## Cost

A traced path is copied once into oriented points. Nothing else changes on any path a measure
does not reach.
