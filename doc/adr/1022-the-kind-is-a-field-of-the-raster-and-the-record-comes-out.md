# ADR 1022 — The kind is a field of the raster, the record comes out, and `Both` is conforming

Status: accepted, 2026-09-12. Session 1002.

## Context

ADR 1017 decided *what* an image's alpha channel carries — §11.6.4.2's shape or §11.6.4.3's
opacity — and put the answer where the round could reach it: a record inside `MaskCache`, keyed by
the raster's identity, written as the interpreter drew and read when a knockout group closed. Its
§2 said in as many words that the right home is a field of `pdf_render::Image`, and its §7 listed
what the substitute cost: the field, the callers that were handed no record, and `SampleAlpha::Both`.
This round took all three.

Files: `crates/pdf-render/src/{lib.rs, paint.rs, shading.rs}`,
`crates/pdf-render/examples/area_bench.rs`, `crates/pdf-model/src/image.rs`,
`crates/pdf-model/src/content/{image.rs, path.rs, text.rs, transparency.rs}`,
`crates/render-cpu/src/images.rs`, `crates/render-cpu/tests/image_placement.rs`,
`crates/render-gpu/tests/headless_gpu.rs`, `crates/render-raster/examples/filtered_edge_colour.rs`,
`crates/viewer-confined/src/protocol.rs`,
`crates/viewer-confined/src/protocol/{display_list.rs, panels.rs}`,
`crates/viewer-core/src/transition.rs`, `crates/viewer-host/src/clock.rs`,
`crates/viewer-ui/src/bin/quorra-confined/screen.rs`, `crates/viewer-ui/tests/panel.rs`,
`crates/pdf-model/tests/{transparency_groups.rs, raster_golden.tsv}`,
`doc/conformance/ledger.toml` (§11.3.7.2, §11.4.6, §11.6.4.3), `doc/todo/23`, `doc/todo/00`.

## 1. The field, and the constructors it reaches

`pdf_render::SampleAlpha` sits beside `interpolate` on `pdf_render::Image`, with the three values
ADR 1017's reading gives: `Shape` for §11.6.4.2's rectangle — a stencil's painted areas, a
rectangle less what an explicit or colour key mask removed, an unmasked image whose alpha is 1.0
throughout — `Opacity` for §11.6.4.3's mask, an `/SMask` on either route or a non-zero
`/SMaskInData`, and `Both` for a stencil under an `/SMask` of its own.

`ImageAtDeviceScale` gains `sample_alpha()` beside `samples()`, answered **without** producing a
raster: a deferred source exists because the device decides the grid, and the kind is a fact about
the masks rather than about the grid. `ImageSource::sample_alpha()` is the one call site a reader
needs, and it is what `transparency::stated_shape` now asks.

The literal constructors are the ones `grep -rn 'interpolate[:,]' --include=*.rs crates` names, and
every one of them is a decision rather than a default. Three kinds, and the third is the one worth
having a rule for:

- **A document's image**, in `pdf_model::image::decode_parts`, which is where the masks are applied
  and therefore the only place the kind is *derived*. A free function `sample_alpha(stencil,
  soft_masked)` replaces ADR 1017's `image::SampleAlpha::of` — the vocabulary moved to `pdf-render`
  and only the decision stayed.
- **A raster this crate builds out of two**, which carries the base's: `combine_on_the_finer_grid`
  for the eager route, `MaskedAtDeviceScale::sample_alpha` for the deferred one, and
  `Image::area_averaged`'s reduction, where averaging does not change what an alpha *is* — a stencil
  reduced is still a stencil, with fractional coverage where its edges fell inside a block.
- **A raster that is not a document's image at all**, which is most of the list: a shading's own
  mesh, radial and sampled rasters in `pdf-render`'s `shading.rs`; §12.3.4's thumbnail; a viewport's
  own pixels in `viewer-core`'s transition and `viewer-host`'s clock; every backend's test fixture.
  Their alpha is `Shape` and the reason is not a default: nothing in §11 ever reads it, and what
  such a raster states is the rectangle it covers. A shading's is the sharpest instance — §11.6.4.2
  bounds a `sh`'s shape by "the shading's painting geometry", and the raster's alpha is exactly
  where the shading paints.

## 2. What came out, and what it became

