# 1191 — A measurement is a path the window traces and a string the document writes

Session 1177. Status: **accepted**. Carries out ISO 32000-2 §12.9 and the part of §12.10 that needs
no registry, on the readings [ADR 0082](0082-what-a-unit-of-user-space-is-worth.md) and
[ADR 0782](0782-a-count-is-not-a-page.md) built. Moves §12.9
and §12.9.1 from `partial` to `implemented` and takes §12.10 and §12.10.2 out of
[`doc/todo/65`](../todo/65-the-remaining-frontier.md)'s host-UI bucket.

## 1. The refusal that expired

§12.9's row has said, in one wording or another, since the module existed: *`partial` because
nothing takes the two points*. Every part of the arithmetic was here — Table 265's viewports, Table
266's measure dictionaries, Table 267's conversions, §12.9.2's five formatting steps and the
clause's own worked example as their test — and the missing thing was a person's hand.

That is the shape `doc/todo/02` §1 calls *a capability that reached the crate and never reached the
program*. It is the cheapest kind of refusal to retire and the easiest to leave standing, because
nothing fails while it stands.

## 2. Why one `Query` and no `Command`

`doc/ui-boundary.md`'s test for a new message is whether a host could answer the question itself.
This one fails it twice: the points are in the viewport's device pixels and §12.9's `/VP` is in
default user space, so the mapping is the arithmetic ADR 0118 keeps in one place; and what a unit of
that space is *worth* is Table 267's conversions and §12.9.2's steps, which are a reading of the
document.

`Command` fails the opposite test. §12.9 states no state for a viewer to be in — it says a
measurement is what "users of interactive PDF processors" perform and leaves every part of
performing one to the processor. So the mode and the points are the host's, exactly as a selection
rubber band is, and `viewer_core` keeps nothing between two presses. What crosses is one question
asked with the points already in hand:

```rust
Query::Measure(&'a [[f32; 2]])  ->  Answer::Measured(pdf_model::measurement::Traced)
```

The mode itself is `viewer_host::Measuring`, beside `Presenting` and `Clock` and for their reason:
when a press is a point, and what the answer says, are one decision for three windows, and a
`GtkLabel` against a `QStatusBar` against a title bar is what a toolkit is.

## 3. The path rather than the pair, and what the clause decides about it

§12.9.1 states the rule for two points and the query takes a list, so the rule has to be read once
more rather than assumed:

> Any measurement that potentially involves multiple viewports, such as one specifying the distance
> between two points, shall use the information specified in the viewport of the first point.

*such as* is what settles it: two points are the clause's example of a measurement involving
several viewports, not the definition of one. So the whole path is measured in the first point's
viewport, even where it leaves that rectangle. `crates/viewer-core/tests/measuring.rs` asserts it by
a difference rather than by a number — one page, two viewports at two scales, the same hundred
pixels traced from each end in turn.

A path that leaves the *page* is refused instead. `/VP` is an entry in a page dictionary, so two
pages of Table 29's continuous arrangement state two unrelated arrays and the clause has no
viewport to choose; measuring the part that stayed would answer a shorter path than the one a person
drew, with nothing on the screen saying so.

## 4. Table 267's two quantities that had no arithmetic, and the one that needed a reading

`/D` and `/A` were here. `/T` and `/S` were read into `Rectilinear` and used by nothing.

`/T` is a conversion and a unit: its first element converts "to the largest angle unit from
degrees", and §12.9.2's step a) fixes the initial value by naming this very entry — "the `T` entry
specifies degrees". So the angle is taken in degrees in the measuring system and the array converts
it; a reader working in radians is out by a factor of 57. Which angle a person meant at a vertex is
not stated, so the non-reflex one is measured and the choice is documented, on `Measure::area`'s own
precedent.

`/S` is the one that needed a reading, because two of the table's cells disagree in appearance. The
`/S` cell says "[t]he scale factors from `X`, `Y` (if present) and `CYX` (if `Y` is present) shall
be used"; the `/CYX` cell says that entry "shall be used for calculations (distance, area, and
angle)" and then "[o]ther calculations (change in x , change in y , and slope) shall not require
this value". The second is the specific one and it is a `shall not require`. It is also the one that
makes sense of the unit: `/S`'s first element converts "from units represented by the first element
in `Y` divided by the first element in `X`", which is a ratio *between* the axes' units and never a
length in one of them — so the entry that makes the two commensurable has nothing to do.

**The consequence is not cosmetic.** `/CYX`'s absence is a refusal the table states outright, and
the clause names the drawing it is about: "x representing time and y representing temperature". Such
a plot has no distance and no area, and a gradient is the one measurement it is *for*.
`Rectilinear::gradient_axes` is that sentence, and it is why `Measure::slope` answers where
`Measure::distance` refuses.

## 5. §12.10: what the file states, and where the registry begins

A `GEO` viewport has no Table 267 arrays, so a traced path in one has no length. What it has is
everything §12.10 states, and that is worth a person's while:

- **`/Bounds`** — Table 269 gives it one job, its points "describe the bounds of an area for which
  geospatial transformations are valid", and "[f]or maps, this bounding polygon is known as a
  neatline". `Geospatial::within_bounds` is the question asked before anything is said about where a
  point is on the earth. The absent entry is the whole unit square, which the table states.
- **The unit square itself** — `/LPTS` holds "points in a 2D unit square" and the table says which
  square, "mapped to the rectangular bounds of the `Viewport`". `Viewport::unit_square` is that
  division, with the corners taken as stated because Table 265 makes their order "determine the
  orientation of the measuring coordinate system".
- **The system, by name** — `/GCS`'s EPSG code or its Well Known Text, whether it is projected,
  `/DCS` beside it, `/PDU`'s three preferred display units, and how many `/GPTS`–`/LPTS` pairs
  register the object.

**What is absent is a latitude, and it is absent by reading rather than by omission.** §12.10 states
the correspondence at the registration points and states no function between them; where `/GCS` is
projected it needs the EPSG registry and ISO 19162's grammar, which §12.10.3 names as texts outside
this standard. Interpolating would produce coordinates that look right and are somewhere else, which
is principle 5's failure mode with a map attached. So the window says what the file says and says
that the clause states no position between them — §12.10 and §12.10.2 stay `partial` for exactly
that leg, and move out of the host-UI bucket into the dependency one.

## 6. One thing the tests caught that no reading would have

§12.9.2's `format` answers an empty number format array with the empty string, which is right: "an
array is one or more number format dictionaries", so a file stating none has said nothing about how
to display anything. Table 267 makes each of its four arrays *(Optional)*. Composed naively, a
measure dictionary stating no `/S` therefore produced `Some("")` — a slope of nothing, shown beside
the word *slope*. `measurement::stated` filters it, and the distinction is the clause's: an absent
array is the file saying nothing about a quantity, and the empty string is what an algorithm with no
unit to walk produces. `the_clauses_own_formatting_reaches_a_host_through_the_query` is where it
failed.

## 7. What it cost, and what it did not

One `Query`, one `Answer`, no `Command`, no `Event`. `viewer-confined`'s wire gained a query kind, an
answer kind and a boxed `Reply` variant; the C ABI gained `quorra_measure` and seven constants, with
a `part` selector rather than seven symbols because six quantities and a sentence over one path
would be six chances to be handed a string from a different reading of it. `QUORRA_EVENT_KIND_COUNT`
and `QUORRA_ABI_VERSION` both stayed where they are.

**No pixel of any page changes.** The mode takes one thing away — while it is on, a press is a point
rather than the start of §12.4.2's selection, because the two gestures are the same gesture and a
window cannot tell them apart from the pointer alone.
