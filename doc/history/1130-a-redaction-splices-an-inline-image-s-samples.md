# 1130 — a redaction destroys an inline image's samples in the region

2026-09-16. ADR 1132, continuing round 1119 (ADR 1126). Contract: build what 1119's redaction
refused — an inline image (§8.9.7) meeting the region — as the content-stream splice §12.5.6.23 asks
for, proven unrecoverable.

**Census (trap 8).** `/Redact` is object-stream-hidden (byte-grep finds none corpus-wide). Round
1119's parsed census stands: five `/Redact`-bearing documents, **none meets any image, path or form**,
so none meets an inline image. Running the `redact` verb over the eleven Isartor `6.5.2` annotation-
type witnesses (the `/Redact` carriers) confirms no inline image meets a region. Fixture-proven
(trap 13), the same standing as 1119's image-XObject case.

**The build** (`crates/pdf-transform/src/redact.rs`). An inline image's samples live in the content
stream, not a referenced object, so destroying them is a byte edit of `/Contents`: the whole
`BI`…`ID`…`EI` run is replaced with a freshly built inline image whose region samples are the same
zero constant (§8.9.5.2: 0 → `Dmin`, one constant, any `/Decode`), re-encoded FlateDecode, every byte
outside the run left as it was. Samples come from `Document::image_stream`; the clearing reuses 1119's
`clear_region`/`zero_sample`. Two splice-only cares: the edit ends at `EI` (trimming the white space
`scan.resume` skips) so `EI` stays white-space-terminated and the separator is byte-identical; and no
§7.3.8 indirection may reach a content stream, so `inline_safe` refuses a `/CS` that resolves to a
resource object holding a reference or stream. Device spaces and inline `[/Indexed …]` arrays pass.
`Origin::Redacted`'s `images` count now includes spliced inline images.

**Refused, each narrower (trap 5, principle 1).** A codec inline image (DCT/JPX/CCITT/JBIG2), a
colour space that resolves to a resource object an inline image cannot carry, or a grid the data does
not fill — 1119's three limits, at the splice rather than at a `Do`. The refusal is the whole run or
nothing; a partly cleared inline image is never produced.

**Proven unrecoverable.** `inline_image_samples_inside_a_quadpoints_region_are_destroyed_and_the_stream_is_spliced`:
an 8×8 unfiltered inline image over [50,150]², `/QuadPoints` on its left half — after redaction the
left four columns are zero in the re-read image, the right four byte-identical, the content before
`BI` and after `EI` byte-identical, the original block gone, the file re-opens and the image decodes
and draws. Plus a codec-refused witness and an untouched-when-clear witness.

**Row moved:** §12.5.6.23 stays `partial` (inline splice built; overlay departs; path/form, codec/
shared XObject and codec/reference inline images owed), note rewritten, cites redact.rs + three new
tests + ADR 1132.

**Gates.** Shared tree, five siblings mid-flight (pdf-model in churn). Scoped to `redact*` and the
row. See the report for exact lines and exit codes.
