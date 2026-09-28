# 1263 — A `d0` glyph, a column, a hairline, a form met twice and located shading data

Session 1263, 2026-09-28. ADR 1363.

**Finding.** The brief's list of what 1257 left was the tree's, less the codec residue that
`doc/todo/64` also names. One premise was wrong: a sampled function behind an axial shading holds
nothing located in the region alone. §8.7.4.5.3 puts each value on a line that "extends
indefinitely perpendicular to that axis", so the clip cut is the whole removal. A radial shading's
values lie on circles, and those can lie wholly inside a region. The first walk also over-removed
silently: a form drawn twice, one placement missed and one met, was replaced for both.

- **§12.5.6.23, partial (narrowed to the codec residue).** A `d0` glyph under an all-zero
  `/FontBBox` is run through the interpreter on a probe page and measured, inheriting the line
  width. A vertical `CMap`'s removed code restores its `/W2` or `/DW2` `w1` through
  `pdf_font::VerticalDisplacements` (new). A zero-width stroke is cut along its path by
  `paths::subtract_along`. A form met differently at two placements is split on a second walk, one
  copy per edit, named in `/XObject` or `/ExtGState` (a soft mask's `gs`), at most 64 a page.
  Located shading data are destroyed where they are stated: meshes are cut, types 5 and 6 are
  written as 4 and 7, radial and function-based samples are cleared, and exponential pieces the
  region alone serves are zeroed. A calculator function serving region-only values is refused, and
  that is a decision. Nineteen fixtures (four replace refusal tests) and four unit tests of the
  split, with §9.7.4.3's EXAMPLES 2 and 3 and §8.7.4.5.4's EXAMPLES 1 and 2.
- The row stays `partial`. What keeps it there is a matte a re-expressed codec picture cannot
  keep, a `JPXDecode` whose components disagree on a depth above eight, and a decode shape no
  fresh raster holds. Each is a re-expression not yet built. `doc/todo/64` is kept, trimmed to
  these three.
- Pixel checks on meshes hold within 2 levels. §8.7.4.4 lets a renderer interpolate within
  smoothness tolerance, and a cut mesh is subdivided from different vertices. The zero-width curve
  test checks its geometry, because the renderer flattens each piece separately.
- A square cut into eight pieces by a region in its middle leaves a one-level pixel just outside
  its outer corner in render-cpu, on a plain path with no form. The form fixture's region was
  chosen to avoid it. It is not this round's, and it is in trap 56's family.
- Tier 1: `rustfmt`, clippy `--no-deps` (render-cpu and pdf-render were mid-edit by siblings),
  nextest `pdf-transform` and `pdf-font`, `cargo test -p conformance`. Tier 2: `the_transform_gate`.

Files: `crates/pdf-transform/src/redact.rs`, `redact/{forms,located,mesh,type3}.rs` (new),
`redact/{paths,shading}.rs`, `crates/pdf-transform/tests/redact.rs`,
`crates/pdf-font/src/{metrics,lib}.rs`, `doc/conformance/ledger.toml` (§12.5.6.23), `doc/todo/64`,
`doc/todo/65` bucket 4, `doc/state-of-play.md`, ADR 1363.
