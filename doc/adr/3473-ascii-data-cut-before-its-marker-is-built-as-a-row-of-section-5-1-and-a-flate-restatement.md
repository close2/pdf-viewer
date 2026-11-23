# 3473 — ASCII data cut before its marker is built as a row of section 5.1 and a Flate restatement

Session 2193. Status: **accepted**; built. Context: ADR 3462 (the decision), ADR 3281 (the decoder's
report of the cut), ADR 2689 (`undecodable-stream`), `doc/pdf-a-mitigations.md` section 2.

## What was built, and the three choices the build made

ADR 3462's three items stand as decided: `pdf-archive`'s row
`file-structure/ascii-data-ends-at-its-end-of-data-marker` at section 5.1 of both parts, the rewrite
`ascii-data-as-decoded` and the loss of the same word, granted by `discard` in `as-shown` and
`as-if-printed` and by `try = ["append-statement", "discard"]` in `state-every-loss`.

1. **One question, asked by both crates.** `pdf_archive::table::ascii_stage_cut_short` decodes a
   stream's chain stage by stage up to its last ASCII stage and answers the stage that reported
   `Damage::Truncated`; a stage in front of it that refuses or whose bytes are in doubt answers
   nothing, because the cut is then not the ASCII stage's. The converter's preparation asks the same
   function of every stream the row names, so the stage the report names is the one the row failed.
2. **The re-encoding is `LZWDecode`'s, shared.** The body of the `LZWDecode` re-encoding became
   `rewrite::reencoded_through_flate`, which takes whether an ASCII stage's cut is admitted; for
   `LZWDecode` it is not, so that path is unchanged. A stage after the cut that reports damage of its
   own keeps the refusal (`NOT_REENCODED`), because its bytes are no longer the file's. A
   `/DecodeParms` array shorter than the chain is read as no parameters for the stages it does not
   reach, which is how `pdf_syntax`'s chain reads it, so the stages were decoded without any; the
   census's first run found pdf.js's `xobject-image` (`/DecodeParms [ ]` beside
   `[/ASCIIHexDecode /DCTDecode]`, the hexadecimal data with no `>`) refused there.
3. **The report names each stream; `xmpMM:History` does not.** The report's sentence and its JSON
   entry `ascii_data_restated` carry the object, the stage and the source's length. The history's
   entries are written before this preparation runs, as for `actual-text-private-use` (ADR 2889);
   adding one is a change to the packet's preparation order, not to this remedy.

## Proof

`PDFBOX-3148-2-fuzzed` converts under both shipped profiles at 2b and 4: two `A85` streams are
restated (objects 14 and 16; the first holds no data at all, which is still data without its
marker). veraPDF 1.30.3 passes all four outputs, and page 1 of each renders byte for byte the
source's raster; so do `xobject-image`'s four, its image restated. The tests are `tests/archive/ascii_data_as_decoded.rs`.
