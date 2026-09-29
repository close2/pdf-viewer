# 1275 — A non-extended end is a hard stop, and §7.6.5's witness set is held by a test

Batch forty-two. No ledger status moved. ADR 1387.

## A168, written where it belongs
§7.6.5's note corrects the census claim and names the ruling; §7.6, §7.6.5.1–.3 and §7.6.6 carry one
shared sentence naming the four `doc/corpora/pdfbox` documents, both keystores (`CN=testnutzer`
`5F609C62`; `CN=test` `60FFD550`), the keyless fifth (`3006236.pdf`) and A168. `doc/questions/A66`
gained one dated paragraph, appended and saying so. `doc/todo/65` bucket 5 and `doc/todo/51` re-read.
Measured on the way: all four census `PDFBOX-4421-*.pdf` are encrypted to `CN=testnutzer`; `-0` and
`-1` are byte-identical to `AESkeylength128/256.pdf`.
`crates/pdf-syntax/tests/public_key_witnesses.rs`: each document's `/Recipients` read by this crate's
parser and a test-local DER walk (pdf-signature's `der` is above pdf-syntax), held to the keystore's
issuer and serial; the certificates are `openssl pkcs12 -legacy` evidence (40-bit RC2 bags). Planted
(5249 documents held to the 4421 keystore): fails.

## The CPU oracle's stripe rows
Not precision. `render-cpu`'s `stops` compressed every ramp stop into `[0.0005, 0.9995]` to fade a
non-extended end inside the ramp — error zero at the axis middle (t = 0 here, so 1× saw nothing), 3.6
rows at t = 3 at 8×. Now a hard transparent stop at the end's own position; nothing else moves.
`issue10572.pdf` at 8×, column 1 200: before −1 … −4 rows, after 0 on all five boundaries.
Non-cone radials shared it and are fixed; cones, `/Background` and type 1 are evaluated per pixel.
`render-gpu` (asked after the report): same construction, bounded by Vello's 512-texel ramp — a hard
stop at 1 is lost there (texel 511 takes the first stop), so the far end's pair sits 1e-6 below 1.
Before: `/Extend` moved stripes by a texel (16 rows at 8×); after: the four pairs agree, every boundary
and both cuts within half a texel. Two `headless_gpu.rs` tests; each planted variant fails one.
Fixture `crates/render-cpu/tests/stripe_rows.rs` (rows from the file in f64, 1×–8×, all four
`/Extend` pairs, and both cut ends); planted away: both tests fail.

## Pages moved
`raster_golden`: 8 raster-only (ShowText-ShadingPattern, bug852992_reduced, issue6298,
issue7339_reduced, issue8092, issue9243, pattern_text_embedded_font, shading_extend), each at most one
level on smooth gradients, 0.000% differing at the oracle's threshold; shading_extend looked at.
Oracle green, no group moved; `render-raster` corpus 958 agree / 2 differ, as before.
`headless_gpu` (gates, 39 passed; its `--ignored` line runs 0 tests) and `render-raster` corpus
(958 agree / 2 differ, unmoved) re-run behind the lock after the render-gpu change.
