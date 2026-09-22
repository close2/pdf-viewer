# 1248 — A boundary is not a mark, a variant is reached through an entry, and a codestream states its own precision

Status: **accepted**.
Context: `crates/pdf-transform/src/redact.rs` (`PathObject::clip_bytes`, `Walk::run_stream`,
`Walk::paint_path`, `admits_a_cut`, `is_painting_operator`, `Walk::plan_image_clear`,
`Walk::plan_codec_clear`, `Walk::jpx_admits_a_clear`, `build_cleared_image`),
`crates/pdf-transform/tests/redact.rs`.
Builds: ADR 1195 (the nine-cell cut), ADR 1196 (forms, and copy-rather-than-refuse for a shared
object), ADR 1236 (the two refusals it lifted and the shape of the argument), ADR 1174 (which wrote
the `/Alternates` refusal), ADR 1133, ADR 1143, `doc/questions/A64`.
Amends: ADR 1195 §4's clipping-path refusal, ADR 1174 §3's `/Alternates` refusal, ADR 1133's
`JPXDecode` refusal. None is reversed by attrition: each is read against the clause that wrote it.
Clauses: ISO 32000-2 §12.5.6.23, §8.5.4, §8.5.3, §8.9.5.4, §8.9.5.1 (Table 87), §7.4.9.

## Three refusals, three different mistakes

ADR 1236 said of its two that "both sentences were true of the code and neither was true of the
geometry". These three are not one mistake repeated. The clipping path was a category error about
what a clip *is*; the alternate image was a solved problem answered with the wrong verb; the JPEG
2000 image was a real limit stated at the wrong grain.

## §8.5.4: the boundary is not a mark, and the clause separates the two acts in time

ADR 1195 refused a path that is also the clipping path because "cutting its geometry would move the
boundary every mark after the painting operator is held to". True — and it assumed the cut must be
one edit to one thing. The clause does not:

> Although the clipping path operator appears before the painting operator, it shall not alter the
> clipping path at the point where it appears. Rather, it shall modify the effect of the succeeding
> painting operator. After the path has been painted, the clipping path in the graphics state shall
> be set to the intersection of the current clipping path and the newly constructed path.

So `<path> W f` is two acts in an order: paint under the clip already in force, then intersect the
clip with the constructed path. Writing them back as two path objects reproduces that order exactly
— the cut marks with `f` first, then the producer's own bytes for the path followed by `n`, which

> shall cause no marks to be placed on the page, but can be used with a clipping path operator to
> establish a new clipping path

The boundary is therefore re-stated from the producer's bytes, not one coordinate of it re-derived,
which is what the old refusal was protecting and what the new construction keeps.

**The bytes are taken at the painting operator, not at the clipping one.** §8.5.4 says the operator
"may appear after the last path construction operator" — a permission, not a requirement — while
what it bounds is "the newly constructed path", which is whole only where the path object
terminates. Capturing at `W` would have cost a refusal for the out-of-order case; capturing at the
painter costs none, and a fixture constructs a second rectangle after the `W` to prove the boundary
still holds it.

`W n` was never refused and still is not: the operator dispatch drops a path `n` terminates without
ever reaching `paint_path`, which is right — such a path marks nothing, so a removal has nothing of
its to remove.

## §8.9.5.4: an alternate is reached through an entry, and the entry is the base's

ADR 1174 §3 paired §12.5.6.23's destruction sentence with §8.9.5.4's "variant representations" and
concluded that clearing the base alone "destroys one copy of the picture and leaves another in the
file". The inference is right; the remedy it reached for was destroying the alternates' samples,
which this writer cannot do (each variant is its own grid and filter). But destruction of samples
is not the only way a thing leaves a file.

§8.9.5.4 defines the entry as "an array of alternate image dictionaries specifying variant
representations of the base image", and its selection algorithm runs from that array. A base
stating none is therefore the image every reader draws, the printing path included. So the redacted
page's copy of the image drops `/Alternates`, the variants are reached from nothing, and the
closure the writer copies reaches only what is referenced — they are not in the output at all.
That is §12.5.6.23's "remove all traces of the specified content", by removal rather than by
re-encoding somebody else's grid.

Where the image is another page's too it is **copied** rather than replaced (ADR 1196), and that
page keeps its own base and its own variants. That is not a hole this opens: it is the position
that page's un-destroyed base samples were already in, and ADR 1196 decided it — the other
placement's marks are content the annotation did not identify.

The codec path already built a fresh dictionary and so already dropped the entry; what changed is
the codec-free path's carry-over list and the removal of the refusal in front of both.