`image::DrawnAlphas` held two records and only one of them was a substitute for this field. The
image half — the `ImageSource`, cloned to pin its allocation, beside its kind, found again by
`Arc::ptr_eq` — is gone with `same_source` and `alpha_of`. The other half stays and is the whole of
the type now, renamed `image::ShapeMasks` for what it is: **which soft masks this run built out of a
stencil**. §8.9.6.2's stencil painted through a pattern becomes a §11.5.2 alpha mask on the fill that
paints the pattern (ADR 0151), and that mask *is* the stencil's shape where every other mask a
command carries is §11.6.4.3's opacity. A `SoftMaskId` cannot say which, and no field on the raster
can either: the mask is on a **fill**, and the stencil that made it is not that command's image.
So the record is not a leftover — it is the part of ADR 1017's construction that a field on `Image`
was never going to replace.

**One behaviour changed by the field existing, and it is a gain rather than a side effect.**
`stated_shape`'s image arm was `drawn.alpha_of(image)?` — `None` for a raster this run had not
recorded — and `unstatable_shape` named that case "an image drawn outside this run, whose alpha's
kind was not recorded". A raster carries its kind wherever it came from, so that sentence has no
referent and is gone; the only image a knockout group still cannot state is `Both`.

## 3. The wire carries it, because the samples cannot

`viewer-confined`'s display-list codec writes one byte after `/Interpolate`'s and reads it back,
with `sample_alpha_tag` written out both ways for `blend_tag`'s reason: a closed enumeration named
variant by variant makes an addition to it a build failure in the codec rather than a kind that
crossed as a different one. The bit is not recoverable from what is already on the wire — a
stencil's `{0, 255}` and a one-bit soft mask's are the same bytes — so it is carried or it is lost,
and the host is where §11.4.6 reads it.

§12.3.4's thumbnail is the one `Image` the codec does **not** carry it for, and the encoder's
destructure says so by naming the field rather than eliding it with `..`: a thumbnail's alpha is the
rectangle it covers, so a byte on the wire would be a constant the untrusted side could contradict.
A field added to `Image` is a build failure at that destructure, which is the point of writing it
out.

**Calibrated** by planting the decoder's `Opacity` arm to answer `Shape`: two tests fail —
`what_decodes_re_encodes_to_the_same_bytes` and `a_whole_page_round_trips_to_an_equal_list` — and
pass with the arm right. The fixture `an_image` carries `Opacity` deliberately, so that a codec
which dropped the field would disagree with the round trip rather than agree with its default.

## 4. The implicit callers, and the picture they were missing

`implicit_knockout_group` takes `&ShapeMasks` as a fourth argument and `path.rs`'s one site and
`text.rs`'s two pass `self.image_masks.shape_masks()`. Until this round they passed an empty record,
so §11.7.4.4's and §9.3.8's implicit group — a `B`'s two portions, a text object's glyphs — refused
any group holding an image and the parts were composited one by one with §11.6.2's report. The image
half of that refusal is closed by the field alone; the fourth argument closes the stencil-through-a-
pattern half, which is the one the record still answers.

`an_image_among_the_parts_states_the_shape_its_raster_names` is the fixture, and it is §11.4.6's two
stages over an element whose alpha is opacity: a 2×1 image, left sample transparent and right green,
over an opaque blue square it exactly covers. §11.6.4.2 gives the image the shape "1.0 inside the
image rectangle and 0.0 outside it", so the blue is knocked out under the **whole** square, and
§11.4.6's NOTE 5 says what a shape of 1.0 at an opacity of 0 leaves — "the colour and opacity that
result from compositing the object with the initial backdrop", which on this isolated group's
transparent backdrop is nothing. The page shows through on the left and the image on the right.

Calibrated both ways, which is the whole of its value:

| plant | what fails |
|---|---|
| `stated_shape`'s image arm answers `None`, as an unrecorded raster did | the group is refused: `expect("an image states the shape its raster names")` |
| the `Opacity` arm states the image itself — mupdf's and ghostscript's reading (ADR 1017 §5) | the left pixel draws blue where §11.4.6 NOTE 5 gives the page |

## 5. `SampleAlpha::Both` stays reported, and the price is not what ADR 1017 quoted

