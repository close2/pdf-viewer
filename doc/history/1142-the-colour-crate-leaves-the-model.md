# 1142 — the colour crate leaves the model

Round 1142, architecture slot. ADR 1131 step 3 done: `crates/pdf-colour` created, the six
colour-stack modules moved into it, `pdf-model` re-exporting so consumers are unchanged.

## The crate

`crates/pdf-colour`, `#![forbid(unsafe_code)]`, one responsibility: PDF colour spaces and their
conversions. Its dependencies are only what the six use — `pdf-syntax`, `pdf-render`, `rayon`,
`md-5` (the ICC profile identifier), `thiserror` — and nothing from `pdf-model`. The modules moved
with their `#[cfg(test)]` tests: `colour`, `icc`, `function`, `shading`, `mesh`, `transfer`.

`pdf-model` gains `pdf-colour` as a dependency and re-exports each module —
`pub use pdf_colour::{colour, function, icc, mesh, shading}` plus
`pub(crate) use pdf_colour::transfer` (was a private `mod`) — so `crate::colour::…` still resolves
inside `pdf-model` and consumers still write `pdf_model::colour::ColourSpace`.

## The four snags 1138 named, and one it did not

1. The `transfer` test called `crate::Pages::new` (a layer above colour); it now walks the
   fixture's page tree through `pdf_syntax::Document` (catalogue → tree → first kid → resources).
2. `image.rs`'s `[crate::colour::cie_to_srgb]` — `cie_to_srgb` widened to `pub`, resolving through the re-export.
3. Three upward doc links (`colour.rs` `[crate::image]`, `shading.rs`/`mesh.rs`
   `[crate::Unsupported::LimitReached]`) and the boundary-crossing prose code-spans became
   `pdf_model::…`.

Not in 1138's list: `content` and `image` reach across the new boundary, so
`Transfer::{read, from_channels, channel}`, `Stated` and `ColourSpace::{default_decode,
component_range}` widened from `pub(crate)` to `pub` (with the `#[must_use]`/`Debug` that implies)
— the one visible cost of the split, since cross-crate use requires `pub`; the `transfer` doc that
called `read` "private" was corrected. The ledger's `code`/`test` sites for the six files
repointed `pdf-model`→`pdf-colour` (111 sites), test-function names unchanged.

## Gates

`cargo build --workspace` exit 0. `RUSTFLAGS="-D warnings" cargo clippy -p pdf-colour -p pdf-model
--lib --tests` exit 0. `cargo nextest run -p pdf-colour -p pdf-model` 1516 passed, 16 skipped
(pdf-colour 103, incl. the moved transfer test). `cargo tree`/`metadata`: pdf-colour over
render+syntax, pdf-model over pdf-colour, no cycle. `raster_golden` held 974, moved 0.
`cargo test -p conformance` 259 passed, 0 failed. `doc/crate-map.md` gains the row.
