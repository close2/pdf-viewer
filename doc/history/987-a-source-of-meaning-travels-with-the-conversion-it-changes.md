# 987 — The output intent reached the operators and not the images, shadings or masks; now it travels with the conversion

2026-09-12. ADR 1008. Colour, continued from 980: ISO 32000-2 §14.11.5, §8.9.5.1, §8.9.7,
§8.7.4.3, §8.7.4.5.5, §11.6.5.1, §11.6.5.2, §8.6.5.5; ISO 15076-1's lookup-table clauses.

Files: `crates/pdf-model/src/colour.rs`, `crates/pdf-model/src/content/colour.rs`,
`crates/pdf-model/src/content/image.rs`, `crates/pdf-model/src/image.rs`,
`crates/pdf-model/src/shading.rs`, `crates/pdf-model/src/inline_image.rs`,
`crates/pdf-model/src/soft_mask.rs`, `crates/pdf-model/src/icc.rs`,
`crates/pdf-model/tests/colour_paths.rs`, `doc/conformance/ledger.toml` (§14.11.5, §8.6.5.5,
§8.9.5.1, §8.9.7, §8.7.4.3, §8.7.4.5.5, §11.6.5.1, §11.6.5.2),
`doc/adr/1008-a-source-of-meaning-travels-with-the-conversion-it-changes.md`, this file.

Session 980 carried §14.11.5's output intent to the operator route and named four files it could
not reach. This round carried it to them by putting it into `Conversion`, the value the
interpreter already hands every route that converts after it — the compositing target and the
black point travelled there; the intent is the third thing that is not a property of the colour —
so an image XObject, an inline image, a shading stated inline or by reference, and a mesh's
vertices now parse their `/ColorSpace` under the page's intent. One integration test names all
five routes and was calibrated by dropping the intent from the conversion, under which all five
answer the assumed press's cyan.

The soft mask's function reads the intent too — `soft_mask::entry_with_output_intent`, with a unit
test showing a `/DeviceCMYK` luminosity group become the intent's press — but its one caller is
`content/ext_gstate.rs`'s `gs`, a neighbour's file this session, and still calls `entry`, which
hands `None`. §14.11.5's row is `partial` for that one line.

`ColourSpace::Icc` holds an `Arc<Profile>` now, measured first: 4000 `k` operators under the
corpus's one CMYK intent cost 1 799 M instructions, 95% of them copying the profile's tables once
per operator, and cost 49.8 M after; the `cs … scn` form went from 3 568 M to 71.6 M.

`'Lab '` data-space profiles stay refused, and the refusal now says why at the site: ISO 15076-1
defines a Lab encoding for a table's connection-space side only, so the scale of a Lab *device*
input is Table 66's `/Range` to state — which this tree does not read, and which §8.6.5.5's row
said it did for nine hundred sessions. Corrected. A scan of 1249 documents across every corpus on
this disk found 333 embedded profiles and none with a Lab data space.

What moved: one corpus first page, `issue20513.pdf`, whose `/DeviceCMYK` shadings and image now
draw through the same press as the fills beside them — 6 384 of 1 767 168 pixels, at most 53
levels, in a logo's lettering that reads the same on both arms. The other 973 digests are
byte-identical.

Found and not fixed: an image's `/SMask` is decoded against the resources in force, so a
`/DefaultGray` would remap an alpha as a grey; no corpus document states one.
