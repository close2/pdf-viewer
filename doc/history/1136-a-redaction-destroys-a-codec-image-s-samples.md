# 1136 — a redaction destroys a codec image's samples in the region

2026-09-16. ADR 1133, continuing 1119 (ADR 1126) and 1130 (ADR 1132). Contract: build what 1119/1130
refused — a `DCTDecode` or `CCITTFaxDecode` image XObject (§8.9.5) meeting the region — as the decode
→ clear → re-encode §12.5.6.23 asks for, proven unrecoverable.

**Census (trap 8).** Rounds 1119/1130's parsed census stands: five `/Redact`-bearing documents, none
meeting any image, so none meets a codec image. The corpus submodules are not checked out in this
worktree (the isartor test skips), so this is the carried-forward parsed finding; the merge
re-validates. Fixture-proven (trap 13), the same standing as the XObject and inline cases.

**The build** (`crates/pdf-transform/src/redact.rs`). A codec's stream bytes are the codec's input,
not samples, so ADR 1126's packed-grid destruction has nothing to address. `plan_codec_clear`
decodes the image the interpreter's own way (`pdf_model::image::decode` → opaque `RGBA8`), the region
is cleared in the decoded raster with 1119's `clear_region` (§8.9.5.2: 0 → `Dmin`), and the opaque
colour components are written as an 8-bit `DeviceRGB` **FlateDecode** image under a fresh dictionary
(`build_cleared_image`). FlateDecode is lossless, so the cleared region is exactly zero and the
output is no codec — it cannot round-trip back through the lossy filter that would leak it. The codec
runs under the process's isolation (confined in the program, in-process in the test — principle 3).

**Refused, each narrower (trap 5, principle 1).** `JPXDecode` (an over-budget decode is a reduced
resolution level §7.4.9 NOTE 3, so the raster is not the image's grid); `JBIG2Decode` (buildable by
the same route but not proven this round); a `DCTDecode`/`CCITTFaxDecode` image that does not decode,
decodes short of its grid, or decodes with transparency (§11.6.4.2 — the opaque re-encode cannot
preserve it). The refusal is the whole image or nothing.

**Proven unrecoverable.** No JPEG/CCITT encoder exists in the tree, so the two 16×16 fixtures are
captured bytes (PIL), each decoded through the real codec and asserted against that decode. After
redacting the left half: the region pixels are the zero constant in the re-read image, the rest
byte-identical, the original codec bytes gone, the output a `FlateDecode` `DeviceRGB` stream, the file
re-opens and draws. `a_dct_image_…`, `a_ccitt_image_…`, `an_image_behind_jpx_is_refused_by_name`.

**Row moved:** §12.5.6.23 stays `partial` (codec-image XObject destruction for DCT/CCITT built;
overlay departs; path/form, JPX/JBIG2/shared and codec/resource inline images owed), note rewritten,
tests updated, cites ADR 1133. `pdf-sandbox` added as a `pdf-transform` dev-dependency for the CCITT
test's in-process isolation.

**Gates.** Shared worktree, five siblings mid-flight (a sibling's crypt.rs carries a pre-existing
`too_many_lines` clippy error, not mine). Scoped to `redact*`, the row, the ADR. Exact lines and exit
codes in the report.