## §7.4.9: the limit is the grid and the precision, not the codec

ADR 1133 refused `JPXDecode` because "a codestream over the decoder's budget comes back at a
reduced resolution level (§7.4.9 NOTE 3), so its raster is not the image's grid". That is a
condition, and it was being applied as a classification: the code refused every JPEG 2000 image,
including the overwhelming majority whose decode is exactly the image. (The module's own doc
comment promised a JPX-specific reason and the match arm gave the generic one, so the refusal did
not even say what the ADR said it said.)

**Is a JPX-to-Flate rewrite "destroying that portion of the image data"?** The clause asks one
thing of an image — "that portion of the image data shall be destroyed; clipping or image masks
shall not be used to hide that data" — and says nothing about the encoding the rest of it survives
in. Decoding the codestream, zeroing the region's samples and writing the whole grid back under
`FlateDecode` satisfies it: the removed samples are in the output in no form at all, which is what
the `DCTDecode` path already does on the same reading (ADR 1133). That the codestream was lossy
does not weaken it — a lossless re-encode of a lossy codestream keeps the decoded samples exactly,
and the samples a reader sees *are* the decoder's output.

**Is the outside-the-region content unchanged in the sense the clause requires?** The clause
requires nothing of it, but principle 1 and ADR 1196's rule do: content the annotation did not
identify may not be changed. Three conditions make that provable, and each is a refusal by name
where it fails:

- **The decode is on the image's own grid.** §7.4.9 NOTE 3 is the permission this tree's decoder
  takes for a codestream over its sample budget, and a raster on a reduced level would resample
  everything outside the region. The dictionary's `/Width` and `/Height` are what the raster is
  held to, and they are the codestream's own — §7.4.9 requires them to "match the corresponding
  width and height values in the JPEG 2000 data", and `pdf_model::image::decode` has already
  refused the image where they do not. The check is asked of **every** codec, because the question
  is about the grid rather than about which decoder produced it.
- **Table 87's `/SMaskInData` is absent or zero.** A non-zero value means the codestream carries an
  opacity channel, and §7.4.9 states "there shall be only one opacity channel in the JPEG 2000 data
  and it shall apply to all colour channels" — which the opaque re-encode has nowhere to put. The
  entry is meaningless for every other filter, so this is the one codec that needs the question
  asked, and asking it is what the existing alpha scan could never reach while JPX was refused
  first.
- **No component is deeper than eight bits.** The re-encode writes 8-bit `DeviceRGB`, so a deeper
  codestream would come back coarser outside the region. The precision is read from the codestream
  by `pdf_model::jpeg2000::Headers` rather than from the dictionary, because Table 87 withdraws
  `/BitsPerComponent` for this filter.

Data that states no precision at all is refused too: a gate that cannot read the thing it is about
must not pass.

**What this does not do.** It does not decode at a precision the sandbox does not return. The
re-encode's 8-bit `DeviceRGB` and the loss of a non-RGB colour space outside the region are the
costs the `DCTDecode` path already bears (ADR 1133) and they are borne here on the same terms; what
is new is that a codestream *stating* more than the re-encode can carry is refused rather than
quietly flattened.

**ADR 1242 landed in the same batch and does not move that condition**, which is worth writing down
because it looks as though it should. It carries a JPEG 2000 raster out of the confined worker at
the codestream's own depth so that §8.9.6.4's colour-key ranges can be compared in their own
domain — and says of everything above that boundary that "nothing else changed shape": the
eight-bit raster is still what `pdf_model::image::decode` hands back, and it is `image::decode` this
removal calls. So the depth condition is about the *re-encode*, not about the decoder, and what
would move it is a clearing path that carried more than eight bits through to the `FlateDecode`
stream it writes.

## Proof

Hand-built fixtures, because the corpus census for redaction is zero and cannot rank any of this
(trap 8). The clipping path is proved in **pixels**, not in operators, because a clip is not a mark
and a bounding box cannot see one: a black bar clips itself and a blue fill then covers the whole
page, so the blue can only stay inside the bar's rectangle if the boundary survived, can only be
absent from the region if the marks were cut, and a second rectangle constructed after the `W`
shows the boundary is the whole path. The alternate is proved by its own bytes being absent from
the output and the entry with them, with the base still destroyed in the region. The JPEG 2000 case
is proved on a real 8×8 lossless codestream — left four columns zero, right four still the encoded
sample, the original codestream gone from the file, the output `FlateDecode` `DeviceRGB` — with the
deep and the opacity conditions each refused by name off the same codestream, one byte changed for
the first and one dictionary entry added for the second.
