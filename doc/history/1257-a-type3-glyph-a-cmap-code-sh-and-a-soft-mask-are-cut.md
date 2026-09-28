# 1257 — A Type 3 glyph, a `CMap` code, `sh` and a soft mask are cut, and a matte is undone once

Session 1257, 2026-09-28. ADRs 1351, 1352.

**Finding.** Two premises of the brief were wrong. Type 3 fonts are §9.6.4, not §9.6.5, and the tree
cited §9.6.5 too. `sh` is §8.7.4.2, not §8.7.4.5.2. The four font and paint refusals each gave way
to a reading of the clause that defines the content. The strokes' refusals were a missing
restatement, not a missing reading. Only one question needed an ADR of its own: whether a soft
mask's group is content.

- **§12.5.6.23, partial (narrowed).** A Type 3 code is tested against its glyph's declared box
  (`d1`, else `/FontBBox`). A composite code takes the bytes `CMap::next_code` delimits, from the
  loader's `CMap` (`pdf_font::composite_cmap`, now public). `sh` is painted through its clip cut to
  the region's complement, but only for a gradient law; a mesh, a function-based shading or a
  sampled function is refused, and so is a path in such a shading pattern. A soft mask's group is
  entered at its `gs` and cut. A stroke's survivors are filled in the initial colour, or under a
  new `gs` restating `/CA` and overprint. Still refused: a `d0` glyph under an all-zero
  `/FontBBox`, a vertical `CMap`, located shading data, a zero-width stroke, the codec cases, and a
  form drawn twice whose placements the region meets differently. Fifteen fixtures, including
  §9.6.4's EXAMPLE font.
- **§11.6.5.2 (matte split).** The `DCTDecode` route now repacks its samples and undoes the matte in
  `unpack`, through `Prematte::restore`. That is the raw route's function. `restore_samples`,
  `unblend` and `Decode::raw_of` are gone. `matte_routes.rs` shows both routes give identical bytes
  for three mattes. The old integer route differs on up to 24 of the 64 opacities in those fixtures.
  `issue13931.pdf` with a scratch `/Redact [460 712 540 752]` over its seal: **0 pixels differ
  outside the region** at scales 1 and 2. 1248 measured about 1.9k. The only marks left inside
  are within one pixel of the edge.
- Tier 1: `rustfmt`, clippy (`--no-deps`: render-cpu was mid-edit by 1255), nextest `pdf-transform`,
  `pdf-font`, `pdf-model`, `cargo test -p conformance`. Tier 2: `pdf-transform --test gate` passes,
  and so does `pdf-model --test corpus`. `raster_golden` fails with 20 moved. `issue13931.pdf` moves
  "raster and list": its matte is now undone by the raw route's rounding, which is intended.
  The other 19 are "raster only", which is 1255's render-cpu work. The golden is not regenerated.

Files: `crates/pdf-transform/src/redact.rs`, `crates/pdf-transform/src/redact/shading.rs` (new),
`crates/pdf-transform/tests/redact.rs`, `crates/pdf-font/src/{composite,lib}.rs`,
`crates/pdf-model/src/image.rs` (matte hunks), `crates/pdf-model/tests/matte_routes.rs` (new),
`doc/conformance/ledger.toml` (§12.5.6.23), `doc/todo/64`, `doc/todo/65` bucket 4,
`doc/state-of-play.md` (one paragraph), ADRs 1351, 1352.
