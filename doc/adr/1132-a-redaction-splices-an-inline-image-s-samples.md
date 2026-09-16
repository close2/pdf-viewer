# 1132 — A redaction destroys an inline image's samples by splicing the content stream

Status: accepted. Session 1130.
Context: `crates/pdf-transform/src/redact.rs` (the `Walk::inline_image`, `Walk::spliced_inline_image`,
`build_inline_image`, `inline_safe`, `end_of_ei` items, and the `inline_images` count carried through
`PageEdit`/`Origin::Redacted`), `crates/pdf-transform/tests/redact.rs`.
Builds: ADR 1126 (round 1119's image XObject sample destruction, whose `clear_region`, `row_stride`,
`image_layout` and `zero_sample` this reuses), ADR 1124 (round 1112's content walk and refusal set),
`pdf_model::inline_image::scan`, `pdf_syntax::write::object`, `pdf_syntax::serialize::flate_encode`.
Clauses: ISO 32000-2 §12.5.6.23 (Table 195), §8.9.7 (Table 91, Table 92), §8.9.5, §8.9.5.2, §7.2.3.

## 1. The inline-image case ADR 1126 refused is built

ADR 1126 destroyed the samples of an image **XObject** whose placement meets the region and refused
an **inline image** (§8.9.7) as "an owed content-stream splice". An inline image's samples live in
the content stream itself — between `ID` and `EI` — not in a referenced object, so destroying them
is a byte-range edit of `/Contents` rather than the replacement of a copied object.

The whole `BI`…`ID`…`EI` run is replaced with a freshly built inline image. Its samples come from
`Document::image_stream` exactly as an XObject's do (codec-free bytes are the packed samples), are
zeroed under this placement by ADR 1126's `clear_region` — §8.9.5.2 maps sample value 0 through any
`/Decode` to `Dmin`, one constant carrying none of the original sample — re-encoded `FlateDecode`,
and written back under a dictionary carrying every §8.9.7 key the source stated with only the old
encoding replaced. **Every byte outside `[bi_start, EI-end)` is left exactly as it was.**

## 2. Two things a splice must get right that an object replacement need not

**The run ends at `EI`, not at `scan.resume`.** `pdf_model::inline_image::scan` resumes *past* `EI`
and the white space §7.2.3 lets delimit it; the edit ends at `EI` itself so that separator — part of
the surrounding stream — crosses the output byte for byte and, more than cosmetically, so `EI` stays
white-space-terminated (a run ending `EI` immediately followed by the next operator would re-lex as
one keyword). `end_of_ei` trims the trailing white space off `resume` to find it.

**No indirection reaches a content stream.** The scan's dictionary is Table 91's keys in full and
Table 92's colour-space and filter abbreviations resolved, so it is written the long way, which
§8.9.7 permits ("the abbreviations … may be used in place of the full names"). But a `/CS` that named
a resource colour space is resolved to that resource *object*, which may hold a §7.3.8 reference or
stream — an `/ICCBased` space, a `/Separation` tint — that no content stream can carry. `inline_safe`
refuses such a page by name rather than write a reference into a `BI` run. Device spaces and inline
arrays (an `[/Indexed /DeviceRGB 255 <palette>]`, the common small inline image) pass.

## 3. What stays refused, each with a narrower reason (trap 5, principle 1)

An inline image meeting the region is refused, its content and `/Redact` left as the file wrote them,
where the samples cannot be re-encoded without trace: an image behind a **codec** (§8.9.5
`DCTDecode`/`JPXDecode`/`CCITTFaxDecode`/`JBIG2Decode`), whose bytes this splice does not re-encode; a
colour space that resolves to a **resource object** an inline image cannot carry (§2); or a grid the
sample data does not fill. These are ADR 1126's three limits, now decided at the splice rather than at
a `Do`. A partly cleared inline image is never produced — the refusal is the whole run or nothing.

## 4. Census and proof

**Census (trap 8).** Round 1119's parsed census found five corpus `/Redact`-bearing documents (the
Isartor `6.5.2` witnesses and four veraPDF PDF/A annotation fixtures), **none meeting any image, path
or form** — so none meets an inline image. Running the `redact` verb over the eleven Isartor `6.5.2`
annotation-type files confirms no inline image meets a region. The inline-image `shall` has no corpus
witness and is proven by a calibrated fixture (trap 13), the same standing as ADR 1126's XObject case.

**Proof.** `inline_image_samples_inside_a_quadpoints_region_are_destroyed_and_the_stream_is_spliced`:
an 8×8 unfiltered inline image over the page's [50,150]² square, `/QuadPoints` on its left half —
after redaction its left four columns are zero in the re-read image, the right four byte-identical,
the content before `BI` and after `EI` byte-identical (but for the reader's trailing white space), the
original sample block gone, and the file re-opens and the image decodes and draws. A codec inline
image is refused by name; an inline image clear of the region is left untouched. `Origin::Redacted`'s
`images` count now includes spliced inline images.
