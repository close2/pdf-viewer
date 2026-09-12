# ADR 1017 — The kind of an alpha, carried where it is decided, and two references that read a soft mask as shape

Status: accepted, 2026-09-12. Session 997.

## Context

ADR 1009 §6 proved that one alpha per pixel costs this tree nothing outside §11.4.6, and left
that clause two elements it could neither draw nor state: an image whose samples may be
§8.9.6.2's stencil (shape) or §11.6.5.2's `/SMask` (opacity), and a shading whose colours carry
§11.6.4.4's constant. Its §8 priced the close as "one bit beside the value, in the colour
round's files" — `crate::image` and `crate::shading`. This round took it, and the bit turned
out to live in three places and to be a bit in only one of them.

## 1. What the kind is, at each site

§11.6.4.2 lists the sources of an object's shape and names the masks that modify an image's:

> For images (8.9, "Images"), the shape shall be 1.0 inside the image rectangle and 0.0
> outside it. This may be further modified by an explicit or colour key mask (8.9.6.3,
> "Explicit masking" and 8.9.6.4, "Colour key masking").

> For image masks (8.9.6.2, "Stencil masking"), the shape shall be 1.0 for painted areas and
> 0.0 for masked areas.

An image's own `/SMask` and a codestream's `/SMaskInData` are not on that list. They are on
§11.6.4.3's — among the ways "[t]he soft mask shall be specified" — and that clause makes a
soft mask "a source of either shape (fm) or opacity (qm) values, depending on the setting of
the alpha source parameter", whose Table 57 default is opacity. The two are exclusive by the
same clause ("shall override any explicit or colour key mask"), so one image's alpha channel is
one kind — except a stencil carrying an `/SMask` of its own, whose alpha is the product.

- **An image**: `image::SampleAlpha`, decided in `decode_parts` where the masks are applied.
  `Shape` for a stencil, an explicit mask, a colour-key mask, and an unmasked image (whose alpha
  is 1.0 throughout, which is the rectangle); `Opacity` for an `/SMask` on either route and a
  non-zero `/SMaskInData`; `Both` for a stencil under an `/SMask`.
- **A shading**: the kind is a constant fact and needs no bit. Every colour a shading carries
  is opaque before `Shading::with_alpha` folds §11.6.4.4's constant in — the one exception is
  §8.6.6.4's `/None` colourant, which this tree reads as opacity 0 the way it reads a `/None`
  fill — and *where the shading paints* is not in its colours at all: an axial ramp that does
  not extend, a mesh's triangles and a sampled grid's cover each leave their unpainted region
  unpainted whatever the colours say. So the shape is `Shading::opaque`, every colour at
  alpha 1.0, which is the clause's "1.0 inside and 0.0 outside the bounds of the shading's
  painting geometry" — the `/Background` included, because Table 77 confines it to a pattern
  and §11.6.7 fills the pattern's implicit group with it before the `sh`.
- **A stencil painted through a pattern** (ADR 0151) is a fill of the unit square through a
  §11.5.2 alpha mask made of the stencil. That mask is §8.9.6.2's shape wearing §11.6.4.3's
  vocabulary, and the fill cannot say so; the mask's identifier is recorded as shape.

## 2. Where it is carried, and why not on the display list

