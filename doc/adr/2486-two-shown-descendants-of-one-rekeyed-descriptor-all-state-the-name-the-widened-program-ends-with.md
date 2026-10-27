# 2486 — Two shown descendants of one re-keyed descriptor all state the name the widened program ends with

Status: implemented. Session 1826, the archive slot of batch one hundred and twenty-three. Context:
ADR 2378 (re-keying an `Adobe-Identity` font from its `/ToUnicode`), ADR 2476 (a font that names a
renamed descriptor keeps a copy of the producer's own), `doc/history/1821` (where the question was
left unverified).

## The defect

ISO 32000-2 Table 120 makes a descriptor's `/FontName` the name its fonts state:

> This name shall be the same as the value of BaseFont in the font or CIDFont dictionary that
> refers to this font descriptor.

`fonts::rekeyed::embed` embeds a shipped face's glyphs into the descriptor a shown descendant
names. A second shown descendant over the same descriptor widens the same program: the shown set is
merged, the face is chosen again, and the program is written again at the same object. §9.9.2 makes
a subset's name a six-letter tag before the face's name, and `pdf_font::embed::renumbered` derives
the tag from the glyphs the subset holds, so the widened program has **another name**. The route
set the descriptor's `/FontName` and the *second* descendant's `/BaseFont` to it, and left the
first descendant's `/BaseFont` at the first name: the first descendant then referred to a
descriptor that did not state its name.

The hypothesis of the brief (the defect occurs where the second descendant's shown set is larger)
held, and is real rather than latent: with two descendants over one descriptor, the first showing
`A` and `B` and the second `C`, the output read `DDUAZV+ChromSerifOTF` on the descriptor against
`JZBHUQ+ChromSerifOTF` on the first descendant.

## The decision

`Rekeyed` records the `CIDFont` objects that show the descriptor's program. Each time the route
embeds, the gains it computed (the `/BaseFont`, and the `/CIDToGIDMap` of a `glyf` face) are
written to **every** descendant on the list, not to the one being embedded. The one gain that
depends on the dictionary it goes into is `/Subtype`, which Table 124 ties to the key the program
sits under (`/CIDFontType2` for `/FontFile2`, `/CIDFontType0` for `/FontFile3`); `gains_for` states
it only where a descendant does not already state the one the program needs, as the single
descendant's own case did.

Not decided here: the Type 0 parent's `/BaseFont`, which is the producer's and is not a font that
refers to the descriptor (Table 120 speaks of "the font or CIDFont dictionary that refers to this
font descriptor").
Table 119's `BaseFont` row for a Type 0 parent is a "should" ("this name should be the same as the
CIDFont's BaseFont name" for a Type 2 `CIDFont`), and its NOTE says that "[i]n principle, this is
an arbitrary name"; restating the parent's name is left for a later round.

## Proof

- `tests/archive/rekeyed.rs::two_shown_descendants_of_one_rekeyed_descriptor_state_the_descriptors_final_name`:
  a serif Identity font and a second Type 0 font whose `CIDFont` names the same descriptor and
  shows one more character. Before the change it fails on Table 120 (`left` the descriptor's
  name, `right` the first descendant's); with the change it passes with both descendants carrying
  the program and none the producer's name.
- `fonts_and_their_descriptors` holds every font dictionary of the output to Table 120, so the
  existing sibling test is held to it too.
- veraPDF 1.30.3 passes the planted two-descendant document converted at PDF/A-2b and PDF/A-4.
- The census (`archive_unbuilt` over pdf.js and `doc/corpora`, `--tree 6`) is line for line the
  pre-change one: no corpus document has two shown descendants over one descriptor.

## Consequences

- A document with such a pair converts to a file whose descriptor and descendants agree; before,
  it converted to one that disagreed.
- No other route is touched: the CJK pack's route gives each descendant a descriptor of its own
  (ADR 2461).
