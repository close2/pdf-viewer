# ADR 1098 — a Lab profile's device scale is stated by §8.6.5.5, not by the ICC text

## Status

Accepted.

## Context

ISO 32000-2 §8.6.5.5 Table 67 admits four ICC data colour spaces: `'GRAY'`, `'RGB '`, `'CMYK'` and
`'Lab '`. This tree read the first three and refused the fourth, so such a space fell to Table 65's
`/Alternate` — the clause's own instruction for a profile "not supported" — and drew a colour the
producer did not specify.

The refusal had a reason, recorded in ADR 1008 and in `icc.rs` beside the match arm. An ICC lookup
table is indexed on the unit interval. The ICC texts define a Lab encoding for the
*connection-space* side of a table and say the definition does not reach the header's
data-colour-space field, so the scale by which a Lab **device** value — `L*` in hundreds, `a*` and
`b*` signed — lands on a table's input is not something the ICC specification states. Without it
there is no way to evaluate the profile, and a reading invented for the purpose would be
curve-fitting.

The half that closes it was in §8.6.5.5 the whole time, and the row said so: "the one statement of
that scale a PDF carries is Table 66's `/Range`, … which the sentence above says is unread." What
the row did not see is that the clause states the scale **twice**, in an order, and the second
statement does not depend on the file at all.

## Decision

**Read the component range from the specification, and admit the `'Lab '` class on it.**

1. The prose under Table 67 says the range of each component "is a function of the colour space
   specified by the profile and is indicated in the ICC specification", and **Table 68 prints it**:
   `[0.0 1.0]` for `Gray`, `RGB` and `CMYK`, and "𝐿 ∗ : [0 100] ; a ∗ and 𝑏 ∗ : [-128 127]" for
   `L*a*b*`. So the *profile's own data colour space* states the range; a file need say nothing.
2. Table 65's `/Range` is the file restating the same fact — "[t]hese values shall match the
   information in the ICC profile" — and where a document states one it is taken
   (`Profile::with_range`). An array of the wrong length, or a pair whose minimum exceeds its
   maximum, is not this entry and leaves the profile's own range standing.
3. `Profile::encoded` is the one place a device value crosses onto the unit interval a table indexes
   by: clipped to the range, then mapped onto it. The clip is §8.4.1's — "[p]arameters that are
   numeric values, such as the current colour, line width, and miter limit, shall be clipped into
   valid range, if necessary" — applied where the bound is known.
4. A `'Lab '` profile's `A2B` table drops its `mft` matrix. ISO 15076-1 section 10.10 confines that
   stage to an input colour space of PCSXYZ, and a "to CIE" table's input is the data colour space.
5. **Table 88 is the same fact from the image's side** — an `ICCBased` image's default `/Decode` is
   "[s]ame as the value of Range in the ICC profile of the image's colour space" — so
   `ColourSpace::component_range` answers with the profile's range rather than the unit interval.

**Table 65's literal default of `[0.0 1.0 …]` is not applied to a `'Lab '` profile, and that is the
one place two sentences of this clause disagree.** A `'Lab '` space whose file omits `/Range` would
then have `L*` confined to the unit interval, which contradicts Table 68's row for the same space
and the sentence above it that sends a reader to the profile. The entry's default is what a file
means when it says nothing, and what this file's clause says it means is the profile's own range.

## Consequences

- Nothing that drew before draws differently. Table 68 gives the other three data colour spaces the
  unit interval, where `encoded` subtracts zero and divides by one;
  `the_unit_range_leaves_every_component_where_it_was` pins that bit for bit on the shipped sRGB
  profile, and `raster_golden` holds every page.
- **The population is zero and the fixtures are therefore hand-built** (trap 8). Session 987's scan
  of every directly filtered stream across the 1249 documents in `doc/pdf.js/test/pdfs`,
  `doc/corpora/` and `doc/corpora-own/` found 333 embedded profiles — 235 `'RGB '`, 95 `'GRAY'`,
  3 `'CMYK'` — and no `'Lab '`. What closing it buys is a class of file this reader can no longer
  draw wrong in silence, not a corpus page.
- `Profile::identity` moves when a stated `/Range` is applied, so two `ICCBased` spaces over one
  profile stating different ranges are two spaces to the press registry and to §8.6.5.7's
  passthrough. A conformant file cannot produce that case, since both ranges must match the profile.
- The `/Alternate` path is untouched, and the sentence that would bind it is disposed by arithmetic:
  §8.6.5.5 asks that a substituted value be constrained to the ICCBased range and then to the
  alternate's, and clamping to an interval *contained* in another is the same as clamping to it
  alone. Every alternate space Table 67 and Table 68 admit has a range inside the ICCBased one, and
  every space in this tree clamps to its own.

## Alternatives rejected

- **Keep refusing the class.** It is a colour the producer specified and a reader that can now read
  it; the refusal was waiting on a statement of scale, and the statement is in the clause.
- **Carry the range on `ColourSpace::Icc` instead of on the profile.** The range is the profile's
  information travelling through PDF — Table 65 says so in as many words — and it is what every
  table in `icc.rs` indexes by, so holding it anywhere else would put the conversion's own unit
  outside the type that performs the conversion.
- **Apply Table 65's literal `[0.0 1.0 …]` default to a `'Lab '` profile.** It would make a
  conformant file that omits an optional entry render nonsense, which is the reading Table 68 exists
  to prevent.