The right home for an image's kind is a field of `pdf_render::Image`, beside `interpolate`:
decided by the decoder, read by whoever asks. It was not put there this round, and the reason
is worth writing down rather than leaving as an omission. `pdf_render::Image` is built by
struct literal in six crates outside this round's files — `grep -rn 'interpolate[:,]'
--include=*.rs crates` counts the sites — `render-gpu`'s and `render-raster`'s tests and
examples, `viewer-host`, `viewer-core`, `viewer-ui`, and `viewer-confined`'s protocol, which
also serialises the struct field by field — so a new field is a change to the wire and to six
crates the round was not given, and the rule for a parallel round is that a file it was not
given is not its to change (ADR 1000 §5). `Command::Image`,
`ImageSource` and `Shading` are the same shape of constraint, and a new `ImageSource` variant
would fall to the codec's `Unknown` arm and refuse the whole list.

What the reader actually needs is narrower than a field: the interpreter decides the kind in
`image::decode_parts` and reads it in `transparency::stated_shape`, both in `pdf-model`, and
nothing between them needs to know. So the kind is **recorded where the image is drawn and
found again by the raster's identity** — `image::DrawnAlphas`, kept inside `MaskCache` because
that is the interpreter's per-run knowledge about image masks and the one field of the
interpreter's both files could reach. A decoded raster is the same one when its samples are the
same allocation (`Arc::ptr_eq` on `data`, the allocation pinned by the record's own clone), and
a deferred one when it is the same producer, which is the equality `DeferredImage` already
defines for itself. Identity rather than content because content cannot decide it: a stencil's
`{0, 255}` and a one-bit soft mask's are the same bytes. The record is taken *after* §10.5's
transfer, which produces a new raster, so a transferred image is found too.

The cost of the choice is stated in the code and here: a field would be read by anyone, the
record only by the interpreter that wrote it. `implicit_knockout_group` — the `B` and text
object callers in `path.rs` and `text.rs`, neither this round's — is not handed the record and
passes an empty one, so a Type 3 glyph drawn as a stencil among a text object's parts keeps
today's report; the owed change is the record as a fourth argument at three call sites.

## 3. What each element states as its shape

Under the opacity reading (`shape_without_the_mask_and_the_constants`):

- an image whose alpha is **shape** is its own shape: the same raster at constant alpha 1.0,
  no graphics-state mask, blend Normal;
- an image whose alpha is **opacity** has the clause's rectangle for a shape: the unit square
  under the image's transform, filled white, clipped as the image is. The CPU oracle draws an
  image by filling exactly that path with a pattern shader, so the shape's coverage is the
  object's at every edge pixel;
- a fill or stroke whose paint is a shading has `Shading::opaque` for a paint, shared where the
  shading was opaque already;
- a mask on a fill that the record names as a stencil's is kept, where every other mask comes
  off.

Under the shape reading nothing changes: `shape_the_alpha_already_is` already made an image
its own shape, mask and constant included, by §11.6.4.3's NOTE 1 and §11.6.4.4.

## 4. The fixtures, and what the pictures show

Four fixtures in `transparency.rs`'s tests, each an isolated knockout group holding an opaque
blue square and one element over its left half, on a yellow page, with the two pixels
`(30, 50)` and `(70, 50)` computed from §11.4.6's two stages:

| fixture | element | before (flat) | after | the other kind would give |
|---|---|---|---|---|
| `a_stencils_alpha_is_its_shape_inside_a_knockout_group` | 2×1 stencil, red at `ca ½` | `(128, 0, 128)` / blue | `(255, 128, 0)` / blue | left the same, right knocked out to yellow |
| `an_smasks_alpha_is_opacity_and_the_shape_is_the_image_rectangle` | 2×1 red image, `/SMask` `[0, 255]` | blue / red | **yellow** / red | blue / red |
| `a_translucent_shadings_shape_is_where_it_paints` | axial `sh`, `/Extend [false false]`, `ca ½` | `(128, 0, 128)` / blue | `(255, 128, 0)` / blue | — |
| `a_stencil_painted_through_a_pattern_keeps_its_shape` | the stencil, painted with a shading pattern at `ca ½` | `(128, 0, 128)` / blue | `(255, 128, 0)` / blue | left the same, right knocked out to yellow |

Each was calibrated by planting: the record answering nothing fails the first two and nothing
else; the shading's opaque form withheld fails the third and fourth; the stencil mask's record
withheld fails the fourth alone. One existing assertion went the other way —
`transparency_groups::a_knockout_group_reports_only_where_the_two_models_differ` required a
translucent `sh` inside a knockout group to *keep* the report, which is the report this round
closes; it requires the drawing now. The same four documents were written to disk and rendered
through `open_one` with all three plants in and with none, and the sheet is what the table
says.

## 5. Two references read the soft mask as shape, and this tree does not follow them

mupdf and ghostscript both draw the `/SMask` fixture with the blue surviving under the mask's
zero — `(0, 0, 255)` at `(30, 50)` — which is what a knockout weighted by the image's *alpha*
produces; poppler draws the whole square blue. On the stencil, the shading and the pattern
fixtures mupdf, ghostscript and this tree agree pixel for pixel (poppler draws the first flat,
and ghostscript drops the `ca` on the fourth). And on the shading fixture mupdf knocks the blue
out under the whole clip, where §11.6.4.2 bounds a `sh`'s shape by "the shading's painting
geometry"; ghostscript and poppler draw it as this tree does.

Principle 5's direction of inference is one way. The disagreement went back to the clause, and
the clause answers three times: §11.6.4.2 names the two masks that modify an image's shape and
the soft mask is not among them; §11.6.4.3 lists an image's `/SMask` among the ways the soft
mask is specified and makes it opacity under Table 57's default; and §11.4.6's NOTE 5 says
what a shape of 1.0 at an opacity of 0 yields — "the colour and opacity that result from
compositing the object with the initial backdrop", which at opacity 0 is that backdrop. The
alternative reading would make `/AIS` meaningless for an image's own mask, and the flag exists
because the two quantities differ. Two implementations agreeing is evidence about a shared
shortcut as often as about a clause; the one alpha per pixel this tree used to carry is the
same shortcut, and ADR 0234 built `Command::Shaped` to stop carrying it. If the project owner
wants Acrobat's picture over the clause's here, that is a decision for `doc/questions/`, and
the fixture is the question's exhibit.

## 6. What it measures

The gate lines are in `doc/history/997-*.md`. No corpus page states either element inside a
knockout group (ADR 1009 §6), so the corpus, oracle and `render-raster` summaries were
expected to hold and did — no page moved in any of the three — and the four fixtures are the
whole population of the change. `render-raster`'s corpus gate, red in 988's run for the one
name its list still carried, is green.

## 7. What this leaves

- **The field.** `pdf_render::Image` owes a `SampleAlpha` beside `interpolate`, and the
  record in `MaskCache` comes out when it lands. The constructors to touch are the ones §2's
  command counts, and `viewer-confined`'s codec has to carry the bit for its round trip to
  hold.
- **The implicit callers.** `implicit_knockout_group` takes the record as a fourth argument
  and `path.rs`'s one site and `text.rs`'s two pass `self.image_masks.drawn()`.
- **`SampleAlpha::Both`.** A stencil under an `/SMask` of its own stays reported by name. It
  is statable — the stencil decoded alone is the shape — at the cost of a second decode, and
  no document has asked.
- **`doc/todo/23`**'s two paragraphs that end "[t]he bit that would close both is the *kind* of
  an image's or a shading's alpha carried beside its value" are answered by this ADR; the file
  was not this round's.

## 8. Two habits, for the files that hold them

**"Carry the kind beside the value" is a sentence about a reader, not about a struct.** The
pricing named the two crates that decide the kind, and the natural field turned out to sit on
a type eleven files construct and one serialises. Before adding a field to a vocabulary type,
count its literal constructors across the workspace (one `grep` on a field name every
constructor has to write) and ask who actually reads the new fact; here one reader in the deciding crate
was enough for a record, and the field stays owed with its list.

**A quotation is attributed to the nearest clause cited before it.** The conformance checker
read `Shading::opaque`'s blockquote as §11.6.4.4's because that clause was named in the
sentence before the quote; naming §11.6.4.2 last, immediately before the colon, is what the
checker needs, and the failure names the wrong clause rather than the right one.
