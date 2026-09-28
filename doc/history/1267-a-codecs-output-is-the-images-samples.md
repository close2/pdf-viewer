# 1267 — A codec's output is the image's samples, and §12.5.6.23 departs on its overlay alone

Session 1267, 2026-09-28. ADR 1371.

**Finding.** The brief's three codec residues were two mistakes and one debt. Table 87 already
says what a codec delivers: "a CCITTFaxDecode or JBIG2Decode filter shall always deliver 1-bit
samples, a RunLengthDecode or DCTDecode filter shall always deliver 8-bit samples". ADR 1133's
eight-bit `DeviceRGB` re-expression was the one choice that left the image's own domain. The matte
refusal, the colour-key stencil and the "shape no fresh raster holds" (`reexpressed`'s errors) all
came from it.

- **Residue 1, the matte.** `pdf_model::image::filter_samples` runs the codec as a filter. The
  redaction clears those samples and carries the dictionary, with `/BitsPerComponent` stated as the
  filter's. Table 144's matte ("valid colour components in that colour space") is therefore kept in
  every space. There are fixtures for `CalRGB`, `ICCBased`, `DeviceCMYK`, `Separation` and a
  one-bit CCITT picture. The colour key is carried, and the stencil construction is deleted.
- **Residue 2, depths.** The writer's JPX request now returns each channel's own integers.
  `Raster::depths` crosses the sandbox pipe. They are written at the widest depth (8 or 16), each
  `/Decode` pair widened from its own depth. The fixture is a 12/16/12-bit codestream
  (`opj_compress` with one `Ssiz` byte patched, confirmed by `opj_decompress`). A uniform depth
  below eight is no longer stretched.
- **Residue 3.** With no fresh raster, the shape errors are gone. A JPX palette is carried
  value-exact as direct colour. Components subsampled differently are upsampled by the decoder.
  Uniform subsampling is refused by §7.4.9's grid rule. A JPX image mask is packed from its 1-bit
  samples. A dictionary that contradicts its filter is refused under §8.9.5.1 and Table 87.
- **Decision (ADR 1371):** a JPX component above sixteen bits is refused, since carrying it needs a
  lossless JPEG 2000 writer. Census: 0 of 175 raw codestreams above sixteen bits or mixed.
- **Rows:** §12.5.6.23 partial → **departed** (overlay only, A64/ADR 1124). §12.5.6 and §12.5
  partial → departed, following their children. §12.1 stays partial for §12.8 and §12.10.
  `doc/todo/64` is deleted (`git rm`). Its README row is gone. HANDOVER and crate-map pointers
  now point to ADRs 1124 and 1371 (small edits in 1271's files).
- **Gates.** rustfmt: 0. Clippy on pdf-sandbox, pdf-model and pdf-transform: clean on my files;
  1268's `tests/zz_r1268_corner.rs` fails pedantic. nextest: pdf-sandbox and pdf-transform 506/506,
  pdf-model 1679/1679. `cargo test -p conformance`: 0. `pdf-transform --test gate`: pass.
  `pdf-model --test corpus`: pass. `raster_golden` exits 101 with 11 moved, all "raster only
  under an unchanged list". That is render-cpu work, not mine: 0 display lists moved.

Files: `crates/pdf-sandbox/src/{decode,protocol}.rs`, `crates/pdf-model/src/image.rs`,
`crates/pdf-transform/{src/redact.rs,tests/redact.rs}`, `doc/conformance/ledger.toml`,
`doc/todo/{64 (deleted),README.md,65}`, `doc/{state-of-play,HANDOVER,crate-map}.md`, ADR 1371.