ADR 1017 §7 priced stating a stencil's shape under its own `/SMask` at "a second decode". **That is
wrong and the corrected price is worth the paragraph**, because it is cheaper and still not
affordable, for a different reason.

There is no second decode anywhere. On the deferred route the stencil *is* retained —
`MaskedAtDeviceScale.base` is the unmasked raster, kept for the device to combine — so the shape is
already in hand at no cost at all. On the eager route `apply_soft_mask` takes the base by reference
and allocates a new raster, so the pre-mask raster could be kept for the price of an `Arc` clone.
What is actually unaffordable is neither: it is that **one `Image` would have to carry two rasters**,
and the display list's vocabulary carries one. Stating `Both` means a second raster on
`pdf_render::Image` or on `Command::Image` — a field every backend, both censuses and the codec
carry, and memory held for the life of the display list — paid for **every** stencil under an
`/SMask`, whether or not any knockout group ever asks. §11.4.6 is the only reader (ADR 1009 §6), and
no corpus page states such an element inside one.

**And `Both` is a conforming construction rather than a malformed file, which is a clause reading
this round owed and had assumed the other way.** Three places say so, and the third is the one the
"read the paragraph, then the one before it" habit produced:

- **Table 87's `/ImageMask` row** enumerates what a stencil may not carry — `/BitsPerComponent`
  other than 1, `/ColorSpace`, `/Mask` — and `/Mask`'s own row says it twice, once in its condition
  and once in its text: "If `ImageMask` is true, this entry shall not be present". `/SMask`'s row
  says nothing of the kind. A row that lists the forbidden entries and omits this one has been read.
- **§8.9.6.2** lists the three ways "[a]n image mask differs from an ordinary image" and they are
  those same three: no `/ColorSpace`, `/BitsPerComponent` 1, and what `/Decode` means. Not a fourth.
- **§8.9.6.1, the paragraph before it**, adds the soft mask to the masking effects an image
  dictionary may carry and does not exclude a stencil from them: "a fourth type of masking effect,
  soft masking, is available through the SMask entry" — written of image dictionaries generally, in
  the subclause whose next page defines the stencil.

§11.6.5.2's Table 143 restricts the *soft-mask image's* dictionary rather than the parent's, so it
does not narrow any of the three. `Both` is a file a producer may write, and what this tree owes it
is a report by name, which it has.

## 6. The golden, and the pages that moved

`crates/pdf-model/tests/raster_golden.tsv` is regenerated in this change and the diff is the review
(ADR 1016). **231 of 974 entries moved and every one of them is `list only (an interpreter change no
pixel shows)`** — checked against the diff rather than taken from the gate's classification: on all
231 lines the outcome word, the extent, the raster digest and the reports digest are identical and
the display list's digest alone moved. That is what a new `Debug` field on `Image` predicts, and it
is also the proof that nothing structural moved anywhere in the corpus: a knockout group that had
been reporting and now draws would move the raster *and* the reports, and neither moved. The oracle
run beside it reports the same three counts as session 997's — `agrees 991, contradicted 62,
ambiguous 835` — which is a second instrument saying the same thing over 1957 pages.
`doc/history/1002-*.md` has the gate lines.

## 7. What this leaves

- **`SampleAlpha::Both`**, above, with its corrected price. Revisit it with a document, not with a
  budget.
- **A second raster per command** is the shape of the remaining §11.3.7.2 debt and it is the same
  shape §11.4.4's row has owed since the seventy-first session: a shape channel every command
  carries. Nothing but §11.4.6 would read it, and §11.4.6 reads a *stated* shape.

## 8. The habit

ADR 1017 §8 wrote: "'[c]arry the kind beside the value' is a sentence about a reader, not about a
struct", and asked a round adding a field to a vocabulary type to count its literal constructors
first. That habit held and the count was right — but the conclusion it reached, that one reader in
the deciding crate was enough for a record, is what cost the two implicit callers their picture for
five rounds. So the habit gains its second half: **a record keyed by identity is a substitute with
an expiry date, and what expires it is a second reader.** The moment `implicit_knockout_group` was
asked the same question the form caller asks, the record's key stopped being reachable and the field
became the cheaper construction — not because the count changed, but because the *readers* did.
Count the constructors to price the field; count the readers to decide whether a record can stand in
for it at all.
