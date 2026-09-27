# 1333 — A redacted JPEG 2000 image keeps its own samples and its own grid

Status: accepted and **built**.
Context: `crates/pdf-transform/src/redact.rs` (`Walk::plan_jpx_clear`, `Walk::matte_fill`,
`Walk::colour_key_stencil`, `own_dictionary`, `widened_decode`), `crates/pdf-model/src/image.rs`
(`jpx_samples`), `crates/pdf-sandbox/src/{decode,lib,protocol,worker,lockdown}.rs`
(`Request::JpxWhole`, `Sandbox::whole`), `quorra-transform redact --image-samples`,
`crates/pdf-transform/tests/redact.rs`, `crates/pdf-sandbox/tests/confinement.rs`.
Supersedes: ADR 1248's refusal of a `JPXDecode` image above eight bits; ADR 1277's refusals of a
codec picture's colour key and of a `/Matte` on its soft mask; ADR 1324's "Unchanged" paragraph,
which kept the reduced-level refusal on the ground that the decoder's budget is not lifted for one
verb.

§12.5.6.23 destroys "that portion of the image data" and asks nothing of the rest, which is
therefore to be carried as the file had it. Four refusals stood because the redaction wrote every
codec picture back as eight-bit `DeviceRGB`, a domain the file's own integers, ranges and matte are
not in. Each is answered by keeping the domain rather than by translating into a new one.

## 1. A `JPXDecode` image is written as its own samples

Table 87 leaves the depth to the processor — "[t]he bit depth is determined by the PDF processor in
the process of decoding the JPEG 2000 image" — and the confined decoder already determines the
codestream's own (ADR 1242). `pdf_model::image::jpx_samples` hands those integers over before any
colour is made of them, in the space §7.4.9's precedence names. The redaction writes them back
under `FlateDecode`: at 8 bits where the decode was eight or fewer, at 16 above. A precision that is
neither keeps its integers in the wider field and states `/Decode` with the far end moved to
`D min + (D max − D min) × (2^16 − 1) ÷ (2^p − 1)`, so §8.9.5.2's map gives every integer the value it
had. The dictionary's colour space and entries are carried; a codestream's own space is stated as a
device name, or as an `ICCBased` stream holding the profile it carried. `/SMaskInData`'s opacity
becomes Table 143's soft-mask image at the same precision, `/SMaskInData` 2 stating `/Matte` of the
space's zero, which is exactly the multiplication the code names.

Refused by name: a decode narrower than a component the codestream states (components that
disagree on a depth above eight, which the decoder delivers stretched to eight), and `/SMaskInData`
2 over an `Indexed` space.

## 2. It is decoded at full resolution within the operator's budget

§7.4.9 NOTE 3 lets an application "select and decode only the data making up a lower-resolution
version", and the viewer does. A redaction cannot: a reduced grid written back resamples every sample
outside the region. So the redaction asks `Request::JpxWhole`, which never steps down: the
codestream's grid fits the stated samples or the answer is a refusal naming both numbers.

ADR 1324 read the bound as principle 3's and not the verb's to lift. It is both: the viewer's bound
is the reader's protection and stays exactly where it was; how much memory and time one offline run
may spend is a fact about the machine it runs on, which is the operator's. `--image-samples`
defaults to the viewer's own `ORDINARY_SAMPLES` (2^26), and a larger value starts a **separate**
worker (`Sandbox::whole`) whose every bound is derived from it before a byte is read — an
address-space ceiling of sixteen bytes a sample (the ordinary worker's gigabyte over its 2^26, the
ratio it was measured inside, never below that gigabyte), an answer of two bytes a sample, thirty
seconds per 2^26 samples. Its confinement is otherwise the decoder's: Landlock, the same system-call
list, the kernel enforcing the ceiling. The worker reads its one argument before lockdown, as it reads
nothing else, and holds a request to what its ceiling was sized for.

## 3. A re-expressed codec picture's colour key becomes a stencil

A `DCTDecode`, `CCITTFaxDecode` or `JBIG2Decode` picture is still re-expressed (ADRs 1133, 1143),
and §8.9.6.4's ranges are "colour values before decoding with the Decode array", which the
re-expression leaves. What the ranges *mean* survives: "[s]amples in the image that fall within this
range shall not be painted", which is a §8.9.6.3 explicit mask. The picture is decoded once more with
the key as its only mask, the transparent samples become the 1s of an image mask on the picture's
grid, and that mask is cleared with the picture. The picture's colours come from the unkeyed decode,
so a masked sample keeps its file colour. A `JPXDecode` picture carries its key as written, because
its samples are.

The region's samples are the domain's zero in every raster (ADR 1126), stencil included, so the
region paints the zero colour. Where a carried key's range includes zero, the region is unpainted
instead. The clause states what is destroyed, not what the place then shows, and nothing of the
picture survives in either.

## 4. A picture pre-blended with a matte is cleared to the matte

§11.6.5.2: `c′ = m + α × (c − m)`, and the cleared soft mask states α = 0 over the region, where the
formula gives `c′ = m`. So the picture's cleared samples are the matte and not zero, and the region
holds pre-blended data the formula could have produced. Table 143 puts a matted mask on its
picture's grid, so the two clears cover the same samples. The matte is carried into samples by
§8.9.5.2's map run backwards, since the clause says the computation "shall use actual colour
component values, with the effects of the Filter and Decode transformations already performed". A
codec-free picture and a JPX one keep their domain and the mask keeps `/Matte`. A re-expressed
picture keeps the matte only where eight-bit `DeviceRGB` is the identity on its components:
`DeviceRGB` itself, and `DeviceGray`, whose `/Matte` is then restated three times. Any other space is
refused by name, because its conversion is not affine and Table 144's relation would not survive it.
An `Indexed` picture keeps zero, its matte being a base colour no index is guaranteed to name.

## Measured on two corpus documents

No corpus document carries a `/Redact` (the census is zero, trap 8), so one was added to two that
carry these images. `issue19326.pdf` (a sixteen-bit JPX with a RunLength soft mask, left half
redacted) came back at `/BitsPerComponent 16` and its page, rendered before and after at 144 dpi,
differs in no pixel outside the region. `issue13931.pdf` (a `DCTDecode` photograph with a `/Matte
[0 0 0]` mask) came back with the matte carried and the region cleared. Outside, 1869 pixels differ
by one level: the DCT route undoes the matte in eight-bit samples (`Prematte::restore_samples`) and
the Flate route in floats (`Prematte::restore`), and they round apart. The samples written are the
decoder's own, which the fixture test asserts. The larger differences sit at glyphs the region's
bounding-box rule removed whole.
